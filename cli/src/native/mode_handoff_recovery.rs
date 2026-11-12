//! Rejoin only a frozen handoff through a completed owner mode replacement.
//! This reads existing journals; it never launches, releases input or backfills
//! retirement mappings. Legacy mode journals qualify only their frozen primary
//! task. New journals freeze every preexisting handoff before any effect.
use super::*;
use agent_browser_service_model::{
    remote_view_native_target_digest, resolve_remote_view_tab_handoff,
};

/// Admit only read-only controls for an exact frozen conversion whose original
/// is positively absent. This does not qualify a desktop, input or native use.
pub(in crate::native) fn pending_mode_handoff_proof(
    connection: &rusqlite::Connection,
    state: &BrowserSessionState,
    handoff: &str,
) -> Result<Option<String>, String> {
    let target = resolve_remote_view_tab_handoff(state, handoff)?;
    if !state.profile_operation_ids.contains_key(&target.browser.profile_id) {
        return Ok(None);
    }
    if state.shared_browser_ids.contains(&target.browser.id)
        || state
            .app_browser_attachments
            .contains_key(&target.browser.id)
        || state.idle_closed_browsers.contains_key(&target.browser.id)
        || state.browsers.values().any(|other| {
            other.id != target.browser.id && other.profile_id == target.browser.profile_id
        })
    {
        return Ok(None);
    }
    let Some(database) = connection.path() else {
        return Ok(None);
    };
    let path = Path::new(database).with_extension("mode.sqlite3");
    if !path.is_file() {
        return Ok(None);
    }
    let refused = || "browser_handoff_mode_recovery_unconfirmed".to_string();
    let modes =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| refused())?;
    let exists: bool = modes.query_row("SELECT count(*)=2 FROM sqlite_master WHERE type='table' AND name IN ('browser_mode_owner_bindings','browser_mode_operations')", [], |r| r.get(0)).map_err(|_| refused())?;
    if !exists {
        return Ok(None);
    }
    let mut query = modes.prepare("SELECT o.id,b.binding,o.state FROM browser_mode_operations o LEFT JOIN browser_mode_owner_bindings b ON b.id=o.id WHERE json_extract(o.state,'$.request.browser_id')=?1").map_err(|_| refused())?;
    let rows = query
        .query_map([&target.browser.id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|_| refused())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| refused())?;
    let mut active = Vec::new();
    for (id, raw_binding, raw_operation) in rows {
        let operation: Operation = serde_json::from_str(&raw_operation).map_err(|_| refused())?;
        if !matches!(operation.phase, Phase::Completed | Phase::Cancelled) {
            active.push((id, raw_binding, operation));
        }
    }
    let [(id, Some(raw_binding), operation)] = active.as_slice() else {
        return Ok(None);
    };
    if state.profile_operation_ids.get(&target.browser.profile_id) != Some(id) {
        return Ok(None);
    }
    let binding: Binding = serde_json::from_str(raw_binding).map_err(|_| refused())?;
    if operation.phase != Phase::ClosePrompt
        || operation.launch_attempts != 0
        || operation.replacement.is_some()
        || operation.restoring_previous
        || operation.close_method
            != Some(crate::native::browser_mode_operation::CloseMethod::Graceful)
        || operation.request.operation_id != *id
        || operation.request.browser_id != binding.browser.id
        || operation.request.profile_id != binding.profile.id
        || operation.request.profile_id != target.browser.profile_id
    {
        return Ok(None);
    }
    let Some(frozen) = binding.handoff_targets.get(handoff) else {
        return Ok(None);
    };
    let prior = RemoteViewTabHandoffTarget {
        browser: frozen.browser.clone(),
        session: frozen.session.clone(),
        tab: frozen.tab.clone(),
    };
    if retained_mode_native_target(&prior, state).is_err() || binding.browser != frozen.browser {
        return Ok(None);
    }
    let Ok(original) =
        serde_json::from_str::<RecordedProcessIdentity>(&operation.request.original_process)
    else {
        return Ok(None);
    };
    let tasks = state
        .sessions
        .values()
        .filter(|t| t.browser_id == target.browser.id)
        .map(|t| t.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let requested = operation
        .request
        .task_ids
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if original != binding.original
        || original.pid != frozen.browser.pid
        || !tasks.contains(&target.session.id)
        || tasks != requested
        || !matches!(
            crate::process_identity::observe_process(original.pid),
            crate::process_identity::ProcessObservation::Missing
        )
    {
        return Ok(None);
    }
    Ok(Some(id.clone()))
}

