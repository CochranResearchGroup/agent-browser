//! Bounded cold shutdown orchestration for the installed workstation.
//!
//! This module deliberately does not accept upgrade transactions, admission
//! drains, census digests, rollback state, or runtime-generation decisions.
//! Those values may be useful diagnostics, but they cannot veto an explicit
//! operator request to stop Agent Browser-owned machinery.

use serde::Serialize;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use crate::native::service_store::{
    JsonServiceStateStore, LockedServiceStateRepository, ServiceStateRepository,
};
use crate::process_identity::{
    ProcessObservation, RecordedProcessIdentity, VerifiedProcessSignal, VerifiedProcessTermination,
};

const WORKSTATION_USER_UNITS: [&str; 6] = [
    "agent-browser-dashboard-backend.service",
    "agent-browser-dashboard.service",
    "agent-browser-runtime-interlock.service",
    "agent-browser-runtime-interlock.timer",
    "agent-browser-guacamole-postgres-backup.service",
    "agent-browser-guacamole-postgres-backup.timer",
];
const PRESENTATION_CONTAINERS: [&str; 3] = [
    "agent-browser-guacamole",
    "agent-browser-guacd",
    "agent-browser-guacamole-postgres",
];

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

trait ShutdownPlatform {
    fn close_owned_browsers(&mut self, deadline: Duration) -> Result<(bool, bool), String>;
    fn stop_owned_user_units(&mut self, deadline: Duration) -> Result<bool, String>;
    fn stop_owned_containers(&mut self, deadline: Duration) -> Result<bool, String>;
    fn release_ownership(&mut self) -> Result<bool, String>;
    fn remove_transient_metadata(&mut self) -> Result<bool, String>;
    fn observe_residue(&mut self, deadline: Duration) -> Result<ShutdownResidue, String>;
}

struct PlatformShutdownEffects<P> {
    platform: P,
}

impl<P> PlatformShutdownEffects<P> {
    fn new(platform: P) -> Self {
        Self { platform }
    }
}

impl<P: ShutdownPlatform> WorkstationShutdownEffects for PlatformShutdownEffects<P> {
    fn execute_phase(&mut self, phase: ShutdownPhase, deadline: Duration) -> ShutdownPhaseResult {
        let result = match phase {
            ShutdownPhase::Browsers => self
                .platform
                .close_owned_browsers(deadline)
                .map(|(changed, escalated)| (changed, escalated, None)),
            ShutdownPhase::UserUnits => self
                .platform
                .stop_owned_user_units(deadline)
                .map(|changed| (changed, false, None)),
            ShutdownPhase::Containers => self
                .platform
                .stop_owned_containers(deadline)
                .map(|changed| (changed, false, None)),
            ShutdownPhase::Ownership => self
                .platform
                .release_ownership()
                .map(|changed| (changed, false, None)),
            ShutdownPhase::TransientMetadata => self
                .platform
                .remove_transient_metadata()
                .map(|changed| (changed, false, None)),
            ShutdownPhase::Verify => self
                .platform
                .observe_residue(deadline)
                .map(|residue| (false, false, Some(residue))),
        };
        match result {
            Ok((changed, escalated, residue)) => ShutdownPhaseResult {
                step: ShutdownStepReceipt {
                    phase,
                    deadline_ms: deadline.as_millis() as u64,
                    changed,
                    escalated,
                    warnings: Vec::new(),
                    error: None,
                },
                residue,
            },
            Err(error) => ShutdownPhaseResult::step(ShutdownStepReceipt {
                phase,
                deadline_ms: deadline.as_millis() as u64,
                changed: false,
                escalated: false,
                warnings: Vec::new(),
                error: Some(error),
            }),
        }
    }
}

struct LiveShutdownPlatform {
    repository: LockedServiceStateRepository<JsonServiceStateStore>,
}

impl LiveShutdownPlatform {
    fn new() -> Result<Self, String> {
        Ok(Self {
            repository: LockedServiceStateRepository::default_json()?,
        })
    }

