//! Passive profile-lease records shared by service adapters.

use crate::PrincipalContinuityRecourse;
use crate::ServicePrincipalProvenance;
use serde::{Deserialize, Serialize};

pub const PROFILE_LEASE_SCHEMA_VERSION: &str = "agent-browser.profile-lease.v1";
pub const PROFILE_LEASE_RECONCILE_PLAN_SCHEMA_VERSION: &str =
    "agent-browser.profile-lease-reconcile-plan.v1";
pub const PROFILE_LEASE_RECONCILE_RECEIPT_SCHEMA_VERSION: &str =
    "agent-browser.profile-lease-reconcile-receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileLeaseRecord {
    pub schema_version: String,
    pub id: String,
    pub lease_revision: String,
    pub principal_id: Option<String>,
    pub principal_provenance: Option<ServicePrincipalProvenance>,
    pub profile_id: String,
    pub profile_identity_digest: Option<String>,
    pub browser_id: Option<String>,
    pub session_ids: Vec<String>,
    pub tab_ids: Vec<String>,
    pub mode: String,
    pub state: String,
    pub owner_generation: Option<u64>,
    pub process_instance_digest: Option<String>,
    pub route_ids: Vec<String>,
    pub last_heartbeat_at: Option<String>,
    pub expires_at: Option<String>,
    pub cleanup_obligation: Option<String>,
    pub blocking_identity_axes: Vec<String>,
    pub authorized_actions: Vec<String>,
    pub recourse: PrincipalContinuityRecourse,
    pub observation_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileLeaseFinding {
    pub code: String,
    pub severity: String,
    pub lease_id: String,
    pub profile_id: String,
    pub message: String,
    pub safe_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileLeaseDoctorReport {
    pub schema_version: String,
    pub observed_at: String,
    pub healthy: bool,
    pub lease_count: usize,
    pub findings: Vec<ProfileLeaseFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileLeaseTransition {
    pub action: String,
    pub session_id: String,
    pub from_state: String,
    pub to_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileLeaseReconcilePlan {
    pub schema_version: String,
    pub plan_id: String,
    pub lease_id: String,
    pub lease_revision: String,
    pub owner_generation: Option<u64>,
    pub principal_id: String,
    pub profile_id: String,
    pub browser_id: Option<String>,
    pub process_instance_digest: Option<String>,
    pub route_ids: Vec<String>,
    pub boot_epoch: Option<String>,
    pub proposed_transitions: Vec<ProfileLeaseTransition>,
    pub idempotency_key: String,
    pub issued_at: String,
    pub expires_at: String,
    pub effect_capable: bool,
    pub blocked_reasons: Vec<String>,
    pub seal: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileLeaseReconcileReceipt {
    pub schema_version: String,
    pub idempotency_key: String,
    pub plan_id: String,
    pub lease_id: String,
    pub principal_id: String,
    pub applied_at: String,
    pub replayed: bool,
    pub transition_count: usize,
    pub resulting_lease_revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{de::DeserializeOwned, Serialize};
    use serde_json::{json, Value};

    fn record() -> ProfileLeaseRecord {
        ProfileLeaseRecord {
            schema_version: PROFILE_LEASE_SCHEMA_VERSION.to_string(),
            id: "profile-lease-1".to_string(),
            lease_revision: "lease-revision-1".to_string(),
            principal_id: Some("principal-1".to_string()),
            principal_provenance: Some(ServicePrincipalProvenance::RegisteredCapability),
            profile_id: "profile-1".to_string(),
            profile_identity_digest: Some("profile-digest-1".to_string()),
            browser_id: Some("browser-1".to_string()),
            session_ids: vec!["session-1".to_string()],
            tab_ids: vec!["tab-1".to_string()],
            mode: "exclusive".to_string(),
            state: "active".to_string(),
            owner_generation: Some(7),
            process_instance_digest: Some("process-digest-1".to_string()),
            route_ids: vec!["route-1".to_string()],
            last_heartbeat_at: Some("2026-09-16T12:00:00Z".to_string()),
            expires_at: Some("2026-09-16T12:05:00Z".to_string()),
            cleanup_obligation: Some("close_browser".to_string()),
            blocking_identity_axes: vec!["boot_epoch".to_string()],
            authorized_actions: vec!["inspect".to_string()],
            recourse: PrincipalContinuityRecourse::RejoinOwnedBrowser,
            observation_only: false,
        }
    }

    fn finding() -> ProfileLeaseFinding {
        ProfileLeaseFinding {
            code: "lease_expired".to_string(),
            severity: "warning".to_string(),
            lease_id: "profile-lease-1".to_string(),
            profile_id: "profile-1".to_string(),
            message: "lease requires reconciliation".to_string(),
            safe_actions: vec!["inspect".to_string()],
        }
    }

    fn doctor_report() -> ProfileLeaseDoctorReport {
        ProfileLeaseDoctorReport {
            schema_version: PROFILE_LEASE_SCHEMA_VERSION.to_string(),
            observed_at: "2026-09-16T12:00:00Z".to_string(),
            healthy: false,
            lease_count: 1,
            findings: vec![finding()],
        }
    }

    fn transition() -> ProfileLeaseTransition {
        ProfileLeaseTransition {
            action: "release".to_string(),
            session_id: "session-1".to_string(),
            from_state: "active".to_string(),
            to_state: "released".to_string(),
        }
    }

    fn reconcile_plan() -> ProfileLeaseReconcilePlan {
        ProfileLeaseReconcilePlan {
            schema_version: PROFILE_LEASE_RECONCILE_PLAN_SCHEMA_VERSION.to_string(),
            plan_id: "plan-1".to_string(),
            lease_id: "profile-lease-1".to_string(),
            lease_revision: "lease-revision-1".to_string(),
            owner_generation: Some(7),
            principal_id: "principal-1".to_string(),
            profile_id: "profile-1".to_string(),
            browser_id: Some("browser-1".to_string()),
            process_instance_digest: Some("process-digest-1".to_string()),
            route_ids: vec!["route-1".to_string()],
            boot_epoch: Some("boot-1".to_string()),
            proposed_transitions: vec![transition()],
            idempotency_key: "idempotency-1".to_string(),
            issued_at: "2026-09-16T12:00:00Z".to_string(),
            expires_at: "2026-09-16T12:05:00Z".to_string(),
            effect_capable: true,
            blocked_reasons: vec![],
            seal: "seal-1".to_string(),
        }
    }

    fn reconcile_receipt() -> ProfileLeaseReconcileReceipt {
        ProfileLeaseReconcileReceipt {
            schema_version: PROFILE_LEASE_RECONCILE_RECEIPT_SCHEMA_VERSION.to_string(),
            idempotency_key: "idempotency-1".to_string(),
            plan_id: "plan-1".to_string(),
            lease_id: "profile-lease-1".to_string(),
            principal_id: "principal-1".to_string(),
            applied_at: "2026-09-16T12:01:00Z".to_string(),
            replayed: false,
            transition_count: 1,
            resulting_lease_revision: "lease-revision-2".to_string(),
        }
    }

    fn assert_round_trip<T>(value: T)
    where
        T: std::fmt::Debug + PartialEq + Serialize + DeserializeOwned,
    {
        let wire = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<T>(&wire).unwrap(), value);
    }

    #[test]
    fn full_wire_records_round_trip() {
        assert_round_trip(record());
        assert_round_trip(finding());
        assert_round_trip(doctor_report());
        assert_round_trip(transition());
        assert_round_trip(reconcile_plan());
        assert_round_trip(reconcile_receipt());
    }

    #[test]
    fn records_reject_unknown_fields() {
        let mut wire = serde_json::to_value(record()).unwrap();
        wire.as_object_mut()
            .unwrap()
            .insert("unexpectedField".to_string(), json!(true));

        assert!(serde_json::from_value::<ProfileLeaseRecord>(wire).is_err());
    }

    #[test]
    fn record_uses_camel_case_fields() {
        let wire = serde_json::to_value(record()).unwrap();
        let object = wire.as_object().unwrap();

        for field in [
            "schemaVersion",
            "leaseRevision",
            "principalId",
            "principalProvenance",
            "profileIdentityDigest",
            "ownerGeneration",
            "processInstanceDigest",
            "lastHeartbeatAt",
            "cleanupObligation",
            "blockingIdentityAxes",
            "authorizedActions",
            "observationOnly",
        ] {
            assert!(
                object.contains_key(field),
                "missing camelCase field {field}"
            );
        }
        assert!(!object.contains_key("schema_version"));
    }

    #[test]
    fn findings_and_transitions_are_nested_with_camel_case_fields() {
        let doctor_wire = serde_json::to_value(doctor_report()).unwrap();
        assert_eq!(doctor_wire["findings"][0]["leaseId"], "profile-lease-1");
        assert_eq!(
            doctor_wire["findings"][0]["safeActions"],
            json!(["inspect"])
        );

        let plan_wire = serde_json::to_value(reconcile_plan()).unwrap();
        assert_eq!(
            plan_wire["proposedTransitions"][0],
            json!({
                "action": "release",
                "sessionId": "session-1",
                "fromState": "active",
                "toState": "released",
            })
        );
    }

    #[test]
    fn canonical_provenance_and_recourse_use_their_wire_values() {
        let wire = serde_json::to_value(record()).unwrap();

        assert_eq!(
            wire["principalProvenance"],
            Value::String("registered_capability".to_string())
        );
        assert_eq!(
            wire["recourse"],
            Value::String("rejoin_owned_browser".to_string())
        );
    }
}
