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
use crate::native::runtime_lifecycle::{RuntimeLifecycleAuthority, RuntimeLifecycleIntent};
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
    TabLifecycle, ViewStream, ViewStreamProvider, ViewerLease,
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
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const RUNTIME_HANDOFF_SERVICE_STATE_LOCK_TIMEOUT: Duration = Duration::from_secs(30);

/// Upgrade handoffs share the durable service-state lock with active runtimes.
/// Their owner transfer is bounded by the outer transaction, so tolerate a
/// short writer burst instead of failing at the ordinary interactive budget.
fn runtime_handoff_service_repository(
) -> Result<LockedServiceStateRepository<crate::native::service_store::JsonServiceStateStore>, String>
{
    Ok(LockedServiceStateRepository::default_json()?
        .with_lock_timeout(RUNTIME_HANDOFF_SERVICE_STATE_LOCK_TIMEOUT))
}
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
    if let Ok(descriptor) = read_runtime_handoff(&state.session_id) {
        if descriptor.schema_version == 2 {
            let proposal = descriptor.owner_transfer.as_ref().ok_or_else(|| {
                "runtime_handoff_owner_transfer_missing: prepared descriptor has no owner proposal"
                    .to_string()
            })?;
            let repository = runtime_handoff_service_repository()?;
            let mut current_owner = repository
                .load_snapshot()?
                .runtime_owner_registry
                .owner(&proposal.request.profile_identity_digest)
                .cloned()
                .ok_or_else(|| {
                    "runtime_handoff_owner_missing: prepared owner is not registered".to_string()
                })?;
            if current_owner.pending_transfer.as_ref() != Some(proposal) {
                let current_pid = manager
                    .browser_pid()
                    .or(state.attached_browser_pid)
                    .ok_or_else(|| {
                        "runtime_handoff_prepare_stale_retry_process_unavailable".to_string()
                    })?;
                let current_profile = manager
                    .runtime_profile_name()
                    .map(str::to_string)
                    .or_else(|| state.attached_runtime_profile.clone())
                    .ok_or_else(|| {
                        "runtime_handoff_prepare_stale_retry_profile_unavailable".to_string()
                    })?;
                let current_process_identity =
                    crate::process_identity::capture_process_identity(current_pid, None, None)
                        .ok_or_else(|| {
                            "runtime_handoff_prepare_stale_retry_process_identity_unavailable"
                                .to_string()
                        })?;
                let current_profile_identity_digest =
                    runtime_handoff_profile_digest(&current_profile)?;
                let current_process_instance_digest =
                    runtime_handoff_json_digest(&current_process_identity)?;
                let current_cdp_endpoint_identity_digest =
                    runtime_handoff_digest(manager.get_cdp_url());
                let current_target_set_digest = runtime_handoff_target_set_digest(manager)?;
                let descriptor_process_retired = descriptor.browser_pid.is_some_and(|pid| {
                    matches!(
                        runtime_handoff_process_assessment(&descriptor, pid).ownership,
                        crate::process_identity::RuntimeProcessOwnership::Missing
                            | crate::process_identity::RuntimeProcessOwnership::ReusedUnrelated
                    )
                });
                let mut current_binding =
                    crate::runtime_owner_transfer::RuntimeOwnerBinding::effect_capable(
                        crate::runtime_owner_transfer::OwnerAuthorityClaim::from_owner(
                            &current_owner,
                        ),
                    );
                let authority = RuntimeLifecycleAuthority::new(&repository);
                let current_lifecycle_authorized =
                    authority.authorize_effect(&mut current_binding).is_ok();
                let reversed_retry = reversed_handoff_retry_matches_current_owner(
                    &descriptor.session_name,
                    proposal,
                    &current_owner,
                );
                if !reversed_retry
                    && current_lifecycle_authorized
                    && descriptor_process_retired
                    && stale_handoff_current_owner_refresh_allowed(
                        &descriptor.session_name,
                        proposal,
                        &current_owner,
                        &current_profile_identity_digest,
                        &current_process_instance_digest,
                        &state.engine,
                    )
                {
                    current_owner = match authority.transition(
                        RuntimeLifecycleIntent::RefreshCurrentOwnerEvidence {
                            claim: current_binding.claim.clone(),
                            cdp_endpoint_identity_digest:
                                current_cdp_endpoint_identity_digest.clone(),
                            target_set_digest: current_target_set_digest.clone(),
                        },
                    )? {
                        crate::native::runtime_lifecycle::RuntimeLifecycleTransition::OwnerEvidenceRefreshed(
                            owner,
                        ) => owner,
                        _ => {
                            return Err(
                                "runtime_handoff_stale_owner_refresh_outcome_mismatch".to_string(),
                            )
                        }
                    };
                    current_binding =
                        crate::runtime_owner_transfer::RuntimeOwnerBinding::effect_capable(
                            crate::runtime_owner_transfer::OwnerAuthorityClaim::from_owner(
                                &current_owner,
                            ),
                        );
                    authority.authorize_effect(&mut current_binding)?;
                }
                if reversed_retry
                    || (current_lifecycle_authorized
                        && stale_handoff_retry_matches_current_owner(
                            &descriptor.session_name,
                            proposal,
                            &current_owner,
                            &current_profile_identity_digest,
                            &current_process_instance_digest,
                            &state.engine,
                            &current_cdp_endpoint_identity_digest,
                            &current_target_set_digest,
                            descriptor_process_retired,
                        ))
                {
                    fs::remove_file(runtime_handoff_path(&state.session_id)).map_err(|error| {
                        format!("runtime_handoff_prepare_stale_retry_cleanup_failed: {error}")
                    })?;
                    state.runtime_owner_binding = Some(current_binding);
                } else {
                    return Err(
                        "runtime_handoff_prepare_replay_mismatch: descriptor and owner registry differ"
                            .to_string(),
                    );
                }
            } else {
                state.runtime_owner_binding = Some(
                    crate::runtime_owner_transfer::RuntimeOwnerBinding::effect_capable(
                        crate::runtime_owner_transfer::OwnerAuthorityClaim::from_owner(
                            &current_owner,
                        ),
                    ),
                );
                return Ok(runtime_handoff_prepared_response(
                    &descriptor,
                    runtime_handoff_path(&state.session_id),
                    true,
                ));
            }
        }
    }
    let browser_pid = manager
        .browser_pid()
        .or(state.attached_browser_pid)
        .ok_or_else(|| {
            format!(
                "Cannot prepare runtime handoff for session '{}': browser PID is unavailable",
                state.session_id
            )
        })?;
    let runtime_profile = manager
        .runtime_profile_name()
        .map(str::to_string)
        .or_else(|| state.attached_runtime_profile.clone())
        .ok_or_else(|| {
            format!(
                "Cannot prepare runtime handoff for session '{}': canonical runtime profile is unavailable",
                state.session_id
            )
        })?;
    let process_identity = crate::process_identity::capture_process_identity(browser_pid, None, None)
        .ok_or_else(|| {
            format!(
                "Cannot prepare runtime handoff for session '{}': browser process identity is unavailable",
                state.session_id
            )
        })?;
    let profile_identity_digest = runtime_handoff_profile_digest(&runtime_profile)?;
    let process_instance_digest = runtime_handoff_json_digest(&process_identity)?;
    let cdp_endpoint_identity_digest = runtime_handoff_digest(manager.get_cdp_url());
    let target_set_digest = runtime_handoff_target_set_digest(manager)?;
    let active_target_id = manager.active_target_id()?.to_string();
    let selected_target_identity_digest = runtime_handoff_digest(&active_target_id);
    let prepared_at = runtime_handoff_timestamp();
    let owner_id = format!(
        "owner-{}",
        &runtime_handoff_digest(&format!(
            "{}:{profile_identity_digest}:{process_instance_digest}",
            state.session_id
        ))[..20]
    );
    let repository = runtime_handoff_service_repository()?;
    if let Some(binding) = state.runtime_owner_binding.as_mut() {
        let authority = RuntimeLifecycleAuthority::new(&repository);
        authority.authorize_effect(binding)?;
        match authority.transition(RuntimeLifecycleIntent::RefreshCurrentOwnerEvidence {
            claim: binding.claim.clone(),
            cdp_endpoint_identity_digest: cdp_endpoint_identity_digest.clone(),
            target_set_digest: target_set_digest.clone(),
        })? {
            crate::native::runtime_lifecycle::RuntimeLifecycleTransition::OwnerEvidenceRefreshed(
                _,
            ) => {}
            _ => return Err("runtime_handoff_owner_refresh_outcome_mismatch".to_string()),
        }
    }
    let current_owner = if let Some(owner) = repository
        .load_snapshot()?
        .runtime_owner_registry
        .owner(&profile_identity_digest)
        .cloned()
    {
        if !current_owner_matches_preparing_daemon(
            &owner,
            state.runtime_owner_binding.as_ref(),
            &owner_id,
            &service_browser_id(&state.session_id),
            &state.session_id,
            &process_instance_digest,
            &state.engine,
            &cdp_endpoint_identity_digest,
            &target_set_digest,
        ) {
            return Err("runtime_owner_current_evidence_mismatch: existing profile owner does not match the preparing daemon".to_string());
        }
        owner
    } else {
        crate::native::runtime_lifecycle::RuntimeLifecycleAuthority::new(&repository)
            .register_current_owner(crate::runtime_owner_transfer::ProfileOwner {
                owner_id: owner_id.clone(),
                profile_identity_digest: profile_identity_digest.clone(),
                state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
                owner_generation: 1,
                browser_id: service_browser_id(&state.session_id),
                daemon_session_route: state.session_id.clone(),
                process_instance_digest: process_instance_digest.clone(),
                browser_family: state.engine.clone(),
                cdp_endpoint_identity_digest: cdp_endpoint_identity_digest.clone(),
                target_set_digest: target_set_digest.clone(),
                pending_transfer: None,
                last_transition: None,
            })?
    };
    let transfer_nonce_digest = runtime_handoff_digest(&format!(
        "{}:{prepared_at}:{process_instance_digest}:{target_set_digest}",
        state.session_id
    ));
    let candidate_session = format!("handoff-{}", &transfer_nonce_digest[..16]);
    let proposal = crate::native::runtime_lifecycle::RuntimeLifecycleAuthority::new(&repository)
        .begin_transfer(crate::runtime_owner_transfer::OwnerTransferRequest {
            mode: crate::runtime_adoption::BrowserAdoptionMode::CooperativeTransfer,
            logical_browser_id: current_owner.browser_id.clone(),
            profile_identity_digest: profile_identity_digest.clone(),
            expected_owner_id: Some(current_owner.owner_id.clone()),
            expected_owner_generation: current_owner.owner_generation,
            candidate_owner_id: format!("owner-{}", &transfer_nonce_digest[16..36]),
            candidate_daemon_session_route: candidate_session.clone(),
            process_instance_digest,
            browser_family: state.engine.clone(),
            cdp_endpoint_identity_digest,
            target_set_digest,
            selected_target_identity_digest,
            transfer_nonce_digest,
        })?;
    state.runtime_owner_binding = Some(
        crate::runtime_owner_transfer::RuntimeOwnerBinding::effect_capable(
            crate::runtime_owner_transfer::OwnerAuthorityClaim::from_owner(&current_owner),
        ),
    );
    let descriptor = RuntimeHandoffDescriptor {
        schema_version: 2,
        session_name: state.session_id.clone(),
        cdp_url: manager.get_cdp_url().to_string(),
        browser_pid: Some(browser_pid),
        runtime_profile: Some(runtime_profile),
        process_identity: Some(process_identity),
        engine: state.engine.clone(),
        host: current_service_browser_host(&state.session_id),
        close_browser_on_close: state.close_behavior == CloseBehavior::CloseBrowser,
        active_target_id: Some(active_target_id),
        owner_transfer: Some(proposal.clone()),
        prepared_at,
    };
    let path = write_runtime_handoff(&descriptor)?;
    Ok(runtime_handoff_prepared_response(&descriptor, path, false))
}

