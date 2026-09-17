//! Provider-free crash-regeneration records and deterministic transitions.

use serde::{Deserialize, Serialize};

pub const CRASH_REGENERATION_STATUS_SCHEMA_VERSION: &str =
    "agent-browser.crash-regeneration-status.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashRegenerationPhase {
    RuntimeHostAuthority,
    BrowserAuthority,
    DisplayDiscovery,
    GuacamoleRecovery,
    RouteProjection,
    DurableHandoffResolution,
    OperatorVisibleProof,
}

impl CrashRegenerationPhase {
    pub const ORDER: [Self; 7] = [
        Self::RuntimeHostAuthority,
        Self::BrowserAuthority,
        Self::DisplayDiscovery,
        Self::GuacamoleRecovery,
        Self::RouteProjection,
        Self::DurableHandoffResolution,
        Self::OperatorVisibleProof,
    ];

    pub fn operation_name(self) -> &'static str {
        match self {
            Self::RuntimeHostAuthority => "runtime_host_authority",
            Self::BrowserAuthority => "browser_authority",
            Self::DisplayDiscovery => "display_discovery",
            Self::GuacamoleRecovery => "guacamole_recovery",
            Self::RouteProjection => "route_projection",
            Self::DurableHandoffResolution => "durable_handoff_resolution",
            Self::OperatorVisibleProof => "operator_visible_proof",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashRegenerationState {
    #[default]
    Pending,
    InProgress,
    Interrupted,
    Ready,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CrashRegenerationStableIdentities {
    pub principal_id: String,
    pub profile_id: String,
    pub logical_browser_id: String,
    pub session_route: String,
    pub route_id: String,
    pub connection_id: String,
    pub route_user_id: String,
    pub handoff_id: String,
}

impl CrashRegenerationStableIdentities {
    pub fn validate(&self) -> Result<(), String> {
        if [
            &self.principal_id,
            &self.profile_id,
            &self.logical_browser_id,
            &self.session_route,
            &self.route_id,
            &self.connection_id,
            &self.route_user_id,
            &self.handoff_id,
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err("crash_regeneration_stable_identity_missing".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CrashRegenerationEvidence {
    pub runtime_host_id: Option<String>,
    pub runtime_host_pid: Option<u32>,
    pub socket_identity: Option<String>,
    pub browser_pid: Option<u32>,
    pub owner_generation: Option<u64>,
    pub display_name: Option<String>,
    pub guacamole_web_tier_generation: Option<String>,
    pub route_id: Option<String>,
    pub viewer_session_id: Option<String>,
    pub operator_visible_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CrashRegenerationTransaction {
    pub transaction_id: String,
    pub boot_epoch: String,
    pub stable_identities: CrashRegenerationStableIdentities,
    pub state: CrashRegenerationState,
    pub revision: u64,
    pub replay_count: u64,
    pub completed_phases: Vec<CrashRegenerationPhase>,
    pub current_phase: Option<CrashRegenerationPhase>,
    pub evidence: CrashRegenerationEvidence,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashRegenerationStatus {
    pub schema_version: &'static str,
    pub transaction_id: String,
    pub state: CrashRegenerationState,
    pub revision: u64,
    pub replay_count: u64,
    pub completed_phases: Vec<CrashRegenerationPhase>,
    pub current_phase: Option<CrashRegenerationPhase>,
    pub principal_id: String,
    pub profile_id: String,
    pub logical_browser_id: String,
    pub session_route: String,
    pub route_id: String,
    pub connection_id: String,
    pub route_user_id: String,
    pub handoff_id: String,
    pub operator_visible_ready: bool,
    pub recourse: &'static str,
}

pub fn crash_regeneration_statuses(
    transactions: &std::collections::BTreeMap<String, CrashRegenerationTransaction>,
) -> Vec<CrashRegenerationStatus> {
    transactions
        .values()
        .map(|transaction| CrashRegenerationStatus {
            schema_version: CRASH_REGENERATION_STATUS_SCHEMA_VERSION,
            transaction_id: transaction.transaction_id.clone(),
            state: transaction.state,
            revision: transaction.revision,
            replay_count: transaction.replay_count,
            completed_phases: transaction.completed_phases.clone(),
            current_phase: transaction.current_phase,
            principal_id: transaction.stable_identities.principal_id.clone(),
            profile_id: transaction.stable_identities.profile_id.clone(),
            logical_browser_id: transaction.stable_identities.logical_browser_id.clone(),
            session_route: transaction.stable_identities.session_route.clone(),
            route_id: transaction.stable_identities.route_id.clone(),
            connection_id: transaction.stable_identities.connection_id.clone(),
            route_user_id: transaction.stable_identities.route_user_id.clone(),
            handoff_id: transaction.stable_identities.handoff_id.clone(),
            operator_visible_ready: transaction.evidence.operator_visible_ready,
            recourse: match transaction.state {
                CrashRegenerationState::Ready => "reuse_durable_handoff",
                CrashRegenerationState::Interrupted => "resume_same_transaction",
                CrashRegenerationState::Pending | CrashRegenerationState::InProgress => {
                    "inspect_transaction_progress"
                }
            },
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashRegenerationRequest {
    pub transaction_id: String,
    pub boot_epoch: String,
    pub stable_identities: CrashRegenerationStableIdentities,
}

impl CrashRegenerationRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.transaction_id.trim().is_empty() || self.boot_epoch.trim().is_empty() {
            return Err("crash_regeneration_request_invalid".to_string());
        }
        self.stable_identities.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashRegenerationOperation {
    pub transaction_id: String,
    pub operation_id: String,
    pub boot_epoch: String,
    pub phase: CrashRegenerationPhase,
    pub stable_identities: CrashRegenerationStableIdentities,
    pub prior_evidence: CrashRegenerationEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrashRegenerationPhaseReceipt {
    RuntimeHostAuthority {
        boot_epoch: String,
        runtime_host_id: String,
        pid: u32,
        socket_identity: String,
    },
    BrowserAuthority {
        boot_epoch: String,
        logical_browser_id: String,
        pid: u32,
        owner_generation: u64,
    },
    DisplayDiscovery {
        boot_epoch: String,
        display_name: String,
    },
    GuacamoleRecovery {
        boot_epoch: String,
        web_tier_generation: String,
    },
    RouteProjection {
        route_id: String,
        connection_id: String,
        route_user_id: String,
    },
    DurableHandoffResolution {
        handoff_id: String,
    },
    OperatorVisibleProof {
        boot_epoch: String,
        viewer_session_id: String,
        ready: bool,
    },
}

pub fn next_phase(transaction: &CrashRegenerationTransaction) -> Option<CrashRegenerationPhase> {
    CrashRegenerationPhase::ORDER
        .iter()
        .copied()
        .find(|phase| !transaction.completed_phases.contains(phase))
}

pub fn begin_or_resume(
    existing: Option<&CrashRegenerationTransaction>,
    request: &CrashRegenerationRequest,
) -> Result<CrashRegenerationTransaction, String> {
    request.validate()?;
    if let Some(existing) = existing {
        if existing.boot_epoch != request.boot_epoch
            || existing.stable_identities != request.stable_identities
        {
            return Err("crash_regeneration_transaction_identity_mismatch".to_string());
        }
        let mut transaction = existing.clone();
        transaction.replay_count = transaction.replay_count.saturating_add(1);
        transaction.state = if transaction.state == CrashRegenerationState::Ready {
            CrashRegenerationState::Ready
        } else {
            CrashRegenerationState::InProgress
        };
        transaction.last_error = None;
        transaction.revision = transaction.revision.saturating_add(1);
        return Ok(transaction);
    }
    Ok(CrashRegenerationTransaction {
        transaction_id: request.transaction_id.clone(),
        boot_epoch: request.boot_epoch.clone(),
        stable_identities: request.stable_identities.clone(),
        state: CrashRegenerationState::InProgress,
        revision: 1,
        replay_count: 0,
        completed_phases: Vec::new(),
        current_phase: Some(CrashRegenerationPhase::RuntimeHostAuthority),
        evidence: CrashRegenerationEvidence::default(),
        last_error: None,
    })
}

pub fn validate_receipt(
    operation: &CrashRegenerationOperation,
    receipt: &CrashRegenerationPhaseReceipt,
) -> Result<(), String> {
    let phase_matches = matches!(
        (operation.phase, receipt),
        (
            CrashRegenerationPhase::RuntimeHostAuthority,
            CrashRegenerationPhaseReceipt::RuntimeHostAuthority { .. }
        ) | (
            CrashRegenerationPhase::BrowserAuthority,
            CrashRegenerationPhaseReceipt::BrowserAuthority { .. }
        ) | (
            CrashRegenerationPhase::DisplayDiscovery,
            CrashRegenerationPhaseReceipt::DisplayDiscovery { .. }
        ) | (
            CrashRegenerationPhase::GuacamoleRecovery,
            CrashRegenerationPhaseReceipt::GuacamoleRecovery { .. }
        ) | (
            CrashRegenerationPhase::RouteProjection,
            CrashRegenerationPhaseReceipt::RouteProjection { .. }
        ) | (
            CrashRegenerationPhase::DurableHandoffResolution,
            CrashRegenerationPhaseReceipt::DurableHandoffResolution { .. }
        ) | (
            CrashRegenerationPhase::OperatorVisibleProof,
            CrashRegenerationPhaseReceipt::OperatorVisibleProof { .. }
        )
    );
    if !phase_matches {
        return Err("crash_regeneration_receipt_phase_mismatch".to_string());
    }
    let epoch = match receipt {
        CrashRegenerationPhaseReceipt::RuntimeHostAuthority { boot_epoch, .. }
        | CrashRegenerationPhaseReceipt::BrowserAuthority { boot_epoch, .. }
        | CrashRegenerationPhaseReceipt::DisplayDiscovery { boot_epoch, .. }
        | CrashRegenerationPhaseReceipt::GuacamoleRecovery { boot_epoch, .. }
        | CrashRegenerationPhaseReceipt::OperatorVisibleProof { boot_epoch, .. } => {
            Some(boot_epoch)
        }
        CrashRegenerationPhaseReceipt::RouteProjection { .. }
        | CrashRegenerationPhaseReceipt::DurableHandoffResolution { .. } => None,
    };
    if epoch.is_some_and(|epoch| epoch != &operation.boot_epoch) {
        return Err("crash_regeneration_receipt_boot_epoch_mismatch".to_string());
    }
    match receipt {
        CrashRegenerationPhaseReceipt::RuntimeHostAuthority {
            runtime_host_id,
            pid,
            socket_identity,
            ..
        } if runtime_host_id.trim().is_empty()
            || *pid == 0
            || socket_identity.trim().is_empty() =>
        {
            Err("crash_regeneration_runtime_host_receipt_invalid".to_string())
        }
        CrashRegenerationPhaseReceipt::BrowserAuthority {
            logical_browser_id,
            pid,
            owner_generation,
            ..
        } if logical_browser_id != &operation.stable_identities.logical_browser_id
            || *pid == 0
            || *owner_generation == 0 =>
        {
            Err("crash_regeneration_browser_identity_mismatch".to_string())
        }
        CrashRegenerationPhaseReceipt::DisplayDiscovery { display_name, .. }
            if display_name.trim().is_empty() =>
        {
            Err("crash_regeneration_display_receipt_invalid".to_string())
        }
        CrashRegenerationPhaseReceipt::GuacamoleRecovery {
            web_tier_generation,
            ..
        } if web_tier_generation.trim().is_empty() => {
            Err("crash_regeneration_guacamole_receipt_invalid".to_string())
        }
        CrashRegenerationPhaseReceipt::RouteProjection {
            route_id,
            connection_id,
            route_user_id,
        } if route_id.trim().is_empty()
            || route_id != &operation.stable_identities.route_id
            || connection_id != &operation.stable_identities.connection_id
            || route_user_id != &operation.stable_identities.route_user_id =>
        {
            Err("crash_regeneration_route_identity_mismatch".to_string())
        }
        CrashRegenerationPhaseReceipt::DurableHandoffResolution { handoff_id }
            if handoff_id != &operation.stable_identities.handoff_id =>
        {
            Err("crash_regeneration_handoff_identity_mismatch".to_string())
        }
        CrashRegenerationPhaseReceipt::OperatorVisibleProof {
            viewer_session_id,
            ready,
            ..
        } if viewer_session_id.trim().is_empty() || !ready => {
            Err("crash_regeneration_operator_visible_not_ready".to_string())
        }
        _ => Ok(()),
    }
}

pub fn apply_phase_receipt(
    expected: &CrashRegenerationTransaction,
    phase: CrashRegenerationPhase,
    receipt: CrashRegenerationPhaseReceipt,
) -> Result<CrashRegenerationTransaction, String> {
    if next_phase(expected) != Some(phase) {
        return Err("crash_regeneration_compare_and_swap_mismatch".to_string());
    }
    let operation = CrashRegenerationOperation {
        transaction_id: expected.transaction_id.clone(),
        operation_id: format!("{}:{}", expected.transaction_id, phase.operation_name()),
        boot_epoch: expected.boot_epoch.clone(),
        phase,
        stable_identities: expected.stable_identities.clone(),
        prior_evidence: expected.evidence.clone(),
    };
    validate_receipt(&operation, &receipt)?;
    let mut transaction = expected.clone();
    apply_receipt(&mut transaction.evidence, receipt);
    transaction.completed_phases.push(phase);
    transaction.current_phase = next_phase(&transaction);
    transaction.state = if transaction.current_phase.is_some() {
        CrashRegenerationState::InProgress
    } else {
        CrashRegenerationState::Ready
    };
    transaction.last_error = None;
    transaction.revision = transaction.revision.saturating_add(1);
    Ok(transaction)
}

pub fn interrupt(
    expected: &CrashRegenerationTransaction,
    phase: CrashRegenerationPhase,
    error: &str,
) -> Result<CrashRegenerationTransaction, String> {
    if next_phase(expected) != Some(phase) {
        return Err("crash_regeneration_compare_and_swap_mismatch".to_string());
    }
    let mut transaction = expected.clone();
    transaction.state = CrashRegenerationState::Interrupted;
    transaction.current_phase = Some(phase);
    transaction.last_error = Some(error.to_string());
    transaction.revision = transaction.revision.saturating_add(1);
    Ok(transaction)
}

pub fn finish_ready(
    expected: &CrashRegenerationTransaction,
) -> Result<CrashRegenerationTransaction, String> {
    if expected.state == CrashRegenerationState::Ready {
        return Ok(expected.clone());
    }
    if next_phase(expected).is_some() {
        return Err("crash_regeneration_compare_and_swap_mismatch".to_string());
    }
    let mut transaction = expected.clone();
    transaction.state = CrashRegenerationState::Ready;
    transaction.current_phase = None;
    transaction.last_error = None;
    transaction.revision = transaction.revision.saturating_add(1);
    Ok(transaction)
}

fn apply_receipt(evidence: &mut CrashRegenerationEvidence, receipt: CrashRegenerationPhaseReceipt) {
    match receipt {
        CrashRegenerationPhaseReceipt::RuntimeHostAuthority {
            runtime_host_id,
            pid,
            socket_identity,
            ..
        } => {
            evidence.runtime_host_id = Some(runtime_host_id);
            evidence.runtime_host_pid = Some(pid);
            evidence.socket_identity = Some(socket_identity);
        }
        CrashRegenerationPhaseReceipt::BrowserAuthority {
            pid,
            owner_generation,
            ..
        } => {
            evidence.browser_pid = Some(pid);
            evidence.owner_generation = Some(owner_generation);
        }
        CrashRegenerationPhaseReceipt::DisplayDiscovery { display_name, .. } => {
            evidence.display_name = Some(display_name)
        }
        CrashRegenerationPhaseReceipt::GuacamoleRecovery {
            web_tier_generation,
            ..
        } => evidence.guacamole_web_tier_generation = Some(web_tier_generation),
        CrashRegenerationPhaseReceipt::RouteProjection { route_id, .. } => {
            evidence.route_id = Some(route_id)
        }
        CrashRegenerationPhaseReceipt::DurableHandoffResolution { .. } => {}
        CrashRegenerationPhaseReceipt::OperatorVisibleProof {
            viewer_session_id,
            ready,
            ..
        } => {
            evidence.viewer_session_id = Some(viewer_session_id);
            evidence.operator_visible_ready = ready;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn request() -> CrashRegenerationRequest {
        CrashRegenerationRequest {
            transaction_id: "tx-1".into(),
            boot_epoch: "boot-1".into(),
            stable_identities: CrashRegenerationStableIdentities {
                principal_id: "principal-1".into(),
                profile_id: "profile-1".into(),
                logical_browser_id: "browser-1".into(),
                session_route: "session-1".into(),
                route_id: "route-1".into(),
                connection_id: "connection-1".into(),
                route_user_id: "user-1".into(),
                handoff_id: "handoff-1".into(),
            },
        }
    }

    #[test]
    fn transitions_preserve_order_and_increment_revision() {
        let request = request();
        request.validate().unwrap();
        let mut transaction = begin_or_resume(None, &request).unwrap();
        for phase in CrashRegenerationPhase::ORDER {
            assert_eq!(next_phase(&transaction), Some(phase));
            let operation = CrashRegenerationOperation {
                transaction_id: transaction.transaction_id.clone(),
                operation_id: format!("tx-1:{}", phase.operation_name()),
                boot_epoch: transaction.boot_epoch.clone(),
                phase,
                stable_identities: transaction.stable_identities.clone(),
                prior_evidence: transaction.evidence.clone(),
            };
            let receipt = match phase {
                CrashRegenerationPhase::RuntimeHostAuthority => {
                    CrashRegenerationPhaseReceipt::RuntimeHostAuthority {
                        boot_epoch: "boot-1".into(),
                        runtime_host_id: "host".into(),
                        pid: 1,
                        socket_identity: "socket".into(),
                    }
                }
                CrashRegenerationPhase::BrowserAuthority => {
                    CrashRegenerationPhaseReceipt::BrowserAuthority {
                        boot_epoch: "boot-1".into(),
                        logical_browser_id: "browser-1".into(),
                        pid: 2,
                        owner_generation: 1,
                    }
                }
                CrashRegenerationPhase::DisplayDiscovery => {
                    CrashRegenerationPhaseReceipt::DisplayDiscovery {
                        boot_epoch: "boot-1".into(),
                        display_name: ":1".into(),
                    }
                }
                CrashRegenerationPhase::GuacamoleRecovery => {
                    CrashRegenerationPhaseReceipt::GuacamoleRecovery {
                        boot_epoch: "boot-1".into(),
                        web_tier_generation: "web-1".into(),
                    }
                }
                CrashRegenerationPhase::RouteProjection => {
                    CrashRegenerationPhaseReceipt::RouteProjection {
                        route_id: "route-1".into(),
                        connection_id: "connection-1".into(),
                        route_user_id: "user-1".into(),
                    }
                }
                CrashRegenerationPhase::DurableHandoffResolution => {
                    CrashRegenerationPhaseReceipt::DurableHandoffResolution {
                        handoff_id: "handoff-1".into(),
                    }
                }
                CrashRegenerationPhase::OperatorVisibleProof => {
                    CrashRegenerationPhaseReceipt::OperatorVisibleProof {
                        boot_epoch: "boot-1".into(),
                        viewer_session_id: "viewer-1".into(),
                        ready: true,
                    }
                }
            };
            validate_receipt(&operation, &receipt).unwrap();
            transaction = apply_phase_receipt(&transaction, phase, receipt).unwrap();
        }
        assert_eq!(transaction.state, CrashRegenerationState::Ready);
        assert_eq!(transaction.revision, 8);
        assert_eq!(finish_ready(&transaction).unwrap(), transaction);
        let mut map = BTreeMap::new();
        map.insert(transaction.transaction_id.clone(), transaction);
        assert_eq!(crash_regeneration_statuses(&map).len(), 1);
    }

    #[test]
    fn identity_mismatch_and_invalid_receipt_keep_exact_errors() {
        let request = request();
        let transaction = begin_or_resume(None, &request).unwrap();
        let mut changed = request.clone();
        changed.boot_epoch = "other".into();
        assert_eq!(
            begin_or_resume(Some(&transaction), &changed).unwrap_err(),
            "crash_regeneration_transaction_identity_mismatch"
        );
        let operation = CrashRegenerationOperation {
            transaction_id: transaction.transaction_id.clone(),
            operation_id: "op".into(),
            boot_epoch: transaction.boot_epoch.clone(),
            phase: CrashRegenerationPhase::DisplayDiscovery,
            stable_identities: transaction.stable_identities.clone(),
            prior_evidence: transaction.evidence.clone(),
        };
        let receipt = CrashRegenerationPhaseReceipt::DisplayDiscovery {
            boot_epoch: "boot-1".into(),
            display_name: String::new(),
        };
        assert_eq!(
            validate_receipt(&operation, &receipt).unwrap_err(),
            "crash_regeneration_display_receipt_invalid"
        );
        assert_eq!(
            apply_phase_receipt(
                &transaction,
                CrashRegenerationPhase::RuntimeHostAuthority,
                CrashRegenerationPhaseReceipt::RuntimeHostAuthority {
                    boot_epoch: "boot-1".into(),
                    runtime_host_id: String::new(),
                    pid: 0,
                    socket_identity: String::new(),
                },
            )
            .unwrap_err(),
            "crash_regeneration_runtime_host_receipt_invalid"
        );
    }

    #[test]
    fn transitions_enforce_request_replay_interruption_and_completion_invariants() {
        let mut invalid = request();
        invalid.stable_identities.profile_id.clear();
        assert_eq!(
            begin_or_resume(None, &invalid).unwrap_err(),
            "crash_regeneration_stable_identity_missing"
        );

        let request = request();
        let transaction = begin_or_resume(None, &request).unwrap();
        let replayed = begin_or_resume(Some(&transaction), &request).unwrap();
        assert_eq!(replayed.replay_count, 1);
        assert_eq!(replayed.revision, 2);
        assert_eq!(
            finish_ready(&replayed).unwrap_err(),
            "crash_regeneration_compare_and_swap_mismatch"
        );

        let interrupted = interrupt(
            &replayed,
            CrashRegenerationPhase::RuntimeHostAuthority,
            "synthetic_interruption",
        )
        .unwrap();
        assert_eq!(interrupted.state, CrashRegenerationState::Interrupted);
        assert_eq!(
            interrupted.current_phase,
            Some(CrashRegenerationPhase::RuntimeHostAuthority)
        );
        assert_eq!(
            interrupted.last_error.as_deref(),
            Some("synthetic_interruption")
        );
        assert_eq!(interrupted.revision, 3);
    }
}
