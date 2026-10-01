//! Logical operator handoffs independent of provider routes and browser placement.

use serde::{Deserialize, Serialize};

use crate::{
    BrowserSessionState, ManagedBrowserInstance, ManagedBrowserSession, ManagedBrowserTab,
};

/// A durable handoff addresses one logical session and tab, never a desktop URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteViewTabHandoff {
    pub id: String,
    pub session_id: String,
    pub tab_id: String,
}

/// Current joined browser identity; presentation issuance belongs to the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteViewTabHandoffTarget {
    pub session: ManagedBrowserSession,
    pub tab: ManagedBrowserTab,
    pub browser: ManagedBrowserInstance,
}

/// Retain exactly one stable binding for a live tab. Conflicting IDs are rejected
/// before changing the aggregate. The host must publish this aggregate atomically.
pub fn retain_remote_view_tab_handoff(
    state: &mut BrowserSessionState,
    handoff_id: &str,
    session_id: &str,
    tab_id: &str,
) -> Result<RemoteViewTabHandoff, String> {
    if handoff_id.is_empty()
        || handoff_id.len() > 128
        || !handoff_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("remote_view_tab_handoff_id_invalid".into());
    }
    let proposed = RemoteViewTabHandoff {
        id: handoff_id.into(),
        session_id: session_id.into(),
        tab_id: tab_id.into(),
    };
    join(state, &proposed)?;
    if let Some(existing) = state.remote_view_tab_handoffs.get(handoff_id) {
        return if existing == &proposed {
            Ok(existing.clone())
        } else {
            Err("remote_view_tab_handoff_id_conflict".into())
        };
    }
    if let Some(existing) = state
        .remote_view_tab_handoffs
        .values()
        .find(|record| record.session_id == session_id && record.tab_id == tab_id)
    {
        return Ok(existing.clone());
    }
    state
        .remote_view_tab_handoffs
        .insert(handoff_id.into(), proposed.clone());
    Ok(proposed)
}

/// Resolve the retained tab, not the session's mutable current tab. Missing or
/// deliberately closed tabs cannot silently become a replacement target.
pub fn resolve_remote_view_tab_handoff(
    state: &BrowserSessionState,
    handoff_id: &str,
) -> Result<RemoteViewTabHandoffTarget, String> {
    let record = state
        .remote_view_tab_handoffs
        .get(handoff_id)
        .ok_or("remote_view_tab_handoff_missing")?;
    if record.id != handoff_id {
        return Err("remote_view_tab_handoff_record_conflict".into());
    }
    join(state, record)
}

fn join(
    state: &BrowserSessionState,
    record: &RemoteViewTabHandoff,
) -> Result<RemoteViewTabHandoffTarget, String> {
    let session = state
        .sessions
        .get(&record.session_id)
        .ok_or("remote_view_tab_handoff_session_unavailable")?;
    let tab = state
        .tabs
        .get(&record.tab_id)
        .ok_or("remote_view_tab_handoff_tab_unavailable")?;
    let browser = state
        .browsers
        .get(&session.browser_id)
        .ok_or("remote_view_tab_handoff_browser_unavailable")?;
    if session.id != record.session_id
        || tab.id != record.tab_id
        || tab.session_id != session.id
        || tab.browser_id != browser.id
        || browser.id != session.browser_id
        || browser.profile_id != session.profile_id
        || !browser.active_session_ids.contains(&session.id)
        || tab.target_id.is_empty()
    {
        return Err("remote_view_tab_handoff_identity_conflict".into());
    }
    Ok(RemoteViewTabHandoffTarget {
        session: session.clone(),
        tab: tab.clone(),
        browser: browser.clone(),
    })
}
