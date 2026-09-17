//! Provider-free fixtures for the bounded stop, replace, start, readiness flow.

use crate::workstation_cold_install::{
    execute_workstation_cold_install, ColdInstallEffects, ColdInstallPhase, ColdInstallStepReceipt,
};
use std::time::Duration;

#[derive(Debug, Default)]
struct ScriptedInstall {
    calls: Vec<(ColdInstallPhase, Duration)>,
    failures: Vec<ColdInstallPhase>,
}

impl ScriptedInstall {
    fn new(failures: impl IntoIterator<Item = ColdInstallPhase>) -> Self {
        Self {
            failures: failures.into_iter().collect(),
            ..Self::default()
        }
    }
}

impl ColdInstallEffects for ScriptedInstall {
    fn execute_phase(
        &mut self,
        phase: ColdInstallPhase,
        deadline: Duration,
    ) -> ColdInstallStepReceipt {
        self.calls.push((phase, deadline));
        let failed = self.failures.contains(&phase);
        ColdInstallStepReceipt {
            phase,
            changed: !failed,
            ready: phase == ColdInstallPhase::Readiness && !failed,
            deadline_ms: deadline.as_millis() as u64,
            error: failed.then(|| format!("{phase:?}_failed")),
        }
    }
}

#[test]
fn clean_cold_install_runs_stop_replace_start_and_readiness_in_order() {
    let mut effects = ScriptedInstall::default();

    let receipt = execute_workstation_cold_install(&mut effects);

    assert!(receipt.success);
    assert_eq!(
        effects
            .calls
            .iter()
            .map(|(phase, _)| *phase)
            .collect::<Vec<_>>(),
        vec![
            ColdInstallPhase::Stop,
            ColdInstallPhase::Replace,
            ColdInstallPhase::Start,
            ColdInstallPhase::Readiness
        ]
    );
    assert_eq!(effects.calls[0].1, Duration::from_secs(30));
    assert_eq!(effects.calls[1].1, Duration::from_secs(30));
    assert_eq!(effects.calls[2].1, Duration::from_secs(30));
    assert_eq!(effects.calls[3].1, Duration::from_secs(30));
}

#[test]
fn stop_failure_prevents_replacement_and_does_not_rollback() {
    let mut effects = ScriptedInstall::new([ColdInstallPhase::Stop]);

    let receipt = execute_workstation_cold_install(&mut effects);

    assert!(!receipt.success);
    assert_eq!(
        effects
            .calls
            .iter()
            .map(|(phase, _)| *phase)
            .collect::<Vec<_>>(),
        vec![ColdInstallPhase::Stop]
    );
    assert!(receipt.rollback.is_none());
}

#[test]
fn post_stop_failure_attempts_one_rollback_and_preserves_original_error() {
    let mut effects = ScriptedInstall::new([ColdInstallPhase::Replace, ColdInstallPhase::Rollback]);

    let receipt = execute_workstation_cold_install(&mut effects);

    assert!(!receipt.success);
    assert_eq!(
        effects
            .calls
            .iter()
            .map(|(phase, _)| *phase)
            .collect::<Vec<_>>(),
        vec![
            ColdInstallPhase::Stop,
            ColdInstallPhase::Replace,
            ColdInstallPhase::Rollback
        ]
    );
    assert_eq!(receipt.original_error.as_deref(), Some("Replace_failed"));
    assert_eq!(
        receipt.rollback.as_ref().map(|step| step.phase),
        Some(ColdInstallPhase::Rollback)
    );
    assert_eq!(
        receipt
            .rollback
            .as_ref()
            .and_then(|step| step.error.as_deref()),
        Some("Rollback_failed")
    );
}

#[test]
fn readiness_failure_rolls_back_exactly_once() {
    let mut effects = ScriptedInstall::new([ColdInstallPhase::Readiness]);

    let receipt = execute_workstation_cold_install(&mut effects);

    assert!(!receipt.success);
    assert_eq!(
        effects
            .calls
            .iter()
            .filter(|(phase, _)| *phase == ColdInstallPhase::Rollback)
            .count(),
        1
    );
    assert_eq!(receipt.original_error.as_deref(), Some("Readiness_failed"));
}
