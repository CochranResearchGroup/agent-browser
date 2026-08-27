use std::collections::HashSet;

use super::{ClosedTabProjectionMetadata, ServiceStatusProjectionError, ORDINARY_CLOSED_TAB_CAP};
use crate::native::service_model::{ServiceState, TabLifecycle};

pub(super) fn validate_authority(state: &ServiceState) -> Result<(), ServiceStatusProjectionError> {
    for (key, browser) in &state.browsers {
        if browser.id.is_empty() || browser.id != *key {
            return Err(ServiceStatusProjectionError::InvalidAuthority(format!(
                "browser map key {key:?} does not match browser id {:?}",
                browser.id
            )));
        }
    }
    Ok(())
}

pub(super) fn project_closed_tabs(
    state: &ServiceState,
    full_tab_history: bool,
) -> (ServiceState, ClosedTabProjectionMetadata) {
    let total_closed_count = state
        .tabs
        .values()
        .filter(|tab| tab.lifecycle == TabLifecycle::Closed)
        .count();
    if full_tab_history {
        let mut projected = state.clone();
        projected.crash_regeneration_transactions.clear();
        return (
            projected,
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
    projected.crash_regeneration_transactions.clear();
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
    use crate::native::service_crash_regeneration::CrashRegenerationTransaction;
    use serde_json::json;

    fn state_with_private_crash_evidence() -> ServiceState {
        let transaction: CrashRegenerationTransaction = serde_json::from_value(json!({
            "transactionId": "tx-private",
            "bootEpoch": "boot-private",
            "stableIdentities": {
                "principalId": "principal-stable",
                "profileId": "profile-stable",
                "logicalBrowserId": "browser-stable",
                "sessionRoute": "session:stable",
                "routeId": "route-stable",
                "connectionId": "connection-stable",
                "routeUserId": "route-user-stable",
                "handoffId": "handoff-stable"
            },
            "state": "interrupted",
            "revision": 2,
            "replayCount": 1,
            "completedPhases": ["runtime_host_authority"],
            "currentPhase": "browser_authority",
            "evidence": {
                "runtimeHostPid": 4242,
                "socketIdentity": "socket-private",
                "displayName": ":101"
            },
            "lastError": "private-error"
        }))
        .unwrap();
        let mut state = ServiceState::default();
        state
            .crash_regeneration_transactions
            .insert("tx-private".to_string(), transaction);
        state
    }

    #[test]
    fn public_service_state_omits_private_crash_evidence_in_both_history_modes() {
        let state = state_with_private_crash_evidence();
        for full_tab_history in [false, true] {
            let (projected, _) = project_closed_tabs(&state, full_tab_history);
            assert!(projected.crash_regeneration_transactions.is_empty());
        }
        assert_eq!(state.crash_regeneration_transactions.len(), 1);
    }
}
