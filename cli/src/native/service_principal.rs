//! Stable service-principal authority for managed browser profiles.
//!
//! Caller-supplied service, agent, task, and session labels are attribution
//! only. Profile authority comes from a registered principal plus a secret
//! capability whose digest is retained in Service State. Raw capabilities are
//! never persisted or projected into diagnostics.

#[cfg(test)]
use std::collections::BTreeMap;

use super::service_model::{BrowserSession, ServiceState};
#[cfg(test)]
pub(crate) use agent_browser_lease_authority::{
    authenticate_profile_capability, authenticated_authority_is_current,
    register_profile_capability, ServicePrincipalFailureCode,
};
pub(crate) use agent_browser_lease_authority::{
    generate_profile_capability_token, AuthenticatedServicePrincipal, RegisteredProfileCapability,
    ServicePrincipalError, ServicePrincipalProvenance, ServicePrincipalRegistrationRequest,
    ServicePrincipalState, ServiceProfileCapability, ServiceProfileCapabilityState,
};
#[cfg(test)]
pub(crate) use agent_browser_service_model::LegacyPrincipalMigrationDisposition;
pub(crate) use agent_browser_service_model::{
    LegacySessionPrincipalMigrationPlan, PrincipalContinuityDecision, PrincipalContinuityRecourse,
};

pub(crate) fn authenticated_session_work_authority(
    state: &ServiceState,
    session_id: &str,
    now: &str,
) -> Option<AuthenticatedServicePrincipal> {
    state.authenticated_session_work_authority(session_id, now)
}

pub(crate) fn bind_session_work_lease(
    state: &mut ServiceState,
    session_id: &str,
    authority: &AuthenticatedServicePrincipal,
    expires_at: String,
) -> Result<BrowserSession, ServicePrincipalError> {
    let boot_epoch = crate::process_identity::current_boot_epoch();
    state.bind_session_work_lease(session_id, authority, expires_at, boot_epoch)
}

pub(crate) fn bind_tab_work_lease(
    state: &mut ServiceState,
    tab_id: &str,
    authority: &AuthenticatedServicePrincipal,
    expires_at: String,
) -> Result<super::service_model::BrowserTab, ServicePrincipalError> {
    state.bind_tab_work_lease(tab_id, authority, expires_at)
}

pub(crate) fn principal_continuity_decision(
    state: &ServiceState,
    authority: &AuthenticatedServicePrincipal,
) -> PrincipalContinuityDecision {
    state.principal_continuity_decision(authority)
}

pub(crate) fn plan_legacy_session_principal_migration(
    state: &ServiceState,
) -> Vec<LegacySessionPrincipalMigrationPlan> {
    state.plan_legacy_session_principal_migration()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{BrowserProfile, BrowserTab, LeaseState};
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
