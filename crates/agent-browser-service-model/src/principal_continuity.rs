//! Pure principal continuity, legacy migration projections, and subordinate work transitions.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::{BrowserSession, BrowserTab, LeaseState, ServiceState};
use agent_browser_lease_authority::{
    AuthenticatedServicePrincipal, ServicePrincipalError, ServicePrincipalFailureCode,
    ServicePrincipalProvenance, ServicePrincipalState, ServiceProfileCapabilityState,
};

/// Available recourse for preserving an authenticated principal's continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalContinuityRecourse {
    ContinueWithActiveClaim,
    ContinueWithSelfDeclaredAccess,
    RejoinOwnedBrowser,
    ReplaceStaleSamePrincipalSession,
    WaitForForeignPrincipal,
    ReconcilePrincipalIdentity,
}

impl PrincipalContinuityRecourse {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContinueWithActiveClaim => "continue_with_active_claim",
            Self::ContinueWithSelfDeclaredAccess => "continue_with_self_declared_access",
            Self::RejoinOwnedBrowser => "rejoin_owned_browser",
            Self::ReplaceStaleSamePrincipalSession => "replace_stale_same_principal_session",
            Self::WaitForForeignPrincipal => "wait_for_foreign_principal",
            Self::ReconcilePrincipalIdentity => "reconcile_principal_identity",
        }
    }
}

