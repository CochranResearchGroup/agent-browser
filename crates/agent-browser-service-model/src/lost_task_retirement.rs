//! Exact, signal-free retirement after trusted owning-service cross-boot proof.
//! Kernel/profile observations are supplied only by the owner adapter, never by
//! public request JSON. This transition grants no provider or credential authority.
use crate::{
    BrowserSessionState, BrowserTabEndReason, BrowserTaskOperation, BrowserTaskOperationState,
    RecordedProcessIdentity, SessionEndReason, TerminalBrowserSession, TerminalBrowserTab,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LostTaskRetirementRequest {
    pub task_id: String,
    pub operation_id: String,
    pub expected_binding_ref: String,
}

/// Not deserializable: an HTTP/MCP caller cannot supply owning-service proof.
#[derive(Clone, Debug)]
pub struct LostTaskRetirementObservation {
    pub original_process: RecordedProcessIdentity,
    pub current_boot_id: String,
    pub original_pid_absent: bool,
    pub profile_unoccupied: bool,
    pub control_epoch: u64,
}

fn held() -> String {
    "browser_task_retirement_unverified".into()
}
fn digest(value: &impl Serialize) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(|_| held())?)
    ))
}
fn valid_boot_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn boot(token: &str) -> Result<&str, String> {
    let parts: Vec<_> = token.split(':').collect();
    if parts.len() != 3
        || parts[0] != "linux"
        || !valid_boot_id(parts[1])
        || parts[2].is_empty()
        || !parts[2].bytes().all(|b| b.is_ascii_digit())
    {
        return Err(held());
    }
    Ok(parts[1])
}

/// Freeze only owner-retained identity, excluding transient activity timestamps.
pub fn freeze_task_retirement(
    state: &BrowserSessionState,
    task_id: &str,
    original: &RecordedProcessIdentity,
    control_epoch: u64,
) -> Result<String, String> {
    let task = state
        .sessions
        .get(task_id)
        .filter(|t| t.id == task_id)
        .ok_or_else(held)?;
    let browser = state
        .browsers
        .get(&task.browser_id)
        .filter(|b| {
            b.id == task.browser_id && b.profile_id == task.profile_id && b.pid == original.pid
        })
        .ok_or_else(held)?;
    let tab = task
        .current_tab_id
        .as_ref()
        .and_then(|id| state.tabs.get(id))
        .filter(|t| t.session_id == task_id && t.browser_id == browser.id)
        .ok_or_else(held)?;
    boot(&original.start_token)?;
    digest(&(
        task.id.as_str(),
        task.created_at_ms,
        browser,
        tab.id.as_str(),
        tab.target_id.as_str(),
        tab.created_at_ms,
        original,
        control_epoch,
    ))
}

