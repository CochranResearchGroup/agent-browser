//! Agent Browser-owned retention for public Remote View presentation identity.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    BrowserSessionState, RemoteViewAssignmentState, RemoteViewDesktopPresentationBinding,
    RemoteViewJoinedReleaseOutcome, REMOTE_VIEW_FOUNDATION_SCHEMA_VERSION,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteViewPresentationRetentionState {
    Active,
    Detached,
    Released,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewPresentationRetention {
    pub browser_id: String,
    pub profile_id: String,
    pub session_id: String,
    pub tab_id: String,
    pub target_id: String,
    pub registration_id: String,
    pub pool_id: String,
    pub desktop_id: String,
    pub generation: u64,
    pub assignment_id: String,
    /// Legacy J3 compatibility evidence. New Agent Browser retention does not
    /// depend on Remote View managed application placement.
    #[serde(default)]
    pub placement_id: String,
    pub route_id: String,
    pub viewer_session_ids: Vec<String>,
    pub state: RemoteViewPresentationRetentionState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewOperatorHandoffLink {
    pub handoff_id: String,
    pub handoff_url: String,
}

pub fn project_remote_view_operator_handoff(
    retained: &RemoteViewPresentationRetention,
    handoff_id: &str,
) -> Result<RemoteViewOperatorHandoffLink, String> {
    let id_is_safe = !handoff_id.is_empty()
        && handoff_id.len() <= 128
        && !matches!(handoff_id, "." | "..")
        && handoff_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'));
    if retained.state != RemoteViewPresentationRetentionState::Active {
        return Err("remote_view_operator_handoff_binding_not_active".to_string());
    }
    if !id_is_safe {
        return Err("remote_view_operator_handoff_id_invalid".to_string());
    }
    Ok(RemoteViewOperatorHandoffLink {
        handoff_id: handoff_id.to_string(),
        handoff_url: format!("/remote-view/{handoff_id}"),
    })
}

pub fn retain_remote_view_presentation(
    state: &mut BrowserSessionState,
    browser_id: &str,
    session_id: &str,
    tab_id: &str,
    binding: &RemoteViewDesktopPresentationBinding,
) -> Result<RemoteViewPresentationRetention, String> {
    let browser = state
        .browsers
        .get(browser_id)
        .ok_or_else(|| format!("remote_view_retention_browser_missing:{browser_id}"))?;
    let session = state
        .sessions
        .get(session_id)
        .ok_or_else(|| format!("remote_view_retention_session_missing:{session_id}"))?;
    let tab = state
        .tabs
        .get(tab_id)
        .ok_or_else(|| format!("remote_view_retention_tab_missing:{tab_id}"))?;
    let desktop = browser
        .desktop
        .as_ref()
        .ok_or_else(|| format!("remote_view_retention_desktop_missing:{browser_id}"))?;
    let viewer_ids = binding.viewer_session_ids.iter().collect::<BTreeSet<_>>();
    if browser.id != browser_id
        || session.id != session_id
        || session.browser_id != browser_id
        || session.profile_id != browser.profile_id
        || !browser
            .active_session_ids
            .iter()
            .any(|candidate| candidate == session_id)
        || tab.id != tab_id
        || tab.browser_id != browser_id
        || tab.session_id != session_id
        || desktop.desktop_id != binding.desktop_id
        || desktop.generation != binding.generation
        || binding.generation == 0
        || binding.registration_id.trim().is_empty()
        || binding.pool_id.trim().is_empty()
        || binding.assignment_id.trim().is_empty()
        || binding.route_id.trim().is_empty()
        || binding.viewer_session_ids.is_empty()
        || viewer_ids.len() != binding.viewer_session_ids.len()
        || binding
            .viewer_session_ids
            .iter()
            .any(|id| id.trim().is_empty())
    {
        return Err("remote_view_retention_identity_mismatch".to_string());
    }
    let mut viewer_session_ids = binding.viewer_session_ids.clone();
    viewer_session_ids.sort();
    let retained = RemoteViewPresentationRetention {
        browser_id: browser_id.to_string(),
        profile_id: browser.profile_id.clone(),
        session_id: session_id.to_string(),
        tab_id: tab_id.to_string(),
        target_id: tab.target_id.clone(),
        registration_id: binding.registration_id.clone(),
        pool_id: binding.pool_id.clone(),
        desktop_id: binding.desktop_id.clone(),
        generation: binding.generation,
        assignment_id: binding.assignment_id.clone(),
        placement_id: String::new(),
        route_id: binding.route_id.clone(),
        viewer_session_ids,
        state: RemoteViewPresentationRetentionState::Active,
    };
    if let Some(existing) = state.remote_view_presentations.get(browser_id) {
        return if existing == &retained {
            Ok(existing.clone())
        } else {
            Err(format!("remote_view_retention_conflict:{browser_id}"))
        };
    }
    let conflicts = state.remote_view_presentations.values().any(|existing| {
        if existing.state == RemoteViewPresentationRetentionState::Released {
            return false;
        }
        let shares_desktop_identity = existing.desktop_id == retained.desktop_id
            || existing.assignment_id == retained.assignment_id
            || existing.route_id == retained.route_id
            || existing
                .viewer_session_ids
                .iter()
                .any(|id| viewer_ids.contains(id));
        shares_desktop_identity && !same_desktop_presentation(existing, &retained)
    });
    if conflicts {
        return Err("remote_view_retention_identity_already_bound".to_string());
    }
    state
        .remote_view_presentations
        .insert(browser_id.to_string(), retained.clone());
    Ok(retained)
}

/// Detach one browser from a shared desktop presentation without releasing the
/// Remote View assignment. The returned count is the number of other active
/// Agent Browser references to the exact assignment.
pub fn detach_remote_view_presentation(
    state: &mut BrowserSessionState,
    browser_id: &str,
) -> Result<usize, String> {
    let retained = state
        .remote_view_presentations
        .get(browser_id)
        .ok_or_else(|| format!("remote_view_retention_missing:{browser_id}"))?;
    if retained.state == RemoteViewPresentationRetentionState::Released {
        return Err(format!(
            "remote_view_retention_already_released:{browser_id}"
        ));
    }
    let assignment_id = retained.assignment_id.clone();
    if retained.state == RemoteViewPresentationRetentionState::Active {
        state
            .remote_view_presentations
            .get_mut(browser_id)
            .expect("retention existence was checked")
            .state = RemoteViewPresentationRetentionState::Detached;
    }
    Ok(state
        .remote_view_presentations
        .values()
        .filter(|candidate| {
            candidate.state == RemoteViewPresentationRetentionState::Active
                && candidate.assignment_id == assignment_id
        })
        .count())
}

pub fn release_remote_view_presentation(
    state: &mut BrowserSessionState,
    browser_id: &str,
    release: &RemoteViewJoinedReleaseOutcome,
) -> Result<RemoteViewPresentationRetention, String> {
    let retained = state
        .remote_view_presentations
        .get(browser_id)
        .ok_or_else(|| format!("remote_view_retention_missing:{browser_id}"))?;
    let peer_reference_count = state
        .remote_view_presentations
        .iter()
        .filter(|(candidate_browser_id, candidate)| {
            candidate_browser_id.as_str() != browser_id
                && candidate.state == RemoteViewPresentationRetentionState::Active
                && candidate.assignment_id == retained.assignment_id
        })
        .count();
    if peer_reference_count != 0 {
        return Err(format!(
            "remote_view_retention_release_still_referenced:{browser_id}:{peer_reference_count}"
        ));
    }
    let retired_routes = release.retirement.routes.iter().collect::<BTreeSet<_>>();
    let retired_sessions = release.retirement.sessions.iter().collect::<BTreeSet<_>>();
    let expected_sessions = retained.viewer_session_ids.iter().collect::<BTreeSet<_>>();
    if release.assignment.state != RemoteViewAssignmentState::Released
        || release.assignment.assignment_id != retained.assignment_id
        || release.assignment.registration_id != retained.registration_id
        || release.assignment.pool_id != retained.pool_id
        || release.assignment.desktop_id != retained.desktop_id
        || release.assignment.generation != retained.generation
        || release.retirement.schema_version != REMOTE_VIEW_FOUNDATION_SCHEMA_VERSION
        || release.retirement.desktop_id != retained.desktop_id
        || release.retirement.generation != retained.generation
        || retired_routes != BTreeSet::from([&retained.route_id])
        || retired_sessions != expected_sessions
    {
        return Err(format!(
            "remote_view_retention_release_mismatch:{browser_id}"
        ));
    }
    let retained = state
        .remote_view_presentations
        .get_mut(browser_id)
        .expect("retention existence was checked");
    retained.state = RemoteViewPresentationRetentionState::Released;
    Ok(retained.clone())
}

fn same_desktop_presentation(
    left: &RemoteViewPresentationRetention,
    right: &RemoteViewPresentationRetention,
) -> bool {
    left.registration_id == right.registration_id
        && left.pool_id == right.pool_id
        && left.desktop_id == right.desktop_id
        && left.generation == right.generation
        && left.assignment_id == right.assignment_id
        && left.route_id == right.route_id
        && left.viewer_session_ids == right.viewer_session_ids
}