/// Deterministic recourse derived from current principal and retained owner/work state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrincipalContinuityDecision {
    pub recourse: PrincipalContinuityRecourse,
    pub requester_principal_id: String,
    pub profile_id: String,
    pub holder_session_ids: Vec<String>,
    pub holder_principal_ids: Vec<String>,
    pub effect_capable: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyPrincipalMigrationDisposition {
    AlreadyPrincipalBound,
    PrincipalBindingAvailable,
    UnprovenPrincipal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacySessionPrincipalMigrationPlan {
    pub session_id: String,
    pub profile_id: Option<String>,
    pub candidate_principal_id: Option<String>,
    pub disposition: LegacyPrincipalMigrationDisposition,
    pub observation_only: bool,
    pub recourse: PrincipalContinuityRecourse,
    pub reasons: Vec<String>,
}

pub(crate) fn authenticated_session_work_authority(
    state: &ServiceState,
    session_id: &str,
    now: &str,
) -> Option<AuthenticatedServicePrincipal> {
    let session = state.sessions.get(session_id)?;
    let principal_id = session.principal_id.as_deref()?;
    let profile_id = session.profile_id.as_deref()?;
    if session.principal_provenance != Some(ServicePrincipalProvenance::RegisteredCapability)
        || matches!(session.lease, LeaseState::Released | LeaseState::Expired)
        || session
            .expires_at
            .as_deref()
            .is_none_or(|expiry| expiry <= now)
        || session.work_lease_id.as_deref().is_none_or(str::is_empty)
        || session.work_lease_revision == 0
    {
        return None;
    }
    let matching = state
        .runtime_owner_registry
        .principal_bindings()
        .values()
        .filter(|binding| {
            binding.principal_id == principal_id
                && binding.profile_id == profile_id
                && binding.provenance == ServicePrincipalProvenance::RegisteredCapability
        })
        .collect::<Vec<_>>();
    let [binding] = matching.as_slice() else {
        return None;
    };
    let capability = state.profile_capability(&binding.capability_id)?;
    let authority = AuthenticatedServicePrincipal {
        principal_id: principal_id.to_string(),
        profile_id: profile_id.to_string(),
        capability_id: binding.capability_id.clone(),
        capability_revision: capability.revision,
        provenance: binding.provenance,
    };
    state
        .authenticated_authority_is_current(&authority)
        .then_some(authority)
}

pub(crate) fn bind_session_work_lease(
    state: &mut ServiceState,
    session_id: &str,
    authority: &AuthenticatedServicePrincipal,
    expires_at: String,
    boot_epoch: Option<String>,
) -> Result<BrowserSession, ServicePrincipalError> {
    if !state.authenticated_authority_is_current(authority) {
        return Err(principal_error(
            ServicePrincipalFailureCode::CapabilityMismatch,
        ));
    }
    let session = state
        .sessions
        .get_mut(session_id)
        .ok_or_else(|| principal_error(ServicePrincipalFailureCode::WorkLeaseConflict))?;
    if session.profile_id.as_deref() != Some(authority.profile_id.as_str()) {
        return Err(principal_error(
            ServicePrincipalFailureCode::ProfileMismatch,
        ));
    }
    if session
        .principal_id
        .as_deref()
        .is_some_and(|principal_id| principal_id != authority.principal_id)
    {
        return Err(principal_error(
            ServicePrincipalFailureCode::WorkLeaseConflict,
        ));
    }
    session.principal_id = Some(authority.principal_id.clone());
    session.boot_epoch = boot_epoch;
    session.principal_provenance = Some(authority.provenance);
    session.work_lease_id = Some(work_lease_id(
        "session",
        &authority.principal_id,
        session_id,
        &authority.profile_id,
    ));
    session.work_lease_revision = session.work_lease_revision.saturating_add(1).max(1);
    session.expires_at = Some(expires_at);
    Ok(session.clone())
}

pub(crate) fn bind_tab_work_lease(
    state: &mut ServiceState,
    tab_id: &str,
    authority: &AuthenticatedServicePrincipal,
    expires_at: String,
) -> Result<BrowserTab, ServicePrincipalError> {
    if !state.authenticated_authority_is_current(authority) {
        return Err(principal_error(
            ServicePrincipalFailureCode::CapabilityMismatch,
        ));
    }
    let owner_session_id = state
        .tabs
        .get(tab_id)
        .and_then(|tab| tab.owner_session_id.clone())
        .ok_or_else(|| principal_error(ServicePrincipalFailureCode::WorkLeaseConflict))?;
    let session = state
        .sessions
        .get(&owner_session_id)
        .ok_or_else(|| principal_error(ServicePrincipalFailureCode::WorkLeaseConflict))?;
    if session.principal_id.as_deref() != Some(authority.principal_id.as_str())
        || session.profile_id.as_deref() != Some(authority.profile_id.as_str())
    {
        return Err(principal_error(
            ServicePrincipalFailureCode::WorkLeaseConflict,
        ));
    }
    let tab = state
        .tabs
        .get_mut(tab_id)
        .ok_or_else(|| principal_error(ServicePrincipalFailureCode::WorkLeaseConflict))?;
    let session_id = tab
        .owner_session_id
        .as_deref()
        .ok_or_else(|| principal_error(ServicePrincipalFailureCode::WorkLeaseConflict))?;
    debug_assert_eq!(session_id, owner_session_id);
    tab.principal_id = Some(authority.principal_id.clone());
    tab.principal_provenance = Some(authority.provenance);
    tab.work_lease_id = Some(work_lease_id(
        "tab",
        &authority.principal_id,
        tab_id,
        &authority.profile_id,
    ));
    tab.work_lease_revision = tab.work_lease_revision.saturating_add(1).max(1);
    tab.work_lease_expires_at = Some(expires_at);
    Ok(tab.clone())
}

pub(crate) fn principal_continuity_decision(
    state: &ServiceState,
    authority: &AuthenticatedServicePrincipal,
) -> PrincipalContinuityDecision {
    let mut reasons = Vec::new();
    if !state.authenticated_authority_is_current(authority) {
        reasons.push("principal_capability_not_current".to_string());
        return continuity_decision(
            authority,
            PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
            Vec::new(),
            Vec::new(),
            false,
            reasons,
        );
    }

    let owner_bindings = state
        .runtime_owner_registry
        .principal_bindings()
        .values()
        .filter(|binding| binding.profile_id == authority.profile_id)
        .collect::<Vec<_>>();
    if owner_bindings.len() != 1
        || !state
            .runtime_owner_registry
            .principal_binding_is_current(owner_bindings.first().copied())
    {
        reasons.push("runtime_owner_principal_binding_not_current".to_string());
        return continuity_decision(
            authority,
            PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
            Vec::new(),
            Vec::new(),
            false,
            reasons,
        );
    }
    let owner_binding = owner_bindings[0];
    if owner_binding.principal_id != authority.principal_id
        || owner_binding.capability_id != authority.capability_id
        || owner_binding.provenance != authority.provenance
    {
        reasons.push("runtime_owner_principal_mismatch".to_string());
        return continuity_decision(
            authority,
            PrincipalContinuityRecourse::WaitForForeignPrincipal,
            Vec::new(),
            vec![owner_binding.principal_id.clone()],
            false,
            reasons,
        );
    }

    let mut active_holders = state
        .sessions
        .values()
        .filter(|session| {
            session.profile_id.as_deref() == Some(authority.profile_id.as_str())
                && matches!(
                    session.lease,
                    LeaseState::Exclusive | LeaseState::HumanTakeover
                )
        })
        .collect::<Vec<_>>();
    active_holders.sort_by(|left, right| left.id.cmp(&right.id));
    let holder_session_ids = active_holders
        .iter()
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    let holder_principal_ids = active_holders
        .iter()
        .filter_map(|session| session.principal_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if active_holders
        .iter()
        .any(|session| session.principal_id.is_none())
    {
        reasons.push("legacy_holder_principal_unproven".to_string());
        return continuity_decision(
            authority,
            PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
            holder_session_ids,
            holder_principal_ids,
            false,
            reasons,
        );
    }
    if holder_principal_ids
        .iter()
        .any(|principal_id| principal_id != &authority.principal_id)
    {
        reasons.push("foreign_principal_holds_profile".to_string());
        return continuity_decision(
            authority,
            PrincipalContinuityRecourse::WaitForForeignPrincipal,
            holder_session_ids,
            holder_principal_ids,
            false,
            reasons,
        );
    }
    if !active_holders.is_empty() {
        reasons.push("same_principal_retained_holder".to_string());
        return continuity_decision(
            authority,
            PrincipalContinuityRecourse::RejoinOwnedBrowser,
            holder_session_ids,
            holder_principal_ids,
            true,
            reasons,
        );
    }

    let stale_same_principal = state.sessions.values().any(|session| {
        session.profile_id.as_deref() == Some(authority.profile_id.as_str())
            && session.principal_id.as_deref() == Some(authority.principal_id.as_str())
            && matches!(session.lease, LeaseState::Released | LeaseState::Expired)
    });
    if stale_same_principal {
        reasons.push("stale_same_principal_session".to_string());
        continuity_decision(
            authority,
            PrincipalContinuityRecourse::ReplaceStaleSamePrincipalSession,
            Vec::new(),
            vec![authority.principal_id.clone()],
            true,
            reasons,
        )
    } else {
        reasons.push("principal_owner_ready_without_session".to_string());
        continuity_decision(
            authority,
            PrincipalContinuityRecourse::RejoinOwnedBrowser,
            Vec::new(),
            vec![authority.principal_id.clone()],
            true,
            reasons,
        )
    }
}

pub(crate) fn plan_legacy_session_principal_migration(
    state: &ServiceState,
) -> Vec<LegacySessionPrincipalMigrationPlan> {
    let mut plans = state
        .sessions
        .values()
        .map(|session| legacy_session_plan(state, session))
        .collect::<Vec<_>>();
    plans.sort_by(|left, right| left.session_id.cmp(&right.session_id));
    plans
}

fn legacy_session_plan(
    state: &ServiceState,
    session: &BrowserSession,
) -> LegacySessionPrincipalMigrationPlan {
    if let Some(principal_id) = session.principal_id.clone() {
        if session.profile_id.as_deref().is_some_and(|profile_id| {
            verified_principal_owner_binding(state, profile_id, &principal_id).is_some()
        }) && session.principal_provenance
            == Some(ServicePrincipalProvenance::RegisteredCapability)
        {
            return LegacySessionPrincipalMigrationPlan {
                session_id: session.id.clone(),
                profile_id: session.profile_id.clone(),
                candidate_principal_id: Some(principal_id),
                disposition: LegacyPrincipalMigrationDisposition::AlreadyPrincipalBound,
                observation_only: false,
                recourse: PrincipalContinuityRecourse::RejoinOwnedBrowser,
                reasons: vec!["session_principal_and_current_owner_authority_agree".to_string()],
            };
        }
        return LegacySessionPrincipalMigrationPlan {
            session_id: session.id.clone(),
            profile_id: session.profile_id.clone(),
            candidate_principal_id: Some(principal_id),
            disposition: LegacyPrincipalMigrationDisposition::UnprovenPrincipal,
            observation_only: true,
            recourse: PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
            reasons: vec!["session_principal_lacks_current_owner_authority".to_string()],
        };
    }

    let owner_bindings = session
        .profile_id
        .as_deref()
        .map(|profile_id| {
            state
                .runtime_owner_registry
                .principal_bindings()
                .values()
                .filter(|binding| binding.profile_id == profile_id)
                .filter(|binding| {
                    verified_principal_owner_binding(state, profile_id, &binding.principal_id)
                        == Some(*binding)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if owner_bindings.len() == 1 {
        return LegacySessionPrincipalMigrationPlan {
            session_id: session.id.clone(),
            profile_id: session.profile_id.clone(),
            candidate_principal_id: Some(owner_bindings[0].principal_id.clone()),
            disposition: LegacyPrincipalMigrationDisposition::PrincipalBindingAvailable,
            observation_only: true,
            recourse: PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
            reasons: vec![
                "authenticated_registration_capability_and_owner_binding_agree".to_string(),
                "staged_migration_requires_explicit_commit".to_string(),
            ],
        };
    }

    LegacySessionPrincipalMigrationPlan {
        session_id: session.id.clone(),
        profile_id: session.profile_id.clone(),
        candidate_principal_id: None,
        disposition: LegacyPrincipalMigrationDisposition::UnprovenPrincipal,
        observation_only: true,
        recourse: PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
        reasons: vec!["legacy_labels_are_not_principal_authority".to_string()],
    }
}

fn verified_principal_owner_binding<'a>(
    state: &'a ServiceState,
    profile_id: &str,
    principal_id: &str,
) -> Option<&'a agent_browser_lease_authority::RuntimeOwnerPrincipalBinding> {
    let matching = state
        .runtime_owner_registry
        .principal_bindings()
        .values()
        .filter(|binding| {
            binding.profile_id == profile_id
                && binding.principal_id == principal_id
                && binding.provenance == ServicePrincipalProvenance::RegisteredCapability
                && state
                    .runtime_owner_registry
                    .principal_binding_is_current(Some(binding))
                && state
                    .service_principal(principal_id)
                    .is_some_and(|principal| {
                        principal.state == ServicePrincipalState::Active
                            && principal.provenance
                                == ServicePrincipalProvenance::RegisteredCapability
                    })
                && state
                    .profile_capability(&binding.capability_id)
                    .is_some_and(|capability| {
                        capability.state == ServiceProfileCapabilityState::Active
                            && capability.principal_id == binding.principal_id
                            && capability.profile_id == binding.profile_id
                    })
        })
        .collect::<Vec<_>>();
    if matching.len() == 1 {
        Some(matching[0])
    } else {
        None
    }
}

fn continuity_decision(
    authority: &AuthenticatedServicePrincipal,
    recourse: PrincipalContinuityRecourse,
    holder_session_ids: Vec<String>,
    holder_principal_ids: Vec<String>,
    effect_capable: bool,
    mut reasons: Vec<String>,
) -> PrincipalContinuityDecision {
    reasons.sort();
    reasons.dedup();
    PrincipalContinuityDecision {
        recourse,
        requester_principal_id: authority.principal_id.clone(),
        profile_id: authority.profile_id.clone(),
        holder_session_ids,
        holder_principal_ids,
        effect_capable,
        reasons,
    }
}

fn work_lease_id(kind: &str, principal_id: &str, resource_id: &str, profile_id: &str) -> String {
    let canonical = format!("{kind}\0{principal_id}\0{resource_id}\0{profile_id}");
    format!(
        "{kind}-work-lease-v1:{}",
        digest_prefix(canonical.as_bytes())
    )
}

fn digest_prefix(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))[..24].to_string()
}

fn principal_error(code: ServicePrincipalFailureCode) -> ServicePrincipalError {
    let message = match code {
        ServicePrincipalFailureCode::InvalidRegistration => {
            "principal registration or capability is invalid"
        }
        ServicePrincipalFailureCode::RegistrationConflict => {
            "principal or profile capability conflicts with current registration"
        }
        ServicePrincipalFailureCode::RegistryRevisionMismatch => {
            "service principal registry revision changed"
        }
        ServicePrincipalFailureCode::CapabilityRotationConflict => {
            "profile capability rotation does not match the one active grant"
        }
        ServicePrincipalFailureCode::CapabilityMissing => "profile capability is missing",
        ServicePrincipalFailureCode::CapabilityMismatch => {
            "profile capability does not match a current registered grant"
        }
        ServicePrincipalFailureCode::CapabilityRevoked => "profile capability is revoked",
        ServicePrincipalFailureCode::PrincipalUnavailable => "registered principal is unavailable",
        ServicePrincipalFailureCode::ProfileMismatch => {
            "profile capability does not authorize the requested profile"
        }
        ServicePrincipalFailureCode::WorkLeaseConflict => {
            "subordinate work lease conflicts with current principal authority"
        }
    };
    ServicePrincipalError { code, message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_lease_authority::{
        ProfileOwner, ProfileOwnerState, RuntimeOwnerPrincipalBinding, RuntimeOwnerRegistry,
        ServicePrincipalRegistrationRequest,
    };
    use serde_json::{json, Value};

    const TOKEN: &str = "synthetic-continuity-capability-more-than-thirty-two-characters";
    const NOW: &str = "2026-09-17T12:00:00Z";
    const EXPIRY: &str = "2026-09-17T12:05:00Z";
    // SHA-256 of "profile", matching the Lease Authority owner fixture's evidence shape.
    const PROFILE_DIGEST: &str = "1900eab6c028483d7126599ee6f50de0d27907b5c65fa90524580b4b0f9852b0";

    fn fixture() -> (ServiceState, AuthenticatedServicePrincipal) {
        let mut state = ServiceState::default();
        let registered = state
            .register_profile_capability(
                ServicePrincipalRegistrationRequest {
                    principal_id: "principal:test".to_string(),
                    display_name: None,
                    profile_id: "profile:test".to_string(),
                    registered_at: Some(NOW.to_string()),
                    registered_by: None,
                },
                TOKEN,
            )
            .unwrap();
        state.runtime_owner_registry = RuntimeOwnerRegistry::from_owner(ProfileOwner {
            owner_id: "owner".to_string(),
            profile_identity_digest: PROFILE_DIGEST.to_string(),
            state: ProfileOwnerState::Ready,
            owner_generation: 7,
            browser_id: "browser".to_string(),
            daemon_session_route: "session".to_string(),
            process_instance_digest: "process".to_string(),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: "cdp".to_string(),
            target_set_digest: "targets".to_string(),
            pending_transfer: None,
            last_transition: None,
        });
        state
            .runtime_owner_registry
            .bind_principal_authority(RuntimeOwnerPrincipalBinding {
                principal_id: "principal:test".to_string(),
                profile_id: "profile:test".to_string(),
                profile_identity_digest: PROFILE_DIGEST.to_string(),
                capability_id: registered.capability.capability_id,
                provenance: ServicePrincipalProvenance::RegisteredCapability,
                owner_generation: 7,
            })
            .unwrap();
        state.sessions.insert(
            "session".to_string(),
            BrowserSession {
                id: "session".to_string(),
                profile_id: Some("profile:test".to_string()),
                lease: LeaseState::Shared,
                ..BrowserSession::default()
            },
        );
        state.tabs.insert(
            "tab".to_string(),
            BrowserTab {
                id: "tab".to_string(),
                browser_id: "browser".to_string(),
                owner_session_id: Some("session".to_string()),
                ..BrowserTab::default()
            },
        );
        let authority = state.authenticate_profile_capability(TOKEN, None).unwrap();
        (state, authority)
    }

    // Malformed or stale records are compatibility fixtures, never production mutations.
    fn edit_registry_fixture(state: &mut ServiceState, edit: impl FnOnce(&mut Value)) {
        let mut wire = serde_json::to_value(&state.runtime_owner_registry).unwrap();
        edit(&mut wire);
        state.runtime_owner_registry = serde_json::from_value(wire).unwrap();
    }

    fn bind_fixture_session(state: &mut ServiceState, authority: &AuthenticatedServicePrincipal) {
        state
            .bind_session_work_lease(
                "session",
                authority,
                EXPIRY.to_string(),
                Some("boot".to_string()),
            )
            .unwrap();
    }

    #[test]
    fn continuity_records_keep_wire_names_nulls_and_unknown_field_tolerance() {
        let (state, authority) = fixture();
        let decision = state.principal_continuity_decision(&authority);
        let mut wire = serde_json::to_value(&decision).unwrap();
        assert!(wire.get("requesterPrincipalId").is_some());
        assert!(wire.get("holderSessionIds").is_some());
        assert!(wire.get("holderPrincipalIds").is_some());
        assert!(wire.get("effectCapable").is_some());
        wire["futureField"] = json!(true);
        assert_eq!(
            serde_json::from_value::<PrincipalContinuityDecision>(wire).unwrap(),
            decision
        );
        for (disposition, name) in [
            (
                LegacyPrincipalMigrationDisposition::AlreadyPrincipalBound,
                "already_principal_bound",
            ),
            (
                LegacyPrincipalMigrationDisposition::PrincipalBindingAvailable,
                "principal_binding_available",
            ),
            (
                LegacyPrincipalMigrationDisposition::UnprovenPrincipal,
                "unproven_principal",
            ),
        ] {
            assert_eq!(serde_json::to_value(disposition).unwrap(), json!(name));
            let plan = LegacySessionPrincipalMigrationPlan {
                session_id: "legacy".to_string(),
                profile_id: None,
                candidate_principal_id: None,
                disposition,
                observation_only: true,
                recourse: PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
                reasons: vec![],
            };
            let mut wire = serde_json::to_value(&plan).unwrap();
            assert_eq!(wire.get("profileId"), Some(&Value::Null));
            assert_eq!(wire.get("candidatePrincipalId"), Some(&Value::Null));
            assert_eq!(wire["observationOnly"], true);
            wire["futureField"] = json!(true);
            assert_eq!(
                serde_json::from_value::<LegacySessionPrincipalMigrationPlan>(wire).unwrap(),
                plan
            );
        }
    }

    #[test]
    fn session_authority_requires_exact_work_but_not_current_owner_generation() {
        let (mut state, authority) = fixture();
        bind_fixture_session(&mut state, &authority);
        assert_eq!(
            state.authenticated_session_work_authority("missing", NOW),
            None
        );
        let before = state.clone();
        assert_eq!(
            state.authenticated_session_work_authority("session", NOW),
            Some(authority.clone())
        );
        assert_eq!(state, before);
        for (field, value) in [
            ("principalId", Value::Null),
            ("profileId", Value::Null),
            ("principalProvenance", json!("unproven_legacy")),
            ("lease", json!("released")),
            ("lease", json!("expired")),
            ("expiresAt", Value::Null),
            ("expiresAt", json!(NOW)),
            ("workLeaseId", Value::Null),
            ("workLeaseId", json!("")),
            ("workLeaseRevision", json!(0)),
        ] {
            let mut changed = state.clone();
            let mut session = serde_json::to_value(&changed.sessions["session"]).unwrap();
            session[field] = value;
            changed.sessions.insert(
                "session".to_string(),
                serde_json::from_value(session).unwrap(),
            );
            assert_eq!(
                changed.authenticated_session_work_authority("session", NOW),
                None,
                "{field}"
            );
        }
        // Equivalent instants with different offsets retain the old lexical comparison.
        state.sessions.get_mut("session").unwrap().expires_at =
            Some("2026-09-17T13:00:00+01:00".to_string());
        edit_registry_fixture(&mut state, |wire| {
            wire["owners"][PROFILE_DIGEST]["ownerGeneration"] = json!(8)
        });
        assert_eq!(
            state.authenticated_session_work_authority("session", NOW),
            Some(authority.clone())
        );
        assert_eq!(
            state.principal_continuity_decision(&authority).reasons,
            vec!["runtime_owner_principal_binding_not_current"]
        );
        edit_registry_fixture(&mut state, |wire| {
            wire["principalBindings"]["duplicate"] =
                wire["principalBindings"][PROFILE_DIGEST].clone();
        });
        assert_eq!(
            state.authenticated_session_work_authority("session", NOW),
            None
        );
    }

    #[test]
    fn session_authority_rejects_missing_binding_capability_and_revoked_registration() {
        let (mut state, authority) = fixture();
        bind_fixture_session(&mut state, &authority);
        let mut missing = state.clone();
        missing.runtime_owner_registry = RuntimeOwnerRegistry::default();
        assert_eq!(
            missing.authenticated_session_work_authority("session", NOW),
            None
        );
        edit_registry_fixture(&mut missing, |wire| {
            wire["principalBindings"] = json!({"digest": {
                "principalId": "principal:test", "profileId": "profile:test", "profileIdentityDigest": "digest",
                "capabilityId": "missing", "provenance": "registered_capability", "ownerGeneration": 7
            }});
        });
        assert_eq!(
            missing.authenticated_session_work_authority("session", NOW),
            None
        );
        state
            .service_principals
            .profile_capabilities
            .get_mut(&authority.capability_id)
            .unwrap()
            .state = ServiceProfileCapabilityState::Revoked;
        assert_eq!(
            state.authenticated_session_work_authority("session", NOW),
            None
        );
    }

    #[test]
    fn continuity_preserves_precedence_holder_order_and_expiry_independence() {
        let (mut state, authority) = fixture();
        assert_eq!(
            state.principal_continuity_decision(&authority).reasons,
            vec!["principal_owner_ready_without_session"]
        );
        bind_fixture_session(&mut state, &authority);
        state.sessions.get_mut("session").unwrap().lease = LeaseState::Expired;
        assert_eq!(
            state.principal_continuity_decision(&authority).recourse,
            PrincipalContinuityRecourse::ReplaceStaleSamePrincipalSession
        );
        let mut holder = state.sessions["session"].clone();
        holder.id = "z-holder".to_string();
        holder.lease = LeaseState::Exclusive;
        holder.expires_at = Some("2000-01-01T00:00:00Z".to_string());
        state
            .sessions
            .insert("a-map-key".to_string(), holder.clone());
        holder.id = "a-holder".to_string();
        holder.lease = LeaseState::HumanTakeover;
        state.sessions.insert("z-map-key".to_string(), holder);
        let before = state.clone();
        let decision = state.principal_continuity_decision(&authority);
        assert_eq!(decision.holder_session_ids, vec!["a-holder", "z-holder"]);
        assert_eq!(decision.holder_principal_ids, vec!["principal:test"]);
        assert_eq!(decision.reasons, vec!["same_principal_retained_holder"]);
        assert!(decision.effect_capable);
        assert_eq!(state, before);
        state.sessions.get_mut("a-map-key").unwrap().principal_id =
            Some("principal:foreign".to_string());
        assert_eq!(
            state.principal_continuity_decision(&authority).reasons,
            vec!["foreign_principal_holds_profile"]
        );
        state.sessions.get_mut("z-map-key").unwrap().principal_id = None;
        assert_eq!(
            state.principal_continuity_decision(&authority).reasons,
            vec!["legacy_holder_principal_unproven"]
        );
        edit_registry_fixture(&mut state, |wire| {
            wire["principalBindings"][PROFILE_DIGEST]["principalId"] = json!("principal:foreign")
        });
        let foreign = state.principal_continuity_decision(&authority);
        assert_eq!(foreign.reasons, vec!["runtime_owner_principal_mismatch"]);
        assert_eq!(
            foreign.recourse,
            PrincipalContinuityRecourse::WaitForForeignPrincipal
        );
        assert!(!foreign.effect_capable);
        state.runtime_owner_registry = RuntimeOwnerRegistry::default();
        assert_eq!(
            state.principal_continuity_decision(&authority).reasons,
            vec!["runtime_owner_principal_binding_not_current"]
        );
        let mut stale = authority;
        stale.capability_revision += 1;
        assert_eq!(
            state.principal_continuity_decision(&stale).reasons,
            vec!["principal_capability_not_current"]
        );
    }

    #[test]
    fn migration_preserves_dispositions_order_and_observation_only_candidates() {
        let (mut state, authority) = fixture();
        let available = state.plan_legacy_session_principal_migration();
        assert_eq!(
            available[0].disposition,
            LegacyPrincipalMigrationDisposition::PrincipalBindingAvailable
        );
        assert!(available[0].observation_only);
        assert_eq!(
            available[0].reasons,
            vec![
                "authenticated_registration_capability_and_owner_binding_agree",
                "staged_migration_requires_explicit_commit"
            ]
        );
        bind_fixture_session(&mut state, &authority);
        let bound = state.plan_legacy_session_principal_migration();
        assert_eq!(
            bound[0].disposition,
            LegacyPrincipalMigrationDisposition::AlreadyPrincipalBound
        );
        assert!(!bound[0].observation_only);
        state.sessions.insert(
            "z-key".to_string(),
            BrowserSession {
                id: "a-legacy".to_string(),
                profile_id: Some("unknown".to_string()),
                service_name: Some(authority.principal_id.clone()),
                agent_name: Some(authority.principal_id.clone()),
                ..BrowserSession::default()
            },
        );
        let before = state.clone();
        let plans = state.plan_legacy_session_principal_migration();
        assert_eq!(
            plans
                .iter()
                .map(|plan| plan.session_id.as_str())
                .collect::<Vec<_>>(),
            vec!["a-legacy", "session"]
        );
        assert_eq!(
            plans[0].disposition,
            LegacyPrincipalMigrationDisposition::UnprovenPrincipal
        );
        assert_eq!(plans[0].candidate_principal_id, None);
        assert!(plans[0].observation_only);
        assert_eq!(state, before);
        edit_registry_fixture(&mut state, |wire| {
            wire["owners"][PROFILE_DIGEST]["ownerGeneration"] = json!(8)
        });
        let stale = state.plan_legacy_session_principal_migration();
        assert_eq!(
            stale[1].disposition,
            LegacyPrincipalMigrationDisposition::UnprovenPrincipal
        );
        assert_eq!(
            stale[1].reasons,
            vec!["session_principal_lacks_current_owner_authority"]
        );
        state.sessions.get_mut("session").unwrap().principal_id = None;
        assert_eq!(
            state.plan_legacy_session_principal_migration()[1].candidate_principal_id,
            None
        );
    }

    #[test]
    fn subordinate_mutations_have_exact_deltas_ids_boot_inputs_and_saturation() {
        let (mut state, authority) = fixture();
        let mut expected = state.clone();
        let expected_session = expected.sessions.get_mut("session").unwrap();
        expected_session.principal_id = Some(authority.principal_id.clone());
        expected_session.principal_provenance = Some(authority.provenance);
        expected_session.boot_epoch = Some("explicit-boot".to_string());
        expected_session.work_lease_id =
            Some("session-work-lease-v1:006453e3613c5596e2593e80".to_string());
        expected_session.work_lease_revision = 1;
        expected_session.expires_at = Some(EXPIRY.to_string());
        let result = state
            .bind_session_work_lease(
                "session",
                &authority,
                EXPIRY.to_string(),
                Some("explicit-boot".to_string()),
            )
            .unwrap();
        assert_eq!(result, expected.sessions["session"]);
        assert_eq!(state, expected);
        // A tab's prior principal and lifecycle are not additional binding gates.
        state.tabs.get_mut("tab").unwrap().principal_id = Some("principal:foreign".to_string());
        state.tabs.get_mut("tab").unwrap().lifecycle = crate::TabLifecycle::Closed;
        expected = state.clone();
        let expected_tab = expected.tabs.get_mut("tab").unwrap();
        expected_tab.principal_id = Some(authority.principal_id.clone());
        expected_tab.principal_provenance = Some(authority.provenance);
        expected_tab.work_lease_id = Some("tab-work-lease-v1:83c9a3042080d8b5ab8b7f1d".to_string());
        expected_tab.work_lease_revision = 1;
        expected_tab.work_lease_expires_at = Some("unparsed-expiry".to_string());
        let result = state
            .bind_tab_work_lease("tab", &authority, "unparsed-expiry".to_string())
            .unwrap();
        assert_eq!(result, expected.tabs["tab"]);
        assert_eq!(state, expected);
        state
            .sessions
            .get_mut("session")
            .unwrap()
            .work_lease_revision = u64::MAX;
        state.tabs.get_mut("tab").unwrap().work_lease_revision = u64::MAX;
        let before = state.clone();
        let session = state
            .bind_session_work_lease("session", &authority, EXPIRY.to_string(), None)
            .unwrap();
        let tab = state
            .bind_tab_work_lease("tab", &authority, "unparsed-expiry".to_string())
            .unwrap();
        assert_eq!(session.boot_epoch, None);
        assert_eq!(session.work_lease_revision, u64::MAX);
        assert_eq!(tab.work_lease_revision, u64::MAX);
        let mut expected = before;
        expected.sessions.get_mut("session").unwrap().boot_epoch = None;
        assert_eq!(state, expected);
    }

    #[test]
    fn subordinate_rejections_preserve_full_state_and_error_precedence() {
        let (state, authority) = fixture();
        let mut stale = authority.clone();
        stale.capability_revision += 1;
        for (target, actor, profile, principal, code) in [
            (
                "missing",
                &stale,
                Some("wrong"),
                Some("foreign"),
                ServicePrincipalFailureCode::CapabilityMismatch,
            ),
            (
                "missing",
                &authority,
                None,
                None,
                ServicePrincipalFailureCode::WorkLeaseConflict,
            ),
            (
                "session",
                &authority,
                Some("wrong"),
                Some("foreign"),
                ServicePrincipalFailureCode::ProfileMismatch,
            ),
            (
                "session",
                &authority,
                Some("profile:test"),
                Some("foreign"),
                ServicePrincipalFailureCode::WorkLeaseConflict,
            ),
        ] {
            let mut changed = state.clone();
            let session = changed.sessions.get_mut("session").unwrap();
            session.profile_id = profile.map(str::to_string);
            session.principal_id = principal.map(str::to_string);
            let before = changed.clone();
            let error = changed
                .bind_session_work_lease(target, actor, EXPIRY.to_string(), None)
                .unwrap_err();
            assert_eq!(error, principal_error(code));
            assert_eq!(changed, before);
        }
        for case in 0..6 {
            let mut changed = state.clone();
            bind_fixture_session(&mut changed, &authority);
            let actor = if case == 0 { &stale } else { &authority };
            match case {
                0 | 1 => {
                    changed.tabs.clear();
                }
                2 => changed.tabs.get_mut("tab").unwrap().owner_session_id = None,
                3 => {
                    changed.sessions.clear();
                }
                4 => {
                    changed.sessions.get_mut("session").unwrap().principal_id =
                        Some("foreign".to_string())
                }
                5 => {
                    changed.sessions.get_mut("session").unwrap().profile_id =
                        Some("wrong".to_string())
                }
                _ => unreachable!(),
            }
            let before = changed.clone();
            let error = changed
                .bind_tab_work_lease("tab", actor, EXPIRY.to_string())
                .unwrap_err();
            assert_eq!(
                error,
                principal_error(if case == 0 {
                    ServicePrincipalFailureCode::CapabilityMismatch
                } else {
                    ServicePrincipalFailureCode::WorkLeaseConflict
                })
            );
            assert_eq!(changed, before);
        }
    }

    const CASES: &[(PrincipalContinuityRecourse, &str)] = &[
        (
            PrincipalContinuityRecourse::ContinueWithActiveClaim,
            "continue_with_active_claim",
        ),
        (
            PrincipalContinuityRecourse::ContinueWithSelfDeclaredAccess,
            "continue_with_self_declared_access",
        ),
        (
            PrincipalContinuityRecourse::RejoinOwnedBrowser,
            "rejoin_owned_browser",
        ),
        (
            PrincipalContinuityRecourse::ReplaceStaleSamePrincipalSession,
            "replace_stale_same_principal_session",
        ),
        (
            PrincipalContinuityRecourse::WaitForForeignPrincipal,
            "wait_for_foreign_principal",
        ),
        (
            PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
            "reconcile_principal_identity",
        ),
    ];

    #[test]
    fn as_str_matches_each_wire_name() {
        for (recourse, wire_name) in CASES {
            assert_eq!(recourse.as_str(), *wire_name);
        }
    }

    #[test]
    fn serde_uses_exact_snake_case_wire_names() {
        for (recourse, wire_name) in CASES {
            assert_eq!(
                serde_json::to_string(recourse).unwrap(),
                format!("\"{wire_name}\"")
            );
            assert_eq!(
                serde_json::from_str::<PrincipalContinuityRecourse>(&format!("\"{wire_name}\""))
                    .unwrap(),
                *recourse
            );
        }
    }
}
