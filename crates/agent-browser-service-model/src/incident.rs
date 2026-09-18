//! Provider-free durable service-incident record contracts.

use serde::{Deserialize, Serialize};

use crate::BrowserHealth;

pub const SERVICE_INCIDENT_STATE_VALUES: [&str; 3] = ["active", "recovered", "service"];
pub const SERVICE_INCIDENT_SEVERITY_VALUES: [&str; 4] = ["info", "warning", "error", "critical"];
pub const SERVICE_INCIDENT_ESCALATION_VALUES: [&str; 7] = [
    "none",
    "browser_degraded",
    "browser_recovery",
    "job_attention",
    "monitor_attention",
    "service_triage",
    "os_degraded_possible",
];

/// Grouped service incident derived from event and job history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceIncident {
    pub id: String,
    pub browser_id: Option<String>,
    pub monitor_id: Option<String>,
    pub monitor_target: Option<serde_json::Value>,
    pub monitor_result: Option<String>,
    pub label: String,
    pub state: ServiceIncidentState,
    pub severity: ServiceIncidentSeverity,
    pub escalation: ServiceIncidentEscalation,
    pub recommended_action: String,
    pub acknowledged_at: Option<String>,
    pub acknowledged_by: Option<String>,
    pub acknowledgement_note: Option<String>,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<String>,
    pub resolution_note: Option<String>,
    pub latest_timestamp: String,
    pub latest_message: String,
    pub latest_kind: String,
    pub current_health: Option<BrowserHealth>,
    pub event_ids: Vec<String>,
    pub job_ids: Vec<String>,
}

/// Operator-facing summary state for a grouped incident.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceIncidentState {
    #[default]
    Active,
    Recovered,
    Service,
}

/// Operator-facing severity for a grouped incident.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceIncidentSeverity {
    #[default]
    Info,
    Warning,
    Error,
    Critical,
}

/// Operator escalation bucket for a grouped incident.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceIncidentEscalation {
    #[default]
    None,
    BrowserDegraded,
    BrowserRecovery,
    JobAttention,
    MonitorAttention,
    ServiceTriage,
    OsDegradedPossible,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_match_the_durable_incident_contract() {
        let incident = ServiceIncident::default();
        assert_eq!(incident.state, ServiceIncidentState::Active);
        assert_eq!(incident.severity, ServiceIncidentSeverity::Info);
        assert_eq!(incident.escalation, ServiceIncidentEscalation::None);
        assert!(incident.event_ids.is_empty());
        assert!(incident.job_ids.is_empty());
    }

    #[test]
    fn record_serializes_stable_camel_case_fields_and_enum_values() {
        let incident = ServiceIncident {
            id: "browser:browser-1".to_string(),
            browser_id: Some("browser-1".to_string()),
            monitor_target: Some(json!({"site_policy": "google"})),
            state: ServiceIncidentState::Service,
            severity: ServiceIncidentSeverity::Critical,
            escalation: ServiceIncidentEscalation::OsDegradedPossible,
            current_health: Some(BrowserHealth::Faulted),
            event_ids: vec!["event-1".to_string()],
            job_ids: vec!["job-1".to_string()],
            ..ServiceIncident::default()
        };

        let value = serde_json::to_value(incident).unwrap();
        assert_eq!(value["browserId"], "browser-1");
        assert_eq!(value["monitorTarget"]["site_policy"], "google");
        assert_eq!(value["state"], "service");
        assert_eq!(value["severity"], "critical");
        assert_eq!(value["escalation"], "os_degraded_possible");
        assert_eq!(value["currentHealth"], "faulted");
        assert!(value.get("browser_id").is_none());
        assert!(value.get("monitor_target").is_none());
        assert!(value.get("current_health").is_none());
    }

    #[test]
    fn constants_match_incident_wire_variants() {
        assert_eq!(SERVICE_INCIDENT_STATE_VALUES[2], "service");
        assert_eq!(SERVICE_INCIDENT_SEVERITY_VALUES[3], "critical");
        assert_eq!(
            SERVICE_INCIDENT_ESCALATION_VALUES[6],
            "os_degraded_possible"
        );
    }
}
