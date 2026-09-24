#![allow(unused_imports)]
use super::capability::service_browser_id;
use super::daemon::{BackendType, CloseBehavior};
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
use crate::native::service_challenge_task::{
    admit_challenge_consumer, NAVIGATION_CHALLENGE_INTENT_ID,
};
use crate::native::service_model::{
    retained_display_allocation_candidates, service_profile_allocations,
    service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
    BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
    BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession, BrowserTab,
    ControlInputProvider, DisplayAllocation, JobState as ServiceJobState, LeaseState, MonitorState,
    ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy, ProfileLeaseDisposition,
    ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease, RemoteViewHandoff,
    RemoteViewRoute, RoutePoolEntry, ServiceBrowserProcessIdentity, ServiceEntitySource,
    ServiceEvent, ServiceEventKind, ServiceState, ServiceTabHandle, SessionCleanupPolicy,
    TabLifecycle, ViewStream, ViewStreamProvider,
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
use agent_browser_challenge_control::ChallengeConsumerKind;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const RUNTIME_HANDOFF_SERVICE_STATE_LOCK_TIMEOUT: Duration = Duration::from_secs(30);

fn required_navigation_challenge_string(command: &Value, field: &str) -> Result<String, String> {
    command
        .get(field)
        .or_else(|| command.get("params").and_then(|params| params.get(field)))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("navigation_challenge_{field}_required"))
}