/// Caller holds the owner-store write transaction and qualifies its observations
/// against that transaction's original launch custody. No external effects occur.
pub fn retire_lost_task(
    state: &mut BrowserSessionState,
    request: &LostTaskRetirementRequest,
    observed: &LostTaskRetirementObservation,
    now_ms: u64,
) -> Result<BrowserTaskOperation, String> {
    if request.operation_id.is_empty()
        || request.operation_id.len() > 128
        || !request
            .operation_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
        || request.expected_binding_ref.len() != 64
        || !request
            .expected_binding_ref
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
    {
        return Err(held());
    }
    let fingerprint = digest(request)?;
    if let Some(previous) = state
        .task_operations
        .get(&request.task_id)
        .and_then(|rows| rows.get(&request.operation_id))
    {
        if previous.action == "lost_task_retirement"
            && previous.fingerprint == fingerprint
            && previous.state == BrowserTaskOperationState::OwnerAcknowledged
            && previous
                .evidence_references
                .get("retirementResult")
                .map(String::as_str)
                == Some("retired_lost")
        {
            return Ok(previous.clone());
        }
        return Err("browser_task_retirement_operation_conflict".into());
    }
    if !valid_boot_id(&observed.current_boot_id)
        || observed
            .current_boot_id
            .eq_ignore_ascii_case(boot(&observed.original_process.start_token)?)
        || !observed.original_pid_absent
        || !observed.profile_unoccupied
    {
        return Err(held());
    }
    if freeze_task_retirement(
        state,
        &request.task_id,
        &observed.original_process,
        observed.control_epoch,
    )? != request.expected_binding_ref
    {
        return Err("browser_task_retirement_binding_changed".into());
    }
    let task = state
        .sessions
        .get(&request.task_id)
        .ok_or_else(held)?
        .clone();
    let browser = state
        .browsers
        .get(&task.browser_id)
        .ok_or_else(held)?
        .clone();
    let tab = task
        .current_tab_id
        .as_ref()
        .and_then(|id| state.tabs.get(id))
        .ok_or_else(held)?
        .clone();
    if browser.active_session_ids != [request.task_id.clone()]
        || state
            .sessions
            .values()
            .any(|t| t.profile_id == task.profile_id && t.id != task.id)
        || state
            .browsers
            .values()
            .any(|b| b.profile_id == task.profile_id && b.id != browser.id)
        || state
            .tabs
            .values()
            .any(|t| t.browser_id == browser.id && t.session_id != task.id)
        || state.shared_browser_ids.contains(&browser.id)
        || state.app_browser_attachments.contains_key(&browser.id)
        || state.host_bindings.contains_key(&task.id)
        || state.hosted_continuity.contains_key(&task.id)
        || state.profile_operation_ids.contains_key(&task.profile_id)
        || state.task_operations.get(&task.id).is_some_and(|rows| {
            rows.values()
                .any(|op| op.state != BrowserTaskOperationState::OwnerAcknowledged)
        })
        || state
            .tabs
            .values()
            .any(|t| t.session_id == task.id && state.pending_human_tabs.contains_key(&t.id))
    {
        return Err("browser_task_retirement_resource_held".into());
    }
    let receipt = BrowserTaskOperation {
        request_id: request.operation_id.clone(),
        task_id: task.id.clone(),
        profile_id: task.profile_id.clone(),
        browser_id: browser.id.clone(),
        tab_id: tab.id.clone(),
        target_id: tab.target_id.clone(),
        process_pid: browser.pid,
        process_identity: Some(observed.original_process.clone()),
        control_epoch: observed.control_epoch,
        action: "lost_task_retirement".into(),
        fingerprint,
        state: BrowserTaskOperationState::OwnerAcknowledged,
        created_at_ms: now_ms,
        completed_at_ms: Some(now_ms),
        evidence_references: BTreeMap::from([
            ("retirementResult".into(), "retired_lost".into()),
            (
                "ownerBindingRef".into(),
                request.expected_binding_ref.clone(),
            ),
            ("providerAuthentication".into(), "unverified".into()),
            ("providerCleanup".into(), "not_asserted".into()),
        ]),
    };
    let tabs: Vec<_> = state
        .tabs
        .values()
        .filter(|t| t.session_id == task.id)
        .cloned()
        .collect();
    for old in tabs {
        state.tab_lifetimes.remove(&old.id);
        state.tab_metadata.remove(&old.id);
        state.tabs.remove(&old.id);
        state.tab_history.push(TerminalBrowserTab {
            id: old.id,
            target_id: old.target_id,
            browser_id: old.browser_id,
            session_id: old.session_id,
            profile_id: task.profile_id.clone(),
            created_at_ms: old.created_at_ms,
            last_activity_at_ms: old.last_activity_at_ms,
            closed_at_ms: now_ms,
            reason: BrowserTabEndReason::SessionEnded,
            session_end_reason: Some(SessionEndReason::BrowserTerminated),
        });
    }
    for link in state
        .remote_view_tab_handoffs
        .values_mut()
        .filter(|l| l.session_id == task.id)
    {
        link.target_expired = true;
    }
    state.sessions.remove(&task.id);
    state.browsers.remove(&browser.id);
    state.idle_closed_browsers.remove(&browser.id);
    state.leave_open_session_ids.remove(&task.id);
    state
        .retention_claims
        .retain(|_, claim| claim.session_id != task.id);
    state.session_history.push(TerminalBrowserSession {
        id: task.id.clone(),
        name: task.name,
        profile_id: task.profile_id,
        browser_id: task.browser_id,
        created_at_ms: task.created_at_ms,
        last_activity_at_ms: task.last_activity_at_ms,
        ended_at_ms: now_ms,
        reason: SessionEndReason::BrowserTerminated,
    });
    state
        .task_operations
        .entry(task.id)
        .or_default()
        .insert(request.operation_id.clone(), receipt.clone());
    Ok(receipt)
}
