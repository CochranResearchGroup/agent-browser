//! Bounded cold shutdown orchestration for the installed workstation.
//!
//! This module deliberately does not accept upgrade transactions, admission
//! drains, census digests, rollback state, or runtime-generation decisions.
//! Those values may be useful diagnostics, but they cannot veto an explicit
//! operator request to stop Agent Browser-owned machinery.

use serde::Serialize;
use std::time::Duration;

use crate::native::service_store::ServiceStateRepository;

pub(crate) const SHUTDOWN_SCHEMA_VERSION: &str = "agent-browser.workstation-shutdown.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ShutdownPhase {
    Browsers,
    UserUnits,
    Containers,
    Ownership,
    TransientMetadata,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShutdownStepReceipt {
    pub(crate) phase: ShutdownPhase,
    pub(crate) deadline_ms: u64,
    pub(crate) changed: bool,
    pub(crate) escalated: bool,
    pub(crate) warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}

impl ShutdownStepReceipt {
    pub(crate) fn unchanged(phase: ShutdownPhase) -> Self {
        Self {
            phase,
            deadline_ms: 0,
            changed: false,
            escalated: false,
            warnings: Vec::new(),
            error: None,
        }
    }

    #[cfg(test)]
    fn changed(phase: ShutdownPhase) -> Self {
        Self {
            phase,
            changed: true,
            ..Self::unchanged(phase)
        }
    }

