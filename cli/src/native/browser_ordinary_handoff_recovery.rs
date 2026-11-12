//! Ordinary process exit has launch custody, but no fictional idle retirement.
//! Freeze its exact old scope only in the replacement publication transaction.
use super::*;
use agent_browser_service_model::{
    remote_view_native_target_digest, resolve_remote_view_tab_handoff, ManagedBrowserInstance,
    ManagedBrowserTab, RemoteViewTabHandoffTarget,
};

const SCHEMA: &str = "agent-browser.handoff-ordinary-recovery-binding.v1";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Binding {
    handoff_id: String,
    previous_target_digest: String,
    target_digest: String,
    browser: ManagedBrowserInstance,
    tabs: Vec<ManagedBrowserTab>,
    recovery: serde_json::Value,
}

fn kind(id: &str, digest: &str) -> String {
    format!("ordinary_{}", handoff_recovery_kind(id, digest))
}

fn qualify(
    connection: &Connection,
    state: &BrowserSessionState,
    binding: &Binding,
) -> Result<serde_json::Value, String> {
    let refused = || "browser_handoff_recovery_chain_unconfirmed".to_string();
    let target = resolve_remote_view_tab_handoff(state, &binding.handoff_id)?;
    let browser = &target.browser;
    if state.idle_closed_browsers.contains_key(&browser.id)
        || state.shared_browser_ids.contains(&browser.id)
        || state.app_browser_attachments.contains_key(&browser.id)
        || state
            .profile_operation_ids
            .contains_key(&browser.profile_id)
        || state
            .browsers
            .values()
            .any(|other| other.profile_id == browser.profile_id && other.id != browser.id)
    {
        return Err(refused());
    }
    // A pending retirement, or a physical retirement of this exact original,
    // must use its own qualified owner chain. Earlier terminal histories stay
    // intact and do not masquerade as this ordinary process exit.
    let retirements = super::super::browser_idle_retirement_store::retirement_records(connection)
        .map_err(|_| refused())?;
    if retirements.iter().any(|record| {
        record.intent.scopes.iter().any(|scope| {
            scope.browser.profile_id == browser.profile_id
                && (record.completed.is_none()
                    || (scope.browser.pid == binding.browser.pid
                        && !(record.retained_live && record.has_owner_resolution())))
        })
    }) {
        return Err(refused());
    }
    let mut expected = binding.browser.clone();
    expected.pid = browser.pid;
    expected.cdp_endpoint = browser.cdp_endpoint.clone();
    if expected != *browser
        || binding.browser.pid == browser.pid
        || binding.tabs.len()
            != state
                .tabs
                .values()
                .filter(|tab| tab.browser_id == browser.id)
                .count()
    {
        return Err(refused());
    }
    for original in &binding.tabs {
        let current = state.tabs.get(&original.id).ok_or_else(refused)?;
        let mut expected = original.clone();
        expected.target_id = current.target_id.clone();
        expected.last_activity_at_ms = current.last_activity_at_ms;
        if expected != *current {
            return Err(refused());
        }
    }
    let original_tab = binding
        .tabs
        .iter()
        .find(|tab| tab.id == target.tab.id)
        .ok_or_else(refused)?;
    let original_target = RemoteViewTabHandoffTarget {
        session: target.session.clone(),
        tab: original_tab.clone(),
        browser: binding.browser.clone(),
    };
    if remote_view_native_target_digest(&binding.handoff_id, &original_target)?
        != binding.previous_target_digest
        || remote_view_native_target_digest(&binding.handoff_id, &target)? != binding.target_digest
        || !binding
            .browser
            .active_session_ids
            .contains(&target.session.id)
        || original_tab.session_id != target.session.id
    {
        return Err(refused());
    }
    let records = ledger(connection).map_err(|_| refused())?;
    let matching = records
        .records
        .values()
        .filter(|record| record.intent.profile_id == browser.profile_id)
        .collect::<Vec<_>>();
    if matching.iter().any(|record| {
        !record.published
            || record.observed_browser_id.as_deref() != Some(browser.id.as_str())
            || record.observed_process_identity.is_none()
    }) {
        return Err(refused());
    }
    let prior = matching
        .iter()
        .copied()
        .filter(|record| record.observed_pid == Some(binding.browser.pid))
        .collect::<Vec<_>>();
    let current = matching
        .iter()
        .copied()
        .filter(|record| record.observed_pid == Some(browser.pid))
        .collect::<Vec<_>>();
    let ([prior], [current]) = (prior.as_slice(), current.as_slice()) else {
        return Err(refused());
    };
    if assignment_fenced(&records, &current.intent.assignment) {
        return Err(refused());
    }
    let mut original_state = state.clone();
    original_state
        .browsers
        .insert(browser.id.clone(), binding.browser.clone());
    for tab in &binding.tabs {
        original_state.tabs.insert(tab.id.clone(), tab.clone());
    }
    (**prior)
        .clone()
        .confirm_publication(&original_state)
        .map_err(|_| refused())?;
    (**current)
        .clone()
        .confirm_publication(state)
        .map_err(|_| refused())?;
    if !matches!(
        crate::process_identity::observe_process(binding.browser.pid),
        crate::process_identity::ProcessObservation::Missing
    ) {
        return Err(refused());
    }
    Ok(
        serde_json::json!({"kind":"ordinaryProcessExit", "priorLaunchIntentId":prior.intent.intent_id,"launchIntentId":current.intent.intent_id}),
    )
}

