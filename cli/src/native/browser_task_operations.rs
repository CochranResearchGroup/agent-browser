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
use sha2::{Digest, Sha256};

use super::browser_session_store::BrowserSessionSqliteStore;

/// Native budgets include archived originals, while ordinary task reads keep a
/// bounded active working set. Archival never renews an attempt or login budget.
fn operation_history(
    state: &BrowserSessionState,
    task: &str,
) -> Result<BTreeMap<String, BrowserTaskOperation>, String> {
    let store = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path()?,
    )?;
    let mut rows = store.archived_task_operation_history(task)?;
    rows.extend(
        state
            .task_operations
            .get(task)
            .into_iter()
            .flat_map(|rows| rows.iter())
            .map(|(id, row)| (id.clone(), row.clone())),
    );
    if rows
        .values()
        .any(|row| row.state == BrowserTaskOperationState::OwnerRefusedNoEffect)
    {
        let secret = key(true)?;
        for row in rows
            .values()
            .filter(|row| row.state == BrowserTaskOperationState::OwnerRefusedNoEffect)
        {
            verified_no_effect(row, &secret)?;
        }
    }
    Ok(rows)
}

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

/// Only single addressed page effects use the ordinary journal admission.
/// Explicit Native challenge attempts use `begin_native` in the same store.
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

/// Opt-in only: generic Native input keeps its existing owner contract.
pub(crate) fn eligible_native(action: &str, params: &Value) -> bool {
    action == "native_capture"
        && (params.get("registeredActionId").is_some() || params.get("loginControls").is_some())
        || action == "native_input"
            && (params.get("nativeAttempt").is_some()
                || params.get("imageAttempt").is_some()
                || params.get("loginTarget").is_some())
}

pub(crate) fn begin_native(task_id: &str, command: &mut Value) -> Result<Admission, String> {
    if !eligible_native(command["action"].as_str().unwrap_or_default(), command) {
        return Err("browser_native_original_operation_invalid".into());
    }
    if command.get("loginControls").is_some() || command.get("loginTarget").is_some() {
        if command.get("loginControls").is_some() {
            super::native_login_perception::validate_request(&command["loginControls"])?;
        } else {
            login_target_request(command)?;
        }
        return begin_inner(task_id, command, true);
    }
    let action_id = if command["action"] == "native_capture" {
        if command["classifyChallenges"] != true {
            return Err("browser_native_registered_action_requires_classification".into());
        }
        command["registeredActionId"].as_str()
    } else if command.get("imageAttempt").is_some() {
        image_attempt_selection(command)?;
        Some(super::browser_native_image::command_profile(command))
    } else {
        let attempt = command["nativeAttempt"]
            .as_object()
            .filter(|attempt| {
                attempt.len() == 2
                    && attempt.contains_key("observationRequestId")
                    && attempt.contains_key("registeredActionId")
            })
            .ok_or("browser_native_attempt_invalid")?;
        if !attempt["observationRequestId"]
            .as_str()
            .is_some_and(valid_request_id)
        {
            return Err("browser_native_original_observation_request_invalid".into());
        }
        attempt["registeredActionId"].as_str()
    };
    if !action_id.is_some_and(|id| {
        super::browser_native_image::is_profile(id)
            || agent_browser_challenge_control::challenge_profile(id).is_some()
    }) {
        return Err("browser_native_registered_action_invalid".into());
    }
    begin_inner(task_id, command, true)
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

/// Serialize readers with the complete create, write and durable publication.
/// The stable private sidecar is never removed while another process may use it.
fn key_publication_guard(path: &std::path::Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path.with_extension("key.lock"))
        .map_err(|_| "browser_task_operation_key_publication_failed")?;
    let metadata = file
        .metadata()
        .map_err(|_| "browser_task_operation_key_publication_failed")?;
    if !metadata.is_file() {
        return Err("browser_task_operation_key_publication_failed".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // SAFETY: geteuid has no arguments or failure path.
        if metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
            || metadata.nlink() != 1
        {
            return Err("browser_task_operation_key_publication_failed".into());
        }
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(std::fs::TryLockError::WouldBlock) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            Err(_) => return Err("browser_task_operation_key_publication_failed".into()),
        }
    }
}

fn key(has_history: bool) -> Result<[u8; 32], String> {
    let path = key_path()?;
    let _publication_guard = key_publication_guard(&path)?;
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
            #[cfg(all(test, target_os = "linux"))]
            key_tests::after_create(&path)?;
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

#[cfg(all(test, target_os = "linux"))]
mod key_tests;

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

pub(crate) fn native_caller_fingerprint(task: &str, command: &Value) -> Result<String, String> {
    Ok(hex::encode(mac(&key(true)?, &json!({"schemaVersion":"agent-browser.native-caller.v1","taskId":task,"command":command}))?.finalize().into_bytes()))
}
pub(crate) fn exact_original(
    task: &str,
    request: &str,
) -> Result<Option<BrowserTaskOperation>, String> {
    let store = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path()?,
    )?;
    let state = store.load_session_state()?;
    Ok(state
        .task_operations
        .get(task)
        .and_then(|rows| rows.get(request))
        .cloned()
        .or(store.archived_task_operation(task, request)?))
}

fn dispatch_scope(original: &BrowserTaskOperation) -> Value {
    json!({"schemaVersion":"agent-browser.native-dispatch.v1","requestId":original.request_id,
        "taskId":original.task_id,"fingerprint":original.fingerprint,"context":RetainedContext::from_operation(original)})
}

const NO_EFFECT_PROOF: &str = "nativeOwnerNoEffectProof";
const NO_EFFECT_SIGNATURE: &str = "nativeOwnerNoEffectSignature";

/// The unconstructible permit comes only from the actual Native effect boundary.
/// It is not a public command argument or an interpretation of an error prefix.
pub(crate) fn record_native_no_effect(
    command: &Value,
    refusal: &super::browser_native_control::PreInputRefusal,
) -> Result<(), String> {
    let secret = key(true)?;
    let task = command["sessionId"]
        .as_str()
        .ok_or("browser_native_no_effect_scope_invalid")?;
    let request = command["id"]
        .as_str()
        .ok_or("browser_native_no_effect_scope_invalid")?;
    BrowserSessionSqliteStore::default_sqlite()?.mutate_session_state(|state| {
        let original = state.task_operations.get_mut(task).and_then(|rows| rows.get_mut(request))
            .filter(|row| row.action == "native_input" && matches!(row.state, BrowserTaskOperationState::Pending | BrowserTaskOperationState::Uncertain))
            .ok_or("browser_native_no_effect_original_missing")?;
        let scope = RetainedContext::from_operation(original);
        let expected_scope: RetainedContext = serde_json::from_value(command["taskOperationExpected"]["context"].clone()).map_err(|_| "browser_native_no_effect_scope_invalid")?;
        let seal = command["taskOperationExpected"]["dispatchSeal"].as_str().and_then(|seal| hex::decode(seal).ok()).ok_or("browser_native_no_effect_scope_invalid")?;
        mac(&secret, &dispatch_scope(original))?.verify_slice(&seal).map_err(|_| "browser_native_no_effect_scope_invalid")?;
        if scope != expected_scope || command["taskOperationExpected"]["requestId"] != request || command["action"] != original.action
            || original.process_identity.as_ref() != Some(refusal.process()) { return Err("browser_native_no_effect_scope_invalid".into()); }
        let proof = json!({"schemaVersion":"agent-browser.native-no-effect.v1","taskId":task,"requestId":request,
            "fingerprint":original.fingerprint,"callerFingerprint":original.evidence_references.get("callerCommandFingerprint"),"context":scope,"nativeTarget":refusal.target(),"effectStatus":"no_effect",
            "originalResponse":{"success":false,"code":refusal.code(),"error":refusal.response_error()}});
        let encoded = serde_json::to_string(&proof).map_err(|_| "browser_native_no_effect_proof_invalid")?;
        if let Some(prior) = original.evidence_references.get(NO_EFFECT_PROOF) {
            return if prior == &encoded { Ok(()) } else { Err("browser_native_no_effect_proof_conflict".into()) };
        }
        let signature = hex::encode(mac(&secret, &proof)?.finalize().into_bytes());
        original.evidence_references.insert(NO_EFFECT_PROOF.into(), encoded);
        original.evidence_references.insert(NO_EFFECT_SIGNATURE.into(), signature);
        Ok(())
    })
}

