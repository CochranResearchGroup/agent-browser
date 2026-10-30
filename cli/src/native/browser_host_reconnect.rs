//! Explicit hosted reconnect and same-task recovery. No viewer grants or action replay.
use agent_browser_service_model::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Duration;

use super::browser_host_control;
use super::remote_view_application_http::RemoteViewApplicationHttp;

fn task<'a>(state: &'a BrowserSessionState, id: &str) -> Result<&'a ManagedBrowserSession, String> {
    if !state.host_bindings.contains_key(id) {
        return Err("browser_reconnect_hosted_task_required".into());
    }
    state
        .sessions
        .get(id)
        .ok_or_else(|| "browser_reconnect_task_missing".into())
}
fn field<'a>(command: &'a Value, key: &str) -> Result<&'a str, String> {
    command[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("browser_reconnect_{key}_required"))
}
/// Compact projection joins the existing host-task watcher; it does not observe providers.
pub(crate) fn metadata(state: &BrowserSessionState, id: &str) -> Value {
    let value = state.hosted_continuity.get(id).cloned().unwrap_or_default();
    json!({"revision":value.revision,"state":value.state,"disconnectHold":value.disconnect_hold,"retained":state.leave_open_session_ids.contains(id),"lossPolicy":value.loss_policy,
        "lastRequestId":value.last_request_id,"observedDesktop":value.observed_desktop.as_ref().map(|d|json!({"desktopId":d.desktop_id,"generation":d.generation})),
        "recoveryDesktop":value.recovery_desktop.as_ref().map(|d|json!({"desktopId":d.desktop_id,"generation":d.generation})),"pending":value.requests.values().any(|r|r.pending),"automaticReplay":false})
}
/// Only current live hosted tasks participate. Historical/ended tasks cannot veto peers.
pub(crate) fn check_state(
    state: &BrowserSessionState,
    name: &str,
    command: &Value,
) -> Result<(), String> {
    if matches!(
        command["action"].as_str(),
        Some(
            "browser_session_reconnect"
                | "browser_session_close"
                | "browser_session_status"
                | "browser_session_retention"
                | "browser_session_reap"
        )
    ) {
        return Ok(());
    }
    for session in state.sessions.values().filter(|s| {
        (!name.is_empty() && s.name == name)
            || command["sessionName"].as_str() == Some(s.name.as_str())
            || command["sessionId"].as_str() == Some(s.id.as_str())
            || command["taskId"].as_str() == Some(s.id.as_str())
            || command["browserId"].as_str() == Some(s.browser_id.as_str())
    }) {
        if let Some(binding) = state.host_bindings.get(&session.id) {
            if state
                .sessions
                .values()
                .filter(|peer| peer.browser_id == session.browser_id)
                .any(|peer| {
                    state
                        .hosted_continuity
                        .get(&peer.id)
                        .is_some_and(|row| row.requests.values().any(|r| r.pending))
                })
            {
                return Err("browser_reconnect_pending_no_replay".into());
            }
            if state
                .hosted_continuity
                .get(&session.id)
                .is_some_and(BrowserHostedContinuity::blocks_automation)
            {
                return Err("browser_reconnect_required: read hosted status and explicitly reconnect/rebind/reopen; no automatic recovery".into());
            }
            if state
                .browsers
                .get(&session.browser_id)
                .and_then(|b| b.desktop.as_ref())
                .is_some_and(|d| {
                    d.desktop_id != binding.desktop.desktop_id
                        || d.generation != binding.desktop.generation
                })
            {
                return Err("browser_reconnect_rebind_required".into());
            }
        }
    }
    Ok(())
}
/// Exact current Native process/window observations, with no acquire/release or launch.
fn observe_desktop(pid: u32) -> Result<RemoteViewFixedDesktop, String> {
    let before = match crate::process_identity::observe_process(pid) {
        crate::process_identity::ProcessObservation::Observed(identity)
            if identity.start_token.is_some() =>
        {
            identity
        }
        _ => return Err("browser_reconnect_process_identity_unverified".into()),
    };
    let origin = std::env::var("AGENT_BROWSER_REMOTE_VIEW_ORIGIN")
        .map_err(|_| "browser_reconnect_native_not_configured")?;
    let application = std::env::var("AGENT_BROWSER_REMOTE_VIEW_APPLICATION")
        .unwrap_or_else(|_| "agent-browser".into());
    let transport = RemoteViewApplicationHttp::new(&origin, Duration::from_secs(5))
        .map_err(|_| "browser_reconnect_native_transport_unavailable")?;
    let mut adapter = RemoteViewApplicationAdapter::new(application, transport)
        .map_err(|_| "browser_reconnect_native_context_invalid")?;
    let inventory = adapter
        .inventory()
        .map_err(|_| "browser_reconnect_native_observation_unavailable")?;
    let mut matches = Vec::new();
    for assignment in inventory
        .assignments
        .iter()
        .filter(|a| a.state == RemoteViewAssignmentState::Active)
    {
        let windows = adapter
            .windows(assignment)
            .map_err(|_| "browser_reconnect_native_observation_unavailable")?;
        if windows
            .windows
            .iter()
            .any(|w| w.pid == Some(pid) && w.width > 0 && w.height > 0)
        {
            adapter
                .observe_assignment(assignment)
                .map_err(|_| "browser_reconnect_native_generation_changed")?;
            matches.push(RemoteViewFixedDesktop {
                desktop_id: assignment.desktop_id.clone(),
                generation: assignment.generation,
                friendly_route_label: "Hosted browser".into(),
            });
        }
    }
    match crate::process_identity::observe_process(pid) {
        crate::process_identity::ProcessObservation::Observed(after)
            if after.start_token == before.start_token
                && after.executable_path == before.executable_path => {}
        _ => return Err("browser_reconnect_process_generation_changed".into()),
    }
    match matches.as_slice() {
        [desktop] => Ok(desktop.clone()),
        _ => Err("browser_reconnect_native_target_unverified".into()),
    }
}
fn same_desktop(a: &RemoteViewFixedDesktop, b: &RemoteViewFixedDesktop) -> bool {
    a.desktop_id == b.desktop_id && a.generation == b.generation
}
/// Refresh only the explicitly observed assignment; this adapter route never allocates capacity.
fn refresh_assignment<E: BrowserSessionEffects>(
    effects: &mut E,
    desktop: &RemoteViewFixedDesktop,
) -> Result<(), String> {
    let placement = BrowserPlacement::Required {
        desktop: BrowserDesktopBinding {
            desktop_id: desktop.desktop_id.clone(),
            generation: desktop.generation,
        },
    };
    if let Some(candidates) = effects.remote_view_placement_candidates(&placement)? {
        if !candidates
            .iter()
            .any(|candidate| candidate.ready && same_desktop(&candidate.desktop, desktop))
        {
            return Err("browser_reconnect_native_assignment_unavailable".into());
        }
    }
    Ok(())
}

