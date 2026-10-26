//! Original task effects in the existing Browser Runtime aggregate.
//! Pending is committed before transport. Neither a lost reply nor owner restart
//! authorizes replay. The separate private MAC key protects command fingerprints.
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

use agent_browser_service_model::{
    BrowserSessionState, BrowserTaskOperation, BrowserTaskOperationState,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;

use super::browser_session_store::BrowserSessionSqliteStore;

const MAXIMUM_TASK_OPERATIONS: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RetainedContext {
    task_id: String,
    profile_id: String,
    browser_id: String,
    tab_id: String,
    target_id: String,
    process_pid: u32,
    control_epoch: u64,
    process_identity: Option<agent_browser_service_model::RecordedProcessIdentity>,
}

impl RetainedContext {
    fn from_operation(operation: &BrowserTaskOperation) -> Self {
        Self {
            task_id: operation.task_id.clone(),
            profile_id: operation.profile_id.clone(),
            browser_id: operation.browser_id.clone(),
            tab_id: operation.tab_id.clone(),
            target_id: operation.target_id.clone(),
            process_pid: operation.process_pid,
            control_epoch: operation.control_epoch,
            process_identity: operation.process_identity.clone(),
        }
    }
}

pub(crate) enum Admission {
    Existing(BrowserTaskOperation),
    Dispatch(BrowserTaskOperation),
}

pub(crate) fn valid_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

/// Only single addressed page effects use this journal. Lifetime, profile,
/// Auth, Native input and multi-step playbooks keep their existing owner journals.
pub(crate) fn eligible_action(action: &str) -> bool {
    matches!(
        action,
        "navigate"
            | "click"
            | "fill"
            | "type"
            | "press"
            | "select"
            | "check"
            | "uncheck"
            | "scroll"
            | "scrollintoview"
            | "drag"
            | "evaluate"
            | "screenshot"
            | "download"
            | "reload"
            | "back"
            | "forward"
            | "setcontent"
            | "focus"
            | "clear"
            | "hover"
            | "keyboard"
            | "mouse"
    )
}

fn now_ms() -> Result<u64, String> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "browser_task_operation_clock_unavailable")?
        .as_millis() as u64)
}

fn context(state: &BrowserSessionState, task_id: &str) -> Result<RetainedContext, String> {
    let task = state
        .sessions
        .get(task_id)
        .filter(|task| task.id == task_id)
        .ok_or("browser_task_not_found")?;
    let browser = state
        .browsers
        .get(&task.browser_id)
        .filter(|browser| browser.profile_id == task.profile_id)
        .ok_or("browser_task_operation_browser_context_missing")?;
    let tab = task.current_tab_id.as_ref().and_then(|id| state.tabs.get(id))
        .filter(|tab| tab.session_id == task_id && tab.browser_id == task.browser_id)
        .ok_or("browser_task_operation_target_missing: transient state is unavailable; do not replay navigation")?;
    let epoch = super::browser_host_control::read(&browser.id)?.epoch;
    Ok(RetainedContext {
        task_id: task_id.into(),
        profile_id: task.profile_id.clone(),
        browser_id: browser.id.clone(),
        tab_id: tab.id.clone(),
        target_id: tab.target_id.clone(),
        process_pid: browser.pid,
        control_epoch: epoch,
        process_identity: BrowserSessionSqliteStore::open_read_only(
            &BrowserSessionSqliteStore::default_sqlite_path()?,
        )?
        .original_browser_identity(state, browser),
    })
}

/// Current retained identity only. Physical process verification remains the
/// existing immutable launch/adoption inspector and the runtime owner's check.
pub(crate) fn retained_context(task_id: &str) -> Result<Value, String> {
    let path = BrowserSessionSqliteStore::default_sqlite_path()?;
    let state = BrowserSessionSqliteStore::open_read_only(&path)?.load_session_state()?;
    serde_json::to_value(context(&state, task_id)?)
        .map_err(|_| "browser_task_operation_context_invalid".into())
}

fn key_path() -> Result<PathBuf, String> {
    let database = BrowserSessionSqliteStore::default_sqlite_path()?;
    Ok(database
        .parent()
        .ok_or("browser_task_operation_directory_missing")?
        .join("task-operation-mac.key"))
}

