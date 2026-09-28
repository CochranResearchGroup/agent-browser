//! Provider-free contract fixtures for the simple cold-shutdown boundary.
//!
//! This file is registered by the workstation command owner.  Keeping the
//! fixtures in a separate module makes the absence of transaction and drain
//! inputs visible at the call seam.

use crate::workstation_shutdown::{
    execute_workstation_shutdown, ShutdownPhase, ShutdownPhaseResult, ShutdownResidue,
    ShutdownStepReceipt, WorkstationShutdownEffects, SHUTDOWN_SCHEMA_VERSION,
};
use std::time::Duration;

#[derive(Clone, Debug, Default)]
struct ScriptedShutdown {
    calls: Vec<(ShutdownPhase, Duration)>,
    failures: Vec<ShutdownPhase>,
    residue: ShutdownResidue,
}

impl ScriptedShutdown {
    fn new(failures: impl IntoIterator<Item = ShutdownPhase>) -> Self {
        Self {
            failures: failures.into_iter().collect(),
            ..Self::default()
        }
    }

    fn result(&self, phase: ShutdownPhase, deadline: Duration) -> ShutdownPhaseResult {
        let failed = self.failures.contains(&phase);
        ShutdownPhaseResult {
            step: ShutdownStepReceipt {
                phase,
                changed: !failed,
                escalated: false,
                deadline_ms: deadline.as_millis() as u64,
                warnings: Vec::new(),
                error: failed.then(|| format!("{phase:?}_failed")),
            },
            residue: (phase == ShutdownPhase::Verify).then(|| self.residue.clone()),
        }
    }
}

impl WorkstationShutdownEffects for ScriptedShutdown {
    fn execute_phase(&mut self, phase: ShutdownPhase, deadline: Duration) -> ShutdownPhaseResult {
        self.calls.push((phase, deadline));
        self.result(phase, deadline)
    }
}

#[test]
fn stale_transaction_and_drain_metadata_cannot_enter_or_veto_shutdown() {
    // These are deliberately unconsumed diagnostics, proving the controller
    // has no coordination-state argument and only observes owned residue.
    let stale_transaction_revision = Some("old-revision");
    let active_drain = true;
    let mut effects = ScriptedShutdown::default();

    let receipt = execute_workstation_shutdown(&mut effects);

    assert!(stale_transaction_revision.is_some());
    assert!(active_drain);
    assert!(receipt.success);
    assert_eq!(receipt.schema_version, SHUTDOWN_SCHEMA_VERSION);
    assert_eq!(effects.calls.len(), 6);
    assert_eq!(
        effects.calls[0],
        (ShutdownPhase::Browsers, Duration::from_secs(5))
    );
    assert_eq!(
        effects.calls[1],
        (ShutdownPhase::UserUnits, Duration::from_secs(5))
    );
    assert_eq!(
        effects.calls[2],
        (ShutdownPhase::Containers, Duration::from_secs(10))
    );
    assert_eq!(
        effects.calls[3],
        (ShutdownPhase::Ownership, Duration::from_secs(2))
    );
    assert_eq!(
        effects.calls[4],
        (ShutdownPhase::TransientMetadata, Duration::from_secs(2))
    );
    assert_eq!(
        effects.calls[5],
        (ShutdownPhase::Verify, Duration::from_secs(2))
    );
}

#[test]
fn clean_machine_shutdown_is_successful_and_idempotent() {
    let mut first = ScriptedShutdown::default();
    let first_receipt = execute_workstation_shutdown(&mut first);
    let mut second = ScriptedShutdown::default();
    let second_receipt = execute_workstation_shutdown(&mut second);

    assert!(first_receipt.success);
    assert!(second_receipt.success);
    assert_eq!(first_receipt.residue, ShutdownResidue::default());
    assert_eq!(second_receipt.residue, ShutdownResidue::default());
    assert_eq!(first.calls, second.calls);
}

#[test]
fn partial_replay_continues_all_fixed_phases_and_reports_failure() {
    let mut effects = ScriptedShutdown::new([ShutdownPhase::Containers]);

    let receipt = execute_workstation_shutdown(&mut effects);

    assert!(!receipt.success);
    assert_eq!(receipt.steps.len(), 6);
    assert_eq!(
        effects.calls.last().map(|(phase, _)| *phase),
        Some(ShutdownPhase::Verify)
    );
    assert!(effects
        .calls
        .iter()
        .any(|(phase, _)| *phase == ShutdownPhase::Ownership));
    assert_eq!(receipt.steps[2].error.as_deref(), Some("Containers_failed"));
}

#[test]
fn foreign_process_observation_does_not_veto_owned_shutdown() {
    let mut effects = ScriptedShutdown {
        residue: ShutdownResidue {
            foreign_processes: 3,
            ..Default::default()
        },
        ..Default::default()
    };

    let receipt = execute_workstation_shutdown(&mut effects);

    assert!(receipt.success);
    assert_eq!(receipt.residue.foreign_processes, 3);
}