fn recovery_candidate<E: BrowserSessionEffects>(
    effects: &mut E,
    requested: BrowserDesktopBinding,
) -> Result<RemoteViewFixedDesktop, String> {
    let candidates = effects
        .remote_view_placement_candidates(&BrowserPlacement::Required {
            desktop: requested.clone(),
        })?
        .ok_or("browser_reconnect_recovery_placement_unsupported")?;
    candidates
        .into_iter()
        .find(|c| {
            c.ready
                && c.desktop.desktop_id == requested.desktop_id
                && c.desktop.generation == requested.generation
        })
        .map(|c| c.desktop)
        .ok_or_else(|| "browser_reconnect_recovery_target_unavailable".into())
}

fn publish_next<E, F>(
    state: &mut BrowserSessionState,
    next: BrowserSessionState,
    effects: &mut E,
    publish: &mut F,
) -> Result<(), String>
where
    F: FnMut(&BrowserSessionState, &BrowserSessionState, &mut E) -> Result<(), String>,
{
    publish(state, &next, effects)?;
    *state = next;
    Ok(())
}

/// The host supplies its ordinary atomic session/custody publisher. Reopen intent
/// is durable before any effect; unknown completion remains pending, never retried.
pub(crate) fn execute<E, F>(
    state: &mut BrowserSessionState,
    catalog: &BrowserProfileCatalog,
    effects: &mut E,
    command: &Value,
    publish: &mut F,
) -> Result<Value, String>
where
    E: BrowserSessionEffects,
    F: FnMut(&BrowserSessionState, &BrowserSessionState, &mut E) -> Result<(), String>,
{
    execute_with_observer(
        state,
        catalog,
        effects,
        command,
        publish,
        &mut observe_desktop,
    )
}
fn execute_with_observer<E, F, O>(
    state: &mut BrowserSessionState,
    catalog: &BrowserProfileCatalog,
    effects: &mut E,
    command: &Value,
    publish: &mut F,
    observe: &mut O,
) -> Result<Value, String>
where
    E: BrowserSessionEffects,
    F: FnMut(&BrowserSessionState, &BrowserSessionState, &mut E) -> Result<(), String>,
    O: FnMut(u32) -> Result<RemoteViewFixedDesktop, String>,
{
    let id = field(command, "sessionId")?;
    let session = task(state, id)?.clone();
    let binding = state
        .host_bindings
        .get(id)
        .ok_or("browser_reconnect_binding_missing")?
        .clone();
    let operation = field(command, "operation")?;
    if operation == "status" {
        return Ok(metadata(state, id));
    }
    super::browser_host_context::validate_provider(&binding.request)?;
    if !matches!(
        operation,
        "disconnect" | "reconnect" | "rebind" | "reopen" | "configure"
    ) {
        return Err("browser_reconnect_operation_invalid".into());
    }
    let request_id = field(command, "requestId")?;
    let revision = command["expectedRevision"]
        .as_u64()
        .ok_or("browser_reconnect_expected_revision_required")?;
    let fingerprint=format!("{:x}",Sha256::digest(serde_json::to_vec(&json!({"taskId":id,"operation":operation,"desktop":command.get("desktop"),"attachment":command.get("attachment"),"lossPolicy":command.get("lossPolicy")})).map_err(|_|"browser_reconnect_request_invalid")?));
    let continuity = state.hosted_continuity.get(id).cloned().unwrap_or_default();
    if !continuity.admit(request_id, &fingerprint, revision)? {
        return Ok(metadata(state, id));
    }
    let browser = state
        .browsers
        .get(&session.browser_id)
        .ok_or("browser_reconnect_browser_record_missing")?
        .clone();
    let mut next = state.clone();
    let mut row = continuity;
    if operation == "configure" {
        row.loss_policy = serde_json::from_value(command["lossPolicy"].clone())
            .map_err(|_| "browser_reconnect_loss_policy_invalid")?;
    } else if operation == "disconnect" {
        if !next.leave_open_session_ids.contains(id) {
            row.disconnect_hold = true;
            next.leave_open_session_ids.insert(id.into());
        }
        if !row.blocks_automation() {
            row.state = BrowserHostedConnectionState::Disconnected;
        }
    } else if operation == "reopen" {
        if effects.pending_launch_intent().is_some() {
            return Err("browser_reconnect_existing_launch_pending".into());
        }
        if row.loss_policy != BrowserHostedLossPolicy::Reopen {
            return Err(
                "browser_reconnect_report_only: app must explicitly select reopen policy".into(),
            );
        }
        let control = browser_host_control::metadata(&browser.id)?;
        if (control["owner"] != "agent" && !state.app_browser_attachments.contains_key(&browser.id))
            || control["pendingOutcomeCount"].as_u64() != Some(0)
        {
            return Err("browser_reconnect_control_or_pending_effect_blocks_reopen".into());
        }
        if !matches!(
            crate::process_identity::observe_process(browser.pid),
            crate::process_identity::ProcessObservation::Missing
        ) {
            return Err("browser_reconnect_process_absence_unproven".into());
        }
        // A shared browser's still-pending lifecycle request cannot be bypassed by a peer task.
        for peer in state
            .sessions
            .values()
            .filter(|s| s.browser_id == browser.id)
        {
            if state
                .hosted_continuity
                .get(&peer.id)
                .is_some_and(|r| r.requests.values().any(|request| request.pending))
            {
                return Err("browser_reconnect_pending_no_replay".into());
            }
        }
        let profile = catalog
            .profiles
            .get(&browser.profile_id)
            .or_else(|| {
                state
                    .disposable_profiles
                    .get(&browser.profile_id)
                    .map(|a| &a.profile)
            })
            .ok_or("browser_reconnect_profile_missing")?
            .clone();
        let tabs = state
            .tabs
            .values()
            .filter(|t| t.browser_id == browser.id)
            .cloned()
            .collect::<Vec<_>>();
        let recovery_desktop = row
            .recovery_desktop
            .clone()
            .unwrap_or_else(|| binding.desktop.clone());
        if !state.app_browser_attachments.contains_key(&browser.id) {
            for peer in state
                .sessions
                .values()
                .filter(|peer| peer.browser_id == browser.id && peer.id != id)
            {
                if let Some(peer_binding) = state.host_bindings.get(&peer.id) {
                    let peer_target = state
                        .hosted_continuity
                        .get(&peer.id)
                        .and_then(|r| r.recovery_desktop.as_ref())
                        .unwrap_or(&peer_binding.desktop);
                    if !same_desktop(peer_target, &recovery_desktop) {
                        return Err("browser_reconnect_shared_recovery_target_conflict".into());
                    }
                }
            }
        }
        let prepared_attachment =
            if let Some(previous) = state.app_browser_attachments.get(&browser.id) {
                if previous.owner != BrowserHostIdentity::from(&binding.request) {
                    return Err("browser_reconnect_attachment_owner_required".into());
                }
                let attachment: BrowserAttachmentRequest =
                    serde_json::from_value(command["attachment"].clone())
                        .map_err(|_| "browser_reconnect_fresh_attachment_required")?;
                attachment.validate()?;
                if attachment.profile_path != profile.user_data_dir
                    || attachment.shared_with != previous.request.shared_with
                {
                    return Err("browser_reconnect_attachment_scope_changed".into());
                }
                let tab_id = session
                    .current_tab_id
                    .as_ref()
                    .ok_or("browser_reconnect_tab_missing")?;
                if !state.tabs.contains_key(tab_id) {
                    return Err("browser_reconnect_tab_missing".into());
                }
                let desktop = observe(attachment.process.pid)?;
                validate_requested_desktop(command, &desktop)?;
                effects.begin_operation(state)?;
                refresh_assignment(effects, &desktop)?;
                Some((previous.clone(), attachment, desktop))
            } else {
                if state.shared_browser_ids.contains(&browser.id) {
                    return Err("browser_reconnect_borrowed_reopen_requires_app_attachment".into());
                }
                effects.begin_operation(state)?;
                if row.recovery_desktop.is_some() {
                    recovery_candidate(
                        effects,
                        BrowserDesktopBinding {
                            desktop_id: recovery_desktop.desktop_id.clone(),
                            generation: recovery_desktop.generation,
                        },
                    )?;
                }
                if let Some(path) = state.profile_executables.get(&browser.profile_id) {
                    let profile_id = browser.profile_id.clone();
                    effects.select_profile_browser_executable(profile_id, path.clone())?;
                }
                None
            };
        row.state = BrowserHostedConnectionState::Pending;
        row.record(request_id, &fingerprint, operation, true);
        next.hosted_continuity.insert(id.into(), row.clone());
        publish_next(state, next, effects, publish)?;
        effects.begin_operation(state)?;
        let mut completed = state.clone();
        if let Some((previous, attachment, desktop)) = prepared_attachment {
            let observed = effects.attach_app_browser(&attachment, &profile, &desktop)?;
            if observed.launch.pid != attachment.process.pid
                || observed.launch.cdp_endpoint != attachment.cdp_endpoint
                || !observed.borrowed_target_ids.contains(&attachment.target_id)
                || observed.assignment.as_ref().is_none_or(|a| {
                    a.desktop_id != desktop.desktop_id || a.generation != desktop.generation
                })
            {
                return Err("browser_reconnect_attachment_observation_mismatch".into());
            }
            let mut retained = previous.clone();
            retained.request = attachment.clone();
            retained.desktop = desktop.clone();
            retained.assignment = observed.assignment;
            retained.borrowed_tab_ids = observed.borrowed_target_ids;
            completed
                .app_browser_attachments
                .insert(browser.id.clone(), retained);
            let current = completed
                .browsers
                .get_mut(&browser.id)
                .ok_or("browser_reconnect_browser_record_missing")?;
            current.pid = attachment.process.pid;
            current.cdp_endpoint = attachment.cdp_endpoint;
            current.desktop = Some(desktop.clone());
            let tab_id = session
                .current_tab_id
                .as_ref()
                .ok_or("browser_reconnect_tab_missing")?;
            completed
                .tabs
                .get_mut(tab_id)
                .ok_or("browser_reconnect_tab_missing")?
                .target_id = attachment.target_id.clone();
            completed
                .attached_task_targets
                .insert(id.into(), attachment.target_id);
            // Other tasks still have old physical targets and must explicitly reconnect.
            for peer in state
                .sessions
                .values()
                .filter(|s| s.browser_id == browser.id && s.id != id)
            {
                let peer_row = completed
                    .hosted_continuity
                    .entry(peer.id.clone())
                    .or_default();
                peer_row.state = BrowserHostedConnectionState::BrowserLost;
                peer_row.revision = peer_row
                    .revision
                    .checked_add(1)
                    .ok_or("browser_reconnect_revision_exhausted")?;
            }
            row.observed_desktop = Some(desktop.clone());
            update_binding(&mut completed, id, &desktop)?;
        } else {
            if state.shared_browser_ids.contains(&browser.id) {
                return Err("browser_reconnect_borrowed_reopen_requires_app_attachment".into());
            }
            // Empty history restores blank targets only. Never replay prior URLs or actions.
            let recovered = if row.recovery_desktop.is_some() {
                effects.recover_browser_at(&browser, &profile, &tabs, &[], &recovery_desktop)?
            } else {
                effects.recover_browser(&browser, &profile, &tabs, &[])?
            };
            if recovered.launch.browser_id != browser.id
                || recovered.launch.pid == 0
                || recovered.target_ids.len() != tabs.len()
                || tabs
                    .iter()
                    .any(|t| recovered.target_ids.get(&t.id).is_none_or(String::is_empty))
            {
                return Err("browser_reconnect_recovery_identity_changed".into());
            }
            let desktop = recovered
                .launch
                .desktop
                .clone()
                .ok_or("browser_reconnect_native_target_unverified")?;
            if !same_desktop(&desktop, &recovery_desktop) {
                return Err("browser_reconnect_rebind_required_after_reopen".into());
            }
            let current = completed
                .browsers
                .get_mut(&browser.id)
                .ok_or("browser_reconnect_browser_record_missing")?;
            current.pid = recovered.launch.pid;
            current.cdp_endpoint = recovered.launch.cdp_endpoint;
            current.desktop = Some(desktop.clone());
            for (tab, target) in recovered.target_ids {
                completed
                    .tabs
                    .get_mut(&tab)
                    .ok_or("browser_reconnect_tab_missing")?
                    .target_id = target;
            }
            completed.idle_closed_browsers.remove(&browser.id);
            // Every active peer was required to approve this exact recovery target above.
            for peer in state
                .sessions
                .values()
                .filter(|peer| peer.browser_id == browser.id)
            {
                if completed.host_bindings.contains_key(&peer.id) {
                    update_binding(&mut completed, &peer.id, &desktop)?;
                }
                if peer.id != id {
                    let current = completed
                        .hosted_continuity
                        .entry(peer.id.clone())
                        .or_default();
                    current.state = BrowserHostedConnectionState::Connected;
                    current.observed_desktop = Some(desktop.clone());
                    current.recovery_desktop = None;
                    current.revision = current
                        .revision
                        .checked_add(1)
                        .ok_or("browser_reconnect_revision_exhausted")?;
                }
            }
            row.observed_desktop = Some(desktop);
        }
        for handoff in completed
            .remote_view_tab_handoffs
            .values_mut()
            .filter(|h| tabs.iter().any(|t| t.id == h.tab_id))
        {
            handoff.view = None;
        }
        row.state = BrowserHostedConnectionState::Connected;
        row.recovery_desktop = None;
        if row.disconnect_hold {
            completed.leave_open_session_ids.remove(id);
            row.disconnect_hold = false;
        }
        row.record(request_id, &fingerprint, operation, false);
        completed.hosted_continuity.insert(id.into(), row);
        publish_next(state, completed, effects, publish)?;
        return Ok(metadata(state, id));
    } else {
        effects.begin_operation(state)?;
        let live = effects.browser_is_live(&browser)?;
        if !live {
            row.state = if matches!(
                crate::process_identity::observe_process(browser.pid),
                crate::process_identity::ProcessObservation::Missing
            ) {
                BrowserHostedConnectionState::BrowserLost
            } else {
                BrowserHostedConnectionState::BrowserUnreachable
            };
            if operation == "rebind" && row.state == BrowserHostedConnectionState::BrowserLost {
                let requested: BrowserDesktopBinding =
                    serde_json::from_value(command["desktop"].clone())
                        .map_err(|_| "browser_reconnect_explicit_desktop_required")?;
                row.recovery_desktop = Some(recovery_candidate(effects, requested)?);
                // BrowserLost remains honest: this is future recovery intent, not a physical move.
            }
        } else {
            let desktop = observe(browser.pid)?;
            if let Some(attachment) = state.app_browser_attachments.get(&browser.id) {
                refresh_assignment(effects, &desktop)?;
                let mut exact = attachment.request.clone();
                exact.target_id = session
                    .current_tab_id
                    .as_ref()
                    .and_then(|id| state.tabs.get(id))
                    .ok_or("browser_reconnect_tab_missing")?
                    .target_id
                    .clone();
                let profile = catalog
                    .profiles
                    .get(&browser.profile_id)
                    .ok_or("browser_reconnect_profile_missing")?;
                // Exact target observation prevents a lost peer page becoming connected merely because its shared process exists.
                let proof = effects.attach_app_browser(&exact, profile, &desktop)?;
                if !proof.borrowed_target_ids.contains(&exact.target_id) {
                    return Err("browser_reconnect_task_target_lost".into());
                }
            }
            row.observed_desktop = Some(desktop.clone());
            if operation == "rebind" {
                if !state.app_browser_attachments.contains_key(&browser.id)
                    && browser
                        .desktop
                        .as_ref()
                        .is_none_or(|old| !same_desktop(old, &desktop))
                {
                    return Err("browser_reconnect_managed_rebind_capability_unavailable: Native launch-custody generation cannot be rewritten; preserve browser and report changed desktop".into());
                }
                refresh_assignment(effects, &desktop)?;
                validate_requested_desktop(command, &desktop)?;
                // Rebind observes one existing target. It never launches/recreates a page.
                if let Some(attachment) = state.app_browser_attachments.get(&browser.id) {
                    let profile = catalog
                        .profiles
                        .get(&browser.profile_id)
                        .ok_or("browser_reconnect_profile_missing")?;
                    let observed =
                        effects.attach_app_browser(&attachment.request, profile, &desktop)?;
                    if observed.launch.pid != browser.pid
                        || observed.launch.cdp_endpoint != browser.cdp_endpoint
                        || observed.assignment.as_ref().is_none_or(|a| {
                            a.desktop_id != desktop.desktop_id || a.generation != desktop.generation
                        })
                    {
                        return Err("browser_reconnect_attachment_observation_mismatch".into());
                    }
                    let current = next
                        .app_browser_attachments
                        .get_mut(&browser.id)
                        .ok_or("browser_reconnect_attachment_missing")?;
                    current.desktop = desktop.clone();
                    current.assignment = observed.assignment;
                }
                next.browsers
                    .get_mut(&browser.id)
                    .ok_or("browser_reconnect_browser_record_missing")?
                    .desktop = Some(desktop.clone());
                update_binding(&mut next, id, &desktop)?;
                row.state = BrowserHostedConnectionState::Connected;
                for handoff in next.remote_view_tab_handoffs.values_mut().filter(|h| {
                    state
                        .tabs
                        .get(&h.tab_id)
                        .is_some_and(|t| t.browser_id == browser.id)
                }) {
                    handoff.view = None;
                }
            } else {
                row.state = if same_desktop(&binding.desktop, &desktop) {
                    BrowserHostedConnectionState::Connected
                } else {
                    BrowserHostedConnectionState::RebindRequired
                };
            }
        }
    }
    if row.state == BrowserHostedConnectionState::Connected && row.disconnect_hold {
        next.leave_open_session_ids.remove(id);
        row.disconnect_hold = false;
    }
    row.record(request_id, &fingerprint, operation, false);
    next.hosted_continuity.insert(id.into(), row);
    publish_next(state, next, effects, publish)?;
    Ok(metadata(state, id))
}
fn validate_requested_desktop(
    command: &Value,
    current: &RemoteViewFixedDesktop,
) -> Result<(), String> {
    let requested: BrowserDesktopBinding = serde_json::from_value(command["desktop"].clone())
        .map_err(|_| "browser_reconnect_explicit_desktop_required")?;
    if requested.desktop_id != current.desktop_id || requested.generation != current.generation {
        return Err("browser_reconnect_desktop_observation_mismatch".into());
    }
    Ok(())
}
fn update_binding(
    state: &mut BrowserSessionState,
    id: &str,
    desktop: &RemoteViewFixedDesktop,
) -> Result<(), String> {
    let binding = state
        .host_bindings
        .get_mut(id)
        .ok_or("browser_reconnect_binding_missing")?;
    binding.desktop = desktop.clone();
    binding.request.placement = BrowserPlacement::Required {
        desktop: BrowserDesktopBinding {
            desktop_id: desktop.desktop_id.clone(),
            generation: desktop.generation,
        },
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;
    use std::collections::BTreeSet;
    struct Effects {
        live: bool,
        recoveries: u32,
        attachments: u32,
        fail_recovery: bool,
    }
    impl BrowserSessionEffects for Effects {
        fn browser_is_live(&mut self, _: &ManagedBrowserInstance) -> Result<bool, String> {
            Ok(self.live)
        }
        fn remote_view_placement_candidates(
            &mut self,
            placement: &BrowserPlacement,
        ) -> Result<Option<Vec<RemoteViewDesktopCandidate>>, String> {
            let desktop = match placement {
                BrowserPlacement::Required { desktop }
                | BrowserPlacement::Preferred { desktop, .. } => desktop,
            };
            Ok(Some(vec![RemoteViewDesktopCandidate {
                desktop: RemoteViewFixedDesktop {
                    desktop_id: desktop.desktop_id.clone(),
                    generation: desktop.generation,
                    friendly_route_label: "ready fixture".into(),
                },
                ready: true,
            }]))
        }
        fn recover_browser_at(
            &mut self,
            browser: &ManagedBrowserInstance,
            profile: &BrowserProfileCatalogEntry,
            tabs: &[ManagedBrowserTab],
            history: &[BrowserNavigationRecord],
            desktop: &RemoteViewFixedDesktop,
        ) -> Result<BrowserRecovery, String> {
            let mut recovered = self.recover_browser(browser, profile, tabs, history)?;
            recovered.launch.desktop = Some(desktop.clone());
            Ok(recovered)
        }
        fn recover_browser(
            &mut self,
            b: &ManagedBrowserInstance,
            _: &BrowserProfileCatalogEntry,
            tabs: &[ManagedBrowserTab],
            history: &[BrowserNavigationRecord],
        ) -> Result<BrowserRecovery, String> {
            self.recoveries += 1;
            assert!(history.is_empty(), "website history must never replay");
            if self.fail_recovery {
                return Err("synthetic_uncertain_launch".into());
            }
            Ok(BrowserRecovery {
                launch: BrowserLaunch {
                    browser_id: b.id.clone(),
                    pid: 4242,
                    cdp_endpoint: "ws://127.0.0.1:9222/devtools/browser/new".into(),
                    desktop: b.desktop.clone(),
                    process_identity: None,
                },
                target_ids: tabs
                    .iter()
                    .map(|t| (t.id.clone(), format!("new-{}", t.target_id)))
                    .collect(),
            })
        }
        fn attach_app_browser(
            &mut self,
            r: &BrowserAttachmentRequest,
            _: &BrowserProfileCatalogEntry,
            d: &RemoteViewFixedDesktop,
        ) -> Result<BrowserAttachmentObservation, String> {
            self.attachments += 1;
            Ok(BrowserAttachmentObservation {
                launch: BrowserLaunch {
                    browser_id: "different-physical-id".into(),
                    pid: r.process.pid,
                    cdp_endpoint: r.cdp_endpoint.clone(),
                    desktop: Some(d.clone()),
                    process_identity: Some(r.process.clone()),
                },
                target_id: r.target_id.clone(),
                borrowed_target_ids: BTreeSet::from([r.target_id.clone()]),
                assignment: Some(RemoteViewAssignmentRecord {
                    assignment_id: "assignment".into(),
                    registration_id: "registration".into(),
                    pool_id: "main".into(),
                    desktop_id: d.desktop_id.clone(),
                    generation: d.generation,
                    state: RemoteViewAssignmentState::Active,
                    presentation: None,
                }),
            })
        }
        fn launch_browser(
            &mut self,
            _: &BrowserProfileCatalogEntry,
            _: Option<&RemoteViewFixedDesktop>,
        ) -> Result<BrowserLaunch, String> {
            panic!("unexpected launch")
        }
        fn close_browser(&mut self, _: &ManagedBrowserInstance) -> Result<(), String> {
            panic!("unexpected close")
        }
        fn acquire_initial_tab(
            &mut self,
            _: &ManagedBrowserInstance,
            _: &[String],
        ) -> Result<BrowserTabAcquisition, String> {
            panic!("unexpected tab acquire")
        }
        fn create_tab(
            &mut self,
            _: &ManagedBrowserInstance,
        ) -> Result<BrowserTabAcquisition, String> {
            panic!("unexpected tab creation")
        }
        fn close_tab(
            &mut self,
            _: &ManagedBrowserInstance,
            _: &ManagedBrowserTab,
        ) -> Result<(), String> {
            panic!("unexpected tab close")
        }
        fn navigate(
            &mut self,
            _: &ManagedBrowserInstance,
            _: &ManagedBrowserTab,
            _: &str,
        ) -> Result<(), String> {
            panic!("unexpected navigation")
        }
        fn focus_browser(
            &mut self,
            _: &ManagedBrowserInstance,
            _: Option<&ManagedBrowserTab>,
        ) -> Result<(), String> {
            panic!("unexpected focus")
        }
        fn allocate_disposable_profile(
            &mut self,
            _: &BrowserDisposableProfilePolicy,
            _: &str,
            _: &str,
        ) -> Result<BrowserProfileCatalogEntry, String> {
            panic!("unexpected allocation")
        }
        fn delete_disposable_profile(
            &mut self,
            _: &ManagedDisposableProfile,
        ) -> Result<(), String> {
            panic!("unexpected deletion")
        }
    }
    fn fixture(run: impl FnOnce(BrowserSessionState, BrowserProfileCatalog, &str)) {
        let guard = EnvGuard::new(&[
            "HOME",
            "AGENT_BROWSER_HOME",
            "AGENT_BROWSER_HOSTS_FILE",
            "AGENT_BROWSER_REMOTE_VIEW_APPLICATION",
            "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        ]);
        let directory =
            std::env::temp_dir().join(format!("hosted-reconnect-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(directory.join(".agent-browser/service")).unwrap();
        let registry = directory.join("hosts.json");
        std::fs::write(&registry,r#"{"version":1,"hosts":{"app":{"origin":"https://app.example.test","providerApplication":"app"}}}"#).unwrap();
        guard.set("HOME", directory.to_str().unwrap());
        guard.set(
            "AGENT_BROWSER_HOME",
            directory.join(".agent-browser").to_str().unwrap(),
        );
        guard.set("AGENT_BROWSER_HOSTS_FILE", registry.to_str().unwrap());
        guard.set("AGENT_BROWSER_REMOTE_VIEW_APPLICATION", "app");
        guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
        let desktop = RemoteViewFixedDesktop {
            desktop_id: "desktop".into(),
            generation: 1,
            friendly_route_label: "fixture".into(),
        };
        let mut state = BrowserSessionState::default();
        let browser_id = format!("browser:{}", uuid::Uuid::new_v4());
        state.browsers.insert(
            browser_id.clone(),
            ManagedBrowserInstance {
                id: browser_id.clone(),
                profile_id: "profile".into(),
                pid: u32::MAX,
                cdp_endpoint: "ws://127.0.0.1:9222/devtools/browser/old".into(),
                desktop: Some(desktop.clone()),
                active_session_ids: vec!["task".into()],
            },
        );
        state.sessions.insert(
            "task".into(),
            ManagedBrowserSession {
                id: "task".into(),
                name: "fixture".into(),
                profile_id: "profile".into(),
                browser_id: browser_id.clone(),
                created_at_ms: 0,
                last_activity_at_ms: 0,
                expires_at_ms: u64::MAX,
                current_tab_id: Some("tab".into()),
            },
        );
        state.tabs.insert(
            "tab".into(),
            ManagedBrowserTab {
                id: "tab".into(),
                target_id: "target".into(),
                browser_id: browser_id.clone(),
                session_id: "task".into(),
                created_at_ms: 0,
                last_activity_at_ms: 0,
            },
        );
        state.host_bindings.insert(
            "task".into(),
            BrowserHostTaskBinding {
                request: BrowserHostBinding {
                    version: 1,
                    host_id: "app".into(),
                    participant_id: "participant".into(),
                    session_id: "app-session".into(),
                    placement: BrowserPlacement::Required {
                        desktop: BrowserDesktopBinding {
                            desktop_id: "desktop".into(),
                            generation: 1,
                        },
                    },
                },
                desktop,
            },
        );
        let mut catalog = BrowserProfileCatalog::default();
        catalog.profiles.insert(
            "profile".into(),
            BrowserProfileCatalogEntry {
                manual_login_url: None,
                id: "profile".into(),
                name: "Fixture".into(),
                user_data_dir: directory.join("profile").to_string_lossy().into(),
                kind: BrowserProfileKind::Named,
            },
        );
        run(state, catalog, &browser_id);
        std::fs::remove_dir_all(directory).unwrap();
    }
    fn command(operation: &str, revision: u64) -> Value {
        json!({"sessionId":"task","operation":operation,"requestId":format!("{operation}-{revision}"),"expectedRevision":revision})
    }
    fn effects(live: bool) -> Effects {
        Effects {
            live,
            recoveries: 0,
            attachments: 0,
            fail_recovery: false,
        }
    }
    fn observe(_: u32) -> Result<RemoteViewFixedDesktop, String> {
        Ok(RemoteViewFixedDesktop {
            desktop_id: "desktop".into(),
            generation: 1,
            friendly_route_label: "current".into(),
        })
    }
    #[test]
    fn hosted_reconnect_disconnect_preserves_task_browser_and_takeover() {
        fixture(|mut state, catalog, browser_id| {
            browser_host_control::change(browser_id, "takeover", 0, "fixture human", "test", true)
                .unwrap();
            let before = state.clone();
            let mut e = effects(true);
            let mut publish =
                |_: &BrowserSessionState, _: &BrowserSessionState, _: &mut Effects| Ok(());
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &command("disconnect", 0),
                &mut publish,
                &mut observe,
            )
            .unwrap();
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &command("reconnect", 1),
                &mut publish,
                &mut observe,
            )
            .unwrap();
            assert_eq!(state.sessions, before.sessions);
            assert_eq!(state.browsers, before.browsers);
            assert_eq!(metadata(&state, "task")["state"], "connected");
            assert_eq!(
                browser_host_control::metadata(browser_id).unwrap()["owner"],
                "human"
            );
            assert_eq!(e.recoveries, 0);
        });
    }
    #[test]
    fn hosted_reconnect_generation_change_requires_explicit_exact_rebind() {
        fixture(|mut state, catalog, _| {
            let mut e = effects(true);
            let mut publish =
                |_: &BrowserSessionState, _: &BrowserSessionState, _: &mut Effects| Ok(());
            let mut changed = |pid| {
                observe(pid).map(|mut d| {
                    d.generation = 2;
                    d
                })
            };
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &command("reconnect", 0),
                &mut publish,
                &mut changed,
            )
            .unwrap();
            assert_eq!(metadata(&state, "task")["state"], "rebind_required");
            assert!(check_state(&state, "fixture", &json!({"action":"click"})).is_err());
            let mut rebind = command("rebind", 1);
            rebind["desktop"] = json!({"desktopId":"desktop","generation":2});
            assert!(execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &rebind,
                &mut publish,
                &mut changed
            )
            .unwrap_err()
            .contains("managed_rebind_capability_unavailable"));
            assert_eq!(state.host_bindings["task"].desktop.generation, 1);
            let browser = state.browsers.values().next().unwrap().clone();
            let request = BrowserAttachmentRequest {
                version: 1,
                process: RecordedProcessIdentity {
                    pid: browser.pid,
                    start_token: "old".into(),
                    executable_path: Some("/synthetic/chrome".into()),
                    browser_family: None,
                },
                profile_path: catalog.profiles["profile"].user_data_dir.clone(),
                cdp_endpoint: browser.cdp_endpoint,
                target_id: "target".into(),
                shared_with: vec![],
            };
            state.app_browser_attachments.insert(
                browser.id,
                BrowserAppAttachment {
                    request,
                    owner: BrowserHostIdentity::from(&state.host_bindings["task"].request),
                    borrowed_tab_ids: BTreeSet::from(["target".into()]),
                    profile_id: "profile".into(),
                    desktop: state.host_bindings["task"].desktop.clone(),
                    assignment: None,
                },
            );
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &rebind,
                &mut publish,
                &mut changed,
            )
            .unwrap();
            assert_eq!(state.host_bindings["task"].desktop.generation, 2);
            assert_eq!(metadata(&state, "task")["state"], "connected");
            assert_eq!(e.recoveries, 0);
        });
    }
    #[test]
    fn hosted_reconnect_loss_report_only_then_explicit_blank_reopen_same_identity() {
        fixture(|mut state, catalog, browser_id| {
            let mut e = effects(false);
            let mut publish =
                |_: &BrowserSessionState, _: &BrowserSessionState, _: &mut Effects| Ok(());
            let original = state.sessions.clone();
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &command("reconnect", 0),
                &mut publish,
                &mut observe,
            )
            .unwrap();
            assert_eq!(metadata(&state, "task")["state"], "browser_lost");
            assert!(execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &command("reopen", 1),
                &mut publish,
                &mut observe
            )
            .unwrap_err()
            .contains("report_only"));
            assert_eq!(e.recoveries, 0);
            let mut configure = command("configure", 1);
            configure["lossPolicy"] = json!("reopen");
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &configure,
                &mut publish,
                &mut observe,
            )
            .unwrap();
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &command("reopen", 2),
                &mut publish,
                &mut observe,
            )
            .unwrap();
            assert_eq!(e.recoveries, 1);
            assert_eq!(state.sessions, original);
            assert_eq!(state.browsers[browser_id].pid, 4242);
            assert_eq!(state.tabs["tab"].target_id, "new-target");
        });
    }
    #[test]
    fn hosted_reconnect_uncertain_reopen_persists_and_never_replays() {
        fixture(|mut state, catalog, _| {
            state
                .hosted_continuity
                .entry("task".into())
                .or_default()
                .loss_policy = BrowserHostedLossPolicy::Reopen;
            let mut e = effects(false);
            e.fail_recovery = true;
            let mut durable = state.clone();
            let mut publish =
                |expected: &BrowserSessionState, next: &BrowserSessionState, _: &mut Effects| {
                    assert_eq!(expected, &durable);
                    durable = next.clone();
                    Ok(())
                };
            assert!(execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &command("reopen", 0),
                &mut publish,
                &mut observe
            )
            .is_err());
            assert_eq!(metadata(&durable, "task")["pending"], true);
            let mut publish =
                |_: &BrowserSessionState, _: &BrowserSessionState, _: &mut Effects| Ok(());
            assert!(execute_with_observer(
                &mut durable,
                &catalog,
                &mut e,
                &command("reopen", 0),
                &mut publish,
                &mut observe
            )
            .unwrap_err()
            .contains("no_replay"));
            assert_eq!(e.recoveries, 1);
        });
    }
    #[test]
    fn hosted_reconnect_pending_peer_blocks_shared_automation_but_ended_history_does_not() {
        fixture(|mut state, _, _| {
            let mut peer = state.sessions["task"].clone();
            peer.id = "peer".into();
            peer.name = "peer".into();
            state.sessions.insert("peer".into(), peer);
            let mut pending = BrowserHostedContinuity::default();
            pending.record("request", "fingerprint", "reopen", true);
            state.hosted_continuity.insert("peer".into(), pending);
            assert!(check_state(&state, "fixture", &json!({"action":"click"})).is_err());
            state.sessions.remove("peer");
            assert!(check_state(&state, "fixture", &json!({"action":"click"})).is_ok());
        });
    }
    #[test]
    fn hosted_reconnect_borrowed_reopen_preserves_logical_identity_and_human_control() {
        fixture(|mut state, catalog, browser_id| {
            let request = BrowserAttachmentRequest {
                version: 1,
                process: RecordedProcessIdentity {
                    pid: u32::MAX,
                    start_token: "old".into(),
                    executable_path: Some("/synthetic/chrome".into()),
                    browser_family: None,
                },
                profile_path: catalog.profiles["profile"].user_data_dir.clone(),
                cdp_endpoint: state.browsers[browser_id].cdp_endpoint.clone(),
                target_id: "target".into(),
                shared_with: vec![],
            };
            state.app_browser_attachments.insert(
                browser_id.into(),
                BrowserAppAttachment {
                    request: request.clone(),
                    owner: BrowserHostIdentity::from(&state.host_bindings["task"].request),
                    borrowed_tab_ids: BTreeSet::from(["target".into()]),
                    profile_id: "profile".into(),
                    desktop: state.host_bindings["task"].desktop.clone(),
                    assignment: None,
                },
            );
            state
                .hosted_continuity
                .entry("task".into())
                .or_default()
                .loss_policy = BrowserHostedLossPolicy::Reopen;
            browser_host_control::change(
                browser_id,
                "takeover",
                0,
                "fixture human",
                "reattach while held",
                true,
            )
            .unwrap();
            let mut fresh = request;
            fresh.process.pid = 4242;
            fresh.process.start_token = "fresh".into();
            fresh.target_id = "fresh-target".into();
            fresh.cdp_endpoint = "ws://127.0.0.1:9222/devtools/browser/new".into();
            let mut command = command("reopen", 0);
            command["attachment"] = json!(fresh);
            command["desktop"] = json!({"desktopId":"desktop","generation":1});
            let mut e = effects(false);
            let mut publish =
                |_: &BrowserSessionState, _: &BrowserSessionState, _: &mut Effects| Ok(());
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &command,
                &mut publish,
                &mut observe,
            )
            .unwrap();
            assert_eq!(state.sessions["task"].browser_id, browser_id);
            assert_eq!(
                state.sessions["task"].current_tab_id.as_deref(),
                Some("tab")
            );
            assert_eq!(state.tabs["tab"].target_id, "fresh-target");
            assert_eq!(state.browsers[browser_id].pid, 4242);
            assert_eq!(e.recoveries, 0);
            assert_eq!(e.attachments, 1);
            assert_eq!(
                browser_host_control::metadata(browser_id).unwrap()["owner"],
                "human"
            );
        });
    }
    #[test]
    fn hosted_reconnect_pending_website_effect_blocks_reopen_without_any_lifecycle_effect() {
        fixture(|mut state, catalog, browser_id| {
            state
                .hosted_continuity
                .entry("task".into())
                .or_default()
                .loss_policy = BrowserHostedLossPolicy::Reopen;
            browser_host_control::admit(browser_id, "click", "uncertain-click")
                .unwrap()
                .unwrap()
                .finish(false)
                .unwrap();
            let before = state.clone();
            let mut e = effects(false);
            let mut publish =
                |_: &BrowserSessionState, _: &BrowserSessionState, _: &mut Effects| Ok(());
            assert!(execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &command("reopen", 0),
                &mut publish,
                &mut observe
            )
            .unwrap_err()
            .contains("pending_effect"));
            assert_eq!(state, before);
            assert_eq!(e.recoveries, 0);
        });
    }
    #[test]
    fn hosted_reconnect_disconnect_hold_never_erases_existing_leave_open() {
        fixture(|mut state, catalog, _| {
            state.leave_open_session_ids.insert("task".into());
            let mut e = effects(true);
            let mut publish =
                |_: &BrowserSessionState, _: &BrowserSessionState, _: &mut Effects| Ok(());
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &command("disconnect", 0),
                &mut publish,
                &mut observe,
            )
            .unwrap();
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &command("reconnect", 1),
                &mut publish,
                &mut observe,
            )
            .unwrap();
            assert!(state.leave_open_session_ids.contains("task"));
        });
    }
    #[test]
    fn hosted_reconnect_absent_managed_browser_records_future_target_then_reopens_exact_generation()
    {
        fixture(|mut state, catalog, browser_id| {
            let original = state.browsers[browser_id].clone();
            let original_task = state.sessions["task"].clone();
            let mut e = effects(false);
            let mut publish =
                |_: &BrowserSessionState, _: &BrowserSessionState, _: &mut Effects| Ok(());
            let mut rebind = command("rebind", 0);
            rebind["desktop"] = json!({"desktopId":"desktop","generation":2});
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &rebind,
                &mut publish,
                &mut observe,
            )
            .unwrap();
            assert_eq!(metadata(&state, "task")["state"], "browser_lost");
            assert_eq!(metadata(&state, "task")["recoveryDesktop"]["generation"], 2);
            assert_eq!(state.browsers[browser_id], original);
            assert_eq!(state.host_bindings["task"].desktop.generation, 1);
            assert_eq!(e.recoveries, 0);
            let restored: BrowserSessionState =
                serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
            assert_eq!(restored, state);
            let mut configure = command("configure", 1);
            configure["lossPolicy"] = json!("reopen");
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &configure,
                &mut publish,
                &mut observe,
            )
            .unwrap();
            execute_with_observer(
                &mut state,
                &catalog,
                &mut e,
                &command("reopen", 2),
                &mut publish,
                &mut observe,
            )
            .unwrap();
            assert_eq!(
                state.browsers[browser_id]
                    .desktop
                    .as_ref()
                    .unwrap()
                    .generation,
                2
            );
            assert_eq!(state.host_bindings["task"].desktop.generation, 2);
            assert_eq!(state.sessions["task"], original_task);
            assert_eq!(metadata(&state, "task")["state"], "connected");
            assert!(state.hosted_continuity["task"].recovery_desktop.is_none());
            assert_eq!(e.recoveries, 1);
        });
    }
}
