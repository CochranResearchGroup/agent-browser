//! Idempotent, dependency-ordered crash regeneration for one managed lane.
//!
//! Effects are delegated through a phase adapter and keyed by a stable
//! transaction/phase operation id. The coordinator persists a compare-and-swap
//! receipt after every phase so interruption resumes at the first incomplete
//! dependency without changing stable identities or duplicating resources.

use super::service_store::ServiceStateRepository;

#[cfg(test)]
pub(crate) use agent_browser_service_model::{
    crash_regeneration_statuses, CrashRegenerationStableIdentities,
};
pub(crate) use agent_browser_service_model::{
    next_phase, validate_receipt, CrashRegenerationOperation, CrashRegenerationPhase,
    CrashRegenerationPhaseReceipt, CrashRegenerationRequest, CrashRegenerationState,
    CrashRegenerationTransaction,
};

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
    repository.mutate(|state| state.begin_or_resume_crash_regeneration(request))
}

fn persist_phase_receipt<R: ServiceStateRepository>(
    repository: &R,
    expected: &CrashRegenerationTransaction,
    phase: CrashRegenerationPhase,
    receipt: CrashRegenerationPhaseReceipt,
) -> Result<CrashRegenerationTransaction, String> {
    repository
        .mutate(|state| state.apply_crash_regeneration_phase(expected, phase, receipt.clone()))
}

fn persist_interruption<R: ServiceStateRepository>(
    repository: &R,
    expected: &CrashRegenerationTransaction,
    phase: CrashRegenerationPhase,
    error: &str,
) -> Result<(), String> {
    repository.mutate(|state| {
        state.interrupt_crash_regeneration(expected, phase, error)?;
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
    repository.mutate(|state| state.finish_crash_regeneration(&transaction))
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
            mut mutator: impl FnMut(&mut ServiceState) -> Result<T, String>,
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

        fn mutation_count(&self) -> usize {
            self.mutation_count.load(Ordering::SeqCst)
        }
    }

    impl ServiceStateRepository for FailOneMutationRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            Ok(self.state.lock().unwrap().clone())
        }

        fn mutate<T>(
            &self,
            mut mutator: impl FnMut(&mut ServiceState) -> Result<T, String>,
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
        // Two first-attempt mutations, then one resume plus seven durable
        // phase receipts. Reaching Ready must not add an eleventh mutation.
        assert_eq!(repository.mutation_count(), 10);
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
