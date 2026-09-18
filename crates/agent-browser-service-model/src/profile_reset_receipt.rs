//! Provider-free profile reset receipt records.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROFILE_RESET_RECEIPT_SCHEMA_V1: &str = "agent-browser.profile-reset-receipt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileResetScope {
    Runtime,
    Authentication,
    ProfileData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileResetReceipt {
    pub schema_version: String,
    pub reset_id: String,
    pub plan_id: String,
    pub principal_id: String,
    pub profile_id: String,
    pub producer_build_identity: Value,
    pub scope: ProfileResetScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_service_id: Option<String>,
    pub terminal_result: String,
    pub applied_at: String,
    pub final_state_revision: u64,
    pub browser_cookies_erased: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seeding_handoff: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;
    use serde_json::json;

    fn receipt(scope: ProfileResetScope) -> ProfileResetReceipt {
        ProfileResetReceipt {
            schema_version: PROFILE_RESET_RECEIPT_SCHEMA_V1.to_string(),
            reset_id: "profile-reset:test".to_string(),
            plan_id: "profile-reset-plan:test".to_string(),
            principal_id: "principal:operator".to_string(),
            profile_id: "profile:research".to_string(),
            producer_build_identity: json!({"version": "test-build"}),
            scope,
            target_service_id: Some("service:test".to_string()),
            terminal_result: "applied".to_string(),
            applied_at: "2026-09-17T12:00:00Z".to_string(),
            final_state_revision: 42,
            browser_cookies_erased: false,
            seeding_handoff: Some(json!({"handoffId": "handoff:test"})),
        }
    }

    fn assert_round_trip<T>(record: &T)
    where
        T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let wire = serde_json::to_value(record).unwrap();
        let decoded = serde_json::from_value(wire).unwrap();
        assert_eq!(*record, decoded);
    }

    #[test]
    fn receipt_round_trips_over_the_wire() {
        assert_round_trip(&receipt(ProfileResetScope::Runtime));
        assert_round_trip(&receipt(ProfileResetScope::Authentication));
        assert_round_trip(&receipt(ProfileResetScope::ProfileData));
    }

    #[test]
    fn scope_states_use_exact_snake_case_wire_values() {
        assert_eq!(
            serde_json::to_value(ProfileResetScope::Runtime).unwrap(),
            json!("runtime")
        );
        assert_eq!(
            serde_json::to_value(ProfileResetScope::Authentication).unwrap(),
            json!("authentication")
        );
        assert_eq!(
            serde_json::to_value(ProfileResetScope::ProfileData).unwrap(),
            json!("profile_data")
        );
    }

    #[test]
    fn receipt_uses_camel_case_and_omits_absent_optional_fields() {
        let mut value = serde_json::to_value(receipt(ProfileResetScope::Runtime)).unwrap();
        let object = value.as_object().unwrap();
        for field in [
            "schemaVersion",
            "resetId",
            "planId",
            "principalId",
            "profileId",
            "producerBuildIdentity",
            "scope",
            "targetServiceId",
            "terminalResult",
            "appliedAt",
            "finalStateRevision",
            "browserCookiesErased",
            "seedingHandoff",
        ] {
            assert!(
                object.contains_key(field),
                "missing camelCase field {field}"
            );
        }

        value.as_object_mut().unwrap().remove("targetServiceId");
        value.as_object_mut().unwrap().remove("seedingHandoff");
        let decoded: ProfileResetReceipt = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.target_service_id, None);
        assert_eq!(decoded.seeding_handoff, None);

        let wire = serde_json::to_value(ProfileResetReceipt {
            target_service_id: None,
            seeding_handoff: None,
            ..receipt(ProfileResetScope::Runtime)
        })
        .unwrap();
        assert!(!wire.as_object().unwrap().contains_key("targetServiceId"));
        assert!(!wire.as_object().unwrap().contains_key("seedingHandoff"));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut wire = serde_json::to_value(receipt(ProfileResetScope::Runtime)).unwrap();
        wire.as_object_mut()
            .unwrap()
            .insert("unexpectedField".to_string(), json!(true));
        assert!(serde_json::from_value::<ProfileResetReceipt>(wire).is_err());
    }

    #[test]
    fn schema_constant_matches_wire_value() {
        assert_eq!(
            PROFILE_RESET_RECEIPT_SCHEMA_V1,
            "agent-browser.profile-reset-receipt.v1"
        );
        assert_eq!(
            serde_json::to_value(receipt(ProfileResetScope::Runtime)).unwrap()["schemaVersion"],
            json!(PROFILE_RESET_RECEIPT_SCHEMA_V1)
        );
    }
}
