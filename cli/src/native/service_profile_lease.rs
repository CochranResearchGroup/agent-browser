//! First-class profile lease projection and guarded lifecycle operations.
//!
//! Profile leases are derived from authenticated principal capability, the
//! existing runtime owner registry, and subordinate session and tab work
//! leases. Labels never grant authority. Every mutation uses an exact lease
//! revision and the caller's authenticated profile capability.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use super::service_model::{LeaseState, ServiceState, TabLifecycle};
use super::service_principal::{
    authenticated_authority_is_current, AuthenticatedServicePrincipal, PrincipalContinuityRecourse,
    ServicePrincipalProvenance, ServicePrincipalState, ServiceProfileCapabilityState,
};
use super::service_resources::load_service_state_for_maintenance;
use super::service_trace::service_commands::service_now_timestamp;

pub(crate) const PROFILE_LEASE_SCHEMA_VERSION: &str = "agent-browser.profile-lease.v1";
pub(crate) const PROFILE_LEASE_RECONCILE_PLAN_SCHEMA_VERSION: &str =
    "agent-browser.profile-lease-reconcile-plan.v1";
pub(crate) const PROFILE_LEASE_RECONCILE_RECEIPT_SCHEMA_VERSION: &str =
    "agent-browser.profile-lease-reconcile-receipt.v1";