pub(crate) fn reversed_handoff_retry_matches_current_owner(
    descriptor_session: &str,
    proposal: &crate::runtime_owner_transfer::OwnerTransferProposal,
    current_owner: &crate::runtime_owner_transfer::ProfileOwner,
) -> bool {
    current_owner.pending_transfer.is_none()
        && current_owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Ready
        && proposal
            .candidate_owner_generation
            .checked_add(1)
            .is_some_and(|generation| generation == current_owner.owner_generation)
        && proposal.request.expected_owner_id.as_deref() == Some(current_owner.owner_id.as_str())
        && proposal.request.expected_owner_generation == proposal.previous_owner_generation
        && current_owner.browser_id == proposal.request.logical_browser_id
        && current_owner.daemon_session_route == descriptor_session
        && current_owner.profile_identity_digest == proposal.request.profile_identity_digest
        && current_owner.process_instance_digest == proposal.request.process_instance_digest
        && current_owner.browser_family == proposal.request.browser_family
        && current_owner.cdp_endpoint_identity_digest
            == proposal.request.cdp_endpoint_identity_digest
        && current_owner.target_set_digest == proposal.request.target_set_digest
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn stale_handoff_current_owner_refresh_allowed(
    descriptor_session: &str,
    proposal: &crate::runtime_owner_transfer::OwnerTransferProposal,
    current_owner: &crate::runtime_owner_transfer::ProfileOwner,
    current_profile_identity_digest: &str,
    current_process_instance_digest: &str,
    current_browser_family: &str,
) -> bool {
    current_owner.pending_transfer.is_none()
        && current_owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Ready
        && current_owner.owner_generation > proposal.previous_owner_generation
        && current_owner.browser_id == proposal.request.logical_browser_id
        && current_owner.daemon_session_route == descriptor_session
        && current_owner.profile_identity_digest == proposal.request.profile_identity_digest
        && current_owner.profile_identity_digest == current_profile_identity_digest
        && current_owner.process_instance_digest == current_process_instance_digest
        && current_owner.browser_family == current_browser_family
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn stale_handoff_retry_matches_current_owner(
    descriptor_session: &str,
    proposal: &crate::runtime_owner_transfer::OwnerTransferProposal,
    current_owner: &crate::runtime_owner_transfer::ProfileOwner,
    current_profile_identity_digest: &str,
    current_process_instance_digest: &str,
    current_browser_family: &str,
    current_cdp_endpoint_identity_digest: &str,
    current_target_set_digest: &str,
    descriptor_process_retired: bool,
) -> bool {
    current_owner.pending_transfer.is_none()
        && current_owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Ready
        && current_owner.owner_generation > proposal.previous_owner_generation
        && current_owner.browser_id == proposal.request.logical_browser_id
        && current_owner.daemon_session_route == descriptor_session
        && current_owner.profile_identity_digest == proposal.request.profile_identity_digest
        && current_owner.profile_identity_digest == current_profile_identity_digest
        && current_owner.process_instance_digest == current_process_instance_digest
        && current_owner.browser_family == current_browser_family
        && current_owner.cdp_endpoint_identity_digest == current_cdp_endpoint_identity_digest
        && current_owner.target_set_digest == current_target_set_digest
        && (descriptor_process_retired
            || (proposal.request.expected_owner_id.as_deref()
                == Some(current_owner.owner_id.as_str())
                && current_owner.process_instance_digest
                    == proposal.request.process_instance_digest
                && current_owner.browser_family == proposal.request.browser_family))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn current_owner_matches_preparing_daemon(
    owner: &crate::runtime_owner_transfer::ProfileOwner,
    binding: Option<&crate::runtime_owner_transfer::RuntimeOwnerBinding>,
    route_derived_owner_id: &str,
    route_derived_browser_id: &str,
    daemon_session_route: &str,
    process_instance_digest: &str,
    browser_family: &str,
    cdp_endpoint_identity_digest: &str,
    target_set_digest: &str,
) -> bool {
    let owner_identity_is_current = binding.is_some_and(|binding| {
        binding.effect_capable
            && binding.claim
                == crate::runtime_owner_transfer::OwnerAuthorityClaim::from_owner(owner)
    }) || (owner.owner_id == route_derived_owner_id
        && owner.browser_id == route_derived_browser_id);

    owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Ready
        && owner_identity_is_current
        && owner.daemon_session_route == daemon_session_route
        && owner.process_instance_digest == process_instance_digest
        && owner.browser_family == browser_family
        && owner.cdp_endpoint_identity_digest == cdp_endpoint_identity_digest
        && owner.target_set_digest == target_set_digest
}

pub(crate) async fn handle_runtime_handoff_resume(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let source_session = cmd
        .get("sourceSession")
        .and_then(Value::as_str)
        .unwrap_or(&state.session_id)
        .to_string();
    let logical_browser_id_hint = cmd.get("logicalBrowserId").and_then(Value::as_str);
    if !crate::validation::is_valid_session_name(&source_session) {
        return Err(crate::validation::session_name_error(&source_session));
    }
    if let Some(manager) = state.browser.as_ref() {
        let binding = state.runtime_owner_binding.as_ref().ok_or_else(|| {
            format!(
                "Cannot replay runtime handoff for session '{}': daemon browser is not owner-bound",
                state.session_id
            )
        })?;
        let repository = runtime_handoff_service_repository()?;
        if !crate::runtime_owner_transfer::owner_authority_is_current(&repository, &binding.claim)?
        {
            return Err(format!(
                "Cannot replay runtime handoff for session '{}': owner generation is stale",
                state.session_id
            ));
        }
        return Ok(json!({
            "resumed": true,
            "replayed": true,
            "sourceSessionName": source_session,
            "sessionName": state.session_id,
            "browserPid": state.attached_browser_pid,
            "cdpUrl": manager.get_cdp_url(),
            "runtimeProfile": state.attached_runtime_profile,
            "activeTargetId": manager.active_target_id().ok(),
            "retryRecordRemoved": false,
            "transferState": "candidate_committed",
            "targetsReattached": manager.page_count(),
        }));
    }
    let descriptor_path = runtime_handoff_path(&source_session);
    if !descriptor_path.exists() {
        return handle_runtime_handoff_orphan_adoption(
            &source_session,
            logical_browser_id_hint,
            None,
            state,
        )
        .await;
    }
    let descriptor = read_runtime_handoff(&source_session)?;
    if descriptor.schema_version == 1 {
        return handle_runtime_handoff_orphan_adoption(
            &source_session,
            logical_browser_id_hint,
            Some(descriptor),
            state,
        )
        .await;
    }
    if descriptor.schema_version != 2 || descriptor.session_name != source_session {
        return Err(format!(
            "Runtime handoff identity mismatch for source session '{}'",
            source_session
        ));
    }
    let proposal = descriptor.owner_transfer.as_ref().ok_or_else(|| {
        "runtime_handoff_owner_transfer_missing: descriptor has no two-phase owner proposal"
            .to_string()
    })?;
    if proposal.request.candidate_daemon_session_route != state.session_id {
        return Err(format!(
            "Runtime handoff candidate session mismatch: expected '{}' but received '{}'",
            proposal.request.candidate_daemon_session_route, state.session_id
        ));
    }
    if let Some(browser_pid) = descriptor.browser_pid {
        let assessment = runtime_handoff_process_assessment(&descriptor, browser_pid);
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
    let runtime_profile = descriptor.runtime_profile.as_deref().ok_or_else(|| {
        "runtime_handoff_profile_missing: candidate requires canonical profile identity".to_string()
    })?;
    let process_identity = descriptor.process_identity.as_ref().ok_or_else(|| {
        "runtime_handoff_process_identity_missing: candidate requires process-instance evidence"
            .to_string()
    })?;
    let attachment = crate::runtime_owner_transfer::CandidateOwnerAttachment {
        candidate_owner_id: proposal.request.candidate_owner_id.clone(),
        candidate_daemon_session_route: state.session_id.clone(),
        candidate_owner_generation: proposal.candidate_owner_generation,
        logical_browser_id: proposal.request.logical_browser_id.clone(),
        profile_identity_digest: runtime_handoff_profile_digest(runtime_profile)?,
        process_instance_digest: runtime_handoff_json_digest(process_identity)?,
        browser_family: descriptor.engine.clone(),
        cdp_endpoint_identity_digest: runtime_handoff_digest(&descriptor.cdp_url),
        target_set_digest: runtime_handoff_target_set_digest(&manager)?,
        selected_target_identity_digest: runtime_handoff_digest(manager.active_target_id()?),
        transfer_nonce_digest: proposal.request.transfer_nonce_digest.clone(),
        effect_capable: false,
    };
    let repository = runtime_handoff_service_repository()?;
    let owner_receipt =
        crate::native::runtime_lifecycle::RuntimeLifecycleAuthority::new(&repository)
            .commit_candidate(attachment)?;
    let candidate_owner = repository
        .load_snapshot()?
        .runtime_owner_registry
        .owner(&proposal.request.profile_identity_digest)
        .cloned()
        .ok_or_else(|| {
            "runtime_handoff_candidate_owner_missing: committed owner was not readable".to_string()
        })?;
    state.runtime_owner_binding = Some(
        crate::runtime_owner_transfer::RuntimeOwnerBinding::effect_capable(
            crate::runtime_owner_transfer::OwnerAuthorityClaim::from_owner(&candidate_owner),
        ),
    );
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
    persist_adopted_logical_browser_health(
        state,
        &proposal.request.logical_browser_id,
        descriptor.host,
    )?;
    Ok(json!(
        { "resumed" : true, "sessionName" : descriptor.session_name, "browserPid" :
        descriptor.browser_pid, "cdpUrl" : descriptor.cdp_url, "runtimeProfile" :
        descriptor.runtime_profile, "activeTargetId" : state.browser.as_ref()
        .and_then(| browser | browser.active_target_id().ok()), "retryRecordRemoved"
        : false, "transferState" : "candidate_committed", "ownerTransferReceipt" :
        owner_receipt, "targetsReattached" : state.browser.as_ref()
        .map(BrowserManager::page_count).unwrap_or(0), }
    ))
}

/// Match process-observed Chromium family labels to the canonical Chrome engine
/// without treating a different engine as compatible adoption evidence.
pub(crate) fn runtime_engine_accepts_browser_family(engine: &str, browser_family: &str) -> bool {
    match engine {
        "chrome" => matches!(browser_family, "chrome" | "chromium" | "brave" | "edge"),
        _ => engine == browser_family,
    }
}

async fn handle_runtime_handoff_orphan_adoption(
    source_session: &str,
    logical_browser_id_hint: Option<&str>,
    legacy_descriptor: Option<RuntimeHandoffDescriptor>,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let repository = runtime_handoff_service_repository()?;
    let snapshot = repository.load_snapshot()?;
    let logical_browser_id =
        orphan_logical_browser_id_with_hint(&snapshot, source_session, logical_browser_id_hint)?;
    let (browser, recorded, recovered_projection) = match (
        snapshot.browsers.get(&logical_browser_id).cloned(),
        snapshot
            .browser_process_identities
            .get(&logical_browser_id)
            .cloned(),
    ) {
        (Some(browser), Some(recorded)) => (browser, recorded, false),
        (None, None) => {
            let (browser, recorded) =
                durable_orphan_runtime_evidence(&snapshot, source_session, &logical_browser_id)?;
            (browser, recorded, true)
        }
        _ => return Err(
            "runtime_handoff_orphan_projection_partial: browser and process evidence must agree"
                .to_string(),
        ),
    };
    if recovered_projection {
        repository.mutate(|service_state| {
            service_state
                .browsers
                .insert(logical_browser_id.clone(), browser.clone());
            service_state
                .browser_process_identities
                .insert(logical_browser_id.clone(), recorded.clone());
            let session = service_state
                .sessions
                .entry(source_session.to_string())
                .or_insert_with(|| BrowserSession {
                    id: source_session.to_string(),
                    ..BrowserSession::default()
                });
            session.profile_id = browser.profile_id.clone();
            if !session.browser_ids.contains(&logical_browser_id) {
                session.browser_ids.push(logical_browser_id.clone());
            }
            Ok(())
        })?;
    }
    let browser_pid = browser.pid.ok_or_else(|| {
        "runtime_handoff_orphan_pid_missing: exact browser PID is required".to_string()
    })?;
    let cdp_url = browser
        .cdp_endpoint
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            "runtime_handoff_orphan_cdp_missing: bounded DevTools evidence is required".to_string()
        })?;
    let verified_runtime_profile =
        runtime_handoff_verified_profile_from_runtime_state(&recorded, &cdp_url)?;
    let runtime_profile = runtime_handoff_orphan_profile(
        recorded.runtime_profile.as_deref(),
        legacy_descriptor
            .as_ref()
            .and_then(|descriptor| descriptor.runtime_profile.as_deref()),
        verified_runtime_profile.as_deref(),
    )?;
    if let Some(legacy) = legacy_descriptor.as_ref() {
        if legacy.session_name != source_session
            || legacy.browser_pid != Some(browser_pid)
            || legacy.cdp_url != cdp_url
            || legacy.runtime_profile.as_deref() != Some(runtime_profile.as_str())
            || legacy.process_identity.as_ref() != Some(&recorded.process_identity)
        {
            return Err(
                "runtime_handoff_legacy_orphan_mismatch: descriptor and independent service evidence differ"
                    .to_string(),
            );
        }
    }
    let observed_browser_family = recorded
        .process_identity
        .browser_family
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| state.engine.clone());
    if !runtime_engine_accepts_browser_family(&state.engine, &observed_browser_family) {
        return Err(format!(
            "runtime_handoff_orphan_browser_family_mismatch: expected '{}' but observed '{}'",
            state.engine, observed_browser_family
        ));
    }
    let engine = state.engine.clone();
    let provisional = RuntimeHandoffDescriptor {
        schema_version: 2,
        session_name: source_session.to_string(),
        cdp_url: cdp_url.clone(),
        browser_pid: Some(browser_pid),
        runtime_profile: Some(runtime_profile.clone()),
        process_identity: Some(recorded.process_identity.clone()),
        engine: engine.clone(),
        host: browser.host,
        close_browser_on_close: false,
        active_target_id: None,
        owner_transfer: None,
        prepared_at: runtime_handoff_timestamp(),
    };
    let assessment = runtime_handoff_process_assessment(&provisional, browser_pid);
    if !assessment.authorizes_adoption() {
        return Err(format!(
            "runtime_handoff_orphan_process_mismatch: {}",
            assessment.reason
        ));
    }
    let manager = BrowserManager::connect_cdp_for_handoff(&cdp_url, None).await?;
    let profile_identity_digest = runtime_handoff_profile_digest(&runtime_profile)?;
    let prior_orphan = snapshot
        .runtime_owner_registry
        .owner(&profile_identity_digest)
        .cloned();
    if prior_orphan.as_ref().is_some_and(|owner| {
        owner.state != crate::runtime_owner_transfer::ProfileOwnerState::Orphaned
    }) {
        return Err(
            "runtime_handoff_orphan_owner_present: direct adoption requires an ownerless or explicitly orphaned registry owner"
                .to_string(),
        );
    }
    let process_instance_digest = runtime_handoff_json_digest(&recorded.process_identity)?;
    let target_set_digest = runtime_handoff_target_set_digest(&manager)?;
    let selected_target_identity_digest = runtime_handoff_digest(manager.active_target_id()?);
    let transfer_nonce_digest = runtime_handoff_digest(&format!(
        "orphan:{logical_browser_id}:{process_instance_digest}:{target_set_digest}"
    ));
    let proposal = crate::native::runtime_lifecycle::RuntimeLifecycleAuthority::new(&repository)
        .begin_transfer(crate::runtime_owner_transfer::OwnerTransferRequest {
            mode: crate::runtime_adoption::BrowserAdoptionMode::OrphanAdoption,
            logical_browser_id: logical_browser_id.clone(),
            profile_identity_digest: profile_identity_digest.clone(),
            expected_owner_id: prior_orphan.as_ref().map(|owner| owner.owner_id.clone()),
            expected_owner_generation: prior_orphan
                .as_ref()
                .map(|owner| owner.owner_generation)
                .unwrap_or(0),
            candidate_owner_id: format!("owner-{}", &transfer_nonce_digest[16..36]),
            candidate_daemon_session_route: state.session_id.clone(),
            process_instance_digest,
            browser_family: engine,
            cdp_endpoint_identity_digest: runtime_handoff_digest(&cdp_url),
            target_set_digest,
            selected_target_identity_digest,
            transfer_nonce_digest,
        })?;
    let descriptor = RuntimeHandoffDescriptor {
        active_target_id: Some(manager.active_target_id()?.to_string()),
        owner_transfer: Some(proposal.clone()),
        ..provisional
    };
    let path = write_runtime_handoff(&descriptor)?;
    let receipt = crate::native::runtime_lifecycle::RuntimeLifecycleAuthority::new(&repository)
        .commit_candidate(
            crate::runtime_owner_transfer::CandidateOwnerAttachment::from_request(
                &proposal.request,
                proposal.candidate_owner_generation,
            ),
        )?;
    let candidate_owner = repository
        .load_snapshot()?
        .runtime_owner_registry
        .owner(&profile_identity_digest)
        .cloned()
        .ok_or_else(|| {
            "runtime_handoff_orphan_commit_missing: committed owner was not readable".to_string()
        })?;
    state.runtime_owner_binding = Some(
        crate::runtime_owner_transfer::RuntimeOwnerBinding::effect_capable(
            crate::runtime_owner_transfer::OwnerAuthorityClaim::from_owner(&candidate_owner),
        ),
    );
    state.reset_input_state();
    state.attached_runtime_profile = Some(runtime_profile.clone());
    state.attached_browser_pid = Some(browser_pid);
    state.close_behavior = CloseBehavior::Detach;
    state.engine = descriptor.engine.clone();
    write_engine_file(&state.session_id, &state.engine);
    state.browser = Some(manager);
    state.subscribe_to_browser_events();
    state.start_fetch_handler();
    state.start_dialog_handler();
    state.update_stream_client().await;
    persist_adopted_logical_browser_health(state, &logical_browser_id, browser.host)?;
    Ok(json!({
        "resumed": true,
        "replayed": false,
        "adoptionMode": "orphan_adoption",
        "sourceSessionName": source_session,
        "sessionName": state.session_id,
        "browserPid": browser_pid,
        "cdpUrl": cdp_url,
        "runtimeProfile": runtime_profile,
        "handoffPath": path,
        "retryRecordRemoved": false,
        "transferState": "candidate_committed",
        "ownerTransferReceipt": receipt,
        "targetsReattached": state.browser.as_ref().map(BrowserManager::page_count).unwrap_or(0),
    }))
}

pub(crate) fn runtime_handoff_orphan_profile(
    recorded_runtime_profile: Option<&str>,
    legacy_runtime_profile: Option<&str>,
    verified_runtime_profile: Option<&str>,
) -> Result<String, String> {
    recorded_runtime_profile
        .filter(|value| !value.trim().is_empty())
        .or_else(|| legacy_runtime_profile.filter(|value| !value.trim().is_empty()))
        .or_else(|| verified_runtime_profile.filter(|value| !value.trim().is_empty()))
        .map(str::to_string)
        .ok_or_else(|| {
            "runtime_handoff_orphan_profile_missing: canonical runtime profile is required"
                .to_string()
        })
}

/// Resolve omitted service projection metadata only from one runtime-state
/// record with the same exact process identity and DevTools browser endpoint.
fn runtime_handoff_verified_profile_from_runtime_state(
    recorded: &ServiceBrowserProcessIdentity,
    cdp_url: &str,
) -> Result<Option<String>, String> {
    let root = crate::runtime_profile::runtime_profiles_root()?;
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "runtime_handoff_profile_inventory_failed: unable to read '{}': {error}",
                root.display()
            ));
        }
    };
    let mut states = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if crate::runtime_profile::validate_runtime_profile_name(&name).is_err() {
            continue;
        }
        if let Ok(Some(state)) = read_runtime_state(&name) {
            states.push(state);
        }
    }
    runtime_handoff_verified_profile_from_states(recorded, cdp_url, states)
}

