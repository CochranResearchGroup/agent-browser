//! Provider-free durable request-provenance record contracts.

use serde::{Deserialize, Serialize};

pub const SERVICE_REQUEST_PROVENANCE_SCHEMA_VERSION: &str =
    "agent-browser.service-request-provenance.v1";

/// Immutable, redacted causal identity captured when a request enters a
/// runtime lane. Only contract-approved scalar identifiers are retained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceRequestProvenance {
    pub schema_version: String,
    pub request_id: String,
    pub job_id: String,
    pub trace_id: Option<String>,
    pub caused_by_request_id: Option<String>,
    pub client_subject_id: Option<String>,
    pub identity_assurance: String,
    pub connection_instance_id: Option<String>,
    pub runtime_environment_id: Option<String>,
    pub runtime_lane_id: Option<String>,
    pub profile_id: Option<String>,
    pub profile_resource_key: Option<String>,
    pub browser_id: Option<String>,
    pub session_id: Option<String>,
    pub tab_id: Option<String>,
    pub service_name: Option<String>,
    pub agent_name: Option<String>,
    pub task_name: Option<String>,
    pub action: String,
    pub policy_revision: Option<u64>,
    pub access_decision_id: Option<String>,
}

impl Default for ServiceRequestProvenance {
    fn default() -> Self {
        Self {
            schema_version: SERVICE_REQUEST_PROVENANCE_SCHEMA_VERSION.to_string(),
            request_id: "unknown".to_string(),
            job_id: "unknown".to_string(),
            trace_id: None,
            caused_by_request_id: None,
            client_subject_id: None,
            identity_assurance: "unknown".to_string(),
            connection_instance_id: None,
            runtime_environment_id: None,
            runtime_lane_id: None,
            profile_id: None,
            profile_resource_key: None,
            browser_id: None,
            session_id: None,
            tab_id: None,
            service_name: None,
            agent_name: None,
            task_name: None,
            action: "unknown".to_string(),
            policy_revision: None,
            access_decision_id: None,
        }
    }
}

/// Derive the stable self-declared identity used by trusted local clients
/// that provide ordinary attribution labels instead of a registered
/// capability.
pub fn stable_self_declared_subject(
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Option<String> {
    let parts = [
        service_name.map(|value| format!("service:{value}")),
        agent_name.map(|value| format!("agent:{value}")),
        task_name.map(|value| format!("task:{value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Retain only the contract-recognized identity-assurance labels.
pub fn normalize_identity_assurance(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| {
            matches!(
                *value,
                "self-declared"
                    | "authenticated-ingress"
                    | "registered-capability"
                    | "operator"
                    | "unknown"
            )
        })
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_and_wire_shape_match_the_durable_contract() {
        let provenance = ServiceRequestProvenance::default();
        assert_eq!(
            provenance.schema_version,
            SERVICE_REQUEST_PROVENANCE_SCHEMA_VERSION
        );
        assert_eq!(provenance.request_id, "unknown");
        assert_eq!(provenance.job_id, "unknown");
        assert_eq!(provenance.identity_assurance, "unknown");
        assert_eq!(provenance.action, "unknown");

        let value = serde_json::to_value(provenance).unwrap();
        assert_eq!(
            value["schemaVersion"],
            SERVICE_REQUEST_PROVENANCE_SCHEMA_VERSION
        );
        assert_eq!(value["requestId"], "unknown");
        assert_eq!(value["identityAssurance"], "unknown");
        assert!(value.get("request_id").is_none());
        assert!(value.get("runtime_environment_id").is_none());
    }

    #[test]
    fn record_rejects_unknown_wire_fields() {
        let error = serde_json::from_value::<ServiceRequestProvenance>(json!({
            "requestId": "request-1",
            "credential": "must-not-be-retained"
        }))
        .unwrap_err();
        assert!(error.to_string().contains("unknown field `credential`"));
    }

    #[test]
    fn stable_self_declared_subject_preserves_component_order() {
        assert_eq!(
            stable_self_declared_subject(
                Some("research-fieldwork"),
                Some("codex"),
                Some("collect-evidence")
            )
            .as_deref(),
            Some("service:research-fieldwork/agent:codex/task:collect-evidence")
        );
        assert_eq!(stable_self_declared_subject(None, None, None), None);
    }

    #[test]
    fn identity_assurance_fails_closed_to_unknown() {
        assert_eq!(
            normalize_identity_assurance(Some("registered-capability")),
            "registered-capability"
        );
        assert_eq!(
            normalize_identity_assurance(Some("unrecognized-proof")),
            "unknown"
        );
        assert_eq!(
            normalize_identity_assurance(Some(" registered-capability ")),
            "registered-capability"
        );
        assert_eq!(normalize_identity_assurance(None), "unknown");
    }
}