fn verified_no_effect(original: &BrowserTaskOperation, secret: &[u8; 32]) -> Result<Value, String> {
    let proof: Value = serde_json::from_str(original.evidence_references.get(NO_EFFECT_PROOF)
        .ok_or("browser_native_no_effect_proof_missing: inspect the original operation; uncertain input cannot be declared no-effect by the caller")?)
        .map_err(|_| "browser_native_no_effect_proof_invalid")?;
    let signature = hex::decode(
        original
            .evidence_references
            .get(NO_EFFECT_SIGNATURE)
            .ok_or("browser_native_no_effect_proof_invalid")?,
    )
    .map_err(|_| "browser_native_no_effect_proof_invalid")?;
    mac(secret, &proof)?
        .verify_slice(&signature)
        .map_err(|_| "browser_native_no_effect_proof_invalid")?;
    if proof["schemaVersion"] != "agent-browser.native-no-effect.v1"
        || proof["taskId"] != original.task_id
        || proof["requestId"] != original.request_id
        || proof["fingerprint"] != original.fingerprint
        || proof["callerFingerprint"]
            != serde_json::to_value(original.evidence_references.get("callerCommandFingerprint"))
                .map_err(|_| "browser_native_no_effect_scope_invalid")?
        || proof["context"]
            != serde_json::to_value(RetainedContext::from_operation(original))
                .map_err(|_| "browser_native_no_effect_scope_invalid")?
        || proof["effectStatus"] != "no_effect"
        || proof["originalResponse"]["success"] != false
    {
        return Err("browser_native_no_effect_scope_invalid".into());
    }
    Ok(proof)
}

/// Explicit read-first settlement. Only durable owner proof can resolve this
/// original; no input is dispatched and changed current scope stays refused.
pub(crate) fn reconcile_native_no_effect(
    task: &str,
    request: &str,
) -> Result<BrowserTaskOperation, String> {
    let secret = key(true)?;
    let mut store = BrowserSessionSqliteStore::default_sqlite()?;
    if let Some(row) = store.archived_task_operation(task, request)? {
        if row.state == BrowserTaskOperationState::OwnerRefusedNoEffect {
            verified_no_effect(&row, &secret)?;
            return Ok(row);
        }
        return Err("browser_native_no_effect_original_not_eligible".into());
    }
    store.mutate_session_state_with_connection(|state, connection| {
        let original = state
            .task_operations
            .get(task)
            .and_then(|rows| rows.get(request))
            .cloned()
            .ok_or("browser_task_operation_not_found")?;
        verified_no_effect(&original, &secret)?;
        if original.state == BrowserTaskOperationState::OwnerRefusedNoEffect {
            return Ok(original);
        }
        if original.action != "native_input"
            || !matches!(
                original.state,
                BrowserTaskOperationState::Pending | BrowserTaskOperationState::Uncertain
            )
        {
            return Err("browser_native_no_effect_original_not_eligible".into());
        }
        // Current identity/epoch may have advanced since this historical refusal.
        // Its immutable signed original remains valid; only a NEW intent needs
        // current context/freshness. No rollback/adoption is authorized here.
        let row = state
            .task_operations
            .get_mut(task)
            .unwrap()
            .get_mut(request)
            .unwrap();
        row.state = BrowserTaskOperationState::OwnerRefusedNoEffect;
        if row.completed_at_ms.is_none() {
            row.completed_at_ms = Some(now_ms()?);
        }
        connection
            .pragma_update(None, "user_version", 7)
            .map_err(|_| "browser_native_no_effect_publication_failed")?;
        Ok(row.clone())
    })
}

pub(crate) fn validate_no_effect_outcome(original: &BrowserTaskOperation) -> Result<(), String> {
    if original.state == BrowserTaskOperationState::OwnerRefusedNoEffect {
        verified_no_effect(original, &key(true)?)?;
    }
    Ok(())
}