pub(crate) fn runtime_handoff_verified_profile_from_states(
    recorded: &ServiceBrowserProcessIdentity,
    cdp_url: &str,
    states: impl IntoIterator<Item = crate::runtime_profile::RuntimeState>,
) -> Result<Option<String>, String> {
    let matches = states
        .into_iter()
        .filter(|state| {
            state.browser_pid == recorded.process_identity.pid
                && state.process_identity.as_ref() == Some(&recorded.process_identity)
                && state.ws_url.as_deref() == Some(cdp_url)
                && crate::runtime_profile::validate_runtime_profile_name(&state.runtime_profile)
                    .is_ok()
        })
        .map(|state| state.runtime_profile)
        .collect::<BTreeSet<_>>();
    if matches.len() > 1 {
        return Err(
            "runtime_handoff_orphan_profile_ambiguous: multiple exact runtime-profile states match the retained browser"
                .to_string(),
        );
    }
    Ok(matches.into_iter().next())
}

pub(crate) fn orphan_logical_browser_id(
    snapshot: &crate::native::service_model::ServiceState,
    source_session: &str,
) -> Result<String, String> {
    let matches = snapshot
        .runtime_owner_registry
        .owners
        .values()
        .filter(|owner| {
            owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Orphaned
                && owner.daemon_session_route == source_session
        })
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(format!(
            "runtime_handoff_orphan_owner_ambiguous: source session '{source_session}' matches multiple revoked owners"
        ));
    }
    if let Some(owner) = matches.first() {
        return Ok(owner.browser_id.clone());
    }
    let mapped_orphans = snapshot
        .sessions
        .get(source_session)
        .into_iter()
        .flat_map(|session| session.browser_ids.iter())
        .filter(|browser_id| {
            snapshot
                .runtime_owner_registry
                .owners
                .values()
                .any(|owner| {
                    owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Orphaned
                        && owner.browser_id.as_str() == browser_id.as_str()
                })
        })
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if mapped_orphans.len() > 1 {
        return Err(format!(
            "runtime_handoff_orphan_browser_ambiguous: source session '{source_session}' maps to multiple revoked logical browsers"
        ));
    }
    Ok(mapped_orphans
        .into_iter()
        .next()
        .unwrap_or_else(|| service_browser_id(source_session)))
}