pub(super) fn record(
    connection: &Connection,
    intent: &BrowserLaunchIntent,
    before: &BrowserSessionState,
    after: &BrowserSessionState,
    id: &str,
) -> Result<(), String> {
    let previous = resolve_remote_view_tab_handoff(before, id)?;
    let current = resolve_remote_view_tab_handoff(after, id)?;
    let mut binding = Binding {
        handoff_id: id.to_owned(),
        previous_target_digest: remote_view_native_target_digest(id, &previous)?,
        target_digest: remote_view_native_target_digest(id, &current)?,
        tabs: before
            .tabs
            .values()
            .filter(|tab| tab.browser_id == previous.browser.id)
            .cloned()
            .collect(),
        browser: previous.browser,
        recovery: serde_json::Value::Null,
    };
    let Ok(recovery) = qualify(connection, after, &binding) else {
        return Ok(());
    };
    if recovery["launchIntentId"] != intent.intent_id {
        return Ok(());
    }
    binding.recovery = recovery;
    let key = kind(id, &binding.target_digest);
    let existing: Option<Binding> = load_optional_document(connection, &key, SCHEMA)?;
    if existing.as_ref().is_some_and(|saved| saved != &binding) {
        return Err("browser_handoff_recovery_binding_conflict".into());
    }
    save_document(connection, &key, SCHEMA, &binding)
}

pub(super) fn proof(
    connection: &Connection,
    state: &BrowserSessionState,
    id: &str,
    previous_digest: &str,
) -> Result<serde_json::Value, String> {
    let target = resolve_remote_view_tab_handoff(state, id)?;
    let digest = remote_view_native_target_digest(id, &target)?;
    let binding: Option<Binding> = load_optional_document(connection, &kind(id, &digest), SCHEMA)?;
    let binding = binding.ok_or("browser_handoff_recovery_mapping_unrecorded")?;
    if binding.handoff_id != id
        || binding.previous_target_digest != previous_digest
        || binding.target_digest != digest
    {
        return Err("browser_handoff_recovery_mapping_changed".into());
    }
    let recovery = qualify(connection, state, &binding)?;
    if recovery != binding.recovery {
        return Err("browser_handoff_recovery_chain_changed".into());
    }
    Ok(recovery)
}

/// Qualify current owner custody for a fresh target. Do not infer or persist
/// any old target mapping from these browser-level observations.
pub(super) fn qualify_reconstruction(
    connection: &Connection,
    state: &BrowserSessionState,
    id: &str,
) -> Result<(), String> {
    let refused = || "browser_handoff_reconstruction_custody_unconfirmed".to_string();
    let target = resolve_remote_view_tab_handoff(state, id)?;
    let browser = &target.browser;
    if browser.cdp_endpoint.is_empty()
        || state.idle_closed_browsers.contains_key(&browser.id)
        || state.shared_browser_ids.contains(&browser.id)
        || state.app_browser_attachments.contains_key(&browser.id)
        || state
            .profile_operation_ids
            .contains_key(&browser.profile_id)
        || state
            .browsers
            .values()
            .any(|other| other.profile_id == browser.profile_id && other.id != browser.id)
    {
        return Err(refused());
    }
    let records = ledger(connection).map_err(|_| refused())?;
    let matching = records
        .records
        .values()
        .filter(|record| record.intent.profile_id == browser.profile_id)
        .collect::<Vec<_>>();
    if matching.len() != 2
        || matching.iter().any(|record| {
            !record.published
                || record.observed_browser_id.as_deref() != Some(browser.id.as_str())
                || record.observed_process_identity.is_none()
        })
    {
        return Err(refused());
    }
    let current = matching
        .iter()
        .copied()
        .filter(|record| record.observed_pid == Some(browser.pid))
        .collect::<Vec<_>>();
    let prior = matching
        .iter()
        .copied()
        .filter(|record| record.observed_pid != Some(browser.pid))
        .collect::<Vec<_>>();
    let ([current], [prior]) = (current.as_slice(), prior.as_slice()) else {
        return Err(refused());
    };
    if assignment_fenced(&records, &current.intent.assignment)
        || current.intent.assignment.desktop_id != prior.intent.assignment.desktop_id
        || current.intent.assignment.generation != prior.intent.assignment.generation
    {
        return Err(refused());
    }
    (**current)
        .clone()
        .confirm_publication(state)
        .map_err(|_| refused())?;
    let identity = current
        .observed_process_identity
        .as_ref()
        .ok_or_else(refused)?;
    let crate::process_identity::ProcessObservation::Observed(observed) =
        crate::process_identity::observe_process(browser.pid)
    else {
        return Err(refused());
    };
    if observed.start_token.as_deref() != Some(identity.start_token.as_str())
        || observed.executable_path != identity.executable_path
        || !matches!(
            crate::process_identity::observe_process(prior.observed_pid.ok_or_else(refused)?),
            crate::process_identity::ProcessObservation::Missing
        )
    {
        return Err(refused());
    }
    let retirements = super::super::browser_idle_retirement_store::retirement_records(connection)
        .map_err(|_| refused())?;
    if retirements.iter().any(|record| {
        record.intent.scopes.iter().any(|scope| {
            scope.browser.profile_id == browser.profile_id
                && (record.completed.is_none()
                    || (Some(scope.browser.pid) == prior.observed_pid
                        && !(record.retained_live && record.has_owner_resolution())))
        })
    }) {
        return Err(refused());
    }
    Ok(())
}
