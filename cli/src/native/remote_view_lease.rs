use serde::{Deserialize, Serialize};

/// Typed lifecycle for a route-bound remote-view acquisition lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteBoundLeaseState {
    Requested,
    Planned,
    Reserved,
    DisplayReady,
    BrowserAttached,
    TabAcquired,
    ProofReady,
    Finalized,
    RolledBack,
    FailedDiagnostic,
}

impl RouteBoundLeaseState {
    pub fn as_state_phase(self) -> (&'static str, &'static str) {
        match self {
            Self::Requested => ("pending", "requested"),
            Self::Planned => ("pending", "planned"),
            Self::Reserved => ("pending", "reserved"),
            Self::DisplayReady => ("pending", "display_ready"),
            Self::BrowserAttached => ("pending", "browser_attached"),
            Self::TabAcquired => ("pending", "tab_acquired"),
            Self::ProofReady => ("pending", "proof_ready"),
            Self::Finalized => ("completed", "checked_out"),
            Self::RolledBack => ("failed", "rollback_complete"),
            Self::FailedDiagnostic => ("failed", "failed_diagnostic"),
        }
    }

    pub fn from_state_phase(state: &str, phase: &str) -> Option<Self> {
        match (state, phase) {
            ("pending", "requested") => Some(Self::Requested),
            ("pending", "planned") => Some(Self::Planned),
            ("pending", "reserved") => Some(Self::Reserved),
            ("pending", "display_ready") => Some(Self::DisplayReady),
            ("pending", "browser_attached") => Some(Self::BrowserAttached),
            ("pending", "tab_acquired") => Some(Self::TabAcquired),
            ("pending", "proof_ready") => Some(Self::ProofReady),
            ("completed", "checked_out") => Some(Self::Finalized),
            ("failed", "rollback_complete") => Some(Self::RolledBack),
            ("failed", "failed_diagnostic") => Some(Self::FailedDiagnostic),
            _ => None,
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        use RouteBoundLeaseState::*;
        matches!(
            (self, next),
            (Requested, Planned)
                | (Planned, Reserved)
                | (Reserved, DisplayReady)
                | (DisplayReady, BrowserAttached)
                | (BrowserAttached, TabAcquired)
                | (TabAcquired, ProofReady)
                | (ProofReady, Finalized)
                | (Requested, RolledBack)
                | (Planned, RolledBack)
                | (Reserved, RolledBack)
                | (DisplayReady, RolledBack)
                | (BrowserAttached, RolledBack)
                | (TabAcquired, RolledBack)
                | (ProofReady, RolledBack)
                | (Requested, FailedDiagnostic)
                | (Planned, FailedDiagnostic)
                | (Reserved, FailedDiagnostic)
                | (DisplayReady, FailedDiagnostic)
                | (BrowserAttached, FailedDiagnostic)
                | (TabAcquired, FailedDiagnostic)
                | (ProofReady, FailedDiagnostic)
        )
    }

    pub fn can_publish_live_control_row(self) -> bool {
        self == Self::Finalized
    }

    pub fn failure_publishable_as_live_control(self) -> bool {
        matches!(self, Self::RolledBack | Self::FailedDiagnostic)
            && self.can_publish_live_control_row()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteBoundLeaseTransitionError {
    pub current: RouteBoundLeaseState,
    pub next: RouteBoundLeaseState,
}

impl std::fmt::Display for RouteBoundLeaseTransitionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "invalid_route_bound_lease_transition: {:?} -> {:?}",
            self.current, self.next
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteBoundLeaseLifecycle {
    pub state: RouteBoundLeaseState,
}

impl RouteBoundLeaseLifecycle {
    pub fn new() -> Self {
        Self {
            state: RouteBoundLeaseState::Requested,
        }
    }

    pub fn from_state_phase(state: &str, phase: &str) -> Option<Self> {
        RouteBoundLeaseState::from_state_phase(state, phase).map(|state| Self { state })
    }

    pub fn transition_to(
        &mut self,
        next: RouteBoundLeaseState,
    ) -> Result<(), RouteBoundLeaseTransitionError> {
        if self.state.can_transition_to(next) {
            self.state = next;
            Ok(())
        } else {
            Err(RouteBoundLeaseTransitionError {
                current: self.state,
                next,
            })
        }
    }

    pub fn state_phase(&self) -> (&'static str, &'static str) {
        self.state.as_state_phase()
    }
}

impl Default for RouteBoundLeaseLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{RouteBoundLeaseLifecycle, RouteBoundLeaseState};

    #[test]
    fn route_bound_lease_lifecycle_allows_ordered_success_path() {
        let mut lifecycle = RouteBoundLeaseLifecycle::new();
        for next in [
            RouteBoundLeaseState::Planned,
            RouteBoundLeaseState::Reserved,
            RouteBoundLeaseState::DisplayReady,
            RouteBoundLeaseState::BrowserAttached,
            RouteBoundLeaseState::TabAcquired,
            RouteBoundLeaseState::ProofReady,
            RouteBoundLeaseState::Finalized,
        ] {
            lifecycle.transition_to(next).unwrap();
        }

        assert_eq!(
            lifecycle.state_phase(),
            RouteBoundLeaseState::Finalized.as_state_phase()
        );
        assert!(lifecycle.state.can_publish_live_control_row());
    }

    #[test]
    fn route_bound_lease_lifecycle_rejects_illegal_skips() {
        let mut lifecycle = RouteBoundLeaseLifecycle::new();

        let error = lifecycle
            .transition_to(RouteBoundLeaseState::Finalized)
            .unwrap_err();

        assert_eq!(error.current, RouteBoundLeaseState::Requested);
        assert_eq!(error.next, RouteBoundLeaseState::Finalized);
        assert!(!lifecycle.state.can_publish_live_control_row());
    }

    #[test]
    fn route_bound_lease_lifecycle_allows_failure_from_pending_states_only() {
        let mut lifecycle = RouteBoundLeaseLifecycle::new();
        lifecycle
            .transition_to(RouteBoundLeaseState::Planned)
            .unwrap();
        lifecycle
            .transition_to(RouteBoundLeaseState::Reserved)
            .unwrap();
        lifecycle
            .transition_to(RouteBoundLeaseState::RolledBack)
            .unwrap();

        assert_eq!(
            lifecycle.state_phase(),
            RouteBoundLeaseState::RolledBack.as_state_phase()
        );
        assert!(!lifecycle.state.can_publish_live_control_row());
        assert!(!lifecycle.state.failure_publishable_as_live_control());
        assert!(lifecycle
            .transition_to(RouteBoundLeaseState::Finalized)
            .is_err());
    }

    #[test]
    fn route_bound_lease_lifecycle_round_trips_persisted_state_phase() {
        let lifecycle =
            RouteBoundLeaseLifecycle::from_state_phase("pending", "proof_ready").unwrap();

        assert_eq!(lifecycle.state, RouteBoundLeaseState::ProofReady);
        assert_eq!(lifecycle.state_phase(), ("pending", "proof_ready"));
        assert!(RouteBoundLeaseLifecycle::from_state_phase("completed", "reserved").is_none());
    }
}
