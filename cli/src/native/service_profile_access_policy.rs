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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileAccessDrain {
    pub target_mode: ProfileAccessMode,
    pub expected_revision: u64,
    pub incompatible_occupancy: Vec<String>,
    pub force_authorized: bool,
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
    pub drain: Option<ProfileAccessDrain>,
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
    if policy.state != ProfileAccessPolicyState::Active {
        return Vec::new();
    }
    profile_permissions_for_subject(policy, subject_id, assurance)
}

fn profile_permissions_for_subject(
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
    if !mode_assurance_satisfied {
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
    let current_permissions = if input.current_policy.state == ProfileAccessPolicyState::Draining
        && input.permission == ProfilePermission::TabCloseOwn
    {
        profile_permissions_for_subject(input.current_policy, input.subject_id, input.assurance)
    } else {
        effective_profile_permissions(input.current_policy, input.subject_id, input.assurance)
    };
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfilePolicyTarget {
    pub mode: ProfileAccessMode,
    pub default_permissions: Vec<ProfilePermission>,
    pub grants: Vec<ProfileAccessGrant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileEvictionMode {
    GracefulOnly,
    ForceImmediate,
    ForceAfterGrace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfilePolicyMutationRequest<'a> {
    pub(crate) expected_revision: u64,
    pub(crate) target: ProfilePolicyTarget,
    pub(crate) subject_id: Option<&'a str>,
    pub(crate) assurance: ProfileIdentityAssurance,
    pub(crate) incompatible_occupancy: Vec<String>,
    pub(crate) eviction_mode: Option<ProfileEvictionMode>,
    pub(crate) grace_deadline: Option<String>,
    pub(crate) now: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfilePolicyMutationOutcome {
    Unchanged,
    Widened,
    DrainStarted,
    DrainUpdated,
    Restricted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfilePolicyRevisionDiff {
    pub mode_changed: bool,
    pub default_permissions_added: usize,
    pub default_permissions_removed: usize,
    pub grant_count_changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfilePolicyMutationFailure {
    pub code: String,
    pub expected_revision: u64,
    pub current_revision: u64,
    pub missing_permission: Option<ProfilePermission>,
    pub current_diff: ProfilePolicyRevisionDiff,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileEvictionPlan {
    pub plan_id: String,
    pub profile_id: String,
    pub policy_revision: u64,
    pub requested_by: Option<String>,
    pub mode: ProfileEvictionMode,
    pub grace_deadline: Option<String>,
    pub target_resource_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileEvictionReceipt {
    pub receipt_id: String,
    pub plan_id: String,
    pub profile_id: String,
    pub policy_revision: u64,
    pub mode: ProfileEvictionMode,
    pub gracefully_released_resource_ids: Vec<String>,
    pub forcibly_evicted_resource_ids: Vec<String>,
    pub remaining_resource_ids: Vec<String>,
    pub outcome: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfilePolicyAuditReceipt {
    pub receipt_id: String,
    pub profile_id: String,
    pub subject_id: Option<String>,
    pub assurance: ProfileIdentityAssurance,
    pub operation: String,
    pub prior_revision: u64,
    pub resulting_revision: u64,
    pub drain_outcome: String,
    pub eviction_outcome: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfilePolicyMutationResult {
    pub(crate) outcome: ProfilePolicyMutationOutcome,
    pub(crate) policy: ServiceProfileAccessPolicy,
    pub(crate) blocking_occupancy: Vec<String>,
    pub(crate) eviction_plan: Option<ProfileEvictionPlan>,
    pub(crate) eviction_receipt: Option<ProfileEvictionReceipt>,
    pub(crate) receipt: ProfilePolicyAuditReceipt,
}

/// Apply one revision-fenced policy edit. Widening commits immediately.
/// Narrowing first establishes a durable admission fence and commits only
/// after the caller proves that incompatible occupancy has reached zero.
pub(crate) fn mutate_profile_policy(
    current: &ServiceProfileAccessPolicy,
    request: ProfilePolicyMutationRequest<'_>,
) -> Result<ProfilePolicyMutationResult, ProfilePolicyMutationFailure> {
    let target = canonical_policy_target(request.target.clone());
    let diff = policy_revision_diff(current, &target);
    if request.expected_revision != current.revision {
        return Err(policy_mutation_failure(
            "policy_revision_conflict",
            request.expected_revision,
            current,
            None,
            diff,
        ));
    }
    for permission in [ProfilePermission::PolicyWrite] {
        if !profile_permissions_for_subject(current, request.subject_id, request.assurance)
            .contains(&permission)
        {
            return Err(policy_mutation_failure(
                "profile_policy_permission_denied",
                request.expected_revision,
                current,
                Some(permission),
                diff,
            ));
        }
    }

    let unchanged = current.state == ProfileAccessPolicyState::Active
        && current.mode == target.mode
        && canonical_permissions(&current.default_permissions) == target.default_permissions
        && canonical_grants(&current.grants) == target.grants;
    if unchanged {
        return Ok(profile_policy_mutation_result(
            current.clone(),
            current,
            &request,
            ProfilePolicyMutationOutcome::Unchanged,
            Vec::new(),
            None,
        ));
    }

    let widening = current.state == ProfileAccessPolicyState::Active
        && policy_target_is_widening(current, &target);
    if widening {
        let policy = committed_policy(current, &target, request.now);
        return Ok(profile_policy_mutation_result(
            policy,
            current,
            &request,
            ProfilePolicyMutationOutcome::Widened,
            Vec::new(),
            None,
        ));
    }

    if !profile_permissions_for_subject(current, request.subject_id, request.assurance)
        .contains(&ProfilePermission::Drain)
    {
        return Err(policy_mutation_failure(
            "profile_drain_permission_denied",
            request.expected_revision,
            current,
            Some(ProfilePermission::Drain),
            diff,
        ));
    }
    if let Some(drain) = current.drain.as_ref() {
        if drain.target_mode != target.mode || drain.expected_revision != request.expected_revision
        {
            return Err(policy_mutation_failure(
                "profile_drain_target_conflict",
                request.expected_revision,
                current,
                None,
                diff,
            ));
        }
    }

    let occupancy = canonical_strings(request.incompatible_occupancy.clone());
    if occupancy.is_empty() {
        let policy = committed_policy(current, &target, request.now);
        return Ok(profile_policy_mutation_result(
            policy,
            current,
            &request,
            ProfilePolicyMutationOutcome::Restricted,
            Vec::new(),
            None,
        ));
    }

    let force_requested = matches!(
        request.eviction_mode,
        Some(ProfileEvictionMode::ForceImmediate | ProfileEvictionMode::ForceAfterGrace)
    );
    if matches!(
        request.eviction_mode,
        Some(ProfileEvictionMode::GracefulOnly | ProfileEvictionMode::ForceAfterGrace)
    ) && request.grace_deadline.is_none()
    {
        return Err(policy_mutation_failure(
            "profile_eviction_grace_deadline_required",
            request.expected_revision,
            current,
            None,
            diff,
        ));
    }
    if force_requested
        && !profile_permissions_for_subject(current, request.subject_id, request.assurance)
            .contains(&ProfilePermission::Evict)
    {
        return Err(policy_mutation_failure(
            "profile_evict_permission_denied",
            request.expected_revision,
            current,
            Some(ProfilePermission::Evict),
            diff,
        ));
    }
    let eviction_plan = request.eviction_mode.map(|mode| ProfileEvictionPlan {
        plan_id: stable_policy_operation_id(
            "profile-eviction-plan",
            &current.profile_id,
            current.revision,
            request.subject_id,
            &format!("{mode:?}:{occupancy:?}"),
        ),
        profile_id: current.profile_id.clone(),
        policy_revision: current.revision,
        requested_by: request.subject_id.map(str::to_string),
        mode,
        grace_deadline: request.grace_deadline.clone(),
        target_resource_ids: occupancy.clone(),
    });
    let mut policy = current.clone();
    let outcome = if policy.state == ProfileAccessPolicyState::Draining {
        ProfilePolicyMutationOutcome::DrainUpdated
    } else {
        ProfilePolicyMutationOutcome::DrainStarted
    };
    policy.state = ProfileAccessPolicyState::Draining;
    policy.drain = Some(ProfileAccessDrain {
        target_mode: target.mode,
        expected_revision: current.revision,
        incompatible_occupancy: occupancy.clone(),
        force_authorized: force_requested,
    });
    policy.updated_at = request.now.to_string();
    Ok(profile_policy_mutation_result(
        policy,
        current,
        &request,
        outcome,
        occupancy,
        eviction_plan,
    ))
}

/// Produce the minimal outcome receipt after the lease-authority and exact
/// lifecycle layers execute an explicit eviction plan.
pub(crate) fn record_profile_eviction_receipt(
    plan: &ProfileEvictionPlan,
    gracefully_released_resource_ids: Vec<String>,
    forcibly_evicted_resource_ids: Vec<String>,
    completed_at: &str,
) -> Result<ProfileEvictionReceipt, String> {
    let targets = plan
        .target_resource_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let graceful = canonical_strings(gracefully_released_resource_ids);
    let forced = canonical_strings(forcibly_evicted_resource_ids);
    if graceful
        .iter()
        .chain(forced.iter())
        .any(|resource_id| !targets.contains(resource_id))
    {
        return Err("profile_eviction_receipt_target_mismatch".to_string());
    }
    if graceful
        .iter()
        .any(|resource_id| forced.contains(resource_id))
    {
        return Err("profile_eviction_receipt_duplicate_outcome".to_string());
    }
    if plan.mode == ProfileEvictionMode::GracefulOnly && !forced.is_empty() {
        return Err("profile_eviction_receipt_force_not_authorized".to_string());
    }
    let completed = graceful
        .iter()
        .chain(forced.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    let remaining = targets.difference(&completed).cloned().collect::<Vec<_>>();
    let outcome = if remaining.is_empty() {
        if forced.is_empty() {
            "released_gracefully"
        } else {
            "forced_eviction_completed"
        }
    } else {
        "incomplete"
    };
    Ok(ProfileEvictionReceipt {
        receipt_id: stable_policy_operation_id(
            "profile-eviction-receipt",
            &plan.profile_id,
            plan.policy_revision,
            plan.requested_by.as_deref(),
            &format!("{}:{graceful:?}:{forced:?}", plan.plan_id),
        ),
        plan_id: plan.plan_id.clone(),
        profile_id: plan.profile_id.clone(),
        policy_revision: plan.policy_revision,
        mode: plan.mode,
        gracefully_released_resource_ids: graceful,
        forcibly_evicted_resource_ids: forced,
        remaining_resource_ids: remaining,
        outcome: outcome.to_string(),
        completed_at: completed_at.to_string(),
    })
}

fn committed_policy(
    current: &ServiceProfileAccessPolicy,
    target: &ProfilePolicyTarget,
    now: &str,
) -> ServiceProfileAccessPolicy {
    ServiceProfileAccessPolicy {
        mode: target.mode,
        revision: current.revision.saturating_add(1),
        state: ProfileAccessPolicyState::Active,
        default_permissions: target.default_permissions.clone(),
        grants: target.grants.clone(),
        drain: None,
        updated_at: now.to_string(),
        ..current.clone()
    }
}

fn profile_policy_mutation_result(
    policy: ServiceProfileAccessPolicy,
    current: &ServiceProfileAccessPolicy,
    request: &ProfilePolicyMutationRequest<'_>,
    outcome: ProfilePolicyMutationOutcome,
    blocking_occupancy: Vec<String>,
    eviction_plan: Option<ProfileEvictionPlan>,
) -> ProfilePolicyMutationResult {
    let drain_outcome = match outcome {
        ProfilePolicyMutationOutcome::DrainStarted => "started",
        ProfilePolicyMutationOutcome::DrainUpdated => "updated",
        ProfilePolicyMutationOutcome::Restricted => "completed",
        _ => "not_required",
    };
    let eviction_outcome = if eviction_plan.is_some() {
        "planned"
    } else {
        "not_requested"
    };
    let eviction_receipt = eviction_plan.as_ref().map(|plan| {
        record_profile_eviction_receipt(plan, Vec::new(), Vec::new(), request.now)
            .expect("an exact eviction plan must produce its initial receipt")
    });
    ProfilePolicyMutationResult {
        receipt: ProfilePolicyAuditReceipt {
            receipt_id: stable_policy_operation_id(
                "profile-policy-receipt",
                &current.profile_id,
                current.revision,
                request.subject_id,
                &format!("{outcome:?}:{}", policy.revision),
            ),
            profile_id: current.profile_id.clone(),
            subject_id: request.subject_id.map(str::to_string),
            assurance: request.assurance,
            operation: "profile_policy_update".to_string(),
            prior_revision: current.revision,
            resulting_revision: policy.revision,
            drain_outcome: drain_outcome.to_string(),
            eviction_outcome: eviction_outcome.to_string(),
            occurred_at: request.now.to_string(),
        },
        outcome,
        policy,
        blocking_occupancy,
        eviction_plan,
        eviction_receipt,
    }
}

fn policy_mutation_failure(
    code: &str,
    expected_revision: u64,
    current: &ServiceProfileAccessPolicy,
    missing_permission: Option<ProfilePermission>,
    current_diff: ProfilePolicyRevisionDiff,
) -> ProfilePolicyMutationFailure {
    ProfilePolicyMutationFailure {
        code: code.to_string(),
        expected_revision,
        current_revision: current.revision,
        missing_permission,
        current_diff,
    }
}

fn policy_target_is_widening(
    current: &ServiceProfileAccessPolicy,
    target: &ProfilePolicyTarget,
) -> bool {
    if access_mode_rank(target.mode) > access_mode_rank(current.mode) {
        return false;
    }
    let target_defaults = target
        .default_permissions
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if current
        .default_permissions
        .iter()
        .any(|permission| !target_defaults.contains(permission))
    {
        return false;
    }
    current.grants.iter().all(|current_grant| {
        target.grants.iter().any(|target_grant| {
            target_grant.subject_id == current_grant.subject_id
                && target_grant.minimum_assurance.rank() <= current_grant.minimum_assurance.rank()
                && current_grant
                    .permissions
                    .iter()
                    .all(|permission| target_grant.permissions.contains(permission))
        })
    })
}

fn access_mode_rank(mode: ProfileAccessMode) -> u8 {
    match mode {
        ProfileAccessMode::SharedLocal => 0,
        ProfileAccessMode::Restricted => 1,
        ProfileAccessMode::Exclusive => 2,
    }
}

fn canonical_policy_target(mut target: ProfilePolicyTarget) -> ProfilePolicyTarget {
    target.default_permissions = canonical_permissions(&target.default_permissions);
    target.grants = canonical_grants(&target.grants);
    target
}

fn canonical_permissions(permissions: &[ProfilePermission]) -> Vec<ProfilePermission> {
    permissions
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn canonical_grants(grants: &[ProfileAccessGrant]) -> Vec<ProfileAccessGrant> {
    let mut grants = grants
        .iter()
        .cloned()
        .map(|mut grant| {
            grant.permissions = canonical_permissions(&grant.permissions);
            grant
        })
        .collect::<Vec<_>>();
    grants.sort_by(|left, right| {
        left.subject_id.cmp(&right.subject_id).then_with(|| {
            left.minimum_assurance
                .rank()
                .cmp(&right.minimum_assurance.rank())
        })
    });
    grants
}

fn canonical_strings(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn policy_revision_diff(
    current: &ServiceProfileAccessPolicy,
    target: &ProfilePolicyTarget,
) -> ProfilePolicyRevisionDiff {
    let current_permissions = current
        .default_permissions
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let target_permissions = target
        .default_permissions
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    ProfilePolicyRevisionDiff {
        mode_changed: current.mode != target.mode,
        default_permissions_added: target_permissions.difference(&current_permissions).count(),
        default_permissions_removed: current_permissions.difference(&target_permissions).count(),
        grant_count_changed: current.grants.len() != target.grants.len(),
    }
}

fn stable_policy_operation_id(
    prefix: &str,
    profile_id: &str,
    revision: u64,
    subject_id: Option<&str>,
    operation: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    hasher.update(b"\0");
    hasher.update(profile_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(revision.to_le_bytes());
    hasher.update(b"\0");
    hasher.update(subject_id.unwrap_or("unknown").as_bytes());
    hasher.update(b"\0");
    hasher.update(operation.as_bytes());
    let suffix = hasher
        .finalize()
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{prefix}:{suffix}")
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

    fn administrative_policy(include_evict: bool) -> ServiceProfileAccessPolicy {
        let mut permissions = vec![ProfilePermission::PolicyWrite, ProfilePermission::Drain];
        if include_evict {
            permissions.push(ProfilePermission::Evict);
        }
        ServiceProfileAccessPolicy {
            profile_id: "research-gov".to_string(),
            revision: 7,
            grants: vec![ProfileAccessGrant {
                subject_id: "operator:admin".to_string(),
                minimum_assurance: ProfileIdentityAssurance::Operator,
                permissions,
            }],
            ..ServiceProfileAccessPolicy::shared_local_default("research-gov")
        }
    }

    fn restricted_target() -> ProfilePolicyTarget {
        ProfilePolicyTarget {
            mode: ProfileAccessMode::Restricted,
            default_permissions: Vec::new(),
            grants: vec![ProfileAccessGrant {
                subject_id: "operator:admin".to_string(),
                minimum_assurance: ProfileIdentityAssurance::Operator,
                permissions: vec![
                    ProfilePermission::PolicyWrite,
                    ProfilePermission::Drain,
                    ProfilePermission::Evict,
                    ProfilePermission::ProfileUse,
                ],
            }],
        }
    }

    fn mutation_request(
        target: ProfilePolicyTarget,
        occupancy: Vec<String>,
        eviction_mode: Option<ProfileEvictionMode>,
        grace_deadline: Option<&str>,
    ) -> ProfilePolicyMutationRequest<'static> {
        ProfilePolicyMutationRequest {
            expected_revision: 7,
            target,
            subject_id: Some("operator:admin"),
            assurance: ProfileIdentityAssurance::Operator,
            incompatible_occupancy: occupancy,
            eviction_mode,
            grace_deadline: grace_deadline.map(str::to_string),
            now: "2026-09-02T18:00:00Z",
        }
    }

    #[test]
    fn narrowing_fences_admission_then_commits_only_after_occupancy_drains() {
        let current = administrative_policy(true);
        let started = mutate_profile_policy(
            &current,
            mutation_request(
                restricted_target(),
                vec!["tab:z".to_string(), "tab:a".to_string()],
                None,
                None,
            ),
        )
        .expect("authorized narrowing should begin a drain");

        assert_eq!(started.outcome, ProfilePolicyMutationOutcome::DrainStarted);
        assert_eq!(started.policy.revision, 7);
        assert_eq!(started.policy.state, ProfileAccessPolicyState::Draining);
        assert_eq!(started.blocking_occupancy, vec!["tab:a", "tab:z"]);
        assert!(started.eviction_plan.is_none());
        let (_, denied_during_drain) = evaluate_profile_access(ProfileAccessEvaluation {
            profile_id: "research-gov",
            explicit_policy: Some(&started.policy),
            subject_id: Some("client:new".to_string()),
            assurance: ProfileIdentityAssurance::SelfDeclared,
            connection_instance_id: Some("connection:new".to_string()),
            permission: ProfilePermission::TabCreate,
            operation: "tab_create",
            incompatible_occupancy: Vec::new(),
        });
        assert!(!denied_during_drain.allowed);
        let draining_child = shared_child(ProfileConnectionState::Active);
        let close_during_drain = evaluate_profile_child_access(ProfileChildAccessRequest {
            child: &draining_child,
            current_policy: &started.policy,
            subject_id: Some("client:fieldwork"),
            assurance: ProfileIdentityAssurance::SelfDeclared,
            connection_instance_id: "connection:owner",
            permission: ProfilePermission::TabCloseOwn,
            reconnect: false,
        });
        let control_during_drain = evaluate_profile_child_access(ProfileChildAccessRequest {
            child: &draining_child,
            current_policy: &started.policy,
            subject_id: Some("client:fieldwork"),
            assurance: ProfileIdentityAssurance::SelfDeclared,
            connection_instance_id: "connection:owner",
            permission: ProfilePermission::TabControlOwn,
            reconnect: false,
        });
        assert!(close_during_drain.allowed);
        assert!(!control_during_drain.allowed);

        let completed = mutate_profile_policy(
            &started.policy,
            mutation_request(restricted_target(), Vec::new(), None, None),
        )
        .expect("the same target should commit after occupancy reaches zero");
        assert_eq!(completed.outcome, ProfilePolicyMutationOutcome::Restricted);
        assert_eq!(completed.policy.revision, 8);
        assert_eq!(completed.policy.state, ProfileAccessPolicyState::Active);
        assert!(completed.policy.drain.is_none());
    }

    #[test]
    fn widening_commits_immediately_at_a_new_revision() {
        let mut current = administrative_policy(true);
        current.mode = ProfileAccessMode::Restricted;
        current.default_permissions = Vec::new();
        let target = ProfilePolicyTarget {
            mode: ProfileAccessMode::SharedLocal,
            default_permissions: ServiceProfileAccessPolicy::shared_local_default("research-gov")
                .default_permissions,
            grants: current.grants.clone(),
        };

        let result = mutate_profile_policy(
            &current,
            mutation_request(target, vec!["tab:existing".to_string()], None, None),
        )
        .expect("widening should not wait for occupancy");
        assert_eq!(result.outcome, ProfilePolicyMutationOutcome::Widened);
        assert_eq!(result.policy.revision, 8);
        assert_eq!(result.policy.state, ProfileAccessPolicyState::Active);
        assert!(result.blocking_occupancy.is_empty());
    }

    #[test]
    fn revision_conflict_returns_current_revision_and_redacted_diff() {
        let current = administrative_policy(true);
        let mut request = mutation_request(restricted_target(), Vec::new(), None, None);
        request.expected_revision = 6;

        let failure = mutate_profile_policy(&current, request)
            .expect_err("stale expected revision must fail closed");
        assert_eq!(failure.code, "policy_revision_conflict");
        assert_eq!(failure.expected_revision, 6);
        assert_eq!(failure.current_revision, 7);
        assert!(failure.current_diff.mode_changed);
        assert!(failure.current_diff.default_permissions_removed > 0);
    }

    #[test]
    fn force_is_explicit_permission_checked_and_minimally_receipted() {
        let current = administrative_policy(true);
        let planned = mutate_profile_policy(
            &current,
            mutation_request(
                restricted_target(),
                vec!["tab:b".to_string(), "tab:a".to_string()],
                Some(ProfileEvictionMode::ForceAfterGrace),
                Some("2026-09-02T18:05:00Z"),
            ),
        )
        .expect("authorized explicit force should produce an exact plan");
        let plan = planned
            .eviction_plan
            .as_ref()
            .expect("explicit force should be planned");
        assert_eq!(plan.target_resource_ids, vec!["tab:a", "tab:b"]);
        assert!(planned.policy.drain.as_ref().unwrap().force_authorized);

        let denied = mutate_profile_policy(
            &administrative_policy(false),
            mutation_request(
                restricted_target(),
                vec!["tab:a".to_string()],
                Some(ProfileEvictionMode::ForceImmediate),
                None,
            ),
        )
        .expect_err("force requires the separate evict permission");
        assert_eq!(denied.missing_permission, Some(ProfilePermission::Evict));

        let receipt = record_profile_eviction_receipt(
            plan,
            vec!["tab:a".to_string()],
            vec!["tab:b".to_string()],
            "2026-09-02T18:05:01Z",
        )
        .expect("exact target outcomes should produce a receipt");
        assert_eq!(receipt.outcome, "forced_eviction_completed");
        assert!(receipt.remaining_resource_ids.is_empty());
        let serialized = serde_json::to_string(&receipt).unwrap();
        assert!(!serialized.contains("page"));
        assert!(!serialized.contains("form"));
    }
}