    fn installed_unit_names(&self) -> Result<Vec<&'static str>, String> {
        let root = crate::workstation_install::workstation_root()?;
        let unit_dir = root.join(".config/systemd/user");
        Ok(WORKSTATION_USER_UNITS
            .into_iter()
            .filter(|unit| unit_dir.join(unit).is_file())
            .collect())
    }

    fn stop_daemon_endpoint(
        &self,
        endpoint: &str,
        deadline: Duration,
    ) -> Result<(bool, bool), String> {
        let identity = match crate::connection::load_daemon_process_identity(endpoint) {
            Ok(identity) => identity,
            Err(_) if !crate::connection::daemon_ready(endpoint) => return Ok((false, false)),
            Err(error) => return Err(error),
        };
        let Some(process) = VerifiedProcessTermination::open(&identity)? else {
            return Ok((false, false));
        };
        if deadline.is_zero() {
            return Err(format!("daemon_shutdown_deadline_exceeded:{endpoint}"));
        }
        let started = Instant::now();
        process.signal(VerifiedProcessSignal::Terminate)?;
        while started.elapsed() < deadline && process.is_running()? {
            thread::sleep(Duration::from_millis(25));
        }
        if !process.is_running()? {
            return Ok((true, false));
        }
        process.signal(VerifiedProcessSignal::Kill)?;
        while started.elapsed() < deadline && process.is_running()? {
            thread::sleep(Duration::from_millis(25));
        }
        if process.is_running()? {
            return Err(format!("daemon_shutdown_deadline_exceeded:{endpoint}"));
        }
        Ok((true, true))
    }

    fn stop_recorded_browsers(
        &self,
        deadline: Duration,
        state: &crate::native::service_model::ServiceState,
    ) -> Result<(bool, bool), String> {
        let started = Instant::now();
        let mut changed = false;
        let mut escalated = false;
        for identity in state.browser_process_identities.values() {
            let Some(process) = VerifiedProcessTermination::open(&identity.process_identity)?
            else {
                continue;
            };
            if !process.is_running()? {
                continue;
            }
            if started.elapsed() >= deadline {
                return Err("browser_shutdown_deadline_exceeded".to_string());
            }
            process.signal(VerifiedProcessSignal::Terminate)?;
            changed = true;
            let polite_deadline = deadline
                .saturating_sub(started.elapsed())
                .min(Duration::from_millis(500));
            let polite_started = Instant::now();
            while polite_started.elapsed() < polite_deadline && process.is_running()? {
                thread::sleep(Duration::from_millis(25));
            }
            if process.is_running()? {
                process.signal(VerifiedProcessSignal::Kill)?;
                escalated = true;
            }
            while started.elapsed() < deadline && process.is_running()? {
                thread::sleep(Duration::from_millis(25));
            }
            if process.is_running()? {
                return Err(format!(
                    "browser_shutdown_deadline_exceeded:{}",
                    identity.process_identity.pid
                ));
            }
        }
        Ok((changed, escalated))
    }

    fn stop_exact_process_identities<'a>(
        &self,
        deadline: Duration,
        identities: impl IntoIterator<Item = &'a RecordedProcessIdentity>,
    ) -> Result<(bool, bool), String> {
        let started = Instant::now();
        let mut changed = false;
        let mut escalated = false;
        for identity in identities {
            let Some(process) = VerifiedProcessTermination::open(identity)? else {
                continue;
            };
            if !process.is_running()? {
                continue;
            }
            if started.elapsed() >= deadline {
                return Err("browser_session_shutdown_deadline_exceeded".to_string());
            }
            process.signal(VerifiedProcessSignal::Terminate)?;
            changed = true;
            let polite_deadline = deadline
                .saturating_sub(started.elapsed())
                .min(Duration::from_millis(500));
            let polite_started = Instant::now();
            while polite_started.elapsed() < polite_deadline && process.is_running()? {
                thread::sleep(Duration::from_millis(25));
            }
            if process.is_running()? {
                process.signal(VerifiedProcessSignal::Kill)?;
                escalated = true;
            }
            while started.elapsed() < deadline && process.is_running()? {
                thread::sleep(Duration::from_millis(25));
            }
            if process.is_running()? {
                return Err(format!(
                    "browser_session_shutdown_deadline_exceeded:{}",
                    identity.pid
                ));
            }
        }
        Ok((changed, escalated))
    }
}

