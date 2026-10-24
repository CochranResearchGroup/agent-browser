//! Preparatory, unexposed mode-conversion coordinator checkpoint.
//! Provider-free durable mode-conversion coordination. This core does not launch,
//! close, attach, or operate a desktop. A lifecycle adapter must supply exact
//! process evidence before it can execute a returned action.
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

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
    pub task_ids: Vec<String>,
    pub requested_mode: Mode,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Prepared,
    Closing,
    ClosePrompt,
    OriginalExited,
    LaunchDispatched,
    Completed,
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    pub request: Request,
    pub phase: Phase,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    GracefulClose,
    ReturnToBrowser,
    RetryGracefulClose,
    ConfirmForceClose,
    Cancel,
    LaunchReplacement,
    ReconcileReplacement,
    Readback,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observation {
    ClosePrompt,
}

pub struct OperationStore {
    connection: Connection,
}
impl OperationStore {
    pub fn open(path: &std::path::Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|_| "mode_operation_store_unavailable")?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS browser_mode_operations (id TEXT PRIMARY KEY, browser_id TEXT NOT NULL, terminal INTEGER NOT NULL, state TEXT NOT NULL); CREATE UNIQUE INDEX IF NOT EXISTS browser_mode_operation_active ON browser_mode_operations(browser_id) WHERE terminal=0;").map_err(|_| "mode_operation_store_unavailable")?;
        Ok(Self { connection })
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
            Phase::Prepared => vec![Action::GracefulClose, Action::Cancel],
            Phase::Closing | Phase::ClosePrompt => vec![
                Action::ReturnToBrowser,
                Action::RetryGracefulClose,
                Action::ConfirmForceClose,
                Action::Cancel,
            ],
            Phase::OriginalExited => vec![Action::LaunchReplacement],
            Phase::LaunchDispatched => vec![Action::ReconcileReplacement],
            Phase::Completed | Phase::Cancelled => vec![Action::Readback],
        }
    }
}
