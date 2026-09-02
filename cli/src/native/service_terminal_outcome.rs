use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::service_failure::{ServiceEffectState, ServiceFailureRecourse, ServiceRetryDisposition};
use super::service_request_provenance::ServiceRequestProvenance;

pub const SERVICE_TERMINAL_OUTCOME_SCHEMA_VERSION: &str =
    "agent-browser.service-terminal-outcome.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTerminalState {
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTerminalPhase {
    Ingress,
    QueueAdmission,
    SchedulerAdmission,
    Dispatch,
    Execution,
    Commit,
    Finalize,
}

/// Canonical terminal result shared by the response, job, event, and trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceTerminalOutcome {
    pub schema_version: String,
    pub state: ServiceTerminalState,
    pub phase: ServiceTerminalPhase,
    pub effect_state: ServiceEffectState,
    pub retry_disposition: ServiceRetryDisposition,
    pub failure: Option<ServiceFailureRecourse>,
    pub provenance: ServiceRequestProvenance,
    pub completed_at: String,
}

impl ServiceTerminalOutcome {
    pub fn from_response(
        provenance: &ServiceRequestProvenance,
        response: &Value,
        state: ServiceTerminalState,
        phase: ServiceTerminalPhase,
        completed_at: String,
    ) -> Self {
        let failure = if state == ServiceTerminalState::Succeeded {
            None
        } else {
            response
                .get("failure")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok())
        };
        let (effect_state, retry_disposition) = failure
            .as_ref()
            .map(|failure: &ServiceFailureRecourse| {
                (failure.effect_state, failure.retry_disposition)
            })
            .unwrap_or((
                ServiceEffectState::VerifiedEffect,
                ServiceRetryDisposition::DoNotRetry,
            ));
        Self {
            schema_version: SERVICE_TERMINAL_OUTCOME_SCHEMA_VERSION.to_string(),
            state,
            phase,
            effect_state,
            retry_disposition,
            failure,
            provenance: provenance.clone(),
            completed_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn outcome_copies_one_failure_and_provenance_without_private_expansion() {
        let provenance = ServiceRequestProvenance::capture(
            &json!({ "action": "open", "clientSubjectId": "client:test" }),
            "request-1",
            "job-1",
            "connection-1",
            "lane-1",
        );
        let response = json!({
            "success": false,
            "failure": {
                "schemaVersion": "agent-browser.service-failure-recourse.v1",
                "code": "profile_lease_conflict",
                "axis": "profile_lease",
                "phase": "launch_admission",
                "effectState": "no_effect",
                "retryDisposition": "refresh_access_plan",
                "recommendedAction": "refresh_access_plan",
                "reuseAllowed": false
            }
        });

        let outcome = ServiceTerminalOutcome::from_response(
            &provenance,
            &response,
            ServiceTerminalState::Rejected,
            ServiceTerminalPhase::SchedulerAdmission,
            "2026-09-02T12:00:00Z".to_string(),
        );

        assert_eq!(
            outcome.failure.as_ref().unwrap().code,
            "profile_lease_conflict"
        );
        assert_eq!(
            outcome.provenance.runtime_lane_id.as_deref(),
            Some("lane-1")
        );
        assert_eq!(outcome.effect_state, ServiceEffectState::NoEffect);
    }
}
