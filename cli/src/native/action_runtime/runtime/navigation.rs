#![allow(unused_imports)]
use super::capability::service_browser_id;
use super::daemon::{launch_hash, BackendType, CloseBehavior, RuntimeHandoffDescriptor};
use super::launch::terminate_runtime_browser;
use super::recovery::{persist_closed_browser_health, runtime_profile_pid, DaemonState};
use super::remote_headed::persist_current_browser_health;
use crate::connection::get_socket_dir;
use crate::native::action_runtime::cancellation::cancellable;
use crate::native::browser::{
    should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo, ProcessExitObservation,
    WaitUntil,
};
use crate::native::browser_navigation::{
    add_manual_login_hint_warning, persist_service_owned_navigate_tab,
};
use crate::native::network::resolve_fetch_paused;
use crate::native::network_archive::{har_cdp_protocol_to_http_version, har_extract_headers};
use crate::native::service_model::{
    retained_display_allocation_candidates, service_profile_allocations,
    service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
    BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
    BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession, BrowserTab,
    ControlInputProvider, DisplayAllocation, JobState as ServiceJobState, LeaseState, MonitorState,
    ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy, ProfileLeaseDisposition,
    ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease, RemoteViewHandoff,
    RemoteViewRoute, RoutePoolEntry, ServiceEntitySource, ServiceEvent, ServiceEventKind,
    ServiceState, ServiceTabHandle, SessionCleanupPolicy, TabLifecycle, ViewStream,
    ViewStreamProvider, ViewerLease,
};
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use crate::native::snapshot::{self, SnapshotOptions};
use crate::native::state;
use crate::native::stream_runtime::{
    stream_file_path, write_engine_file, write_extensions_file, write_provider_file,
};
use crate::native::webdriver::backend::BrowserBackend;
use crate::runtime_profile::{
    clear_runtime_state, looks_like_path, read_devtools_port, read_runtime_state,
    runtime_profile_user_data_dir,
};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
pub(crate) async fn handle_navigate(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let cancellation = state.current_cancellation.clone();
    let url = cmd
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'url' parameter")?;
    {
        let df = state.domain_filter.read().await;
        if let Some(ref filter) = *df {
            filter.check_url(url)?;
        }
    }
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            state.ref_map.clear();
            cancellable(wb.navigate(url), cancellation.clone()).await?;
            let new_url = cancellable(wb.get_url(), cancellation.clone())
                .await
                .unwrap_or_else(|_| url.to_string());
            let title = cancellable(wb.get_title(), cancellation.clone())
                .await
                .unwrap_or_default();
            let mut data = json!({ "url" : new_url, "title" : title });
            add_manual_login_hint_warning(cmd, &mut data);
            return Ok(data);
        }
    }
    let pending_shared_profile_acquisition = state.pending_shared_profile_acquisition.take();
    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
    let wait_until = cmd
        .get("waitUntil")
        .and_then(|v| v.as_str())
        .map(WaitUntil::from_str)
        .unwrap_or(WaitUntil::Load);
    let scoped_headers = cmd
        .get("headers")
        .and_then(|v| v.as_object())
        .filter(|m| !m.is_empty());
    if let Some(headers_map) = scoped_headers {
        if let Some(origin) = url::Url::parse(url)
            .ok()
            .map(|u| u.origin().ascii_serialization())
        {
            let headers: HashMap<String, String> = headers_map
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect();
            let first_origin_header = {
                let mut map = state.origin_headers.write().await;
                let first = map.is_empty();
                map.insert(origin, headers);
                first
            };
            if first_origin_header {
                let session_id = mgr.active_session_id()?.to_string();
                let has_proxy_creds = state.proxy_credentials.read().await.is_some();
                let mut params = json!({ "patterns" : [{ "urlPattern" : "*" }] });
                if has_proxy_creds {
                    params["handleAuthRequests"] = json!(true);
                }
                cancellable(
                    mgr.client
                        .send_command("Fetch.enable", Some(params), Some(&session_id)),
                    cancellation.clone(),
                )
                .await?;
            }
        }
    }
    state.ref_map.clear();
    state.iframe_sessions.clear();
    state.active_frame_id = None;
    let mut data = cancellable(mgr.navigate(url, wait_until), cancellation).await?;
    if let (Some(object), Some(shared_acquisition)) = (
        data.as_object_mut(),
        pending_shared_profile_acquisition.as_ref(),
    ) {
        object.insert("sharedAcquisition".to_string(), shared_acquisition.clone());
    }
    add_manual_login_hint_warning(cmd, &mut data);
    persist_service_owned_navigate_tab(cmd, &state.session_id, mgr, &data)?;
    Ok(data)
}
pub(crate) fn read_runtime_handoff(session_name: &str) -> Result<RuntimeHandoffDescriptor, String> {
    let path = runtime_handoff_path(session_name);
    let payload = fs::read(&path).map_err(|error| {
        format!(
            "No prepared runtime handoff is available for session '{}': {}",
            session_name, error
        )
    })?;
    serde_json::from_slice(&payload).map_err(|error| {
        format!(
            "Runtime handoff for session '{}' is invalid: {}",
            session_name, error
        )
    })
}
pub(crate) fn current_service_browser_host(session_name: &str) -> ServiceBrowserHost {
    LockedServiceStateRepository::default_json()
        .ok()
        .and_then(|repository| repository.load_snapshot().ok())
        .and_then(|service_state| {
            service_state
                .browsers
                .get(&service_browser_id(session_name))
                .map(|browser| browser.host)
        })
        .unwrap_or(ServiceBrowserHost::AttachedExisting)
}
pub(crate) async fn handle_runtime_handoff_prepare(
    state: &mut DaemonState,
) -> Result<Value, String> {
    let Some(manager) = state.browser.as_mut() else {
        let path = runtime_handoff_path(&state.session_id);
        let _ = fs::remove_file(path);
        return Ok(json!(
            { "prepared" : false, "browserPresent" : false, "sessionName" : state
            .session_id, }
        ));
    };
    if !manager.is_connection_alive().await {
        return Err(format!(
            "Cannot prepare runtime handoff for session '{}': browser CDP connection is not alive",
            state.session_id
        ));
    }
    let descriptor = RuntimeHandoffDescriptor {
        schema_version: 1,
        session_name: state.session_id.clone(),
        cdp_url: manager.get_cdp_url().to_string(),
        browser_pid: manager.browser_pid().or(state.attached_browser_pid),
        runtime_profile: manager
            .runtime_profile_name()
            .map(str::to_string)
            .or_else(|| state.attached_runtime_profile.clone()),
        engine: state.engine.clone(),
        host: current_service_browser_host(&state.session_id),
        close_browser_on_close: state.close_behavior == CloseBehavior::CloseBrowser,
        active_target_id: manager.active_target_id().ok().map(str::to_string),
        prepared_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| OffsetDateTime::now_utc().unix_timestamp().to_string()),
    };
    let path = write_runtime_handoff(&descriptor)?;
    manager.relinquish_browser_for_handoff();
    state.browser = None;
    state.screencasting = false;
    state.reset_input_state();
    state.update_stream_client().await;
    Ok(json!(
        { "prepared" : true, "browserPresent" : true, "sessionName" : descriptor
        .session_name, "browserPid" : descriptor.browser_pid, "cdpUrl" : descriptor
        .cdp_url, "runtimeProfile" : descriptor.runtime_profile, "handoffPath" :
        path, }
    ))
}
pub(crate) async fn handle_runtime_handoff_resume(
    state: &mut DaemonState,
) -> Result<Value, String> {
    if state.browser.is_some() {
        return Err(format!(
            "Cannot resume runtime handoff for session '{}': daemon already has a browser",
            state.session_id
        ));
    }
    let descriptor = read_runtime_handoff(&state.session_id)?;
    if descriptor.schema_version != 1 || descriptor.session_name != state.session_id {
        return Err(format!(
            "Runtime handoff identity mismatch for session '{}'",
            state.session_id
        ));
    }
    if let Some(browser_pid) = descriptor.browser_pid {
        let assessment = crate::runtime_profile::runtime_process_assessment(
            descriptor.runtime_profile.as_deref(),
            browser_pid,
        );
        if !assessment.authorizes_adoption() {
            return Err(format!(
                "Runtime handoff browser PID no longer matches the recorded browser for session '{}' ({})",
                state.session_id, assessment.reason
            ));
        }
    }
    let manager = BrowserManager::connect_cdp_for_handoff(
        &descriptor.cdp_url,
        descriptor.active_target_id.as_deref(),
    )
    .await?;
    state.reset_input_state();
    state.attached_runtime_profile = descriptor.runtime_profile.clone();
    state.attached_browser_pid = descriptor.browser_pid;
    state.close_behavior = if descriptor.close_browser_on_close {
        CloseBehavior::CloseBrowser
    } else {
        CloseBehavior::Detach
    };
    state.engine = descriptor.engine.clone();
    write_engine_file(&state.session_id, &state.engine);
    state.browser = Some(manager);
    state.subscribe_to_browser_events();
    state.start_fetch_handler();
    state.start_dialog_handler();
    state.update_stream_client().await;
    persist_current_browser_health(state, descriptor.host, ServiceBrowserHealth::Ready, None);
    let retry_record_removed = fs::remove_file(runtime_handoff_path(&state.session_id)).is_ok();
    Ok(json!(
        { "resumed" : true, "sessionName" : descriptor.session_name, "browserPid" :
        descriptor.browser_pid, "cdpUrl" : descriptor.cdp_url, "runtimeProfile" :
        descriptor.runtime_profile, "activeTargetId" : state.browser.as_ref()
        .and_then(| browser | browser.active_target_id().ok()), "retryRecordRemoved"
        : retry_record_removed, "targetsReattached" : state.browser.as_ref()
        .map(BrowserManager::page_count).unwrap_or(0), }
    ))
}
pub(crate) async fn handle_close(state: &mut DaemonState) -> Result<Value, String> {
    let attached_runtime_profile = state.attached_runtime_profile.take();
    let attached_browser_pid = state.attached_browser_pid.take();
    let close_behavior = std::mem::take(&mut state.close_behavior);
    let mut shutdown_outcome = BrowserShutdownOutcome::default();
    if let Some(ref mgr) = state.browser {
        if let Some(ref session_name) = state.session_name {
            if let Ok(session_id) = mgr.active_session_id() {
                let tracked_origins = state
                    .tracked_origin_storage
                    .values()
                    .cloned()
                    .collect::<Vec<_>>();
                let _ = state::save_state(
                    &mgr.client,
                    session_id,
                    None,
                    Some(session_name.as_str()),
                    &state.session_id,
                    mgr.visited_origins(),
                    &tracked_origins,
                )
                .await;
            }
        }
    }
    if let Some(ref mut mgr) = state.browser {
        let runtime_profile = mgr.runtime_profile_name().map(str::to_string);
        if (attached_runtime_profile.is_some() || attached_browser_pid.is_some())
            && close_behavior == CloseBehavior::CloseBrowser
        {
            let _ = mgr
                .client
                .send_command_no_params("Browser.close", None)
                .await;
        }
        if close_behavior == CloseBehavior::Detach && runtime_profile.is_some() {
            mgr.detach_runtime_browser()?;
        } else {
            let outcome = mgr.close_with_outcome().await?;
            shutdown_outcome.polite_close_attempted |= outcome.polite_close_attempted;
            shutdown_outcome.polite_close_succeeded |= outcome.polite_close_succeeded;
            shutdown_outcome.polite_close_failed |= outcome.polite_close_failed;
            shutdown_outcome.force_kill_attempted |= outcome.force_kill_attempted;
            shutdown_outcome.force_kill_succeeded |= outcome.force_kill_succeeded;
            shutdown_outcome.force_kill_failed |= outcome.force_kill_failed;
            shutdown_outcome.errors.extend(outcome.errors);
            if let Some(runtime_profile) = runtime_profile {
                if attached_runtime_profile.as_deref() != Some(runtime_profile.as_str())
                    && browser_shutdown_confirmed(&shutdown_outcome)
                {
                    let _ = clear_runtime_state(&runtime_profile);
                }
            }
        }
    }
    if let Some(runtime_profile) = attached_runtime_profile.as_ref() {
        if close_behavior == CloseBehavior::CloseBrowser {
            let pid = attached_browser_pid.or_else(|| runtime_profile_pid(Some(runtime_profile)));
            if let Some(pid) = pid {
                let outcome = terminate_runtime_browser(Some(runtime_profile.clone()), pid).await;
                shutdown_outcome.polite_close_attempted |= outcome.polite_close_attempted;
                shutdown_outcome.polite_close_succeeded |= outcome.polite_close_succeeded;
                shutdown_outcome.polite_close_failed |= outcome.polite_close_failed;
                shutdown_outcome.force_kill_attempted |= outcome.force_kill_attempted;
                shutdown_outcome.force_kill_succeeded |= outcome.force_kill_succeeded;
                shutdown_outcome.force_kill_failed |= outcome.force_kill_failed;
                shutdown_outcome.errors.extend(outcome.errors);
            }
            if browser_shutdown_confirmed(&shutdown_outcome) {
                let _ = clear_runtime_state(runtime_profile);
            }
        }
    } else if close_behavior == CloseBehavior::CloseBrowser {
        if let Some(pid) = attached_browser_pid {
            let outcome = terminate_runtime_browser(None, pid).await;
            shutdown_outcome.polite_close_attempted |= outcome.polite_close_attempted;
            shutdown_outcome.polite_close_succeeded |= outcome.polite_close_succeeded;
            shutdown_outcome.polite_close_failed |= outcome.polite_close_failed;
            shutdown_outcome.force_kill_attempted |= outcome.force_kill_attempted;
            shutdown_outcome.force_kill_succeeded |= outcome.force_kill_succeeded;
            shutdown_outcome.force_kill_failed |= outcome.force_kill_failed;
            shutdown_outcome.errors.extend(outcome.errors);
        }
    }
    state.browser = None;
    if close_behavior == CloseBehavior::CloseBrowser
        && !browser_shutdown_confirmed(&shutdown_outcome)
    {
        state.attached_runtime_profile = attached_runtime_profile;
        state.attached_browser_pid = attached_browser_pid;
        state.close_behavior = CloseBehavior::CloseBrowser;
    }
    state.launch_hash = None;
    state.screencasting = false;
    state.reset_input_state();
    state.update_stream_client().await;
    persist_closed_browser_health(state, Some(&shutdown_outcome));
    if let Some(task) = state.fetch_handler_task.take() {
        task.abort();
    }
    {
        let mut map = state.origin_headers.write().await;
        map.clear();
    }
    if let Some(ref mut wb) = state.webdriver_backend {
        let _ = wb.close().await;
    }
    state.webdriver_backend = None;
    if let Some(ref mut appium) = state.appium {
        let _ = appium.close().await;
    }
    state.appium = None;
    if let Some(ref mut driver) = state.safari_driver {
        driver.kill();
    }
    state.safari_driver = None;
    state.backend_type = BackendType::Cdp;
    if let Some(server) = state.inspect_server.take() {
        server.shutdown();
    }
    state.ref_map.clear();
    Ok(json!({ "closed" : true }))
}