pub(crate) fn original_no_effect_failure(original: &BrowserTaskOperation) -> Option<String> {
    (original.state == BrowserTaskOperationState::OwnerRefusedNoEffect).then(|| {
        original
            .evidence_references
            .get(NO_EFFECT_PROOF)
            .and_then(|encoded| serde_json::from_str::<Value>(encoded).ok())
            .and_then(|proof| {
                proof["originalResponse"]["error"]
                    .as_str()
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "browser_native_input_refused".into())
    })
}

/// Admit once against exact current logical/physical scope before daemon ensure.
pub(crate) fn begin(task_id: &str, command: &mut Value) -> Result<Admission, String> {
    begin_inner(task_id, command, false)
}

fn begin_inner(task_id: &str, command: &mut Value, native: bool) -> Result<Admission, String> {
    let request_id = command["id"]
        .as_str()
        .filter(|id| valid_request_id(id))
        .ok_or("browser_task_operation_request_id_invalid")?
        .to_owned();
    let action = command["action"]
        .as_str()
        .filter(|action| eligible_action(action) || native && eligible_native(action, command))
        .ok_or("browser_task_operation_action_invalid")?
        .to_owned();
    let path = BrowserSessionSqliteStore::default_sqlite_path()?;
    let reader = BrowserSessionSqliteStore::open_read_only(&path)?;
    let state = reader.load_session_state()?;
    let has_history = state.task_operations.values().any(|rows| !rows.is_empty())
        || reader.has_archived_task_operations()?;
    if let Some(original) = state
        .task_operations
        .get(task_id)
        .and_then(|rows| rows.get(&request_id))
        .cloned()
        .or(reader.archived_task_operation(task_id, &request_id)?)
    {
        if original.task_id != task_id || original.request_id != request_id {
            return Err("browser_task_operation_identity_conflict".into());
        }
        let secret = key(true)?;
        same_request(&secret, command, &original)?;
        if original.state == BrowserTaskOperationState::OwnerRefusedNoEffect {
            verified_no_effect(&original, &secret)?;
        }
        return Ok(Admission::Existing(original.clone()));
    }
    // Observe once before acquiring any writer lock. This is admission evidence,
    // never a substitute for the Native owner's later physical-input check.
    let native_time = if native && action == "native_input" {
        Some(crate::connection::stable_time::native_now()?)
    } else {
        None
    };
    // Resolve an original capture before transport or process recovery. Missing
    // evidence is never replaced by a caller timestamp or guessed coordinates.
    let observation = if native && action == "native_input" {
        if command.get("loginTarget").is_some() {
            original_login_observation(&state, task_id, command)?;
            login_episode_step(&state, task_id, command, None, native_time.as_ref())?;
        } else if command.get("imageAttempt").is_some() {
            let observed = original_native_image_observation(&state, task_id, command)?;
            let current = native_time
                .as_ref()
                .ok_or("browser_native_time_evidence_invalid")?;
            observed.validate_at(&super::browser_native_image::NativeImageClockReading {
                wall_time_ms: current.earliest_ms(),
                elapsed: None,
                service_time: Some(current.clone()),
            })?;
        } else {
            let observed = original_native_observation(&state, task_id, command)?;
            observed
                .service_time
                .as_ref()
                .ok_or("browser_native_time_recapture_required: obtain a fresh native_capture")?
                .fresh_at(
                    native_time
                        .as_ref()
                        .ok_or("browser_native_time_evidence_invalid")?,
                    10_000,
                )?;
        }
        Some(())
    } else {
        None
    };
    if observation.is_some() && command.get("loginTarget").is_none() {
        image_attempt_budget(&state, task_id, command)?;
    }
    let expected = context(&state, task_id)?;
    let mode = reader.inspect_task_mode(task_id)?;
    if mode["mode"] == "unverified" {
        let reason = match mode["reason"].as_str() {
            Some("original_identity_unavailable") => "original_identity_unavailable",
            Some("process_unavailable") => "process_unavailable",
            Some("process_identity_changed") => "process_identity_changed",
            Some("command_line_unavailable") => "command_line_unavailable",
            Some("not_browser_root") => "not_browser_root",
            Some("process_changed_during_observation") => "process_changed_during_observation",
            Some("owner_changed_during_observation") => "owner_changed_during_observation",
            Some("retained_debugger_endpoint") => "retained_debugger_endpoint",
            _ => "observation_unavailable",
        };
        return Err(format!("browser_task_operation_process_unverified: reason={reason}; no new effect dispatched; inspect browser_command action=mode for this task, then inspect task_briefing and original operations; missing original proof requires supported explicit mode_convert prepare/confirm recovery, never implicit adoption or replay"));
    }
    let key = key(has_history)?;
    let fingerprint = hex::encode(mac(&key, command)?.finalize().into_bytes());
    let mut original = BrowserTaskOperation {
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
        evidence_references: if command.get("loginTarget").is_some() {
            let login_observation = original_login_observation(&state, task_id, command)?;
            let qualified_now = super::native_login_perception::qualified_time(
                &login_observation,
                native_time
                    .as_ref()
                    .ok_or("browser_native_time_evidence_invalid")?,
            )?;
            let selected = super::native_login_perception::select_target(
                &login_observation,
                command["loginTarget"]["targetRef"]
                    .as_str()
                    .ok_or("browser_native_login_target_invalid")?,
                qualified_now,
            )?;
            BTreeMap::from([
                (
                    "nativeLoginObservationRequestId".into(),
                    command["loginTarget"]["observationRequestId"]
                        .as_str()
                        .unwrap()
                        .into(),
                ),
                (
                    "nativeLoginEpisode".into(),
                    command["loginTarget"]["episodeId"].as_str().unwrap().into(),
                ),
                ("nativeLoginTargetRef".into(), selected.target_ref),
                ("nativeLoginKind".into(), selected.kind),
            ])
        } else if observation.is_some() {
            BTreeMap::from([
                (
                    "nativeObservationRequestId".into(),
                    command
                        .get("imageAttempt")
                        .unwrap_or(&command["nativeAttempt"])["observationRequestId"]
                        .as_str()
                        .unwrap()
                        .into(),
                ),
                (
                    "nativeRegisteredActionId".into(),
                    if command.get("imageAttempt").is_some() {
                        super::browser_native_image::command_profile(command)
                    } else {
                        command["nativeAttempt"]["registeredActionId"]
                            .as_str()
                            .unwrap()
                    }
                    .into(),
                ),
            ])
        } else if command.get("loginControls").is_some() {
            BTreeMap::from([("nativeLoginProfile".into(), "english-chrome-v1".into())])
        } else {
            BTreeMap::new()
        },
    };
    let mut writer = BrowserSessionSqliteStore::default_sqlite()?;
    let admitted = writer.mutate_session_state_with_connection(|state, connection| {
        if let Some(prior) = state
            .task_operations
            .get(task_id)
            .and_then(|rows| rows.get(&request_id))
            .cloned()
            .or(
                super::browser_session_store::task_operation_archive::archived(
                    connection,
                    task_id,
                    &request_id,
                )?,
            )
        {
            same_request(&key, command, &prior)?;
            if prior.state == BrowserTaskOperationState::OwnerRefusedNoEffect {
                verified_no_effect(&prior, &key)?;
            }
            return Ok(Admission::Existing(prior.clone()));
        }
        if context(state, task_id)? != expected {
            return Err("browser_task_operation_context_changed".into());
        }
        let mut admitted = original.clone();
        if command["imageAttempt"].get("originalRequestId").is_some() {
            admitted.evidence_references.insert(
                "nativeImageOriginalRequestId".into(),
                command["imageAttempt"]["originalRequestId"]
                    .as_str()
                    .ok_or("browser_native_image_attempt_invalid")?
                    .into(),
            );
        }
        if observation.is_some() {
            if command.get("loginTarget").is_some() {
                original_login_observation(state, task_id, command)?;
                let step = login_episode_step(state, task_id, command, None, native_time.as_ref())?;
                admitted
                    .evidence_references
                    .insert("nativeLoginStep".into(), step.to_string());
            } else if command.get("imageAttempt").is_some() {
                original_native_image_observation(state, task_id, command)?;
            } else {
                original_native_observation(state, task_id, command)?;
            }
            super::browser_host_policy::check_state(state, task_id, command)?;
            if command.get("loginTarget").is_none() {
                image_attempt_budget(state, task_id, command)?;
            }
        }
        super::browser_session_store::task_operation_archive::make_room(
            connection, state, task_id,
        )?;
        let rows = state.task_operations.entry(task_id.into()).or_default();
        rows.insert(request_id.clone(), admitted.clone());
        Ok(Admission::Dispatch(admitted))
    })?;
    if let Admission::Dispatch(ref original) = admitted {
        let dispatch_seal = hex::encode(
            mac(&key, &dispatch_scope(original))?
                .finalize()
                .into_bytes(),
        );
        command["taskOperationExpected"] = json!({"requestId":original.request_id,"context":expected,"dispatchSeal":dispatch_seal});
        command["retainedTaskOnly"] = json!(true);
        command["noTransportReplay"] = json!(true);
        if native && action == "native_input" && command.get("ownerTimeoutMs").is_some() {
            // The original MAC and durable admission above exclude this private
            // observation. Reuse this boundary's qualified observation; the
            // physical owner still samples fresh time immediately before input.
            command["_nativeOwnerAdmittedServiceTime"] = serde_json::to_value(
                native_time
                    .as_ref()
                    .ok_or("browser_native_time_evidence_invalid")?,
            )
            .map_err(|_| "browser_native_time_evidence_invalid")?;
        }
    }
    Ok(admitted)
}

#[cfg(test)]
#[path = "browser_native_policy_tests.rs"]
mod native_policy_tests;

fn native_kernel_admit(
    state: &BrowserSessionState,
    task_id: &str,
    command: &Value,
    observation: &super::browser_native_control::NativeChallengeObservation,
    binding: &Value,
    current_time: &crate::connection::stable_time::Stamp,
) -> Result<agent_browser_challenge_control::ChallengePolicy, String> {
    use agent_browser_challenge_control::{
        decide, ChallengeDecision, ChallengeEvent, ChallengePolicy, ChallengeSnapshot,
        ObservationPosture,
    };
    super::browser_host_policy::check_state(state, task_id, command)?;
    let provider = binding["providerId"]
        .as_str()
        .ok_or("browser_native_site_policy_provider_missing")?
        .to_owned();
    let prior = operation_history(state, task_id)?.values().any(|row| {
        row.request_id != command["id"]
            && row.state != BrowserTaskOperationState::OwnerRefusedNoEffect
            && row.action == "native_input"
            && row
                .evidence_references
                .contains_key("nativeObservationRequestId")
    });
    let capture_time = observation
        .service_time
        .as_ref()
        .ok_or("browser_native_time_recapture_required: obtain a fresh native_capture")?;
    capture_time.fresh_at(current_time, 10_000)?;
    let policy = ChallengePolicy {
        policy_digest: binding["digest"]
            .as_str()
            .ok_or("browser_native_site_policy_invalid")?
            .into(),
        allowed_provider_ids: vec![provider.clone()],
        max_attempts: 1,
        max_steps: 2,
        max_pointer_events: 3,
        max_key_events: 0,
        cooldown_ms: 0,
        deadline_at_ms: capture_time.earliest_ms().saturating_add(10_000),
    };
    let snapshot = ChallengeSnapshot {
        attempts_started: u8::from(prior),
        ..Default::default()
    };
    let snapshot = match decide(
        &policy,
        &snapshot,
        ChallengeEvent::Observed {
            posture: ObservationPosture::Eligible,
            provider_id: provider,
            now_ms: current_time.latest_ms(),
        },
    )
    .map_err(|_| "browser_native_challenge_admission_refused")?
    {
        ChallengeDecision::NoEffect(snapshot) => snapshot,
        _ => return Err("browser_native_challenge_admission_refused".into()),
    };
    if !matches!(
        decide(
            &policy,
            &snapshot,
            ChallengeEvent::StartAttempt {
                now_ms: current_time.latest_ms()
            }
        ),
        Ok(ChallengeDecision::PermitAttempt(_))
    ) {
        return Err(
            "browser_native_challenge_attempt_budget_exhausted:human assistance required".into(),
        );
    }
    Ok(policy)
}

fn original_native_observation(
    state: &BrowserSessionState,
    task_id: &str,
    command: &Value,
) -> Result<super::browser_native_control::NativeChallengeObservation, String> {
    let request = command["nativeAttempt"]["observationRequestId"]
        .as_str()
        .filter(|id| valid_request_id(id))
        .ok_or("browser_native_original_observation_request_invalid")?;
    let history = operation_history(state, task_id)?;
    let capture = history
        .get(request)
        .ok_or("browser_native_original_observation_missing")?;
    let encoded = capture
        .evidence_references
        .get("nativeObservation")
        .filter(|value| value.len() <= 4096)
        .ok_or("browser_native_original_observation_unqualified")?;
    let observation: super::browser_native_control::NativeChallengeObservation =
        serde_json::from_str(encoded).map_err(|_| "browser_native_original_observation_invalid")?;
    if capture.action != "native_capture"
        || capture.state != BrowserTaskOperationState::OwnerAcknowledged
        || command["nativeAttempt"]["registeredActionId"] != observation.registered_action_id
        || RetainedContext::from_operation(capture) != context(state, task_id)?
    {
        return Err("browser_native_original_observation_context_changed".into());
    }
    observation.validate_structure()?;
    Ok(observation)
}

/// The actual Native adapter reopens this pending original immediately before
/// observing the physical window. Caller-supplied internal fields grant nothing.
pub(crate) fn native_attempt_observation(
    command: &Value,
) -> Result<super::browser_native_control::NativeChallengeObservation, String> {
    let task = command["sessionId"]
        .as_str()
        .ok_or("browser_native_original_operation_invalid")?;
    let request = command["id"]
        .as_str()
        .ok_or("browser_native_original_operation_invalid")?;
    let state = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path()?,
    )?
    .load_session_state()?;
    check_expected(&state, command)?;
    let original = state
        .task_operations
        .get(task)
        .and_then(|rows| rows.get(request))
        .ok_or("browser_native_original_operation_missing")?;
    if original.action != "native_input"
        || original.state != BrowserTaskOperationState::Pending
        || original
            .evidence_references
            .get("nativeObservationRequestId")
            .map(String::as_str)
            != command["nativeAttempt"]["observationRequestId"].as_str()
        || RetainedContext::from_operation(original) != context(&state, task)?
    {
        return Err("browser_native_original_operation_context_changed".into());
    }
    original_native_observation(&state, task, command)
}