const READ_ACTIONS: [&str; 5] = ["list", "inspect", "explain", "doctor", "watch"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLeaseRecord {
    pub(crate) schema_version: String,
    pub(crate) id: String,
    pub(crate) lease_revision: String,
    pub(crate) principal_id: Option<String>,
    pub(crate) principal_provenance: Option<ServicePrincipalProvenance>,
    pub(crate) profile_id: String,
    pub(crate) profile_identity_digest: Option<String>,
    pub(crate) browser_id: Option<String>,
    pub(crate) session_ids: Vec<String>,
    pub(crate) tab_ids: Vec<String>,
    pub(crate) mode: String,
    pub(crate) state: String,
    pub(crate) owner_generation: Option<u64>,
    pub(crate) process_instance_digest: Option<String>,
    pub(crate) route_ids: Vec<String>,
    pub(crate) last_heartbeat_at: Option<String>,
    pub(crate) expires_at: Option<String>,
    pub(crate) cleanup_obligation: Option<String>,
    pub(crate) blocking_identity_axes: Vec<String>,
    pub(crate) authorized_actions: Vec<String>,
    pub(crate) recourse: PrincipalContinuityRecourse,
    pub(crate) observation_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLeaseFinding {
    pub(crate) code: String,
    pub(crate) severity: String,
    pub(crate) lease_id: String,
    pub(crate) profile_id: String,
    pub(crate) message: String,
    pub(crate) safe_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLeaseDoctorReport {
    pub(crate) schema_version: String,
    pub(crate) observed_at: String,
    pub(crate) healthy: bool,
    pub(crate) lease_count: usize,
    pub(crate) findings: Vec<ProfileLeaseFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLeaseTransition {
    pub(crate) action: String,
    pub(crate) session_id: String,
    pub(crate) from_state: String,
    pub(crate) to_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLeaseReconcilePlan {
    pub(crate) schema_version: String,
    pub(crate) plan_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_revision: String,
    pub(crate) owner_generation: Option<u64>,
    pub(crate) principal_id: String,
    pub(crate) profile_id: String,
    pub(crate) browser_id: Option<String>,
    pub(crate) process_instance_digest: Option<String>,
    pub(crate) route_ids: Vec<String>,
    pub(crate) boot_epoch: Option<String>,
    pub(crate) proposed_transitions: Vec<ProfileLeaseTransition>,
    pub(crate) idempotency_key: String,
    pub(crate) issued_at: String,
    pub(crate) expires_at: String,
    pub(crate) effect_capable: bool,
    pub(crate) blocked_reasons: Vec<String>,
    pub(crate) seal: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLeaseReconcileReceipt {
    pub(crate) schema_version: String,
    pub(crate) idempotency_key: String,
    pub(crate) plan_id: String,
    pub(crate) lease_id: String,
    pub(crate) principal_id: String,
    pub(crate) applied_at: String,
    pub(crate) replayed: bool,
    pub(crate) transition_count: usize,
    pub(crate) resulting_lease_revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileLeaseFailureCode {
    LeaseNotFound,
    AuthorityMismatch,
    RevisionMismatch,
    ActionNotAuthorized,
    ActiveSubordinateWork,
    PlanInvalid,
    PlanExpired,
    BootEpochMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileLeaseError {
    pub(crate) code: ProfileLeaseFailureCode,
    pub(crate) message: String,
}

/// Projects the canonical first-class profile lease collection from retained authority and work.
pub(crate) fn profile_leases_for_state(state: &ServiceState, now: &str) -> Vec<ProfileLeaseRecord> {
    let mut records = Vec::new();
    let mut bound_profiles = BTreeSet::new();
    for binding in state.runtime_owner_registry.principal_bindings.values() {
        bound_profiles.insert(binding.profile_id.clone());
        records.push(bound_profile_lease(state, binding, now));
    }

    let mut legacy_profiles = state
        .sessions
        .values()
        .filter_map(|session| session.profile_id.clone())
        .filter(|profile_id| !bound_profiles.contains(profile_id))
        .collect::<BTreeSet<_>>();
    for owner in state.runtime_owner_registry.owners.values() {
        if state
            .runtime_owner_registry
            .principal_bindings
            .contains_key(&owner.profile_identity_digest)
        {
            continue;
        }
        if let Some(profile_id) = state
            .browsers
            .get(&owner.browser_id)
            .and_then(|browser| browser.profile_id.clone())
        {
            legacy_profiles.insert(profile_id);
        }
    }
    for profile_id in legacy_profiles {
        records.push(legacy_profile_lease(state, &profile_id, now));
    }
    records.sort_by(|left, right| left.id.cmp(&right.id));
    records
}

/// Handles the no-launch CLI collection read and includes the matching doctor result.
pub(crate) async fn handle_service_profile_leases(
    command: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let state = load_service_state_for_maintenance(command)?;
    let now = service_now_timestamp();
    let profile_leases = profile_leases_for_state(&state, &now);
    let doctor = doctor_profile_leases(&state, &now);
    Ok(json!({
        "profileLeases": profile_leases,
        "count": profile_leases.len(),
        "observedAt": now,
        "doctor": doctor,
    }))
}

pub(crate) fn inspect_profile_lease(
    state: &ServiceState,
    lease_id: &str,
    now: &str,
) -> Result<ProfileLeaseRecord, ProfileLeaseError> {
    profile_leases_for_state(state, now)
        .into_iter()
        .find(|lease| lease.id == lease_id)
        .ok_or_else(|| lease_error(ProfileLeaseFailureCode::LeaseNotFound, lease_id))
}

pub(crate) fn doctor_profile_leases(state: &ServiceState, now: &str) -> ProfileLeaseDoctorReport {
    let leases = profile_leases_for_state(state, now);
    let mut findings = Vec::new();
    for lease in &leases {
        for axis in &lease.blocking_identity_axes {
            findings.push(ProfileLeaseFinding {
                code: axis.clone(),
                severity: if lease.observation_only {
                    "warning"
                } else {
                    "error"
                }
                .to_string(),
                lease_id: lease.id.clone(),
                profile_id: lease.profile_id.clone(),
                message: format!("Profile lease {} is blocked by {}", lease.id, axis),
                safe_actions: lease
                    .authorized_actions
                    .iter()
                    .filter(|action| {
                        matches!(
                            action.as_str(),
                            "inspect" | "explain" | "doctor" | "reconcile_plan"
                        )
                    })
                    .cloned()
                    .collect(),
            });
        }
    }
    ProfileLeaseDoctorReport {
        schema_version: PROFILE_LEASE_SCHEMA_VERSION.to_string(),
        observed_at: now.to_string(),
        healthy: findings.is_empty(),
        lease_count: leases.len(),
        findings,
    }
}

pub(crate) fn rejoin_profile_lease(
    state: &ServiceState,
    lease_id: &str,
    expected_revision: &str,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
) -> Result<ProfileLeaseRecord, ProfileLeaseError> {
    let lease = authorized_lease(state, lease_id, expected_revision, authority, now)?;
    require_action(&lease, "rejoin")?;
    Ok(lease)
}

pub(crate) fn renew_profile_lease(
    state: &mut ServiceState,
    lease_id: &str,
    expected_revision: &str,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
    expires_at: &str,
) -> Result<ProfileLeaseRecord, ProfileLeaseError> {
    let lease = authorized_lease(state, lease_id, expected_revision, authority, now)?;
    require_action(&lease, "renew")?;
    if expires_at <= now {
        return Err(lease_error(
            ProfileLeaseFailureCode::ActionNotAuthorized,
            lease_id,
        ));
    }
    for session in state.sessions.values_mut().filter(|session| {
        session.profile_id.as_deref() == Some(authority.profile_id.as_str())
            && session.principal_id.as_deref() == Some(authority.principal_id.as_str())
            && !inactive_session(session.lease)
    }) {
        session.expires_at = Some(expires_at.to_string());
        session.last_lease_observed_at = Some(now.to_string());
        session.work_lease_revision = session.work_lease_revision.saturating_add(1).max(1);
    }
    inspect_profile_lease(state, lease_id, now)
}

pub(crate) fn release_profile_lease(
    state: &mut ServiceState,
    lease_id: &str,
    expected_revision: &str,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
) -> Result<ProfileLeaseRecord, ProfileLeaseError> {
    let lease = authorized_lease(state, lease_id, expected_revision, authority, now)?;
    require_action(&lease, "release")?;
    if active_subordinate_tabs(state, authority, now)
        .next()
        .is_some()
    {
        return Err(lease_error(
            ProfileLeaseFailureCode::ActiveSubordinateWork,
            lease_id,
        ));
    }
    for session in state.sessions.values_mut().filter(|session| {
        session.profile_id.as_deref() == Some(authority.profile_id.as_str())
            && session.principal_id.as_deref() == Some(authority.principal_id.as_str())
            && !inactive_session(session.lease)
    }) {
        session.lease = LeaseState::Released;
        session.expires_at = Some(now.to_string());
        session.last_lease_observed_at = Some(now.to_string());
        session.work_lease_revision = session.work_lease_revision.saturating_add(1).max(1);
    }
    inspect_profile_lease(state, lease_id, now)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_profile_lease_reconciliation(
    state: &ServiceState,
    lease_id: &str,
    expected_revision: &str,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
    expires_at: &str,
    boot_epoch: Option<String>,
    idempotency_key: String,
    seal_key: &[u8],
) -> Result<ProfileLeaseReconcilePlan, ProfileLeaseError> {
    let lease = authorized_lease(state, lease_id, expected_revision, authority, now)?;
    require_action(&lease, "reconcile_plan")?;
    if seal_key.len() < 32 || idempotency_key.trim().is_empty() {
        return Err(lease_error(ProfileLeaseFailureCode::PlanInvalid, lease_id));
    }
    if expires_at <= now {
        return Err(lease_error(ProfileLeaseFailureCode::PlanExpired, lease_id));
    }
    let boot_epoch = boot_epoch.filter(|epoch| !epoch.trim().is_empty());
    let proposed_transitions = state
        .sessions
        .values()
        .filter(|session| {
            session.profile_id.as_deref() == Some(authority.profile_id.as_str())
                && session.principal_id.as_deref() == Some(authority.principal_id.as_str())
                && inactive_session(session.lease)
        })
        .map(|session| ProfileLeaseTransition {
            action: "confirm_stale_session_released".to_string(),
            session_id: session.id.clone(),
            from_state: lease_state_name(session.lease).to_string(),
            to_state: "released".to_string(),
        })
        .collect::<Vec<_>>();
    let mut blocked_reasons = Vec::new();
    if boot_epoch.is_none() {
        blocked_reasons.push("boot_epoch_unavailable".to_string());
    }
    if proposed_transitions.is_empty() {
        blocked_reasons.push("no_safe_reconciliation_transition".to_string());
    }
    let effect_capable = blocked_reasons.is_empty();
    let mut plan = ProfileLeaseReconcilePlan {
        schema_version: PROFILE_LEASE_RECONCILE_PLAN_SCHEMA_VERSION.to_string(),
        plan_id: format!(
            "profile-lease-reconcile-plan-v1:{}",
            digest_prefix(format!("{lease_id}\0{expected_revision}\0{idempotency_key}").as_bytes())
        ),
        lease_id: lease.id,
        lease_revision: lease.lease_revision,
        owner_generation: lease.owner_generation,
        principal_id: authority.principal_id.clone(),
        profile_id: authority.profile_id.clone(),
        browser_id: lease.browser_id,
        process_instance_digest: lease.process_instance_digest,
        route_ids: lease.route_ids,
        boot_epoch,
        proposed_transitions,
        idempotency_key,
        issued_at: now.to_string(),
        expires_at: expires_at.to_string(),
        effect_capable,
        blocked_reasons,
        seal: String::new(),
    };
    plan.seal = seal_reconcile_plan(&plan, seal_key);
    Ok(plan)
}

pub(crate) fn apply_profile_lease_reconciliation(
    state: &mut ServiceState,
    plan: &ProfileLeaseReconcilePlan,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
    current_boot_epoch: Option<&str>,
    seal_key: &[u8],
) -> Result<ProfileLeaseReconcileReceipt, ProfileLeaseError> {
    if !authenticated_authority_is_current(&state.service_principals, authority) {
        return Err(lease_error(
            ProfileLeaseFailureCode::AuthorityMismatch,
            &plan.lease_id,
        ));
    }
    if let Some(receipt) = state
        .profile_lease_reconcile_receipts
        .get(&plan.idempotency_key)
    {
        if receipt.principal_id != authority.principal_id {
            return Err(lease_error(
                ProfileLeaseFailureCode::AuthorityMismatch,
                &plan.lease_id,
            ));
        }
        let mut replay = receipt.clone();
        replay.replayed = true;
        return Ok(replay);
    }
    if seal_key.len() < 32
        || plan.seal != seal_reconcile_plan(plan, seal_key)
        || !plan.effect_capable
    {
        return Err(lease_error(
            ProfileLeaseFailureCode::PlanInvalid,
            &plan.lease_id,
        ));
    }
    if plan.expires_at.as_str() <= now {
        return Err(lease_error(
            ProfileLeaseFailureCode::PlanExpired,
            &plan.lease_id,
        ));
    }
    if plan.boot_epoch.as_deref() != current_boot_epoch {
        return Err(lease_error(
            ProfileLeaseFailureCode::BootEpochMismatch,
            &plan.lease_id,
        ));
    }
    let lease = authorized_lease(state, &plan.lease_id, &plan.lease_revision, authority, now)?;
    if plan.owner_generation != lease.owner_generation
        || plan.principal_id != authority.principal_id
        || plan.profile_id != authority.profile_id
        || plan.browser_id != lease.browser_id
        || plan.process_instance_digest != lease.process_instance_digest
        || plan.route_ids != lease.route_ids
    {
        return Err(lease_error(
            ProfileLeaseFailureCode::PlanInvalid,
            &plan.lease_id,
        ));
    }
    for transition in &plan.proposed_transitions {
        let session = state
            .sessions
            .get_mut(&transition.session_id)
            .ok_or_else(|| lease_error(ProfileLeaseFailureCode::PlanInvalid, &plan.lease_id))?;
        if session.profile_id.as_deref() != Some(authority.profile_id.as_str())
            || session.principal_id.as_deref() != Some(authority.principal_id.as_str())
            || !inactive_session(session.lease)
            || transition.action != "confirm_stale_session_released"
        {
            return Err(lease_error(
                ProfileLeaseFailureCode::PlanInvalid,
                &plan.lease_id,
            ));
        }
        session.lease = LeaseState::Released;
        session.expires_at = Some(now.to_string());
        session.last_lease_observed_at = Some(now.to_string());
        session.work_lease_revision = session.work_lease_revision.saturating_add(1).max(1);
    }
    let resulting = inspect_profile_lease(state, &plan.lease_id, now)?;
    let receipt = ProfileLeaseReconcileReceipt {
        schema_version: PROFILE_LEASE_RECONCILE_RECEIPT_SCHEMA_VERSION.to_string(),
        idempotency_key: plan.idempotency_key.clone(),
        plan_id: plan.plan_id.clone(),
        lease_id: plan.lease_id.clone(),
        principal_id: plan.principal_id.clone(),
        applied_at: now.to_string(),
        replayed: false,
        transition_count: plan.proposed_transitions.len(),
        resulting_lease_revision: resulting.lease_revision,
    };
    state
        .profile_lease_reconcile_receipts
        .insert(receipt.idempotency_key.clone(), receipt.clone());
    Ok(receipt)
}

fn bound_profile_lease(
    state: &ServiceState,
    binding: &crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding,
    now: &str,
) -> ProfileLeaseRecord {
    let owner = state
        .runtime_owner_registry
        .owners
        .get(&binding.profile_identity_digest);
    let owner_current = state
        .runtime_owner_registry
        .principal_binding_is_current(Some(binding));
    let capability_current = state
        .service_principals
        .profile_capabilities
        .get(&binding.capability_id)
        .is_some_and(|capability| {
            capability.state == ServiceProfileCapabilityState::Active
                && capability.principal_id == binding.principal_id
                && capability.profile_id == binding.profile_id
                && state
                    .service_principals
                    .principals
                    .get(&binding.principal_id)
                    .is_some_and(|principal| principal.state == ServicePrincipalState::Active)
        });
    let sessions = sessions_for_profile(state, &binding.profile_id);
    let active = sessions
        .iter()
        .copied()
        .filter(|session| !inactive_or_expired(session.lease, session.expires_at.as_deref(), now))
        .collect::<Vec<_>>();
    let same = active
        .iter()
        .copied()
        .filter(|session| session.principal_id.as_deref() == Some(binding.principal_id.as_str()))
        .collect::<Vec<_>>();
    let foreign = active
        .iter()
        .copied()
        .filter(|session| {
            session
                .principal_id
                .as_deref()
                .is_some_and(|principal_id| principal_id != binding.principal_id)
        })
        .collect::<Vec<_>>();
    let unproven = active
        .iter()
        .copied()
        .filter(|session| {
            session.principal_id.is_none()
                || session.principal_provenance
                    != Some(ServicePrincipalProvenance::RegisteredCapability)
        })
        .collect::<Vec<_>>();
    let stale_same = sessions.iter().copied().any(|session| {
        session.principal_id.as_deref() == Some(binding.principal_id.as_str())
            && inactive_or_expired(session.lease, session.expires_at.as_deref(), now)
    });
    let mut blocking = Vec::new();
    if !owner_current {
        blocking.push("owner_generation_or_binding_mismatch".to_string());
    }
    if !capability_current {
        blocking.push("principal_capability_not_current".to_string());
    }
    if !foreign.is_empty() {
        blocking.push("foreign_principal_active_work".to_string());
    }
    if !unproven.is_empty() {
        blocking.push("unproven_session_authority".to_string());
    }
    let coherent = blocking.is_empty();
    let (lease_state, recourse) = if !foreign.is_empty() {
        (
            "foreign_held",
            PrincipalContinuityRecourse::WaitForForeignPrincipal,
        )
    } else if !coherent {
        (
            "identity_reconciliation_required",
            PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
        )
    } else if !same.is_empty() {
        ("active", PrincipalContinuityRecourse::RejoinOwnedBrowser)
    } else if stale_same {
        (
            "stale",
            PrincipalContinuityRecourse::ReplaceStaleSamePrincipalSession,
        )
    } else {
        (
            "owned_idle",
            PrincipalContinuityRecourse::RejoinOwnedBrowser,
        )
    };
    let mut actions = READ_ACTIONS
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if coherent {
        actions.push("rejoin".to_string());
        if !same.is_empty() {
            actions.push("renew".to_string());
        }
        let authority = authority_for_binding(state, binding);
        let has_active_tabs = authority.as_ref().is_some_and(|authority| {
            active_subordinate_tabs(state, authority, now)
                .next()
                .is_some()
        });
        if !same.is_empty() && !has_active_tabs {
            actions.push("release".to_string());
        }
    }
    if stale_same || !blocking.is_empty() {
        actions.push("reconcile_plan".to_string());
    }
    actions.sort();
    actions.dedup();
    let session_ids = sessions
        .iter()
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    let tab_ids = tabs_for_sessions(state, &session_ids);
    let route_ids = owner
        .map(|owner| routes_for_browser(state, &owner.browser_id))
        .unwrap_or_default();
    let mut record = ProfileLeaseRecord {
        schema_version: PROFILE_LEASE_SCHEMA_VERSION.to_string(),
        id: profile_lease_id(&binding.principal_id, &binding.profile_identity_digest),
        lease_revision: String::new(),
        principal_id: Some(binding.principal_id.clone()),
        principal_provenance: Some(binding.provenance),
        profile_id: binding.profile_id.clone(),
        profile_identity_digest: Some(binding.profile_identity_digest.clone()),
        browser_id: owner.map(|owner| owner.browser_id.clone()),
        session_ids,
        tab_ids,
        mode: lease_mode(&active),
        state: lease_state.to_string(),
        owner_generation: owner.map(|owner| owner.owner_generation),
        process_instance_digest: owner.map(|owner| owner.process_instance_digest.clone()),
        route_ids,
        last_heartbeat_at: latest_value(
            sessions
                .iter()
                .filter_map(|session| session.last_lease_observed_at.as_deref()),
        ),
        expires_at: earliest_value(
            same.iter()
                .filter_map(|session| session.expires_at.as_deref()),
        ),
        cleanup_obligation: owner
            .and_then(|owner| cleanup_obligation_for_browser(state, &owner.browser_id)),
        blocking_identity_axes: blocking,
        authorized_actions: actions,
        recourse,
        observation_only: !coherent,
    };
    record.lease_revision = lease_revision(&record);
    record
}

fn legacy_profile_lease(state: &ServiceState, profile_id: &str, now: &str) -> ProfileLeaseRecord {
    let sessions = sessions_for_profile(state, profile_id);
    let session_ids = sessions
        .iter()
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    let active = sessions
        .iter()
        .copied()
        .filter(|session| !inactive_or_expired(session.lease, session.expires_at.as_deref(), now))
        .collect::<Vec<_>>();
    let mut record = ProfileLeaseRecord {
        schema_version: PROFILE_LEASE_SCHEMA_VERSION.to_string(),
        id: profile_lease_id("unproven-legacy", profile_id),
        lease_revision: String::new(),
        principal_id: None,
        principal_provenance: Some(ServicePrincipalProvenance::UnprovenLegacy),
        profile_id: profile_id.to_string(),
        profile_identity_digest: None,
        browser_id: None,
        session_ids: session_ids.clone(),
        tab_ids: tabs_for_sessions(state, &session_ids),
        mode: lease_mode(&active),
        state: "identity_reconciliation_required".to_string(),
        owner_generation: None,
        process_instance_digest: None,
        route_ids: Vec::new(),
        last_heartbeat_at: latest_value(
            sessions
                .iter()
                .filter_map(|session| session.last_lease_observed_at.as_deref()),
        ),
        expires_at: earliest_value(
            active
                .iter()
                .filter_map(|session| session.expires_at.as_deref()),
        ),
        cleanup_obligation: None,
        blocking_identity_axes: vec!["legacy_principal_unproven".to_string()],
        authorized_actions: READ_ACTIONS
            .iter()
            .chain(std::iter::once(&"reconcile_plan"))
            .map(ToString::to_string)
            .collect(),
        recourse: PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
        observation_only: true,
    };
    record.lease_revision = lease_revision(&record);
    record
}

fn authorized_lease(
    state: &ServiceState,
    lease_id: &str,
    expected_revision: &str,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
) -> Result<ProfileLeaseRecord, ProfileLeaseError> {
    if !authenticated_authority_is_current(&state.service_principals, authority) {
        return Err(lease_error(
            ProfileLeaseFailureCode::AuthorityMismatch,
            lease_id,
        ));
    }
    let lease = inspect_profile_lease(state, lease_id, now)?;
    if lease.lease_revision != expected_revision {
        return Err(lease_error(
            ProfileLeaseFailureCode::RevisionMismatch,
            lease_id,
        ));
    }
    if lease.principal_id.as_deref() != Some(authority.principal_id.as_str())
        || lease.profile_id != authority.profile_id
        || lease.principal_provenance != Some(authority.provenance)
    {
        return Err(lease_error(
            ProfileLeaseFailureCode::AuthorityMismatch,
            lease_id,
        ));
    }
    Ok(lease)
}

fn require_action(lease: &ProfileLeaseRecord, action: &str) -> Result<(), ProfileLeaseError> {
    if lease
        .authorized_actions
        .iter()
        .any(|candidate| candidate == action)
    {
        Ok(())
    } else {
        Err(lease_error(
            ProfileLeaseFailureCode::ActionNotAuthorized,
            &lease.id,
        ))
    }
}

fn authority_for_binding(
    state: &ServiceState,
    binding: &crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding,
) -> Option<AuthenticatedServicePrincipal> {
    let capability = state
        .service_principals
        .profile_capabilities
        .get(&binding.capability_id)?;
    Some(AuthenticatedServicePrincipal {
        principal_id: binding.principal_id.clone(),
        profile_id: binding.profile_id.clone(),
        capability_id: binding.capability_id.clone(),
        capability_revision: capability.revision,
        provenance: binding.provenance,
    })
    .filter(|authority| authenticated_authority_is_current(&state.service_principals, authority))
}

fn active_subordinate_tabs<'a>(
    state: &'a ServiceState,
    authority: &'a AuthenticatedServicePrincipal,
    now: &'a str,
) -> impl Iterator<Item = &'a super::service_model::BrowserTab> + 'a {
    state.tabs.values().filter(move |tab| {
        let session_matches_profile = tab
            .owner_session_id
            .as_deref()
            .and_then(|session_id| state.sessions.get(session_id))
            .is_some_and(|session| {
                session.profile_id.as_deref() == Some(authority.profile_id.as_str())
                    && session.principal_id.as_deref() == Some(authority.principal_id.as_str())
            });
        tab.principal_id.as_deref() == Some(authority.principal_id.as_str())
            && tab.principal_provenance == Some(authority.provenance)
            && session_matches_profile
            && !matches!(tab.lifecycle, TabLifecycle::Closed | TabLifecycle::Crashed)
            && tab
                .work_lease_expires_at
                .as_deref()
                .is_none_or(|expires_at| expires_at > now)
    })
}

fn sessions_for_profile<'a>(
    state: &'a ServiceState,
    profile_id: &str,
) -> Vec<&'a super::service_model::BrowserSession> {
    let mut sessions = state
        .sessions
        .values()
        .filter(|session| session.profile_id.as_deref() == Some(profile_id))
        .collect::<Vec<_>>();
    sessions.sort_by(|left, right| left.id.cmp(&right.id));
    sessions
}

fn tabs_for_sessions(state: &ServiceState, session_ids: &[String]) -> Vec<String> {
    let session_ids = session_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut tab_ids = state
        .tabs
        .values()
        .filter(|tab| {
            tab.owner_session_id
                .as_deref()
                .is_some_and(|id| session_ids.contains(id))
        })
        .map(|tab| tab.id.clone())
        .collect::<Vec<_>>();
    tab_ids.sort();
    tab_ids
}

fn routes_for_browser(state: &ServiceState, browser_id: &str) -> Vec<String> {
    let mut route_ids = state
        .remote_view_routes
        .values()
        .filter(|route| route.browser_id.as_deref() == Some(browser_id))
        .map(|route| route.id.clone())
        .collect::<Vec<_>>();
    route_ids.sort();
    route_ids
}

fn cleanup_obligation_for_browser(state: &ServiceState, browser_id: &str) -> Option<String> {
    state
        .runtime_owner_registry
        .lifecycle_records
        .get(browser_id)
        .and_then(|record| {
            serde_json::to_value(record.cleanup_obligation_state)
                .ok()
                .and_then(|value| value.as_str().map(ToString::to_string))
        })
}

fn lease_mode(sessions: &[&super::service_model::BrowserSession]) -> String {
    if sessions
        .iter()
        .any(|session| session.lease == LeaseState::HumanTakeover)
    {
        "human_takeover".to_string()
    } else if sessions
        .iter()
        .any(|session| session.lease == LeaseState::Exclusive)
    {
        "exclusive".to_string()
    } else if sessions.is_empty() {
        "idle".to_string()
    } else {
        "shared".to_string()
    }
}

fn inactive_session(lease: LeaseState) -> bool {
    matches!(lease, LeaseState::Released | LeaseState::Expired)
}

fn inactive_or_expired(lease: LeaseState, expires_at: Option<&str>, now: &str) -> bool {
    inactive_session(lease) || expires_at.is_some_and(|expires_at| expires_at <= now)
}

fn lease_state_name(lease: LeaseState) -> &'static str {
    match lease {
        LeaseState::Shared => "shared",
        LeaseState::Exclusive => "exclusive",
        LeaseState::HumanTakeover => "human_takeover",
        LeaseState::Released => "released",
        LeaseState::Expired => "expired",
    }
}

fn latest_value<'a>(values: impl Iterator<Item = &'a str>) -> Option<String> {
    values.max().map(ToString::to_string)
}

fn earliest_value<'a>(values: impl Iterator<Item = &'a str>) -> Option<String> {
    values.min().map(ToString::to_string)
}

fn profile_lease_id(principal_id: &str, profile_identity: &str) -> String {
    format!(
        "profile-lease-v1:{}",
        digest_prefix(format!("{principal_id}\0{profile_identity}").as_bytes())
    )
}

fn lease_revision(record: &ProfileLeaseRecord) -> String {
    let mut projection = serde_json::to_value(record).expect("profile lease record must serialize");
    projection["leaseRevision"] = json!("");
    format!(
        "profile-lease-revision-v1:{}",
        digest_prefix(
            serde_json::to_vec(&projection)
                .expect("profile lease projection must encode")
                .as_slice()
        )
    )
}

fn seal_reconcile_plan(plan: &ProfileLeaseReconcilePlan, seal_key: &[u8]) -> String {
    let mut projection = serde_json::to_value(plan).expect("reconcile plan must serialize");
    projection["seal"] = json!("");
    let encoded = serde_json::to_vec(&projection).expect("reconcile plan must encode");
    let mut hasher = Sha256::new();
    hasher.update(b"agent-browser.profile-lease-reconcile-seal.v1\0");
    hasher.update(seal_key);
    hasher.update(b"\0");
    hasher.update(encoded);
    hasher.update(b"\0");
    hasher.update(seal_key);
    format!("sha256:{:x}", hasher.finalize())
}

fn digest_prefix(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))[..24].to_string()
}

fn lease_error(code: ProfileLeaseFailureCode, lease_id: &str) -> ProfileLeaseError {
    let label = match code {
        ProfileLeaseFailureCode::LeaseNotFound => "profile lease was not found",
        ProfileLeaseFailureCode::AuthorityMismatch => "profile lease authority does not match",
        ProfileLeaseFailureCode::RevisionMismatch => "profile lease revision does not match",
        ProfileLeaseFailureCode::ActionNotAuthorized => "profile lease action is not authorized",
        ProfileLeaseFailureCode::ActiveSubordinateWork => {
            "profile lease has active subordinate work"
        }
        ProfileLeaseFailureCode::PlanInvalid => "profile lease reconcile plan is invalid",
        ProfileLeaseFailureCode::PlanExpired => "profile lease reconcile plan is expired",
        ProfileLeaseFailureCode::BootEpochMismatch => {
            "profile lease reconcile boot epoch does not match"
        }
    };
    ProfileLeaseError {
        code,
        message: format!("{label}: {lease_id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{BrowserProfile, BrowserSession, BrowserTab};
    use crate::native::service_principal::{
        authenticate_profile_capability, bind_session_work_lease, bind_tab_work_lease,
        register_profile_capability, ServicePrincipalRegistrationRequest,
    };
    use crate::runtime_owner_transfer::{
        ProfileOwner, ProfileOwnerState, RuntimeOwnerPrincipalBinding, RuntimeOwnerRegistry,
    };
    use std::collections::BTreeMap;

    const CAPABILITY: &str = "profile-lease-test-capability-with-more-than-thirty-two-characters";
    const SEAL_KEY: &[u8] = b"profile-lease-test-seal-key-more-than-thirty-two-bytes";
    const NOW: &str = "2026-08-27T12:00:00Z";

    fn state_with_lease() -> (ServiceState, AuthenticatedServicePrincipal, String) {
        let profile_id = "odollo-fulfillment";
        let principal_id = "principal:odollo-fulfillment";
        let profile_path = "/tmp/agent-browser-p134/odollo-fulfillment";
        let digest = crate::runtime_profile::canonical_profile_identity_digest(
            std::path::Path::new(profile_path),
        )
        .unwrap();
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                profile_id.to_string(),
                BrowserProfile {
                    id: profile_id.to_string(),
                    user_data_dir: Some(profile_path.to_string()),
                    ..BrowserProfile::default()
                },
            )]),
            runtime_owner_registry: RuntimeOwnerRegistry::from_owner(ProfileOwner {
                owner_id: "owner-odollo".to_string(),
                profile_identity_digest: digest.clone(),
                state: ProfileOwnerState::Ready,
                owner_generation: 9,
                browser_id: "browser-odollo".to_string(),
                daemon_session_route: "session-odollo".to_string(),
                process_instance_digest: "process-odollo".to_string(),
                browser_family: "chrome".to_string(),
                cdp_endpoint_identity_digest: "cdp-odollo".to_string(),
                target_set_digest: "targets-odollo".to_string(),
                pending_transfer: None,
                last_transition: None,
            }),
            ..ServiceState::default()
        };
        let registered = register_profile_capability(
            &mut state.service_principals,
            ServicePrincipalRegistrationRequest {
                principal_id: principal_id.to_string(),
                display_name: Some("Odollo fulfillment".to_string()),
                profile_id: profile_id.to_string(),
                registered_at: Some(NOW.to_string()),
                registered_by: Some("test".to_string()),
            },
            CAPABILITY,
        )
        .unwrap();
        state
            .runtime_owner_registry
            .bind_principal_authority(RuntimeOwnerPrincipalBinding {
                principal_id: principal_id.to_string(),
                profile_id: profile_id.to_string(),
                profile_identity_digest: digest,
                capability_id: registered.capability.capability_id,
                provenance: ServicePrincipalProvenance::RegisteredCapability,
                owner_generation: 9,
            })
            .unwrap();
        let authority = authenticate_profile_capability(
            &state.service_principals,
            CAPABILITY,
            Some(profile_id),
        )
        .unwrap();
        state.sessions.insert(
            "session-odollo".to_string(),
            BrowserSession {
                id: "session-odollo".to_string(),
                profile_id: Some(profile_id.to_string()),
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        );
        bind_session_work_lease(
            &mut state,
            "session-odollo",
            &authority,
            "2026-08-27T13:00:00Z".to_string(),
        )
        .unwrap();
        let lease_id = profile_leases_for_state(&state, NOW)[0].id.clone();
        (state, authority, lease_id)
    }

    #[test]
    fn projection_exposes_exact_owner_and_authorized_recourse() {
        let (state, _, lease_id) = state_with_lease();
        let lease = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        assert_eq!(lease.state, "active");
        assert_eq!(lease.owner_generation, Some(9));
        assert_eq!(
            lease.recourse,
            PrincipalContinuityRecourse::RejoinOwnedBrowser
        );
        assert!(lease.authorized_actions.contains(&"rejoin".to_string()));
        assert!(lease.authorized_actions.contains(&"renew".to_string()));
        assert!(lease.authorized_actions.contains(&"release".to_string()));
        assert!(!lease.observation_only);

        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-profile-lease-record.v1.schema.json"
        ))
        .unwrap();
        let value = serde_json::to_value(&lease).unwrap();
        for field in schema["required"].as_array().unwrap() {
            assert!(
                value.get(field.as_str().unwrap()).is_some(),
                "profile lease record omitted required field {field}"
            );
        }
        assert_eq!(
            value.as_object().unwrap().len(),
            schema["required"].as_array().unwrap().len()
        );
    }

    #[test]
    fn legacy_profile_is_observation_only_and_doctor_reports_it() {
        let state = ServiceState {
            sessions: BTreeMap::from([(
                "legacy".to_string(),
                BrowserSession {
                    id: "legacy".to_string(),
                    profile_id: Some("legacy-profile".to_string()),
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };
        let leases = profile_leases_for_state(&state, NOW);
        assert_eq!(leases.len(), 1);
        assert!(leases[0].observation_only);
        assert!(!leases[0]
            .authorized_actions
            .contains(&"release".to_string()));
        let doctor = doctor_profile_leases(&state, NOW);
        assert!(!doctor.healthy);
        assert_eq!(doctor.findings[0].code, "legacy_principal_unproven");
    }

    #[test]
    fn renew_uses_revision_cas_and_release_refuses_active_tab() {
        let (mut state, authority, lease_id) = state_with_lease();
        let initial = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        let renewed = renew_profile_lease(
            &mut state,
            &lease_id,
            &initial.lease_revision,
            &authority,
            NOW,
            "2026-08-27T14:00:00Z",
        )
        .unwrap();
        assert_ne!(renewed.lease_revision, initial.lease_revision);
        let stale = renew_profile_lease(
            &mut state,
            &lease_id,
            &initial.lease_revision,
            &authority,
            NOW,
            "2026-08-27T15:00:00Z",
        )
        .unwrap_err();
        assert_eq!(stale.code, ProfileLeaseFailureCode::RevisionMismatch);

        state.tabs.insert(
            "tab-odollo".to_string(),
            BrowserTab {
                id: "tab-odollo".to_string(),
                browser_id: "browser-odollo".to_string(),
                owner_session_id: Some("session-odollo".to_string()),
                lifecycle: TabLifecycle::Ready,
                ..BrowserTab::default()
            },
        );
        bind_tab_work_lease(
            &mut state,
            "tab-odollo",
            &authority,
            "2026-08-27T13:30:00Z".to_string(),
        )
        .unwrap();
        let with_tab = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        let error = release_profile_lease(
            &mut state,
            &lease_id,
            &with_tab.lease_revision,
            &authority,
            NOW,
        )
        .unwrap_err();
        assert_eq!(error.code, ProfileLeaseFailureCode::ActionNotAuthorized);
    }

    #[test]
    fn reconcile_plan_is_boot_bound_sealed_and_idempotent() {
        let (mut state, authority, lease_id) = state_with_lease();
        let session = state.sessions.get_mut("session-odollo").unwrap();
        session.lease = LeaseState::Expired;
        session.expires_at = Some("2026-08-27T11:00:00Z".to_string());
        let stale = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        let blocked = plan_profile_lease_reconciliation(
            &state,
            &lease_id,
            &stale.lease_revision,
            &authority,
            NOW,
            "2026-08-27T12:05:00Z",
            None,
            "idempotency-blocked".to_string(),
            SEAL_KEY,
        )
        .unwrap();
        assert!(!blocked.effect_capable);
        assert_eq!(blocked.blocked_reasons, vec!["boot_epoch_unavailable"]);

        let plan = plan_profile_lease_reconciliation(
            &state,
            &lease_id,
            &stale.lease_revision,
            &authority,
            NOW,
            "2026-08-27T12:05:00Z",
            Some("boot-epoch-1".to_string()),
            "idempotency-1".to_string(),
            SEAL_KEY,
        )
        .unwrap();
        assert!(plan.effect_capable);
        let receipt = apply_profile_lease_reconciliation(
            &mut state,
            &plan,
            &authority,
            "2026-08-27T12:01:00Z",
            Some("boot-epoch-1"),
            SEAL_KEY,
        )
        .unwrap();
        assert!(!receipt.replayed);
        let replay = apply_profile_lease_reconciliation(
            &mut state,
            &plan,
            &authority,
            "2026-08-27T12:02:00Z",
            Some("boot-epoch-1"),
            SEAL_KEY,
        )
        .unwrap();
        assert!(replay.replayed);
    }
}
