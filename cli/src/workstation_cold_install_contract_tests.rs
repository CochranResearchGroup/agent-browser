//! Provider-free fixtures for the bounded stop, replace, start, readiness flow.

use crate::workstation_cold_install::{
    execute_workstation_cold_install, execute_workstation_cold_install_with_clock,
    ColdInstallClock, ColdInstallEffects, ColdInstallPhase, ColdInstallStepReceipt,
};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

#[derive(Debug, Default)]
struct ScriptedInstall {
    calls: Vec<(ColdInstallPhase, Duration)>,
    failures: Vec<ColdInstallPhase>,
    clock: Option<Rc<Cell<u64>>>,
    overrun_phase: Option<ColdInstallPhase>,
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
        if self.overrun_phase == Some(phase) {
            if let Some(clock) = self.clock.as_ref() {
                clock.set(clock.get() + deadline.as_millis() as u64);
            }
        }
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

struct InjectedClock(Rc<Cell<u64>>);

impl ColdInstallClock for InjectedClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.get())
    }
}

#[test]
fn clean_cold_install_runs_stop_migrate_replace_start_and_readiness_in_order() {
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
            ColdInstallPhase::Migrate,
            ColdInstallPhase::Replace,
            ColdInstallPhase::Start,
            ColdInstallPhase::Readiness
        ]
    );
    assert_eq!(effects.calls[0].1, Duration::from_secs(30));
    assert_eq!(effects.calls[1].1, Duration::from_secs(30));
    assert_eq!(effects.calls[2].1, Duration::from_secs(60));
    assert_eq!(effects.calls[3].1, Duration::from_secs(30));
    assert_eq!(effects.calls[4].1, Duration::from_secs(30));
}

#[test]
fn stop_failure_prevents_migration_and_replacement() {
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
}

#[test]
fn post_migration_failure_preserves_original_error_without_rollback() {
    let mut effects = ScriptedInstall::new([ColdInstallPhase::Replace]);

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
            ColdInstallPhase::Migrate,
            ColdInstallPhase::Replace,
        ]
    );
    assert_eq!(receipt.original_error.as_deref(), Some("Replace_failed"));
}

#[test]
fn readiness_failure_stays_on_the_new_generation() {
    let mut effects = ScriptedInstall::new([ColdInstallPhase::Readiness]);

    let receipt = execute_workstation_cold_install(&mut effects);

    assert!(!receipt.success);
    assert_eq!(
        effects.calls.last().map(|(phase, _)| *phase),
        Some(ColdInstallPhase::Readiness)
    );
    assert_eq!(receipt.original_error.as_deref(), Some("Readiness_failed"));
}

#[test]
fn replacement_deadline_overrun_fails_without_restoring_the_old_generation() {
    let now = Rc::new(Cell::new(0));
    let clock = InjectedClock(now.clone());
    let mut effects = ScriptedInstall {
        clock: Some(now),
        overrun_phase: Some(ColdInstallPhase::Replace),
        ..ScriptedInstall::default()
    };

    let receipt = execute_workstation_cold_install_with_clock(&mut effects, &clock);

    assert!(!receipt.success);
    assert_eq!(
        receipt.original_error.as_deref(),
        Some("cold_install_replace_deadline_exceeded")
    );
    assert_eq!(
        effects
            .calls
            .iter()
            .map(|(phase, _)| *phase)
            .collect::<Vec<_>>(),
        vec![
            ColdInstallPhase::Stop,
            ColdInstallPhase::Migrate,
            ColdInstallPhase::Replace,
        ]
    );
}
