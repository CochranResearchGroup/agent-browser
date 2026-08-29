//! Versioned, no-launch profile acquisition recovery contracts.
//!
//! Recovery planning consumes an authenticated principal identity and an
//! immutable Service State snapshot. It never persists state or launches a
//! browser. Effectful application remains behind a later repository-backed
//! compare-and-swap boundary.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::future::Future;
use std::path::PathBuf;

use super::action_runtime::runtime::{auto_launch, service_browser_id, DaemonState};
use super::service_model::{BrowserProfile, ServiceState};
use super::service_principal::{
    authenticate_profile_capability, AuthenticatedServicePrincipal, ServicePrincipalProvenance,
};
use super::service_resources::load_service_state_for_maintenance;
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use super::service_trace::service_commands::service_now_timestamp;
use crate::runtime_owner_transfer::{CleanupObligationState, RuntimeLaneLifecycleState};

pub(crate) const PROFILE_ACQUISITION_OUTCOME_SCHEMA_V1: &str =
    "agent-browser.profile-acquisition-outcome.v1";
pub(crate) const PROFILE_RECOVERY_PLAN_SCHEMA_V1: &str = "agent-browser.profile-recovery-plan.v1";
pub(crate) const PROFILE_RECOVERY_RECEIPT_SCHEMA_V1: &str =
    "agent-browser.profile-recovery-receipt.v1";
pub(crate) const PROFILE_MITIGATION_ACTION_SCHEMA_V1: &str =
    "agent-browser.profile-mitigation-action.v1";