    #[cfg(test)]
    fn failed(phase: ShutdownPhase, error: &str) -> Self {
        Self {
            phase,
            error: Some(error.to_string()),
            ..Self::unchanged(phase)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShutdownPhaseResult {
    pub(crate) step: ShutdownStepReceipt,
    pub(crate) residue: Option<ShutdownResidue>,
}

impl ShutdownPhaseResult {
    pub(crate) fn step(step: ShutdownStepReceipt) -> Self {
        Self {
            step,
            residue: None,
        }
    }

    pub(crate) fn verification(step: ShutdownStepReceipt, residue: ShutdownResidue) -> Self {
        Self {
            step,
            residue: Some(residue),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShutdownResidue {
    pub(crate) owned_browsers: usize,
    pub(crate) owned_user_units: usize,
    pub(crate) owned_containers: usize,
    pub(crate) runtime_owners: usize,
    pub(crate) active_leases: usize,
    /// Foreign processes are observable, but never targets of this command.
    pub(crate) foreign_processes: usize,
}

impl ShutdownResidue {
    fn owned_residue_count(&self) -> usize {
        self.owned_browsers
            + self.owned_user_units
            + self.owned_containers
            + self.runtime_owners
            + self.active_leases
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkstationShutdownReceipt {
    pub(crate) schema_version: &'static str,
    pub(crate) success: bool,
    pub(crate) changed: bool,
    pub(crate) steps: Vec<ShutdownStepReceipt>,
    pub(crate) residue: ShutdownResidue,
}

/// Effects needed by cold shutdown.
///
/// The interface is intentionally small. Implementations own target discovery,
/// bounded polite-close and escalation, and platform-specific commands. Every
/// method returns a receipt instead of withholding later cleanup phases.
pub(crate) trait WorkstationShutdownEffects {
    fn execute_phase(&mut self, phase: ShutdownPhase, deadline: Duration) -> ShutdownPhaseResult;
}

fn shutdown_phase_deadline(phase: ShutdownPhase) -> Duration {
    Duration::from_millis(match phase {
        ShutdownPhase::Browsers | ShutdownPhase::UserUnits => 5_000,
        ShutdownPhase::Containers => 10_000,
        ShutdownPhase::Ownership | ShutdownPhase::TransientMetadata | ShutdownPhase::Verify => {
            2_000
        }
    })
}

/// Execute every cold-shutdown phase exactly once and always perform final
/// verification. A failed phase cannot prevent later cleanup from running.
pub(crate) fn execute_workstation_shutdown(
    effects: &mut impl WorkstationShutdownEffects,
) -> WorkstationShutdownReceipt {
    let phases = [
        ShutdownPhase::Browsers,
        ShutdownPhase::UserUnits,
        ShutdownPhase::Containers,
        ShutdownPhase::Ownership,
        ShutdownPhase::TransientMetadata,
        ShutdownPhase::Verify,
    ];
    let mut steps = Vec::with_capacity(phases.len());
    let mut residue = None;
    for phase in phases {
        let deadline = shutdown_phase_deadline(phase);
        let mut result = effects.execute_phase(phase, deadline);
        result.step.phase = phase;
        result.step.deadline_ms = deadline.as_millis() as u64;
        if phase == ShutdownPhase::Verify {
            residue = result.residue;
            if residue.is_none() && result.step.error.is_none() {
                result.step.error = Some("shutdown_verification_residue_missing".to_string());
            }
        }
        steps.push(result.step);
    }
    let residue = residue.unwrap_or_default();

    let success =
        steps.iter().all(|step| step.error.is_none()) && residue.owned_residue_count() == 0;
    let changed = steps.iter().any(|step| step.changed);
    WorkstationShutdownReceipt {
        schema_version: SHUTDOWN_SCHEMA_VERSION,
        success,
        changed,
        steps,
        residue,
    }
}

/// Commit the state-only shutdown boundary under the canonical repository
/// lock. Process, unit, and container adapters must finish before calling this
/// helper so durable authority never claims an effect is gone prematurely.
pub(crate) fn release_service_authority_for_cold_shutdown(
    repository: &impl ServiceStateRepository,
    observed_at: &str,
) -> Result<agent_browser_service_model::ColdShutdownStateReceipt, String> {
    repository.mutate(|state| state.release_local_authority_for_cold_shutdown(observed_at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{BrowserProfile, BrowserSession, LeaseState};
    use crate::native::service_store::{
        JsonServiceStateStore, LockedServiceStateRepository, ServiceStateStore,
    };
    use std::fs;
    use std::path::PathBuf;

    #[derive(Default)]
    struct FakeEffects {
        calls: Vec<ShutdownPhase>,
        fail_browser_close: bool,
        residue: ShutdownResidue,
    }

    impl FakeEffects {
        fn step(&mut self, phase: ShutdownPhase) -> ShutdownStepReceipt {
            self.calls.push(phase);
            ShutdownStepReceipt::changed(phase)
        }
    }

    impl WorkstationShutdownEffects for FakeEffects {
        fn execute_phase(
            &mut self,
            phase: ShutdownPhase,
            _deadline: Duration,
        ) -> ShutdownPhaseResult {
            if phase == ShutdownPhase::Verify {
                self.calls.push(phase);
                return ShutdownPhaseResult::verification(
                    ShutdownStepReceipt::unchanged(phase),
                    self.residue.clone(),
                );
            }
            if phase == ShutdownPhase::Browsers && self.fail_browser_close {
                self.calls.push(phase);
                return ShutdownPhaseResult::step(ShutdownStepReceipt::failed(
                    phase,
                    "browser_close_failed",
                ));
            }
            ShutdownPhaseResult::step(self.step(phase))
        }
    }

    #[test]
    fn shutdown_runs_the_small_fixed_sequence_without_coordination_inputs() {
        let mut effects = FakeEffects::default();

        let receipt = execute_workstation_shutdown(&mut effects);

        assert!(receipt.success);
        assert_eq!(receipt.schema_version, SHUTDOWN_SCHEMA_VERSION);
        assert_eq!(
            effects.calls,
            vec![
                ShutdownPhase::Browsers,
                ShutdownPhase::UserUnits,
                ShutdownPhase::Containers,
                ShutdownPhase::Ownership,
                ShutdownPhase::TransientMetadata,
                ShutdownPhase::Verify,
            ]
        );
    }

    #[test]
    fn shutdown_records_the_fixed_deadline_for_every_phase() {
        let mut effects = FakeEffects::default();

        let receipt = execute_workstation_shutdown(&mut effects);

        assert_eq!(
            receipt
                .steps
                .iter()
                .map(|step| step.deadline_ms)
                .collect::<Vec<_>>(),
            vec![5_000, 5_000, 10_000, 2_000, 2_000, 2_000]
        );
    }

    #[test]
    fn a_failed_phase_never_vetoes_later_cleanup_or_verification() {
        let mut effects = FakeEffects {
            fail_browser_close: true,
            ..FakeEffects::default()
        };

        let receipt = execute_workstation_shutdown(&mut effects);

        assert!(!receipt.success);
        assert_eq!(effects.calls.last(), Some(&ShutdownPhase::Verify));
        assert!(effects.calls.contains(&ShutdownPhase::Ownership));
        assert_eq!(receipt.steps.len(), 6);
    }

    #[test]
    fn foreign_processes_are_reported_but_do_not_block_success() {
        let mut effects = FakeEffects {
            residue: ShutdownResidue {
                foreign_processes: 4,
                ..ShutdownResidue::default()
            },
            ..FakeEffects::default()
        };

        let receipt = execute_workstation_shutdown(&mut effects);

        assert!(receipt.success);
        assert_eq!(receipt.residue.foreign_processes, 4);
    }

    #[test]
    fn owned_residue_fails_the_postcondition_without_skipping_receipts() {
        let mut effects = FakeEffects {
            residue: ShutdownResidue {
                owned_browsers: 1,
                active_leases: 2,
                ..ShutdownResidue::default()
            },
            ..FakeEffects::default()
        };

        let receipt = execute_workstation_shutdown(&mut effects);

        assert!(!receipt.success);
        assert_eq!(receipt.residue.owned_browsers, 1);
        assert_eq!(receipt.residue.active_leases, 2);
        assert_eq!(receipt.steps.len(), 6);
    }

    #[test]
    fn repository_shutdown_release_preserves_profile_data_and_replays_cleanly() {
        let directory = std::env::temp_dir().join(format!(
            "agent-browser-cold-shutdown-state-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("state.json");
        let store = JsonServiceStateStore::new(&path);
        let mut state = agent_browser_service_model::ServiceState::default();
        state.profiles.insert(
            "named".to_string(),
            BrowserProfile {
                id: "named".to_string(),
                user_data_dir: Some("/profiles/named".to_string()),
                ..BrowserProfile::default()
            },
        );
        state.sessions.insert(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                ..BrowserSession::default()
            },
        );
        store.save(&state).unwrap();
        let repository = LockedServiceStateRepository::new(store);

        let first =
            release_service_authority_for_cold_shutdown(&repository, "2026-09-17T12:00:00Z")
                .unwrap();
        assert_eq!(first.released_sessions, 1);
        let persisted = repository.load_snapshot().unwrap();
        assert_eq!(persisted.sessions["session-1"].lease, LeaseState::Released);
        assert_eq!(
            persisted.profiles["named"].user_data_dir.as_deref(),
            Some("/profiles/named")
        );

        let replay =
            release_service_authority_for_cold_shutdown(&repository, "2026-09-17T12:00:01Z")
                .unwrap();
        assert_eq!(
            replay,
            agent_browser_service_model::ColdShutdownStateReceipt::default()
        );
        fs::remove_dir_all(PathBuf::from(directory)).unwrap();
    }
}
