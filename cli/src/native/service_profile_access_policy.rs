//! Revisioned profile access policy and deterministic authorization decisions.
//!
//! Access policy answers whether a subject may use a profile. Coordination
//! leases and exact runtime ownership remain separate downstream concerns.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const PROFILE_ACCESS_POLICY_SCHEMA_V1: &str = "agent-browser.profile-access-policy.v1";
pub const PROFILE_ACCESS_DECISION_SCHEMA_V1: &str = "agent-browser.profile-access-decision.v1";
pub const PROFILE_CHILD_ACCESS_SCHEMA_V1: &str = "agent-browser.profile-child-access.v1";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileConnectionState {
    #[default]
    Active,
    Disconnected,
}

/// Profile authority inherited by one browser, session, tab, or view child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileChildAccess {
    pub schema_version: String,
    pub parent_policy_revision: u64,
    pub access_decision_id: String,
    pub subject_id: Option<String>,
    pub identity_assurance: ProfileIdentityAssurance,
    pub connection_instance_id: Option<String>,
    pub connection_state: ProfileConnectionState,
    pub permissions: Vec<ProfilePermission>,
}

impl Default for ProfileChildAccess {
    fn default() -> Self {
        Self {
            schema_version: PROFILE_CHILD_ACCESS_SCHEMA_V1.to_string(),
            parent_policy_revision: 0,
            access_decision_id: String::new(),
            subject_id: None,
            identity_assurance: ProfileIdentityAssurance::Unknown,
            connection_instance_id: None,
            connection_state: ProfileConnectionState::Active,
            permissions: Vec::new(),
        }
    }
}

impl ProfileChildAccess {
    pub(crate) fn from_admission(
        policy: &ServiceProfileAccessPolicy,
        decision: &ServiceProfileAccessDecision,
        connection_instance_id: Option<String>,
    ) -> Self {
        Self {
            schema_version: PROFILE_CHILD_ACCESS_SCHEMA_V1.to_string(),
            parent_policy_revision: policy.revision,
            access_decision_id: decision.decision_id.clone(),
            subject_id: decision.subject.subject_id.clone(),
            identity_assurance: decision.subject.assurance,
            connection_instance_id,
            connection_state: ProfileConnectionState::Active,
            permissions: effective_profile_permissions(
                policy,
                decision.subject.subject_id.as_deref(),
                decision.subject.assurance,
            ),
        }
    }

