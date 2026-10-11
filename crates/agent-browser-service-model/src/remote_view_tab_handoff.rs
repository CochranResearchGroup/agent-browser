//! Logical operator handoffs independent of provider routes and browser placement.

use serde::{Deserialize, Serialize};

use crate::{
    BrowserSessionState, ManagedBrowserInstance, ManagedBrowserSession, ManagedBrowserTab,
};

/// Persisted profile-aware retention. None means no automatic link expiry.
/// Changes apply to newly issued links, never rewriting an existing expiry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserRetentionPolicy {
    pub durable_handoff_ttl_ms: Option<u64>,
    pub disposable_handoff_ttl_ms: Option<u64>,
    pub disposable_inactivity_ms: u64,
}
impl Default for BrowserRetentionPolicy {
    fn default() -> Self {
        Self {
            durable_handoff_ttl_ms: None,
            disposable_handoff_ttl_ms: None,
            disposable_inactivity_ms: 24 * 60 * 60 * 1_000,
        }
    }
}
impl BrowserRetentionPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.durable_handoff_ttl_ms == Some(0)
            || self.disposable_handoff_ttl_ms == Some(0)
            || self.disposable_inactivity_ms == 0
        {
            return Err("browser_retention_policy_invalid".into());
        }
        Ok(())
    }
}

/// A durable handoff addresses one logical session and tab, never a desktop URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteViewTabHandoff {
    pub id: String,
    pub session_id: String,
    pub tab_id: String,
    /// Link retention is independent of browser idle and native viewer lifetimes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<RemoteViewTabView>,
    /// Task intent survives automatic tab expiry and history compaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<RemoteViewTabHandoffResumeIntent>,
    /// A frozen creation request prevents duplicate tabs after an uncertain effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recreation: Option<RemoteViewTabHandoffRecreation>,
    /// Automatic retirement permits recreation; explicit close remains distinct.
    #[serde(default)]
    pub target_expired: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewTabHandoffResumeIntent {
    pub session_name: String,
    pub profile_id: String,
    pub disposable_policy_id: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewTabHandoffRecreation {
    pub request_id: String,
    pub requested_at_ms: u64,
}

/// A request retained before issuance, with its qualified outcome when known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteViewTabView {
    pub idempotency_key: String,
    pub issuance: Option<crate::RemoteViewApplicationViewIssuance>,
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
    retain_remote_view_tab_handoff_at(state, handoff_id, session_id, tab_id, 0)
}