fn navigation_challenge_admission_in_state(
    command: &Value,
    service_state: &ServiceState,
) -> Result<Option<Value>, String> {
    let challenge_task_id = command
        .get("challengeTaskId")
        .or_else(|| {
            command
                .get("params")
                .and_then(|params| params.get("challengeTaskId"))
        })
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(challenge_task_id) = challenge_task_id else {
        return Ok(None);
    };
    let principal_id = command
        .get("clientSubjectId")
        .or_else(|| command.get("callerId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "navigation_challenge_principal_required".to_string())?;
    let site_policy_id = required_navigation_challenge_string(command, "sitePolicyId")?;
    let operation_id = required_navigation_challenge_string(command, "operationId")?;
    let supplied_handle: ServiceTabHandle = serde_json::from_value(
        command
            .get("serviceTabHandle")
            .or_else(|| {
                command
                    .get("params")
                    .and_then(|params| params.get("serviceTabHandle"))
            })
            .cloned()
            .ok_or_else(|| "navigation_challenge_service_tab_handle_required".to_string())?,
    )
    .map_err(|error| format!("navigation_challenge_service_tab_handle_invalid:{error}"))?;
    serde_json::to_value(admit_challenge_consumer(
        service_state,
        challenge_task_id,
        principal_id,
        &supplied_handle,
        &site_policy_id,
        NAVIGATION_CHALLENGE_INTENT_ID,
        &operation_id,
        ChallengeConsumerKind::Navigation,
    )?)
    .map(Some)
    .map_err(|error| format!("navigation_challenge_admission_invalid:{error}"))
}

pub(crate) fn navigation_challenge_admission(command: &Value) -> Result<Option<Value>, String> {
    if command.get("challengeTaskId").is_none()
        && command
            .get("params")
            .and_then(|params| params.get("challengeTaskId"))
            .is_none()
    {
        return Ok(None);
    }
    let repository = LockedServiceStateRepository::default_json()?;
    navigation_challenge_admission_in_state(command, &repository.load_snapshot()?)
}

/// Upgrade handoffs share the durable service-state lock with active runtimes.
/// Their owner transfer is bounded by the outer transaction, so tolerate a
/// short writer burst instead of failing at the ordinary interactive budget.
fn runtime_handoff_service_repository(
) -> Result<LockedServiceStateRepository<crate::native::service_store::JsonServiceStateStore>, String>
{
    Ok(LockedServiceStateRepository::default_json()?
        .with_lock_timeout(RUNTIME_HANDOFF_SERVICE_STATE_LOCK_TIMEOUT))
}
pub(crate) async fn handle_navigate(
    cmd: &Value,
    state: &mut DaemonState,
    challenge_admission: Option<Value>,
) -> Result<Value, String> {
    let cancellation = state.current_cancellation.clone();
    let url = cmd
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'url' parameter")?;
    let challenge_requested = cmd.get("challengeTaskId").is_some()
        || cmd
            .get("params")
            .and_then(|params| params.get("challengeTaskId"))
            .is_some();
    if challenge_requested && challenge_admission.is_none() {
        return Err("navigation_challenge_admission_missing".to_string());
    }
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
    let runtime_owner_browser_id =
        crate::native::action_runtime::runtime::service_tab_handle_browser_id(state);
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
    let navigation_session = mgr.active_session_id()?.to_string();
    let navigation = cancellable(mgr.navigate(url, wait_until), cancellation.clone()).await;
    if navigation.is_err()
        && cancellation
            .as_ref()
            .is_some_and(crate::native::cancellation::CancellationToken::is_cancelled)
    {
        let _ = mgr
            .client
            .send_command(
                "Page.stopLoading",
                Some(json!({})),
                Some(&navigation_session),
            )
            .await;
        let _ = mgr
            .client
            .send_command(
                "Page.navigate",
                Some(json!({ "url": "about:blank" })),
                Some(&navigation_session),
            )
            .await;
    }
    let mut data = navigation?;
    if let (Some(object), Some(shared_acquisition)) = (
        data.as_object_mut(),
        pending_shared_profile_acquisition.as_ref(),
    ) {
        object.insert("sharedAcquisition".to_string(), shared_acquisition.clone());
    }
    add_manual_login_hint_warning(cmd, &mut data);
    persist_service_owned_navigate_tab(
        cmd,
        &state.session_id,
        mgr,
        &data,
        Some(&runtime_owner_browser_id),
    )?;
    Ok(data)
}

pub(crate) async fn handle_close(state: &mut DaemonState) -> Result<Value, String> {
    handle_close_with_context(state, false).await
}

pub(crate) async fn handle_recovery_close(state: &mut DaemonState) -> Result<Value, String> {
    handle_close_with_context(state, true).await
}

async fn handle_close_with_context(
    state: &mut DaemonState,
    preserve_registered_work: bool,
) -> Result<Value, String> {
    let attached_runtime_profile = state.attached_runtime_profile.take();
    let attached_browser_pid = state.attached_browser_pid.take();
    let close_behavior = std::mem::take(&mut state.close_behavior);
    let mut shutdown_outcome = BrowserShutdownOutcome::default();
    if let Some(manager) = state.browser.as_mut() {
        if close_behavior == CloseBehavior::Detach && manager.runtime_profile_name().is_some() {
            manager.detach_runtime_browser()?;
        } else {
            shutdown_outcome = manager.close_with_outcome().await?;
        }
    }
    if close_behavior == CloseBehavior::CloseBrowser {
        if let Some(pid) = attached_browser_pid {
            let outcome = terminate_runtime_browser(attached_runtime_profile.clone(), pid).await;
            shutdown_outcome.polite_close_attempted |= outcome.polite_close_attempted;
            shutdown_outcome.polite_close_succeeded |= outcome.polite_close_succeeded;
            shutdown_outcome.polite_close_failed |= outcome.polite_close_failed;
            shutdown_outcome.exact_process_exited |= outcome.exact_process_exited;
            shutdown_outcome.profile_lock_released |= outcome.profile_lock_released;
            shutdown_outcome.force_kill_attempted |= outcome.force_kill_attempted;
            shutdown_outcome.force_kill_succeeded |= outcome.force_kill_succeeded;
            shutdown_outcome.force_kill_failed |= outcome.force_kill_failed;
            shutdown_outcome.errors.extend(outcome.errors);
        }
        if let Some(profile) = attached_runtime_profile.as_deref() {
            if browser_shutdown_confirmed(&shutdown_outcome) {
                let _ = clear_runtime_state(profile);
            }
        }
    }
    state.browser = None;
    state.launch_hash = None;
    state.screencasting = false;
    state.reset_input_state();
    state.update_stream_client().await;
    if preserve_registered_work {
        super::recovery::persist_recovery_closed_browser_health(state, Some(&shutdown_outcome));
    } else {
        persist_closed_browser_health(state, Some(&shutdown_outcome));
    }
    if let Some(task) = state.fetch_handler_task.take() {
        task.abort();
    }
    state.origin_headers.write().await.clear();
    if let Some(backend) = state.webdriver_backend.as_mut() {
        let _ = backend.close().await;
    }
    state.webdriver_backend = None;
    if let Some(appium) = state.appium.as_mut() {
        let _ = appium.close().await;
    }
    state.appium = None;
    if let Some(driver) = state.safari_driver.as_mut() {
        driver.kill();
    }
    state.safari_driver = None;
    state.backend_type = BackendType::Cdp;
    if let Some(server) = state.inspect_server.take() {
        server.shutdown();
    }
    state.ref_map.clear();
    Ok(json!({ "closed": true }))
}

fn browser_shutdown_confirmed(outcome: &BrowserShutdownOutcome) -> bool {
    outcome.errors.is_empty() && !outcome.polite_close_failed && !outcome.force_kill_failed
}

pub(crate) fn browser_terminal_evidence(outcome: &BrowserShutdownOutcome) -> Option<Vec<String>> {
    (outcome.exact_process_exited && outcome.profile_lock_released && !outcome.force_kill_failed)
        .then(|| {
            vec![
                "exact_process_exited".to_string(),
                "profile_lock_released".to_string(),
            ]
        })
}

pub(crate) async fn handle_snapshot(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let cancellation = state.current_cancellation.clone();
    let manager = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = manager.active_session_id()?.to_string();
    let options = SnapshotOptions {
        selector: cmd
            .get("selector")
            .and_then(Value::as_str)
            .map(String::from),
        interactive: cmd
            .get("interactive")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        compact: cmd.get("compact").and_then(Value::as_bool).unwrap_or(false),
        depth: cmd
            .get("maxDepth")
            .and_then(Value::as_u64)
            .map(|depth| depth as usize),
        urls: cmd.get("urls").and_then(Value::as_bool).unwrap_or(false),
    };
    state.ref_map.clear();
    let tree = cancellable(
        snapshot::take_snapshot(
            &manager.client,
            &session_id,
            &options,
            &mut state.ref_map,
            state.active_frame_id.as_deref(),
            &state.iframe_sessions,
        ),
        cancellation.clone(),
    )
    .await?;
    let url = cancellable(manager.get_url(), cancellation)
        .await
        .unwrap_or_default();
    let refs = state
        .ref_map
        .entries_sorted()
        .into_iter()
        .map(|(ref_id, entry)| (ref_id, json!({ "role": entry.role, "name": entry.name })))
        .collect::<serde_json::Map<String, Value>>();
    Ok(json!({ "snapshot": tree, "origin": url, "refs": refs }))
}
