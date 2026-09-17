//! Stable service-principal authority for managed browser profiles.
//!
//! Caller-supplied service, agent, task, and session labels are attribution
//! only. Profile authority comes from a registered principal plus a secret
//! capability whose digest is retained in Service State. Raw capabilities are
//! never persisted or projected into diagnostics.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(test)]
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::service_model::{BrowserSession, LeaseState, ServiceState};
pub(crate) use agent_browser_lease_authority::{
    authenticate_profile_capability, authenticated_authority_is_current,
    generate_profile_capability_token, register_profile_capability, rotate_profile_capability,
    AuthenticatedServicePrincipal, RegisteredProfileCapability, ServicePrincipalError,
    ServicePrincipalFailureCode, ServicePrincipalProvenance, ServicePrincipalRegistrationRequest,
    ServicePrincipalState, ServiceProfileCapability, ServiceProfileCapabilityState,
};
pub(crate) use agent_browser_service_model::PrincipalContinuityRecourse;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrincipalContinuityDecision {
    pub(crate) recourse: PrincipalContinuityRecourse,
    pub(crate) requester_principal_id: String,
    pub(crate) profile_id: String,
    pub(crate) holder_session_ids: Vec<String>,
    pub(crate) holder_principal_ids: Vec<String>,
    pub(crate) effect_capable: bool,
    pub(crate) reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LegacyPrincipalMigrationDisposition {
    AlreadyPrincipalBound,
    PrincipalBindingAvailable,
    UnprovenPrincipal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LegacySessionPrincipalMigrationPlan {
    pub(crate) session_id: String,
    pub(crate) profile_id: Option<String>,
    pub(crate) candidate_principal_id: Option<String>,
    pub(crate) disposition: LegacyPrincipalMigrationDisposition,
    pub(crate) observation_only: bool,
    pub(crate) recourse: PrincipalContinuityRecourse,
    pub(crate) reasons: Vec<String>,
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
    let capability = state
        .service_principals
        .profile_capabilities
        .get(&binding.capability_id)?;
    let authority = AuthenticatedServicePrincipal {
        principal_id: principal_id.to_string(),
        profile_id: profile_id.to_string(),
        capability_id: binding.capability_id.clone(),
        capability_revision: capability.revision,
        provenance: binding.provenance,
    };
    authenticated_authority_is_current(&state.service_principals, &authority).then_some(authority)
}

pub(crate) fn bind_session_work_lease(
    state: &mut ServiceState,
    session_id: &str,
    authority: &AuthenticatedServicePrincipal,
    expires_at: String,
) -> Result<BrowserSession, ServicePrincipalError> {
    if !authenticated_authority_is_current(&state.service_principals, authority) {
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
    session.boot_epoch = crate::process_identity::current_boot_epoch();
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
) -> Result<super::service_model::BrowserTab, ServicePrincipalError> {
    if !authenticated_authority_is_current(&state.service_principals, authority) {
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
    if !authenticated_authority_is_current(&state.service_principals, authority) {
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
) -> Option<&'a crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding> {
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
                    .service_principals
                    .principals
                    .get(principal_id)
                    .is_some_and(|principal| {
                        principal.state == ServicePrincipalState::Active
                            && principal.provenance
                                == ServicePrincipalProvenance::RegisteredCapability
                    })
                && state
                    .service_principals
                    .profile_capabilities
                    .get(&binding.capability_id)
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
    use crate::native::service_model::{BrowserProfile, BrowserTab};
    use crate::runtime_owner_transfer::{
        ProfileOwner, ProfileOwnerState, RuntimeOwnerPrincipalBinding, RuntimeOwnerRegistry,
    };

    const CAPABILITY: &str = "synthetic-capability-token-with-more-than-thirty-two-characters";

    fn principal_state() -> (ServiceState, AuthenticatedServicePrincipal) {
        let profile_id = "synthetic-profile";
        let principal_id = "principal:synthetic-service";
        let profile_path = "/tmp/agent-browser-p134/synthetic-profile";
        let profile_identity_digest =
            agent_browser_lease_authority::canonical_profile_identity_digest(std::path::Path::new(
                profile_path,
            ))
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
                owner_id: "runtime-owner-1".to_string(),
                profile_identity_digest: profile_identity_digest.clone(),
                state: ProfileOwnerState::Ready,
                owner_generation: 7,
                browser_id: "synthetic-browser".to_string(),
                daemon_session_route: "synthetic-session".to_string(),
                process_instance_digest: "synthetic-process".to_string(),
                browser_family: "chrome".to_string(),
                cdp_endpoint_identity_digest: "synthetic-cdp".to_string(),
                target_set_digest: "synthetic-target".to_string(),
                pending_transfer: None,
                last_transition: None,
            }),
            ..ServiceState::default()
        };
        let registered = register_profile_capability(
            &mut state.service_principals,
            ServicePrincipalRegistrationRequest {
                principal_id: principal_id.to_string(),
                display_name: Some("Synthetic service".to_string()),
                profile_id: profile_id.to_string(),
                registered_at: Some("2026-08-27T00:00:00Z".to_string()),
                registered_by: Some("local-operator".to_string()),
            },
            CAPABILITY,
        )
        .unwrap();
        state
            .runtime_owner_registry
            .bind_principal_authority(RuntimeOwnerPrincipalBinding {
                principal_id: principal_id.to_string(),
                profile_id: profile_id.to_string(),
                profile_identity_digest,
                capability_id: registered.capability.capability_id,
                provenance: ServicePrincipalProvenance::RegisteredCapability,
                owner_generation: 7,
            })
            .unwrap();
        let authority = authenticate_profile_capability(
            &state.service_principals,
            CAPABILITY,
            Some(profile_id),
        )
        .unwrap();
        (state, authority)
    }

    #[test]
    fn same_and_foreign_principal_recourse_remain_distinct() {
        let (mut state, authority) = principal_state();
        state.sessions.insert(
            "synthetic-session".to_string(),
            BrowserSession {
                id: "synthetic-session".to_string(),
                profile_id: Some(authority.profile_id.clone()),
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        );
        bind_session_work_lease(
            &mut state,
            "synthetic-session",
            &authority,
            "2026-08-27T01:00:00Z".to_string(),
        )
        .unwrap();
        let same = principal_continuity_decision(&state, &authority);
        assert_eq!(
            same.recourse,
            PrincipalContinuityRecourse::RejoinOwnedBrowser
        );
        assert!(same.effect_capable);

        let foreign_capability = "foreign-capability-token-with-more-than-thirty-two-characters";
        let foreign = register_profile_capability(
            &mut state.service_principals,
            ServicePrincipalRegistrationRequest {
                principal_id: "principal:foreign".to_string(),
                display_name: Some("Foreign service".to_string()),
                profile_id: authority.profile_id.clone(),
                registered_at: Some("2026-08-27T00:10:00Z".to_string()),
                registered_by: Some("local-operator".to_string()),
            },
            foreign_capability,
        )
        .unwrap();
        let foreign_authority = authenticate_profile_capability(
            &state.service_principals,
            foreign_capability,
            Some(&authority.profile_id),
        )
        .unwrap();
        let profile_digest = state
            .runtime_owner_registry
            .principal_bindings()
            .keys()
            .next()
            .unwrap()
            .clone();
        crate::runtime_owner_transfer::edit_registry_fixture(&mut state.runtime_owner_registry)
            .principal_records
            .insert(
                profile_digest.clone(),
                RuntimeOwnerPrincipalBinding {
                    principal_id: foreign_authority.principal_id.clone(),
                    profile_id: foreign_authority.profile_id.clone(),
                    profile_identity_digest: profile_digest,
                    capability_id: foreign.capability.capability_id,
                    provenance: foreign_authority.provenance,
                    owner_generation: 7,
                },
            );
        let session = state.sessions.get_mut("synthetic-session").unwrap();
        session.principal_id = Some(foreign_authority.principal_id);
        session.principal_provenance = Some(foreign_authority.provenance);
        let foreign = principal_continuity_decision(&state, &authority);
        assert_eq!(
            foreign.recourse,
            PrincipalContinuityRecourse::WaitForForeignPrincipal
        );
        assert!(!foreign.effect_capable);
    }

    #[test]
    fn stale_same_principal_session_has_replacement_recourse() {
        let (mut state, authority) = principal_state();
        state.sessions.insert(
            "stale-session".to_string(),
            BrowserSession {
                id: "stale-session".to_string(),
                profile_id: Some(authority.profile_id.clone()),
                principal_id: Some(authority.principal_id.clone()),
                principal_provenance: Some(ServicePrincipalProvenance::RegisteredCapability),
                lease: LeaseState::Expired,
                ..BrowserSession::default()
            },
        );
        let decision = principal_continuity_decision(&state, &authority);
        assert_eq!(
            decision.recourse,
            PrincipalContinuityRecourse::ReplaceStaleSamePrincipalSession
        );
        assert!(decision.effect_capable);
    }

    #[test]
    fn session_and_tab_work_leases_are_subordinate_to_unchanged_profile_owner() {
        let (mut state, authority) = principal_state();
        state.sessions.insert(
            "new-task-session".to_string(),
            BrowserSession {
                id: "new-task-session".to_string(),
                profile_id: Some(authority.profile_id.clone()),
                lease: LeaseState::Shared,
                ..BrowserSession::default()
            },
        );
        state.tabs.insert(
            "new-task-tab".to_string(),
            BrowserTab {
                id: "new-task-tab".to_string(),
                browser_id: "synthetic-browser".to_string(),
                owner_session_id: Some("new-task-session".to_string()),
                ..BrowserTab::default()
            },
        );
        let owner_before = state.runtime_owner_registry.clone();
        let session = bind_session_work_lease(
            &mut state,
            "new-task-session",
            &authority,
            "2026-08-27T01:00:00Z".to_string(),
        )
        .unwrap();
        let tab = bind_tab_work_lease(
            &mut state,
            "new-task-tab",
            &authority,
            "2026-08-27T00:30:00Z".to_string(),
        )
        .unwrap();
        assert!(session
            .work_lease_id
            .as_deref()
            .is_some_and(|lease_id| lease_id.starts_with("session-work-lease-v1:")));
        assert!(tab
            .work_lease_id
            .as_deref()
            .is_some_and(|lease_id| lease_id.starts_with("tab-work-lease-v1:")));
        assert_eq!(state.runtime_owner_registry, owner_before);
    }

    #[test]
    fn legacy_labels_remain_observation_only_without_exact_authority() {
        let mut state = ServiceState::default();
        state.sessions.insert(
            "legacy-session".to_string(),
            BrowserSession {
                id: "legacy-session".to_string(),
                service_name: Some("SyntheticBooksReceipts".to_string()),
                agent_name: Some("receipt-agent".to_string()),
                task_name: Some("resume-download".to_string()),
                profile_id: Some("synthetic-profile".to_string()),
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        );
        let plans = plan_legacy_session_principal_migration(&state);
        assert_eq!(plans.len(), 1);
        assert_eq!(
            plans[0].disposition,
            LegacyPrincipalMigrationDisposition::UnprovenPrincipal
        );
        assert!(plans[0].observation_only);
        assert_eq!(
            plans[0].recourse,
            PrincipalContinuityRecourse::ReconcilePrincipalIdentity
        );
        assert!(plans[0].candidate_principal_id.is_none());
    }

    #[test]
    fn principal_shaped_legacy_session_remains_observation_only_without_owner_proof() {
        let mut state = ServiceState::default();
        state.sessions.insert(
            "legacy-session".to_string(),
            BrowserSession {
                id: "legacy-session".to_string(),
                profile_id: Some("synthetic-profile".to_string()),
                principal_id: Some("principal:synthetic-service".to_string()),
                principal_provenance: Some(ServicePrincipalProvenance::RegisteredCapability),
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        );

        let plans = plan_legacy_session_principal_migration(&state);
        assert_eq!(
            plans[0].disposition,
            LegacyPrincipalMigrationDisposition::UnprovenPrincipal
        );
        assert!(plans[0].observation_only);
        assert_eq!(
            plans[0].recourse,
            PrincipalContinuityRecourse::ReconcilePrincipalIdentity
        );
    }

    #[test]
    fn legacy_service_state_deserializes_without_promoting_labels_to_authority() {
        let legacy = serde_json::json!({
            "sessions": {
                "legacy-session": {
                    "id": "legacy-session",
                    "serviceName": "OdolloFulfillment",
                    "agentName": "tracking-agent",
                    "taskName": "fedex-lookup",
                    "profileId": "odollo-fulfillment",
                    "lease": "exclusive"
                }
            }
        });

        let state: ServiceState = serde_json::from_value(legacy).unwrap();
        assert!(state.service_principals.is_empty());
        assert!(state.runtime_owner_registry.principal_bindings().is_empty());
        let plans = plan_legacy_session_principal_migration(&state);
        assert_eq!(
            plans[0].disposition,
            LegacyPrincipalMigrationDisposition::UnprovenPrincipal
        );
        assert!(plans[0].observation_only);
    }

    #[test]
    fn principal_registry_and_owner_binding_round_trip_without_raw_capability() {
        let (state, authority) = principal_state();
        let encoded = serde_json::to_string(&state).unwrap();
        assert!(!encoded.contains(CAPABILITY));

        let decoded: ServiceState = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.service_principals, state.service_principals);
        assert_eq!(decoded.runtime_owner_registry, state.runtime_owner_registry);
        assert!(authenticated_authority_is_current(
            &decoded.service_principals,
            &authority
        ));
    }

    #[test]
    fn stale_capability_revision_cannot_bind_a_work_lease() {
        let (mut state, authority) = principal_state();
        state.sessions.insert(
            "new-task-session".to_string(),
            BrowserSession {
                id: "new-task-session".to_string(),
                profile_id: Some(authority.profile_id.clone()),
                lease: LeaseState::Shared,
                ..BrowserSession::default()
            },
        );
        state
            .service_principals
            .profile_capabilities
            .get_mut(&authority.capability_id)
            .unwrap()
            .revision += 1;

        let error = bind_session_work_lease(
            &mut state,
            "new-task-session",
            &authority,
            "2026-08-27T01:00:00Z".to_string(),
        )
        .unwrap_err();
        assert_eq!(error.code, ServicePrincipalFailureCode::CapabilityMismatch);
        let session = &state.sessions["new-task-session"];
        assert!(session.principal_id.is_none());
        assert!(session.work_lease_id.is_none());
    }
}
