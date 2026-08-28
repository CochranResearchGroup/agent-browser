//! Versioned, no-launch profile acquisition recovery contracts.
//!
//! Recovery planning consumes an authenticated principal identity and an
//! immutable Service State snapshot. It never persists state or launches a
//! browser. Effectful application remains behind a later repository-backed
//! compare-and-swap boundary.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::service_model::ServiceState;
use crate::runtime_owner_transfer::{CleanupObligationState, RuntimeLaneLifecycleState};

pub(crate) const PROFILE_ACQUISITION_OUTCOME_SCHEMA_V1: &str =
    "agent-browser.profile-acquisition-outcome.v1";
pub(crate) const PROFILE_RECOVERY_PLAN_SCHEMA_V1: &str = "agent-browser.profile-recovery-plan.v1";
pub(crate) const PROFILE_RECOVERY_RECEIPT_SCHEMA_V1: &str =
    "agent-browser.profile-recovery-receipt.v1";
pub(crate) const PROFILE_MITIGATION_ACTION_SCHEMA_V1: &str =
    "agent-browser.profile-mitigation-action.v1";

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecoveryEffectAuthority {
    ExactProfileGraph,
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
    pub(crate) terminal_result: String,
    pub(crate) precondition_comparison: String,
    pub(crate) attempted_operation_ids: Vec<String>,
    pub(crate) compensation_result: String,
    pub(crate) final_state_revision: u64,
    pub(crate) acquisition_retry_state: ProfileAcquisitionState,
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

pub(crate) fn plan_terminal_owner_recovery(
    state: &ServiceState,
    intent: ProfileAcquisitionIntent,
    created_at: &str,
    expires_at: &str,
    idempotency_key: &str,
) -> Result<ProfileAcquisitionOutcome, String> {
    let profile = state
        .profiles
        .get(&intent.profile_id)
        .ok_or_else(|| "profile_recovery_profile_missing".to_string())?;
    let profile_path = profile
        .user_data_dir
        .as_deref()
        .ok_or_else(|| "profile_recovery_identity_unavailable".to_string())?;
    let profile_identity_digest = crate::runtime_profile::canonical_profile_identity_digest(
        std::path::Path::new(profile_path),
    )?;
    let owner = state
        .runtime_owner_registry
        .owner(&profile_identity_digest)
        .ok_or_else(|| "profile_recovery_owner_missing".to_string())?;
    let lifecycle = state
        .runtime_owner_registry
        .lifecycle_records
        .get(&owner.browser_id)
        .ok_or_else(|| "profile_recovery_lifecycle_missing".to_string())?;
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
    let integrity_seal = digest_json(&(
        &plan_id,
        &recovery_id,
        &idempotency_key_digest,
        state.runtime_owner_registry.revision,
        &identities,
        &blocker,
        &evidence,
        &action,
        &intent,
        created_at,
        expires_at,
    ))?;
    let plan = RecoveryPlan {
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
        integrity_seal,
    };
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

    use super::*;
    use crate::native::service_model::BrowserProfile;
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
    fn terminal_owner_plan_is_sealed_deterministic_and_zero_effect() {
        let state = state();
        let before = state.clone();
        let first = plan_terminal_owner_recovery(
            &state,
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-1",
        )
        .unwrap();
        let replay = plan_terminal_owner_recovery(
            &state,
            intent(),
            "2026-08-28T12:00:00Z",
            "2026-08-28T12:05:00Z",
            "request-1",
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
        assert_eq!(plan.integrity_seal.len(), 64);
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
}