struct NativePolicyObservation {
    family: String,
    registered_action_id: String,
}
fn native_policy_observation(command: &Value) -> Result<NativePolicyObservation, String> {
    if command.get("imageAttempt").is_some() {
        let observation = native_image_attempt_observation(command)?;
        Ok(NativePolicyObservation {
            family: "hcaptcha".into(),
            registered_action_id: observation.registered_action_id,
        })
    } else {
        let observation = native_attempt_observation(command)?;
        Ok(NativePolicyObservation {
            family: observation.family,
            registered_action_id: observation.registered_action_id,
        })
    }
}

fn native_profile_sources(
    command: &Value,
) -> Result<
    (
        agent_browser_service_model::ServiceState,
        String,
        BrowserSessionState,
    ),
    String,
> {
    native_policy_observation(command)?;
    let task_id = command["sessionId"]
        .as_str()
        .ok_or("browser_native_site_policy_task_missing")?;
    let reader = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path()?,
    )?;
    let state = reader.load_session_state()?;
    let task = state
        .sessions
        .get(task_id)
        .ok_or("browser_native_site_policy_task_missing")?;
    let catalog = reader.load_profile_catalog()?.catalog;
    let runtime_profile = catalog
        .profiles
        .get(&task.profile_id)
        .ok_or("browser_native_site_policy_profile_unbound")?;
    // This strict overlay deliberately does not use the flags convenience
    // loader, which falls back to defaults after a durable store read failure.
    let mut service = super::service_store::load_default_service_state_snapshot()?;
    service.overlay_configured_entities(crate::flags::load_config(&[])?.service_state_snapshot());
    let profile = service
        .profiles
        .get(&task.profile_id)
        .filter(|profile| profile.id == task.profile_id)
        .ok_or("browser_native_site_policy_profile_unbound")?;
    let source_path = profile
        .user_data_dir
        .as_ref()
        .ok_or("browser_native_site_policy_profile_unbound")?;
    if std::fs::canonicalize(source_path)
        .map_err(|_| "browser_native_site_policy_profile_unbound")?
        != std::fs::canonicalize(&runtime_profile.user_data_dir)
            .map_err(|_| "browser_native_site_policy_profile_unbound")?
    {
        return Err("browser_native_site_policy_profile_unbound".into());
    }
    let profile_id = task.profile_id.clone();
    Ok((service, profile_id, state))
}

pub(crate) fn native_site_mode_preflight(command: &Value) -> Result<(), String> {
    let (service, profile_id, _) = native_profile_sources(command)?;
    let profile = service
        .profiles
        .get(&profile_id)
        .ok_or("browser_native_site_policy_profile_unbound")?;
    if profile.site_policy_ids.is_empty() {
        return Err("browser_native_site_policy_requires_manual_assistance".into());
    }
    for id in &profile.site_policy_ids {
        let policy = service
            .site_policies
            .get(id)
            .ok_or("browser_native_site_policy_missing_or_ambiguous")?;
        if policy.requires_cdp_free {
            return Err(
                "browser_native_site_policy_requires_manual_or_cdp_free:human assistance required"
                    .into(),
            );
        }
    }
    Ok(())
}