pub(in crate::native) fn completed_mode_handoff_proof(
    connection: &rusqlite::Connection,
    state: &BrowserSessionState,
    handoff: &str,
    previous_digest: &str,
) -> Result<Option<Value>, String> {
    let refused = || "browser_handoff_mode_lineage_unconfirmed".to_string();
    let target = resolve_remote_view_tab_handoff(state, handoff)?;
    if state.shared_browser_ids.contains(&target.browser.id)
        || state
            .app_browser_attachments
            .contains_key(&target.browser.id)
        || state.idle_closed_browsers.contains_key(&target.browser.id)
        || state
            .profile_operation_ids
            .contains_key(&target.browser.profile_id)
        || state.browsers.values().any(|other| {
            other.id != target.browser.id && other.profile_id == target.browser.profile_id
        })
    {
        return Ok(None);
    }
    let Some(database) = connection.path() else {
        return Ok(None);
    };
    let path = Path::new(database).with_extension("mode.sqlite3");
    if !path.is_file() {
        return Ok(None);
    }
    let modes =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| refused())?;
    let exists:bool=modes.query_row("SELECT count(*)=2 FROM sqlite_master WHERE type='table' AND name IN ('browser_mode_owner_bindings','browser_mode_operations')",[],|row|row.get(0)).map_err(|_| refused())?;
    if !exists {
        return Ok(None);
    }
    let mut query=modes.prepare("SELECT o.id,b.binding,o.state FROM browser_mode_operations o JOIN browser_mode_owner_bindings b ON b.id=o.id WHERE json_extract(o.state,'$.request.browser_id')=?1").map_err(|_| refused())?;
    let rows = query
        .query_map([&target.browser.id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|_| refused())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| refused())?;
    let mut proofs = Vec::new();
    for (id, raw_binding, raw_operation) in rows {
        let binding: Binding = serde_json::from_str(&raw_binding).map_err(|_| refused())?;
        let operation: Operation = serde_json::from_str(&raw_operation).map_err(|_| refused())?;
        if operation.phase != Phase::Completed
            || operation.request.operation_id != id
            || operation.request.browser_id != binding.browser.id
            || operation.request.profile_id != binding.profile.id
            || operation.request.profile_id != target.browser.profile_id
        {
            continue;
        }
        let frozen = if let Some(frozen) = binding.handoff_targets.get(handoff) {
            frozen.clone()
        } else if binding.handoff_targets.is_empty()
            && binding.session.id == target.session.id
            && binding.tab.id == target.tab.id
        {
            FrozenHandoffTarget {
                browser: binding.browser.clone(),
                session: binding.session.clone(),
                tab: binding.tab.clone(),
            }
        } else {
            continue;
        };
        let prior = RemoteViewTabHandoffTarget {
            browser: frozen.browser.clone(),
            session: frozen.session.clone(),
            tab: frozen.tab.clone(),
        };
        if remote_view_native_target_digest(handoff, &prior)? != previous_digest {
            continue;
        }
        let Ok(original) =
            serde_json::from_str::<RecordedProcessIdentity>(&operation.request.original_process)
        else {
            continue;
        };
        let Some(replacement) = operation.replacement.as_ref() else {
            continue;
        };
        let Ok(process) =
            serde_json::from_str::<RecordedProcessIdentity>(&replacement.process_identity)
        else {
            continue;
        };
        let effective = operation
            .effective_mode
            .as_ref()
            .unwrap_or(&operation.request.requested_mode);
        let tasks = state
            .sessions
            .values()
            .filter(|task| task.browser_id == target.browser.id)
            .map(|task| task.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let requested = operation
            .request
            .task_ids
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        if original != binding.original
            || original.pid != frozen.browser.pid
            || original.pid == process.pid
            || process.pid != target.browser.pid
            || !tasks.contains(&target.session.id)
            || tasks != requested
            || replacement.mode != *effective
            || replacement.debugging != (*effective == Mode::Cdp)
            || !replacement.native_observation
            || operation.launch_attempts == 0
            || target.browser.cdp_endpoint.is_empty() != (*effective == Mode::CdpFree)
        {
            continue;
        }
        let mut expected_browser = frozen.browser.clone();
        if expected_browser
            .desktop
            .as_ref()
            .zip(target.browser.desktop.as_ref())
            .is_none_or(|(old, new)| {
                old.desktop_id != new.desktop_id || old.generation != new.generation
            })
        {
            continue;
        }
        expected_browser.pid = target.browser.pid;
        expected_browser.cdp_endpoint = target.browser.cdp_endpoint.clone();
        expected_browser.desktop = target.browser.desktop.clone();
        let mut expected_session = target.session.clone();
        expected_session.last_activity_at_ms = frozen.session.last_activity_at_ms;
        expected_session.expires_at_ms = frozen.session.expires_at_ms;
        let mut expected_tab = target.tab.clone();
        expected_tab.target_id = frozen.tab.target_id.clone();
        expected_tab.last_activity_at_ms = frozen.tab.last_activity_at_ms;
        if expected_browser != target.browser
            || expected_session != frozen.session
            || expected_tab != frozen.tab
        {
            continue;
        }
        let published = crate::native::browser_launch_custody_store::published_process_identity(
            connection,
            &target.browser,
        )
        .map_err(|_| refused())?;
        if published.as_ref() != Some(&process)
            || !matches!(
                crate::process_identity::observe_process(original.pid),
                crate::process_identity::ProcessObservation::Missing
            )
            || !original_matches(&process)?
        {
            continue;
        }
        proofs.push(json!({"modeOperationId":id,"previousTargetDigest":previous_digest,"targetDigest":remote_view_native_target_digest(handoff,&target)?}));
    }
    match proofs.as_slice() {
        [] => Ok(None),
        [proof] => Ok(Some(proof.clone())),
        _ => Err("browser_handoff_mode_lineage_ambiguous".into()),
    }
}
