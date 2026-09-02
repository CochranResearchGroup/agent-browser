//! Revisioned profile access policy and deterministic authorization decisions.
//!
//! Access policy answers whether a subject may use a profile. Coordination
//! leases and exact runtime ownership remain separate downstream concerns.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const PROFILE_ACCESS_POLICY_SCHEMA_V1: &str = "agent-browser.profile-access-policy.v1";
pub const PROFILE_ACCESS_DECISION_SCHEMA_V1: &str = "agent-browser.profile-access-decision.v1";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProfileAccessMode {
    #[default]
    SharedLocal,
    Restricted,
    Exclusive,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProfileIdentityAssurance {
    SelfDeclared,
    AuthenticatedIngress,
    RegisteredCapability,
    Operator,
    #[default]
    Unknown,
}

impl ProfileIdentityAssurance {
    fn rank(self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::SelfDeclared => 1,
            Self::AuthenticatedIngress => 2,
            Self::RegisteredCapability => 3,
            Self::Operator => 4,
        }
    }

    pub(crate) fn satisfies(self, minimum: Self) -> bool {
        self.rank() >= minimum.rank()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfilePermission {
    ProfileUse,
    PolicyRead,
    PolicyWrite,
    TabCreate,
    TabObserve,
    TabControlOwn,
    TabCloseOwn,
    TabControlAny,
    TabCloseAny,
    ViewOpen,
    ViewControl,
    Drain,
    Evict,
    LifecycleManage,
    FullShutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileAccessGrant {
    pub subject_id: String,
    pub minimum_assurance: ProfileIdentityAssurance,
    pub permissions: Vec<ProfilePermission>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileAccessPolicyState {
    #[default]
    Active,
    Draining,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceProfileAccessPolicy {
    pub schema_version: String,
    pub profile_id: String,
    pub mode: ProfileAccessMode,
    pub revision: u64,
    pub state: ProfileAccessPolicyState,
    pub default_permissions: Vec<ProfilePermission>,
    pub grants: Vec<ProfileAccessGrant>,
    pub drain: Option<Value>,
    pub updated_at: String,
}

impl ServiceProfileAccessPolicy {
    pub(crate) fn shared_local_default(profile_id: &str) -> Self {
        Self {
            schema_version: PROFILE_ACCESS_POLICY_SCHEMA_V1.to_string(),
            profile_id: profile_id.to_string(),
            mode: ProfileAccessMode::SharedLocal,
            revision: 1,
            state: ProfileAccessPolicyState::Active,
            default_permissions: vec![
                ProfilePermission::ProfileUse,
                ProfilePermission::PolicyRead,
                ProfilePermission::TabCreate,
                ProfilePermission::TabObserve,
                ProfilePermission::TabControlOwn,
                ProfilePermission::TabCloseOwn,
                ProfilePermission::ViewOpen,
            ],
            grants: Vec::new(),
            drain: None,
            updated_at: "1970-01-01T00:00:00Z".to_string(),
        }
    }
}

impl Default for ServiceProfileAccessPolicy {
    fn default() -> Self {
        Self::shared_local_default("")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileAccessSubject {
    pub subject_id: Option<String>,
    pub assurance: ProfileIdentityAssurance,
    pub connection_instance_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileAccessResource {
    pub profile_id: Option<String>,
    pub resource_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileAccessNextAction {
    pub action: String,
    pub executable: bool,
    pub request: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceProfileAccessDecision {
    pub schema_version: String,
    pub decision_id: String,
    pub subject: ProfileAccessSubject,
    pub resource: ProfileAccessResource,
    pub operation: String,
    pub policy_revision: u64,
    pub allowed: bool,
    pub missing_permission: Option<String>,
    pub blocking_occupancy: Vec<String>,
    pub next_action: ProfileAccessNextAction,
}

pub(crate) struct ProfileAccessEvaluation<'a> {
    pub(crate) profile_id: &'a str,
    pub(crate) explicit_policy: Option<&'a ServiceProfileAccessPolicy>,
    pub(crate) subject_id: Option<String>,
    pub(crate) assurance: ProfileIdentityAssurance,
    pub(crate) connection_instance_id: Option<String>,
    pub(crate) permission: ProfilePermission,
    pub(crate) operation: &'a str,
    pub(crate) incompatible_occupancy: Vec<String>,
}

pub(crate) fn evaluate_profile_access(
    input: ProfileAccessEvaluation<'_>,
) -> (ServiceProfileAccessPolicy, ServiceProfileAccessDecision) {
    let policy = input
        .explicit_policy
        .cloned()
        .unwrap_or_else(|| ServiceProfileAccessPolicy::shared_local_default(input.profile_id));
    let subject_grant = input.subject_id.as_deref().and_then(|subject_id| {
        policy.grants.iter().find(|grant| {
            grant.subject_id == subject_id
                && input.assurance.satisfies(grant.minimum_assurance)
                && grant.permissions.contains(&input.permission)
        })
    });
    let mode_assurance_satisfied = match policy.mode {
        ProfileAccessMode::SharedLocal => true,
        ProfileAccessMode::Restricted => input
            .assurance
            .satisfies(ProfileIdentityAssurance::AuthenticatedIngress),
        ProfileAccessMode::Exclusive => input
            .assurance
            .satisfies(ProfileIdentityAssurance::RegisteredCapability),
    };
    let has_permission = mode_assurance_satisfied
        && (policy.default_permissions.contains(&input.permission) || subject_grant.is_some());
    let profile_matches = policy.profile_id == input.profile_id;
    let draining = policy.state == ProfileAccessPolicyState::Draining;
    let exclusive_occupancy =
        policy.mode == ProfileAccessMode::Exclusive && !input.incompatible_occupancy.is_empty();
    let allowed = profile_matches && !draining && has_permission && !exclusive_occupancy;
    let missing_permission = (!has_permission).then(|| {
        serde_json::to_value(input.permission)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    });
    let next_action = if allowed {
        ProfileAccessNextAction {
            action: "proceed".to_string(),
            executable: true,
            request: None,
        }
    } else if draining || exclusive_occupancy {
        ProfileAccessNextAction {
            action: "inspect_profile_occupancy".to_string(),
            executable: true,
            request: Some(json!({
                "action": "service_profile_allocation",
                "profileId": input.profile_id,
            })),
        }
    } else {
        ProfileAccessNextAction {
            action: "inspect_profile_access_policy".to_string(),
            executable: true,
            request: Some(json!({
                "action": "service_access_plan",
                "profileId": input.profile_id,
                "clientSubjectId": input.subject_id,
            })),
        }
    };
    let decision_id = stable_decision_id(
        input.profile_id,
        policy.revision,
        input.subject_id.as_deref(),
        input.assurance,
        input.operation,
    );
    let policy_revision = policy.revision;
    (
        policy,
        ServiceProfileAccessDecision {
            schema_version: PROFILE_ACCESS_DECISION_SCHEMA_V1.to_string(),
            decision_id,
            subject: ProfileAccessSubject {
                subject_id: input.subject_id,
                assurance: input.assurance,
                connection_instance_id: input.connection_instance_id,
            },
            resource: ProfileAccessResource {
                profile_id: Some(input.profile_id.to_string()),
                resource_key: format!("profile:{}", input.profile_id),
            },
            operation: input.operation.to_string(),
            policy_revision,
            allowed,
            missing_permission,
            blocking_occupancy: input.incompatible_occupancy,
            next_action,
        },
    )
}

fn stable_decision_id(
    profile_id: &str,
    policy_revision: u64,
    subject_id: Option<&str>,
    assurance: ProfileIdentityAssurance,
    operation: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(profile_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(policy_revision.to_le_bytes());
    hasher.update(b"\0");
    hasher.update(subject_id.unwrap_or("unknown").as_bytes());
    hasher.update(b"\0");
    hasher.update(serde_json::to_vec(&assurance).unwrap());
    hasher.update(b"\0");
    hasher.update(operation.as_bytes());
    let suffix = hasher
        .finalize()
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("profile-access-decision:{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evaluate(
        policy: Option<&ServiceProfileAccessPolicy>,
        assurance: ProfileIdentityAssurance,
        occupancy: Vec<String>,
    ) -> (ServiceProfileAccessPolicy, ServiceProfileAccessDecision) {
        evaluate_profile_access(ProfileAccessEvaluation {
            profile_id: "research-gov",
            explicit_policy: policy,
            subject_id: Some("client:fieldwork".to_string()),
            assurance,
            connection_instance_id: Some("connection:7".to_string()),
            permission: ProfilePermission::TabCreate,
            operation: "tab_create",
            incompatible_occupancy: occupancy,
        })
    }

    fn strict_policy(mode: ProfileAccessMode) -> ServiceProfileAccessPolicy {
        ServiceProfileAccessPolicy {
            profile_id: "research-gov".to_string(),
            mode,
            revision: 7,
            default_permissions: Vec::new(),
            grants: vec![ProfileAccessGrant {
                subject_id: "client:fieldwork".to_string(),
                minimum_assurance: ProfileIdentityAssurance::AuthenticatedIngress,
                permissions: vec![ProfilePermission::TabCreate],
            }],
            ..ServiceProfileAccessPolicy::default()
        }
    }

    #[test]
    fn shared_local_default_admits_stable_self_declared_subject_deterministically() {
        let (policy, first) = evaluate(None, ProfileIdentityAssurance::SelfDeclared, Vec::new());
        let (_, second) = evaluate(None, ProfileIdentityAssurance::SelfDeclared, Vec::new());

        assert_eq!(policy.mode, ProfileAccessMode::SharedLocal);
        assert_eq!(policy.revision, 1);
        assert!(first.allowed);
        assert_eq!(
            first.subject.assurance,
            ProfileIdentityAssurance::SelfDeclared
        );
        assert_eq!(first.decision_id, second.decision_id);
        assert_eq!(first.next_action.action, "proceed");
    }

    #[test]
    fn restricted_requires_the_granted_subject_and_minimum_assurance() {
        let policy = strict_policy(ProfileAccessMode::Restricted);
        let (_, denied) = evaluate(
            Some(&policy),
            ProfileIdentityAssurance::SelfDeclared,
            Vec::new(),
        );
        let (_, allowed) = evaluate(
            Some(&policy),
            ProfileIdentityAssurance::AuthenticatedIngress,
            Vec::new(),
        );

        assert!(!denied.allowed);
        assert_eq!(denied.missing_permission.as_deref(), Some("tab_create"));
        assert_eq!(denied.next_action.action, "inspect_profile_access_policy");
        assert!(denied.next_action.executable);
        assert!(allowed.allowed);
    }

    #[test]
    fn exclusive_requires_permission_and_zero_incompatible_occupancy() {
        let policy = strict_policy(ProfileAccessMode::Exclusive);
        let (_, denied) = evaluate(
            Some(&policy),
            ProfileIdentityAssurance::Operator,
            vec!["session:other".to_string()],
        );
        let (_, allowed) = evaluate(
            Some(&policy),
            ProfileIdentityAssurance::Operator,
            Vec::new(),
        );

        assert!(!denied.allowed);
        assert_eq!(denied.blocking_occupancy, vec!["session:other"]);
        assert_eq!(denied.next_action.action, "inspect_profile_occupancy");
        assert!(allowed.allowed);
    }
}
