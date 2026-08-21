#![allow(unused_imports)]
use super::action_runtime::runtime::{
    service_browser_id, validate_service_tab_handle_for_current_session, DaemonState,
    RuntimeHandoffDescriptor, TrackedRequest,
};
use super::browser_navigation::handle_reload;
use super::interaction::{
    handle_clear, handle_click, handle_dialog, handle_fill, handle_focus, handle_select,
    handle_type, handle_wait,
};
use super::network::matches_status_filter;
use super::service_model::{LeaseState, ServiceState};
use crate::native::interaction;
use crate::native::screenshot::{self, ScreenshotOptions};
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use crate::native::state;
use crate::runtime_owner_transfer::ProfileOwnerState;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
pub(crate) async fn handle_service_diagnostics(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "diagnostics requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_for_current_session(handle, &state.session_id)?;
    let target_id = handle.get("targetId").and_then(Value::as_str);
    let observed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let max_console_entries = bounded_usize(cmd, "maxConsoleEntries", 10, 50);
    let max_error_entries = bounded_usize(cmd, "maxErrorEntries", 10, 50);
    let max_request_entries = bounded_usize(cmd, "maxRequestEntries", 10, 50);
    let include_screenshot = cmd
        .get("includeScreenshot")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let (url, title, active_target_id, active_session_id, screenshot) =
        if let Some(mgr) = state.browser.as_mut() {
            if let Some(target_id) = target_id {
                if mgr.active_target_id().ok() != Some(target_id) {
                    let _ = mgr.tab_switch_target_id(target_id).await?;
                }
            }
            let active_target_id = mgr.active_target_id().ok().map(ToString::to_string);
            let active_session_id = mgr.active_session_id().ok().map(ToString::to_string);
            let url = mgr.get_url().await.unwrap_or_default();
            let title = mgr.get_title().await.unwrap_or_default();
            let screenshot = if include_screenshot {
                if let Some(session_id) = active_session_id.as_deref() {
                    let options = ScreenshotOptions {
                        selector: None,
                        path: None,
                        full_page: false,
                        format: "png".to_string(),
                        quality: None,
                        annotate: false,
                        output_dir: cmd
                            .get("screenshotDir")
                            .and_then(Value::as_str)
                            .map(String::from),
                    };
                    match screenshot::take_screenshot(
                        &mgr.client,
                        session_id,
                        &state.ref_map,
                        &options,
                        &state.iframe_sessions,
                    )
                    .await
                    {
                        Ok(result) => Some(json!({ "captured" : true, "path" : result.path, })),
                        Err(error) => Some(json!({ "captured" : false, "error" : error, })),
                    }
                } else {
                    None
                }
            } else {
                None
            };
            (url, title, active_target_id, active_session_id, screenshot)
        } else {
            (String::new(), String::new(), None, None, None)
        };
    let browser_id = handle
        .get("browserId")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| service_browser_id(&state.session_id));
    let tab_id = handle
        .get("tabId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let profile_id = handle
        .get("profileId")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let mut service_state = LockedServiceStateRepository::default_json()
        .and_then(|repository| repository.load_snapshot())
        .ok();
    if let Some(service_state) = service_state.as_mut() {
        service_state.refresh_derived_views();
    }
    let browser_record = service_state
        .as_ref()
        .and_then(|service_state| service_state.browsers.get(&browser_id))
        .cloned();
    let tab_record = service_state
        .as_ref()
        .and_then(|service_state| service_state.tabs.get(&tab_id))
        .cloned();
    let session_name = handle
        .get("sessionName")
        .and_then(Value::as_str)
        .or(active_session_id.as_deref())
        .unwrap_or(&state.session_id)
        .to_string();
    let session_record = service_state
        .as_ref()
        .and_then(|service_state| service_state.sessions.get(&session_name))
        .cloned();
    let profile_record = profile_id.as_deref().and_then(|profile_id| {
        service_state
            .as_ref()
            .and_then(|service_state| service_state.profiles.get(profile_id))
            .cloned()
    });
    let control_plane_attestation = service_state
        .as_ref()
        .map(|service_state| {
            service_control_plane_attestation(
                service_state,
                handle,
                &browser_id,
                &session_name,
                &tab_id,
                profile_id.as_deref(),
                &observed_at,
            )
        })
        .transpose()?
        .unwrap_or_else(|| unavailable_control_plane_attestation(&observed_at));
    let routes = service_state
        .as_ref()
        .map(|service_state| {
            service_state
                .remote_view_routes
                .values()
                .filter(|route| {
                    route.browser_id.as_deref() == Some(browser_id.as_str())
                        || route.session_id.as_deref() == Some(session_name.as_str())
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let console = cap_array_items(
        state.event_tracker.get_console_json(),
        "messages",
        max_console_entries,
    );
    let errors = cap_array_items(
        state.event_tracker.get_errors_json(),
        "errors",
        max_error_entries,
    );
    let requests = recent_request_summaries(&state.tracked_requests, max_request_entries);
    Ok(json!(
        { "ok" : true, "action" : "diagnostics", "observedAt" : observed_at,
        "compact" : true, "browserId" : browser_id, "sessionName" : session_name,
        "tabId" : tab_id, "targetId" : target_id.or(active_target_id.as_deref()),
        "activeSessionId" : active_session_id, "profileId" : profile_id,
        "profileOrigin" : handle.get("profileOrigin").cloned()
        .unwrap_or(Value::Null), "url" : if url.is_empty() { handle.get("url")
        .cloned().unwrap_or(Value::Null) } else { json!(url) }, "title" : if title
        .is_empty() { handle.get("title").cloned().unwrap_or(Value::Null) } else {
        json!(title) }, "serviceTabHandle" : cmd.get("serviceTabHandle").cloned()
        .unwrap_or(Value::Null), "traceFilter" : handle.get("traceFilter").cloned()
        .unwrap_or(Value::Null), "browser" : browser_record.as_ref().map(| browser |
        json!({ "id" : browser.id, "profileId" : browser.profile_id, "host" : browser
        .host, "health" : browser.health, "displayIsolation" : browser
        .display_isolation, "displayName" : browser.display_name,
        "displayAllocationId" : browser.display_allocation_id, "pid" : browser.pid,
        "activeSessionIds" : browser.active_session_ids, "viewStreams" : browser
        .view_streams, "lastError" : browser.last_error, "lastHealthObservation" :
        browser.last_health_observation, })), "session" : session_record.as_ref()
        .map(| session | json!({ "id" : session.id, "serviceName" : session
        .service_name, "agentName" : session.agent_name, "taskName" : session
        .task_name, "lease" : session.lease, "cleanup" : session.cleanup, "profileId"
        : session.profile_id, "browserIds" : session.browser_ids, "tabIds" : session
        .tab_ids, })), "tab" : tab_record.as_ref().map(| tab | json!({ "id" : tab.id,
        "browserId" : tab.browser_id, "targetId" : tab.target_id, "lifecycle" : tab
        .lifecycle, "url" : tab.url, "title" : tab.title, "ownerSessionId" : tab
        .owner_session_id, "latestSnapshotId" : tab.latest_snapshot_id,
        "latestScreenshotId" : tab.latest_screenshot_id, "challengeId" : tab
        .challenge_id, })), "profile" : profile_record.as_ref().map(| profile |
        json!({ "id" : profile.id, "name" : profile.name, "profileOrigin" : profile
        .profile_origin, "targetServiceIds" : profile.target_service_ids,
        "authenticatedServiceIds" : profile.authenticated_service_ids, "accountIds" :
        profile.account_ids, "browserBuild" : profile.browser_build, "allocation" :
        profile.allocation, "targetReadiness" : profile.target_readiness,
        "registration" : profile.registration, "browserCompatibilityEvidence" :
        profile.browser_compatibility_evidence, })), "controlPlaneAttestation" :
        control_plane_attestation, "remoteViewRoutes" : routes,
        "snapshotSummary" : { "refCount" : state.ref_map.entries_sorted().len(),
        "hasActiveFrame" : state.active_frame_id.is_some(), "latestSnapshotId" :
        tab_record.as_ref().and_then(| tab | tab.latest_snapshot_id.clone()), },
        "screenshot" : screenshot.unwrap_or_else(|| json!({ "captured" : false,
        "reason" : if include_screenshot { "unavailable" } else { "not_requested" },
        })), "console" : console, "errors" : errors, "requests" : { "count" : state
        .tracked_requests.len(), "returned" : requests.len(), "items" : requests, },
        "caller" : { "serviceName" : cmd.get("serviceName").cloned()
        .unwrap_or(Value::Null), "agentName" : cmd.get("agentName").cloned()
        .unwrap_or(Value::Null), "taskName" : cmd.get("taskName").cloned()
        .unwrap_or(Value::Null), "jobId" : cmd.get("id").cloned()
        .unwrap_or(Value::Null), }, }
    ))
}

fn unavailable_control_plane_attestation(observed_at: &str) -> Value {
    json!({
        "schemaVersion": "agent-browser.service-control-plane-attestation.v1",
        "observedAt": observed_at,
        "complete": false,
        "browserOwner": Value::Null,
        "processIdentity": Value::Null,
        "profileLease": Value::Null,
        "handoffReceipt": Value::Null,
        "missingProofs": [
            "service_state",
            "browser_owner",
            "process_identity",
            "profile_lease",
            "handoff_receipt",
        ],
    })
}

/// Project the exact current effect authority for one service tab handle.
///
/// Every proof is derived from the same persisted service snapshot. Missing or
/// conflicting owner, process, profile-lease, or handoff evidence keeps the
/// attestation incomplete so callers can fail closed before an input action.
fn service_control_plane_attestation(
    service_state: &ServiceState,
    handle: &Map<String, Value>,
    browser_id: &str,
    session_name: &str,
    tab_id: &str,
    profile_id: Option<&str>,
    observed_at: &str,
) -> Result<Value, String> {
    let browser = service_state.browsers.get(browser_id);
    let session = service_state.sessions.get(session_name);
    let tab = service_state.tabs.get(tab_id);
    let owner = service_state
        .runtime_owner_registry
        .attestation_for_session(session_name)?;

    let owner_authoritative = owner.as_ref().is_some_and(|owner| {
        owner.effect_capable
            && owner.owner_state == ProfileOwnerState::Ready
            && owner.logical_browser_id == browser_id
            && owner.daemon_session_route == session_name
    });
    let browser_owner = owner.as_ref().map(|owner| {
        json!({
            "ownerId": owner.owner_id,
            "ownerGeneration": owner.owner_generation,
            "ownerState": owner.owner_state,
            "logicalBrowserId": owner.logical_browser_id,
            "daemonSessionRoute": owner.daemon_session_route,
            "processInstanceDigest": owner.process_instance_digest,
            "effectCapable": owner.effect_capable,
            "authoritative": owner_authoritative,
        })
    });

    let process_identity = service_state.browser_process_identities.get(browser_id);
    let process_projection = process_identity
        .map(|identity| {
            let serialized = serde_json::to_vec(&identity.process_identity).map_err(|error| {
                format!("could not serialize service browser process identity: {error}")
            })?;
            let process_instance_digest = format!("{:x}", Sha256::digest(serialized));
            let matches_owner = browser
                .and_then(|browser| browser.pid)
                .is_some_and(|pid| pid == identity.process_identity.pid)
                && owner
                    .as_ref()
                    .is_some_and(|owner| owner.process_instance_digest == process_instance_digest);
            Ok::<Value, String>(json!({
                "pid": identity.process_identity.pid,
                "startToken": identity.process_identity.start_token,
                "processInstanceDigest": process_instance_digest,
                "matchesOwner": matches_owner,
            }))
        })
        .transpose()?;
    let process_authoritative = process_projection
        .as_ref()
        .and_then(|projection| projection.get("matchesOwner"))
        .and_then(Value::as_bool)
        == Some(true);

    let profile_conflict = profile_id.is_some_and(|profile_id| {
        service_state
            .sessions
            .iter()
            .any(|(candidate_id, candidate)| {
                candidate_id != session_name
                    && candidate.profile_id.as_deref() == Some(profile_id)
                    && candidate.lease == LeaseState::Exclusive
            })
    });
    let lease_active = browser.is_some()
        && tab.is_some()
        && profile_id.is_some_and(|profile_id| service_state.profiles.contains_key(profile_id))
        && session.is_some_and(|session| {
            session.id == session_name
                && session.lease == LeaseState::Exclusive
                && session.profile_id.as_deref() == profile_id
                && session.browser_ids.iter().any(|id| id == browser_id)
                && session.tab_ids.iter().any(|id| id == tab_id)
        })
        && browser.is_some_and(|browser| {
            browser.profile_id.as_deref() == profile_id
                && browser
                    .active_session_ids
                    .iter()
                    .any(|id| id == session_name)
        })
        && tab.is_some_and(|tab| {
            tab.browser_id == browser_id
                && tab
                    .owner_session_id
                    .as_deref()
                    .or(tab.session_id.as_deref())
                    == Some(session_name)
        })
        && handle.get("valid").and_then(Value::as_bool) == Some(true)
        && handle.get("leaseId").and_then(Value::as_str) == Some(session_name)
        && handle.get("leaseState").and_then(Value::as_str) == Some("exclusive")
        && !profile_conflict;
    let profile_lease = session.map(|session| {
        json!({
            "id": handle.get("leaseId").cloned().unwrap_or(Value::Null),
            "mode": session.lease,
            "state": if lease_active { "active" } else { "unproven" },
            "holder": {
                "sessionId": session.id,
                "serviceName": session.service_name,
                "agentName": session.agent_name,
                "taskName": session.task_name,
            },
            "lastObservedAt": session.last_lease_observed_at,
            "expiresAt": session.expires_at,
            "conflictSessionIds": session.profile_lease_conflict_session_ids,
        })
    });

    let handoff_receipt = owner
        .as_ref()
        .and_then(|owner| owner.handoff_receipt.as_ref())
        .map(|receipt| {
            json!({
                "id": receipt.receipt_id,
                "sha256": receipt.receipt_sha256,
                "transitionKind": receipt.transition_kind,
                "ownerGeneration": receipt.owner_generation,
                "state": receipt.state,
            })
        });
    let handoff_accepted = handoff_receipt
        .as_ref()
        .and_then(|receipt| receipt.get("state"))
        .and_then(Value::as_str)
        == Some("accepted");

    let mut missing_proofs = Vec::new();
    if !owner_authoritative {
        missing_proofs.push("browser_owner");
    }
    if !process_authoritative {
        missing_proofs.push("process_identity");
    }
    if !lease_active {
        missing_proofs.push("profile_lease");
    }
    if !handoff_accepted {
        missing_proofs.push("handoff_receipt");
    }
    Ok(json!({
        "schemaVersion": "agent-browser.service-control-plane-attestation.v1",
        "observedAt": observed_at,
        "complete": missing_proofs.is_empty(),
        "browserOwner": browser_owner,
        "processIdentity": process_projection,
        "profileLease": profile_lease,
        "handoffReceipt": handoff_receipt,
        "missingProofs": missing_proofs,
    }))
}
pub(crate) fn bounded_usize(
    cmd: &Value,
    key: &str,
    default_value: usize,
    max_value: usize,
) -> usize {
    cmd.get(key)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
        .min(max_value)
}
pub(crate) fn cap_array_items(mut value: Value, key: &str, limit: usize) -> Value {
    let total = value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if let Some(items) = value.get_mut(key).and_then(Value::as_array_mut) {
        if items.len() > limit {
            let keep_from = items.len().saturating_sub(limit);
            *items = items.split_off(keep_from);
        }
    }
    let returned = value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if let Some(obj) = value.as_object_mut() {
        obj.insert("count".to_string(), json!(total));
        obj.insert("returned".to_string(), json!(returned));
        obj.insert("truncated".to_string(), json!(total > limit));
    }
    value
}
pub(crate) fn recent_request_summaries(requests: &[TrackedRequest], limit: usize) -> Vec<Value> {
    let keep_from = requests.len().saturating_sub(limit);
    requests
        .iter()
        .skip(keep_from)
        .map(|request| {
            json!(
                { "requestId" : request.request_id, "url" : request.url, "method" :
                request.method, "status" : request.status, "resourceType" : request
                .resource_type, "mimeType" : request.mime_type, }
            )
        })
        .collect()
}
pub(crate) fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserHealth, BrowserProcess, BrowserProfile, BrowserSession, BrowserTab,
        ServiceBrowserProcessIdentity, SessionCleanupPolicy, TabLifecycle,
    };
    use crate::process_identity::RecordedProcessIdentity;
    use crate::runtime_adoption::BrowserAdoptionMode;
    use crate::runtime_owner_transfer::{
        CandidateOwnerAttachment, OwnerTransferRequest, ProfileOwner, RuntimeOwnerRegistry,
    };

    fn digest(value: impl AsRef<[u8]>) -> String {
        format!("{:x}", Sha256::digest(value))
    }

    fn process_identity() -> RecordedProcessIdentity {
        RecordedProcessIdentity {
            pid: 4242,
            start_token: "process-start-1".to_string(),
            executable_path: None,
            browser_family: Some("chromium".to_string()),
        }
    }

    fn state_with_attestation(include_handoff: bool) -> ServiceState {
        let process_identity = process_identity();
        let process_instance_digest = digest(serde_json::to_vec(&process_identity).unwrap());
        let profile_identity_digest = digest("profile-1");
        let initial_owner = ProfileOwner {
            owner_id: if include_handoff {
                "owner-old".to_string()
            } else {
                "owner-current".to_string()
            },
            profile_identity_digest: profile_identity_digest.clone(),
            state: ProfileOwnerState::Ready,
            owner_generation: 1,
            browser_id: "browser-1".to_string(),
            daemon_session_route: if include_handoff {
                "session-old".to_string()
            } else {
                "session-1".to_string()
            },
            process_instance_digest: process_instance_digest.clone(),
            browser_family: "chromium".to_string(),
            cdp_endpoint_identity_digest: digest("cdp-1"),
            target_set_digest: digest("target-set-1"),
            pending_transfer: None,
            last_transition: None,
        };
        let mut runtime_owner_registry = RuntimeOwnerRegistry::from_owner(initial_owner);
        if include_handoff {
            let request = OwnerTransferRequest {
                mode: BrowserAdoptionMode::CooperativeTransfer,
                logical_browser_id: "browser-1".to_string(),
                profile_identity_digest,
                expected_owner_id: Some("owner-old".to_string()),
                expected_owner_generation: 1,
                candidate_owner_id: "owner-current".to_string(),
                candidate_daemon_session_route: "session-1".to_string(),
                process_instance_digest: process_instance_digest.clone(),
                browser_family: "chromium".to_string(),
                cdp_endpoint_identity_digest: digest("cdp-1"),
                target_set_digest: digest("target-set-1"),
                selected_target_identity_digest: digest("target-1"),
                transfer_nonce_digest: digest("transfer-1"),
            };
            runtime_owner_registry
                .begin_transfer(request.clone())
                .unwrap();
            runtime_owner_registry
                .commit_candidate(CandidateOwnerAttachment::from_request(&request, 2))
                .unwrap();
        }

        let mut state = ServiceState {
            runtime_owner_registry,
            ..ServiceState::default()
        };
        state.profiles.insert(
            "profile-1".to_string(),
            BrowserProfile {
                id: "profile-1".to_string(),
                name: "Attestation fixture profile".to_string(),
                ..BrowserProfile::default()
            },
        );
        state.browsers.insert(
            "browser-1".to_string(),
            BrowserProcess {
                id: "browser-1".to_string(),
                profile_id: Some("profile-1".to_string()),
                health: BrowserHealth::Ready,
                pid: Some(4242),
                active_session_ids: vec!["session-1".to_string()],
                ..BrowserProcess::default()
            },
        );
        state.browser_process_identities.insert(
            "browser-1".to_string(),
            ServiceBrowserProcessIdentity {
                process_identity,
                user_data_dir: None,
                runtime_profile: Some("runtime-profile-1".to_string()),
            },
        );
        state.sessions.insert(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                service_name: Some("service-1".to_string()),
                agent_name: Some("agent-1".to_string()),
                task_name: Some("task-1".to_string()),
                lease: LeaseState::Exclusive,
                profile_id: Some("profile-1".to_string()),
                cleanup: SessionCleanupPolicy::ReleaseOnly,
                browser_ids: vec!["browser-1".to_string()],
                tab_ids: vec!["tab-1".to_string()],
                last_lease_observed_at: Some("2026-08-21T22:00:00Z".to_string()),
                ..BrowserSession::default()
            },
        );
        state.tabs.insert(
            "tab-1".to_string(),
            BrowserTab {
                id: "tab-1".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("target-1".to_string()),
                session_id: Some("session-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("session-1".to_string()),
                ..BrowserTab::default()
            },
        );
        state
    }

    fn handle() -> Map<String, Value> {
        json!({
            "browserId": "browser-1",
            "sessionName": "session-1",
            "tabId": "tab-1",
            "targetId": "target-1",
            "profileId": "profile-1",
            "leaseId": "session-1",
            "leaseState": "exclusive",
            "valid": true,
        })
        .as_object()
        .unwrap()
        .clone()
    }

    #[test]
    fn control_plane_attestation_is_complete_only_for_exact_current_authority() {
        let attestation = service_control_plane_attestation(
            &state_with_attestation(true),
            &handle(),
            "browser-1",
            "session-1",
            "tab-1",
            Some("profile-1"),
            "2026-08-21T22:00:01Z",
        )
        .unwrap();

        assert_eq!(attestation["complete"], true);
        assert_eq!(attestation["missingProofs"], json!([]));
        assert_eq!(attestation["browserOwner"]["authoritative"], true);
        assert_eq!(attestation["processIdentity"]["matchesOwner"], true);
        assert_eq!(attestation["profileLease"]["state"], "active");
        assert_eq!(attestation["handoffReceipt"]["state"], "accepted");
        assert_eq!(
            attestation["handoffReceipt"]["sha256"]
                .as_str()
                .unwrap()
                .len(),
            64
        );
    }

    #[test]
    fn control_plane_attestation_reports_missing_current_handoff() {
        let attestation = service_control_plane_attestation(
            &state_with_attestation(false),
            &handle(),
            "browser-1",
            "session-1",
            "tab-1",
            Some("profile-1"),
            "2026-08-21T22:00:01Z",
        )
        .unwrap();

        assert_eq!(attestation["complete"], false);
        assert_eq!(attestation["handoffReceipt"], Value::Null);
        assert_eq!(attestation["missingProofs"], json!(["handoff_receipt"]));
    }

    #[test]
    fn control_plane_attestation_rejects_browser_profile_mismatch() {
        let mut state = state_with_attestation(true);
        state.browsers.get_mut("browser-1").unwrap().profile_id =
            Some("different-profile".to_string());

        let attestation = service_control_plane_attestation(
            &state,
            &handle(),
            "browser-1",
            "session-1",
            "tab-1",
            Some("profile-1"),
            "2026-08-21T22:00:01Z",
        )
        .unwrap();

        assert_eq!(attestation["complete"], false);
        assert_eq!(attestation["missingProofs"], json!(["profile_lease"]));
    }
}
