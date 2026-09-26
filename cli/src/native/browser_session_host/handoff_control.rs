//! SQLite-fenced focus activation for ordinary manager handoffs.
//!
//! This is an effect boundary, not proof of a connected viewer. Viewer input
//! must use the same durable authority before control transfer is complete.

use agent_browser_service_model::{
    BrowserSessionEffects, BrowserSessionManager, FocusBrowserResult, RemoteViewHandoff,
    RouteKeeperHandoffBinding,
};

use super::{
    BrowserSessionHost, BrowserSessionPersistence, DesktopControlLease,
    DesktopControlTransferRequest,
};

pub(super) struct HandoffControlActivation<'a> {
    pub operation_id: &'a str,
    pub client_connection_id: &'a str,
}

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects> BrowserSessionHost<P, E> {
    pub(super) fn focus_controlled_handoff(
        &mut self,
        handoff: &RemoteViewHandoff,
        expected_binding: &RouteKeeperHandoffBinding,
        activation: &HandoffControlActivation<'_>,
        activity_at_ms: u64,
    ) -> Result<(FocusBrowserResult, DesktopControlLease), String> {
        // Drain any process-local event before transfer, then retain the SQLite
        // write reservation across exact authority validation and focus. Other
        // processes cannot transfer control or replace the keeper during focus.
        let _mutation =
            super::super::desktop_control_coordinator::global_desktop_control_coordinator()
                .begin_controller_mutation(&expected_binding.slot_id)?;
        let expected_state = self.state.clone();
        let state = &mut self.state;
        let effects = &mut self.effects;
        let catalog = &self.catalog;
        let config = self.manager_config.clone();
        let activation_result = self.persistence.activate_desktop_control(
            &DesktopControlTransferRequest {
                operation_id: activation.operation_id.to_owned(),
                handoff_id: handoff.id.clone(),
                client_connection_id: activation.client_connection_id.to_owned(),
                expected_binding: expected_binding.clone(),
            },
            &expected_state,
            |lease| {
                if &lease.route_binding != expected_binding {
                    return Err("browser_session_desktop_control_binding_changed".into());
                }
                let focused = BrowserSessionManager::new(state, catalog, effects, config)
                    .focus_browser(&lease.browser_id, Some(&lease.target_id), activity_at_ms)?;
                Ok((focused, state.clone()))
            },
        );
        match activation_result {
            Ok((focused, lease)) => Ok((focused, lease)),
            Err(error) => {
                self.state = expected_state;
                Err(error)
            }
        }
    }
}
