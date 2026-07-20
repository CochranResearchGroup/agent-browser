use std::collections::HashSet;

use serde::Serialize;

use super::service_model::{ServiceState, TabLifecycle};

pub const ORDINARY_CLOSED_TAB_CAP: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedTabProjectionMetadata {
    pub mode: &'static str,
    pub cap: Option<usize>,
    pub total_closed_count: usize,
    pub retained_closed_count: usize,
    pub omitted_closed_count: usize,
    pub ordering: &'static str,
    pub diagnostic_available: bool,
}

/// Builds a response-only status projection. The input remains the persisted
/// lifecycle authority and is never mutated by compaction.
pub fn project_service_status(
    state: &ServiceState,
    full_tab_history: bool,
) -> (ServiceState, ClosedTabProjectionMetadata) {
    let total_closed_count = state
        .tabs
        .values()
        .filter(|tab| tab.lifecycle == TabLifecycle::Closed)
        .count();
    if full_tab_history {
        return (
            state.clone(),
            ClosedTabProjectionMetadata {
                mode: "full",
                cap: None,
                total_closed_count,
                retained_closed_count: total_closed_count,
                omitted_closed_count: 0,
                ordering: "tab_id_descending",
                diagnostic_available: true,
            },
        );
    }

    let session_tab_ids = state
        .sessions
        .values()
        .flat_map(|session| session.tab_ids.iter().cloned())
        .collect::<HashSet<_>>();
    let mut compactable_closed_ids = state
        .tabs
        .values()
        .filter(|tab| {
            tab.lifecycle == TabLifecycle::Closed
                && !session_tab_ids.contains(&tab.id)
                && tab.owner_session_id.is_none()
                && tab.service_tab_handle.is_none()
                && tab.challenge_id.is_none()
                && tab.latest_snapshot_id.is_none()
                && tab.latest_screenshot_id.is_none()
        })
        .map(|tab| tab.id.clone())
        .collect::<Vec<_>>();
    compactable_closed_ids.sort_by(|left, right| right.cmp(left));
    let omitted_ids = compactable_closed_ids
        .into_iter()
        .skip(ORDINARY_CLOSED_TAB_CAP)
        .collect::<HashSet<_>>();

    let mut projected = state.clone();
    projected.tabs.retain(|id, _| !omitted_ids.contains(id));
    let retained_closed_count = projected
        .tabs
        .values()
        .filter(|tab| tab.lifecycle == TabLifecycle::Closed)
        .count();
    (
        projected,
        ClosedTabProjectionMetadata {
            mode: "bounded",
            cap: Some(ORDINARY_CLOSED_TAB_CAP),
            total_closed_count,
            retained_closed_count,
            omitted_closed_count: omitted_ids.len(),
            ordering: "tab_id_descending",
            diagnostic_available: true,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{BrowserSession, BrowserTab, ServiceTabHandle};

    fn closed_tab(id: &str) -> BrowserTab {
        BrowserTab {
            id: id.to_string(),
            lifecycle: TabLifecycle::Closed,
            ..BrowserTab::default()
        }
    }

    #[test]
    fn ordinary_status_caps_only_unreferenced_closed_history() {
        let mut state = ServiceState::default();
        for index in 0..55 {
            let id = format!("closed-{index:03}");
            state.tabs.insert(id.clone(), closed_tab(&id));
        }
        state.tabs.insert(
            "ready".to_string(),
            BrowserTab {
                id: "ready".to_string(),
                lifecycle: TabLifecycle::Ready,
                ..BrowserTab::default()
            },
        );
        let mut referenced = closed_tab("closed-referenced");
        referenced.service_tab_handle = Some(ServiceTabHandle {
            valid: false,
            stale_reason: Some("target_closed".to_string()),
            ..ServiceTabHandle::default()
        });
        state.tabs.insert(referenced.id.clone(), referenced);

        let (projected, metadata) = project_service_status(&state, false);

        assert_eq!(state.tabs.len(), 57);
        assert_eq!(projected.tabs.len(), 52);
        assert!(projected.tabs.contains_key("ready"));
        assert!(projected.tabs.contains_key("closed-referenced"));
        assert_eq!(
            projected.tabs["closed-referenced"]
                .service_tab_handle
                .as_ref()
                .and_then(|handle| handle.stale_reason.as_deref()),
            Some("target_closed")
        );
        assert_eq!(metadata.total_closed_count, 56);
        assert_eq!(metadata.retained_closed_count, 51);
        assert_eq!(metadata.omitted_closed_count, 5);
    }

    #[test]
    fn session_referenced_closed_tab_survives_ordinary_projection() {
        let mut state = ServiceState::default();
        state
            .tabs
            .insert("session-tab".to_string(), closed_tab("session-tab"));
        state.sessions.insert(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                tab_ids: vec!["session-tab".to_string()],
                ..BrowserSession::default()
            },
        );
        for index in 0..55 {
            let id = format!("plain-{index:03}");
            state.tabs.insert(id.clone(), closed_tab(&id));
        }

        let (projected, _) = project_service_status(&state, false);
        assert!(projected.tabs.contains_key("session-tab"));
    }

    #[test]
    fn full_history_returns_the_complete_unmodified_authority() {
        let mut state = ServiceState::default();
        for index in 0..55 {
            let id = format!("closed-{index:03}");
            state.tabs.insert(id.clone(), closed_tab(&id));
        }

        let (projected, metadata) = project_service_status(&state, true);
        assert_eq!(projected, state);
        assert_eq!(metadata.mode, "full");
        assert_eq!(metadata.omitted_closed_count, 0);
    }
}
