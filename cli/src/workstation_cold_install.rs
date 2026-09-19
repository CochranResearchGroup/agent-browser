//! Provider-neutral cold workstation installation orchestration.
//!
//! The controller owns phase ordering and bounded deadlines. Platform adapters
//! perform the effects without exposing legacy hot-upgrade transaction state
//! or restoring the replaced architecture through this interface.

use serde::Serialize;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub(crate) const COLD_INSTALL_SCHEMA_VERSION: &str = "agent-browser.workstation-cold-install.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ColdInstallPhase {
    Stop,
    Migrate,
    Replace,
    Start,
    Readiness,
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

fn phase_deadline(phase: ColdInstallPhase) -> Duration {
    Duration::from_secs(match phase {
        ColdInstallPhase::Replace => 60,
        ColdInstallPhase::Stop
        | ColdInstallPhase::Migrate
        | ColdInstallPhase::Start
        | ColdInstallPhase::Readiness => 30,
    })
}

pub(crate) trait ColdInstallClock {
    fn now(&self) -> Duration;
}

struct SystemColdInstallClock;

impl ColdInstallClock for SystemColdInstallClock {
    fn now(&self) -> Duration {
        static ORIGIN: OnceLock<Instant> = OnceLock::new();
        ORIGIN.get_or_init(Instant::now).elapsed()
    }
}

fn execute_step(
    effects: &mut impl ColdInstallEffects,
    phase: ColdInstallPhase,
    clock: &impl ColdInstallClock,
) -> ColdInstallStepReceipt {
    let deadline = phase_deadline(phase);
    let started = clock.now();
    let mut step = effects.execute_phase(phase, deadline);
    step.phase = phase;
    step.deadline_ms = deadline.as_millis() as u64;
    if clock.now().saturating_sub(started) >= deadline {
        step.ready = false;
        let phase_name = match phase {
            ColdInstallPhase::Stop => "stop",
            ColdInstallPhase::Migrate => "migrate",
            ColdInstallPhase::Replace => "replace",
            ColdInstallPhase::Start => "start",
            ColdInstallPhase::Readiness => "readiness",
        };
        step.error = Some(format!("cold_install_{phase_name}_deadline_exceeded"));
    }
    step
}

/// Execute stop, migration, payload replacement, startup, and readiness in
/// fixed order. Every failure is terminal for this attempt and preserves the
/// new forward-repair architecture. The controller never restores an old
/// generation.
pub(crate) fn execute_workstation_cold_install(
    effects: &mut impl ColdInstallEffects,
) -> WorkstationColdInstallReceipt {
    execute_workstation_cold_install_with_clock(effects, &SystemColdInstallClock)
}

pub(crate) fn execute_workstation_cold_install_with_clock(
    effects: &mut impl ColdInstallEffects,
    clock: &impl ColdInstallClock,
) -> WorkstationColdInstallReceipt {
    let mut steps = Vec::with_capacity(5);
    for phase in [
        ColdInstallPhase::Stop,
        ColdInstallPhase::Migrate,
        ColdInstallPhase::Replace,
        ColdInstallPhase::Start,
        ColdInstallPhase::Readiness,
    ] {
        let mut step = execute_step(effects, phase, clock);
        if phase == ColdInstallPhase::Readiness && step.error.is_none() && !step.ready {
            step.error = Some("cold_install_readiness_not_ready".to_string());
        }
        let original_error = step.error.clone();
        steps.push(step);
        if let Some(original_error) = original_error {
            return WorkstationColdInstallReceipt {
                schema_version: COLD_INSTALL_SCHEMA_VERSION,
                success: false,
                changed: steps.iter().any(|step| step.changed),
                steps,
                original_error: Some(original_error),
            };
        }
    }

    WorkstationColdInstallReceipt {
        schema_version: COLD_INSTALL_SCHEMA_VERSION,
        success: true,
        changed: steps.iter().any(|step| step.changed),
        steps,
        original_error: None,
    }
}