/// Resolve actual owner SitePolicy records for a current protocol-observed
/// origin. Only the Native runtime adapter supplies this observation; public
/// input parameters cannot select a policy, origin or digest.
pub(crate) fn native_site_policy(command: &Value, verified_origin: &str) -> Result<Value, String> {
    let observation = native_policy_observation(command)?;
    let (service, profile_id, state) = native_profile_sources(command)?;
    let task_id = command["sessionId"]
        .as_str()
        .ok_or("browser_native_site_policy_task_missing")?;
    let profile = service
        .profiles
        .get(&profile_id)
        .ok_or("browser_native_site_policy_profile_unbound")?;
    let origin =
        url::Url::parse(verified_origin).map_err(|_| "browser_native_site_origin_unverified")?;
    if origin.scheme() != "https"
        || origin.origin().ascii_serialization() != verified_origin
        || !origin.username().is_empty()
        || origin.password().is_some()
    {
        return Err("browser_native_site_origin_unverified".into());
    }
    let applicable = profile
        .site_policy_ids
        .iter()
        .filter_map(|id| {
            service.site_policies.get(id).filter(|policy| {
                policy.id == *id
                    && (policy.origin_pattern == verified_origin
                        || policy
                            .origins
                            .iter()
                            .any(|origin| origin == verified_origin))
            })
        })
        .collect::<Vec<_>>();
    let [policy] = applicable.as_slice() else {
        return Err(
            "browser_native_site_policy_missing_or_ambiguous:human assistance required".into(),
        );
    };
    if policy.requires_cdp_free
        || policy.interaction_mode == agent_browser_service_model::InteractionMode::Manual
    {
        return Err(
            "browser_native_site_policy_requires_manual_or_cdp_free:human assistance required"
                .into(),
        );
    }
    if !matches!(
        policy.challenge_policy,
        agent_browser_service_model::ChallengePolicy::ProviderAllowed
            | agent_browser_service_model::ChallengePolicy::ProviderPreferred
    ) {
        return Err("browser_native_site_policy_attempt_denied:human assistance required".into());
    }
    let providers = service
        .providers
        .iter()
        .filter(|(id, provider)| provider.id == **id && provider.enabled)
        .map(|(_, provider)| provider.clone())
        .collect::<Vec<_>>();
    let decision = agent_browser_service_model::provider_decision(
        Some(profile),
        Some(policy),
        &[agent_browser_service_model::Challenge {
            id: "native-checkbox-observation".into(),
            kind: agent_browser_service_model::ChallengeKind::Captcha,
            ..Default::default()
        }],
        &providers,
    );
    let native = providers
        .iter()
        .filter(|provider| {
            provider.kind == agent_browser_service_model::ProviderKind::Captcha
                && provider
                    .capabilities
                    .contains(&agent_browser_service_model::ProviderCapability::CaptchaSolve)
                && provider.config_ref.as_deref()
                    == Some(if command.get("imageAttempt").is_some() {
                        if observation.registered_action_id
                            == super::browser_native_image::ROUNDS_PROFILE_ID
                        {
                            super::browser_native_image::ROUNDS_CONFIG_REF
                        } else {
                            super::browser_native_image::CONFIG_REF
                        }
                    } else {
                        "agent-browser:remote-view-native:v1"
                    })
                && decision.challenge_provider_ids.contains(&provider.id)
                && (command.get("imageAttempt").is_none()
                    || policy.allowed_challenge_providers.contains(&provider.id))
        })
        .collect::<Vec<_>>();
    let [provider] = native.as_slice() else {
        return Err(
            "browser_native_site_policy_provider_missing_or_ambiguous:human assistance required"
                .into(),
        );
    };
    let digest = hex::encode(Sha256::digest(serde_json::to_vec(&json!({
        "profileId":profile.id,"profileSitePolicyIds":profile.site_policy_ids,"sitePolicy":policy,"provider":provider,
        "taskPolicy":state.host_bindings.get(task_id).and_then(|binding| state.host_policies.effective(&binding.request.host_id,task_id)),
        "sitePolicySource":service.entity_sources.site_policies.get(&policy.id).map(|source| source.as_str()),
        "profileSource":service.entity_sources.profiles.get(&profile.id).map(|source| source.as_str()),
        "originSha256":hex::encode(Sha256::digest(verified_origin.as_bytes())),
    })).map_err(|_| "browser_native_site_policy_invalid")?));
    Ok(
        json!({"sitePolicyId":policy.id,"digest":digest,"originSha256":hex::encode(Sha256::digest(verified_origin.as_bytes())),
        "observedServiceRevision":service.state_revision(),"providerId":provider.id,"family":observation.family,"registeredActionId":observation.registered_action_id}),
    )
}

/// Bind verified owner policy to the already-pending original before focus or
/// input. Later checks tolerate unrelated Service State activity, never policy
/// or origin changes. No recognized page text or URL path is retained.
#[cfg(test)]
pub(crate) fn retain_native_site_policy(command: &Value, binding: &Value) -> Result<(), String> {
    if let Some(tag) = command["_taskOperationCallerFingerprint"].as_str() {
        original
            .evidence_references
            .insert("callerCommandFingerprint".into(), tag.into());
    }
    let mut store = BrowserSessionSqliteStore::default_sqlite()?;
    let expected = store.load_session_state()?;
    retain_native_site_policy_for_owner(&mut store, command, binding, &expected).map(|_| ())
}

/// Process-local proof of one exact owner journal publication. It cannot be
/// supplied by a browser command or reconstructed from a caller label.
pub(crate) struct NativePolicyPublication {
    before: BrowserSessionState,
    after: BrowserSessionState,
    context: RetainedContext,
}

impl NativePolicyPublication {
    pub(crate) fn check_before(&self, state: &BrowserSessionState) -> Result<(), String> {
        if state != &self.before {
            return Err("browser_native_policy_publication_conflict".into());
        }
        Ok(())
    }
    pub(crate) fn check_current(&self, state: &BrowserSessionState) -> Result<(), String> {
        if state != &self.after || context(state, &self.context.task_id)? != self.context {
            return Err("browser_native_policy_publication_conflict".into());
        }
        Ok(())
    }
    pub(crate) fn snapshot(&self) -> &BrowserSessionState {
        &self.after
    }
}

pub(crate) fn retain_native_site_policy_for_owner(
    store: &mut BrowserSessionSqliteStore,
    command: &Value,
    binding: &Value,
    expected: &BrowserSessionState,
) -> Result<NativePolicyPublication, String> {
    let task = command["sessionId"]
        .as_str()
        .ok_or("browser_native_original_operation_invalid")?;
    let request = command["id"]
        .as_str()
        .ok_or("browser_native_original_operation_invalid")?;
    // The supported client may wait on I/O; unrelated tasks must never wait
    // behind that I/O while this process owns the SQLite writer transaction.
    let qualified_time = crate::connection::stable_time::native_now()?;
    let after = store.mutate_session_state(|state| {
        if state != expected {
            return Err("browser_native_policy_publication_conflict".into());
        }
        check_expected(state, command)?;
        let original = state
            .task_operations
            .get(task)
            .and_then(|rows| rows.get(request))
            .filter(|row| {
                row.action == "native_input"
                    && row.state == BrowserTaskOperationState::Pending
                    && row.request_id == request
                    && row.task_id == task
            })
            .ok_or("browser_native_original_operation_missing")?;
        if RetainedContext::from_operation(original) != context(state, task)? {
            return Err("browser_native_original_observation_context_changed".into());
        }
        let image_plan = if command.get("imageAttempt").is_some() {
            Some(native_image_kernel_plan(
                state,
                task,
                command,
                binding,
                &qualified_time,
            )?)
        } else {
            None
        };
        let observation = if image_plan.is_none() {
            Some(original_native_observation(state, task, command)?)
        } else {
            None
        };
        if let Some(prior) = state
            .task_operations
            .get(task)
            .and_then(|rows| rows.get(request))
            .and_then(|row| row.evidence_references.get("nativeSitePolicy"))
        {
            let prior: Value =
                serde_json::from_str(prior).map_err(|_| "browser_native_site_policy_invalid")?;
            return if prior["digest"] == binding["digest"] {
                Ok(state.clone())
            } else {
                Err("browser_native_site_policy_changed".into())
            };
        }
        let (family, policy_digest, deadline) = if let Some(plan) = &image_plan {
            (
                "hcaptcha".to_owned(),
                plan.policy.policy_digest.clone(),
                plan.policy.deadline_at_ms,
            )
        } else {
            let observation = observation
                .as_ref()
                .ok_or("browser_native_original_observation_unqualified")?;
            let policy =
                native_kernel_admit(state, task, command, observation, binding, &qualified_time)?;
            (
                observation.family.clone(),
                policy.policy_digest,
                policy.deadline_at_ms,
            )
        };
        let original = state
            .task_operations
            .get_mut(task)
            .and_then(|rows| rows.get_mut(request))
            .filter(|row| {
                row.action == "native_input" && row.state == BrowserTaskOperationState::Pending
            })
            .ok_or("browser_native_original_operation_missing")?;
        let refs = &mut original.evidence_references;
        refs.insert(
            "nativeSitePolicy".into(),
            serde_json::to_string(binding).map_err(|_| "browser_native_site_policy_invalid")?,
        );
        refs.insert(
            "nativeProviderId".into(),
            binding["providerId"]
                .as_str()
                .ok_or("browser_native_site_policy_provider_missing")?
                .into(),
        );
        refs.insert("nativeChallengeFamily".into(), family);
        refs.insert("nativeChallengeState".into(), "solving".into());
        refs.insert("nativeChallengePolicyDigest".into(), policy_digest);
        refs.insert("nativeChallengeDeadlineAtMs".into(), deadline.to_string());
        if let Some(plan) = image_plan {
            refs.insert(
                "nativeImageRound".into(),
                super::browser_native_image::retain_plan(&plan)?,
            );
        }
        Ok(state.clone())
    })?;
    let retained = RetainedContext::from_operation(&after.task_operations[task][request]);
    Ok(NativePolicyPublication {
        before: expected.clone(),
        after,
        context: retained,
    })
}

