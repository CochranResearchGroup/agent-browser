//! Provider-neutral cold workstation installation orchestration.
//!
//! The controller owns phase ordering, bounded deadlines, and rollback
//! selection. Platform adapters perform the effects without exposing legacy
//! hot-upgrade transaction state through this interface.

use serde::Serialize;
use std::time::Duration;

pub(crate) const COLD_INSTALL_SCHEMA_VERSION: &str = "agent-browser.workstation-cold-install.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ColdInstallPhase {
    Stop,
    Replace,
    Start,
    Readiness,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ColdInstallStepReceipt {
    pub(crate) phase: ColdInstallPhase,
    pub(crate) changed: bool,
    pub(crate) ready: bool,
    pub(crate) deadline_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkstationColdInstallReceipt {
    pub(crate) schema_version: &'static str,
    pub(crate) success: bool,
    pub(crate) changed: bool,
    pub(crate) steps: Vec<ColdInstallStepReceipt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) original_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rollback: Option<ColdInstallStepReceipt>,
}

/// Effects required by the fixed cold-install sequence.
///
/// The adapter receives only the selected phase and controller-owned deadline.
/// It cannot make legacy transaction metadata a prerequisite for execution.
pub(crate) trait ColdInstallEffects {
    fn execute_phase(
        &mut self,
        phase: ColdInstallPhase,
        deadline: Duration,
    ) -> ColdInstallStepReceipt;
}

fn phase_deadline(_phase: ColdInstallPhase) -> Duration {
    Duration::from_secs(30)
}

fn execute_step(
    effects: &mut impl ColdInstallEffects,
    phase: ColdInstallPhase,
) -> ColdInstallStepReceipt {
    let deadline = phase_deadline(phase);
    let mut step = effects.execute_phase(phase, deadline);
    step.phase = phase;
    step.deadline_ms = deadline.as_millis() as u64;
    step
}

/// Execute stop, payload replacement, startup, and readiness in fixed order.
///
/// Stop failure is terminal because replacement never began. Any later
/// failure attempts exactly one bounded rollback while retaining the original
/// cause in the aggregate receipt.
pub(crate) fn execute_workstation_cold_install(
    effects: &mut impl ColdInstallEffects,
) -> WorkstationColdInstallReceipt {
    let mut steps = Vec::with_capacity(5);
    for phase in [
        ColdInstallPhase::Stop,
        ColdInstallPhase::Replace,
        ColdInstallPhase::Start,
        ColdInstallPhase::Readiness,
    ] {
        let mut step = execute_step(effects, phase);
        if phase == ColdInstallPhase::Readiness && step.error.is_none() && !step.ready {
            step.error = Some("cold_install_readiness_not_ready".to_string());
        }
        let original_error = step.error.clone();
        steps.push(step);
        if let Some(original_error) = original_error {
            let rollback = if phase == ColdInstallPhase::Stop {
                None
            } else {
                let rollback = execute_step(effects, ColdInstallPhase::Rollback);
                steps.push(rollback.clone());
                Some(rollback)
            };
            return WorkstationColdInstallReceipt {
                schema_version: COLD_INSTALL_SCHEMA_VERSION,
                success: false,
                changed: steps.iter().any(|step| step.changed),
                steps,
                original_error: Some(original_error),
                rollback,
            };
        }
    }

    WorkstationColdInstallReceipt {
        schema_version: COLD_INSTALL_SCHEMA_VERSION,
        success: true,
        changed: steps.iter().any(|step| step.changed),
        steps,
        original_error: None,
        rollback: None,
    }
}