impl ShutdownPlatform for LiveShutdownPlatform {
    fn close_owned_browsers(&mut self, deadline: Duration) -> Result<(bool, bool), String> {
        let state = self.repository.load_snapshot()?;
        let manager_store =
            crate::native::browser_session_store::BrowserSessionJsonStore::default_json()?;
        let mut manager_state = manager_store.load_session_state()?;
        let mut sessions = BTreeSet::new();
        for browser in state.browsers.values() {
            sessions.extend(browser.active_session_ids.iter().cloned());
        }
        for session in state.sessions.values() {
            if !session.browser_ids.is_empty() {
                sessions.insert(session.id.clone());
            }
        }
        let mut daemon_endpoints = sessions
            .iter()
            .map(|session| crate::runtime_host::endpoint_key(session).to_string())
            .collect::<BTreeSet<_>>();
        if crate::runtime_host::admission_enabled() {
            daemon_endpoints.insert(crate::runtime_host::RUNTIME_HOST_ENDPOINT_KEY.to_string());
        }

        let started = Instant::now();
        let mut changed = false;
        let mut close_failures = Vec::new();
        for session in sessions {
            let remaining = deadline.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                close_failures.push("browser_close_deadline_exceeded".to_string());
                break;
            }
            match crate::connection::send_command_with_timeout(
                json!({"id": uuid::Uuid::new_v4().to_string(), "action": "close"}),
                &session,
                remaining,
            ) {
                Ok(response) if response.success => changed = true,
                Ok(response) => close_failures.push(
                    response
                        .error
                        .unwrap_or_else(|| format!("browser_close_failed:{session}")),
                ),
                Err(error) => {
                    close_failures.push(format!("browser_close_failed:{session}:{error}"))
                }
            }
        }
        let mut escalated = false;
        for endpoint in daemon_endpoints {
            let (endpoint_changed, endpoint_escalated) =
                self.stop_daemon_endpoint(&endpoint, deadline.saturating_sub(started.elapsed()))?;
            changed |= endpoint_changed;
            escalated |= endpoint_escalated;
        }
        let (browser_changed, browser_escalated) =
            self.stop_recorded_browsers(deadline.saturating_sub(started.elapsed()), &state)?;
        changed |= browser_changed;
        escalated |= browser_escalated;
        for browser in manager_state.browsers.values() {
            if browser.process_identity.is_none()
                && matches!(
                    crate::process_identity::observe_process(browser.pid),
                    ProcessObservation::Observed(_)
                )
            {
                return Err(format!(
                    "browser_session_shutdown_identity_missing:{}",
                    browser.id
                ));
            }
        }
        let (manager_changed, manager_escalated) = self.stop_exact_process_identities(
            deadline.saturating_sub(started.elapsed()),
            manager_state
                .browsers
                .values()
                .filter_map(|browser| browser.process_identity.as_ref()),
        )?;
        changed |= manager_changed;
        escalated |= manager_escalated;
        if !manager_state.browsers.is_empty()
            || !manager_state.sessions.is_empty()
            || !manager_state.tabs.is_empty()
        {
            manager_state.terminalize_after_cold_shutdown(
                chrono::Utc::now().timestamp_millis().max(0) as u64,
            );
            manager_store.save_session_state(&manager_state)?;
            changed = true;
        }
        if !close_failures.is_empty()
            && state.browser_process_identities.values().any(|identity| {
                VerifiedProcessTermination::open(&identity.process_identity)
                    .ok()
                    .flatten()
                    .is_some_and(|process| process.is_running().unwrap_or(true))
            })
        {
            return Err(close_failures.join("; "));
        }
        Ok((changed, escalated))
    }

    fn stop_owned_user_units(&mut self, deadline: Duration) -> Result<bool, String> {
        let units = self.installed_unit_names()?;
        if units.is_empty() {
            return Ok(false);
        }
        let started = Instant::now();
        let mut active = Vec::new();
        for unit in units {
            let status = run_bounded(
                "systemctl",
                &["--user", "is-active", "--quiet", unit],
                deadline.saturating_sub(started.elapsed()),
            )?;
            if status.success() {
                active.push(unit);
            }
        }
        if active.is_empty() {
            return Ok(false);
        }
        let mut args = vec!["--user", "stop"];
        args.extend(active.iter().copied());
        let status = run_bounded(
            "systemctl",
            &args,
            deadline.saturating_sub(started.elapsed()),
        )?;
        status
            .success()
            .then_some(true)
            .ok_or_else(|| format!("workstation_user_unit_stop_failed:{status}"))
    }

    fn stop_owned_containers(&mut self, deadline: Duration) -> Result<bool, String> {
        let started = Instant::now();
        let mut present = Vec::new();
        for container in PRESENTATION_CONTAINERS {
            let remaining = deadline.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Err("presentation_container_inspection_deadline_exceeded".to_string());
            }
            match run_bounded("docker", &["top", container], remaining) {
                Ok(status) if status.success() => present.push(container),
                Ok(_) => {}
                Err(error) if error.contains("No such file or directory") => return Ok(false),
                Err(error) => return Err(error),
            }
        }
        if present.is_empty() {
            return Ok(false);
        }
        let remaining = deadline.saturating_sub(started.elapsed());
        let mut args = vec!["stop", "--time", "2"];
        args.extend(present.iter().copied());
        let status = run_bounded("docker", &args, remaining)?;
        status
            .success()
            .then_some(true)
            .ok_or_else(|| format!("presentation_container_stop_failed:{status}"))
    }

    fn release_ownership(&mut self) -> Result<bool, String> {
        let receipt = release_service_authority_for_cold_shutdown(
            &self.repository,
            &chrono::Utc::now().to_rfc3339(),
        )?;
        Ok(receipt.released_runtime_owners > 0
            || receipt.released_resource_claims > 0
            || receipt.released_sessions > 0
            || receipt.released_viewer_leases > 0
            || receipt.failed_pending_acquisitions > 0)
    }

    fn remove_transient_metadata(&mut self) -> Result<bool, String> {
        let socket_dir = crate::connection::get_socket_dir();
        let before = fs::read_dir(&socket_dir)
            .map(|entries| entries.flatten().count())
            .unwrap_or(0);
        let state = self.repository.load_snapshot()?;
        for session in state.sessions.keys() {
            crate::connection::cleanup_stale_files(session);
        }
        crate::connection::cleanup_stale_files(crate::runtime_host::RUNTIME_HOST_ENDPOINT_KEY);
        for name in ["dashboard.pid", "dashboard-backend.pid"] {
            match fs::remove_file(socket_dir.join(name)) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!("transient_metadata_remove_failed:{name}:{error}"))
                }
            }
        }
        let after = fs::read_dir(&socket_dir)
            .map(|entries| entries.flatten().count())
            .unwrap_or(0);
        Ok(after < before)
    }

    fn observe_residue(&mut self, deadline: Duration) -> Result<ShutdownResidue, String> {
        let state = self.repository.load_snapshot()?;
        let manager_state =
            crate::native::browser_session_store::BrowserSessionJsonStore::default_json()?
                .load_session_state()?;
        let started = Instant::now();
        let legacy_owned_browsers = state
            .browser_process_identities
            .values()
            .filter_map(|identity| {
                VerifiedProcessTermination::open(&identity.process_identity).ok()
            })
            .flatten()
            .filter(|process| process.is_running().unwrap_or(true))
            .count();
        let manager_owned_browsers = manager_state
            .browsers
            .values()
            .filter(|browser| match browser.process_identity.as_ref() {
                Some(identity) => VerifiedProcessTermination::open(identity)
                    .ok()
                    .flatten()
                    .is_some_and(|process| process.is_running().unwrap_or(true)),
                None => true,
            })
            .count();
        let owned_browsers = legacy_owned_browsers + manager_owned_browsers;
        let mut owned_user_units = 0;
        for unit in self.installed_unit_names()? {
            let status = run_bounded(
                "systemctl",
                &["--user", "is-active", "--quiet", unit],
                deadline.saturating_sub(started.elapsed()),
            )?;
            owned_user_units += usize::from(status.success());
        }
        let owned_containers = PRESENTATION_CONTAINERS
            .into_iter()
            .filter(|container| {
                run_bounded(
                    "docker",
                    &["top", container],
                    deadline.saturating_sub(started.elapsed()),
                )
                .is_ok_and(|status| status.success())
            })
            .count();
        let authority = state.runtime_lifecycle_authority_summary();
        let active_leases = state.lease_authority().active_claim_count();
        Ok(ShutdownResidue {
            owned_browsers,
            owned_user_units,
            owned_containers,
            runtime_owners: authority.owner_count,
            active_leases,
            foreign_processes: 0,
        })
    }
}

