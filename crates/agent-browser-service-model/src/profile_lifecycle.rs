//! Provider-free profile lifecycle authorization and durable effect records.
//!
//! This module owns deterministic authorization registration and wire models.
//! Adapters own physical observations, proof construction, settlement, clocks,
//! persistence, runtime commands, and effects.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::profile_access::{
    ProfileEvictionMode, ProfileEvictionPlan, ProfileIdentityAssurance, ProfilePermission,
};

pub const PROFILE_LIFECYCLE_AUTHORIZATION_SCHEMA_V1: &str =
    "agent-browser.profile-lifecycle-authorization.v1";
pub const PROFILE_LIFECYCLE_PROOF_SCHEMA_V1: &str = "agent-browser.profile-lifecycle-proof.v1";
pub const PROFILE_LIFECYCLE_RECEIPT_SCHEMA_V1: &str = "agent-browser.profile-lifecycle-receipt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileLifecycleAuthorizationState {
    Authorized,
    Completed,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileLifecycleAuthorization {
    pub schema_version: String,
    pub authorization_id: String,
    pub profile_id: String,
    pub policy_revision: u64,
    pub subject_id: Option<String>,
    pub assurance: ProfileIdentityAssurance,
    pub permission: ProfilePermission,
    pub eviction_mode: ProfileEvictionMode,
    pub grace_deadline: Option<String>,
    pub target_resource_ids: Vec<String>,
    pub issued_at: String,
    pub state: ProfileLifecycleAuthorizationState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileLifecycleProof {
    pub schema_version: String,
    pub proof_id: String,
    pub authorization_id: String,
    pub profile_id: String,
    pub policy_revision: u64,
    pub tab_id: String,
    pub browser_id: String,
    pub target_id: String,
    pub daemon_session_id: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileLifecycleEffectReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub authorization_id: String,
    pub proof_id: String,
    pub profile_id: String,
    pub policy_revision: u64,
    pub tab_id: String,
    pub browser_id: String,
    pub target_id: String,
    pub cancelled_job_ids: Vec<String>,
    pub released_session_id: Option<String>,
    pub terminated_viewer_lease_ids: Vec<String>,
    pub outcome: String,
    pub completed_at: String,
}

pub fn register_profile_eviction_authorization(
    authorizations: &mut BTreeMap<String, ProfileLifecycleAuthorization>,
    plan: &ProfileEvictionPlan,
    assurance: ProfileIdentityAssurance,
    issued_at: &str,
) -> Result<ProfileLifecycleAuthorization, String> {
    if plan.plan_id.trim().is_empty()
        || plan.profile_id.trim().is_empty()
        || plan.target_resource_ids.is_empty()
        || !assurance.satisfies(ProfileIdentityAssurance::RegisteredCapability)
    {
        return Err("profile_lifecycle_authorization_invalid".to_string());
    }
    let mut target_resource_ids = plan.target_resource_ids.clone();
    target_resource_ids.sort();
    target_resource_ids.dedup();
    if target_resource_ids != plan.target_resource_ids {
        return Err("profile_lifecycle_authorization_targets_noncanonical".to_string());
    }
    let authorization = ProfileLifecycleAuthorization {
        schema_version: PROFILE_LIFECYCLE_AUTHORIZATION_SCHEMA_V1.to_string(),
        authorization_id: plan.plan_id.clone(),
        profile_id: plan.profile_id.clone(),
        policy_revision: plan.policy_revision,
        subject_id: plan.requested_by.clone(),
        assurance,
        permission: ProfilePermission::Evict,
        eviction_mode: plan.mode,
        grace_deadline: plan.grace_deadline.clone(),
        target_resource_ids,
        issued_at: issued_at.to_string(),
        state: ProfileLifecycleAuthorizationState::Authorized,
    };
    match authorizations.get(&authorization.authorization_id) {
        Some(existing) if existing == &authorization => return Ok(existing.clone()),
        Some(_) => return Err("profile_lifecycle_authorization_conflict".to_string()),
        None => {}
    }
    authorizations.insert(
        authorization.authorization_id.clone(),
        authorization.clone(),
    );
    Ok(authorization)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;
    use serde_json::{json, Value};

    fn plan() -> ProfileEvictionPlan {
        ProfileEvictionPlan {
            plan_id: "profile-eviction-plan:test".to_string(),
            profile_id: "research-gov".to_string(),
            policy_revision: 7,
            requested_by: Some("principal:admin".to_string()),
            mode: ProfileEvictionMode::ForceImmediate,
            grace_deadline: None,
            target_resource_ids: vec!["tab:fieldwork".to_string(), "tab:review".to_string()],
        }
    }

    fn authorization() -> ProfileLifecycleAuthorization {
        let mut authorizations = BTreeMap::new();
        register_profile_eviction_authorization(
            &mut authorizations,
            &plan(),
            ProfileIdentityAssurance::RegisteredCapability,
            "2026-09-02T20:00:00Z",
        )
        .unwrap()
    }

    fn proof() -> ProfileLifecycleProof {
        ProfileLifecycleProof {
            schema_version: PROFILE_LIFECYCLE_PROOF_SCHEMA_V1.to_string(),
            proof_id: "profile-lifecycle-proof:test".to_string(),
            authorization_id: "profile-eviction-plan:test".to_string(),
            profile_id: "research-gov".to_string(),
            policy_revision: 7,
            tab_id: "tab:fieldwork".to_string(),
            browser_id: "browser:research".to_string(),
            target_id: "target:research".to_string(),
            daemon_session_id: "research-runtime".to_string(),
            observed_at: "2026-09-02T20:00:01Z".to_string(),
        }
    }

    fn receipt() -> ProfileLifecycleEffectReceipt {
        ProfileLifecycleEffectReceipt {
            schema_version: PROFILE_LIFECYCLE_RECEIPT_SCHEMA_V1.to_string(),
            receipt_id: "profile-lifecycle-receipt:test".to_string(),
            authorization_id: "profile-eviction-plan:test".to_string(),
            proof_id: "profile-lifecycle-proof:test".to_string(),
            profile_id: "research-gov".to_string(),
            policy_revision: 7,
            tab_id: "tab:fieldwork".to_string(),
            browser_id: "browser:research".to_string(),
            target_id: "target:research".to_string(),
            cancelled_job_ids: vec!["job:one".to_string()],
            released_session_id: Some("research-runtime".to_string()),
            terminated_viewer_lease_ids: vec!["viewer:one".to_string()],
            outcome: "forced_eviction_completed".to_string(),
            completed_at: "2026-09-02T20:00:02Z".to_string(),
        }
    }

    fn assert_wire_round_trip<T>(record: &T)
    where
        T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let wire = serde_json::to_string(record).unwrap();
        assert_eq!(serde_json::from_str::<T>(&wire).unwrap(), *record);
    }

    fn assert_unknown_field_rejected<T>(record: &T)
    where
        T: Serialize + DeserializeOwned + std::fmt::Debug,
    {
        let mut wire = serde_json::to_value(record).unwrap();
        wire.as_object_mut()
            .unwrap()
            .insert("unexpectedField".to_string(), Value::Bool(true));
        let error = serde_json::from_value::<T>(wire).unwrap_err();
        assert!(error
            .to_string()
            .contains("unknown field `unexpectedField`"));
    }

    #[test]
    fn lifecycle_records_round_trip_on_the_wire() {
        let authorization = authorization();
        let proof = proof();
        let receipt = receipt();

        assert_wire_round_trip(&authorization);
        assert_wire_round_trip(&proof);
        assert_wire_round_trip(&receipt);

        let wire = serde_json::to_value(authorization).unwrap();
        assert_eq!(
            wire["schemaVersion"],
            PROFILE_LIFECYCLE_AUTHORIZATION_SCHEMA_V1
        );
        assert_eq!(wire["authorizationId"], "profile-eviction-plan:test");
        assert_eq!(wire["state"], "authorized");
    }

    #[test]
    fn lifecycle_records_reject_unknown_wire_fields() {
        assert_unknown_field_rejected(&authorization());
        assert_unknown_field_rejected(&proof());
        assert_unknown_field_rejected(&receipt());
    }

    #[test]
    fn authorization_states_use_stable_snake_case_wire_values() {
        let cases = [
            (ProfileLifecycleAuthorizationState::Authorized, "authorized"),
            (ProfileLifecycleAuthorizationState::Completed, "completed"),
            (ProfileLifecycleAuthorizationState::Incomplete, "incomplete"),
        ];

        for (state, wire) in cases {
            assert_eq!(serde_json::to_value(state).unwrap(), json!(wire));
            assert_eq!(
                serde_json::from_value::<ProfileLifecycleAuthorizationState>(json!(wire)).unwrap(),
                state
            );
        }
    }

    #[test]
    fn registration_rejects_invalid_inputs() {
        let cases = [
            ProfileEvictionPlan {
                plan_id: " ".to_string(),
                ..plan()
            },
            ProfileEvictionPlan {
                profile_id: " ".to_string(),
                ..plan()
            },
            ProfileEvictionPlan {
                target_resource_ids: Vec::new(),
                ..plan()
            },
        ];

        for invalid in cases {
            let error = register_profile_eviction_authorization(
                &mut BTreeMap::new(),
                &invalid,
                ProfileIdentityAssurance::RegisteredCapability,
                "2026-09-02T20:00:00Z",
            )
            .unwrap_err();
            assert_eq!(error, "profile_lifecycle_authorization_invalid");
        }

        let error = register_profile_eviction_authorization(
            &mut BTreeMap::new(),
            &plan(),
            ProfileIdentityAssurance::AuthenticatedIngress,
            "2026-09-02T20:00:00Z",
        )
        .unwrap_err();
        assert_eq!(error, "profile_lifecycle_authorization_invalid");
    }

    #[test]
    fn registration_rejects_noncanonical_targets() {
        for target_resource_ids in [
            vec!["tab:review".to_string(), "tab:fieldwork".to_string()],
            vec!["tab:fieldwork".to_string(), "tab:fieldwork".to_string()],
        ] {
            let error = register_profile_eviction_authorization(
                &mut BTreeMap::new(),
                &ProfileEvictionPlan {
                    target_resource_ids,
                    ..plan()
                },
                ProfileIdentityAssurance::RegisteredCapability,
                "2026-09-02T20:00:00Z",
            )
            .unwrap_err();
            assert_eq!(
                error,
                "profile_lifecycle_authorization_targets_noncanonical"
            );
        }
    }

    #[test]
    fn repeated_identical_registration_is_idempotent() {
        let mut authorizations = BTreeMap::new();
        let first = register_profile_eviction_authorization(
            &mut authorizations,
            &plan(),
            ProfileIdentityAssurance::RegisteredCapability,
            "2026-09-02T20:00:00Z",
        )
        .unwrap();
        let second = register_profile_eviction_authorization(
            &mut authorizations,
            &plan(),
            ProfileIdentityAssurance::RegisteredCapability,
            "2026-09-02T20:00:00Z",
        )
        .unwrap();

        assert_eq!(second, first);
        assert_eq!(authorizations.len(), 1);
    }

    #[test]
    fn reused_authorization_id_with_different_content_conflicts() {
        let mut authorizations = BTreeMap::new();
        register_profile_eviction_authorization(
            &mut authorizations,
            &plan(),
            ProfileIdentityAssurance::RegisteredCapability,
            "2026-09-02T20:00:00Z",
        )
        .unwrap();

        let error = register_profile_eviction_authorization(
            &mut authorizations,
            &ProfileEvictionPlan {
                policy_revision: 8,
                ..plan()
            },
            ProfileIdentityAssurance::RegisteredCapability,
            "2026-09-02T20:00:00Z",
        )
        .unwrap_err();

        assert_eq!(error, "profile_lifecycle_authorization_conflict");
        assert_eq!(authorizations.len(), 1);
        assert_eq!(authorizations.values().next().unwrap().policy_revision, 7);
    }
}