fn read_key(path: &std::path::Path) -> Result<[u8; 32], String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options.open(path).map_err(|_| "browser_task_operation_key_unavailable: inspect original operations; do not regenerate their key")?;
    let metadata = file
        .metadata()
        .map_err(|_| "browser_task_operation_key_unavailable")?;
    if !metadata.is_file() || metadata.len() != 32 {
        return Err("browser_task_operation_key_invalid".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // SAFETY: geteuid has no arguments or failure path.
        if metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
            || metadata.nlink() != 1
        {
            return Err("browser_task_operation_key_permissions_invalid".into());
        }
    }
    let mut key = [0u8; 32];
    file.read_exact(&mut key)
        .map_err(|_| "browser_task_operation_key_unavailable")?;
    Ok(key)
}

fn key(has_history: bool) -> Result<[u8; 32], String> {
    let path = key_path()?;
    match std::fs::symlink_metadata(&path) {
        Ok(_) => return read_key(&path),
        Err(error) if error.kind()==std::io::ErrorKind::NotFound && !has_history => {},
        Err(_) => return Err("browser_task_operation_key_unavailable: inspect original operations; do not regenerate their key".into()),
    }
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key).map_err(|_| "browser_task_operation_key_generation_failed")?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    match options.open(&path) {
        Ok(mut file) => {
            file.write_all(&key)
                .and_then(|_| file.sync_all())
                .map_err(|_| "browser_task_operation_key_publication_failed")?;
            #[cfg(unix)]
            File::open(
                path.parent()
                    .ok_or("browser_task_operation_directory_missing")?,
            )
            .and_then(|directory| directory.sync_all())
            .map_err(|_| "browser_task_operation_key_publication_failed")?;
            Ok(key)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => read_key(&path),
        Err(_) => Err("browser_task_operation_key_publication_failed".into()),
    }
}

fn mac(key: &[u8; 32], command: &Value) -> Result<Hmac<Sha256>, String> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key).map_err(|_| "browser_task_operation_key_invalid")?;
    let mut bytes =
        serde_json::to_vec(command).map_err(|_| "browser_task_operation_request_invalid")?;
    mac.update(&bytes);
    bytes.fill(0);
    Ok(mac)
}
fn same_request(
    key: &[u8; 32],
    command: &Value,
    original: &BrowserTaskOperation,
) -> Result<(), String> {
    let expected = hex::decode(&original.fingerprint)
        .map_err(|_| "browser_task_operation_fingerprint_invalid")?;
    mac(key, command)?.verify_slice(&expected).map_err(|_| {
        "browser_task_operation_request_conflict: inspect the original request; no replay".into()
    })
}

/// Admit once against exact current logical/physical scope before daemon ensure.
pub(crate) fn begin(task_id: &str, command: &mut Value) -> Result<Admission, String> {
    let request_id = command["id"]
        .as_str()
        .filter(|id| valid_request_id(id))
        .ok_or("browser_task_operation_request_id_invalid")?
        .to_owned();
    let action = command["action"]
        .as_str()
        .filter(|action| eligible_action(action))
        .ok_or("browser_task_operation_action_invalid")?
        .to_owned();
    let path = BrowserSessionSqliteStore::default_sqlite_path()?;
    let reader = BrowserSessionSqliteStore::open_read_only(&path)?;
    let state = reader.load_session_state()?;
    let has_history = state.task_operations.values().any(|rows| !rows.is_empty());
    if let Some(original) = state
        .task_operations
        .get(task_id)
        .and_then(|rows| rows.get(&request_id))
    {
        if original.task_id != task_id || original.request_id != request_id {
            return Err("browser_task_operation_identity_conflict".into());
        }
        same_request(&key(true)?, command, original)?;
        return Ok(Admission::Existing(original.clone()));
    }
    let expected = context(&state, task_id)?;
    let mode = reader.inspect_task_mode(task_id)?;
    if mode["mode"] == "unverified" {
        return Err("browser_task_operation_process_unverified: inspect current task; do not relaunch to discover an outcome".into());
    }
    let key = key(has_history)?;
    let fingerprint = hex::encode(mac(&key, command)?.finalize().into_bytes());
    let original = BrowserTaskOperation {
        request_id: request_id.clone(),
        task_id: task_id.into(),
        profile_id: expected.profile_id.clone(),
        browser_id: expected.browser_id.clone(),
        tab_id: expected.tab_id.clone(),
        target_id: expected.target_id.clone(),
        process_pid: expected.process_pid,
        process_identity: expected.process_identity.clone(),
        control_epoch: expected.control_epoch,
        action,
        fingerprint,
        state: BrowserTaskOperationState::Pending,
        created_at_ms: now_ms()?,
        completed_at_ms: None,
        evidence_references: BTreeMap::new(),
    };
    let mut writer = BrowserSessionSqliteStore::default_sqlite()?;
    let admitted = writer.mutate_session_state(|state| {
        if let Some(prior) = state
            .task_operations
            .get(task_id)
            .and_then(|rows| rows.get(&request_id))
        {
            same_request(&key, command, prior)?;
            return Ok(Admission::Existing(prior.clone()));
        }
        if context(state, task_id)? != expected {
            return Err("browser_task_operation_context_changed".into());
        }
        let rows = state.task_operations.entry(task_id.into()).or_default();
        if rows.len() >= MAXIMUM_TASK_OPERATIONS {
            return Err(
                "browser_task_operation_capacity_reached: original history retained".into(),
            );
        }
        rows.insert(request_id.clone(), original.clone());
        Ok(Admission::Dispatch(original))
    })?;
    if let Admission::Dispatch(ref original) = admitted {
        command["taskOperationExpected"] =
            json!({"requestId":original.request_id,"context":expected});
        command["retainedTaskOnly"] = json!(true);
        command["noTransportReplay"] = json!(true);
    }
    Ok(admitted)
}