pub(crate) fn check_native_site_policy(
    command: &Value,
    verified_origin: &str,
) -> Result<(), String> {
    let current = native_site_policy(command, verified_origin)?;
    let state = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path()?,
    )?
    .load_session_state()?;
    let retained = state
        .task_operations
        .get(command["sessionId"].as_str().unwrap_or_default())
        .and_then(|rows| rows.get(command["id"].as_str().unwrap_or_default()))
        .and_then(|row| row.evidence_references.get("nativeSitePolicy"))
        .ok_or("browser_native_site_policy_binding_missing")?;
    let retained: Value =
        serde_json::from_str(retained).map_err(|_| "browser_native_site_policy_invalid")?;
    if retained["digest"] != current["digest"]
        || retained["originSha256"] != current["originSha256"]
    {
        return Err("browser_native_site_policy_changed:human assistance required".into());
    }
    Ok(())
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
    let mut acknowledged = response.is_some_and(|response| {
        response.success
            && (original.action != "native_input"
                || response.data.as_ref().is_some_and(|data| {
                    data["source"] == "remote_view_native" && data["delivery"] == "dispatched"
                }))
    });
    let mut references = original.evidence_references.clone();
    if original.action == "native_input" && references.contains_key("nativeObservationRequestId") {
        let state = BrowserSessionSqliteStore::open_read_only(
            &BrowserSessionSqliteStore::default_sqlite_path()?,
        )?
        .load_session_state()?;
        references = state
            .task_operations
            .get(&original.task_id)
            .and_then(|rows| rows.get(&original.request_id))
            .filter(|row| {
                row.fingerprint == original.fingerprint
                    && row.state == BrowserTaskOperationState::Pending
            })
            .ok_or("browser_native_original_operation_missing")?
            .evidence_references
            .clone();
        if !references.contains_key("nativeSitePolicy")
            || !references.contains_key("nativeProviderId")
        {
            acknowledged = false;
        }
    }
    if let Some(response) = response.filter(|response| response.success) {
        // Page/evaluate result bodies are untrusted and may contain secrets.
        // Native input has no opaque provider receipt in its public contract.
        if original.action == "screenshot" && screenshot_produced(response) {
            references.insert("artifactId".into(), original.request_id.clone());
            references.insert("artifactKind".into(), "screenshot".into());
        }
        if original.action == "native_capture"
            && references.contains_key("nativeLoginProfile")
            && response
                .data
                .as_ref()
                .and_then(|d| d.get("nativeLoginObservation"))
                .is_none()
        {
            return Err("browser_native_login_producer_contract_unavailable".into());
        }
        if original.action == "native_capture" {
            if let Some(observation) = response
                .data
                .as_ref()
                .and_then(|d| d.get("nativeLoginObservation"))
            {
                let curated = super::native_login_perception::curate_observation(observation)?;
                let encoded = serde_json::to_string(&curated)
                    .map_err(|_| "browser_native_login_observation_invalid")?;
                if encoded.len() > 32768 {
                    return Err("browser_native_login_observation_invalid".into());
                }
                references.insert("nativeLoginObservation".into(), encoded);
            }
            if let Some(value) = response
                .data
                .as_ref()
                .and_then(|data| data.get("nativeImageObservation"))
            {
                let observation: super::browser_native_image::NativeImageObservation =
                    serde_json::from_value(value.clone())
                        .map_err(|_| "browser_native_image_observation_invalid")?;
                observation.validate_structure()?;
                references.insert(
                    "nativeImageObservation".into(),
                    serde_json::to_string(&observation)
                        .map_err(|_| "browser_native_image_observation_invalid")?,
                );
            }
            if let Some(value) = response
                .data
                .as_ref()
                .and_then(|data| data.get("nativeObservation"))
            {
                let observation: super::browser_native_control::NativeChallengeObservation =
                    serde_json::from_value(value.clone())
                        .map_err(|_| "browser_native_original_observation_invalid")?;
                observation.validate_structure()?;
                references.insert(
                    "nativeObservation".into(),
                    serde_json::to_string(&observation)
                        .map_err(|_| "browser_native_original_observation_invalid")?,
                );
            }
        }
        if original.action == "native_input" && acknowledged {
            references.insert("nativeDelivery".into(), "dispatched".into());
        }
    }
    if original.action == "native_input" && references.contains_key("nativeImageRound") {
        let (state, round) = super::browser_native_image::finish_round(
            &references["nativeImageRound"],
            acknowledged,
            &original.request_id,
        )?;
        references.insert("nativeChallengeState".into(), state);
        references.insert("nativeImageRound".into(), round);
        references.insert(
            "nativeImageBudgetAccounting".into(),
            if acknowledged {
                "derived_batch_acknowledged"
            } else {
                "reserved_maximum_on_uncertain"
            }
            .into(),
        );
    } else if original.action == "native_input"
        && references.contains_key("nativeChallengePolicyDigest")
    {
        use agent_browser_challenge_control::{
            decide, ChallengeDecision, ChallengeEvent, ChallengePolicy, ChallengeSnapshot,
            ChallengeState, DeliveryState,
        };
        let provider = references
            .get("nativeProviderId")
            .cloned()
            .ok_or("browser_native_original_operation_invalid")?;
        let policy = ChallengePolicy {
            policy_digest: references
                .get("nativeChallengePolicyDigest")
                .cloned()
                .ok_or("browser_native_original_operation_invalid")?,
            deadline_at_ms: references
                .get("nativeChallengeDeadlineAtMs")
                .and_then(|value| value.parse().ok())
                .ok_or("browser_native_original_operation_invalid")?,
            allowed_provider_ids: vec![provider.clone()],
            max_attempts: 1,
            max_steps: 2,
            max_pointer_events: 3,
            max_key_events: 0,
            cooldown_ms: 0,
        };
        let snapshot = ChallengeSnapshot {
            state: ChallengeState::Solving,
            provider_id: Some(provider),
            attempts_started: 1,
            ..Default::default()
        };
        let decision = decide(
            &policy,
            &snapshot,
            ChallengeEvent::EffectFinished {
                delivery: if acknowledged {
                    DeliveryState::Acknowledged
                } else {
                    DeliveryState::Uncertain
                },
                // A lost/rejected reply does not establish emitted counts. The
                // durable task budget is consumed independently of these counts.
                steps: if acknowledged { 2 } else { 0 },
                pointer_events: if acknowledged { 3 } else { 0 },
                key_events: 0,
            },
        )
        .map_err(|_| "browser_native_original_operation_completion_invalid")?;
        let next = match decision {
            ChallengeDecision::AwaitVerification(snapshot)
            | ChallengeDecision::Terminal { snapshot, .. } => snapshot,
            _ => return Err("browser_native_original_operation_completion_invalid".into()),
        };
        references.insert(
            "nativeChallengeState".into(),
            serde_json::to_value(next.state)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .ok_or("browser_native_original_operation_completion_invalid")?,
        );
    }
    if original.action == "native_input"
        && references.contains_key("nativeObservationRequestId")
        && !references.contains_key("nativeChallengePolicyDigest")
    {
        references.insert(
            "nativeChallengeState".into(),
            "human_intervention_required".into(),
        );
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
        // Runtime admission can add an owner policy binding after the pending
        // marker was returned to MCP. Preserve that evidence on completion.
        current.evidence_references.extend(references);
        Ok(current.clone())
    })
}

