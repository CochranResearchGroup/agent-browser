//! Durable hosted connectivity is independent of browser, viewer and control custody.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserHostedLossPolicy {
    #[default]
    ReportOnly,
    Reopen,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserHostedConnectionState {
    #[default]
    Unverified,
    Connected,
    Disconnected,
    BrowserUnreachable,
    BrowserLost,
    RebindRequired,
    Pending,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserHostedReconnectRequest {
    pub fingerprint: String,
    pub operation: String,
    pub revision: u64,
    /// Pending includes uncertain effects. It never authorizes a retry.
    pub pending: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserHostedContinuity {
    /// Only a hold created by disconnect may be released by reconnect.
    #[serde(default)]
    pub disconnect_hold: bool,
    pub revision: u64,
    pub state: BrowserHostedConnectionState,
    pub loss_policy: BrowserHostedLossPolicy,
    pub last_request_id: Option<String>,
    pub observed_desktop: Option<crate::RemoteViewFixedDesktop>,
    /// Explicit future recovery placement, never proof a browser moved or connected.
    #[serde(default)]
    pub recovery_desktop: Option<crate::RemoteViewFixedDesktop>,
    /// Exact request receipts deliberately fail full rather than evict uncertain work.
    #[serde(default)]
    pub requests: BTreeMap<String, BrowserHostedReconnectRequest>,
}
impl BrowserHostedContinuity {
    pub fn admit(
        &self,
        request_id: &str,
        fingerprint: &str,
        expected: u64,
    ) -> Result<bool, String> {
        if request_id.is_empty()
            || request_id.len() > 128
            || request_id.chars().any(char::is_control)
        {
            return Err("browser_reconnect_request_id_invalid".into());
        }
        if let Some(prior) = self.requests.get(request_id) {
            if prior.fingerprint != fingerprint {
                return Err("browser_reconnect_request_identity_conflict".into());
            }
            if prior.pending {
                return Err("browser_reconnect_pending_no_replay".into());
            }
            return Ok(false);
        }
        if self.revision != expected {
            return Err("browser_reconnect_revision_conflict".into());
        }
        if self.requests.values().any(|row| row.pending) {
            return Err("browser_reconnect_pending_no_replay".into());
        }
        if self.requests.len() >= 128 {
            return Err("browser_reconnect_request_capacity_reached".into());
        }
        self.revision
            .checked_add(2)
            .ok_or("browser_reconnect_revision_exhausted")?;
        Ok(true)
    }
    pub fn record(&mut self, request_id: &str, fingerprint: &str, operation: &str, pending: bool) {
        self.revision += 1; // admit checks both intent and terminal increments.
        self.last_request_id = Some(request_id.into());
        self.requests.insert(
            request_id.into(),
            BrowserHostedReconnectRequest {
                fingerprint: fingerprint.into(),
                operation: operation.into(),
                revision: self.revision,
                pending,
            },
        );
    }
    /// Disconnect itself never denies existing automation or returns human control.
    pub fn blocks_automation(&self) -> bool {
        matches!(
            self.state,
            BrowserHostedConnectionState::BrowserLost
                | BrowserHostedConnectionState::RebindRequired
                | BrowserHostedConnectionState::Pending
        ) || self.requests.values().any(|row| row.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hosted_reconnect_preserves_disconnect_availability_and_fences_loss() {
        let mut state = BrowserHostedContinuity {
            state: BrowserHostedConnectionState::Disconnected,
            ..Default::default()
        };
        assert!(!state.blocks_automation());
        for phase in [
            BrowserHostedConnectionState::BrowserLost,
            BrowserHostedConnectionState::RebindRequired,
            BrowserHostedConnectionState::Pending,
        ] {
            state.state = phase;
            assert!(state.blocks_automation());
        }
    }
    #[test]
    fn hosted_reconnect_pending_and_changed_request_never_replay() {
        let mut state = BrowserHostedContinuity::default();
        assert_eq!(state.admit("id", "digest", 0), Ok(true));
        state.record("id", "digest", "reopen", true);
        assert!(state
            .admit("id", "digest", 0)
            .unwrap_err()
            .contains("no_replay"));
        assert!(state
            .admit("id", "changed", 1)
            .unwrap_err()
            .contains("identity_conflict"));
        assert!(state
            .admit("other", "digest", 1)
            .unwrap_err()
            .contains("no_replay"));
        let restored: BrowserHostedContinuity =
            serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        assert_eq!(restored, state);
    }
    #[test]
    fn hosted_reconnect_completed_replay_is_readback_only() {
        let mut state = BrowserHostedContinuity::default();
        state.record("id", "digest", "reconnect", false);
        assert_eq!(state.admit("id", "digest", 0), Ok(false));
        assert!(state
            .admit("new", "digest", 0)
            .unwrap_err()
            .contains("revision_conflict"));
    }
}