/// Resolves a stale transaction alias only when the service session and
/// retained browser projection both bind it to the exact logical browser.
fn orphan_logical_browser_id_with_hint(
    snapshot: &crate::native::service_model::ServiceState,
    source_session: &str,
    logical_browser_id_hint: Option<&str>,
) -> Result<String, String> {
    let Some(logical_browser_id) = logical_browser_id_hint else {
        return orphan_logical_browser_id(snapshot, source_session);
    };
    let session_bound = snapshot
        .sessions
        .get(source_session)
        .is_some_and(|session| {
            session
                .browser_ids
                .iter()
                .any(|browser_id| browser_id == logical_browser_id)
        });
    let owner_bound = snapshot
        .runtime_owner_registry
        .owners
        .values()
        .any(|owner| {
            owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Orphaned
                && owner.daemon_session_route == source_session
                && owner.browser_id == logical_browser_id
        });
    let durable_handoff_bound =
        durable_orphan_runtime_profile(snapshot, source_session, logical_browser_id).is_ok();
    if (!session_bound && !owner_bound && !durable_handoff_bound)
        || (!snapshot.browsers.contains_key(logical_browser_id) && !durable_handoff_bound)
    {
        return Err(format!(
            "runtime_handoff_orphan_browser_hint_mismatch: source session '{source_session}' is not bound to '{logical_browser_id}'"
        ));
    }
    Ok(logical_browser_id.to_string())
}

pub(crate) fn durable_orphan_runtime_profile(
    snapshot: &crate::native::service_model::ServiceState,
    source_session: &str,
    logical_browser_id: &str,
) -> Result<String, String> {
    let owner = snapshot
        .runtime_owner_registry
        .owners
        .values()
        .find(|owner| {
            owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Orphaned
                && owner.daemon_session_route == source_session
                && owner.browser_id == logical_browser_id
        })
        .ok_or_else(|| "runtime_handoff_durable_orphan_owner_mismatch".to_string())?;
    let mut profiles = snapshot
        .remote_view_handoffs
        .values()
        .filter(|handoff| {
            handoff.state == "ready"
                && handoff.browser_id.as_deref() == Some(logical_browser_id)
                && handoff.session_name.as_deref() == Some(source_session)
                && handoff
                    .presentation_receipt
                    .as_ref()
                    .is_some_and(|receipt| {
                        receipt.state == "ready"
                            && receipt.logical_browser_id == logical_browser_id
                            && receipt.process_instance_digest.as_deref()
                                == Some(owner.process_instance_digest.as_str())
                            && receipt.daemon_owner_generation.is_some_and(|generation| {
                                generation == owner.owner_generation
                                    || generation.checked_add(1) == Some(owner.owner_generation)
                            })
                    })
        })
        .filter_map(|handoff| handoff.profile_id.as_deref())
        .filter(|profile| !profile.trim().is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    if profiles.len() != 1 {
        return Err("runtime_handoff_durable_orphan_profile_ambiguous".to_string());
    }
    Ok(profiles.pop_first().expect("one durable orphan profile"))
}

fn durable_orphan_runtime_evidence(
    snapshot: &crate::native::service_model::ServiceState,
    source_session: &str,
    logical_browser_id: &str,
) -> Result<(BrowserProcess, ServiceBrowserProcessIdentity), String> {
    let runtime_profile =
        durable_orphan_runtime_profile(snapshot, source_session, logical_browser_id)?;
    let runtime_state = read_runtime_state(&runtime_profile)?
        .ok_or_else(|| "runtime_handoff_durable_orphan_runtime_state_missing".to_string())?;
    let process_identity = runtime_state
        .process_identity
        .clone()
        .ok_or_else(|| "runtime_handoff_durable_orphan_process_identity_missing".to_string())?;
    if !crate::runtime_profile::runtime_process_assessment(
        Some(&runtime_profile),
        runtime_state.browser_pid,
    )
    .authorizes_adoption()
    {
        return Err("runtime_handoff_durable_orphan_process_mismatch".to_string());
    }
    let cdp_endpoint = runtime_state
        .ws_url
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "runtime_handoff_durable_orphan_cdp_missing".to_string())?;
    Ok((
        BrowserProcess {
            id: logical_browser_id.to_string(),
            profile_id: Some(runtime_profile.clone()),
            host: ServiceBrowserHost::RemoteHeaded,
            health: ServiceBrowserHealth::Ready,
            pid: Some(runtime_state.browser_pid),
            cdp_endpoint: Some(cdp_endpoint),
            active_session_ids: vec![source_session.to_string()],
            ..BrowserProcess::default()
        },
        ServiceBrowserProcessIdentity {
            process_identity,
            user_data_dir: Some(runtime_state.user_data_dir),
            runtime_profile: Some(runtime_profile),
        },
    ))
}

fn persist_adopted_logical_browser_health(
    state: &DaemonState,
    logical_browser_id: &str,
    host: ServiceBrowserHost,
) -> Result<(), String> {
    let manager = state.browser.as_ref().ok_or_else(|| {
        "runtime_handoff_persist_browser_missing: adopted browser is unavailable".to_string()
    })?;
    let pid = manager.browser_pid().or(state.attached_browser_pid);
    let cdp_endpoint = manager.get_cdp_url().to_string();
    let process_identity = pid.and_then(|pid| {
        crate::process_identity::capture_process_identity(pid, None, None).map(|identity| {
            ServiceBrowserProcessIdentity {
                process_identity: identity,
                user_data_dir: manager
                    .browser_user_data_dir()
                    .map(|path| path.to_string_lossy().into_owned()),
                runtime_profile: manager
                    .runtime_profile_name()
                    .map(str::to_string)
                    .or_else(|| state.attached_runtime_profile.clone()),
            }
        })
    });
    runtime_handoff_service_repository()?.mutate(|service_state| {
        let prior_session_ids = service_state
            .browsers
            .get(logical_browser_id)
            .ok_or_else(|| {
                "runtime_handoff_logical_browser_missing: owner browser record disappeared"
                    .to_string()
            })?
            .active_session_ids
            .iter()
            .filter(|session_id| session_id.as_str() != state.session_id)
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        rebind_runtime_handoff_service_projection_in_state(
            service_state,
            logical_browser_id,
            &prior_session_ids,
            &state.session_id,
        )?;
        let browser = service_state
            .browsers
            .get_mut(logical_browser_id)
            .ok_or_else(|| {
                "runtime_handoff_logical_browser_missing: owner browser record disappeared"
                    .to_string()
            })?;
        browser.host = host;
        browser.health = ServiceBrowserHealth::Ready;
        browser.pid = pid;
        browser.cdp_endpoint = Some(cdp_endpoint);
        browser.last_error = None;
        if let Some(process_identity) = process_identity {
            service_state
                .browser_process_identities
                .insert(logical_browser_id.to_string(), process_identity);
        }
        Ok(())
    })
}