/// Reuse a live link or retain a new opaque link after expiry, keeping the expired
/// record so its old URL remains rejected. Browser and target identity stay intact.
pub fn retain_remote_view_tab_handoff_at(
    state: &mut BrowserSessionState,
    handoff_id: &str,
    session_id: &str,
    tab_id: &str,
    now_ms: u64,
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
        view: None,
        created_at_ms: None,
        expires_at_ms: None,
        resume: None,
        recreation: None,
        target_expired: false,
    };
    join(state, &proposed)?;
    if let Some(existing) = state.remote_view_tab_handoffs.get(handoff_id) {
        return if existing.id == proposed.id
            && existing.session_id == proposed.session_id
            && existing.tab_id == proposed.tab_id
        {
            Ok(existing.clone())
        } else {
            Err("remote_view_tab_handoff_id_conflict".into())
        };
    }
    if let Some(existing) = state.remote_view_tab_handoffs.values().find(|record| {
        record.session_id == session_id
            && record.tab_id == tab_id
            && record.check_link_at(now_ms).is_ok()
    }) {
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

/// Read task intent before owner recovery without substituting a current tab.
/// Legacy records can derive intent from retained terminal history, but only
/// after positively identifying automatic expiry of their addressed target.
pub fn remote_view_handoff_resume_intent(
    state: &BrowserSessionState,
    handoff_id: &str,
    now_ms: u64,
) -> Result<RemoteViewTabHandoffResumeIntent, String> {
    let link = state
        .remote_view_tab_handoffs
        .get(handoff_id)
        .ok_or("remote_view_tab_handoff_missing")?;
    if link.id != handoff_id {
        return Err("remote_view_tab_handoff_record_conflict".into());
    }
    link.check_link_at(now_ms)?;
    let live = resolve_remote_view_tab_handoff(state, handoff_id);
    if let Ok(target) = live.as_ref() {
        return Ok(RemoteViewTabHandoffResumeIntent {
            session_name: target.session.name.clone(),
            profile_id: target.session.profile_id.clone(),
            disposable_policy_id: state
                .disposable_profiles
                .get(&target.session.profile_id)
                .map(|allocation| allocation.policy_id.clone()),
            url: link
                .resume
                .as_ref()
                .and_then(|intent| intent.url.clone())
                .or_else(|| {
                    state
                        .navigation_history
                        .iter()
                        .filter(|row| row.tab_id == link.tab_id)
                        .max_by_key(|row| row.visited_at_ms)
                        .map(|row| row.url.clone())
                }),
        });
    }
    if !matches!(
        live.as_ref().unwrap_err().as_str(),
        "remote_view_tab_handoff_session_unavailable"
            | "remote_view_tab_handoff_tab_unavailable"
            | "remote_view_tab_handoff_browser_unavailable"
    ) {
        return Err(live.unwrap_err());
    }
    let ended = state
        .session_history
        .iter()
        .rev()
        .find(|session| session.id == link.session_id);
    let automatic =
        ended.is_some_and(|session| session.reason != crate::SessionEndReason::ExplicitClose);
    if !link.target_expired && !automatic {
        return Err(live.unwrap_err());
    }
    if let Some(intent) = &link.resume {
        return Ok(intent.clone());
    }
    let ended = ended.ok_or("remote_view_handoff_resume_intent_missing")?;
    Ok(RemoteViewTabHandoffResumeIntent {
        session_name: ended.name.clone(),
        profile_id: ended.profile_id.clone(),
        disposable_policy_id: state
            .disposable_profiles
            .get(&ended.profile_id)
            .map(|allocation| allocation.policy_id.clone()),
        url: state
            .navigation_history
            .iter()
            .filter(|row| row.tab_id == link.tab_id)
            .max_by_key(|row| row.visited_at_ms)
            .map(|row| row.url.clone()),
    })
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

/// Historical finite TTL retained for explicit compatibility callers.
/// Current new-link defaults are perpetual.
pub const DEFAULT_REMOTE_VIEW_HANDOFF_TTL_MS: u64 = 24 * 60 * 60 * 1_000;

impl RemoteViewTabHandoff {
    /// Initialize once. Reading or reopening a link never renews its lifetime.
    pub fn initialize_link_retention(&mut self, now_ms: u64, ttl_ms: u64) -> Result<(), String> {
        self.initialize_profile_retention(now_ms, Some(ttl_ms))
    }

    /// Initialize a new link once; None retains a link without expiry.
    /// The creation marker distinguishes an initialized unlimited link from a
    /// legacy record awaiting its one-time creation-metadata migration.
    pub fn initialize_profile_retention(
        &mut self,
        now_ms: u64,
        ttl_ms: Option<u64>,
    ) -> Result<(), String> {
        let expiry = ttl_ms.map(|ttl| link_expiry(now_ms, ttl)).transpose()?;
        if self.created_at_ms.is_none() {
            self.created_at_ms = Some(now_ms);
            if self.expires_at_ms.is_none() {
                self.expires_at_ms = expiry;
            }
        }
        Ok(())
    }

    /// Reject an expired link before browser recovery or presentation effects.
    pub fn check_link_at(&self, now_ms: u64) -> Result<(), String> {
        if self.expires_at_ms.is_some_and(|expiry| now_ms >= expiry) {
            return Err("remote_view_handoff_expired".into());
        }
        Ok(())
    }

    /// Ensure at least the requested remaining TTL, preserving a longer expiry.
    /// This changes only the link; it neither touches sessions nor viewer access.
    pub fn extend_link_retention(&mut self, now_ms: u64, ttl_ms: u64) -> Result<(), String> {
        self.check_link_at(now_ms)?;
        let expiry = link_expiry(now_ms, ttl_ms)?;
        if self.created_at_ms.is_some() && self.expires_at_ms.is_none() {
            return Ok(());
        }
        self.initialize_link_retention(now_ms, ttl_ms)?;
        self.expires_at_ms = Some(self.expires_at_ms.unwrap_or(expiry).max(expiry));
        Ok(())
    }
}

fn link_expiry(now_ms: u64, ttl_ms: u64) -> Result<u64, String> {
    if ttl_ms == 0 {
        return Err("remote_view_handoff_ttl_invalid".into());
    }
    now_ms
        .checked_add(ttl_ms)
        .ok_or_else(|| "remote_view_handoff_ttl_invalid".into())
}

#[cfg(test)]
mod retention_tests {
    use super::*;

    #[test]
    fn handoff_link_retention_migrates_once_extends_and_expires_without_changing_target() {
        let mut link: RemoteViewTabHandoff =
            serde_json::from_str(r#"{"id":"link","sessionId":"session","tabId":"tab"}"#).unwrap();
        link.initialize_link_retention(100, 10).unwrap();
        link.initialize_link_retention(105, 10).unwrap();
        assert_eq!(
            (link.created_at_ms, link.expires_at_ms),
            (Some(100), Some(110))
        );
        link.extend_link_retention(105, 20).unwrap();
        assert_eq!(link.expires_at_ms, Some(125));
        link.extend_link_retention(106, 1).unwrap();
        assert_eq!(link.expires_at_ms, Some(125));
        assert!(link.check_link_at(124).is_ok());
        assert_eq!(
            link.check_link_at(125).unwrap_err(),
            "remote_view_handoff_expired"
        );
        let before = link.clone();
        assert!(link.extend_link_retention(125, 20).is_err());
        assert_eq!(link, before);
        assert_eq!((&*link.session_id, &*link.tab_id), ("session", "tab"));
        assert_eq!(
            serde_json::from_value::<RemoteViewTabHandoff>(serde_json::to_value(&link).unwrap())
                .unwrap(),
            link
        );
    }

    #[test]
    fn handoff_link_retention_rejects_zero_and_overflow_without_mutation() {
        let mut link: RemoteViewTabHandoff =
            serde_json::from_str(r#"{"id":"link","sessionId":"session","tabId":"tab"}"#).unwrap();
        let before = link.clone();
        for (now, ttl) in [(1, 0), (u64::MAX, 1)] {
            assert!(link.initialize_link_retention(now, ttl).is_err());
            assert_eq!(link, before);
        }
    }
}
