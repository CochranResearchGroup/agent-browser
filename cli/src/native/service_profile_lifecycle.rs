//! Profile lifecycle authorization joined with exact runtime evidence.
//!
//! Access policy authorizes intent. Coordination leases fence admitted work.
//! This module permits a lifecycle effect only after the authorized logical
//! target is joined to a fresh physical observation made by the executing
//! daemon. It does not evaluate access policy or infer process ownership.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use super::service_model::{ServiceState, TabLifecycle};
use super::service_profile_access_policy::{
    ProfileAccessPolicyState, ProfileEvictionMode, ProfileEvictionPlan, ProfileIdentityAssurance,
    ProfilePermission,
};
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub(crate) const PROFILE_LIFECYCLE_AUTHORIZATION_SCHEMA_V1: &str =
    "agent-browser.profile-lifecycle-authorization.v1";
pub(crate) const PROFILE_LIFECYCLE_PROOF_SCHEMA_V1: &str =
    "agent-browser.profile-lifecycle-proof.v1";
pub(crate) const PROFILE_LIFECYCLE_RECEIPT_SCHEMA_V1: &str =
    "agent-browser.profile-lifecycle-receipt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProfileLifecycleAuthorizationState {
    Authorized,
    Completed,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLifecycleAuthorization {
    pub(crate) schema_version: String,
    pub(crate) authorization_id: String,
    pub(crate) profile_id: String,
    pub(crate) policy_revision: u64,
    pub(crate) subject_id: Option<String>,
    pub(crate) assurance: ProfileIdentityAssurance,
    pub(crate) permission: ProfilePermission,
    pub(crate) eviction_mode: ProfileEvictionMode,
    pub(crate) grace_deadline: Option<String>,
    pub(crate) target_resource_ids: Vec<String>,
    pub(crate) issued_at: String,
    pub(crate) state: ProfileLifecycleAuthorizationState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileTabPhysicalObservation<'a> {
    pub(crate) daemon_session_id: &'a str,
    pub(crate) browser_id: &'a str,
    pub(crate) target_id: &'a str,
    pub(crate) attached_target_ids: &'a [String],
    pub(crate) observed_at: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLifecycleProof {
    pub(crate) schema_version: String,
    pub(crate) proof_id: String,
    pub(crate) authorization_id: String,
    pub(crate) profile_id: String,
    pub(crate) policy_revision: u64,
    pub(crate) tab_id: String,
    pub(crate) browser_id: String,
    pub(crate) target_id: String,
    pub(crate) daemon_session_id: String,
    pub(crate) observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLifecycleEffectReceipt {
    pub(crate) schema_version: String,
    pub(crate) receipt_id: String,
    pub(crate) authorization_id: String,
    pub(crate) proof_id: String,
    pub(crate) profile_id: String,
    pub(crate) policy_revision: u64,
    pub(crate) tab_id: String,
    pub(crate) browser_id: String,
    pub(crate) target_id: String,
    pub(crate) cancelled_job_ids: Vec<String>,
    pub(crate) released_session_id: Option<String>,
    pub(crate) terminated_viewer_lease_ids: Vec<String>,
    pub(crate) outcome: String,
    pub(crate) completed_at: String,
}

pub(crate) fn register_profile_eviction_authorization(
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

pub(crate) fn prove_profile_tab_eviction(
    state: &ServiceState,
    authorization_id: &str,
    tab_id: &str,
    observation: ProfileTabPhysicalObservation<'_>,
) -> Result<ProfileLifecycleProof, String> {
    let authorization = state
        .profile_lifecycle_authorizations
        .get(authorization_id)
        .ok_or_else(|| "profile_lifecycle_authorization_missing".to_string())?;
    if authorization.schema_version != PROFILE_LIFECYCLE_AUTHORIZATION_SCHEMA_V1
        || !matches!(
            authorization.state,
            ProfileLifecycleAuthorizationState::Authorized
                | ProfileLifecycleAuthorizationState::Incomplete
        )
        || authorization.permission != ProfilePermission::Evict
        || !authorization
            .target_resource_ids
            .iter()
            .any(|id| id == tab_id)
    {
        return Err("profile_lifecycle_authorization_mismatch".to_string());
    }
    match authorization.eviction_mode {
        ProfileEvictionMode::GracefulOnly => {
            return Err("profile_lifecycle_force_not_authorized".to_string());
        }
        ProfileEvictionMode::ForceAfterGrace => {
            let deadline = authorization
                .grace_deadline
                .as_deref()
                .ok_or_else(|| "profile_lifecycle_grace_deadline_missing".to_string())?;
            let deadline = chrono::DateTime::parse_from_rfc3339(deadline)
                .map_err(|_| "profile_lifecycle_grace_deadline_invalid".to_string())?;
            let observed = chrono::DateTime::parse_from_rfc3339(observation.observed_at)
                .map_err(|_| "profile_lifecycle_observation_time_invalid".to_string())?;
            if observed < deadline {
                return Err("profile_lifecycle_grace_period_active".to_string());
            }
        }
        ProfileEvictionMode::ForceImmediate => {}
    }
    let profile = state
        .profiles
        .get(&authorization.profile_id)
        .ok_or_else(|| "profile_lifecycle_profile_missing".to_string())?;
    let policy = profile
        .access_policy
        .as_ref()
        .ok_or_else(|| "profile_lifecycle_policy_missing".to_string())?;
    if policy.revision != authorization.policy_revision
        || policy.state != ProfileAccessPolicyState::Draining
        || !policy
            .drain
            .as_ref()
            .is_some_and(|drain| drain.force_authorized)
    {
        return Err("profile_lifecycle_policy_fence_changed".to_string());
    }
    let tab = state
        .tabs
        .get(tab_id)
        .ok_or_else(|| "profile_lifecycle_tab_missing".to_string())?;
    let target_id = tab
        .target_id
        .as_deref()
        .ok_or_else(|| "profile_lifecycle_target_unproven".to_string())?;
    let routed_session_id = tab
        .owner_session_id
        .as_deref()
        .or(tab.session_id.as_deref())
        .ok_or_else(|| "profile_lifecycle_daemon_route_unproven".to_string())?;
    if tab.lifecycle == TabLifecycle::Closed
        || tab.browser_id != observation.browser_id
        || target_id != observation.target_id
        || routed_session_id != observation.daemon_session_id
        || !observation
            .attached_target_ids
            .iter()
            .any(|attached| attached == target_id)
    {
        return Err("profile_lifecycle_physical_target_mismatch".to_string());
    }
    let payload = format!(
        "{authorization_id}\0{}\0{tab_id}\0{}\0{target_id}\0{}\0{}",
        authorization.profile_id,
        tab.browser_id,
        observation.daemon_session_id,
        observation.observed_at
    );
    Ok(ProfileLifecycleProof {
        schema_version: PROFILE_LIFECYCLE_PROOF_SCHEMA_V1.to_string(),
        proof_id: format!("profile-lifecycle-proof:{:x}", Sha256::digest(payload)),
        authorization_id: authorization_id.to_string(),
        profile_id: authorization.profile_id.clone(),
        policy_revision: authorization.policy_revision,
        tab_id: tab_id.to_string(),
        browser_id: tab.browser_id.clone(),
        target_id: target_id.to_string(),
        daemon_session_id: observation.daemon_session_id.to_string(),
        observed_at: observation.observed_at.to_string(),
    })
}

pub(crate) fn settle_profile_tab_eviction(
    state: &mut ServiceState,
    proof: &ProfileLifecycleProof,
    completed_at: &str,
) -> Result<ProfileLifecycleEffectReceipt, String> {
    let authorization = state
        .profile_lifecycle_authorizations
        .get(&proof.authorization_id)
        .cloned()
        .ok_or_else(|| "profile_lifecycle_authorization_missing".to_string())?;
    if proof.schema_version != PROFILE_LIFECYCLE_PROOF_SCHEMA_V1
        || proof.profile_id != authorization.profile_id
        || proof.policy_revision != authorization.policy_revision
        || !authorization
            .target_resource_ids
            .iter()
            .any(|target| target == &proof.tab_id)
    {
        return Err("profile_lifecycle_proof_mismatch".to_string());
    }
    let tab = state
        .tabs
        .get(&proof.tab_id)
        .cloned()
        .ok_or_else(|| "profile_lifecycle_tab_missing".to_string())?;
    if tab.browser_id != proof.browser_id || tab.target_id.as_deref() != Some(&proof.target_id) {
        return Err("profile_lifecycle_effect_target_changed".to_string());
    }
    let evicted_subject_id = tab
        .profile_access
        .as_ref()
        .and_then(|access| access.subject_id.as_deref());
    let cancelled_job_ids = super::service_jobs::cancel_profile_eviction_jobs_in_state(
        state,
        &proof.profile_id,
        &proof.tab_id,
        evicted_subject_id,
    )?;
    if let Some(tab) = state.tabs.get_mut(&proof.tab_id) {
        tab.lifecycle = TabLifecycle::Closed;
        tab.service_tab_handle = None;
        if let Some(access) = tab.profile_access.as_mut() {
            access.connection_state =
                super::service_profile_access_policy::ProfileConnectionState::Disconnected;
        }
    }
    let owner_session_id = tab.owner_session_id.or(tab.session_id);
    let released_session_id = owner_session_id.as_ref().and_then(|session_id| {
        let has_live_sibling = state.tabs.values().any(|candidate| {
            candidate.id != proof.tab_id
                && candidate.lifecycle != TabLifecycle::Closed
                && (candidate.owner_session_id.as_deref() == Some(session_id)
                    || candidate.session_id.as_deref() == Some(session_id))
        });
        (!has_live_sibling).then(|| session_id.clone())
    });
    if let Some(session_id) = released_session_id.as_deref() {
        if let Some(session) = state.sessions.get_mut(session_id) {
            session.lease = super::service_model::LeaseState::Released;
            session.last_lease_observed_at = Some(completed_at.to_string());
        }
    }
    let route_ids = released_session_id
        .as_deref()
        .map(|session_id| {
            state
                .remote_view_routes
                .values()
                .filter(|route| route.session_id.as_deref() == Some(session_id))
                .map(|route| route.id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut terminated_viewer_lease_ids = Vec::new();
    for route_id in &route_ids {
        let lease_ids = state
            .remote_view_routes
            .get(route_id)
            .map(|route| route.viewer_lease_ids.clone())
            .unwrap_or_default();
        for lease_id in lease_ids {
            if let Some(lease) = state.viewer_leases.get_mut(&lease_id) {
                lease.state = "disconnected".to_string();
                lease.last_viewer_event = Some("profile_evicted".to_string());
                lease.updated_at = Some(completed_at.to_string());
                terminated_viewer_lease_ids.push(lease_id);
            }
        }
        super::service_model::advance_route_controller_authority(state, route_id, None)?;
        if let Some(route) = state.remote_view_routes.get_mut(route_id) {
            route.viewer_lease_ids.clear();
            route.last_provider_event = Some("profile_evicted".to_string());
        }
    }
    terminated_viewer_lease_ids.sort();
    terminated_viewer_lease_ids.dedup();
    let all_complete = authorization.target_resource_ids.iter().all(|target| {
        state
            .tabs
            .get(target)
            .is_none_or(|tab| tab.lifecycle == TabLifecycle::Closed)
    });
    if let Some(current) = state
        .profile_lifecycle_authorizations
        .get_mut(&proof.authorization_id)
    {
        current.state = if all_complete {
            ProfileLifecycleAuthorizationState::Completed
        } else {
            ProfileLifecycleAuthorizationState::Incomplete
        };
    }
    let outcome = if all_complete {
        "forced_eviction_completed"
    } else {
        "forced_eviction_incomplete"
    };
    let receipt_payload = format!("{}\0{}\0{outcome}", proof.proof_id, proof.tab_id);
    let receipt = ProfileLifecycleEffectReceipt {
        schema_version: PROFILE_LIFECYCLE_RECEIPT_SCHEMA_V1.to_string(),
        receipt_id: format!(
            "profile-lifecycle-receipt:{:x}",
            Sha256::digest(receipt_payload)
        ),
        authorization_id: proof.authorization_id.clone(),
        proof_id: proof.proof_id.clone(),
        profile_id: proof.profile_id.clone(),
        policy_revision: proof.policy_revision,
        tab_id: proof.tab_id.clone(),
        browser_id: proof.browser_id.clone(),
        target_id: proof.target_id.clone(),
        cancelled_job_ids,
        released_session_id,
        terminated_viewer_lease_ids,
        outcome: outcome.to_string(),
        completed_at: completed_at.to_string(),
    };
    state
        .profile_lifecycle_effect_receipts
        .insert(receipt.receipt_id.clone(), receipt.clone());
    state.refresh_service_tab_handles();
    Ok(receipt)
}

/// Apply one exact tab target from a previously persisted eviction
/// authorization. The executing daemon supplies the attached-target
/// observation; callers cannot assert a physical target or daemon route.
pub(crate) async fn handle_service_profile_tab_evict(
    cmd: &Value,
    daemon_state: &mut super::action_runtime::runtime::DaemonState,
) -> Result<Value, String> {
    let authorization_id = cmd
        .get("authorizationId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "service_profile_tab_evict requires authorizationId".to_string())?;
    let tab_id = cmd
        .get("tabId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "service_profile_tab_evict requires tabId".to_string())?;
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| format!("Could not format profile lifecycle timestamp: {error}"))?;
    let manager = daemon_state
        .browser
        .as_mut()
        .ok_or_else(|| "profile_lifecycle_browser_unavailable".to_string())?;
    let attached_target_ids = manager
        .pages_list()
        .into_iter()
        .map(|page| page.target_id)
        .collect::<Vec<_>>();
    let repository = LockedServiceStateRepository::default_json()?;
    let snapshot = repository.load_snapshot()?;
    if let Some(receipt) = snapshot
        .profile_lifecycle_effect_receipts
        .values()
        .find(|receipt| receipt.authorization_id == authorization_id && receipt.tab_id == tab_id)
        .cloned()
    {
        return Ok(json!({
            "profileLifecycleProof": Value::Null,
            "physicalTabClose": Value::Null,
            "receipt": receipt,
            "replayed": true,
        }));
    }
    let tab = snapshot
        .tabs
        .get(tab_id)
        .ok_or_else(|| "profile_lifecycle_tab_missing".to_string())?;
    let target_id = tab
        .target_id
        .as_deref()
        .ok_or_else(|| "profile_lifecycle_target_unproven".to_string())?;
    let proof = prove_profile_tab_eviction(
        &snapshot,
        authorization_id,
        tab_id,
        ProfileTabPhysicalObservation {
            daemon_session_id: &daemon_state.session_id,
            browser_id: &tab.browser_id,
            target_id,
            attached_target_ids: &attached_target_ids,
            observed_at: &now,
        },
    )?;
    repository.mutate(|state| {
        let current_proof = prove_profile_tab_eviction(
            state,
            authorization_id,
            tab_id,
            ProfileTabPhysicalObservation {
                daemon_session_id: &daemon_state.session_id,
                browser_id: &proof.browser_id,
                target_id: &proof.target_id,
                attached_target_ids: &attached_target_ids,
                observed_at: &now,
            },
        )?;
        if current_proof != proof {
            return Err("profile_lifecycle_proof_changed".to_string());
        }
        let current = state
            .tabs
            .get_mut(tab_id)
            .ok_or_else(|| "profile_lifecycle_tab_missing".to_string())?;
        if current.browser_id != proof.browser_id
            || current.target_id.as_deref() != Some(proof.target_id.as_str())
            || matches!(
                current.lifecycle,
                TabLifecycle::Closing | TabLifecycle::Closed
            )
        {
            return Err("profile_lifecycle_effect_target_changed".to_string());
        }
        current.lifecycle = TabLifecycle::Closing;
        Ok(())
    })?;
    if manager.pages_list().len() <= 1 {
        if let Err(error) = manager.tab_new(Some("about:blank")).await {
            restore_profile_tab_after_failed_eviction(&repository, &proof)?;
            return Err(format!("profile_lifecycle_preservation_tab_failed:{error}"));
        }
    }
    let close_result = match manager
        .tab_close_target_id_for_release(&proof.target_id)
        .await
    {
        Ok(result)
            if result
                .get("closeCommandAcknowledged")
                .and_then(Value::as_bool)
                == Some(true) =>
        {
            result
        }
        Ok(result) => {
            restore_profile_tab_after_failed_eviction(&repository, &proof)?;
            return Err(format!(
                "profile_lifecycle_physical_close_unproven:{}",
                result
                    .get("closeCommandError")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            ));
        }
        Err(error) => {
            restore_profile_tab_after_failed_eviction(&repository, &proof)?;
            return Err(format!("profile_lifecycle_physical_close_failed:{error}"));
        }
    };
    let receipt = match repository.mutate(|state| settle_profile_tab_eviction(state, &proof, &now))
    {
        Ok(receipt) => receipt,
        Err(error) => {
            let receipt = repository
                .mutate(|state| record_uncertain_profile_tab_eviction(state, &proof, &now))?;
            return Err(format!(
                "profile_lifecycle_settlement_uncertain:{}:{}",
                receipt.receipt_id, error
            ));
        }
    };
    Ok(json!({
        "profileLifecycleProof": proof,
        "physicalTabClose": close_result,
        "receipt": receipt,
    }))
}

fn restore_profile_tab_after_failed_eviction(
    repository: &impl ServiceStateRepository,
    proof: &ProfileLifecycleProof,
) -> Result<(), String> {
    repository.mutate(|state| {
        if let Some(tab) = state.tabs.get_mut(&proof.tab_id) {
            if tab.browser_id == proof.browser_id
                && tab.target_id.as_deref() == Some(proof.target_id.as_str())
                && tab.lifecycle == TabLifecycle::Closing
            {
                tab.lifecycle = TabLifecycle::Ready;
            }
        }
        Ok(())
    })
}

fn record_uncertain_profile_tab_eviction(
    state: &mut ServiceState,
    proof: &ProfileLifecycleProof,
    completed_at: &str,
) -> Result<ProfileLifecycleEffectReceipt, String> {
    let authorization = state
        .profile_lifecycle_authorizations
        .get_mut(&proof.authorization_id)
        .ok_or_else(|| "profile_lifecycle_authorization_missing".to_string())?;
    authorization.state = ProfileLifecycleAuthorizationState::Incomplete;
    let receipt_payload = format!("{}\0{}\0settlement_uncertain", proof.proof_id, proof.tab_id);
    let receipt = ProfileLifecycleEffectReceipt {
        schema_version: PROFILE_LIFECYCLE_RECEIPT_SCHEMA_V1.to_string(),
        receipt_id: format!(
            "profile-lifecycle-receipt:{:x}",
            Sha256::digest(receipt_payload)
        ),
        authorization_id: proof.authorization_id.clone(),
        proof_id: proof.proof_id.clone(),
        profile_id: proof.profile_id.clone(),
        policy_revision: proof.policy_revision,
        tab_id: proof.tab_id.clone(),
        browser_id: proof.browser_id.clone(),
        target_id: proof.target_id.clone(),
        cancelled_job_ids: Vec::new(),
        released_session_id: None,
        terminated_viewer_lease_ids: Vec::new(),
        outcome: "physical_close_acknowledged_settlement_uncertain".to_string(),
        completed_at: completed_at.to_string(),
    };
    state
        .profile_lifecycle_effect_receipts
        .insert(receipt.receipt_id.clone(), receipt.clone());
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{BrowserProfile, BrowserTab};
    use crate::native::service_profile_access_policy::{
        ProfileAccessDrain, ProfileAccessMode, ServiceProfileAccessPolicy,
    };

    fn authorized_state() -> (ServiceState, ProfileLifecycleAuthorization) {
        let plan = ProfileEvictionPlan {
            plan_id: "profile-eviction-plan:test".to_string(),
            profile_id: "research-gov".to_string(),
            policy_revision: 7,
            requested_by: Some("principal:admin".to_string()),
            mode: ProfileEvictionMode::ForceImmediate,
            grace_deadline: None,
            target_resource_ids: vec!["tab:fieldwork".to_string()],
        };
        let mut authorizations = BTreeMap::new();
        let authorization = register_profile_eviction_authorization(
            &mut authorizations,
            &plan,
            ProfileIdentityAssurance::RegisteredCapability,
            "2026-09-02T20:00:00Z",
        )
        .unwrap();
        let policy = ServiceProfileAccessPolicy {
            profile_id: "research-gov".to_string(),
            revision: 7,
            state: ProfileAccessPolicyState::Draining,
            drain: Some(ProfileAccessDrain {
                target_mode: ProfileAccessMode::Restricted,
                expected_revision: 7,
                incompatible_occupancy: vec!["tab:fieldwork".to_string()],
                force_authorized: true,
            }),
            ..ServiceProfileAccessPolicy::shared_local_default("research-gov")
        };
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "research-gov".to_string(),
                BrowserProfile {
                    id: "research-gov".to_string(),
                    access_policy: Some(policy),
                    ..BrowserProfile::default()
                },
            )]),
            tabs: BTreeMap::from([(
                "tab:fieldwork".to_string(),
                BrowserTab {
                    id: "tab:fieldwork".to_string(),
                    browser_id: "browser:research".to_string(),
                    target_id: Some("target:research".to_string()),
                    owner_session_id: Some("research-runtime".to_string()),
                    lifecycle: TabLifecycle::Ready,
                    ..BrowserTab::default()
                },
            )]),
            profile_lifecycle_authorizations: authorizations,
            ..ServiceState::default()
        };
        (state, authorization)
    }

    #[test]
    fn eviction_requires_permission_authorization_and_exact_attached_target() {
        let (state, authorization) = authorized_state();
        let attached = vec!["target:research".to_string()];
        let proof = prove_profile_tab_eviction(
            &state,
            &authorization.authorization_id,
            "tab:fieldwork",
            ProfileTabPhysicalObservation {
                daemon_session_id: "research-runtime",
                browser_id: "browser:research",
                target_id: "target:research",
                attached_target_ids: &attached,
                observed_at: "2026-09-02T20:00:01Z",
            },
        )
        .unwrap();
        assert_eq!(proof.tab_id, "tab:fieldwork");
        assert_eq!(proof.target_id, "target:research");

        let wrong_target = prove_profile_tab_eviction(
            &state,
            &authorization.authorization_id,
            "tab:fieldwork",
            ProfileTabPhysicalObservation {
                daemon_session_id: "research-runtime",
                browser_id: "browser:research",
                target_id: "target:other",
                attached_target_ids: &attached,
                observed_at: "2026-09-02T20:00:01Z",
            },
        )
        .unwrap_err();
        assert_eq!(wrong_target, "profile_lifecycle_physical_target_mismatch");
    }

    #[test]
    fn changed_policy_fence_invalidates_an_unconsumed_authorization() {
        let (mut state, authorization) = authorized_state();
        state
            .profiles
            .get_mut("research-gov")
            .unwrap()
            .access_policy
            .as_mut()
            .unwrap()
            .revision = 8;
        let attached = vec!["target:research".to_string()];
        let error = prove_profile_tab_eviction(
            &state,
            &authorization.authorization_id,
            "tab:fieldwork",
            ProfileTabPhysicalObservation {
                daemon_session_id: "research-runtime",
                browser_id: "browser:research",
                target_id: "target:research",
                attached_target_ids: &attached,
                observed_at: "2026-09-02T20:00:01Z",
            },
        )
        .unwrap_err();
        assert_eq!(error, "profile_lifecycle_policy_fence_changed");
    }

    #[test]
    fn settlement_closes_only_the_proven_tab_and_persists_a_minimal_receipt() {
        let (mut state, authorization) = authorized_state();
        state.tabs.insert(
            "tab:other".to_string(),
            BrowserTab {
                id: "tab:other".to_string(),
                browser_id: "browser:research".to_string(),
                target_id: Some("target:other".to_string()),
                owner_session_id: Some("other-runtime".to_string()),
                lifecycle: TabLifecycle::Ready,
                ..BrowserTab::default()
            },
        );
        let attached = vec!["target:research".to_string(), "target:other".to_string()];
        let proof = prove_profile_tab_eviction(
            &state,
            &authorization.authorization_id,
            "tab:fieldwork",
            ProfileTabPhysicalObservation {
                daemon_session_id: "research-runtime",
                browser_id: "browser:research",
                target_id: "target:research",
                attached_target_ids: &attached,
                observed_at: "2026-09-02T20:00:01Z",
            },
        )
        .unwrap();

        let receipt =
            settle_profile_tab_eviction(&mut state, &proof, "2026-09-02T20:00:02Z").unwrap();

        assert_eq!(state.tabs["tab:fieldwork"].lifecycle, TabLifecycle::Closed);
        assert_eq!(state.tabs["tab:other"].lifecycle, TabLifecycle::Ready);
        assert_eq!(
            state.profile_lifecycle_authorizations[&authorization.authorization_id].state,
            ProfileLifecycleAuthorizationState::Completed
        );
        assert_eq!(receipt.outcome, "forced_eviction_completed");
        assert!(state
            .profile_lifecycle_effect_receipts
            .contains_key(&receipt.receipt_id));
        let serialized = serde_json::to_string(&receipt).unwrap();
        assert!(!serialized.contains("page"));
        assert!(!serialized.contains("form"));
        assert!(!serialized.contains("user-data"));
    }

    #[test]
    fn post_grace_force_cannot_run_before_its_authorized_deadline() {
        let (mut state, authorization) = authorized_state();
        let current = state
            .profile_lifecycle_authorizations
            .get_mut(&authorization.authorization_id)
            .unwrap();
        current.eviction_mode = ProfileEvictionMode::ForceAfterGrace;
        current.grace_deadline = Some("2026-09-02T20:05:00Z".to_string());
        let attached = vec!["target:research".to_string()];

        let error = prove_profile_tab_eviction(
            &state,
            &authorization.authorization_id,
            "tab:fieldwork",
            ProfileTabPhysicalObservation {
                daemon_session_id: "research-runtime",
                browser_id: "browser:research",
                target_id: "target:research",
                attached_target_ids: &attached,
                observed_at: "2026-09-02T20:04:59Z",
            },
        )
        .unwrap_err();
        assert_eq!(error, "profile_lifecycle_grace_period_active");

        prove_profile_tab_eviction(
            &state,
            &authorization.authorization_id,
            "tab:fieldwork",
            ProfileTabPhysicalObservation {
                daemon_session_id: "research-runtime",
                browser_id: "browser:research",
                target_id: "target:research",
                attached_target_ids: &attached,
                observed_at: "2026-09-02T20:05:00Z",
            },
        )
        .unwrap();
    }

    #[test]
    fn graceful_only_authorization_cannot_be_used_for_a_forced_effect() {
        let (mut state, authorization) = authorized_state();
        state
            .profile_lifecycle_authorizations
            .get_mut(&authorization.authorization_id)
            .unwrap()
            .eviction_mode = ProfileEvictionMode::GracefulOnly;
        let attached = vec!["target:research".to_string()];

        let error = prove_profile_tab_eviction(
            &state,
            &authorization.authorization_id,
            "tab:fieldwork",
            ProfileTabPhysicalObservation {
                daemon_session_id: "research-runtime",
                browser_id: "browser:research",
                target_id: "target:research",
                attached_target_ids: &attached,
                observed_at: "2026-09-02T20:00:01Z",
            },
        )
        .unwrap_err();

        assert_eq!(error, "profile_lifecycle_force_not_authorized");
    }
}
