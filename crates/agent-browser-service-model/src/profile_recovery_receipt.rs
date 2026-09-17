//! Provider-free profile recovery receipt compatibility model.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROFILE_RECOVERY_RECEIPT_SCHEMA_V1: &str = "agent-browser.profile-recovery-receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileAcquisitionState {
    Acquired,
    RecoveryAvailable,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveryReceipt {
    pub schema_version: String,
    pub recovery_id: String,
    pub plan_id: String,
    pub principal_id: String,
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_build_identity: Option<Value>,
    pub terminal_result: String,
    pub precondition_comparison: String,
    pub attempted_operation_ids: Vec<String>,
    pub compensation_result: String,
    pub final_state_revision: u64,
    pub acquisition_retry_state: ProfileAcquisitionState,
    pub browser_id: String,
    pub daemon_session_route: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Map};

    fn receipt(producer_build_identity: Option<Value>) -> RecoveryReceipt {
        RecoveryReceipt {
            schema_version: PROFILE_RECOVERY_RECEIPT_SCHEMA_V1.to_string(),
            recovery_id: "recovery-1".to_string(),
            plan_id: "plan-1".to_string(),
            principal_id: "principal-1".to_string(),
            profile_id: "profile-1".to_string(),
            producer_build_identity,
            terminal_result: "applied".to_string(),
            precondition_comparison: "matched".to_string(),
            attempted_operation_ids: vec!["operation-1".to_string(), "operation-2".to_string()],
            compensation_result: "not_required".to_string(),
            final_state_revision: 42,
            acquisition_retry_state: ProfileAcquisitionState::Acquired,
            browser_id: "browser-1".to_string(),
            daemon_session_route: "route-1".to_string(),
        }
    }

    #[test]
    fn receipt_round_trips_with_all_fields() {
        let expected = receipt(Some(json!({"version": "build-1"})));
        let encoded = serde_json::to_value(&expected).expect("receipt serializes");
        let decoded: RecoveryReceipt = serde_json::from_value(encoded).expect("receipt parses");

        assert_eq!(decoded, expected);
    }

    #[test]
    fn acquisition_states_use_exact_snake_case_values() {
        assert_eq!(
            serde_json::to_string(&ProfileAcquisitionState::Acquired).unwrap(),
            "\"acquired\""
        );
        assert_eq!(
            serde_json::to_string(&ProfileAcquisitionState::RecoveryAvailable).unwrap(),
            "\"recovery_available\""
        );
        assert_eq!(
            serde_json::to_string(&ProfileAcquisitionState::Blocked).unwrap(),
            "\"blocked\""
        );
    }

    #[test]
    fn receipt_uses_camel_case_and_omits_absent_producer_identity() {
        let value = serde_json::to_value(receipt(None)).expect("receipt serializes");
        let object = value.as_object().expect("receipt is an object");

        assert!(object.contains_key("schemaVersion"));
        assert!(object.contains_key("recoveryId"));
        assert!(object.contains_key("planId"));
        assert!(object.contains_key("principalId"));
        assert!(object.contains_key("profileId"));
        assert!(object.contains_key("terminalResult"));
        assert!(object.contains_key("preconditionComparison"));
        assert!(object.contains_key("attemptedOperationIds"));
        assert!(object.contains_key("compensationResult"));
        assert!(object.contains_key("finalStateRevision"));
        assert!(object.contains_key("acquisitionRetryState"));
        assert!(object.contains_key("browserId"));
        assert!(object.contains_key("daemonSessionRoute"));
        assert!(!object.contains_key("producer_build_identity"));
        assert!(!object.contains_key("producerBuildIdentity"));

        let decoded: RecoveryReceipt =
            serde_json::from_value(value).expect("legacy receipt without producer identity parses");
        assert_eq!(decoded.producer_build_identity, None);
    }

    #[test]
    fn receipt_rejects_unknown_fields() {
        let mut object: Map<String, Value> = serde_json::to_value(receipt(None))
            .expect("receipt serializes")
            .as_object()
            .expect("receipt is an object")
            .clone();
        object.insert("unexpectedField".to_string(), json!(true));

        let error = serde_json::from_value::<RecoveryReceipt>(Value::Object(object))
            .expect_err("unknown fields must be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn schema_constant_is_exact() {
        assert_eq!(
            PROFILE_RECOVERY_RECEIPT_SCHEMA_V1,
            "agent-browser.profile-recovery-receipt.v1"
        );
    }
}
