//! Transitional CLI import facade for the provider-free profile-access model.
//!
//! Persistence, runtime observation, and transport adapters remain in the CLI.
//! The canonical policy records and pure authorization transitions live in
//! `agent-browser-service-model`.

pub use agent_browser_service_model::{
    effective_profile_permissions, evaluate_profile_access, evaluate_profile_child_access,
    mutate_profile_policy, profile_policy_target_for_preset, ProfileAccessEvaluation,
    ProfileAccessGrant, ProfileAccessMode, ProfileAccessPolicyState, ProfileAccessPreset,
    ProfileChildAccess, ProfileChildAccessEvidence, ProfileChildAccessRequest,
    ProfileConnectionState, ProfileEvictionMode, ProfileEvictionPlan, ProfileIdentityAssurance,
    ProfilePermission, ProfilePolicyMutationRequest, ProfilePolicyMutationResult,
    ProfilePolicyTarget, ServiceProfileAccessDecision, ServiceProfileAccessPolicy,
};
#[cfg(test)]
pub use agent_browser_service_model::{ProfileAccessDrain, PROFILE_CHILD_ACCESS_SCHEMA_V1};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_failure::{
        child_access_failure_evidence, classify_service_failure, profile_child_denial_error,
        ServiceEffectState,
    };

    #[test]
    fn child_denial_evidence_remains_privacy_bounded_in_cli_failure_projection() {
        let policy = ServiceProfileAccessPolicy::shared_local_default("research-gov");
        let child = ProfileChildAccess {
            parent_policy_revision: policy.revision,
            subject_id: Some("client:fieldwork".to_string()),
            identity_assurance: ProfileIdentityAssurance::SelfDeclared,
            connection_instance_id: Some("connection:old".to_string()),
            connection_state: ProfileConnectionState::Disconnected,
            permissions: vec![ProfilePermission::TabControlOwn],
            ..ProfileChildAccess::default()
        };
        let denied = evaluate_profile_child_access(ProfileChildAccessRequest {
            child: &child,
            current_policy: &policy,
            subject_id: Some("client:other"),
            assurance: ProfileIdentityAssurance::SelfDeclared,
            connection_instance_id: "connection:new",
            permission: ProfilePermission::TabControlOwn,
            reconnect: true,
        });

        assert_eq!(denied.reason, "subject_mismatch");
        let evidence = denied.denial_evidence.as_ref().unwrap();
        let encoded = profile_child_denial_error(denied.reason, Some(evidence));
        assert!(!encoded.contains("client:other"));
        assert!(!encoded.contains("connection:new"));

        let failure = classify_service_failure(&encoded);
        assert_eq!(failure.code, "profile_child_subject_mismatch");
        assert_eq!(failure.effect_state, ServiceEffectState::NoEffect);
        let projected = child_access_failure_evidence(&failure).unwrap();
        assert_eq!(projected, serde_json::to_value(evidence).unwrap());
        assert_eq!(
            projected["sourceFunction"],
            "native/service_profile_access_policy.rs::evaluate_profile_child_access"
        );

        let mut invalid = projected;
        invalid["expectedSubjectHash"] = serde_json::json!("private raw identity");
        let malformed =
            format!("profile child access denied: subject_mismatch; childAccessEvidence={invalid}");
        assert_eq!(
            classify_service_failure(&malformed).effect_state,
            ServiceEffectState::EffectUncertain
        );
    }
}