fn image_attempt_selection(command: &Value) -> Result<Vec<String>, String> {
    super::browser_native_image::selection(command)
}

fn original_native_image_observation(
    state: &BrowserSessionState,
    task: &str,
    command: &Value,
) -> Result<super::browser_native_image::NativeImageObservation, String> {
    let selected = image_attempt_selection(command)?;
    let request = command["imageAttempt"]["observationRequestId"]
        .as_str()
        .ok_or("browser_native_original_observation_request_invalid")?;
    let history = operation_history(state, task)?;
    let capture = history
        .get(request)
        .ok_or("browser_native_original_observation_missing")?;
    let encoded = capture
        .evidence_references
        .get("nativeImageObservation")
        .filter(|v| v.len() <= 4096)
        .ok_or("browser_native_original_observation_unqualified")?;
    let observation: super::browser_native_image::NativeImageObservation =
        serde_json::from_str(encoded).map_err(|_| "browser_native_image_observation_invalid")?;
    if capture.action != "native_capture"
        || capture.state != BrowserTaskOperationState::OwnerAcknowledged
        || RetainedContext::from_operation(capture) != context(state, task)?
    {
        return Err("browser_native_original_observation_context_changed".into());
    }
    observation.validate_structure()?;
    if observation.registered_action_id != super::browser_native_image::command_profile(command) {
        return Err("browser_native_image_round_changed".into());
    }
    observation.actions(&selected)?;
    Ok(observation)
}

/// Reopen the exact pending image original immediately before current Native
/// owner admission. A caller envelope can neither replace its frame nor reset it.
pub(crate) fn native_image_attempt_observation(
    command: &Value,
) -> Result<super::browser_native_image::NativeImageObservation, String> {
    let task = command["sessionId"]
        .as_str()
        .ok_or("browser_native_original_operation_invalid")?;
    let request = command["id"]
        .as_str()
        .ok_or("browser_native_original_operation_invalid")?;
    let state = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path()?,
    )?
    .load_session_state()?;
    check_expected(&state, command)?;
    let original = state
        .task_operations
        .get(task)
        .and_then(|rows| rows.get(request))
        .ok_or("browser_native_original_operation_missing")?;
    if original.action != "native_input"
        || original.state != BrowserTaskOperationState::Pending
        || original
            .evidence_references
            .get("nativeRegisteredActionId")
            .map(String::as_str)
            != Some(super::browser_native_image::command_profile(command))
        || original
            .evidence_references
            .get("nativeObservationRequestId")
            .map(String::as_str)
            != command["imageAttempt"]["observationRequestId"].as_str()
        || RetainedContext::from_operation(original) != context(&state, task)?
    {
        return Err("browser_native_original_operation_context_changed".into());
    }
    let observation = original_native_image_observation(&state, task, command)?;
    if let Some(binding) = original.evidence_references.get("nativeSitePolicy") {
        let binding: Value =
            serde_json::from_str(binding).map_err(|_| "browser_native_site_policy_invalid")?;
        let current = crate::connection::stable_time::native_now()?;
        let plan = native_image_kernel_plan(&state, task, command, &binding, &current)?;
        let retained = original
            .evidence_references
            .get("nativeImageRound")
            .ok_or("browser_native_image_round_missing")?;
        super::browser_native_image::check_retained_plan(retained, &plan)?;
    }
    Ok(observation)
}

fn native_image_kernel_plan(
    state: &BrowserSessionState,
    task: &str,
    command: &Value,
    binding: &Value,
    qualified_time: &crate::connection::stable_time::Stamp,
) -> Result<super::browser_native_image::NativeImagePlan, String> {
    super::browser_host_policy::check_state(state, task, command)?;
    if binding["registeredActionId"] != super::browser_native_image::command_profile(command)
        || binding["family"] != "hcaptcha"
    {
        return Err("browser_native_site_policy_invalid".into());
    }
    image_attempt_budget(state, task, command)?;
    let observation = original_native_image_observation(state, task, command)?;
    observation.validate_at(&super::browser_native_image::NativeImageClockReading {
        wall_time_ms: qualified_time.earliest_ms(),
        elapsed: None,
        service_time: Some(qualified_time.clone()),
    })?;
    let selected = image_attempt_selection(command)?;
    if let Some(prior) = image_attempt_budget(state, task, command)? {
        let history = operation_history(state, task)?;
        let old_capture = history
            .get(&prior.evidence_references["nativeObservationRequestId"])
            .filter(|capture| {
                capture.action == "native_capture"
                    && capture.state == BrowserTaskOperationState::OwnerAcknowledged
                    && RetainedContext::from_operation(capture)
                        == RetainedContext::from_operation(&prior)
            })
            .ok_or("browser_native_original_observation_missing")?;
        let encoded = old_capture
            .evidence_references
            .get("nativeImageObservation")
            .ok_or("browser_native_original_observation_unqualified")?;
        let old =
            serde_json::from_str(encoded).map_err(|_| "browser_native_image_round_invalid")?;
        let mut prior_binding = prior
            .evidence_references
            .get("nativeSitePolicy")
            .and_then(|value| serde_json::from_str::<Value>(value).ok())
            .ok_or("browser_native_site_policy_changed")?;
        let mut current_binding = binding.clone();
        // Service revision is advisory evidence of the read, not challenge
        // authority. Preserve exact comparison of every other binding field.
        for value in [&mut prior_binding, &mut current_binding] {
            value
                .as_object_mut()
                .ok_or("browser_native_site_policy_invalid")?
                .remove("observedServiceRevision");
        }
        if prior_binding != current_binding {
            return Err("browser_native_site_policy_changed".into());
        }
        return super::browser_native_image::plan_following(
            task,
            &prior.request_id,
            &observation,
            &selected,
            binding,
            qualified_time.latest_ms(),
            (&prior.evidence_references["nativeImageRound"], &old),
        );
    }
    super::browser_native_image::plan(
        task,
        command["id"]
            .as_str()
            .ok_or("browser_native_original_operation_invalid")?,
        &observation,
        &selected,
        binding,
        qualified_time.latest_ms(),
    )
}