    pub(crate) fn narrow_to(&self, requested: &[ProfilePermission]) -> Self {
        let allowed = self.permissions.iter().copied().collect::<BTreeSet<_>>();
        let mut child = self.clone();
        child.permissions = requested
            .iter()
            .copied()
            .filter(|permission| allowed.contains(permission))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        child
    }
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

pub(crate) fn effective_profile_permissions(
    policy: &ServiceProfileAccessPolicy,
    subject_id: Option<&str>,
    assurance: ProfileIdentityAssurance,
) -> Vec<ProfilePermission> {
    let mode_assurance_satisfied = match policy.mode {
        ProfileAccessMode::SharedLocal => true,
        ProfileAccessMode::Restricted => {
            assurance.satisfies(ProfileIdentityAssurance::AuthenticatedIngress)
        }
        ProfileAccessMode::Exclusive => {
            assurance.satisfies(ProfileIdentityAssurance::RegisteredCapability)
        }
    };
    if !mode_assurance_satisfied || policy.state != ProfileAccessPolicyState::Active {
        return Vec::new();
    }
    let mut permissions = policy
        .default_permissions
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if let Some(subject_id) = subject_id {
        for grant in policy.grants.iter().filter(|grant| {
            grant.subject_id == subject_id && assurance.satisfies(grant.minimum_assurance)
        }) {
            permissions.extend(grant.permissions.iter().copied());
        }
    }
    permissions.into_iter().collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileChildAccessRequest<'a> {
    pub(crate) child: &'a ProfileChildAccess,
    pub(crate) current_policy: &'a ServiceProfileAccessPolicy,
    pub(crate) subject_id: Option<&'a str>,
    pub(crate) assurance: ProfileIdentityAssurance,
    pub(crate) connection_instance_id: &'a str,
    pub(crate) permission: ProfilePermission,
    pub(crate) reconnect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileChildAccessResult {
    pub(crate) allowed: bool,
    pub(crate) reconnected: bool,
    pub(crate) reason: &'static str,
    pub(crate) child: ProfileChildAccess,
}

/// Authorize one child-resource operation without treating labels as a live
/// connection credential. Reconnect is possible only after the prior
/// service-generated connection is known disconnected.
pub(crate) fn evaluate_profile_child_access(
    input: ProfileChildAccessRequest<'_>,
) -> ProfileChildAccessResult {
    let current_permissions =
        effective_profile_permissions(input.current_policy, input.subject_id, input.assurance);
    let current_permission_set = current_permissions.into_iter().collect::<BTreeSet<_>>();
    let inherited_permission = input.child.permissions.contains(&input.permission)
        && current_permission_set.contains(&input.permission);
    let same_subject = input.child.subject_id.as_deref() == input.subject_id;
    let same_connection =
        input.child.connection_instance_id.as_deref() == Some(input.connection_instance_id);
    let reconnect_allowed = input.reconnect
        && same_subject
        && input.child.connection_state == ProfileConnectionState::Disconnected;
    let allowed = inherited_permission && same_subject && (same_connection || reconnect_allowed);
    let reason = if !same_subject {
        "subject_mismatch"
    } else if !inherited_permission {
        "permission_not_inherited"
    } else if same_connection {
        "owner_connection"
    } else if input.child.connection_state == ProfileConnectionState::Active {
        "owner_connection_still_active"
    } else if !input.reconnect {
        "explicit_reconnect_required"
    } else {
        "stable_subject_reconnected"
    };
    let mut child = input.child.clone();
    if allowed && reconnect_allowed {
        child.connection_instance_id = Some(input.connection_instance_id.to_string());
        child.connection_state = ProfileConnectionState::Active;
        child.identity_assurance = input.assurance;
        child.parent_policy_revision = input.current_policy.revision;
        child.permissions = child
            .narrow_to(&current_permission_set.iter().copied().collect::<Vec<_>>())
            .permissions;
    }
    ProfileChildAccessResult {
        allowed,
        reconnected: allowed && reconnect_allowed,
        reason,
        child,
    }
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

    fn shared_child(connection_state: ProfileConnectionState) -> ProfileChildAccess {
        let (policy, decision) = evaluate(None, ProfileIdentityAssurance::SelfDeclared, Vec::new());
        let mut child = ProfileChildAccess::from_admission(
            &policy,
            &decision,
            Some("connection:owner".to_string()),
        );
        child.connection_state = connection_state;
        child
    }

    #[test]
    fn live_child_access_belongs_to_the_service_generated_connection() {
        let policy = ServiceProfileAccessPolicy::shared_local_default("research-gov");
        let child = shared_child(ProfileConnectionState::Active);
        let result = evaluate_profile_child_access(ProfileChildAccessRequest {
            child: &child,
            current_policy: &policy,
            subject_id: Some("client:fieldwork"),
            assurance: ProfileIdentityAssurance::SelfDeclared,
            connection_instance_id: "connection:other",
            permission: ProfilePermission::TabControlOwn,
            reconnect: true,
        });

        assert!(!result.allowed);
        assert_eq!(result.reason, "owner_connection_still_active");
    }

    #[test]
    fn disconnected_child_can_reconnect_only_as_the_same_stable_subject() {
        let policy = ServiceProfileAccessPolicy::shared_local_default("research-gov");
        let child = shared_child(ProfileConnectionState::Disconnected);
        let wrong_subject = evaluate_profile_child_access(ProfileChildAccessRequest {
            child: &child,
            current_policy: &policy,
            subject_id: Some("client:other"),
            assurance: ProfileIdentityAssurance::SelfDeclared,
            connection_instance_id: "connection:new",
            permission: ProfilePermission::TabControlOwn,
            reconnect: true,
        });
        let reconnected = evaluate_profile_child_access(ProfileChildAccessRequest {
            child: &child,
            current_policy: &policy,
            subject_id: Some("client:fieldwork"),
            assurance: ProfileIdentityAssurance::SelfDeclared,
            connection_instance_id: "connection:new",
            permission: ProfilePermission::TabControlOwn,
            reconnect: true,
        });

        assert!(!wrong_subject.allowed);
        assert_eq!(wrong_subject.reason, "subject_mismatch");
        assert!(reconnected.allowed);
        assert!(reconnected.reconnected);
        assert_eq!(
            reconnected.child.connection_instance_id.as_deref(),
            Some("connection:new")
        );
    }

    #[test]
    fn child_narrowing_cannot_add_parent_permissions() {
        let child = shared_child(ProfileConnectionState::Active).narrow_to(&[
            ProfilePermission::TabObserve,
            ProfilePermission::TabCloseAny,
        ]);

        assert_eq!(child.permissions, vec![ProfilePermission::TabObserve]);
    }
}
