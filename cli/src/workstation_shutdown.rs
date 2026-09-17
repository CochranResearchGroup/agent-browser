//! Bounded cold shutdown orchestration for the installed workstation.
//!
//! This module deliberately does not accept upgrade transactions, admission
//! drains, census digests, rollback state, or runtime-generation decisions.
//! Those values may be useful diagnostics, but they cannot veto an explicit
//! operator request to stop Agent Browser-owned machinery.

use serde::Serialize;

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
    fn stop_owned_browsers(&mut self) -> ShutdownStepReceipt;
    fn stop_owned_user_units(&mut self) -> ShutdownStepReceipt;
    fn stop_owned_containers(&mut self) -> ShutdownStepReceipt;
    fn release_runtime_ownership(&mut self) -> ShutdownStepReceipt;
    fn clear_transient_metadata(&mut self) -> ShutdownStepReceipt;
    fn verify_shutdown(&mut self) -> (ShutdownStepReceipt, ShutdownResidue);
}

/// Execute every cold-shutdown phase exactly once and always perform final
/// verification. A failed phase cannot prevent later cleanup from running.
pub(crate) fn execute_workstation_shutdown(
    effects: &mut impl WorkstationShutdownEffects,
) -> WorkstationShutdownReceipt {
    let mut steps = vec![
        effects.stop_owned_browsers(),
        effects.stop_owned_user_units(),
        effects.stop_owned_containers(),
        effects.release_runtime_ownership(),
        effects.clear_transient_metadata(),
    ];
    let (verification, residue) = effects.verify_shutdown();
    steps.push(verification);

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

#[cfg(test)]
mod tests {
    use super::*;

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
        fn stop_owned_browsers(&mut self) -> ShutdownStepReceipt {
            self.calls.push(ShutdownPhase::Browsers);
            if self.fail_browser_close {
                ShutdownStepReceipt::failed(ShutdownPhase::Browsers, "browser_close_failed")
            } else {
                ShutdownStepReceipt::changed(ShutdownPhase::Browsers)
            }
        }

        fn stop_owned_user_units(&mut self) -> ShutdownStepReceipt {
            self.step(ShutdownPhase::UserUnits)
        }

        fn stop_owned_containers(&mut self) -> ShutdownStepReceipt {
            self.step(ShutdownPhase::Containers)
        }

        fn release_runtime_ownership(&mut self) -> ShutdownStepReceipt {
            self.step(ShutdownPhase::Ownership)
        }

        fn clear_transient_metadata(&mut self) -> ShutdownStepReceipt {
            self.step(ShutdownPhase::TransientMetadata)
        }

        fn verify_shutdown(&mut self) -> (ShutdownStepReceipt, ShutdownResidue) {
            self.calls.push(ShutdownPhase::Verify);
            (
                ShutdownStepReceipt::unchanged(ShutdownPhase::Verify),
                self.residue.clone(),
            )
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
}