fn run_bounded(command: &str, args: &[&str], deadline: Duration) -> Result<ExitStatus, String> {
    if deadline.is_zero() {
        return Err(format!("command_deadline_exceeded:{command}"));
    }
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("command_start_failed:{command}:{error}"))?;
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("command_wait_failed:{command}:{error}"))?
        {
            return Ok(status);
        }
        if started.elapsed() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("command_deadline_exceeded:{command}"));
        }
        thread::sleep(Duration::from_millis(25));
    }
}

/// Execute the installed cold-shutdown command without accepting coordination
/// state, transaction identifiers, or destructive target selection.
pub(crate) fn run_workstation_shutdown_command(json_mode: bool) -> i32 {
    let receipt = match execute_live_workstation_shutdown() {
        Ok(receipt) => receipt,
        Err(error) => {
            if json_mode {
                println!("{}", json!({"success": false, "error": error}));
            } else {
                eprintln!("{}", error);
            }
            return 1;
        }
    };
    if json_mode {
        println!(
            "{}",
            serde_json::to_string_pretty(&receipt).unwrap_or_else(|_| {
                r#"{"success":false,"error":"serialization failed"}"#.to_string()
            })
        );
    } else if receipt.success {
        println!("Agent Browser shutdown complete");
    } else {
        eprintln!("Agent Browser shutdown incomplete; rerun with --json for phase receipts");
    }
    i32::from(!receipt.success)
}

