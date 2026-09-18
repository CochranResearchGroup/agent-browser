//! Provider-free durable operational snapshot records.

use serde::{Deserialize, Serialize};

/// Latest persisted service reconciliation result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceReconciliationSnapshot {
    pub last_reconciled_at: Option<String>,
    pub last_error: Option<String>,
    pub browser_count: usize,
    pub changed_browsers: usize,
}

/// Latest persisted control-plane status snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ControlPlaneSnapshot {
    pub worker_state: String,
    pub browser_health: String,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    /// Number of retained jobs currently delayed by profile lease contention.
    pub waiting_profile_lease_job_count: usize,
    pub service_job_timeout_ms: Option<u64>,
    pub service_monitor_interval_ms: Option<u64>,
    pub updated_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_input_preserves_snapshot_defaults() {
        let reconciliation: ServiceReconciliationSnapshot =
            serde_json::from_value(json!({})).unwrap();
        let control_plane: ControlPlaneSnapshot = serde_json::from_value(json!({})).unwrap();

        assert_eq!(reconciliation, ServiceReconciliationSnapshot::default());
        assert_eq!(control_plane, ControlPlaneSnapshot::default());
        assert_eq!(
            serde_json::to_value(reconciliation).unwrap(),
            json!({
                "lastReconciledAt": null,
                "lastError": null,
                "browserCount": 0,
                "changedBrowsers": 0
            })
        );
        assert_eq!(
            serde_json::to_value(control_plane).unwrap(),
            json!({
                "workerState": "",
                "browserHealth": "",
                "queueDepth": 0,
                "queueCapacity": 0,
                "waitingProfileLeaseJobCount": 0,
                "serviceJobTimeoutMs": null,
                "serviceMonitorIntervalMs": null,
                "updatedAt": null
            })
        );
    }

    #[test]
    fn populated_snapshots_use_stable_camel_case_fields() {
        let reconciliation = ServiceReconciliationSnapshot {
            last_reconciled_at: Some("2026-09-16T12:00:00Z".to_string()),
            last_error: Some("bounded diagnostic".to_string()),
            browser_count: 4,
            changed_browsers: 2,
        };
        let control_plane = ControlPlaneSnapshot {
            worker_state: "running".to_string(),
            browser_health: "healthy".to_string(),
            queue_depth: 3,
            queue_capacity: 16,
            waiting_profile_lease_job_count: 1,
            service_job_timeout_ms: Some(900_000),
            service_monitor_interval_ms: Some(60_000),
            updated_at: Some("2026-09-16T12:00:01Z".to_string()),
        };

        assert_eq!(
            serde_json::to_value(reconciliation).unwrap(),
            json!({
                "lastReconciledAt": "2026-09-16T12:00:00Z",
                "lastError": "bounded diagnostic",
                "browserCount": 4,
                "changedBrowsers": 2
            })
        );
        assert_eq!(
            serde_json::to_value(control_plane).unwrap(),
            json!({
                "workerState": "running",
                "browserHealth": "healthy",
                "queueDepth": 3,
                "queueCapacity": 16,
                "waitingProfileLeaseJobCount": 1,
                "serviceJobTimeoutMs": 900_000,
                "serviceMonitorIntervalMs": 60_000,
                "updatedAt": "2026-09-16T12:00:01Z"
            })
        );
    }
}