/// Recheck after queueing and before recovery, focus, tab acquisition or input.
pub(crate) fn check_expected(state: &BrowserSessionState, command: &Value) -> Result<(), String> {
    if let Some(expected) = command.get("taskOperationExpected") {
        let request_id = expected["requestId"]
            .as_str()
            .ok_or("browser_task_operation_context_invalid")?;
        let scope: RetainedContext = serde_json::from_value(expected["context"].clone())
            .map_err(|_| "browser_task_operation_context_invalid")?;
        let original = state
            .task_operations
            .get(&scope.task_id)
            .and_then(|rows| rows.get(request_id))
            .ok_or("browser_task_operation_original_missing")?;
        if original.state != BrowserTaskOperationState::Pending
            || RetainedContext::from_operation(original) != scope
            || command["id"] != request_id
            || command["action"] != original.action
            || command["sessionId"] != scope.task_id
            || context(state, &scope.task_id)? != scope
        {
            return Err(
                "browser_task_operation_context_changed: inspect original operation; no replay"
                    .into(),
            );
        }
    }
    if let Some(expected) = command.get("retainedTaskContext") {
        let scope: RetainedContext = serde_json::from_value(expected.clone())
            .map_err(|_| "browser_task_operation_context_invalid")?;
        if command["action"] != "url"
            || command["sessionId"] != scope.task_id
            || context(state, &scope.task_id)? != scope
        {
            return Err("browser_task_continuation_context_changed".into());
        }
    }
    Ok(())
}

fn screenshot_produced(response: &crate::connection::Response) -> bool {
    let Some(path) = response
        .data
        .as_ref()
        .and_then(|data| data["path"].as_str())
    else {
        return false;
    };
    if !std::path::Path::new(path).is_absolute() {
        return false;
    }
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() < 3 {
        return false;
    }
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut header = [0u8; 12];
    let Ok(length) = file.read(&mut header) else {
        return false;
    };
    length >= 8 && header[..8] == [137, 80, 78, 71, 13, 10, 26, 10]
        || length >= 3 && header[..3] == [255, 216, 255]
        || length >= 12 && &header[..4] == b"RIFF" && &header[8..12] == b"WEBP"
}

/// Merge completion into the current owner document, tolerating unrelated task
/// activity while preserving the original immutable dispatch identity.
pub(crate) fn finish(
    original: &BrowserTaskOperation,
    response: Option<&crate::connection::Response>,
) -> Result<BrowserTaskOperation, String> {
    let acknowledged = response.is_some_and(|response| response.success);
    let mut references = BTreeMap::new();
    if let Some(response) = response.filter(|response| response.success) {
        // Page/evaluate result bodies are untrusted and may contain secrets.
        // Native input has no opaque provider receipt in its public contract.
        if original.action == "screenshot" && screenshot_produced(response) {
            references.insert("artifactId".into(), original.request_id.clone());
            references.insert("artifactKind".into(), "screenshot".into());
        }
    }
    let completed = now_ms()?;
    BrowserSessionSqliteStore::default_sqlite()?.mutate_session_state(|state| {
        let current = state
            .task_operations
            .get_mut(&original.task_id)
            .and_then(|rows| rows.get_mut(&original.request_id))
            .ok_or("browser_task_operation_original_missing")?;
        if current.fingerprint != original.fingerprint
            || current.state != BrowserTaskOperationState::Pending
        {
            return Err("browser_task_operation_completion_conflict".into());
        }
        current.state = if acknowledged {
            BrowserTaskOperationState::OwnerAcknowledged
        } else {
            BrowserTaskOperationState::Uncertain
        };
        current.completed_at_ms = Some(completed);
        current.evidence_references = references;
        Ok(current.clone())
    })
}