fn runtime_handoff_prepared_response(
    descriptor: &RuntimeHandoffDescriptor,
    path: PathBuf,
    replayed: bool,
) -> Value {
    let proposal = descriptor
        .owner_transfer
        .as_ref()
        .expect("schema version 2 descriptor must carry an owner proposal");
    json!({
        "prepared": true,
        "replayed": replayed,
        "browserPresent": true,
        "sessionName": descriptor.session_name,
        "browserPid": descriptor.browser_pid,
        "cdpUrl": descriptor.cdp_url,
        "runtimeProfile": descriptor.runtime_profile,
        "handoffPath": path,
        "transferState": "awaiting_candidate",
        "oldOwnerEffectCapable": true,
        "candidateSessionName": proposal.request.candidate_daemon_session_route,
        "previousOwnerGeneration": proposal.previous_owner_generation,
        "candidateOwnerGeneration": proposal.candidate_owner_generation,
    })
}

pub(crate) async fn handle_runtime_handoff_finalize(
    state: &mut DaemonState,
) -> Result<Value, String> {
    let binding = state.runtime_owner_binding.as_ref().ok_or_else(|| {
        "runtime_handoff_finalize_unbound: daemon has no owner-transfer binding".to_string()
    })?;
    if !binding.effect_capable {
        return Err(
            "runtime_handoff_finalize_observation_only: candidate cannot finalize the old owner"
                .to_string(),
        );
    }
    let repository = runtime_handoff_service_repository()?;
    RuntimeLifecycleAuthority::new(&repository)
        .authorize_relinquish_after_transfer(&binding.claim)?;
    let Some(manager) = state.browser.as_mut() else {
        return Err(
            "runtime_handoff_finalize_browser_missing: old browser is unavailable".to_string(),
        );
    };
    manager.relinquish_browser_for_handoff();
    state.browser = None;
    state.screencasting = false;
    state.reset_input_state();
    state.update_stream_client().await;
    let retry_record_removed = fs::remove_file(runtime_handoff_path(&state.session_id)).is_ok();
    Ok(json!({
        "finalized": true,
        "sessionName": state.session_id,
        "browserPreserved": true,
        "retryRecordRemoved": retry_record_removed,
    }))
}

pub(crate) fn handle_runtime_handoff_abort(state: &mut DaemonState) -> Result<Value, String> {
    let binding = state.runtime_owner_binding.as_ref().ok_or_else(|| {
        "runtime_handoff_abort_unbound: old daemon has no owner-transfer binding".to_string()
    })?;
    if !binding.effect_capable || binding.claim.daemon_session_route != state.session_id {
        return Err(
            "runtime_handoff_abort_not_old_owner: only the current old owner may abort".to_string(),
        );
    }
    let descriptor = read_runtime_handoff(&state.session_id)?;
    let proposal = descriptor.owner_transfer.as_ref().ok_or_else(|| {
        "runtime_handoff_owner_transfer_missing: abort descriptor has no owner proposal".to_string()
    })?;
    if descriptor.schema_version != 2
        || proposal.request.profile_identity_digest != binding.claim.profile_identity_digest
        || proposal.request.expected_owner_id.as_deref() != Some(binding.claim.owner_id.as_str())
        || proposal.request.expected_owner_generation != binding.claim.owner_generation
        || proposal.request.logical_browser_id != binding.claim.logical_browser_id
        || proposal.request.process_instance_digest != binding.claim.process_instance_digest
    {
        return Err(
            "runtime_handoff_abort_evidence_mismatch: old owner and descriptor differ".to_string(),
        );
    }
    let repository = runtime_handoff_service_repository()?;
    if !crate::runtime_owner_transfer::owner_authority_is_current(&repository, &binding.claim)? {
        return Err(
            "runtime_handoff_abort_after_commit: candidate owner already committed".to_string(),
        );
    }
    let aborted = crate::native::runtime_lifecycle::RuntimeLifecycleAuthority::new(&repository)
        .abort_transfer(
            &proposal.request.profile_identity_digest,
            &binding.claim.owner_id,
            binding.claim.owner_generation,
            &proposal.request.transfer_nonce_digest,
        )?;
    let retry_record_removed = fs::remove_file(runtime_handoff_path(&state.session_id)).is_ok();
    Ok(json!({
        "aborted": aborted,
        "sessionName": state.session_id,
        "oldOwnerEffectCapable": true,
        "browserPreserved": true,
        "retryRecordRemoved": retry_record_removed,
    }))
}

pub(crate) async fn handle_runtime_handoff_rollback(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let source_session = cmd
        .get("sourceSession")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "runtime_handoff_source_session_missing: rollback requires --source-session".to_string()
        })?;
    if !crate::validation::is_valid_session_name(source_session) {
        return Err(crate::validation::session_name_error(source_session));
    }
    let binding = state.runtime_owner_binding.as_ref().ok_or_else(|| {
        "runtime_handoff_rollback_unbound: candidate has no owner-transfer binding".to_string()
    })?;
    if !binding.effect_capable || binding.claim.daemon_session_route != state.session_id {
        return Err(
            "runtime_handoff_rollback_not_candidate: only the committed candidate may roll back"
                .to_string(),
        );
    }
    let descriptor = read_runtime_handoff(source_session)?;
    let proposal = descriptor.owner_transfer.as_ref().ok_or_else(|| {
        "runtime_handoff_owner_transfer_missing: rollback descriptor has no owner proposal"
            .to_string()
    })?;
    if descriptor.schema_version != 2
        || descriptor.session_name != source_session
        || proposal.request.candidate_owner_id != binding.claim.owner_id
        || proposal.candidate_owner_generation != binding.claim.owner_generation
        || proposal.request.profile_identity_digest != binding.claim.profile_identity_digest
        || proposal.request.logical_browser_id != binding.claim.logical_browser_id
        || proposal.request.process_instance_digest != binding.claim.process_instance_digest
    {
        return Err(
            "runtime_handoff_rollback_evidence_mismatch: candidate and descriptor differ"
                .to_string(),
        );
    }
    let repository = runtime_handoff_service_repository()?;
    if !crate::runtime_owner_transfer::owner_authority_is_current(&repository, &binding.claim)? {
        return Err(
            "runtime_handoff_rollback_owner_stale: candidate is no longer authoritative"
                .to_string(),
        );
    }
    let reverse_nonce_digest = runtime_handoff_digest(&format!(
        "reverse:{}:{}",
        proposal.request.transfer_nonce_digest, state.session_id
    ));
    let receipt = crate::native::runtime_lifecycle::RuntimeLifecycleAuthority::new(&repository)
        .reverse_transfer(crate::runtime_owner_transfer::ReverseOwnerTransferRequest {
            profile_identity_digest: proposal.request.profile_identity_digest.clone(),
            expected_candidate_owner_id: proposal.request.candidate_owner_id.clone(),
            expected_candidate_owner_generation: proposal.candidate_owner_generation,
            transfer_nonce_digest: proposal.request.transfer_nonce_digest.clone(),
            reverse_nonce_digest,
        })?;
    restore_runtime_handoff_service_projection(
        &repository,
        &binding.claim.logical_browser_id,
        &state.session_id,
        source_session,
    )?;
    RuntimeLifecycleAuthority::new(&repository)
        .authorize_relinquish_after_transfer(&binding.claim)?;
    let manager = state.browser.as_mut().ok_or_else(|| {
        "runtime_handoff_rollback_browser_missing: candidate browser is unavailable".to_string()
    })?;
    manager.relinquish_browser_for_handoff();
    state.browser = None;
    state.screencasting = false;
    state.reset_input_state();
    state.update_stream_client().await;
    let retry_record_removed = fs::remove_file(runtime_handoff_path(source_session)).is_ok();
    Ok(json!({
        "rolledBack": true,
        "sourceSessionName": source_session,
        "candidateSessionName": state.session_id,
        "browserPreserved": true,
        "retryRecordRemoved": retry_record_removed,
        "ownerTransferReceipt": receipt,
    }))
}

fn restore_runtime_handoff_service_projection(
    repository: &impl ServiceStateRepository,
    logical_browser_id: &str,
    candidate_session: &str,
    source_session: &str,
) -> Result<(), String> {
    repository.mutate(|service_state| {
        restore_runtime_handoff_service_projection_in_state(
            service_state,
            logical_browser_id,
            candidate_session,
            source_session,
        )
    })
}

fn restore_runtime_handoff_service_projection_in_state(
    service_state: &mut crate::native::service_model::ServiceState,
    logical_browser_id: &str,
    candidate_session: &str,
    source_session: &str,
) -> Result<(), String> {
    rebind_runtime_handoff_service_projection_in_state(
        service_state,
        logical_browser_id,
        &std::collections::BTreeSet::from([candidate_session.to_string()]),
        source_session,
    )
}

