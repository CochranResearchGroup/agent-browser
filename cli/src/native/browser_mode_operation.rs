//! Durable mode-conversion coordinator used by the browser-session owner.
//! Provider-free durable mode-conversion coordination. This core does not launch,
//! close, attach, or operate a desktop. A lifecycle adapter must supply exact
//! process evidence before it can execute a returned action.
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Cdp,
    CdpFree,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub operation_id: String,
    pub browser_id: String,
    pub profile_id: String,
    pub original_process: String,
    pub executable: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_sha256: Option<String>,
    pub task_ids: Vec<String>,
    pub requested_mode: Mode,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Prepared,
    Paused,
    Closing,
    ClosePrompt,
    OriginalExited,
    LaunchDispatched,
    LaunchAbsent,
    ReplacementPending,
    Completed,
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    pub request: Request,
    pub phase: Phase,
    #[serde(default)]
    pub replacement: Option<Replacement>,
    #[serde(default)]
    pub close_method: Option<CloseMethod>,
    #[serde(default)]
    pub effective_mode: Option<Mode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_executable: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_executable_sha256: Option<String>,
    #[serde(default)]
    pub failure: Option<String>,
    #[serde(default)]
    pub launch_attempts: u32,
    #[serde(default)]
    pub restoring_previous: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    GracefulClose,
    Pause,
    ReturnToBrowser,
    RetryGracefulClose,
    ConfirmForceClose,
    Cancel,
    LaunchReplacement,
    ReconcileReplacement,
    RetryReplacement,
    RestorePrevious,
    Readback,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observation {
    ClosePrompt,
}