pub(crate) fn execute_live_workstation_shutdown() -> Result<WorkstationShutdownReceipt, String> {
    let platform = LiveShutdownPlatform::new()?;
    Ok(execute_workstation_shutdown(
        &mut PlatformShutdownEffects::new(platform),
    ))
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

trait ShutdownClock {
    fn now(&self) -> Duration;
}

struct SystemShutdownClock;

impl ShutdownClock for SystemShutdownClock {
    fn now(&self) -> Duration {
        static ORIGIN: OnceLock<Instant> = OnceLock::new();
        ORIGIN.get_or_init(Instant::now).elapsed()
    }
}

/// Execute every cold-shutdown phase exactly once and always perform final
/// verification. A failed phase cannot prevent later cleanup from running.
pub(crate) fn execute_workstation_shutdown(
    effects: &mut impl WorkstationShutdownEffects,
) -> WorkstationShutdownReceipt {
    execute_workstation_shutdown_with_clock(effects, &SystemShutdownClock)
}

fn execute_workstation_shutdown_with_clock(
    effects: &mut impl WorkstationShutdownEffects,
    clock: &impl ShutdownClock,
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
        let started = clock.now();
        let mut result = effects.execute_phase(phase, deadline);
        result.step.phase = phase;
        result.step.deadline_ms = deadline.as_millis() as u64;
        if clock.now().saturating_sub(started) >= deadline {
            let phase_name = match phase {
                ShutdownPhase::Browsers => "browsers",
                ShutdownPhase::UserUnits => "user_units",
                ShutdownPhase::Containers => "containers",
                ShutdownPhase::Ownership => "ownership",
                ShutdownPhase::TransientMetadata => "transient_metadata",
                ShutdownPhase::Verify => "verify",
            };
            result.step.error = Some(format!("shutdown_{phase_name}_deadline_exceeded"));
        }
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
    use std::cell::Cell;
    use std::fs;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[derive(Default)]
    struct FakeEffects {
        calls: Vec<ShutdownPhase>,
        fail_browser_close: bool,
        residue: ShutdownResidue,
    }

    #[derive(Default)]
    struct FakePlatform {
        calls: Vec<ShutdownPhase>,
        fail_units: bool,
        residue: ShutdownResidue,
    }

    impl ShutdownPlatform for FakePlatform {
        fn close_owned_browsers(&mut self, _deadline: Duration) -> Result<(bool, bool), String> {
            self.calls.push(ShutdownPhase::Browsers);
            Ok((true, true))
        }

        fn stop_owned_user_units(&mut self, _deadline: Duration) -> Result<bool, String> {
            self.calls.push(ShutdownPhase::UserUnits);
            if self.fail_units {
                Err("unit_stop_failed".to_string())
            } else {
                Ok(true)
            }
        }

        fn stop_owned_containers(&mut self, _deadline: Duration) -> Result<bool, String> {
            self.calls.push(ShutdownPhase::Containers);
            Ok(false)
        }

        fn release_ownership(&mut self) -> Result<bool, String> {
            self.calls.push(ShutdownPhase::Ownership);
            Ok(true)
        }

        fn remove_transient_metadata(&mut self) -> Result<bool, String> {
            self.calls.push(ShutdownPhase::TransientMetadata);
            Ok(true)
        }

        fn observe_residue(&mut self, _deadline: Duration) -> Result<ShutdownResidue, String> {
            self.calls.push(ShutdownPhase::Verify);
            Ok(self.residue.clone())
        }
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

    struct InjectedClock(Rc<Cell<u64>>);

    impl ShutdownClock for InjectedClock {
        fn now(&self) -> Duration {
            Duration::from_millis(self.0.get())
        }
    }

    struct BrowserOverrunEffects {
        calls: Vec<ShutdownPhase>,
        clock: Rc<Cell<u64>>,
    }

    impl WorkstationShutdownEffects for BrowserOverrunEffects {
        fn execute_phase(
            &mut self,
            phase: ShutdownPhase,
            deadline: Duration,
        ) -> ShutdownPhaseResult {
            self.calls.push(phase);
            if phase == ShutdownPhase::Browsers {
                self.clock
                    .set(self.clock.get() + deadline.as_millis() as u64);
            }
            if phase == ShutdownPhase::Verify {
                ShutdownPhaseResult::verification(
                    ShutdownStepReceipt::unchanged(phase),
                    ShutdownResidue::default(),
                )
            } else {
                ShutdownPhaseResult::step(ShutdownStepReceipt::changed(phase))
            }
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
    fn shutdown_marks_an_overrun_and_still_executes_later_cleanup_and_verification() {
        let now = Rc::new(Cell::new(0));
        let clock = InjectedClock(now.clone());
        let mut effects = BrowserOverrunEffects {
            calls: Vec::new(),
            clock: now,
        };

        let receipt = execute_workstation_shutdown_with_clock(&mut effects, &clock);

        assert!(!receipt.success);
        assert_eq!(
            receipt.steps[0].error.as_deref(),
            Some("shutdown_browsers_deadline_exceeded")
        );
        assert_eq!(effects.calls.len(), 6);
        assert_eq!(effects.calls.last(), Some(&ShutdownPhase::Verify));
    }

    #[test]
    fn platform_adapter_maps_every_phase_and_retains_escalation() {
        let mut effects = PlatformShutdownEffects::new(FakePlatform::default());

        let receipt = execute_workstation_shutdown(&mut effects);

        assert!(receipt.success);
        assert!(receipt.steps[0].escalated);
        assert_eq!(
            effects.platform.calls,
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
    fn platform_adapter_continues_after_effect_failure() {
        let mut effects = PlatformShutdownEffects::new(FakePlatform {
            fail_units: true,
            ..FakePlatform::default()
        });

        let receipt = execute_workstation_shutdown(&mut effects);

        assert!(!receipt.success);
        assert_eq!(receipt.steps[1].error.as_deref(), Some("unit_stop_failed"));
        assert_eq!(effects.platform.calls.last(), Some(&ShutdownPhase::Verify));
        assert!(effects.platform.calls.contains(&ShutdownPhase::Ownership));
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
