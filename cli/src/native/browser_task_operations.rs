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
    action == "native_capture" && params.get("registeredActionId").is_some()
        || action == "native_input" && params.get("nativeAttempt").is_some()
}

pub(crate) fn begin_native(task_id: &str, command: &mut Value) -> Result<Admission, String> {
    if !eligible_native(command["action"].as_str().unwrap_or_default(), command) {
        return Err("browser_native_original_operation_invalid".into());
    }
    let action_id = if command["action"] == "native_capture" {
        if command["classifyChallenges"] != true {
            return Err("browser_native_registered_action_requires_classification".into());
        }
        command["registeredActionId"].as_str()
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
    if action_id
        .and_then(agent_browser_challenge_control::challenge_profile)
        .is_none()
    {
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
    // Resolve an original capture before transport or process recovery. Missing
    // evidence is never replaced by a caller timestamp or guessed coordinates.
    let observation = if native && action == "native_input" {
        Some(original_native_observation(&state, task_id, command)?)
    } else {
        None
    };
    if observation.is_some()
        && state.task_operations.get(task_id).is_some_and(|rows| {
            rows.values().any(|row| {
                row.action == "native_input"
                    && row
                        .evidence_references
                        .contains_key("nativeObservationRequestId")
            })
        })
    {
        return Err(
            "browser_native_challenge_attempt_budget_exhausted:human assistance required".into(),
        );
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
        evidence_references: if observation.is_some() {
            BTreeMap::from([
                (
                    "nativeObservationRequestId".into(),
                    command["nativeAttempt"]["observationRequestId"]
                        .as_str()
                        .unwrap()
                        .into(),
                ),
                (
                    "nativeRegisteredActionId".into(),
                    command["nativeAttempt"]["registeredActionId"]
                        .as_str()
                        .unwrap()
                        .into(),
                ),
            ])
        } else {
            BTreeMap::new()
        },
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
        let admitted = original.clone();
        if observation.is_some() {
            original_native_observation(state, task_id, command)?;
            super::browser_host_policy::check_state(state, task_id, command)?;
            if state.task_operations.get(task_id).is_some_and(|rows| {
                rows.values().any(|row| {
                    row.action == "native_input"
                        && row
                            .evidence_references
                            .contains_key("nativeObservationRequestId")
                })
            }) {
                return Err(
                    "browser_native_challenge_attempt_budget_exhausted:human assistance required"
                        .into(),
                );
            }
        }
        let rows = state.task_operations.entry(task_id.into()).or_default();
        if rows.len() >= MAXIMUM_TASK_OPERATIONS {
            return Err(
                "browser_task_operation_capacity_reached: original history retained".into(),
            );
        }
        rows.insert(request_id.clone(), admitted.clone());
        Ok(Admission::Dispatch(admitted))
    })?;
    if let Admission::Dispatch(ref original) = admitted {
        command["taskOperationExpected"] =
            json!({"requestId":original.request_id,"context":expected});
        command["retainedTaskOnly"] = json!(true);
        command["noTransportReplay"] = json!(true);
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
    let prior = state.task_operations.get(task_id).is_some_and(|rows| {
        rows.values().any(|row| {
            row.request_id != command["id"]
                && row.action == "native_input"
                && row
                    .evidence_references
                    .contains_key("nativeObservationRequestId")
        })
    });
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
        deadline_at_ms: observation.observed_at_ms.saturating_add(10_000),
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
            now_ms: now_ms()?,
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
            ChallengeEvent::StartAttempt { now_ms: now_ms()? }
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
    let capture = state
        .task_operations
        .get(task_id)
        .and_then(|rows| rows.get(request))
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
    observation.validate(now_ms()?)?;
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
    native_attempt_observation(command)?;
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
    let observation = native_attempt_observation(command)?;
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
                && provider.config_ref.as_deref() == Some("agent-browser:remote-view-native:v1")
                && decision.challenge_provider_ids.contains(&provider.id)
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
pub(crate) fn retain_native_site_policy(command: &Value, binding: &Value) -> Result<(), String> {
    let task = command["sessionId"]
        .as_str()
        .ok_or("browser_native_original_operation_invalid")?;
    let request = command["id"]
        .as_str()
        .ok_or("browser_native_original_operation_invalid")?;
    BrowserSessionSqliteStore::default_sqlite()?.mutate_session_state(|state| {
        check_expected(state, command)?;
        let observation = original_native_observation(state, task, command)?;
        if let Some(prior) = state
            .task_operations
            .get(task)
            .and_then(|rows| rows.get(request))
            .and_then(|row| row.evidence_references.get("nativeSitePolicy"))
        {
            let prior: Value =
                serde_json::from_str(prior).map_err(|_| "browser_native_site_policy_invalid")?;
            return if prior["digest"] == binding["digest"] {
                Ok(())
            } else {
                Err("browser_native_site_policy_changed".into())
            };
        }
        let policy = native_kernel_admit(state, task, command, &observation, binding)?;
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
        refs.insert("nativeChallengeFamily".into(), observation.family);
        refs.insert("nativeChallengeState".into(), "solving".into());
        refs.insert("nativeChallengePolicyDigest".into(), policy.policy_digest);
        refs.insert(
            "nativeChallengeDeadlineAtMs".into(),
            policy.deadline_at_ms.to_string(),
        );
        Ok(())
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
        if original.action == "native_capture" {
            if let Some(value) = response
                .data
                .as_ref()
                .and_then(|data| data.get("nativeObservation"))
            {
                let observation: super::browser_native_control::NativeChallengeObservation =
                    serde_json::from_value(value.clone())
                        .map_err(|_| "browser_native_original_observation_invalid")?;
                observation.validate(now_ms()?)?;
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
    if original.action == "native_input" && references.contains_key("nativeChallengePolicyDigest") {
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