/// Admit at most one continuation of the immutable acknowledged first image row.
fn image_attempt_budget(
    state: &BrowserSessionState,
    task: &str,
    command: &Value,
) -> Result<Option<BrowserTaskOperation>, String> {
    let request = command["id"].as_str().unwrap_or_default();
    let rows = operation_history(state, task)?;
    let prior = rows
        .values()
        .filter(|row| {
            row.request_id != request
                && row.state != BrowserTaskOperationState::OwnerRefusedNoEffect
                && row.action == "native_input"
                && row
                    .evidence_references
                    .contains_key("nativeObservationRequestId")
        })
        .collect::<Vec<_>>();
    let original = command["imageAttempt"]["originalRequestId"].as_str();
    if prior.is_empty() && (original.is_none() || original == Some(request)) {
        return Ok(None);
    }
    if let ([old], Some(original)) = (prior.as_slice(), original) {
        if original != request
            && old.request_id == original
            && old.state == BrowserTaskOperationState::OwnerAcknowledged
            && old
                .evidence_references
                .get("nativeImageOriginalRequestId")
                .map(String::as_str)
                == Some(original)
            && old
                .evidence_references
                .get("nativeRegisteredActionId")
                .map(String::as_str)
                == Some(super::browser_native_image::ROUNDS_PROFILE_ID)
            && old.evidence_references.contains_key("nativeImageRound")
            && RetainedContext::from_operation(old) == context(state, task)?
        {
            return Ok(Some((**old).clone()));
        }
    }
    Err("browser_native_challenge_attempt_budget_exhausted:human assistance required".into())
}

fn login_target_request(command: &Value) -> Result<(), String> {
    let target = command["loginTarget"]
        .as_object()
        .filter(|o| o.len() == 3)
        .ok_or("browser_native_login_target_invalid")?;
    if command["action"] != "native_input"
        || ["actions", "nativeAttempt", "imageAttempt", "controlEpoch"]
            .iter()
            .any(|k| command.get(k).is_some())
        || !["observationRequestId", "episodeId"].iter().all(|k| {
            target
                .get(*k)
                .and_then(Value::as_str)
                .is_some_and(valid_request_id)
        })
        || !target
            .get("targetRef")
            .and_then(Value::as_str)
            .is_some_and(|s| {
                s.len() == 70
                    && s.starts_with("login:")
                    && s[6..].bytes().all(|b| b.is_ascii_hexdigit())
            })
    {
        return Err("browser_native_login_target_invalid".into());
    }
    Ok(())
}
fn original_login_observation(
    state: &BrowserSessionState,
    task: &str,
    command: &Value,
) -> Result<Value, String> {
    login_target_request(command)?;
    let history = operation_history(state, task)?;
    let capture = history
        .get(
            command["loginTarget"]["observationRequestId"]
                .as_str()
                .unwrap(),
        )
        .ok_or("browser_native_login_observation_missing")?;
    if capture.action != "native_capture"
        || capture.state != BrowserTaskOperationState::OwnerAcknowledged
        || RetainedContext::from_operation(capture) != context(state, task)?
    {
        return Err("browser_native_login_context_changed".into());
    }
    let encoded = capture
        .evidence_references
        .get("nativeLoginObservation")
        .filter(|v| v.len() <= 32768)
        .ok_or("browser_native_login_observation_unqualified")?;
    let observation: Value =
        serde_json::from_str(encoded).map_err(|_| "browser_native_login_observation_invalid")?;
    super::native_login_perception::select_target(
        &observation,
        command["loginTarget"]["targetRef"].as_str().unwrap(),
        observation["binding"]["serviceTime"]
            .as_object()
            .and_then(|_| {
                serde_json::from_value::<crate::connection::stable_time::Stamp>(
                    observation["binding"]["serviceTime"].clone(),
                )
                .ok()
            })
            .map(|stamp| stamp.earliest_ms())
            .or_else(|| observation["binding"]["observedAtMs"].as_u64())
            .ok_or("browser_native_login_observation_invalid")?,
    )?;
    Ok(observation)
}
/// An episode is a scoped progress sequence, not a permanent task-wide veto.
/// New episodes require all previous task input settled; every new step still
/// binds a fresh owner observation and exact current task/process context.
fn login_episode_step(
    state: &BrowserSessionState,
    task: &str,
    command: &Value,
    current: Option<&str>,
    qualified_time: Option<&crate::connection::stable_time::Stamp>,
) -> Result<usize, String> {
    let episode = command["loginTarget"]["episodeId"]
        .as_str()
        .ok_or("browser_native_login_episode_invalid")?;
    let history = operation_history(state, task)?;
    let task_context = context(state, task)?;
    let mut prior = Vec::new();
    for row in history.values() {
        if row.state == BrowserTaskOperationState::OwnerRefusedNoEffect {
            continue;
        }
        let Some(id) = row.evidence_references.get("nativeLoginEpisode") else {
            continue;
        };
        if current == Some(row.request_id.as_str()) {
            continue;
        }
        if row.state != BrowserTaskOperationState::OwnerAcknowledged {
            return Err(format!("browser_native_login_previous_input_unsettled: requestId={}; inspect browser_command action=task_operation params.requestId for this original and reconcile before continuing any login episode; no input replay", row.request_id));
        }
        if id == episode {
            if RetainedContext::from_operation(row) != task_context {
                return Err("browser_native_login_context_changed: inspect browser_command action=task_briefing and original task_operation records; after settling prior inputs, begin a new scoped episode with a fresh native_capture".into());
            }
            prior.push(row);
        }
    }
    prior.sort_by_key(|row| {
        row.evidence_references
            .get("nativeLoginStep")
            .and_then(|step| step.parse::<usize>().ok())
            .unwrap_or(usize::MAX)
    });
    for (step, row) in prior.iter().enumerate() {
        if row.evidence_references.get("nativeLoginStep") != Some(&step.to_string())
            || !matches!(
                row.evidence_references
                    .get("nativeLoginKind")
                    .map(String::as_str),
                Some("username" | "credential_entry" | "submit")
            )
        {
            return Err("browser_native_login_previous_input_unsettled: inspect original task_operation records before explicit episode continuation; no input replay".into());
        }
    }
    let store = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path()?,
    )?;
    let maximum = store.login_step_limit(task)?;
    if prior.len() >= maximum {
        return Err(format!("browser_native_login_budget_exhausted: completedSteps={};maximumLoginSteps={maximum}; inspect browser_command action=task_operation params={{\"operation\":\"capacity\"}} and increase policy.maximumLoginSteps for this task before continuing the same episode; acknowledged originals are retained", prior.len()));
    }
    let observed = original_login_observation(state, task, command)?;
    if let Some(current) = qualified_time {
        let now = super::native_login_perception::qualified_time(&observed, current)?;
        // Shape/order checks above remain under the original custody boundary;
        // the caller supplies a single service observation acquired outside it.
        super::native_login_perception::select_target(
            &observed,
            command["loginTarget"]["targetRef"].as_str().unwrap(),
            now,
        )?;
    }
    Ok(prior.len())
}
pub(crate) fn native_login_attempt_observation(command: &Value) -> Result<Value, String> {
    let task = command["sessionId"]
        .as_str()
        .ok_or("browser_native_login_original_invalid")?;
    let request = command["id"]
        .as_str()
        .ok_or("browser_native_login_original_invalid")?;
    let state = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path()?,
    )?
    .load_session_state()?;
    check_expected(&state, command)?;
    let original = state
        .task_operations
        .get(task)
        .and_then(|rows| rows.get(request))
        .ok_or("browser_native_login_original_missing")?;
    if original.action != "native_input"
        || original.state != BrowserTaskOperationState::Pending
        || RetainedContext::from_operation(original) != context(&state, task)?
        || original
            .evidence_references
            .get("nativeLoginEpisode")
            .map(String::as_str)
            != command["loginTarget"]["episodeId"].as_str()
        || original
            .evidence_references
            .get("nativeLoginTargetRef")
            .map(String::as_str)
            != command["loginTarget"]["targetRef"].as_str()
        || original
            .evidence_references
            .get("nativeLoginObservationRequestId")
            .map(String::as_str)
            != command["loginTarget"]["observationRequestId"].as_str()
    {
        return Err("browser_native_login_original_context_changed".into());
    }
    let step = login_episode_step(&state, task, command, Some(request), None)?;
    if original.evidence_references.get("nativeLoginStep") != Some(&step.to_string()) {
        return Err("browser_native_login_step_invalid".into());
    }
    original_login_observation(&state, task, command)
}