pub struct OperationStore {
    connection: Connection,
    lock_directory: PathBuf,
}
struct EffectGuard(std::fs::File);
impl Drop for EffectGuard {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
impl OperationStore {
    pub fn open(path: &std::path::Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|_| "mode_operation_store_unavailable")?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS browser_mode_operations (id TEXT PRIMARY KEY, browser_id TEXT NOT NULL, terminal INTEGER NOT NULL, state TEXT NOT NULL); CREATE UNIQUE INDEX IF NOT EXISTS browser_mode_operation_active ON browser_mode_operations(browser_id) WHERE terminal=0;").map_err(|_| "mode_operation_store_unavailable")?;
        let lock_directory = path.with_extension("mode-effects");
        std::fs::create_dir_all(&lock_directory).map_err(|_| "mode_operation_lock_unavailable")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&lock_directory, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| "mode_operation_lock_unavailable")?;
        }
        Ok(Self {
            connection,
            lock_directory,
        })
    }
    fn effect_guard(&self, browser_id: &str) -> Result<EffectGuard, String> {
        let path = self
            .lock_directory
            .join(format!("{:x}.lock", Sha256::digest(browser_id.as_bytes())));
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        }
        let file = options
            .open(path)
            .map_err(|_| "mode_operation_lock_unavailable")?;
        file.try_lock()
            .map_err(|_| "mode_operation_busy:read current operation")?;
        Ok(EffectGuard(file))
    }
    pub fn read(&self, id: &str) -> Result<Option<Operation>, String> {
        let value: Option<String> = self
            .connection
            .query_row(
                "SELECT state FROM browser_mode_operations WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| "mode_operation_read_failed")?;
        value
            .map(|value| {
                serde_json::from_str(&value).map_err(|_| "mode_operation_state_invalid".into())
            })
            .transpose()
    }
    pub fn request(&mut self, request: Request) -> Result<Operation, String> {
        if let Some(existing) = self.read(&request.operation_id)? {
            return if existing.request == request {
                Ok(existing)
            } else {
                Err("mode_operation_id_conflict".into())
            };
        }
        if [
            &request.operation_id,
            &request.browser_id,
            &request.profile_id,
            &request.original_process,
            &request.executable,
        ]
        .iter()
        .any(|value| value.is_empty())
            || request.task_ids.is_empty()
        {
            return Err("mode_operation_target_invalid".into());
        }
        let operation = Operation {
            request,
            phase: Phase::Prepared,
            replacement: None,
            close_method: None,
            effective_mode: None,
            effective_executable: None,
            effective_executable_sha256: None,
            failure: None,
            launch_attempts: 0,
            restoring_previous: false,
        };
        let state =
            serde_json::to_string(&operation).map_err(|_| "mode_operation_encode_failed")?;
        self.connection.execute("INSERT INTO browser_mode_operations(id,browser_id,terminal,state) VALUES (?1,?2,0,?3)", params![operation.request.operation_id, operation.request.browser_id, state]).map_err(|_| "mode_operation_browser_busy:inspect current operation")?;
        Ok(operation)
    }
    pub fn observe(
        &mut self,
        id: &str,
        original_process: &str,
        observation: Observation,
    ) -> Result<Operation, String> {
        let operation = self.read(id)?.ok_or("mode_operation_not_found")?;
        if operation.request.original_process != original_process {
            return Err("mode_operation_target_changed:refresh inventory".into());
        }
        let before =
            serde_json::to_string(&operation).map_err(|_| "mode_operation_encode_failed")?;
        let mut next = operation;
        match observation {
            Observation::ClosePrompt => {
                if !matches!(
                    next.phase,
                    Phase::Prepared | Phase::Closing | Phase::ClosePrompt
                ) {
                    return Err("mode_operation_phase_conflict:read current operation".into());
                }
                next.phase = Phase::ClosePrompt;
            }
        }
        let encoded = serde_json::to_string(&next).map_err(|_| "mode_operation_encode_failed")?;
        let changed = self
            .connection
            .execute(
                "UPDATE browser_mode_operations SET state=?1 WHERE id=?2 AND state=?3",
                params![encoded, id, before],
            )
            .map_err(|_| "mode_operation_write_failed")?;
        if changed != 1 {
            return Err("mode_operation_changed:read current operation".into());
        }
        Ok(next)
    }
}
impl Operation {
    pub fn actions(&self) -> Vec<Action> {
        match self.phase {
            Phase::Prepared => vec![Action::GracefulClose, Action::Pause, Action::Cancel],
            Phase::Paused => vec![Action::GracefulClose, Action::Cancel],
            Phase::Closing | Phase::ClosePrompt => vec![
                Action::ReturnToBrowser,
                Action::RetryGracefulClose,
                Action::ConfirmForceClose,
                Action::Cancel,
            ],
            Phase::OriginalExited => vec![Action::LaunchReplacement],
            Phase::LaunchDispatched | Phase::ReplacementPending => {
                vec![Action::ReconcileReplacement, Action::RestorePrevious]
            }
            Phase::LaunchAbsent => vec![Action::RetryReplacement, Action::RestorePrevious],
            Phase::Completed | Phase::Cancelled => vec![Action::Readback],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    pub process_identity: String,
    pub mode: Mode,
    pub debugging: bool,
    pub native_observation: bool,
    pub native_input: bool,
}
pub enum OriginalStatus {
    Live,
    Exited,
}
pub enum CloseOutcome {
    Exited,
    Prompt,
    Uncertain,
}
pub enum Choice {
    Pause,
    SelectExecutable {
        executable: String,
        sha256: String,
        confirmed: bool,
    },
    Relaunch {
        confirmed: bool,
    },
    Resume,
    RestorePrevious {
        confirmed: bool,
    },
    RetryGracefulClose,
    ForceClose {
        confirmed: bool,
    },
    ReturnToBrowser,
    Cancel,
}
/// OS/owner boundary. Each positive outcome must be independently observed;
/// a dispatch acknowledgement is not an exit, launch or publication receipt.
pub trait ModeRuntime {
    fn original_status(&mut self, request: &Request) -> Result<OriginalStatus, String>;
    fn validate_executable(&mut self, _request: &Request) -> Result<(), String> {
        Ok(())
    }
    fn graceful_close(&mut self, request: &Request) -> Result<CloseOutcome, String>;
    /// Dismiss only the exact live original's pending close and positively
    /// observe it still live. Failure keeps the operation fenced for readback.
    fn cancel_graceful_close(&mut self, _request: &Request) -> Result<bool, String> {
        Ok(false)
    }
    fn force_close(&mut self, request: &Request) -> Result<CloseOutcome, String>;
    fn focus(&mut self, request: &Request) -> Result<(), String>;
    fn launch(&mut self, request: &Request) -> Result<Replacement, String>;
    fn reconcile_replacement(&mut self, request: &Request) -> Result<Option<Replacement>, String>;
    /// Must prove current exact-profile absence, independently of a lost reply.
    fn replacement_absent(&mut self, _request: &Request) -> Result<bool, String> {
        Ok(false)
    }
    /// Reconcile and close only the original operation replacement, then prove
    /// absence before selecting its retained prior configuration.
    fn restore_previous(&mut self, _request: &Request) -> Result<Mode, String> {
        Err("mode_operation_restore_unavailable".into())
    }
    /// Retained previous executable configuration, independent of profile bytes.
    fn previous_executable(&self, _request: &Request) -> Option<String> {
        None
    }
    fn previous_executable_sha256(&self, _request: &Request) -> Option<String> {
        None
    }
    /// Positive exact publication proof permits sealing a lost terminal write.
    /// Missing evidence never authorizes another close or replacement launch.
    fn replacement_is_published(
        &mut self,
        _request: &Request,
        _replacement: &Replacement,
    ) -> Result<bool, String> {
        Ok(false)
    }
    fn publish(&mut self, request: &Request, replacement: &Replacement) -> Result<(), String>;
}
impl OperationStore {
    fn save_transition(&mut self, before: &Operation, next: &Operation) -> Result<(), String> {
        let previous = serde_json::to_string(before).map_err(|_| "mode_operation_encode_failed")?;
        let encoded = serde_json::to_string(next).map_err(|_| "mode_operation_encode_failed")?;
        let terminal = matches!(next.phase, Phase::Completed | Phase::Cancelled);
        let changed = self
            .connection
            .execute(
                "UPDATE browser_mode_operations SET state=?1,terminal=?2 WHERE id=?3 AND state=?4",
                params![encoded, terminal, before.request.operation_id, previous],
            )
            .map_err(|_| "mode_operation_write_failed")?;
        if changed != 1 {
            return Err("mode_operation_changed:read current operation".into());
        }
        Ok(())
    }
    fn transition(&mut self, operation: &mut Operation, phase: Phase) -> Result<(), String> {
        let mut next = operation.clone();
        next.phase = phase;
        self.save_transition(operation, &next)?;
        *operation = next;
        Ok(())
    }
    fn record_failure(&mut self, operation: &mut Operation, error: &str) -> Result<(), String> {
        let mut next = operation.clone();
        next.failure = Some(
            error
                .split(':')
                .next()
                .unwrap_or("mode_operation_unknown")
                .into(),
        );
        if error.starts_with("mode_operation_native_") {
            next.phase = Phase::ReplacementPending;
        }
        self.save_transition(operation, &next)?;
        *operation = next;
        Ok(())
    }
    pub fn advance(
        &mut self,
        id: &str,
        choice: Choice,
        runtime: &mut impl ModeRuntime,
    ) -> Result<Operation, String> {
        let browser_id = self
            .read(id)?
            .ok_or("mode_operation_not_found")?
            .request
            .browser_id;
        let _guard = self.effect_guard(&browser_id)?;
        let mut operation = self.read(id)?.ok_or("mode_operation_not_found")?;
        if matches!(operation.phase, Phase::Completed | Phase::Cancelled) {
            return Ok(operation);
        }
        let mut effect_request = operation.request.clone();
        if let Some(executable) = &operation.effective_executable {
            effect_request.executable = executable.clone();
            effect_request.executable_sha256 = operation.effective_executable_sha256.clone();
        }
        if let Some(mode) = &operation.effective_mode {
            effect_request.requested_mode = mode.clone();
        }
        if let Choice::SelectExecutable {
            executable,
            sha256,
            confirmed,
        } = choice
        {
            if !confirmed {
                return Err("browser_build_owner_confirmation_required".into());
            }
            if !matches!(
                operation.phase,
                Phase::Prepared | Phase::Paused | Phase::OriginalExited | Phase::LaunchAbsent
            ) {
                return Err("mode_operation_phase_conflict:reconcile existing launch before choosing another build".into());
            }
            if !std::path::Path::new(&executable).is_absolute() || sha256.is_empty() {
                return Err("browser_build_selection_invalid".into());
            }
            if matches!(operation.phase, Phase::OriginalExited | Phase::LaunchAbsent)
                && (runtime.reconcile_replacement(&effect_request)?.is_some()
                    || !runtime.replacement_absent(&effect_request)?)
            {
                return Err(
                    "mode_operation_launch_absence_unproven:reconcile existing launch".into(),
                );
            }
            let mut selected = effect_request.clone();
            selected.executable = executable.clone();
            selected.executable_sha256 = Some(sha256.clone());
            runtime.validate_executable(&selected)?;
            let mut next = operation.clone();
            next.effective_executable = Some(executable);
            next.effective_executable_sha256 = Some(sha256);
            next.failure = None;
            self.save_transition(&operation, &next)?;
            return Ok(next);
        }
        if matches!(
            choice,
            Choice::Relaunch { .. } | Choice::RetryGracefulClose | Choice::ForceClose { .. }
        ) {
            runtime.validate_executable(&effect_request)?;
        }
        if let Choice::RestorePrevious { confirmed } = choice {
            if !confirmed {
                return Err("mode_operation_restore_confirmation_required".into());
            }
            if !matches!(
                operation.phase,
                Phase::OriginalExited
                    | Phase::LaunchDispatched
                    | Phase::LaunchAbsent
                    | Phase::ReplacementPending
            ) {
                return Err("mode_operation_phase_conflict:read current operation".into());
            }
        }
        let restore_requested = operation.restoring_previous
            || matches!(choice, Choice::RestorePrevious { confirmed: true });
        if restore_requested {
            if let Some(replacement) = operation.replacement.as_ref() {
                if runtime.replacement_is_published(&effect_request, replacement)? {
                    // The physical outcome already committed. Reject the new
                    // consequence, repair the lost journal tail, and release
                    // custody without closing that published browser.
                    let mut next = operation.clone();
                    next.phase = Phase::Completed;
                    next.restoring_previous = false;
                    self.save_transition(&operation, &next)?;
                    return Err("mode_operation_already_completed:inspect completion, then start a new conversion to change configuration".into());
                }
            }
        }
        if matches!(choice, Choice::RestorePrevious { confirmed: true }) {
            let mut next = operation.clone();
            next.restoring_previous = true;
            self.save_transition(&operation, &next)?;
            operation = next;
        }
        if operation.restoring_previous {
            let mode = runtime.restore_previous(&effect_request)?;
            let mut next = operation.clone();
            next.effective_mode = Some(mode.clone());
            next.effective_executable_sha256 = runtime.previous_executable_sha256(&effect_request);
            next.effective_executable = runtime
                .previous_executable(&effect_request)
                .or_else(|| Some(operation.request.executable.clone()));
            next.restoring_previous = false;
            next.replacement = None;
            next.phase = Phase::OriginalExited;
            self.save_transition(&operation, &next)?;
            operation = next;
            effect_request.requested_mode = mode;
            effect_request.executable_sha256 = operation.effective_executable_sha256.clone();
            effect_request.executable = operation
                .effective_executable
                .clone()
                .unwrap_or_else(|| operation.request.executable.clone());
        }
        if matches!(choice, Choice::Cancel) {
            if operation.phase == Phase::ClosePrompt {
                if !runtime.cancel_graceful_close(&effect_request)?
                    || !matches!(
                        runtime.original_status(&effect_request)?,
                        OriginalStatus::Live
                    )
                {
                    return Err(
                        "mode_operation_cancel_unproven:inspect or reconcile original close".into(),
                    );
                }
            } else if !matches!(operation.phase, Phase::Prepared | Phase::Paused) {
                return Err(
                    "mode_operation_cancel_after_dispatch:inspect and reconcile current effect"
                        .into(),
                );
            }
            self.transition(&mut operation, Phase::Cancelled)?;
            return Ok(operation);
        }
        if matches!(choice, Choice::ReturnToBrowser) {
            if !matches!(
                runtime.original_status(&effect_request)?,
                OriginalStatus::Live
            ) {
                return Err("mode_operation_original_exit_observed:reconcile replacement".into());
            }
            runtime.focus(&effect_request)?;
            return Ok(operation);
        }
        if let Choice::ForceClose { confirmed } = choice {
            if !confirmed {
                return Err("mode_operation_force_confirmation_required:possible data loss".into());
            }
            if !matches!(operation.phase, Phase::Closing | Phase::ClosePrompt) {
                return Err("mode_operation_phase_conflict:read current operation".into());
            }
            let mut next = operation.clone();
            next.phase = Phase::Closing;
            next.close_method = Some(CloseMethod::Forced);
            self.save_transition(&operation, &next)?;
            operation = next;
            match runtime.original_status(&effect_request)? {
                OriginalStatus::Live => match runtime.force_close(&effect_request)? {
                    CloseOutcome::Exited => {
                        self.transition(&mut operation, Phase::OriginalExited)?
                    }
                    CloseOutcome::Prompt | CloseOutcome::Uncertain => return Ok(operation),
                },
                OriginalStatus::Exited => self.transition(&mut operation, Phase::OriginalExited)?,
            }
        }
        if matches!(choice, Choice::RetryGracefulClose) {
            if !matches!(operation.phase, Phase::Closing | Phase::ClosePrompt) {
                return Err("mode_operation_phase_conflict:read current operation".into());
            }
            let mut next = operation.clone();
            next.phase = Phase::Closing;
            next.close_method = Some(CloseMethod::Graceful);
            self.save_transition(&operation, &next)?;
            operation = next;
            match runtime.original_status(&effect_request)? {
                OriginalStatus::Live => match runtime.graceful_close(&effect_request)? {
                    CloseOutcome::Exited => {
                        self.transition(&mut operation, Phase::OriginalExited)?
                    }
                    CloseOutcome::Prompt => {
                        self.transition(&mut operation, Phase::ClosePrompt)?;
                        return Ok(operation);
                    }
                    CloseOutcome::Uncertain => return Ok(operation),
                },
                OriginalStatus::Exited => self.transition(&mut operation, Phase::OriginalExited)?,
            }
        }
        if matches!(operation.phase, Phase::Closing | Phase::ClosePrompt)
            && matches!(
                runtime.original_status(&effect_request)?,
                OriginalStatus::Exited
            )
        {
            self.transition(&mut operation, Phase::OriginalExited)?;
        }
        if matches!(choice, Choice::Pause) {
            if operation.phase != Phase::Prepared && operation.phase != Phase::Paused {
                return Err("mode_operation_phase_conflict:read current operation".into());
            }
            if !matches!(
                runtime.original_status(&effect_request)?,
                OriginalStatus::Live
            ) {
                return Err(
                    "mode_operation_original_exit_observed:reconcile current target".into(),
                );
            }
            // The owner profile fence blocks all affected tasks. No implicit
            // close, timed retry or background relaunch follows this durable pause.
            self.transition(&mut operation, Phase::Paused)?;
            return Ok(operation);
        }
        if matches!(operation.phase, Phase::Prepared | Phase::Paused) {
            let Choice::Relaunch { confirmed } = choice else {
                return Ok(operation);
            };
            if operation.request.task_ids.len() > 1 && !confirmed {
                return Err("mode_operation_confirmation_required:review affected tasks".into());
            }
            match runtime.original_status(&effect_request)? {
                OriginalStatus::Live => {
                    let mut next = operation.clone();
                    next.phase = Phase::Closing;
                    next.close_method = Some(CloseMethod::Graceful);
                    self.save_transition(&operation, &next)?;
                    operation = next;
                    match runtime.graceful_close(&effect_request)? {
                        CloseOutcome::Exited => {
                            self.transition(&mut operation, Phase::OriginalExited)?
                        }
                        CloseOutcome::Prompt => {
                            self.transition(&mut operation, Phase::ClosePrompt)?;
                            return Ok(operation);
                        }
                        CloseOutcome::Uncertain => return Ok(operation),
                    }
                }
                OriginalStatus::Exited => self.transition(&mut operation, Phase::OriginalExited)?,
            }
        }
        if matches!(
            operation.phase,
            Phase::LaunchDispatched | Phase::LaunchAbsent | Phase::ReplacementPending
        ) && operation.replacement.is_none()
        {
            // Reconcile before consulting absence: a lost reply can already own
            // the original launch. Never interpret a missing reply as absence.
            if let Some(replacement) = match runtime.reconcile_replacement(&effect_request) {
                Ok(value) => value,
                Err(error) => {
                    self.record_failure(&mut operation, &error)?;
                    return Err(error);
                }
            } {
                let mut next = operation.clone();
                next.phase = Phase::LaunchDispatched;
                next.replacement = Some(replacement);
                self.save_transition(&operation, &next)?;
                operation = next;
            } else if runtime.replacement_absent(&effect_request)? {
                self.transition(&mut operation, Phase::LaunchAbsent)?;
                if matches!(choice, Choice::Relaunch { confirmed: true }) {
                    self.transition(&mut operation, Phase::OriginalExited)?;
                } else {
                    return Ok(operation);
                }
            } else {
                self.transition(&mut operation, Phase::LaunchDispatched)?;
                return Ok(operation);
            }
        }
        if operation.phase == Phase::OriginalExited {
            runtime.validate_executable(&effect_request)?;
            if !matches!(
                runtime.original_status(&effect_request)?,
                OriginalStatus::Exited
            ) {
                return Err("mode_operation_original_exit_unproven".into());
            }
            let mut next = operation.clone();
            next.phase = Phase::LaunchDispatched;
            next.launch_attempts += 1;
            self.save_transition(&operation, &next)?;
            operation = next;
            let replacement = match runtime.launch(&effect_request) {
                Ok(replacement) => replacement,
                Err(error) => {
                    self.record_failure(&mut operation, &error)?;
                    return Err(error);
                }
            };
            let mut next = operation.clone();
            next.replacement = Some(replacement);
            self.save_transition(&operation, &next)?;
            operation = next;
        }
        if matches!(
            operation.phase,
            Phase::LaunchDispatched | Phase::ReplacementPending
        ) {
            if operation.replacement.is_none() {
                // An interrupted launch is reconciled by operation identity. A
                // missing reply never authorizes another process launch.
                let replacement = match runtime.reconcile_replacement(&effect_request) {
                    Ok(value) => value,
                    Err(error) => {
                        self.record_failure(&mut operation, &error)?;
                        return Err(error);
                    }
                };
                let Some(replacement) = replacement else {
                    return Ok(operation);
                };
                let mut next = operation.clone();
                next.replacement = Some(replacement);
                self.save_transition(&operation, &next)?;
                operation = next;
            }
            let replacement = operation
                .replacement
                .as_ref()
                .ok_or("mode_operation_launch_readback_required")?;
            if replacement.process_identity.is_empty()
                || replacement.process_identity == operation.request.original_process
                || replacement.mode != effect_request.requested_mode
                || replacement.debugging != (replacement.mode == Mode::Cdp)
                || !replacement.native_observation
                || !replacement.native_input
            {
                return Err("mode_operation_replacement_proof_required".into());
            }
            runtime.publish(&effect_request, replacement)?;
            self.transition(&mut operation, Phase::Completed)?;
        }
        Ok(operation)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloseMethod {
    Graceful,
    Forced,
}