fn rebind_runtime_handoff_service_projection_in_state(
    service_state: &mut crate::native::service_model::ServiceState,
    logical_browser_id: &str,
    prior_session_ids: &std::collections::BTreeSet<String>,
    next_session_id: &str,
) -> Result<(), String> {
    let mut browser_session_ids = prior_session_ids.clone();
    if let Some(browser) = service_state.browsers.get(logical_browser_id) {
        browser_session_ids.extend(browser.active_session_ids.iter().cloned());
        for tab in &browser.tab_handles {
            browser_session_ids.extend(tab.session_name.iter().cloned());
            browser_session_ids.extend(tab.owner_session_id.iter().cloned());
            browser_session_ids.extend(tab.trace_filter.session_id.iter().cloned());
        }
    }
    for tab in service_state
        .tabs
        .values()
        .filter(|tab| tab.browser_id == logical_browser_id)
    {
        browser_session_ids.extend(tab.session_id.iter().cloned());
        browser_session_ids.extend(tab.owner_session_id.iter().cloned());
        if let Some(handle) = tab.service_tab_handle.as_ref() {
            browser_session_ids.extend(handle.session_name.iter().cloned());
            browser_session_ids.extend(handle.owner_session_id.iter().cloned());
            browser_session_ids.extend(handle.trace_filter.session_id.iter().cloned());
        }
    }
    browser_session_ids.extend(
        service_state
            .remote_view_routes
            .values()
            .filter(|route| route.browser_id.as_deref() == Some(logical_browser_id))
            .filter_map(|route| route.session_id.clone()),
    );
    browser_session_ids.extend(
        service_state
            .display_allocations
            .values()
            .filter(|allocation| allocation.owner_browser_id.as_deref() == Some(logical_browser_id))
            .filter_map(|allocation| allocation.owner_session_id.clone()),
    );
    for handoff in service_state
        .remote_view_handoffs
        .values()
        .filter(|handoff| handoff.browser_id.as_deref() == Some(logical_browser_id))
    {
        browser_session_ids.extend(handoff.session_name.iter().cloned());
        browser_session_ids.extend(
            handoff
                .last_resolution
                .as_ref()
                .and_then(|resolution| resolution.get("sessionName"))
                .and_then(Value::as_str)
                .map(str::to_string),
        );
    }
    browser_session_ids.extend(
        service_state
            .sessions
            .values()
            .filter(|session| {
                session
                    .browser_ids
                    .iter()
                    .any(|browser_id| browser_id == logical_browser_id)
            })
            .map(|session| session.id.clone()),
    );
    browser_session_ids.remove(next_session_id);

    // The retained browser record is bound to the process/profile identity used
    // by lifecycle ownership. A single-browser session may carry stale lease
    // metadata after a prior handoff, but it must not override that authority.
    let authoritative_browser_profile_id = service_state
        .browsers
        .get(logical_browser_id)
        .ok_or_else(|| {
            "runtime_handoff_rollback_browser_projection_missing: retained browser record disappeared"
                .to_string()
        })?
        .profile_id
        .as_deref()
        .map(str::trim)
        .filter(|profile_id| !profile_id.is_empty())
        .map(str::to_string);

    let transferred_tab_ids = service_state
        .tabs
        .values()
        .filter(|tab| tab.browser_id == logical_browser_id)
        .map(|tab| tab.id.clone())
        .collect::<BTreeSet<_>>();

    let inherited_session = service_state
        .sessions
        .values()
        .filter(|session| {
            browser_session_ids.contains(&session.id)
                && session.profile_id.is_some()
                && !matches!(session.lease, LeaseState::Released | LeaseState::Expired)
        })
        .max_by_key(|session| {
            let lease_priority = match session.lease {
                LeaseState::HumanTakeover => 3,
                LeaseState::Exclusive => 2,
                LeaseState::Shared => 1,
                LeaseState::Released | LeaseState::Expired => 0,
            };
            (
                prior_session_ids.contains(&session.id),
                lease_priority,
                session.service_name.is_some(),
                session.agent_name.is_some(),
                session.task_name.is_some(),
                session.last_lease_observed_at.is_some(),
            )
        })
        .cloned();
    let inherited_profile_id = inherited_session
        .as_ref()
        .and_then(|session| session.profile_id.clone());
    let authoritative_profile_id = authoritative_browser_profile_id
        .clone()
        .or_else(|| inherited_profile_id.clone());
    if let Some(authoritative_profile_id) = authoritative_profile_id.as_deref() {
        let conflicting_profile_ids = service_state
            .sessions
            .values()
            .filter(|session| browser_session_ids.contains(&session.id))
            .filter(|session| !matches!(session.lease, LeaseState::Released | LeaseState::Expired))
            .filter(|session| {
                session.profile_id.as_deref() != Some(authoritative_profile_id)
                    && (authoritative_browser_profile_id.is_none()
                        || session
                            .browser_ids
                            .iter()
                            .any(|browser_id| browser_id != logical_browser_id)
                        || session
                            .tab_ids
                            .iter()
                            .any(|tab_id| !transferred_tab_ids.contains(tab_id)))
            })
            .filter_map(|session| session.profile_id.as_deref())
            .collect::<BTreeSet<_>>();
        if !conflicting_profile_ids.is_empty() {
            return Err(format!(
                "runtime_handoff_profile_projection_conflict: retained browser sessions disagree on profile identity ({})",
                conflicting_profile_ids.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    if let (Some(candidate), Some(authoritative_profile_id)) = (
        service_state.sessions.get(next_session_id),
        authoritative_profile_id.as_deref(),
    ) {
        if candidate
            .profile_id
            .as_deref()
            .is_some_and(|profile_id| profile_id != authoritative_profile_id)
            && (authoritative_browser_profile_id.is_none()
                || candidate
                    .browser_ids
                    .iter()
                    .any(|browser_id| browser_id != logical_browser_id)
                || candidate
                    .tab_ids
                    .iter()
                    .any(|tab_id| !transferred_tab_ids.contains(tab_id)))
        {
            return Err(
                "runtime_handoff_candidate_profile_projection_conflict: candidate session already references a different profile"
                    .to_string(),
            );
        }
    }
    let inherited_profile_matches_authority = inherited_profile_id
        .as_deref()
        .zip(authoritative_profile_id.as_deref())
        .is_none_or(|(inherited, authoritative)| inherited == authoritative);

    for session_id in &browser_session_ids {
        let Some(session) = service_state.sessions.get_mut(session_id) else {
            continue;
        };
        session.browser_ids.retain(|id| id != logical_browser_id);
        session
            .tab_ids
            .retain(|id| !transferred_tab_ids.contains(id));
        session.profile_lease_conflict_session_ids.clear();
        if session.browser_ids.is_empty() && session.tab_ids.is_empty() {
            session.lease = LeaseState::Released;
        }
    }

    let next_session = service_state
        .sessions
        .entry(next_session_id.to_string())
        .or_insert_with(|| BrowserSession {
            id: next_session_id.to_string(),
            ..BrowserSession::default()
        });
    if next_session.id.is_empty() {
        next_session.id = next_session_id.to_string();
    }
    if let Some(inherited) = inherited_session {
        if next_session.service_name.is_none() {
            next_session.service_name = inherited.service_name;
        }
        if next_session.agent_name.is_none() {
            next_session.agent_name = inherited.agent_name;
        }
        if next_session.task_name.is_none() {
            next_session.task_name = inherited.task_name;
        }
        next_session.owner = inherited.owner;
        next_session.lease = inherited.lease;
        if inherited_profile_matches_authority && next_session.profile_selection_reason.is_none() {
            next_session.profile_selection_reason = inherited.profile_selection_reason;
        }
        if next_session.browser_capability_launch.is_none() {
            next_session.browser_capability_launch = inherited.browser_capability_launch;
        }
        next_session.cleanup = inherited.cleanup;
        if next_session.last_lease_observed_at.is_none() {
            next_session.last_lease_observed_at = inherited.last_lease_observed_at;
            next_session.boot_epoch = crate::process_identity::current_boot_epoch();
        }
        if next_session.expires_at.is_none() {
            next_session.expires_at = inherited.expires_at;
        }
    }
    if let Some(authoritative_profile_id) = authoritative_profile_id {
        next_session.profile_id = Some(authoritative_profile_id);
        next_session.profile_lease_disposition = Some(ProfileLeaseDisposition::ReusedBrowser);
    }
    next_session.profile_lease_conflict_session_ids.clear();
    if !next_session
        .browser_ids
        .iter()
        .any(|id| id == logical_browser_id)
    {
        next_session
            .browser_ids
            .push(logical_browser_id.to_string());
    }
    for tab_id in transferred_tab_ids {
        if !next_session.tab_ids.contains(&tab_id) {
            next_session.tab_ids.push(tab_id);
        }
    }

    let browser = service_state
        .browsers
        .get_mut(logical_browser_id)
        .ok_or_else(|| {
            "runtime_handoff_rollback_browser_projection_missing: retained browser record disappeared"
                .to_string()
        })?;
    browser.active_session_ids = vec![next_session_id.to_string()];
    for tab in &mut browser.tab_handles {
        if tab
            .session_name
            .as_ref()
            .is_some_and(|session| browser_session_ids.contains(session))
        {
            tab.session_name = Some(next_session_id.to_string());
        }
        if tab
            .owner_session_id
            .as_ref()
            .is_some_and(|session| browser_session_ids.contains(session))
        {
            tab.owner_session_id = Some(next_session_id.to_string());
        }
        if tab
            .lease_id
            .as_ref()
            .is_some_and(|session| browser_session_ids.contains(session))
        {
            tab.lease_id = Some(next_session_id.to_string());
        }
        if tab
            .trace_filter
            .session_id
            .as_ref()
            .is_some_and(|session| browser_session_ids.contains(session))
        {
            tab.trace_filter.session_id = Some(next_session_id.to_string());
        }
    }
    for tab in service_state
        .tabs
        .values_mut()
        .filter(|tab| tab.browser_id == logical_browser_id)
    {
        if tab
            .session_id
            .as_ref()
            .is_some_and(|session| browser_session_ids.contains(session))
        {
            tab.session_id = Some(next_session_id.to_string());
        }
        if tab
            .owner_session_id
            .as_ref()
            .is_some_and(|session| browser_session_ids.contains(session))
        {
            tab.owner_session_id = Some(next_session_id.to_string());
        }
        if let Some(handle) = tab.service_tab_handle.as_mut() {
            if handle
                .session_name
                .as_ref()
                .is_some_and(|session| browser_session_ids.contains(session))
            {
                handle.session_name = Some(next_session_id.to_string());
            }
            if handle
                .owner_session_id
                .as_ref()
                .is_some_and(|session| browser_session_ids.contains(session))
            {
                handle.owner_session_id = Some(next_session_id.to_string());
            }
            if handle
                .lease_id
                .as_ref()
                .is_some_and(|session| browser_session_ids.contains(session))
            {
                handle.lease_id = Some(next_session_id.to_string());
            }
            if handle
                .trace_filter
                .session_id
                .as_ref()
                .is_some_and(|session| browser_session_ids.contains(session))
            {
                handle.trace_filter.session_id = Some(next_session_id.to_string());
            }
        }
    }
    for route in service_state
        .remote_view_routes
        .values_mut()
        .filter(|route| {
            route.browser_id.as_deref() == Some(logical_browser_id)
                && route
                    .session_id
                    .as_ref()
                    .is_some_and(|session| browser_session_ids.contains(session))
        })
    {
        route.session_id = Some(next_session_id.to_string());
    }
    for allocation in service_state
        .display_allocations
        .values_mut()
        .filter(|allocation| {
            allocation.owner_browser_id.as_deref() == Some(logical_browser_id)
                && allocation
                    .owner_session_id
                    .as_ref()
                    .is_some_and(|session| browser_session_ids.contains(session))
        })
    {
        allocation.owner_session_id = Some(next_session_id.to_string());
    }
    for handoff in service_state
        .remote_view_handoffs
        .values_mut()
        .filter(|handoff| handoff.browser_id.as_deref() == Some(logical_browser_id))
    {
        if handoff
            .session_name
            .as_ref()
            .is_some_and(|session| browser_session_ids.contains(session))
        {
            handoff.session_name = Some(next_session_id.to_string());
        }
        if let Some(last_resolution) = handoff.last_resolution.as_mut() {
            let resolution_session = last_resolution
                .get("sessionName")
                .and_then(Value::as_str)
                .map(str::to_string);
            if resolution_session
                .as_ref()
                .is_some_and(|session| browser_session_ids.contains(session))
            {
                last_resolution["sessionName"] = Value::String(next_session_id.to_string());
            }
        }
    }
    Ok(())
}

fn runtime_handoff_profile_digest(runtime_profile: &str) -> Result<String, String> {
    let user_data_dir = runtime_profile_user_data_dir(runtime_profile)?;
    crate::runtime_profile::canonical_profile_identity_digest(&user_data_dir)
}

fn runtime_handoff_target_set_digest(manager: &BrowserManager) -> Result<String, String> {
    let mut target_ids = manager
        .pages_list()
        .into_iter()
        .map(|page| page.target_id)
        .collect::<Vec<_>>();
    target_ids.sort();
    target_ids.dedup();
    if target_ids.is_empty() {
        return Err(
            "runtime_handoff_target_set_missing: browser has no retained targets".to_string(),
        );
    }
    runtime_handoff_json_digest(&target_ids)
}

fn runtime_handoff_json_digest(value: &impl serde::Serialize) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("could not serialize runtime handoff evidence: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn runtime_handoff_digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn runtime_handoff_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| OffsetDateTime::now_utc().unix_timestamp().to_string())
}

pub(crate) fn runtime_handoff_process_assessment(
    descriptor: &RuntimeHandoffDescriptor,
    browser_pid: u32,
) -> crate::process_identity::RuntimeProcessAssessment {
    if let Some(process_identity) = descriptor.process_identity.as_ref() {
        return crate::process_identity::assess_process_ownership(
            Some(process_identity),
            crate::process_identity::observe_process(browser_pid),
            crate::process_identity::LegacyProfileProof::Unproven,
        );
    }
    crate::runtime_profile::runtime_process_assessment(
        descriptor.runtime_profile.as_deref(),
        browser_pid,
    )
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
    let managed_close_claim = if let Some(binding) = state.runtime_owner_binding.as_mut() {
        let repository = runtime_handoff_service_repository()?;
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let claim = binding.claim.clone();
        let intent = if preserve_registered_work && !binding.effect_capable {
            RuntimeLifecycleIntent::BeginRecoveryClose {
                claim: claim.clone(),
            }
        } else if close_behavior == CloseBehavior::CloseBrowser {
            authority.authorize_effect(binding)?;
            RuntimeLifecycleIntent::BeginClose {
                claim: claim.clone(),
            }
        } else {
            authority.authorize_effect(binding)?;
            RuntimeLifecycleIntent::PreserveRetained {
                claim: claim.clone(),
            }
        };
        authority.transition(intent)?;
        Some(claim)
    } else {
        None
    };
    if managed_close_claim.is_some() && close_behavior == CloseBehavior::CloseBrowser {
        if let Some(manager) = state.browser.as_mut() {
            manager.approve_lifecycle_close();
        }
    }
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
            shutdown_outcome.exact_process_exited |= outcome.exact_process_exited;
            shutdown_outcome.profile_lock_released |= outcome.profile_lock_released;
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
                shutdown_outcome.exact_process_exited |= outcome.exact_process_exited;
                shutdown_outcome.profile_lock_released |= outcome.profile_lock_released;
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
            shutdown_outcome.exact_process_exited |= outcome.exact_process_exited;
            shutdown_outcome.profile_lock_released |= outcome.profile_lock_released;
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
    if preserve_registered_work {
        super::recovery::persist_recovery_closed_browser_health(state, Some(&shutdown_outcome));
    } else {
        persist_closed_browser_health(state, Some(&shutdown_outcome));
    }
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
    if close_behavior == CloseBehavior::CloseBrowser {
        if let (Some(claim), Some(terminal_evidence)) = (
            managed_close_claim,
            browser_terminal_evidence(&shutdown_outcome),
        ) {
            RuntimeLifecycleAuthority::new(&runtime_handoff_service_repository()?)
                .complete_close_and_release_binding(
                    &mut state.runtime_owner_binding,
                    claim,
                    terminal_evidence,
                )?;
        }
    }
    Ok(json!({ "closed" : true }))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orphan_logical_browser_hint_requires_exact_session_and_browser_binding() {
        let source_session = "orphan-prior";
        let logical_browser_id = "session:work";
        let mut state = crate::native::service_model::ServiceState::default();
        state.sessions.insert(
            source_session.to_string(),
            crate::native::service_model::BrowserSession {
                id: source_session.to_string(),
                browser_ids: vec![logical_browser_id.to_string()],
                ..Default::default()
            },
        );
        state.browsers.insert(
            logical_browser_id.to_string(),
            crate::native::service_model::BrowserProcess {
                id: logical_browser_id.to_string(),
                ..Default::default()
            },
        );

        assert_eq!(
            orphan_logical_browser_id_with_hint(&state, source_session, Some(logical_browser_id))
                .unwrap(),
            logical_browser_id
        );
        assert!(orphan_logical_browser_id_with_hint(
            &state,
            source_session,
            Some("session:different")
        )
        .is_err());
    }

    #[test]
    fn revoked_owner_accepts_one_exact_ready_durable_handoff_alias() {
        use crate::native::service_model::{DurableHandoffPresentationReceipt, ViewStreamProvider};
        use crate::runtime_owner_transfer::{ProfileOwner, ProfileOwnerState};

        let source_session = "orphan-recovered";
        let logical_browser_id = "session:payment";
        let process_digest = "2".repeat(64);
        let mut state = crate::native::service_model::ServiceState::default();
        state.runtime_owner_registry.owners.insert(
            "1".repeat(64),
            ProfileOwner {
                owner_id: "owner-payment".to_string(),
                profile_identity_digest: "1".repeat(64),
                state: ProfileOwnerState::Orphaned,
                owner_generation: 10,
                browser_id: logical_browser_id.to_string(),
                daemon_session_route: source_session.to_string(),
                process_instance_digest: process_digest.clone(),
                browser_family: "chrome".to_string(),
                cdp_endpoint_identity_digest: "3".repeat(64),
                target_set_digest: "4".repeat(64),
                pending_transfer: None,
                last_transition: None,
            },
        );
        state.remote_view_handoffs.insert(
            "r-payment".to_string(),
            RemoteViewHandoff {
                id: "r-payment".to_string(),
                state: "ready".to_string(),
                profile_id: Some("default".to_string()),
                browser_id: Some(logical_browser_id.to_string()),
                session_name: Some(source_session.to_string()),
                presentation_receipt: Some(DurableHandoffPresentationReceipt {
                    schema_version: "agent-browser.durable-handoff-presentation.v1".to_string(),
                    generation: 7,
                    dashboard_deployment_generation: "generation-a".to_string(),
                    logical_browser_id: logical_browser_id.to_string(),
                    daemon_owner_generation: Some(9),
                    process_instance_digest: Some(process_digest),
                    target_id: "target-a".to_string(),
                    required_stream_provider: ViewStreamProvider::RdpGateway,
                    observed_stream_provider: ViewStreamProvider::RdpGateway,
                    route_id: "route-a".to_string(),
                    display_allocation_id: "display-a".to_string(),
                    observed_at: "2026-08-21T00:00:00Z".to_string(),
                    state: "ready".to_string(),
                }),
                ..RemoteViewHandoff::default()
            },
        );

        assert_eq!(
            durable_orphan_runtime_profile(&state, source_session, logical_browser_id).unwrap(),
            "default"
        );
        assert_eq!(
            orphan_logical_browser_id_with_hint(&state, source_session, Some(logical_browser_id))
                .unwrap(),
            logical_browser_id
        );
        state
            .remote_view_handoffs
            .get_mut("r-payment")
            .unwrap()
            .presentation_receipt
            .as_mut()
            .unwrap()
            .daemon_owner_generation = Some(8);
        assert!(
            durable_orphan_runtime_profile(&state, source_session, logical_browser_id).is_err()
        );
    }
    use crate::native::service_model::{
        BrowserProcess, BrowserSession, BrowserTab, DisplayAllocation, LeaseState,
        ProfileLeaseDisposition, RemoteViewHandoff, RemoteViewRoute, ServiceState,
        ServiceTabHandle,
    };

    #[test]
    fn handoff_transfers_profile_lease_to_candidate_session() {
        let logical_browser_id = "browser-a";
        let mut state = ServiceState {
            profiles: std::collections::BTreeMap::from([(
                "social-profile".to_string(),
                BrowserProfile {
                    id: "social-profile".to_string(),
                    ..BrowserProfile::default()
                },
            )]),
            sessions: std::collections::BTreeMap::from([
                (
                    "old-owner".to_string(),
                    BrowserSession {
                        id: "old-owner".to_string(),
                        service_name: Some("x".to_string()),
                        lease: LeaseState::Exclusive,
                        profile_id: Some("social-profile".to_string()),
                        profile_lease_disposition: Some(ProfileLeaseDisposition::ReusedBrowser),
                        cleanup: SessionCleanupPolicy::CloseTabs,
                        browser_ids: vec![logical_browser_id.to_string()],
                        tab_ids: vec!["tab-a".to_string()],
                        ..BrowserSession::default()
                    },
                ),
                (
                    "candidate".to_string(),
                    BrowserSession {
                        id: "candidate".to_string(),
                        ..BrowserSession::default()
                    },
                ),
                (
                    "stale-alias".to_string(),
                    BrowserSession {
                        id: "stale-alias".to_string(),
                        lease: LeaseState::Exclusive,
                        profile_id: Some("social-profile".to_string()),
                        browser_ids: vec![logical_browser_id.to_string()],
                        ..BrowserSession::default()
                    },
                ),
            ]),
            browsers: std::collections::BTreeMap::from([(
                logical_browser_id.to_string(),
                BrowserProcess {
                    id: logical_browser_id.to_string(),
                    active_session_ids: vec!["old-owner".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            tabs: std::collections::BTreeMap::from([(
                "tab-a".to_string(),
                BrowserTab {
                    id: "tab-a".to_string(),
                    browser_id: logical_browser_id.to_string(),
                    session_id: Some("old-owner".to_string()),
                    owner_session_id: Some("old-owner".to_string()),
                    ..BrowserTab::default()
                },
            )]),
            ..ServiceState::default()
        };
        state.refresh_service_tab_handles();
        assert_eq!(
            state.tabs["tab-a"]
                .service_tab_handle
                .as_ref()
                .and_then(|handle| handle.session_name.as_deref()),
            Some("old-owner")
        );

        rebind_runtime_handoff_service_projection_in_state(
            &mut state,
            logical_browser_id,
            &std::collections::BTreeSet::from(["old-owner".to_string()]),
            "candidate",
        )
        .unwrap();

        let old_owner = &state.sessions["old-owner"];
        assert_eq!(old_owner.lease, LeaseState::Released);
        assert!(old_owner.browser_ids.is_empty());
        assert!(old_owner.tab_ids.is_empty());
        let stale_alias = &state.sessions["stale-alias"];
        assert_eq!(stale_alias.lease, LeaseState::Released);
        assert!(stale_alias.browser_ids.is_empty());

        let candidate = &state.sessions["candidate"];
        assert_eq!(candidate.service_name.as_deref(), Some("x"));
        assert_eq!(candidate.lease, LeaseState::Exclusive);
        assert_eq!(candidate.profile_id.as_deref(), Some("social-profile"));
        assert_eq!(
            candidate.profile_lease_disposition,
            Some(ProfileLeaseDisposition::ReusedBrowser)
        );
        assert_eq!(candidate.browser_ids, [logical_browser_id]);
        assert_eq!(candidate.tab_ids, ["tab-a"]);
        assert_eq!(candidate.cleanup, SessionCleanupPolicy::CloseTabs);
        assert_eq!(
            state.browsers[logical_browser_id].active_session_ids,
            ["candidate"]
        );
        assert_eq!(state.tabs["tab-a"].session_id.as_deref(), Some("candidate"));
        assert_eq!(
            state.tabs["tab-a"].owner_session_id.as_deref(),
            Some("candidate")
        );
        let tab_handle = state.tabs["tab-a"].service_tab_handle.as_ref().unwrap();
        assert_eq!(tab_handle.session_name.as_deref(), Some("candidate"));
        assert_eq!(tab_handle.owner_session_id.as_deref(), Some("candidate"));
        assert_eq!(
            tab_handle.cleanup_policy,
            Some(SessionCleanupPolicy::CloseTabs)
        );
        assert_eq!(
            state.browsers[logical_browser_id].tab_handles[0],
            *tab_handle
        );
    }

    #[test]
    fn handoff_repairs_stale_single_browser_profile_lease_from_browser_authority() {
        let logical_browser_id = "browser-a";
        let mut state = ServiceState {
            sessions: std::collections::BTreeMap::from([
                (
                    "old-owner".to_string(),
                    BrowserSession {
                        id: "old-owner".to_string(),
                        lease: LeaseState::Exclusive,
                        profile_id: Some("default".to_string()),
                        browser_ids: vec![logical_browser_id.to_string()],
                        tab_ids: vec!["tab-a".to_string()],
                        ..BrowserSession::default()
                    },
                ),
                (
                    "candidate".to_string(),
                    BrowserSession {
                        id: "candidate".to_string(),
                        profile_id: Some("default".to_string()),
                        ..BrowserSession::default()
                    },
                ),
            ]),
            browsers: std::collections::BTreeMap::from([(
                logical_browser_id.to_string(),
                BrowserProcess {
                    id: logical_browser_id.to_string(),
                    profile_id: Some("last30days-facebook".to_string()),
                    active_session_ids: vec!["old-owner".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            tabs: std::collections::BTreeMap::from([(
                "tab-a".to_string(),
                BrowserTab {
                    id: "tab-a".to_string(),
                    browser_id: logical_browser_id.to_string(),
                    session_id: Some("old-owner".to_string()),
                    owner_session_id: Some("old-owner".to_string()),
                    ..BrowserTab::default()
                },
            )]),
            ..ServiceState::default()
        };

        rebind_runtime_handoff_service_projection_in_state(
            &mut state,
            logical_browser_id,
            &std::collections::BTreeSet::from(["old-owner".to_string()]),
            "candidate",
        )
        .unwrap();

        assert_eq!(state.sessions["old-owner"].lease, LeaseState::Released);
        assert_eq!(
            state.sessions["candidate"].profile_id.as_deref(),
            Some("last30days-facebook")
        );
        assert_eq!(
            state.browsers[logical_browser_id].active_session_ids,
            ["candidate"]
        );
    }

    #[test]
    fn handoff_rejects_profile_repair_for_a_multi_browser_session() {
        let logical_browser_id = "browser-a";
        let mut state = ServiceState {
            sessions: std::collections::BTreeMap::from([
                (
                    "old-owner".to_string(),
                    BrowserSession {
                        id: "old-owner".to_string(),
                        lease: LeaseState::Exclusive,
                        profile_id: Some("default".to_string()),
                        browser_ids: vec![logical_browser_id.to_string(), "browser-b".to_string()],
                        ..BrowserSession::default()
                    },
                ),
                (
                    "candidate".to_string(),
                    BrowserSession {
                        id: "candidate".to_string(),
                        ..BrowserSession::default()
                    },
                ),
            ]),
            browsers: std::collections::BTreeMap::from([(
                logical_browser_id.to_string(),
                BrowserProcess {
                    id: logical_browser_id.to_string(),
                    profile_id: Some("last30days-facebook".to_string()),
                    active_session_ids: vec!["old-owner".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };

        let error = rebind_runtime_handoff_service_projection_in_state(
            &mut state,
            logical_browser_id,
            &std::collections::BTreeSet::from(["old-owner".to_string()]),
            "candidate",
        )
        .unwrap_err();

        assert!(error.starts_with("runtime_handoff_profile_projection_conflict:"));
        assert_eq!(state.sessions["old-owner"].lease, LeaseState::Exclusive);
        assert_eq!(
            state.sessions["old-owner"].browser_ids,
            [logical_browser_id, "browser-b"]
        );
    }

    #[test]
    fn rollback_restores_browser_and_tab_projection_to_source_session() {
        let mut state = ServiceState {
            browsers: std::collections::BTreeMap::from([(
                "browser-a".to_string(),
                BrowserProcess {
                    id: "browser-a".to_string(),
                    active_session_ids: vec!["candidate-session".to_string()],
                    tab_handles: vec![ServiceTabHandle {
                        browser_id: "browser-a".to_string(),
                        session_name: Some("candidate-session".to_string()),
                        owner_session_id: Some("candidate-session".to_string()),
                        lease_id: Some("candidate-session".to_string()),
                        trace_filter: crate::native::service_model::ServiceTabHandleTraceFilter {
                            session_id: Some("candidate-session".to_string()),
                            ..Default::default()
                        },
                        valid: true,
                        ..ServiceTabHandle::default()
                    }],
                    ..BrowserProcess::default()
                },
            )]),
            tabs: std::collections::BTreeMap::from([(
                "tab-a".to_string(),
                BrowserTab {
                    id: "tab-a".to_string(),
                    browser_id: "browser-a".to_string(),
                    session_id: Some("candidate-session".to_string()),
                    owner_session_id: Some("candidate-session".to_string()),
                    service_tab_handle: Some(ServiceTabHandle {
                        browser_id: "browser-a".to_string(),
                        session_name: Some("candidate-session".to_string()),
                        owner_session_id: Some("candidate-session".to_string()),
                        lease_id: Some("candidate-session".to_string()),
                        trace_filter: crate::native::service_model::ServiceTabHandleTraceFilter {
                            session_id: Some("candidate-session".to_string()),
                            ..Default::default()
                        },
                        ..ServiceTabHandle::default()
                    }),
                    ..BrowserTab::default()
                },
            )]),
            remote_view_routes: std::collections::BTreeMap::from([(
                "route-a".to_string(),
                RemoteViewRoute {
                    id: "route-a".to_string(),
                    browser_id: Some("browser-a".to_string()),
                    session_id: Some("legacy-session".to_string()),
                    ..RemoteViewRoute::default()
                },
            )]),
            display_allocations: std::collections::BTreeMap::from([(
                "display-a".to_string(),
                DisplayAllocation {
                    id: "display-a".to_string(),
                    owner_browser_id: Some("browser-a".to_string()),
                    owner_session_id: Some("legacy-session".to_string()),
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_handoffs: std::collections::BTreeMap::from([(
                "handoff-a".to_string(),
                RemoteViewHandoff {
                    id: "handoff-a".to_string(),
                    browser_id: Some("browser-a".to_string()),
                    session_name: Some("legacy-session".to_string()),
                    last_resolution: Some(json!({
                        "sessionName": "legacy-session",
                        "status": "ready"
                    })),
                    ..RemoteViewHandoff::default()
                },
            )]),
            ..ServiceState::default()
        };

        restore_runtime_handoff_service_projection_in_state(
            &mut state,
            "browser-a",
            "candidate-session",
            "source-session",
        )
        .unwrap();

        let browser = &state.browsers["browser-a"];
        assert_eq!(browser.active_session_ids, ["source-session"]);
        let tab = &browser.tab_handles[0];
        assert_eq!(tab.session_name.as_deref(), Some("source-session"));
        assert_eq!(tab.owner_session_id.as_deref(), Some("source-session"));
        assert_eq!(tab.lease_id.as_deref(), Some("source-session"));
        assert_eq!(
            tab.trace_filter.session_id.as_deref(),
            Some("source-session")
        );
        let retained_tab = &state.tabs["tab-a"];
        assert_eq!(retained_tab.session_id.as_deref(), Some("source-session"));
        assert_eq!(
            retained_tab.owner_session_id.as_deref(),
            Some("source-session")
        );
        let retained_handle = retained_tab.service_tab_handle.as_ref().unwrap();
        assert_eq!(
            retained_handle.session_name.as_deref(),
            Some("source-session")
        );
        assert_eq!(
            retained_handle.owner_session_id.as_deref(),
            Some("source-session")
        );
        assert_eq!(
            state.remote_view_routes["route-a"].session_id.as_deref(),
            Some("source-session")
        );
        assert_eq!(
            state.display_allocations["display-a"]
                .owner_session_id
                .as_deref(),
            Some("source-session")
        );
        let handoff = &state.remote_view_handoffs["handoff-a"];
        assert_eq!(handoff.session_name.as_deref(), Some("source-session"));
        assert_eq!(
            handoff
                .last_resolution
                .as_ref()
                .and_then(|resolution| resolution.get("sessionName"))
                .and_then(Value::as_str),
            Some("source-session")
        );
    }
}
