//! Idempotent, dependency-ordered crash regeneration for one managed lane.
//!
//! Effects are delegated through a phase adapter and keyed by a stable
//! transaction/phase operation id. The coordinator persists a compare-and-swap
//! receipt after every phase so interruption resumes at the first incomplete
//! dependency without changing stable identities or duplicating resources.

use super::service_store::ServiceStateRepository;
use serde::{Deserialize, Serialize};

pub(crate) const CRASH_REGENERATION_STATUS_SCHEMA_VERSION: &str =
    "agent-browser.crash-regeneration-status.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CrashRegenerationPhase {
    RuntimeHostAuthority,
    BrowserAuthority,
    DisplayDiscovery,
    GuacamoleRecovery,
    RouteProjection,
    DurableHandoffResolution,
    OperatorVisibleProof,
}

impl CrashRegenerationPhase {
    const ORDER: [Self; 7] = [
        Self::RuntimeHostAuthority,
        Self::BrowserAuthority,
        Self::DisplayDiscovery,
        Self::GuacamoleRecovery,
        Self::RouteProjection,
        Self::DurableHandoffResolution,
        Self::OperatorVisibleProof,
    ];

    fn operation_name(self) -> &'static str {
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
pub(crate) enum CrashRegenerationState {
    #[default]
    Pending,
    InProgress,
    Interrupted,
    Ready,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CrashRegenerationStableIdentities {
    pub(crate) principal_id: String,
    pub(crate) profile_id: String,
    pub(crate) logical_browser_id: String,
    pub(crate) session_route: String,
    pub(crate) route_id: String,
    pub(crate) connection_id: String,
    pub(crate) route_user_id: String,
    pub(crate) handoff_id: String,
}

impl CrashRegenerationStableIdentities {
    fn validate(&self) -> Result<(), String> {
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
pub(crate) struct CrashRegenerationEvidence {
    pub(crate) runtime_host_id: Option<String>,
    pub(crate) runtime_host_pid: Option<u32>,
    pub(crate) socket_identity: Option<String>,
    pub(crate) browser_pid: Option<u32>,
    pub(crate) owner_generation: Option<u64>,
    pub(crate) display_name: Option<String>,
    pub(crate) guacamole_web_tier_generation: Option<String>,
    pub(crate) route_id: Option<String>,
    pub(crate) viewer_session_id: Option<String>,
    pub(crate) operator_visible_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CrashRegenerationTransaction {
    pub(crate) transaction_id: String,
    pub(crate) boot_epoch: String,
    pub(crate) stable_identities: CrashRegenerationStableIdentities,
    pub(crate) state: CrashRegenerationState,
    pub(crate) revision: u64,
    pub(crate) replay_count: u64,
    pub(crate) completed_phases: Vec<CrashRegenerationPhase>,
    pub(crate) current_phase: Option<CrashRegenerationPhase>,
    pub(crate) evidence: CrashRegenerationEvidence,
    pub(crate) last_error: Option<String>,
}

/// Public, stable-identity-only projection of one recovery transaction.
///
/// Host PIDs, socket identities, displays, viewer sessions, and provider
/// generations remain private Service State evidence and never cross a public
/// status response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CrashRegenerationStatus {
    pub(crate) schema_version: &'static str,
    pub(crate) transaction_id: String,
    pub(crate) state: CrashRegenerationState,
    pub(crate) revision: u64,
    pub(crate) replay_count: u64,
    pub(crate) completed_phases: Vec<CrashRegenerationPhase>,
    pub(crate) current_phase: Option<CrashRegenerationPhase>,
    pub(crate) principal_id: String,
    pub(crate) profile_id: String,
    pub(crate) logical_browser_id: String,
    pub(crate) session_route: String,
    pub(crate) route_id: String,
    pub(crate) connection_id: String,
    pub(crate) route_user_id: String,
    pub(crate) handoff_id: String,
    pub(crate) operator_visible_ready: bool,
    pub(crate) recourse: &'static str,
}

pub(crate) fn crash_regeneration_statuses(
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
pub(crate) struct CrashRegenerationRequest {
    pub(crate) transaction_id: String,
    pub(crate) boot_epoch: String,
    pub(crate) stable_identities: CrashRegenerationStableIdentities,
}

impl CrashRegenerationRequest {
    fn validate(&self) -> Result<(), String> {
        if self.transaction_id.trim().is_empty() || self.boot_epoch.trim().is_empty() {
            return Err("crash_regeneration_request_invalid".to_string());
        }
        self.stable_identities.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CrashRegenerationOperation {
    pub(crate) transaction_id: String,
    pub(crate) operation_id: String,
    pub(crate) boot_epoch: String,
    pub(crate) phase: CrashRegenerationPhase,
    pub(crate) stable_identities: CrashRegenerationStableIdentities,
    pub(crate) prior_evidence: CrashRegenerationEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CrashRegenerationPhaseReceipt {
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

pub(crate) trait CrashRegenerationEffects {
    /// Execute or replay one operation idempotently. A repeated `operation_id`
    /// must return the original receipt without repeating provider effects.
    fn execute(
        &mut self,
        operation: &CrashRegenerationOperation,
    ) -> Result<CrashRegenerationPhaseReceipt, String>;
}

pub(crate) fn run_crash_regeneration<R, E>(
    repository: &R,
    effects: &mut E,
    request: CrashRegenerationRequest,
) -> Result<CrashRegenerationTransaction, String>
where
    R: ServiceStateRepository,
    E: CrashRegenerationEffects,
{
    request.validate()?;
    let mut transaction = begin_or_resume(repository, &request)?;
    if transaction.state == CrashRegenerationState::Ready {
        return Ok(transaction);
    }

    loop {
        let Some(phase) = next_phase(&transaction) else {
            return finish_ready(repository, transaction);
        };
        let operation = CrashRegenerationOperation {
            transaction_id: transaction.transaction_id.clone(),
            operation_id: format!("{}:{}", transaction.transaction_id, phase.operation_name()),
            boot_epoch: transaction.boot_epoch.clone(),
            phase,
            stable_identities: transaction.stable_identities.clone(),
            prior_evidence: transaction.evidence.clone(),
        };
        let receipt = match effects.execute(&operation) {
            Ok(receipt) => receipt,
            Err(error) => {
                persist_interruption(repository, &transaction, phase, &error)?;
                return Err(format!(
                    "crash_regeneration_phase_failed:{}:{error}",
                    phase.operation_name()
                ));
            }
        };
        if let Err(error) = validate_receipt(&operation, &receipt) {
            persist_interruption(repository, &transaction, phase, &error)?;
            return Err(format!(
                "crash_regeneration_receipt_invalid:{}:{error}",
                phase.operation_name()
            ));
        }
        transaction = persist_phase_receipt(repository, &transaction, phase, receipt)?;
    }
}

fn begin_or_resume<R: ServiceStateRepository>(
    repository: &R,
    request: &CrashRegenerationRequest,
) -> Result<CrashRegenerationTransaction, String> {
    repository.mutate(|state| {
        if let Some(existing) = state
            .crash_regeneration_transactions
            .get_mut(&request.transaction_id)
        {
            if existing.boot_epoch != request.boot_epoch
                || existing.stable_identities != request.stable_identities
            {
                return Err("crash_regeneration_transaction_identity_mismatch".to_string());
            }
            existing.replay_count = existing.replay_count.saturating_add(1);
            existing.state = if existing.state == CrashRegenerationState::Ready {
                CrashRegenerationState::Ready
            } else {
                CrashRegenerationState::InProgress
            };
            existing.last_error = None;
            existing.revision = existing.revision.saturating_add(1);
            return Ok(existing.clone());
        }
        let transaction = CrashRegenerationTransaction {
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
        };
        state
            .crash_regeneration_transactions
            .insert(transaction.transaction_id.clone(), transaction.clone());
        Ok(transaction)
    })
}

fn next_phase(transaction: &CrashRegenerationTransaction) -> Option<CrashRegenerationPhase> {
    CrashRegenerationPhase::ORDER
        .iter()
        .copied()
        .find(|phase| !transaction.completed_phases.contains(phase))
}

fn validate_receipt(
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

fn persist_phase_receipt<R: ServiceStateRepository>(
    repository: &R,
    expected: &CrashRegenerationTransaction,
    phase: CrashRegenerationPhase,
    receipt: CrashRegenerationPhaseReceipt,
) -> Result<CrashRegenerationTransaction, String> {
    repository.mutate(|state| {
        let current = state
            .crash_regeneration_transactions
            .get_mut(&expected.transaction_id)
            .ok_or_else(|| "crash_regeneration_transaction_missing".to_string())?;
        if current.revision != expected.revision
            || current.boot_epoch != expected.boot_epoch
            || current.stable_identities != expected.stable_identities
            || next_phase(current) != Some(phase)
        {
            return Err("crash_regeneration_compare_and_swap_mismatch".to_string());
        }
        apply_receipt(&mut current.evidence, receipt);
        current.completed_phases.push(phase);
        current.current_phase = next_phase(current);
        current.state = if current.current_phase.is_some() {
            CrashRegenerationState::InProgress
        } else {
            CrashRegenerationState::Ready
        };
        current.last_error = None;
        current.revision = current.revision.saturating_add(1);
        Ok(current.clone())
    })
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
            evidence.display_name = Some(display_name);
        }
        CrashRegenerationPhaseReceipt::GuacamoleRecovery {
            web_tier_generation,
            ..
        } => evidence.guacamole_web_tier_generation = Some(web_tier_generation),
        CrashRegenerationPhaseReceipt::RouteProjection { route_id, .. } => {
            evidence.route_id = Some(route_id);
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

fn persist_interruption<R: ServiceStateRepository>(
    repository: &R,
    expected: &CrashRegenerationTransaction,
    phase: CrashRegenerationPhase,
    error: &str,
) -> Result<(), String> {
    repository.mutate(|state| {
        let current = state
            .crash_regeneration_transactions
            .get_mut(&expected.transaction_id)
            .ok_or_else(|| "crash_regeneration_transaction_missing".to_string())?;
        if current.revision != expected.revision || next_phase(current) != Some(phase) {
            return Err("crash_regeneration_compare_and_swap_mismatch".to_string());
        }
        current.state = CrashRegenerationState::Interrupted;
        current.current_phase = Some(phase);
        current.last_error = Some(error.to_string());
        current.revision = current.revision.saturating_add(1);
        Ok(())
    })
}

fn finish_ready<R: ServiceStateRepository>(
    repository: &R,
    transaction: CrashRegenerationTransaction,
) -> Result<CrashRegenerationTransaction, String> {
    if transaction.state == CrashRegenerationState::Ready {
        return Ok(transaction);
    }
    repository.mutate(|state| {
        let current = state
            .crash_regeneration_transactions
            .get_mut(&transaction.transaction_id)
            .ok_or_else(|| "crash_regeneration_transaction_missing".to_string())?;
        if current.revision != transaction.revision || next_phase(current).is_some() {
            return Err("crash_regeneration_compare_and_swap_mismatch".to_string());
        }
        current.state = CrashRegenerationState::Ready;
        current.current_phase = None;
        current.last_error = None;
        current.revision = current.revision.saturating_add(1);
        Ok(current.clone())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::ServiceState;
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::{atomic::AtomicUsize, atomic::Ordering, Mutex};

    #[derive(Default)]
    struct MemoryRepository(Mutex<ServiceState>);

    impl ServiceStateRepository for MemoryRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn mutate<T>(
            &self,
            mutator: impl FnOnce(&mut ServiceState) -> Result<T, String>,
        ) -> Result<T, String> {
            mutator(&mut self.0.lock().unwrap())
        }
    }

    struct FailOneMutationRepository {
        state: Mutex<ServiceState>,
        mutation_count: AtomicUsize,
        fail_at: usize,
    }

    impl FailOneMutationRepository {
        fn new(fail_at: usize) -> Self {
            Self {
                state: Mutex::new(ServiceState::default()),
                mutation_count: AtomicUsize::new(0),
                fail_at,
            }
        }
    }

    impl ServiceStateRepository for FailOneMutationRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            Ok(self.state.lock().unwrap().clone())
        }

        fn mutate<T>(
            &self,
            mutator: impl FnOnce(&mut ServiceState) -> Result<T, String>,
        ) -> Result<T, String> {
            let mutation = self.mutation_count.fetch_add(1, Ordering::SeqCst) + 1;
            if mutation == self.fail_at {
                return Err("synthetic_receipt_persistence_failure".to_string());
            }
            mutator(&mut self.state.lock().unwrap())
        }
    }

    struct ScriptedEffects {
        fail_once_at: Option<CrashRegenerationPhase>,
        failed: bool,
        logical_effects: BTreeMap<String, CrashRegenerationPhaseReceipt>,
        invocation_order: Vec<CrashRegenerationPhase>,
        operator_visible_ready: bool,
    }

    impl ScriptedEffects {
        fn new(fail_once_at: Option<CrashRegenerationPhase>) -> Self {
            Self {
                fail_once_at,
                failed: false,
                logical_effects: BTreeMap::new(),
                invocation_order: Vec::new(),
                operator_visible_ready: true,
            }
        }

        fn receipt(operation: &CrashRegenerationOperation) -> CrashRegenerationPhaseReceipt {
            let stable = &operation.stable_identities;
            match operation.phase {
                CrashRegenerationPhase::RuntimeHostAuthority => {
                    CrashRegenerationPhaseReceipt::RuntimeHostAuthority {
                        boot_epoch: operation.boot_epoch.clone(),
                        runtime_host_id: "host-current".to_string(),
                        pid: 42001,
                        socket_identity: "socket-current".to_string(),
                    }
                }
                CrashRegenerationPhase::BrowserAuthority => {
                    CrashRegenerationPhaseReceipt::BrowserAuthority {
                        boot_epoch: operation.boot_epoch.clone(),
                        logical_browser_id: stable.logical_browser_id.clone(),
                        pid: 42002,
                        owner_generation: 8,
                    }
                }
                CrashRegenerationPhase::DisplayDiscovery => {
                    CrashRegenerationPhaseReceipt::DisplayDiscovery {
                        boot_epoch: operation.boot_epoch.clone(),
                        display_name: ":101".to_string(),
                    }
                }
                CrashRegenerationPhase::GuacamoleRecovery => {
                    CrashRegenerationPhaseReceipt::GuacamoleRecovery {
                        boot_epoch: operation.boot_epoch.clone(),
                        web_tier_generation: "guac-web-current".to_string(),
                    }
                }
                CrashRegenerationPhase::RouteProjection => {
                    CrashRegenerationPhaseReceipt::RouteProjection {
                        route_id: stable.route_id.clone(),
                        connection_id: stable.connection_id.clone(),
                        route_user_id: stable.route_user_id.clone(),
                    }
                }
                CrashRegenerationPhase::DurableHandoffResolution => {
                    CrashRegenerationPhaseReceipt::DurableHandoffResolution {
                        handoff_id: stable.handoff_id.clone(),
                    }
                }
                CrashRegenerationPhase::OperatorVisibleProof => {
                    CrashRegenerationPhaseReceipt::OperatorVisibleProof {
                        boot_epoch: operation.boot_epoch.clone(),
                        viewer_session_id: "viewer-current".to_string(),
                        ready: true,
                    }
                }
            }
        }
    }

    impl CrashRegenerationEffects for ScriptedEffects {
        fn execute(
            &mut self,
            operation: &CrashRegenerationOperation,
        ) -> Result<CrashRegenerationPhaseReceipt, String> {
            self.invocation_order.push(operation.phase);
            if self.fail_once_at == Some(operation.phase) && !self.failed {
                self.failed = true;
                return Err("synthetic_interruption".to_string());
            }
            if let Some(receipt) = self.logical_effects.get(&operation.operation_id) {
                return Ok(receipt.clone());
            }
            let mut receipt = Self::receipt(operation);
            if let CrashRegenerationPhaseReceipt::OperatorVisibleProof { ready, .. } = &mut receipt
            {
                *ready = self.operator_visible_ready;
            }
            self.logical_effects
                .insert(operation.operation_id.clone(), receipt.clone());
            Ok(receipt)
        }
    }

    fn request() -> CrashRegenerationRequest {
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/fixtures/profile-lifecycle/plan-0134-red-fixtures.v1.json"
        ))
        .unwrap();
        let crash = &corpus["crashEpoch"];
        let stable = &crash["stableIdentities"];
        CrashRegenerationRequest {
            transaction_id: "crash-tx-1".to_string(),
            boot_epoch: crash["currentBootEpoch"].as_str().unwrap().to_string(),
            stable_identities: CrashRegenerationStableIdentities {
                principal_id: stable["principalId"].as_str().unwrap().to_string(),
                profile_id: stable["profileId"].as_str().unwrap().to_string(),
                logical_browser_id: stable["logicalBrowserId"].as_str().unwrap().to_string(),
                session_route: stable["sessionRoute"].as_str().unwrap().to_string(),
                route_id: format!("route:{}", stable["connectionId"].as_str().unwrap()),
                connection_id: stable["connectionId"].as_str().unwrap().to_string(),
                route_user_id: stable["routeUserId"].as_str().unwrap().to_string(),
                handoff_id: stable["handoffId"].as_str().unwrap().to_string(),
            },
        }
    }

    #[test]
    fn interrupted_replay_resumes_in_dependency_order_without_duplicate_effects() {
        let repository = MemoryRepository::default();
        let mut effects = ScriptedEffects::new(Some(CrashRegenerationPhase::GuacamoleRecovery));
        let error = run_crash_regeneration(&repository, &mut effects, request()).unwrap_err();
        assert!(error.contains("guacamole_recovery:synthetic_interruption"));

        let interrupted = repository.load_snapshot().unwrap();
        let transaction = &interrupted.crash_regeneration_transactions["crash-tx-1"];
        assert_eq!(transaction.state, CrashRegenerationState::Interrupted);
        assert_eq!(
            transaction.completed_phases,
            vec![
                CrashRegenerationPhase::RuntimeHostAuthority,
                CrashRegenerationPhase::BrowserAuthority,
                CrashRegenerationPhase::DisplayDiscovery,
            ]
        );

        let ready = run_crash_regeneration(&repository, &mut effects, request()).unwrap();
        assert_eq!(ready.state, CrashRegenerationState::Ready);
        assert_eq!(ready.completed_phases, CrashRegenerationPhase::ORDER);
        assert_eq!(ready.evidence.display_name.as_deref(), Some(":101"));
        assert_eq!(
            ready.evidence.viewer_session_id.as_deref(),
            Some("viewer-current")
        );
        assert!(ready.evidence.operator_visible_ready);
        assert_eq!(effects.logical_effects.len(), 7);
        assert_eq!(
            effects
                .logical_effects
                .keys()
                .collect::<BTreeSet<_>>()
                .len(),
            7
        );

        let replayed = run_crash_regeneration(&repository, &mut effects, request()).unwrap();
        assert_eq!(replayed.state, CrashRegenerationState::Ready);
        assert_eq!(effects.logical_effects.len(), 7);
        assert_eq!(
            effects.invocation_order,
            vec![
                CrashRegenerationPhase::RuntimeHostAuthority,
                CrashRegenerationPhase::BrowserAuthority,
                CrashRegenerationPhase::DisplayDiscovery,
                CrashRegenerationPhase::GuacamoleRecovery,
                CrashRegenerationPhase::GuacamoleRecovery,
                CrashRegenerationPhase::RouteProjection,
                CrashRegenerationPhase::DurableHandoffResolution,
                CrashRegenerationPhase::OperatorVisibleProof,
            ]
        );
    }

    #[test]
    fn stable_identity_or_boot_change_cannot_reuse_transaction_id() {
        let repository = MemoryRepository::default();
        let mut effects = ScriptedEffects::new(None);
        run_crash_regeneration(&repository, &mut effects, request()).unwrap();

        let mut changed = request();
        changed.boot_epoch = "boot:another".to_string();
        assert_eq!(
            run_crash_regeneration(&repository, &mut effects, changed).unwrap_err(),
            "crash_regeneration_transaction_identity_mismatch"
        );
        let mut changed = request();
        changed.stable_identities.profile_id = "other-profile".to_string();
        assert_eq!(
            run_crash_regeneration(&repository, &mut effects, changed).unwrap_err(),
            "crash_regeneration_transaction_identity_mismatch"
        );
    }

    #[test]
    fn replay_after_receipt_persistence_failure_reuses_the_same_operation_id() {
        // Mutation 1 creates the transaction. The runtime-host effect then
        // succeeds, while mutation 2 fails before its receipt is durable.
        let repository = FailOneMutationRepository::new(2);
        let mut effects = ScriptedEffects::new(None);
        assert_eq!(
            run_crash_regeneration(&repository, &mut effects, request()).unwrap_err(),
            "synthetic_receipt_persistence_failure"
        );
        assert_eq!(effects.logical_effects.len(), 1);

        let ready = run_crash_regeneration(&repository, &mut effects, request()).unwrap();
        assert_eq!(ready.state, CrashRegenerationState::Ready);
        assert_eq!(effects.logical_effects.len(), 7);
        assert_eq!(
            effects.invocation_order[..2],
            [
                CrashRegenerationPhase::RuntimeHostAuthority,
                CrashRegenerationPhase::RuntimeHostAuthority,
            ]
        );
    }

    #[test]
    fn operator_visible_false_is_persisted_as_an_interruption() {
        let repository = MemoryRepository::default();
        let mut effects = ScriptedEffects::new(None);
        effects.operator_visible_ready = false;

        let error = run_crash_regeneration(&repository, &mut effects, request()).unwrap_err();
        assert!(
            error.contains("operator_visible_proof:crash_regeneration_operator_visible_not_ready")
        );
        let snapshot = repository.load_snapshot().unwrap();
        let transaction = &snapshot.crash_regeneration_transactions["crash-tx-1"];
        assert_eq!(transaction.state, CrashRegenerationState::Interrupted);
        assert_eq!(
            transaction.current_phase,
            Some(CrashRegenerationPhase::OperatorVisibleProof)
        );
        assert!(!transaction.evidence.operator_visible_ready);
        assert_eq!(
            transaction.last_error.as_deref(),
            Some("crash_regeneration_operator_visible_not_ready")
        );
    }

    #[test]
    fn transaction_state_survives_service_state_serialization() {
        let repository = MemoryRepository::default();
        let mut effects = ScriptedEffects::new(None);
        let ready = run_crash_regeneration(&repository, &mut effects, request()).unwrap();
        let encoded = serde_json::to_vec(&repository.load_snapshot().unwrap()).unwrap();
        let decoded: ServiceState = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded.crash_regeneration_transactions["crash-tx-1"], ready);
    }

    #[test]
    fn public_status_contains_only_stable_identity_and_progress() {
        let repository = MemoryRepository::default();
        let mut effects = ScriptedEffects::new(None);
        run_crash_regeneration(&repository, &mut effects, request()).unwrap();
        let snapshot = repository.load_snapshot().unwrap();
        let public = serde_json::to_value(crash_regeneration_statuses(
            &snapshot.crash_regeneration_transactions,
        ))
        .unwrap();
        let encoded = public.to_string();

        assert_eq!(public[0]["state"], "ready");
        assert_eq!(
            public[0]["profileId"],
            snapshot.crash_regeneration_transactions["crash-tx-1"]
                .stable_identities
                .profile_id
        );
        assert_eq!(public[0]["recourse"], "reuse_durable_handoff");
        for private_key in [
            "bootEpoch",
            "runtimeHostId",
            "runtimeHostPid",
            "socketIdentity",
            "browserPid",
            "displayName",
            "guacamoleWebTierGeneration",
            "viewerSessionId",
        ] {
            assert!(!encoded.contains(private_key), "leaked {private_key}");
        }
    }
}