fn recovery_profile_identity_digest(profile: &BrowserProfile) -> Result<String, String> {
    let profile_hint = profile
        .user_data_dir
        .as_deref()
        .ok_or_else(|| "profile_recovery_identity_unavailable".to_string())?;
    let resolved = crate::runtime_profile::resolve_profile(Some(profile_hint), Some(&profile.id))?;
    crate::runtime_profile::canonical_profile_identity_digest(&resolved.user_data_dir)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProfileAcquisitionState {
    Acquired,
    RecoveryAvailable,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MitigationActionType {
    SupersedeTerminalOwner,
    ReconcileExactPrincipalProfileIdentity,
    ReconcileLegacyPrincipal,
    BindOwnerPrincipalAuthority,
    RepairOwnerGenerationBinding,
    ReleaseExpiredOwnerlessLease,
    AdoptExactLiveBrowser,
    RepairSubordinateProfileBinding,
    RepairIndependentRouteIdentity,
    FinalizeTerminalInstallationBookkeeping,
    RetireInertBrowserRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MitigationApplyPosture {
    AutomaticConclusive,
    ReviewedExactGraph,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MitigationActionDescriptor {
    pub(crate) action_type: MitigationActionType,
    pub(crate) recovery_class: String,
    /// Existing or planned server-owned action that performs the guarded
    /// mutation. Clients must discover this value instead of deriving an
    /// executor from blocker text.
    pub(crate) executor_action: String,
    pub(crate) apply_posture: MitigationApplyPosture,
    pub(crate) effect_authority: RecoveryEffectAuthority,
    pub(crate) blocker_codes: Vec<String>,
    pub(crate) preconditions: Vec<String>,
    pub(crate) compensation: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecoveryEffectAuthority {
    ExactProfileGraph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AcquisitionRecoveryPolicy {
    PlanOnly,
    AutoApplyConclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileAcquisitionIntent {
    /// This identifier must come from the authenticated service-principal
    /// layer. Recovery planning never accepts caller-authored authority.
    pub(crate) principal_id: String,
    pub(crate) profile_id: String,
    pub(crate) service_name: String,
    pub(crate) agent_name: String,
    pub(crate) task_name: String,
    pub(crate) target_service_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RecoveryIdentityJoins {
    pub(crate) principal_id: String,
    pub(crate) profile_id: String,
    pub(crate) profile_identity_digest: String,
    pub(crate) lifecycle_owner_id: String,
    pub(crate) lifecycle_owner_generation: u64,
    pub(crate) durable_browser_id: String,
    pub(crate) daemon_session_route: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) service_session_id: Option<String>,
    pub(crate) process_instance_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) presentation_route_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DominantBlocker {
    pub(crate) code: String,
    pub(crate) recoverable: bool,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RecoveryEvidence {
    pub(crate) code: String,
    pub(crate) subject_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MitigationAction {
    pub(crate) schema_version: String,
    pub(crate) action_id: String,
    pub(crate) action_type: MitigationActionType,
    pub(crate) effect_authority: RecoveryEffectAuthority,
    pub(crate) preconditions: Vec<String>,
    pub(crate) expected_postconditions: Vec<String>,
    pub(crate) compensation: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RecoveryPlan {
    pub(crate) schema_version: String,
    pub(crate) plan_id: String,
    pub(crate) recovery_id: String,
    pub(crate) created_at: String,
    pub(crate) expires_at: String,
    pub(crate) idempotency_key_digest: String,
    pub(crate) service_state_revision: u64,
    pub(crate) identities: RecoveryIdentityJoins,
    pub(crate) dominant_blocker: DominantBlocker,
    pub(crate) evidence: Vec<RecoveryEvidence>,
    pub(crate) actions: Vec<MitigationAction>,
    pub(crate) original_intent: ProfileAcquisitionIntent,
    pub(crate) integrity_seal: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RecoveryReceipt {
    pub(crate) schema_version: String,
    pub(crate) recovery_id: String,
    pub(crate) plan_id: String,
    pub(crate) principal_id: String,
    pub(crate) profile_id: String,
    pub(crate) terminal_result: String,
    pub(crate) precondition_comparison: String,
    pub(crate) attempted_operation_ids: Vec<String>,
    pub(crate) compensation_result: String,
    pub(crate) final_state_revision: u64,
    pub(crate) acquisition_retry_state: ProfileAcquisitionState,
    pub(crate) browser_id: String,
    pub(crate) daemon_session_route: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileAcquisitionRetryResult {
    pub(crate) browser_id: String,
    pub(crate) daemon_session_route: String,
}

fn profile_acquisition_retry_command(
    intent: &ProfileAcquisitionIntent,
    daemon_session_route: &str,
) -> Value {
    let service_name = if intent.service_name.trim().is_empty() {
        intent.principal_id.as_str()
    } else {
        intent.service_name.as_str()
    };
    json!({
        "action": "tab_new",
        "profileId": intent.profile_id,
        "serviceName": service_name,
        "agentName": intent.agent_name,
        "taskName": intent.task_name,
        "targetServiceIds": intent.target_service_ids,
        "sessionName": daemon_session_route,
        "servicePrincipalId": intent.principal_id,
        "servicePrincipalProvenance": "registered_capability",
    })
}

fn profile_acquisition_retry_route(
    state: &ServiceState,
    authority: &AuthenticatedServicePrincipal,
) -> Result<String, String> {
    let profile = state
        .profiles
        .get(&authority.profile_id)
        .ok_or_else(|| "profile_recovery_profile_missing".to_string())?;
    let profile_identity_digest = recovery_profile_identity_digest(profile)?;
    state
        .runtime_owner_registry
        .owner(&profile_identity_digest)
        .map(|owner| owner.daemon_session_route.clone())
        .or_else(|| super::service_access::authenticated_cold_session_name(authority, profile))
        .ok_or_else(|| "profile_acquisition_daemon_route_unavailable".to_string())
}

pub(crate) fn profile_acquisition_daemon_route(command: &Value) -> Result<String, String> {
    if command.get("sessionName").is_some() {
        return Err("profile_acquisition_client_route_forbidden".to_string());
    }
    let repository = LockedServiceStateRepository::default_json()?;
    let snapshot = repository.load_snapshot()?;
    let profile_id = required_command_string(command, "profileId")?;
    let raw_capability = profile_capability_from_command(command)?;
    let authority = authenticate_profile_capability(
        &snapshot.service_principals,
        &raw_capability,
        Some(profile_id),
    )
    .map_err(|error| format!("profile_acquisition_principal_{}", error.code.as_str()))?;
    profile_acquisition_retry_route(&snapshot, &authority)
}

pub(crate) fn profile_recovery_apply_daemon_route(command: &Value) -> Result<String, String> {
    let plan = recovery_plan_from_command(command)?;
    let requested_route = required_command_string(command, "sessionName")?;
    if requested_route != plan.identities.daemon_session_route {
        return Err("profile_recovery_daemon_route_mismatch".to_string());
    }
    let raw_capability = profile_capability_from_command(command)?;
    verify_plan_integrity(&plan, raw_capability.as_bytes())?;
    let repository = LockedServiceStateRepository::default_json()?;
    let snapshot = repository.load_snapshot()?;
    let authority = authenticate_profile_capability(
        &snapshot.service_principals,
        &raw_capability,
        Some(&plan.identities.profile_id),
    )
    .map_err(|error| format!("profile_recovery_principal_{}", error.code.as_str()))?;
    if authority.principal_id != plan.identities.principal_id
        || authority.principal_id != plan.original_intent.principal_id
    {
        return Err("profile_recovery_principal_mismatch".to_string());
    }
    Ok(plan.identities.daemon_session_route)
}

fn bind_acquired_profile_principal<R: ServiceStateRepository>(
    repository: &R,
    authority: &AuthenticatedServicePrincipal,
    daemon_session_route: &str,
) -> Result<(), String> {
    repository.mutate(|state| {
        let profile = state
            .profiles
            .get(&authority.profile_id)
            .ok_or_else(|| "profile_recovery_profile_missing".to_string())?;
        let profile_identity_digest = recovery_profile_identity_digest(profile)?;
        let owner_generation = state
            .runtime_owner_registry
            .owner(&profile_identity_digest)
            .filter(|owner| {
                owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Ready
                    && owner.daemon_session_route == daemon_session_route
            })
            .map(|owner| owner.owner_generation)
            .ok_or_else(|| "profile_acquisition_retry_owner_missing".to_string())?;
        state
            .runtime_owner_registry
            .bind_principal_authority(
                crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                    principal_id: authority.principal_id.clone(),
                    profile_id: authority.profile_id.clone(),
                    profile_identity_digest,
                    capability_id: authority.capability_id.clone(),
                    provenance: authority.provenance,
                    owner_generation,
                },
            )
            .map_err(|error| format!("profile_acquisition_owner_binding_failed:{error:?}"))?;
        let session = state
            .sessions
            .get_mut(daemon_session_route)
            .ok_or_else(|| "profile_acquisition_retry_session_missing".to_string())?;
        session.principal_id = Some(authority.principal_id.clone());
        session.principal_provenance = Some(authority.provenance);
        let tab_ids = session.tab_ids.clone();
        for tab_id in tab_ids {
            if let Some(tab) = state.tabs.get_mut(&tab_id) {
                tab.principal_id = Some(authority.principal_id.clone());
                tab.principal_provenance = Some(authority.provenance);
            }
        }
        Ok(())
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecoveryApplyOutcome {
    pub(crate) receipt: RecoveryReceipt,
    pub(crate) acquisition: ProfileAcquisitionOutcome,
    pub(crate) replayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileAcquisitionOutcome {
    pub(crate) schema_version: String,
    pub(crate) state: ProfileAcquisitionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dominant_blocker: Option<DominantBlocker>,
    pub(crate) automatic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) browser_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) daemon_session_route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recovery: Option<RecoveryPlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next_action: Option<String>,
    pub(crate) evidence: Vec<RecoveryEvidence>,
}

/// Return the server-owned mitigation registry. Clients discover new actions
/// from this contract and never infer effect authority from reason-code text.
pub(crate) fn mitigation_action_registry() -> Vec<MitigationActionDescriptor> {
    vec![
        descriptor(
            MitigationActionType::SupersedeTerminalOwner,
            "supersede_terminal_owner",
            "service_profile_acquire",
            MitigationApplyPosture::AutomaticConclusive,
            &[
                "terminal_replacement_route_inconsistent",
                "terminal_owner_cleanup_satisfied",
            ],
            &[
                "terminal_cleanup_satisfied",
                "exact_process_absence_proven",
                "foreign_lease_absent",
            ],
            &["retain_exact_cleanup_obligation_on_uncertain_effect"],
        ),
        descriptor(
            MitigationActionType::ReconcileExactPrincipalProfileIdentity,
            "reconcile_exact_principal_profile_identity",
            "service_profile_acquire",
            MitigationApplyPosture::ReviewedExactGraph,
            &[
                "existing_session_profile_identity_unproven",
                "existing_session_profile_identity_inconsistent",
            ],
            &[
                "profile_capability_current",
                "current_process_identity_matches",
                "foreign_principal_absent",
            ],
            &["retain_existing_binding_on_compare_and_swap_failure"],
        ),
        descriptor(
            MitigationActionType::RepairSubordinateProfileBinding,
            "repair_exact_subordinate_binding",
            "service_profile_lease_reconcile_apply",
            MitigationApplyPosture::ReviewedExactGraph,
            &["existing_session_profile_identity_inconsistent"],
            &[
                "profile_capability_current",
                "browser_session_profile_join_unique",
            ],
            &["preserve_subordinate_work_as_blocked"],
        ),
        descriptor(
            MitigationActionType::ReconcileLegacyPrincipal,
            "reconcile_exact_principal_capability",
            "service_profile_lease_rejoin",
            MitigationApplyPosture::ReviewedExactGraph,
            &["legacy_principal_unproven"],
            &["profile_capability_current", "one_exact_uncontested_owner"],
            &["preserve_legacy_principal_evidence"],
        ),
        descriptor(
            MitigationActionType::BindOwnerPrincipalAuthority,
            "rejoin_exact_owner_principal",
            "service_profile_lease_rejoin",
            MitigationApplyPosture::ReviewedExactGraph,
            &["runtime_owner_principal_binding_missing"],
            &[
                "owner_ready",
                "owner_generation_current",
                "profile_capability_current",
            ],
            &["leave_owner_unbound_on_compare_and_swap_failure"],
        ),
        descriptor(
            MitigationActionType::RepairOwnerGenerationBinding,
            "compare_and_swap_principal_owner_binding",
            "service_profile_lease_reconcile_apply",
            MitigationApplyPosture::ReviewedExactGraph,
            &["owner_generation_or_binding_mismatch"],
            &[
                "same_principal_profile_capability",
                "new_owner_generation_ready",
            ],
            &["preserve_prior_binding_on_stale_generation"],
        ),
        descriptor(
            MitigationActionType::ReleaseExpiredOwnerlessLease,
            "release_expired_ownerless_lease",
            "service_profile_lease_release",
            MitigationApplyPosture::AutomaticConclusive,
            &["expired_ownerless_lease"],
            &["lease_expired", "owner_absent", "subordinate_work_absent"],
            &["restore_exact_lease_from_receipt_on_persistence_failure"],
        ),
        descriptor(
            MitigationActionType::AdoptExactLiveBrowser,
            "adopt_exact_live_browser",
            "external_byop_adopt",
            MitigationApplyPosture::ReviewedExactGraph,
            &["exact_live_browser_unowned"],
            &[
                "process_identity_current",
                "profile_identity_exact",
                "foreign_owner_absent",
            ],
            &["remove_only_new_owner_binding_on_adoption_failure"],
        ),
        descriptor(
            MitigationActionType::RepairSubordinateProfileBinding,
            "repair_subordinate_browser_session_profile_binding",
            "service_profile_lease_reconcile_apply",
            MitigationApplyPosture::ReviewedExactGraph,
            &["subordinate_browser_session_profile_mismatch"],
            &[
                "principal_authority_current",
                "browser_session_profile_join_unique",
            ],
            &["preserve_subordinate_work_as_blocked"],
        ),
        descriptor(
            MitigationActionType::RepairIndependentRouteIdentity,
            "acquire_route_bound_manual_seeding_handoff",
            "service_profile_manual_seeding_acquire",
            MitigationApplyPosture::ReviewedExactGraph,
            &[
                "presentation_route_identity_unproven",
                "independent_route_stale",
            ],
            &["browser_identity_exact", "route_unreferenced_or_same_owner"],
            &["restore_prior_route_binding_or_quarantine_exact_route"],
        ),
        descriptor(
            MitigationActionType::FinalizeTerminalInstallationBookkeeping,
            "finalize_terminal_installation_bookkeeping",
            "install_transactions_close",
            MitigationApplyPosture::AutomaticConclusive,
            &["terminal_installation_transaction"],
            &[
                "installation_effect_terminal",
                "selected_generation_unchanged",
            ],
            &["retain_exact_install_recovery_obligation"],
        ),
        descriptor(
            MitigationActionType::RetireInertBrowserRecord,
            "review_exact_inert_record_retirement",
            "service_browser_retirement_apply",
            MitigationApplyPosture::ReviewedExactGraph,
            &["live_browser_missing_pid", "fixture_shaped_browser_record"],
            &[
                "process_authority_absent",
                "managed_runtime_authority_absent",
                "references_absent",
            ],
            &["replay_receipt_without_broad_cleanup"],
        ),
    ]
}

pub(crate) fn profile_blocker_dominance_order() -> &'static [&'static str] {
    &[
        "live_foreign_principal_authority",
        "existing_session_profile_identity_inconsistent",
        "existing_session_profile_identity_unproven",
        "legacy_principal_unproven",
        "runtime_owner_principal_binding_missing",
        "owner_generation_or_binding_mismatch",
        "subordinate_browser_session_profile_mismatch",
        "expired_ownerless_lease",
        "presentation_route_identity_unproven",
        "terminal_installation_transaction",
    ]
}

pub(crate) fn dominant_profile_blocker<'a>(
    codes: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let codes = codes.into_iter().collect::<std::collections::BTreeSet<_>>();
    profile_blocker_dominance_order()
        .iter()
        .find(|candidate| codes.contains(**candidate))
        .map(|value| (*value).to_string())
}

fn descriptor(
    action_type: MitigationActionType,
    recovery_class: &str,
    executor_action: &str,
    apply_posture: MitigationApplyPosture,
    blocker_codes: &[&str],
    preconditions: &[&str],
    compensation: &[&str],
) -> MitigationActionDescriptor {
    MitigationActionDescriptor {
        action_type,
        recovery_class: recovery_class.to_string(),
        executor_action: executor_action.to_string(),
        apply_posture,
        effect_authority: RecoveryEffectAuthority::ExactProfileGraph,
        blocker_codes: blocker_codes
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        preconditions: preconditions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        compensation: compensation
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    }
}

/// Classify one authenticated profile acquisition intent without performing
/// browser, route, or repository effects. The caller supplies identity from
/// the capability authentication layer, never from caller-authored labels.
pub(crate) fn plan_profile_acquisition(
    state: &ServiceState,
    intent: ProfileAcquisitionIntent,
    created_at: &str,
    expires_at: &str,
    idempotency_key: &str,
    seal_key: &[u8],
) -> Result<ProfileAcquisitionOutcome, String> {
    let profile = state
        .profiles
        .get(&intent.profile_id)
        .ok_or_else(|| "profile_recovery_profile_missing".to_string())?;
    let profile_identity_digest = recovery_profile_identity_digest(profile)?;
    let owner = state.runtime_owner_registry.owner(&profile_identity_digest);
    let Some(owner) = owner else {
        return Ok(blocked_outcome_with_recourse(
            "runtime_owner_missing",
            true,
            "No lifecycle owner is registered for the exact profile identity.",
            "register_exact_profile_owner",
            Vec::new(),
        ));
    };
    let lifecycle = state
        .runtime_owner_registry
        .lifecycle_records
        .get(&owner.browser_id);
    let Some(lifecycle) = lifecycle else {
        return Ok(blocked_outcome_with_recourse(
            "runtime_owner_lifecycle_missing",
            true,
            "The exact lifecycle owner has no matching lifecycle record.",
            "inspect_or_reconcile_exact_profile_graph",
            vec![RecoveryEvidence {
                code: "lifecycle_record_missing".to_string(),
                subject_id: owner.browser_id.clone(),
            }],
        ));
    };
    let process_proven = state
        .browsers
        .get(&owner.browser_id)
        .and_then(|browser| browser.pid)
        .is_some();
    let binding = state
        .runtime_owner_registry
        .principal_bindings
        .get(&profile_identity_digest);

    if process_proven && lifecycle.lifecycle_state == RuntimeLaneLifecycleState::Ready {
        if let Some(binding) = binding {
            if binding.principal_id != intent.principal_id {
                return Ok(blocked_outcome_with_recourse(
                    &dominant_profile_blocker(["live_foreign_principal_authority"])
                        .expect("foreign principal blocker is in the dominance registry"),
                    false,
                    "A different authenticated principal has current process-backed authority for this profile.",
                    "wait_or_coordinate_with_current_principal",
                    vec![
                        RecoveryEvidence {
                            code: "current_process_proven".to_string(),
                            subject_id: owner.browser_id.clone(),
                        },
                        RecoveryEvidence {
                            code: "foreign_principal_binding_current".to_string(),
                            subject_id: binding.principal_id.clone(),
                        },
                    ],
                ));
            }
            if binding.profile_id == intent.profile_id
                && binding.owner_generation == owner.owner_generation
                && lifecycle.profile_identity_digest == profile_identity_digest
                && lifecycle.owner_generation == owner.owner_generation
            {
                return Ok(ProfileAcquisitionOutcome {
                    schema_version: PROFILE_ACQUISITION_OUTCOME_SCHEMA_V1.to_string(),
                    state: ProfileAcquisitionState::Acquired,
                    dominant_blocker: None,
                    automatic: false,
                    browser_id: Some(owner.browser_id.clone()),
                    daemon_session_route: Some(owner.daemon_session_route.clone()),
                    recovery: None,
                    next_action: None,
                    evidence: vec![
                        RecoveryEvidence {
                            code: "current_process_proven".to_string(),
                            subject_id: owner.browser_id.clone(),
                        },
                        RecoveryEvidence {
                            code: "principal_binding_current".to_string(),
                            subject_id: binding.principal_id.clone(),
                        },
                    ],
                });
            }
        }
        return plan_principal_reconciliation(
            state,
            intent,
            profile_identity_digest,
            owner,
            binding.is_some(),
            created_at,
            expires_at,
            idempotency_key,
            seal_key,
        );
    }

    plan_terminal_owner_recovery(
        state,
        intent,
        created_at,
        expires_at,
        idempotency_key,
        seal_key,
    )
}

/// Coordinate planning and the one conclusive automatic recovery class. A
/// successful recovery always retries the original acquisition intent exactly
/// once through the supplied effect boundary.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn coordinate_profile_acquisition<R, F, Fut>(
    repository: &R,
    intent: ProfileAcquisitionIntent,
    created_at: &str,
    expires_at: &str,
    idempotency_key: &str,
    seal_key: &[u8],
    policy: AcquisitionRecoveryPolicy,
    retry_acquisition: F,
) -> Result<ProfileAcquisitionOutcome, String>
where
    R: ServiceStateRepository,
    F: FnOnce(ProfileAcquisitionIntent) -> Fut,
    Fut: Future<Output = Result<ProfileAcquisitionRetryResult, String>>,
{
    let outcome = plan_profile_acquisition(
        &repository.load_snapshot()?,
        intent.clone(),
        created_at,
        expires_at,
        idempotency_key,
        seal_key,
    )?;
    if policy != AcquisitionRecoveryPolicy::AutoApplyConclusive {
        return Ok(outcome);
    }
    if outcome
        .dominant_blocker
        .as_ref()
        .is_some_and(|blocker| blocker.code == "runtime_owner_missing")
    {
        let acquired = retry_acquisition(intent.clone()).await?;
        let mut verified = plan_profile_acquisition(
            &repository.load_snapshot()?,
            intent,
            created_at,
            expires_at,
            idempotency_key,
            seal_key,
        )?;
        if verified.state != ProfileAcquisitionState::Acquired
            || verified.browser_id.as_deref() != Some(acquired.browser_id.as_str())
            || verified.daemon_session_route.as_deref()
                != Some(acquired.daemon_session_route.as_str())
        {
            return Err("profile_acquisition_initial_owner_postcondition_failed".to_string());
        }
        verified.automatic = true;
        verified.evidence.push(RecoveryEvidence {
            code: "initial_owner_registered".to_string(),
            subject_id: acquired.browser_id,
        });
        return Ok(verified);
    }
    let Some(plan) = outcome.recovery.as_ref() else {
        return Ok(outcome);
    };
    let conclusive = plan.actions.len() == 1
        && plan.actions[0].action_type == MitigationActionType::SupersedeTerminalOwner
        && plan.dominant_blocker.code == "terminal_owner_cleanup_satisfied";
    if !conclusive {
        return Ok(outcome);
    }
    apply_terminal_owner_recovery(repository, plan, created_at, seal_key, retry_acquisition)
        .await
        .map(|applied| applied.acquisition)
}

/// Executes the no-launch plan and status operations, or the exact
/// terminal-owner apply operation, for the public recovery command surface.
/// The raw profile capability is consumed only for authentication and plan
/// sealing. It is never copied into Service State, receipts, or launch input.
pub(crate) async fn handle_service_profile_recovery_command(
    command: &Value,
    daemon_state: &mut DaemonState,
) -> Result<Value, String> {
    match required_command_string(command, "action")? {
        "service_profile_acquire" => acquire_profile_command(command, daemon_state).await,
        "service_profile_recovery_plan" => plan_profile_recovery_command(command),
        "service_profile_recovery_status" => status_profile_recovery_command(command),
        "service_profile_recovery_apply" => {
            apply_profile_recovery_command(command, daemon_state).await
        }
        action => Err(format!("Unsupported profile recovery command: {action}")),
    }
}

async fn acquire_profile_command(
    command: &Value,
    daemon_state: &mut DaemonState,
) -> Result<Value, String> {
    if command.get("sessionName").is_some() {
        return Err("profile_acquisition_client_route_forbidden".to_string());
    }
    let repository = LockedServiceStateRepository::default_json()?;
    let snapshot = repository.load_snapshot()?;
    let profile_id = required_command_string(command, "profileId")?;
    let raw_capability = profile_capability_from_command(command)?;
    let authority = authenticate_profile_capability(
        &snapshot.service_principals,
        &raw_capability,
        Some(profile_id),
    )
    .map_err(|error| format!("profile_acquisition_principal_{}", error.code.as_str()))?;
    let retry_authority = authority.clone();
    let intent = ProfileAcquisitionIntent {
        principal_id: authority.principal_id,
        profile_id: authority.profile_id,
        service_name: optional_command_string(command, "serviceName").unwrap_or_default(),
        agent_name: optional_command_string(command, "agentName").unwrap_or_default(),
        task_name: optional_command_string(command, "taskName").unwrap_or_default(),
        target_service_ids: optional_command_string_array(command, "targetServiceIds")?,
    };
    let retry_daemon_session_route = profile_acquisition_retry_route(&snapshot, &retry_authority)?;
    if retry_daemon_session_route != daemon_state.session_id {
        return Err("profile_acquisition_daemon_route_mismatch".to_string());
    }
    let created_at = service_now_timestamp();
    let expires_at = optional_command_string(command, "expiresAt").unwrap_or_else(|| {
        (chrono::Utc::now() + chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    });
    let idempotency_key = optional_command_string(command, "idempotencyKey")
        .unwrap_or_else(|| format!("profile-acquisition-{}", uuid::Uuid::new_v4()));
    let policy = if command
        .get("automaticRecovery")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        AcquisitionRecoveryPolicy::AutoApplyConclusive
    } else {
        AcquisitionRecoveryPolicy::PlanOnly
    };
    let retry_repository = repository.clone();
    let outcome = coordinate_profile_acquisition(
        &repository,
        intent,
        &created_at,
        &expires_at,
        &idempotency_key,
        raw_capability.as_bytes(),
        policy,
        |intent| async move {
            let retry_command =
                profile_acquisition_retry_command(&intent, &retry_daemon_session_route);
            auto_launch(daemon_state, &retry_command).await?;
            bind_acquired_profile_principal(
                &retry_repository,
                &retry_authority,
                &retry_daemon_session_route,
            )?;
            if daemon_state.browser.is_none() {
                return Err("profile_acquisition_retry_missing_browser".to_string());
            }
            Ok(ProfileAcquisitionRetryResult {
                browser_id: service_browser_id(&retry_daemon_session_route),
                daemon_session_route: retry_daemon_session_route,
            })
        },
    )
    .await?;
    Ok(json!({ "outcome": outcome }))
}

fn plan_profile_recovery_command(command: &Value) -> Result<Value, String> {
    let state = load_service_state_for_maintenance(command)?;
    let profile_id = required_command_string(command, "profileId")?;
    let raw_capability = profile_capability_from_command(command)?;
    let authority = authenticate_profile_capability(
        &state.service_principals,
        &raw_capability,
        Some(profile_id),
    )
    .map_err(|error| format!("profile_recovery_principal_{}", error.code.as_str()))?;
    let intent = ProfileAcquisitionIntent {
        principal_id: authority.principal_id,
        profile_id: authority.profile_id,
        service_name: optional_command_string(command, "serviceName").unwrap_or_default(),
        agent_name: optional_command_string(command, "agentName").unwrap_or_default(),
        task_name: optional_command_string(command, "taskName").unwrap_or_default(),
        target_service_ids: optional_command_string_array(command, "targetServiceIds")?,
    };
    let created_at = service_now_timestamp();
    let expires_at = required_command_string(command, "expiresAt")?;
    let idempotency_key = optional_command_string(command, "idempotencyKey")
        .unwrap_or_else(|| format!("profile-recovery-{}", uuid::Uuid::new_v4()));
    let outcome = plan_terminal_owner_recovery(
        &state,
        intent,
        &created_at,
        expires_at,
        &idempotency_key,
        raw_capability.as_bytes(),
    )?;
    Ok(json!({ "outcome": outcome }))
}

fn status_profile_recovery_command(command: &Value) -> Result<Value, String> {
    let recovery_id = required_command_string(command, "recoveryId")?;
    let raw_capability = profile_capability_from_command(command)?;
    let repository = LockedServiceStateRepository::default_json()?;
    let state = repository.load_snapshot()?;
    let receipt = state.profile_recovery_receipts.get(recovery_id).cloned();
    let Some(receipt) = receipt else {
        return Ok(json!({
            "recoveryId": recovery_id,
            "state": "not_found",
            "receipt": null,
        }));
    };
    let authority = authenticate_profile_capability(
        &state.service_principals,
        &raw_capability,
        Some(&receipt.profile_id),
    )
    .map_err(|error| format!("profile_recovery_principal_{}", error.code.as_str()))?;
    if authority.principal_id != receipt.principal_id {
        return Err("profile_recovery_principal_mismatch".to_string());
    }
    Ok(json!({
        "recoveryId": recovery_id,
        "state": receipt.terminal_result,
        "receipt": receipt,
    }))
}

async fn apply_profile_recovery_command(
    command: &Value,
    daemon_state: &mut DaemonState,
) -> Result<Value, String> {
    let plan = recovery_plan_from_command(command)?;
    let raw_capability = profile_capability_from_command(command)?;
    let repository = LockedServiceStateRepository::default_json()?;
    let snapshot = repository.load_snapshot()?;
    let authority = authenticate_profile_capability(
        &snapshot.service_principals,
        &raw_capability,
        Some(&plan.identities.profile_id),
    )
    .map_err(|error| format!("profile_recovery_principal_{}", error.code.as_str()))?;
    if authority.principal_id != plan.identities.principal_id
        || authority.principal_id != plan.original_intent.principal_id
    {
        return Err("profile_recovery_principal_mismatch".to_string());
    }
    if daemon_state.session_id != plan.identities.daemon_session_route {
        return Err("profile_recovery_daemon_route_mismatch".to_string());
    }
    if daemon_state.browser.is_some() {
        return Err("profile_recovery_daemon_route_not_empty".to_string());
    }
    let now = service_now_timestamp();
    let outcome = apply_terminal_owner_recovery(
        &repository,
        &plan,
        &now,
        raw_capability.as_bytes(),
        |intent| async move {
            let retry_command =
                profile_acquisition_retry_command(&intent, &daemon_state.session_id);
            auto_launch(daemon_state, &retry_command).await?;
            if daemon_state.browser.is_none() {
                return Err("profile_recovery_acquisition_retry_missing_browser".to_string());
            }
            Ok(ProfileAcquisitionRetryResult {
                browser_id: service_browser_id(&daemon_state.session_id),
                daemon_session_route: daemon_state.session_id.clone(),
            })
        },
    )
    .await?;
    Ok(json!({
        "outcome": outcome.acquisition,
        "receipt": outcome.receipt,
        "replayed": outcome.replayed,
    }))
}

fn recovery_plan_from_command(command: &Value) -> Result<RecoveryPlan, String> {
    if let Some(plan) = command.get("plan") {
        return serde_json::from_value(plan.clone())
            .map_err(|error| format!("profile_recovery_plan_invalid:{error}"));
    }
    let path = absolute_command_path(command, "planFile")?;
    let encoded = fs::read(&path)
        .map_err(|error| format!("Failed to read recovery plan {}: {error}", path.display()))?;
    serde_json::from_slice(&encoded)
        .map_err(|error| format!("profile_recovery_plan_invalid:{error}"))
}

fn profile_capability_from_command(command: &Value) -> Result<String, String> {
    if let Some(capability) = optional_command_string(command, "profileCapability") {
        return Ok(capability);
    }
    let path = absolute_command_path(command, "profileCapabilityFile")?;
    fs::read_to_string(&path)
        .map(|value| value.trim().to_string())
        .map_err(|error| {
            format!(
                "Failed to read profile capability {}: {error}",
                path.display()
            )
        })
}

fn absolute_command_path(command: &Value, field: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(required_command_string(command, field)?);
    if !path.is_absolute() {
        return Err(format!("profile_recovery_{field}_must_be_absolute"));
    }
    Ok(path)
}

fn required_command_string<'a>(command: &'a Value, field: &str) -> Result<&'a str, String> {
    command
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("profile_recovery_{field}_required"))
}

fn optional_command_string(command: &Value, field: &str) -> Option<String> {
    command
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn optional_command_string_array(command: &Value, field: &str) -> Result<Vec<String>, String> {
    let Some(values) = command.get(field) else {
        return Ok(Vec::new());
    };
    let values = values
        .as_array()
        .ok_or_else(|| format!("profile_recovery_{field}_invalid"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .ok_or_else(|| format!("profile_recovery_{field}_invalid"))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn plan_principal_reconciliation(
    state: &ServiceState,
    intent: ProfileAcquisitionIntent,
    profile_identity_digest: String,
    owner: &crate::runtime_owner_transfer::ProfileOwner,
    binding_present: bool,
    created_at: &str,
    expires_at: &str,
    idempotency_key: &str,
    seal_key: &[u8],
) -> Result<ProfileAcquisitionOutcome, String> {
    if seal_key.len() < 32 || idempotency_key.trim().is_empty() {
        return Err("profile_recovery_plan_invalid".to_string());
    }
    let blocker_code = if binding_present {
        "existing_session_profile_identity_inconsistent"
    } else {
        "existing_session_profile_identity_unproven"
    };
    let identities = RecoveryIdentityJoins {
        principal_id: intent.principal_id.clone(),
        profile_id: intent.profile_id.clone(),
        profile_identity_digest,
        lifecycle_owner_id: owner.owner_id.clone(),
        lifecycle_owner_generation: owner.owner_generation,
        durable_browser_id: owner.browser_id.clone(),
        daemon_session_route: owner.daemon_session_route.clone(),
        service_session_id: state.sessions.values().find_map(|session| {
            session
                .browser_ids
                .iter()
                .any(|browser_id| browser_id == &owner.browser_id)
                .then(|| session.id.clone())
        }),
        process_instance_digest: owner.process_instance_digest.clone(),
        presentation_route_id: None,
    };
    let blocker = DominantBlocker {
        code: blocker_code.to_string(),
        recoverable: true,
        detail: "The process-backed browser exists, but its exact authenticated principal and profile binding requires reviewed reconciliation."
            .to_string(),
    };
    let evidence = vec![
        RecoveryEvidence {
            code: "current_process_proven".to_string(),
            subject_id: owner.browser_id.clone(),
        },
        RecoveryEvidence {
            code: blocker_code.to_string(),
            subject_id: owner.browser_id.clone(),
        },
    ];
    let action = MitigationAction {
        schema_version: PROFILE_MITIGATION_ACTION_SCHEMA_V1.to_string(),
        action_id: digest_json(&(
            "reconcile_exact_principal_profile_identity",
            state.runtime_owner_registry.revision,
            &identities,
        ))?,
        action_type: MitigationActionType::ReconcileExactPrincipalProfileIdentity,
        effect_authority: RecoveryEffectAuthority::ExactProfileGraph,
        preconditions: vec![
            "profile_capability_current".to_string(),
            "profile_identity_digest_matches".to_string(),
            "owner_generation_matches".to_string(),
            "current_process_identity_matches".to_string(),
            "foreign_principal_absent".to_string(),
        ],
        expected_postconditions: vec![
            "principal_profile_binding_current".to_string(),
            "existing_browser_preserved".to_string(),
            "duplicate_launch_absent".to_string(),
        ],
        compensation: vec!["retain_existing_binding_on_compare_and_swap_failure".to_string()],
    };
    let idempotency_key_digest = digest_text(idempotency_key);
    let recovery_id = digest_json(&(
        "profile-recovery",
        &identities.profile_identity_digest,
        identities.lifecycle_owner_generation,
        &idempotency_key_digest,
    ))?;
    let plan_id = digest_json(&(
        PROFILE_RECOVERY_PLAN_SCHEMA_V1,
        &recovery_id,
        state.runtime_owner_registry.revision,
        &identities,
        &blocker,
        &action,
        &intent,
        created_at,
        expires_at,
    ))?;
    let mut plan = RecoveryPlan {
        schema_version: PROFILE_RECOVERY_PLAN_SCHEMA_V1.to_string(),
        plan_id,
        recovery_id,
        created_at: created_at.to_string(),
        expires_at: expires_at.to_string(),
        idempotency_key_digest,
        service_state_revision: state.runtime_owner_registry.revision,
        identities,
        dominant_blocker: blocker.clone(),
        evidence: evidence.clone(),
        actions: vec![action],
        original_intent: intent,
        integrity_seal: String::new(),
    };
    plan.integrity_seal = seal_recovery_plan(&plan, seal_key)?;
    Ok(ProfileAcquisitionOutcome {
        schema_version: PROFILE_ACQUISITION_OUTCOME_SCHEMA_V1.to_string(),
        state: ProfileAcquisitionState::RecoveryAvailable,
        dominant_blocker: Some(blocker),
        automatic: false,
        browser_id: None,
        daemon_session_route: None,
        recovery: Some(plan),
        next_action: Some("review_recovery_plan".to_string()),
        evidence,
    })
}

pub(crate) fn plan_terminal_owner_recovery(
    state: &ServiceState,
    intent: ProfileAcquisitionIntent,
    created_at: &str,
    expires_at: &str,
    idempotency_key: &str,
    seal_key: &[u8],
) -> Result<ProfileAcquisitionOutcome, String> {
    if seal_key.len() < 32 || idempotency_key.trim().is_empty() {
        return Err("profile_recovery_plan_invalid".to_string());
    }
    let profile = state
        .profiles
        .get(&intent.profile_id)
        .ok_or_else(|| "profile_recovery_profile_missing".to_string())?;
    let profile_identity_digest = recovery_profile_identity_digest(profile)?;
    let owner = state
        .runtime_owner_registry
        .owner(&profile_identity_digest)
        .ok_or_else(|| "profile_recovery_owner_missing".to_string())?;
    let lifecycle = state
        .runtime_owner_registry
        .lifecycle_records
        .get(&owner.browser_id)
        .ok_or_else(|| "profile_recovery_lifecycle_missing".to_string())?;
    let active_profile_lease_session_ids =
        active_profile_lease_session_ids(state, &intent.profile_id);
    let current_process_proven = state
        .browsers
        .get(&owner.browser_id)
        .is_some_and(|browser| browser.pid.is_some());
    let exact_terminal = lifecycle.profile_identity_digest == profile_identity_digest
        && lifecycle.logical_browser_id == owner.browser_id
        && lifecycle.owner_generation == owner.owner_generation
        && lifecycle.lifecycle_state == RuntimeLaneLifecycleState::Terminal
        && lifecycle.cleanup_obligation_state == CleanupObligationState::Satisfied
        && lifecycle
            .terminal_evidence
            .iter()
            .any(|evidence| evidence == "exact_process_exited")
        && lifecycle
            .terminal_evidence
            .iter()
            .any(|evidence| evidence == "profile_lock_released")
        && active_profile_lease_session_ids.is_empty()
        && !current_process_proven
        && owner.pending_transfer.is_none();
    if !exact_terminal {
        return Ok(blocked_outcome(owner.browser_id.clone(), lifecycle));
    }

    let identities = RecoveryIdentityJoins {
        principal_id: intent.principal_id.clone(),
        profile_id: intent.profile_id.clone(),
        profile_identity_digest,
        lifecycle_owner_id: owner.owner_id.clone(),
        lifecycle_owner_generation: owner.owner_generation,
        durable_browser_id: owner.browser_id.clone(),
        daemon_session_route: owner.daemon_session_route.clone(),
        service_session_id: state.sessions.values().find_map(|session| {
            session
                .browser_ids
                .iter()
                .any(|browser_id| browser_id == &owner.browser_id)
                .then(|| session.id.clone())
        }),
        process_instance_digest: owner.process_instance_digest.clone(),
        presentation_route_id: None,
    };
    let blocker = DominantBlocker {
        code: "terminal_owner_cleanup_satisfied".to_string(),
        recoverable: true,
        detail: "The exact retained owner is terminal, cleanup is satisfied, and process absence is proven."
            .to_string(),
    };
    let evidence = lifecycle
        .terminal_evidence
        .iter()
        .map(|code| RecoveryEvidence {
            code: code.clone(),
            subject_id: owner.browser_id.clone(),
        })
        .collect::<Vec<_>>();
    let action = MitigationAction {
        schema_version: PROFILE_MITIGATION_ACTION_SCHEMA_V1.to_string(),
        action_id: digest_json(&(
            "supersede_terminal_owner",
            state.runtime_owner_registry.revision,
            &identities,
        ))?,
        action_type: MitigationActionType::SupersedeTerminalOwner,
        effect_authority: RecoveryEffectAuthority::ExactProfileGraph,
        preconditions: vec![
            "service_state_revision_matches".to_string(),
            "profile_identity_digest_matches".to_string(),
            "owner_generation_matches".to_string(),
            "terminal_cleanup_satisfied".to_string(),
            "exact_process_absence_proven".to_string(),
            "foreign_lease_absent".to_string(),
        ],
        expected_postconditions: vec![
            "replacement_generation_is_unique".to_string(),
            "durable_browser_history_preserved".to_string(),
            "original_acquisition_retried".to_string(),
        ],
        compensation: vec!["retain_exact_cleanup_obligation_on_uncertain_effect".to_string()],
    };
    let idempotency_key_digest = digest_text(idempotency_key);
    let recovery_id = digest_json(&(
        "profile-recovery",
        &identities.profile_identity_digest,
        identities.lifecycle_owner_generation,
        &idempotency_key_digest,
    ))?;
    let plan_id = digest_json(&(
        PROFILE_RECOVERY_PLAN_SCHEMA_V1,
        &recovery_id,
        state.runtime_owner_registry.revision,
        &identities,
        &blocker,
        &action,
        &intent,
        created_at,
        expires_at,
    ))?;
    let mut plan = RecoveryPlan {
        schema_version: PROFILE_RECOVERY_PLAN_SCHEMA_V1.to_string(),
        plan_id,
        recovery_id,
        created_at: created_at.to_string(),
        expires_at: expires_at.to_string(),
        idempotency_key_digest,
        service_state_revision: state.runtime_owner_registry.revision,
        identities,
        dominant_blocker: blocker.clone(),
        evidence: evidence.clone(),
        actions: vec![action],
        original_intent: intent,
        integrity_seal: String::new(),
    };
    plan.integrity_seal = seal_recovery_plan(&plan, seal_key)?;
    Ok(ProfileAcquisitionOutcome {
        schema_version: PROFILE_ACQUISITION_OUTCOME_SCHEMA_V1.to_string(),
        state: ProfileAcquisitionState::RecoveryAvailable,
        dominant_blocker: Some(blocker),
        automatic: false,
        browser_id: None,
        daemon_session_route: None,
        recovery: Some(plan),
        next_action: Some("apply_recovery".to_string()),
        evidence,
    })
}

pub(crate) async fn apply_terminal_owner_recovery<R, F, Fut>(
    repository: &R,
    plan: &RecoveryPlan,
    now: &str,
    seal_key: &[u8],
    retry_acquisition: F,
) -> Result<RecoveryApplyOutcome, String>
where
    R: ServiceStateRepository,
    F: FnOnce(ProfileAcquisitionIntent) -> Fut,
    Fut: Future<Output = Result<ProfileAcquisitionRetryResult, String>>,
{
    verify_plan_integrity(plan, seal_key)?;
    let snapshot = repository.load_snapshot()?;
    if let Some(receipt) = snapshot.profile_recovery_receipts.get(&plan.recovery_id) {
        return replay_outcome(plan, receipt);
    }
    validate_plan_preconditions(&snapshot, plan, now)?;

    let acquired = retry_acquisition(plan.original_intent.clone()).await?;
    repository.mutate(|state| {
        if let Some(receipt) = state.profile_recovery_receipts.get(&plan.recovery_id) {
            return replay_outcome(plan, receipt);
        }
        let owner = state
            .runtime_owner_registry
            .owner(&plan.identities.profile_identity_digest)
            .ok_or_else(|| "profile_recovery_postcondition_owner_missing".to_string())?;
        let expected_generation = plan
            .identities
            .lifecycle_owner_generation
            .checked_add(1)
            .ok_or_else(|| "profile_recovery_owner_generation_exhausted".to_string())?;
        let lifecycle = state
            .runtime_owner_registry
            .lifecycle_records
            .get(&owner.browser_id)
            .ok_or_else(|| "profile_recovery_postcondition_lifecycle_missing".to_string())?;
        let principal_binding = state
            .runtime_owner_registry
            .principal_bindings
            .get(&plan.identities.profile_identity_digest);
        if owner.owner_generation != expected_generation
            || owner.browser_id != acquired.browser_id
            || owner.daemon_session_route != acquired.daemon_session_route
            || lifecycle.profile_identity_digest != plan.identities.profile_identity_digest
            || lifecycle.owner_generation != expected_generation
            || lifecycle.lifecycle_state != RuntimeLaneLifecycleState::Ready
            || lifecycle.cleanup_obligation_state != CleanupObligationState::Owned
            || principal_binding.is_none_or(|binding| {
                binding.principal_id != plan.identities.principal_id
                    || binding.profile_id != plan.identities.profile_id
                    || binding.owner_generation != expected_generation
                    || binding.provenance != ServicePrincipalProvenance::RegisteredCapability
            })
        {
            return Err("profile_recovery_postcondition_mismatch".to_string());
        }
        let receipt = RecoveryReceipt {
            schema_version: PROFILE_RECOVERY_RECEIPT_SCHEMA_V1.to_string(),
            recovery_id: plan.recovery_id.clone(),
            plan_id: plan.plan_id.clone(),
            principal_id: plan.identities.principal_id.clone(),
            profile_id: plan.identities.profile_id.clone(),
            terminal_result: "applied".to_string(),
            precondition_comparison: "matched".to_string(),
            attempted_operation_ids: plan
                .actions
                .iter()
                .map(|action| action.action_id.clone())
                .collect(),
            compensation_result: "not_required".to_string(),
            final_state_revision: state.runtime_owner_registry.revision,
            acquisition_retry_state: ProfileAcquisitionState::Acquired,
            browser_id: acquired.browser_id.clone(),
            daemon_session_route: acquired.daemon_session_route.clone(),
        };
        state
            .profile_recovery_receipts
            .insert(plan.recovery_id.clone(), receipt.clone());
        Ok(RecoveryApplyOutcome {
            acquisition: acquired_outcome(&receipt),
            receipt,
            replayed: false,
        })
    })
}

fn validate_plan_preconditions(
    state: &ServiceState,
    plan: &RecoveryPlan,
    now: &str,
) -> Result<(), String> {
    let now = chrono::DateTime::parse_from_rfc3339(now)
        .map_err(|_| "profile_recovery_now_invalid".to_string())?;
    let expires_at = chrono::DateTime::parse_from_rfc3339(&plan.expires_at)
        .map_err(|_| "profile_recovery_expiry_invalid".to_string())?;
    if now >= expires_at {
        return Err("profile_recovery_plan_expired".to_string());
    }
    if state.runtime_owner_registry.revision != plan.service_state_revision {
        return Err("profile_recovery_plan_stale".to_string());
    }
    let profile = state
        .profiles
        .get(&plan.identities.profile_id)
        .ok_or_else(|| "profile_recovery_plan_stale".to_string())?;
    let profile_digest = recovery_profile_identity_digest(profile)
        .map_err(|_| "profile_recovery_plan_stale".to_string())?;
    let owner = state
        .runtime_owner_registry
        .owner(&plan.identities.profile_identity_digest)
        .ok_or_else(|| "profile_recovery_plan_stale".to_string())?;
    let lifecycle = state
        .runtime_owner_registry
        .lifecycle_records
        .get(&plan.identities.durable_browser_id)
        .ok_or_else(|| "profile_recovery_plan_stale".to_string())?;
    let active_profile_lease_session_ids =
        active_profile_lease_session_ids(state, &plan.identities.profile_id);
    let current_process_proven = state
        .browsers
        .get(&plan.identities.durable_browser_id)
        .is_some_and(|browser| browser.pid.is_some());
    let exact = profile_digest == plan.identities.profile_identity_digest
        && plan.original_intent.principal_id == plan.identities.principal_id
        && plan.original_intent.profile_id == plan.identities.profile_id
        && owner.owner_id == plan.identities.lifecycle_owner_id
        && owner.owner_generation == plan.identities.lifecycle_owner_generation
        && owner.browser_id == plan.identities.durable_browser_id
        && owner.daemon_session_route == plan.identities.daemon_session_route
        && owner.process_instance_digest == plan.identities.process_instance_digest
        && owner.pending_transfer.is_none()
        && lifecycle.profile_identity_digest == plan.identities.profile_identity_digest
        && lifecycle.owner_generation == plan.identities.lifecycle_owner_generation
        && lifecycle.lifecycle_state == RuntimeLaneLifecycleState::Terminal
        && lifecycle.cleanup_obligation_state == CleanupObligationState::Satisfied
        && lifecycle
            .terminal_evidence
            .iter()
            .any(|evidence| evidence == "exact_process_exited")
        && lifecycle
            .terminal_evidence
            .iter()
            .any(|evidence| evidence == "profile_lock_released")
        && active_profile_lease_session_ids.is_empty()
        && !current_process_proven;
    if !exact {
        return Err("profile_recovery_plan_stale".to_string());
    }
    Ok(())
}

fn active_profile_lease_session_ids(state: &ServiceState, profile_id: &str) -> Vec<String> {
    state
        .sessions
        .values()
        .filter(|session| {
            session.profile_id.as_deref() == Some(profile_id)
                && matches!(
                    session.lease,
                    super::service_model::LeaseState::Shared
                        | super::service_model::LeaseState::Exclusive
                        | super::service_model::LeaseState::HumanTakeover
                )
        })
        .map(|session| session.id.clone())
        .collect()
}

fn verify_plan_integrity(plan: &RecoveryPlan, seal_key: &[u8]) -> Result<(), String> {
    let action = plan
        .actions
        .first()
        .filter(|_| plan.actions.len() == 1)
        .ok_or_else(|| "profile_recovery_plan_invalid".to_string())?;
    let expected_plan_id = digest_json(&(
        PROFILE_RECOVERY_PLAN_SCHEMA_V1,
        &plan.recovery_id,
        plan.service_state_revision,
        &plan.identities,
        &plan.dominant_blocker,
        action,
        &plan.original_intent,
        plan.created_at.as_str(),
        plan.expires_at.as_str(),
    ))?;
    let expected_seal = seal_recovery_plan(plan, seal_key)?;
    if plan.schema_version != PROFILE_RECOVERY_PLAN_SCHEMA_V1
        || plan.plan_id != expected_plan_id
        || plan.integrity_seal != expected_seal
    {
        return Err("profile_recovery_plan_integrity_mismatch".to_string());
    }
    Ok(())
}

fn seal_recovery_plan(plan: &RecoveryPlan, seal_key: &[u8]) -> Result<String, String> {
    if seal_key.len() < 32 {
        return Err("profile_recovery_plan_invalid".to_string());
    }
    let mut projection = serde_json::to_value(plan)
        .map_err(|error| format!("profile_recovery_contract_encode_failed:{error}"))?;
    projection["integritySeal"] = serde_json::Value::String(String::new());
    let encoded = serde_json::to_vec(&projection)
        .map_err(|error| format!("profile_recovery_contract_encode_failed:{error}"))?;
    let mut hasher = Sha256::new();
    hasher.update(b"agent-browser.profile-recovery-seal.v1\0");
    hasher.update(seal_key);
    hasher.update(b"\0");
    hasher.update(encoded);
    hasher.update(b"\0");
    hasher.update(seal_key);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn replay_outcome(
    plan: &RecoveryPlan,
    receipt: &RecoveryReceipt,
) -> Result<RecoveryApplyOutcome, String> {
    if receipt.plan_id != plan.plan_id
        || receipt.recovery_id != plan.recovery_id
        || receipt.principal_id != plan.identities.principal_id
        || receipt.profile_id != plan.identities.profile_id
        || receipt.terminal_result != "applied"
    {
        return Err("profile_recovery_receipt_conflict".to_string());
    }
    Ok(RecoveryApplyOutcome {
        acquisition: acquired_outcome(receipt),
        receipt: receipt.clone(),
        replayed: true,
    })
}

fn acquired_outcome(receipt: &RecoveryReceipt) -> ProfileAcquisitionOutcome {
    ProfileAcquisitionOutcome {
        schema_version: PROFILE_ACQUISITION_OUTCOME_SCHEMA_V1.to_string(),
        state: ProfileAcquisitionState::Acquired,
        dominant_blocker: None,
        automatic: true,
        browser_id: Some(receipt.browser_id.clone()),
        daemon_session_route: Some(receipt.daemon_session_route.clone()),
        recovery: None,
        next_action: None,
        evidence: vec![RecoveryEvidence {
            code: "recovery_receipt_persisted".to_string(),
            subject_id: receipt.recovery_id.clone(),
        }],
    }
}

fn blocked_outcome(
    browser_id: String,
    lifecycle: &crate::runtime_owner_transfer::RuntimeLifecycleRecord,
) -> ProfileAcquisitionOutcome {
    ProfileAcquisitionOutcome {
        schema_version: PROFILE_ACQUISITION_OUTCOME_SCHEMA_V1.to_string(),
        state: ProfileAcquisitionState::Blocked,
        dominant_blocker: Some(DominantBlocker {
            code: "terminal_owner_evidence_incomplete".to_string(),
            recoverable: false,
            detail: "The retained lifecycle owner lacks exact terminal cleanup and process-absence proof."
                .to_string(),
        }),
        automatic: false,
        browser_id: None,
        daemon_session_route: None,
        recovery: None,
        next_action: Some("inspect_lifecycle_owner".to_string()),
        evidence: lifecycle
            .terminal_evidence
            .iter()
            .map(|code| RecoveryEvidence {
                code: code.clone(),
                subject_id: browser_id.clone(),
            })
            .collect(),
    }
}

fn blocked_outcome_with_recourse(
    code: &str,
    recoverable: bool,
    detail: &str,
    next_action: &str,
    evidence: Vec<RecoveryEvidence>,
) -> ProfileAcquisitionOutcome {
    ProfileAcquisitionOutcome {
        schema_version: PROFILE_ACQUISITION_OUTCOME_SCHEMA_V1.to_string(),
        state: ProfileAcquisitionState::Blocked,
        dominant_blocker: Some(DominantBlocker {
            code: code.to_string(),
            recoverable,
            detail: detail.to_string(),
        }),
        automatic: false,
        browser_id: None,
        daemon_session_route: None,
        recovery: None,
        next_action: Some(next_action.to_string()),
        evidence,
    }
}

fn digest_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn digest_json(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| format!("profile_recovery_contract_encode_failed:{error}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::native::runtime_lifecycle::{ManagedLaneRegistration, RuntimeLifecycleAuthority};
    use crate::native::service_model::{
        BrowserProcess, BrowserProfile, BrowserSession, LeaseState,
    };
    use crate::native::service_store::ServiceStateRepository;
    use crate::runtime_owner_transfer::{
        ProfileOwner, ProfileOwnerState, RuntimeLifecycleRecord, RuntimeOwnerRegistry,
    };

    fn state() -> ServiceState {
        let profile_path = "/tmp/agent-browser-p137/recovery-contract";
        let profile_identity_digest = crate::runtime_profile::canonical_profile_identity_digest(
            std::path::Path::new(profile_path),
        )
        .unwrap();
        let browser_id = "session:durable-browser".to_string();
        ServiceState {
            profiles: BTreeMap::from([(
                "last30days-facebook".to_string(),
                BrowserProfile {
                    id: "last30days-facebook".to_string(),
                    name: "Last30days Facebook".to_string(),
                    user_data_dir: Some(profile_path.to_string()),
                    ..BrowserProfile::default()
                },
            )]),
            runtime_owner_registry: RuntimeOwnerRegistry {
                revision: 55,
                owners: BTreeMap::from([(
                    profile_identity_digest.clone(),
                    ProfileOwner {
                        owner_id: "owner:generation-55".to_string(),
                        profile_identity_digest: profile_identity_digest.clone(),
                        state: ProfileOwnerState::Ready,
                        owner_generation: 55,
                        browser_id: browser_id.clone(),
                        daemon_session_route: "handoff-a79ef2887412addf".to_string(),
                        process_instance_digest: digest_text("old-process"),
                        browser_family: "chrome".to_string(),
                        cdp_endpoint_identity_digest: digest_text("old-cdp"),
                        target_set_digest: digest_text("old-targets"),
                        pending_transfer: None,
                        last_transition: None,
                    },
                )]),
                lifecycle_records: BTreeMap::from([(
                    browser_id.clone(),
                    RuntimeLifecycleRecord {
                        logical_browser_id: browser_id,
                        profile_identity_digest,
                        owner_generation: 55,
                        lifecycle_state: RuntimeLaneLifecycleState::Terminal,
                        cleanup_obligation_state: CleanupObligationState::Satisfied,
                        terminal_evidence: vec![
                            "exact_process_exited".to_string(),
                            "profile_lock_released".to_string(),
                        ],
                        ..RuntimeLifecycleRecord::default()
                    },
                )]),
                ..RuntimeOwnerRegistry::default()
            },
            ..ServiceState::default()
        }
    }

    #[test]
    fn retry_route_reuses_exact_owner_and_falls_back_only_without_owner() {
        let owned = state();
        let authority = AuthenticatedServicePrincipal {
            principal_id: "principal:last30days".to_string(),
            profile_id: "last30days-facebook".to_string(),
            capability_id: "capability:test".to_string(),
            capability_revision: 1,
            provenance: ServicePrincipalProvenance::RegisteredCapability,
        };
        assert_eq!(
            profile_acquisition_retry_route(&owned, &authority).unwrap(),
            "handoff-a79ef2887412addf"
        );

        let mut unowned = owned;
        unowned.runtime_owner_registry = RuntimeOwnerRegistry::default();
        let cold_route = profile_acquisition_retry_route(&unowned, &authority).unwrap();
        assert_ne!(cold_route, "default");
        assert!(cold_route.starts_with("principal-profile-"));
        assert!(crate::validation::is_valid_session_name(&cold_route));
    }

    fn intent() -> ProfileAcquisitionIntent {
        ProfileAcquisitionIntent {
            principal_id: "principal:last30days".to_string(),
            profile_id: "last30days-facebook".to_string(),
            service_name: "Last30days".to_string(),
            agent_name: "last30days-agent".to_string(),
            task_name: "acquire-facebook-profile".to_string(),
            target_service_ids: vec!["facebook".to_string()],
        }
    }

    #[test]
    fn acquisition_retry_command_preserves_authenticated_principal_identity() {
        let command = profile_acquisition_retry_command(&intent(), "recovery-route");

        assert_eq!(command["action"], "tab_new");
        assert_eq!(command["sessionName"], "recovery-route");
        assert_eq!(command["profileId"], "last30days-facebook");
        assert_eq!(command["servicePrincipalId"], "principal:last30days");
        assert_eq!(
            command["servicePrincipalProvenance"],
            "registered_capability"
        );
    }

    #[test]
    fn acquisition_retry_command_uses_principal_as_empty_service_name_fallback() {
        let mut intent = intent();
        intent.service_name.clear();

        let command = profile_acquisition_retry_command(&intent, "recovery-route");

        assert_eq!(command["serviceName"], "principal:last30days");
        assert_eq!(command["servicePrincipalId"], "principal:last30days");
    }

    #[derive(Clone)]
    struct MemoryRepository(Arc<Mutex<ServiceState>>);

    impl MemoryRepository {
        fn new(state: ServiceState) -> Self {
            Self(Arc::new(Mutex::new(state)))
        }
    }

    impl ServiceStateRepository for MemoryRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn mutate<T>(
            &self,
            mutator: impl FnOnce(&mut ServiceState) -> Result<T, String>,
        ) -> Result<T, String> {
            let mut state = self.0.lock().unwrap();
            let mut candidate = state.clone();
            let result = mutator(&mut candidate)?;
            *state = candidate;
            Ok(result)
        }
    }

    fn seal_key() -> &'static [u8] {
        b"synthetic-profile-capability-seal-key-with-at-least-thirty-two-bytes"
    }

    fn ready_state(principal_id: &str, include_binding: bool) -> ServiceState {
        let mut state = state();
        let profile_identity_digest = state
            .runtime_owner_registry
            .owners
            .keys()
            .next()
            .unwrap()
            .clone();
        let owner = state
            .runtime_owner_registry
            .owners
            .get(&profile_identity_digest)
            .unwrap()
            .clone();
        let lifecycle = state
            .runtime_owner_registry
            .lifecycle_records
            .get_mut(&owner.browser_id)
            .unwrap();
        lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Ready;
        lifecycle.cleanup_obligation_state = CleanupObligationState::Owned;
        lifecycle.terminal_evidence.clear();
        state.browsers.insert(
            owner.browser_id.clone(),
            BrowserProcess {
                id: owner.browser_id.clone(),
                pid: Some(7137),
                ..BrowserProcess::default()
            },
        );
        if include_binding {
            state.runtime_owner_registry.principal_bindings.insert(
                profile_identity_digest.clone(),
                crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                    principal_id: principal_id.to_string(),
                    profile_id: "last30days-facebook".to_string(),
                    profile_identity_digest,
                    capability_id: "capability:test".to_string(),
                    provenance:
                        crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability,
                    owner_generation: owner.owner_generation,
                },
            );
        }
        state
    }

    #[test]
    fn acquisition_coordinator_reuses_current_process_backed_principal_lane() {
        let state = ready_state("principal:last30days", true);
        let outcome = plan_profile_acquisition(
            &state,
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-ready",
            seal_key(),
        )
        .unwrap();

        assert_eq!(outcome.state, ProfileAcquisitionState::Acquired);
        assert_eq!(
            outcome.browser_id.as_deref(),
            Some("session:durable-browser")
        );
        assert_eq!(
            outcome.daemon_session_route.as_deref(),
            Some("handoff-a79ef2887412addf")
        );
        assert!(outcome.recovery.is_none());
    }

    #[test]
    fn acquisition_coordinator_hard_blocks_current_foreign_principal() {
        let state = ready_state("principal:foreign", true);
        let outcome = plan_profile_acquisition(
            &state,
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-foreign",
            seal_key(),
        )
        .unwrap();

        assert_eq!(outcome.state, ProfileAcquisitionState::Blocked);
        assert_eq!(
            outcome.dominant_blocker.unwrap().code,
            "live_foreign_principal_authority"
        );
        assert_eq!(
            outcome.next_action.as_deref(),
            Some("wait_or_coordinate_with_current_principal")
        );
        assert!(outcome.recovery.is_none());
    }

    #[test]
    fn acquisition_coordinator_offers_reviewed_identity_reconciliation_without_launch() {
        let state = ready_state("principal:last30days", false);
        let outcome = plan_profile_acquisition(
            &state,
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-unproven",
            seal_key(),
        )
        .unwrap();

        assert_eq!(outcome.state, ProfileAcquisitionState::RecoveryAvailable);
        assert!(!outcome.automatic);
        assert_eq!(
            outcome.dominant_blocker.unwrap().code,
            "existing_session_profile_identity_unproven"
        );
        assert_eq!(
            outcome.recovery.unwrap().actions[0].action_type,
            MitigationActionType::ReconcileExactPrincipalProfileIdentity
        );
    }

    #[test]
    fn mitigation_registry_covers_every_plan_0137_fixture_recovery_class() {
        let fixtures = [
            include_str!("../../../docs/dev/fixtures/profile-recovery/plan-0137-terminal-owner.v1.json"),
            include_str!("../../../docs/dev/fixtures/profile-recovery/plan-0137-odollo-contractor-portal.v1.json"),
            include_str!("../../../docs/dev/fixtures/profile-recovery/plan-0137-soylei-identity-inconsistent.v1.json"),
            include_str!("../../../docs/dev/fixtures/profile-recovery/plan-0137-soylei-legacy-principal.v1.json"),
            include_str!("../../../docs/dev/fixtures/profile-recovery/plan-0137-soylei-owner-binding-missing.v1.json"),
            include_str!("../../../docs/dev/fixtures/profile-recovery/plan-0137-soylei-owner-generation-mismatch.v1.json"),
            include_str!("../../../docs/dev/fixtures/profile-recovery/plan-0137-fictitious-browser-cdp.v1.json"),
            include_str!("../../../docs/dev/fixtures/profile-recovery/plan-0137-fictitious-odollo-ups.v1.json"),
            include_str!("../../../docs/dev/fixtures/profile-recovery/plan-0137-cdp-free-seeding-route.v1.json"),
        ];
        let registry = mitigation_action_registry();
        for encoded in fixtures {
            let fixture: Value = serde_json::from_str(encoded).unwrap();
            let recovery_class = fixture["expectedRecoveryClass"].as_str().unwrap();
            assert!(
                registry
                    .iter()
                    .any(|descriptor| descriptor.recovery_class == recovery_class),
                "missing registry action for {recovery_class}"
            );
        }
    }

    #[test]
    fn mitigation_dominance_keeps_live_foreign_authority_above_recoverable_defects() {
        assert_eq!(
            dominant_profile_blocker([
                "expired_ownerless_lease",
                "runtime_owner_principal_binding_missing",
                "live_foreign_principal_authority",
            ])
            .as_deref(),
            Some("live_foreign_principal_authority")
        );
    }

    #[test]
    fn mitigation_registry_classifies_every_dominant_blocker_with_exact_scope() {
        let registry = mitigation_action_registry();
        for blocker in profile_blocker_dominance_order() {
            if *blocker == "live_foreign_principal_authority" {
                continue;
            }
            let descriptors = registry
                .iter()
                .filter(|descriptor| descriptor.blocker_codes.iter().any(|code| code == blocker))
                .collect::<Vec<_>>();
            assert!(!descriptors.is_empty(), "actionless blocker: {blocker}");
            assert!(descriptors.iter().all(|descriptor| {
                descriptor.effect_authority == RecoveryEffectAuthority::ExactProfileGraph
                    && !descriptor.executor_action.is_empty()
                    && !descriptor.preconditions.is_empty()
                    && !descriptor.compensation.is_empty()
            }));
        }
    }

    #[test]
    fn terminal_owner_plan_is_sealed_deterministic_and_zero_effect() {
        let state = state();
        let before = state.clone();
        let first = plan_terminal_owner_recovery(
            &state,
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-1",
            seal_key(),
        )
        .unwrap();
        let replay = plan_terminal_owner_recovery(
            &state,
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-1",
            seal_key(),
        )
        .unwrap();

        assert_eq!(state, before);
        assert_eq!(first, replay);
        assert_eq!(first.state, ProfileAcquisitionState::RecoveryAvailable);
        let plan = first.recovery.unwrap();
        assert_eq!(plan.schema_version, PROFILE_RECOVERY_PLAN_SCHEMA_V1);
        assert_eq!(plan.service_state_revision, 55);
        assert_eq!(plan.identities.lifecycle_owner_generation, 55);
        assert_eq!(
            plan.identities.durable_browser_id,
            "session:durable-browser"
        );
        assert_eq!(
            plan.identities.daemon_session_route,
            "handoff-a79ef2887412addf"
        );
        assert_eq!(plan.integrity_seal.len(), 71);
        assert!(plan.integrity_seal.starts_with("sha256:"));
        assert_eq!(plan.actions.len(), 1);
        assert_eq!(
            plan.actions[0].action_type,
            MitigationActionType::SupersedeTerminalOwner
        );
    }

    #[test]
    fn terminal_owner_plan_preserves_secondary_evidence_when_blocked() {
        let mut state = state();
        state
            .runtime_owner_registry
            .lifecycle_records
            .values_mut()
            .next()
            .unwrap()
            .terminal_evidence = vec!["profile_lock_released".to_string()];

        let outcome = plan_terminal_owner_recovery(
            &state,
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-2",
            seal_key(),
        )
        .unwrap();

        assert_eq!(outcome.state, ProfileAcquisitionState::Blocked);
        assert_eq!(
            outcome.dominant_blocker.unwrap().code,
            "terminal_owner_evidence_incomplete"
        );
        assert_eq!(outcome.evidence[0].code, "profile_lock_released");
        assert!(outcome.recovery.is_none());
    }

    #[tokio::test]
    async fn apply_retries_once_persists_receipt_and_replays_without_effect() {
        let repository = MemoryRepository::new(state());
        let plan = plan_terminal_owner_recovery(
            &repository.load_snapshot().unwrap(),
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-apply-1",
            seal_key(),
        )
        .unwrap()
        .recovery
        .unwrap();
        let profile_root = std::path::PathBuf::from("/tmp/agent-browser-p137/recovery-contract");
        let first = apply_terminal_owner_recovery(
            &repository,
            &plan,
            "2026-08-28T12:01:00Z",
            seal_key(),
            |_| async {
                let authority = RuntimeLifecycleAuthority::new(&repository);
                let binding = authority.register_managed_lane(ManagedLaneRegistration {
                    logical_browser_id: "session:replacement-browser".to_string(),
                    profile_root,
                    daemon_session_route: "replacement-route".to_string(),
                    process_group_id: Some(7137),
                    process_identity: crate::process_identity::RecordedProcessIdentity {
                        pid: 7137,
                        start_token: "linux:boot:7137".to_string(),
                        executable_path: Some("/opt/agent-browser/chrome".to_string()),
                        browser_family: Some("chrome".to_string()),
                    },
                    browser_family: "chrome".to_string(),
                    cdp_endpoint: "ws://127.0.0.1:9731/devtools/browser/replacement".to_string(),
                    target_ids: vec!["facebook".to_string()],
                })?;
                repository.mutate(|state| {
                    state
                        .runtime_owner_registry
                        .bind_principal_authority(
                            crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                                principal_id: "principal:last30days".to_string(),
                                profile_id: "last30days-facebook".to_string(),
                                profile_identity_digest: binding
                                    .claim
                                    .profile_identity_digest
                                    .clone(),
                                capability_id: "capability:test".to_string(),
                                provenance: ServicePrincipalProvenance::RegisteredCapability,
                                owner_generation: binding.claim.owner_generation,
                            },
                        )
                        .map_err(|error| format!("{error:?}"))?;
                    Ok(())
                })?;
                Ok(ProfileAcquisitionRetryResult {
                    browser_id: binding.claim.logical_browser_id,
                    daemon_session_route: binding.claim.daemon_session_route,
                })
            },
        )
        .await
        .unwrap();

        assert!(!first.replayed);
        assert_eq!(first.acquisition.state, ProfileAcquisitionState::Acquired);
        assert_eq!(first.receipt.terminal_result, "applied");
        assert_eq!(first.receipt.browser_id, "session:replacement-browser");
        assert_eq!(first.receipt.daemon_session_route, "replacement-route");
        let persisted = repository.load_snapshot().unwrap();
        assert_eq!(
            persisted
                .profile_recovery_receipts
                .get(&plan.recovery_id)
                .unwrap(),
            &first.receipt
        );
        let owner = persisted
            .runtime_owner_registry
            .owner(&plan.identities.profile_identity_digest)
            .unwrap();
        assert_eq!(owner.owner_generation, 56);
        assert_eq!(owner.browser_id, "session:replacement-browser");

        let replay = apply_terminal_owner_recovery(
            &repository,
            &plan,
            "2026-08-28T12:02:00Z",
            seal_key(),
            |_| async { panic!("receipt replay must not retry acquisition") },
        )
        .await
        .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.receipt, first.receipt);
    }

    #[tokio::test]
    async fn coordinator_auto_applies_only_conclusive_terminal_recovery_and_retries_once() {
        let repository = MemoryRepository::new(state());
        let profile_root = std::path::PathBuf::from("/tmp/agent-browser-p137/recovery-contract");
        let outcome = coordinate_profile_acquisition(
            &repository,
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-coordinate-1",
            seal_key(),
            AcquisitionRecoveryPolicy::AutoApplyConclusive,
            |_| async {
                let authority = RuntimeLifecycleAuthority::new(&repository);
                let binding = authority.register_managed_lane(ManagedLaneRegistration {
                    logical_browser_id: "session:coordinator-browser".to_string(),
                    profile_root,
                    daemon_session_route: "coordinator-route".to_string(),
                    process_group_id: Some(8137),
                    process_identity: crate::process_identity::RecordedProcessIdentity {
                        pid: 8137,
                        start_token: "linux:boot:8137".to_string(),
                        executable_path: Some("/opt/agent-browser/chrome".to_string()),
                        browser_family: Some("chrome".to_string()),
                    },
                    browser_family: "chrome".to_string(),
                    cdp_endpoint: "ws://127.0.0.1:9831/devtools/browser/replacement".to_string(),
                    target_ids: vec!["facebook".to_string()],
                })?;
                repository.mutate(|state| {
                    state
                        .runtime_owner_registry
                        .bind_principal_authority(
                            crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                                principal_id: "principal:last30days".to_string(),
                                profile_id: "last30days-facebook".to_string(),
                                profile_identity_digest: binding
                                    .claim
                                    .profile_identity_digest
                                    .clone(),
                                capability_id: "capability:test".to_string(),
                                provenance: ServicePrincipalProvenance::RegisteredCapability,
                                owner_generation: binding.claim.owner_generation,
                            },
                        )
                        .map_err(|error| format!("{error:?}"))?;
                    Ok(())
                })?;
                Ok(ProfileAcquisitionRetryResult {
                    browser_id: binding.claim.logical_browser_id,
                    daemon_session_route: binding.claim.daemon_session_route,
                })
            },
        )
        .await
        .unwrap();

        assert_eq!(outcome.state, ProfileAcquisitionState::Acquired);
        assert!(outcome.automatic);
        assert_eq!(
            outcome.browser_id.as_deref(),
            Some("session:coordinator-browser")
        );
        assert_eq!(
            repository
                .load_snapshot()
                .unwrap()
                .profile_recovery_receipts
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn coordinator_registers_an_initial_owner_for_an_uncontested_profile() {
        let mut initial = state();
        initial.runtime_owner_registry = RuntimeOwnerRegistry::default();
        let repository = MemoryRepository::new(initial);
        let profile_root = std::path::PathBuf::from("/tmp/agent-browser-p137/recovery-contract");
        let retries = std::cell::Cell::new(0);

        let outcome = coordinate_profile_acquisition(
            &repository,
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-initial-owner-1",
            seal_key(),
            AcquisitionRecoveryPolicy::AutoApplyConclusive,
            |_| async {
                retries.set(retries.get() + 1);
                let authority = RuntimeLifecycleAuthority::new(&repository);
                let binding = authority.register_managed_lane(ManagedLaneRegistration {
                    logical_browser_id: "session:initial-browser".to_string(),
                    profile_root,
                    daemon_session_route: "initial-route".to_string(),
                    process_group_id: Some(9137),
                    process_identity: crate::process_identity::RecordedProcessIdentity {
                        pid: 9137,
                        start_token: "linux:boot:9137".to_string(),
                        executable_path: Some("/opt/agent-browser/chrome".to_string()),
                        browser_family: Some("chrome".to_string()),
                    },
                    browser_family: "chrome".to_string(),
                    cdp_endpoint: "ws://127.0.0.1:9931/devtools/browser/initial".to_string(),
                    target_ids: vec!["facebook".to_string()],
                })?;
                repository.mutate(|state| {
                    state.browsers.insert(
                        binding.claim.logical_browser_id.clone(),
                        BrowserProcess {
                            id: binding.claim.logical_browser_id.clone(),
                            pid: Some(9137),
                            ..BrowserProcess::default()
                        },
                    );
                    state.runtime_owner_registry.bind_principal_authority(
                        crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                            principal_id: "principal:last30days".to_string(),
                            profile_id: "last30days-facebook".to_string(),
                            profile_identity_digest: binding.claim.profile_identity_digest.clone(),
                            capability_id: "capability:test".to_string(),
                            provenance: crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability,
                            owner_generation: binding.claim.owner_generation,
                        },
                    )
                    .map_err(|error| format!("{:?}", error.code))?;
                    Ok(())
                })?;
                Ok(ProfileAcquisitionRetryResult {
                    browser_id: binding.claim.logical_browser_id,
                    daemon_session_route: binding.claim.daemon_session_route,
                })
            },
        )
        .await
        .unwrap();

        assert_eq!(retries.get(), 1);
        assert_eq!(outcome.state, ProfileAcquisitionState::Acquired);
        assert!(outcome.automatic);
        assert_eq!(
            outcome.browser_id.as_deref(),
            Some("session:initial-browser")
        );
        assert!(outcome
            .evidence
            .iter()
            .any(|item| item.code == "initial_owner_registered"));
    }

    #[tokio::test]
    async fn stale_plan_fails_before_retry_effect() {
        let repository = MemoryRepository::new(state());
        let plan = plan_terminal_owner_recovery(
            &repository.load_snapshot().unwrap(),
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-stale-1",
            seal_key(),
        )
        .unwrap()
        .recovery
        .unwrap();
        repository
            .mutate(|state| {
                state.runtime_owner_registry.revision += 1;
                Ok(())
            })
            .unwrap();
        let retries = std::cell::Cell::new(0);

        let error = apply_terminal_owner_recovery(
            &repository,
            &plan,
            "2026-08-28T12:01:00Z",
            seal_key(),
            |_| async {
                retries.set(retries.get() + 1);
                Err("must not run".to_string())
            },
        )
        .await
        .unwrap_err();

        assert_eq!(error, "profile_recovery_plan_stale");
        assert_eq!(retries.get(), 0);
        assert!(repository
            .load_snapshot()
            .unwrap()
            .profile_recovery_receipts
            .is_empty());
    }

    #[tokio::test]
    async fn altered_seal_fails_before_retry_effect() {
        let repository = MemoryRepository::new(state());
        let mut plan = plan_terminal_owner_recovery(
            &repository.load_snapshot().unwrap(),
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-seal-1",
            seal_key(),
        )
        .unwrap()
        .recovery
        .unwrap();
        plan.identities.daemon_session_route = "tampered-route".to_string();

        let error = apply_terminal_owner_recovery(
            &repository,
            &plan,
            "2026-08-28T12:01:00Z",
            seal_key(),
            |_| async { panic!("invalid seal must fail before retry") },
        )
        .await
        .unwrap_err();

        assert_eq!(error, "profile_recovery_plan_integrity_mismatch");
    }

    #[test]
    fn live_process_or_active_profile_lease_blocks_recovery_planning() {
        let mut live_process = state();
        live_process.browsers.insert(
            "session:durable-browser".to_string(),
            BrowserProcess {
                id: "session:durable-browser".to_string(),
                profile_id: Some("last30days-facebook".to_string()),
                pid: Some(9137),
                ..BrowserProcess::default()
            },
        );
        let live_outcome = plan_terminal_owner_recovery(
            &live_process,
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-live-1",
            seal_key(),
        )
        .unwrap();
        assert_eq!(live_outcome.state, ProfileAcquisitionState::Blocked);

        let mut active_lease = state();
        active_lease.sessions.insert(
            "foreign-session".to_string(),
            BrowserSession {
                id: "foreign-session".to_string(),
                principal_id: Some("principal:foreign".to_string()),
                profile_id: Some("last30days-facebook".to_string()),
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        );
        let lease_outcome = plan_terminal_owner_recovery(
            &active_lease,
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-lease-1",
            seal_key(),
        )
        .unwrap();
        assert_eq!(lease_outcome.state, ProfileAcquisitionState::Blocked);
    }
}
