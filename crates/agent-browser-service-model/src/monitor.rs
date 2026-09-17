//! Provider-free durable monitor record contracts.

use serde::{Deserialize, Serialize};

pub const SERVICE_MONITOR_STATE_VALUES: [&str; 3] = ["active", "paused", "faulted"];

/// Site or tab heartbeat managed by the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SiteMonitor {
    pub id: String,
    pub name: String,
    pub target: MonitorTarget,
    pub interval_ms: u64,
    pub state: MonitorState,
    pub last_checked_at: Option<String>,
    pub last_succeeded_at: Option<String>,
    pub last_failed_at: Option<String>,
    pub last_result: Option<String>,
    pub consecutive_failures: u64,
}

impl Default for SiteMonitor {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            target: MonitorTarget::Url(String::new()),
            interval_ms: 60_000,
            state: MonitorState::Paused,
            last_checked_at: None,
            last_succeeded_at: None,
            last_failed_at: None,
            last_result: None,
            consecutive_failures: 0,
        }
    }
}

/// Monitor target variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorTarget {
    Url(String),
    Tab(String),
    SitePolicy(String),
    /// Checks retained no-launch target readiness for a login/service identity.
    ProfileReadiness(String),
}

/// Monitor execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorState {
    Active,
    Paused,
    Faulted,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_preserves_legacy_monitor_contract() {
        let monitor: SiteMonitor = serde_json::from_value(json!({})).unwrap();

        assert_eq!(monitor, SiteMonitor::default());
        assert_eq!(monitor.target, MonitorTarget::Url(String::new()));
        assert_eq!(monitor.interval_ms, 60_000);
        assert_eq!(monitor.state, MonitorState::Paused);
        assert_eq!(monitor.consecutive_failures, 0);
    }

    #[test]
    fn monitor_serializes_stable_camel_case_fields() {
        let monitor = SiteMonitor {
            id: "monitor-1".to_string(),
            name: "Google readiness".to_string(),
            target: MonitorTarget::ProfileReadiness("google".to_string()),
            interval_ms: 30_000,
            state: MonitorState::Faulted,
            last_checked_at: Some("2026-09-16T12:00:00Z".to_string()),
            last_succeeded_at: None,
            last_failed_at: Some("2026-09-16T12:00:00Z".to_string()),
            last_result: Some("authentication_required".to_string()),
            consecutive_failures: 2,
        };

        assert_eq!(
            serde_json::to_value(monitor).unwrap(),
            json!({
                "id": "monitor-1",
                "name": "Google readiness",
                "target": {"profile_readiness": "google"},
                "intervalMs": 30_000,
                "state": "faulted",
                "lastCheckedAt": "2026-09-16T12:00:00Z",
                "lastSucceededAt": null,
                "lastFailedAt": "2026-09-16T12:00:00Z",
                "lastResult": "authentication_required",
                "consecutiveFailures": 2
            })
        );
    }

    #[test]
    fn target_and_state_wire_values_remain_exact() {
        assert_eq!(
            [
                MonitorTarget::Url("https://example.com".to_string()),
                MonitorTarget::Tab("tab-1".to_string()),
                MonitorTarget::SitePolicy("policy-1".to_string()),
                MonitorTarget::ProfileReadiness("google".to_string()),
            ]
            .map(|target| serde_json::to_value(target).unwrap()),
            [
                json!({"url": "https://example.com"}),
                json!({"tab": "tab-1"}),
                json!({"site_policy": "policy-1"}),
                json!({"profile_readiness": "google"}),
            ]
        );
        assert_eq!(
            [
                MonitorState::Active,
                MonitorState::Paused,
                MonitorState::Faulted,
            ]
            .map(|state| serde_json::to_value(state).unwrap()),
            SERVICE_MONITOR_STATE_VALUES.map(serde_json::Value::from)
        );
    }
}