fn browser_shutdown_confirmed(outcome: &BrowserShutdownOutcome) -> bool {
    outcome.errors.is_empty() && !outcome.polite_close_failed && !outcome.force_kill_failed
}
pub(crate) async fn handle_snapshot(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let cancellation = state.current_cancellation.clone();
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let options = SnapshotOptions {
        selector: cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .map(String::from),
        interactive: cmd
            .get("interactive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        compact: cmd
            .get("compact")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        depth: cmd
            .get("maxDepth")
            .and_then(|v| v.as_u64())
            .map(|d| d as usize),
        urls: cmd.get("urls").and_then(|v| v.as_bool()).unwrap_or(false),
    };
    state.ref_map.clear();
    let tree = cancellable(
        snapshot::take_snapshot(
            &mgr.client,
            &session_id,
            &options,
            &mut state.ref_map,
            state.active_frame_id.as_deref(),
            &state.iframe_sessions,
        ),
        cancellation.clone(),
    )
    .await?;
    let url = cancellable(mgr.get_url(), cancellation)
        .await
        .unwrap_or_default();
    let refs: serde_json::Map<String, Value> = state
        .ref_map
        .entries_sorted()
        .into_iter()
        .map(|(ref_id, entry)| {
            let mut obj = serde_json::Map::new();
            obj.insert("role".into(), Value::String(entry.role));
            obj.insert("name".into(), Value::String(entry.name));
            (ref_id, Value::Object(obj))
        })
        .collect();
    Ok(json!({ "snapshot" : tree, "origin" : url, "refs" : refs }))
}
pub(crate) fn runtime_handoff_path(session_name: &str) -> PathBuf {
    get_socket_dir().join(format!("{}.handoff.json", session_name))
}
pub(crate) fn write_runtime_handoff(
    descriptor: &RuntimeHandoffDescriptor,
) -> Result<PathBuf, String> {
    let path = runtime_handoff_path(&descriptor.session_name);
    let parent = path
        .parent()
        .ok_or_else(|| format!("Runtime handoff path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create runtime handoff directory {}: {}",
            parent.display(),
            error
        )
    })?;
    let staged = path.with_extension(format!("handoff.json.next-{}", std::process::id()));
    let payload = serde_json::to_vec_pretty(descriptor)
        .map_err(|error| format!("Failed to serialize runtime handoff: {}", error))?;
    fs::write(&staged, payload).map_err(|error| {
        format!(
            "Failed to stage runtime handoff {}: {}",
            staged.display(),
            error
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o600)).map_err(|error| {
            format!(
                "Failed to secure runtime handoff {}: {}",
                staged.display(),
                error
            )
        })?;
    }
    if path.exists() {
        fs::remove_file(&path).map_err(|error| {
            format!(
                "Failed to replace runtime handoff {}: {}",
                path.display(),
                error
            )
        })?;
    }
    fs::rename(&staged, &path).map_err(|error| {
        format!(
            "Failed to publish runtime handoff {}: {}",
            path.display(),
            error
        )
    })?;
    Ok(path)
}
