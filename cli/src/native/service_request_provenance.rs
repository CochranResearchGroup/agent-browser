use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SERVICE_REQUEST_PROVENANCE_SCHEMA_VERSION: &str =
    "agent-browser.service-request-provenance.v1";

/// Derive the stable self-declared identity used by trusted local clients
/// that provide ordinary attribution labels instead of a registered
/// capability.
pub(crate) fn stable_self_declared_subject(
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

impl ServiceRequestProvenance {
    pub fn capture(
        command: &Value,
        request_id: &str,
        job_id: &str,
        connection_instance_id: &str,
        runtime_lane_id: &str,
    ) -> Self {
        let profile_id = optional_string(command, "profileId")
            .or_else(|| optional_string(command, "runtimeProfile"));
        let profile_resource_key = optional_string(command, "profileResourceKey")
            .or_else(|| profile_id.as_ref().map(|id| format!("profile:{id}")));
        let identity_assurance = optional_string(command, "identityAssurance")
            .filter(|value| {
                matches!(
                    value.as_str(),
                    "self-declared"
                        | "authenticated-ingress"
                        | "registered-capability"
                        | "operator"
                        | "unknown"
                )
            })
            .unwrap_or_else(|| "unknown".to_string());

        Self {
            schema_version: SERVICE_REQUEST_PROVENANCE_SCHEMA_VERSION.to_string(),
            request_id: optional_string(command, "requestId")
                .unwrap_or_else(|| request_id.to_string()),
            job_id: job_id.to_string(),
            trace_id: optional_string(command, "traceId"),
            caused_by_request_id: optional_string(command, "causedByRequestId"),
            client_subject_id: optional_string(command, "clientSubjectId"),
            identity_assurance,
            connection_instance_id: nonempty(connection_instance_id),
            runtime_environment_id: optional_string(command, "runtimeEnvironmentId"),
            runtime_lane_id: nonempty(runtime_lane_id),
            profile_id,
            profile_resource_key,
            browser_id: optional_string(command, "browserId"),
            session_id: optional_string(command, "sessionId")
                .or_else(|| optional_string(command, "sessionName")),
            tab_id: optional_string(command, "tabId"),
            service_name: optional_string(command, "serviceName"),
            agent_name: optional_string(command, "agentName"),
            task_name: optional_string(command, "taskName"),
            action: optional_string(command, "action").unwrap_or_else(|| "unknown".to_string()),
            policy_revision: command.get("policyRevision").and_then(Value::as_u64),
            access_decision_id: optional_string(command, "accessDecisionId"),
        }
    }
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

fn optional_string(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(Value::as_str).and_then(nonempty)
}

fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn capture_keeps_only_contract_approved_causal_identifiers() {
        let provenance = ServiceRequestProvenance::capture(
            &json!({
                "action": "navigate",
                "requestId": "request-7",
                "clientSubjectId": "client:fieldwork",
                "identityAssurance": "self-declared",
                "connectionInstanceId": "connection-3",
                "runtimeEnvironmentId": "production",
                "profileId": "research-gov",
                "sessionName": "selector-that-will-be-consumed",
                "serviceName": "research-fieldwork",
                "url": "https://private.example/path",
                "profilePath": "/private/profile",
                "credential": "do-not-retain"
            }),
            "transport-id",
            "job-7",
            "connection-ingress-9",
            "runtime-lane-2",
        );

        assert_eq!(provenance.request_id, "request-7");
        assert_eq!(provenance.job_id, "job-7");
        assert_eq!(
            provenance.runtime_lane_id.as_deref(),
            Some("runtime-lane-2")
        );
        assert_eq!(
            provenance.connection_instance_id.as_deref(),
            Some("connection-ingress-9")
        );
        assert_eq!(
            provenance.profile_resource_key.as_deref(),
            Some("profile:research-gov")
        );
        assert_eq!(
            provenance.session_id.as_deref(),
            Some("selector-that-will-be-consumed")
        );

        let serialized = serde_json::to_value(provenance).unwrap();
        assert!(serialized.get("url").is_none());
        assert!(serialized.get("profilePath").is_none());
        assert!(serialized.get("credential").is_none());
    }

    #[test]
    fn capture_normalizes_invalid_assurance_and_blank_identifiers() {
        let provenance = ServiceRequestProvenance::capture(
            &json!({
                "action": "status",
                "identityAssurance": "unrecognized-proof",
                "clientSubjectId": "   "
            }),
            "request-1",
            "job-1",
            "connection-1",
            "lane-1",
        );

        assert_eq!(provenance.identity_assurance, "unknown");
        assert_eq!(provenance.client_subject_id, None);
    }

    #[test]
    fn labeled_local_requests_derive_one_stable_self_declared_subject() {
        assert_eq!(
            stable_self_declared_subject(
                Some("research-fieldwork"),
                Some("codex"),
                Some("collect-evidence")
            )
            .as_deref(),
            Some("service:research-fieldwork/agent:codex/task:collect-evidence")
        );
    }
}
