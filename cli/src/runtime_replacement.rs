//! Plans and executes one policy-fenced workstation runtime replacement.
//!
//! Callers select only the replacement policy and, for effects, provide the
//! digest of a current read-only plan. Process, browser, session, profile,
//! route, and signal selection remain inside this module.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::native::service_profile_access_policy::{ProfileIdentityAssurance, ProfilePermission};
use crate::process_identity::RecordedProcessIdentity;
use crate::process_identity::{VerifiedProcessSignal, VerifiedProcessTermination};
use crate::runtime_adoption::{RuntimeClassification, StableRuntimeCensus};
use crate::runtime_host_ingress::RuntimeHostBackend;
use crate::runtime_host_ingress::RuntimeHostTopology;

const PLAN_SCHEMA_VERSION: &str = "agent-browser.runtime-replacement-plan.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RuntimeReplacementPolicy {
    Preserve,
    FullShutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeReplacementDisposition {
    PreserveContinuity,
    ReadyForFullShutdown,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeReplacementBlocker {
    pub(crate) code: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeReplacementBrowser {
    pub(crate) logical_browser_id: String,
    pub(crate) session_names: Vec<String>,
    pub(crate) profile_identity_digest: String,
    pub(crate) classification: RuntimeClassification,
    pub(crate) process_identity: Option<RecordedProcessIdentity>,
    pub(crate) close_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeReplacementPlan {
    pub(crate) schema_version: String,
    pub(crate) plan_digest: String,
    pub(crate) policy: RuntimeReplacementPolicy,
    pub(crate) disposition: RuntimeReplacementDisposition,
    pub(crate) blockers: Vec<RuntimeReplacementBlocker>,
    pub(crate) ingress_revision: u64,
    pub(crate) selected_backend: RuntimeHostBackend,
    pub(crate) selected_process_identity: Option<RecordedProcessIdentity>,
    pub(crate) census_digest: String,
    pub(crate) browsers: Vec<RuntimeReplacementBrowser>,
    pub(crate) profiles_preserved: bool,
    pub(crate) live_state_will_end: bool,
}

const UPGRADE_SUCCESSOR_PLAN_KEY: &str = "runtimeReplacementPlan";
const UPGRADE_SUCCESSOR_AUTHORIZATION_KEY: &str = "runtimeReplacementAuthorization";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeReplacementAuthorization {
    pub(crate) schema_version: String,
    pub(crate) authorization_id: String,
    pub(crate) subject_id: String,
    pub(crate) assurance: ProfileIdentityAssurance,
    pub(crate) permissions: Vec<ProfilePermission>,
    pub(crate) plan_digest: String,
    pub(crate) target_browser_ids: Vec<String>,
    pub(crate) issued_at: String,
}

/// Compile explicit operator intent into a plan-bound full-shutdown
/// authorization. The reviewed plan remains the sole source of physical
/// targets; caller-supplied browser or process identities are never accepted.
pub(crate) fn authorize_reviewed_full_shutdown(
    plan: &RuntimeReplacementPlan,
    expected_plan_digest: &str,
    subject_id: &str,
    assurance: ProfileIdentityAssurance,
    permissions: &[ProfilePermission],
    issued_at: &str,
) -> Result<RuntimeReplacementAuthorization, String> {
    if plan.policy != RuntimeReplacementPolicy::FullShutdown
        || plan.disposition != RuntimeReplacementDisposition::ReadyForFullShutdown
        || plan.plan_digest != expected_plan_digest
    {
        return Err("runtime_replacement_authorization_plan_mismatch".to_string());
    }
    if subject_id.trim().is_empty()
        || !assurance.satisfies(ProfileIdentityAssurance::Operator)
        || !permissions.contains(&ProfilePermission::LifecycleManage)
        || !permissions.contains(&ProfilePermission::FullShutdown)
    {
        return Err("runtime_replacement_operator_authorization_required".to_string());
    }
    chrono::DateTime::parse_from_rfc3339(issued_at)
        .map_err(|_| "runtime_replacement_authorization_time_invalid".to_string())?;
    let mut target_browser_ids = plan
        .browsers
        .iter()
        .filter(|browser| browser.close_required)
        .map(|browser| browser.logical_browser_id.clone())
        .collect::<Vec<_>>();
    target_browser_ids.sort();
    target_browser_ids.dedup();
    let mut authorized_permissions = permissions.to_vec();
    authorized_permissions.sort();
    authorized_permissions.dedup();
    let payload = format!(
        "agent-browser.runtime-replacement-authorization.v1\n{subject_id}\n{assurance:?}\n{expected_plan_digest}\n{target_browser_ids:?}\n{issued_at}"
    );
    Ok(RuntimeReplacementAuthorization {
        schema_version: "agent-browser.runtime-replacement-authorization.v1".to_string(),
        authorization_id: format!(
            "runtime-replacement-authorization:{:x}",
            Sha256::digest(payload)
        ),
        subject_id: subject_id.to_string(),
        assurance,
        permissions: authorized_permissions,
        plan_digest: expected_plan_digest.to_string(),
        target_browser_ids,
        issued_at: issued_at.to_string(),
    })
}

/// Bind the complete reviewed replacement plan to the workstation transaction.
/// Rebinding is idempotent only for the byte-equivalent plan and therefore
/// prevents policy or target changes during resume.
pub(crate) fn bind_upgrade_transaction(
    transaction: &mut crate::runtime_adoption::UpgradeTransaction,
    plan: &RuntimeReplacementPlan,
) -> Result<(), String> {
    let value = serde_json::to_value(plan)
        .map_err(|error| format!("runtime_replacement_plan_serialize_failed:{error}"))?;
    if let Some(existing) = transaction.successor_fields.get(UPGRADE_SUCCESSOR_PLAN_KEY) {
        if existing == &value {
            return Ok(());
        }
        return Err("runtime_replacement_transaction_plan_changed".to_string());
    }
    transaction
        .successor_fields
        .insert(UPGRADE_SUCCESSOR_PLAN_KEY.to_string(), value);
    Ok(())
}

pub(crate) fn bind_full_shutdown_authorization(
    transaction: &mut crate::runtime_adoption::UpgradeTransaction,
    plan: &RuntimeReplacementPlan,
    authorization: &RuntimeReplacementAuthorization,
) -> Result<(), String> {
    validate_full_shutdown_authorization(plan, authorization)?;
    let value = serde_json::to_value(authorization)
        .map_err(|error| format!("runtime_replacement_authorization_serialize_failed:{error}"))?;
    if let Some(existing) = transaction
        .successor_fields
        .get(UPGRADE_SUCCESSOR_AUTHORIZATION_KEY)
    {
        if existing == &value {
            return Ok(());
        }
        return Err("runtime_replacement_transaction_authorization_changed".to_string());
    }
    transaction
        .successor_fields
        .insert(UPGRADE_SUCCESSOR_AUTHORIZATION_KEY.to_string(), value);
    Ok(())
}

fn authorization_from_upgrade_transaction(
    transaction: &crate::runtime_adoption::UpgradeTransaction,
) -> Result<RuntimeReplacementAuthorization, String> {
    transaction
        .successor_fields
        .get(UPGRADE_SUCCESSOR_AUTHORIZATION_KEY)
        .cloned()
        .ok_or_else(|| "runtime_replacement_authorization_missing".to_string())
        .and_then(|value| {
            serde_json::from_value(value)
                .map_err(|error| format!("runtime_replacement_authorization_invalid:{error}"))
        })
}

fn validate_full_shutdown_authorization(
    plan: &RuntimeReplacementPlan,
    authorization: &RuntimeReplacementAuthorization,
) -> Result<(), String> {
    let mut exact_targets = plan
        .browsers
        .iter()
        .filter(|browser| browser.close_required)
        .map(|browser| browser.logical_browser_id.clone())
        .collect::<Vec<_>>();
    exact_targets.sort();
    exact_targets.dedup();
    if authorization.schema_version != "agent-browser.runtime-replacement-authorization.v1"
        || authorization.authorization_id.trim().is_empty()
        || authorization.subject_id.trim().is_empty()
        || !authorization
            .assurance
            .satisfies(ProfileIdentityAssurance::Operator)
        || !authorization
            .permissions
            .contains(&ProfilePermission::LifecycleManage)
        || !authorization
            .permissions
            .contains(&ProfilePermission::FullShutdown)
        || authorization.plan_digest != plan.plan_digest
        || authorization.target_browser_ids != exact_targets
    {
        return Err("runtime_replacement_authorization_mismatch".to_string());
    }
    Ok(())
}

pub(crate) fn plan_from_upgrade_transaction(
    transaction: &crate::runtime_adoption::UpgradeTransaction,
) -> Result<Option<RuntimeReplacementPlan>, String> {
    transaction
        .successor_fields
        .get(UPGRADE_SUCCESSOR_PLAN_KEY)
        .cloned()
        .map(|value| {
            serde_json::from_value(value)
                .map_err(|error| format!("runtime_replacement_transaction_plan_invalid:{error}"))
        })
        .transpose()
}

pub(crate) fn effect_receipt_from_upgrade_transaction(
    transaction: &crate::runtime_adoption::UpgradeTransaction,
) -> Result<Option<RuntimeReplacementEffectReceipt>, String> {
    transaction
        .successor_fields
        .get(UPGRADE_SUCCESSOR_EFFECT_RECEIPT_KEY)
        .cloned()
        .map(|value| {
            serde_json::from_value(value)
                .map_err(|error| format!("runtime_replacement_effect_receipt_invalid:{error}"))
        })
        .transpose()
}

pub(crate) fn requires_forward_recovery(
    transaction: &crate::runtime_adoption::UpgradeTransaction,
) -> Result<bool, String> {
    Ok(effect_receipt_from_upgrade_transaction(transaction)?
        .is_some_and(|receipt| receipt.state != RuntimeReplacementEffectState::Planned))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeReplacementEffectState {
    Planned,
    BrowsersClosing,
    BrowsersClosed,
    SourceRetiring,
    SourceAbsent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeReplacementEffectReceipt {
    pub(crate) schema_version: String,
    pub(crate) state: RuntimeReplacementEffectState,
    pub(crate) plan_digest: String,
    pub(crate) closed_sessions: Vec<String>,
    #[serde(default)]
    pub(crate) forced_browser_ids: Vec<String>,
    pub(crate) final_census_digest: Option<String>,
    pub(crate) source_exit_proven: bool,
    pub(crate) profiles_preserved: bool,
}

trait RuntimeReplacementEffects {
    fn checkpoint(&mut self, receipt: &RuntimeReplacementEffectReceipt) -> Result<(), String>;
    fn close_session(&mut self, session_name: &str) -> Result<(), String>;
    fn browser_is_running(&mut self, identity: &RecordedProcessIdentity) -> Result<bool, String>;
    fn force_close_browser(&mut self, identity: &RecordedProcessIdentity) -> Result<(), String>;
    fn collect_stable_census(&mut self) -> Result<StableRuntimeCensus, String>;
    fn source_is_running(&mut self) -> Result<bool, String>;
    fn retire_source(&mut self) -> Result<(), String>;
}

const UPGRADE_SUCCESSOR_EFFECT_RECEIPT_KEY: &str = "runtimeReplacementEffectReceipt";
const CLOSE_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const SOURCE_EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const SOURCE_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(25);

struct LiveRuntimeReplacementEffects<'a> {
    transaction_path: &'a Path,
    transaction: &'a mut crate::runtime_adoption::UpgradeTransaction,
    source_binary: PathBuf,
    source_socket_dir: PathBuf,
    source_process: Option<VerifiedProcessTermination>,
}

impl RuntimeReplacementEffects for LiveRuntimeReplacementEffects<'_> {
    fn checkpoint(&mut self, receipt: &RuntimeReplacementEffectReceipt) -> Result<(), String> {
        self.transaction.successor_fields.insert(
            UPGRADE_SUCCESSOR_EFFECT_RECEIPT_KEY.to_string(),
            serde_json::to_value(receipt).map_err(|error| {
                format!("runtime_replacement_effect_receipt_serialize_failed:{error}")
            })?,
        );
        write_private_json_atomic(self.transaction_path, self.transaction)
    }

    fn close_session(&mut self, session_name: &str) -> Result<(), String> {
        if !crate::validation::is_valid_session_name(session_name) {
            return Err(crate::validation::session_name_error(session_name));
        }
        let mut child = Command::new(&self.source_binary)
            .args(["--json", "--session", session_name, "close"])
            .env(crate::runtime_host::RUNTIME_HOST_ENV, "1")
            .env("AGENT_BROWSER_SOCKET_DIR", &self.source_socket_dir)
            .env(
                crate::runtime_adoption::RUNTIME_ADMISSION_TRANSACTION_ID_ENV,
                &self.transaction.transaction_id,
            )
            .env(
                crate::runtime_adoption::RUNTIME_ADMISSION_TRANSACTION_REVISION_ENV,
                self.transaction.revision.to_string(),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                format!("runtime_replacement_close_command_failed:{session_name}:{error}")
            })?;
        let deadline = Instant::now() + CLOSE_COMMAND_TIMEOUT;
        loop {
            if child
                .try_wait()
                .map_err(|error| {
                    format!("runtime_replacement_close_wait_failed:{session_name}:{error}")
                })?
                .is_some()
            {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "runtime_replacement_close_command_timeout:{session_name}"
                ));
            }
            std::thread::sleep(SOURCE_EXIT_POLL_INTERVAL);
        }
        let output = child.wait_with_output().map_err(|error| {
            format!("runtime_replacement_close_output_failed:{session_name}:{error}")
        })?;
        let payload: serde_json::Value =
            serde_json::from_slice(&output.stdout).map_err(|error| {
                format!("runtime_replacement_close_response_invalid:{session_name}:{error}")
            })?;
        if !output.status.success()
            || payload.get("success").and_then(|value| value.as_bool()) != Some(true)
        {
            let diagnostic = payload
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or("close command returned failure");
            return Err(format!(
                "runtime_replacement_close_refused:{session_name}:{diagnostic}"
            ));
        }
        Ok(())
    }

    fn browser_is_running(&mut self, identity: &RecordedProcessIdentity) -> Result<bool, String> {
        VerifiedProcessTermination::open(identity)?
            .map(|process| process.is_running())
            .transpose()
            .map(|running| running.unwrap_or(false))
    }

    fn force_close_browser(&mut self, identity: &RecordedProcessIdentity) -> Result<(), String> {
        let Some(process) = VerifiedProcessTermination::open(identity)? else {
            return Ok(());
        };
        process.signal(VerifiedProcessSignal::Terminate)?;
        if wait_for_source_exit(&process)? {
            return Ok(());
        }
        process.signal(VerifiedProcessSignal::Kill)?;
        if wait_for_source_exit(&process)? {
            Ok(())
        } else {
            Err(format!(
                "runtime_replacement_browser_exit_timeout:{}",
                identity.pid
            ))
        }
    }

    fn collect_stable_census(&mut self) -> Result<StableRuntimeCensus, String> {
        crate::workstation_install::collect_stable_host_runtime_census()
    }

    fn source_is_running(&mut self) -> Result<bool, String> {
        self.source_process
            .as_ref()
            .map(VerifiedProcessTermination::is_running)
            .transpose()
            .map(|running| running.unwrap_or(false))
    }

    fn retire_source(&mut self) -> Result<(), String> {
        let Some(process) = self.source_process.as_ref() else {
            return Ok(());
        };
        if !process.is_running()? {
            return Ok(());
        }
        process.signal(VerifiedProcessSignal::Terminate)?;
        if wait_for_source_exit(process)? {
            return Ok(());
        }
        process.signal(VerifiedProcessSignal::Kill)?;
        if wait_for_source_exit(process)? {
            Ok(())
        } else {
            Err("runtime_replacement_source_exit_timeout".to_string())
        }
    }
}

/// Execute the exact reviewed full-shutdown effects inside an already drained
/// workstation upgrade transaction. Every checkpoint is persisted in that
/// transaction before the next irreversible edge.
pub(crate) fn execute_full_shutdown(
    transaction_path: &Path,
    transaction: &mut crate::runtime_adoption::UpgradeTransaction,
    plan: &RuntimeReplacementPlan,
) -> Result<RuntimeReplacementEffectReceipt, String> {
    bind_upgrade_transaction(transaction, plan)?;
    let authorization = authorization_from_upgrade_transaction(transaction)?;
    validate_full_shutdown_authorization(plan, &authorization)?;
    let identity = plan
        .selected_process_identity
        .as_ref()
        .ok_or_else(|| "runtime_replacement_selected_process_identity_missing".to_string())?;
    let source_binary = identity
        .executable_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "runtime_replacement_source_executable_missing".to_string())?;
    let source_process = VerifiedProcessTermination::open(identity)?;
    let existing = effect_receipt_from_upgrade_transaction(transaction)?;
    let mut effects = LiveRuntimeReplacementEffects {
        transaction_path,
        transaction,
        source_binary,
        source_socket_dir: plan.selected_backend.socket_dir.clone(),
        source_process,
    };
    execute_full_shutdown_with(plan, &authorization, existing, &mut effects)
}

fn wait_for_source_exit(process: &VerifiedProcessTermination) -> Result<bool, String> {
    let deadline = Instant::now() + SOURCE_EXIT_TIMEOUT;
    while Instant::now() < deadline {
        if !process.is_running()? {
            return Ok(true);
        }
        std::thread::sleep(SOURCE_EXIT_POLL_INTERVAL);
    }
    Ok(!process.is_running()?)
}

fn write_private_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "runtime_replacement_transaction_parent_missing".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "runtime_replacement_transaction_directory_failed:{}:{error}",
            parent.display()
        )
    })?;
    let staged = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let body = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("runtime_replacement_transaction_serialize_failed:{error}"))?;
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staged)
            .map_err(|error| format!("runtime_replacement_transaction_stage_failed:{error}"))?;
        file.write_all(&body)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("runtime_replacement_transaction_write_failed:{error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&staged, fs::Permissions::from_mode(0o600)).map_err(|error| {
                format!("runtime_replacement_transaction_permissions_failed:{error}")
            })?;
        }
        fs::rename(&staged, path)
            .map_err(|error| format!("runtime_replacement_transaction_commit_failed:{error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    result
}

fn execute_full_shutdown_with(
    plan: &RuntimeReplacementPlan,
    authorization: &RuntimeReplacementAuthorization,
    existing: Option<RuntimeReplacementEffectReceipt>,
    effects: &mut impl RuntimeReplacementEffects,
) -> Result<RuntimeReplacementEffectReceipt, String> {
    validate_full_shutdown_authorization(plan, authorization)?;
    if plan.policy != RuntimeReplacementPolicy::FullShutdown
        || plan.disposition != RuntimeReplacementDisposition::ReadyForFullShutdown
        || !plan.profiles_preserved
    {
        return Err("runtime_replacement_full_shutdown_plan_not_ready".to_string());
    }
    let mut sessions = plan
        .browsers
        .iter()
        .filter(|browser| browser.close_required)
        .flat_map(|browser| browser.session_names.iter().cloned())
        .collect::<Vec<_>>();
    sessions.sort();
    sessions.dedup();
    if plan.live_state_will_end && sessions.is_empty() {
        return Err("runtime_replacement_close_session_missing".to_string());
    }

    let mut receipt = if let Some(existing) = existing {
        if existing.plan_digest != plan.plan_digest || !existing.profiles_preserved {
            return Err("runtime_replacement_effect_receipt_plan_changed".to_string());
        }
        existing
    } else {
        let receipt = RuntimeReplacementEffectReceipt {
            schema_version: "agent-browser.runtime-replacement-effect-receipt.v1".to_string(),
            state: RuntimeReplacementEffectState::Planned,
            plan_digest: plan.plan_digest.clone(),
            closed_sessions: Vec::new(),
            forced_browser_ids: Vec::new(),
            final_census_digest: None,
            source_exit_proven: false,
            profiles_preserved: true,
        };
        effects.checkpoint(&receipt)?;
        receipt
    };
    if receipt.state == RuntimeReplacementEffectState::SourceAbsent {
        if effects.source_is_running()? {
            return Err("runtime_replacement_source_reappeared_after_receipt".to_string());
        }
        let replay_census = effects.collect_stable_census()?;
        if !browserless_after_full_shutdown(&replay_census) {
            return Err("runtime_replacement_replay_census_not_browserless".to_string());
        }
        if receipt.final_census_digest.as_deref() != Some(replay_census.digest.as_str()) {
            receipt.final_census_digest = Some(replay_census.digest);
            effects.checkpoint(&receipt)?;
        }
        return Ok(receipt);
    }
    if matches!(
        receipt.state,
        RuntimeReplacementEffectState::Planned | RuntimeReplacementEffectState::BrowsersClosing
    ) {
        receipt.state = RuntimeReplacementEffectState::BrowsersClosing;
        effects.checkpoint(&receipt)?;
    }
    for browser in plan
        .browsers
        .iter()
        .filter(|browser| browser.close_required)
    {
        if browser
            .session_names
            .iter()
            .all(|session| receipt.closed_sessions.contains(session))
        {
            continue;
        }
        let identity = browser.process_identity.as_ref().ok_or_else(|| {
            format!(
                "runtime_replacement_browser_process_identity_missing:{}",
                browser.logical_browser_id
            )
        })?;
        let cooperative_close = browser.classification
            == RuntimeClassification::CooperativeLiveOwner
            && browser
                .session_names
                .iter()
                .filter(|session| !receipt.closed_sessions.contains(session))
                .all(|session| effects.close_session(session).is_ok());
        if !cooperative_close || effects.browser_is_running(identity)? {
            effects.force_close_browser(identity)?;
            if !receipt
                .forced_browser_ids
                .contains(&browser.logical_browser_id)
            {
                receipt
                    .forced_browser_ids
                    .push(browser.logical_browser_id.clone());
                receipt.forced_browser_ids.sort();
            }
        }
        if effects.browser_is_running(identity)? {
            return Err(format!(
                "runtime_replacement_browser_exit_unproven:{}",
                browser.logical_browser_id
            ));
        }
        for session in &browser.session_names {
            if !receipt.closed_sessions.contains(session) {
                receipt.closed_sessions.push(session.clone());
            }
        }
        receipt.closed_sessions.sort();
        receipt.closed_sessions.dedup();
        effects.checkpoint(&receipt)?;
    }

    let pre_retirement_census = effects.collect_stable_census()?;
    if !browserless_after_full_shutdown(&pre_retirement_census) {
        return Err("runtime_replacement_post_close_census_not_browserless".to_string());
    }
    receipt.state = RuntimeReplacementEffectState::BrowsersClosed;
    effects.checkpoint(&receipt)?;

    if effects.source_is_running()? {
        receipt.state = RuntimeReplacementEffectState::SourceRetiring;
        effects.checkpoint(&receipt)?;
        effects.retire_source()?;
    }
    if effects.source_is_running()? {
        return Err("runtime_replacement_source_exit_unproven".to_string());
    }
    let final_census = effects.collect_stable_census()?;
    if !browserless_after_full_shutdown(&final_census) {
        return Err("runtime_replacement_post_retirement_census_not_browserless".to_string());
    }
    receipt.final_census_digest = Some(final_census.digest);
    receipt.source_exit_proven = true;
    receipt.state = RuntimeReplacementEffectState::SourceAbsent;
    effects.checkpoint(&receipt)?;
    Ok(receipt)
}

fn browserless_after_full_shutdown(census: &StableRuntimeCensus) -> bool {
    census.activation_allowed
        && census.records.iter().all(|record| {
            matches!(
                record.classification,
                RuntimeClassification::IdleDaemon
                    | RuntimeClassification::StaleMetadata
                    | RuntimeClassification::ExternalObserved
                    | RuntimeClassification::ManualPreserveOnly
            )
        })
}

/// Observe the selected runtime and return a deterministic, effect-free
/// replacement plan. The browserless takeover planner supplies the exact host
/// and process identity evidence, but its expected live-browser blocker is
/// replaced by this module's explicit close inventory.
pub(crate) fn plan_runtime_replacement(
    policy: RuntimeReplacementPolicy,
) -> Result<RuntimeReplacementPlan, String> {
    let takeover = crate::runtime_host_supervisor_takeover::plan_supervisor_takeover()?;
    let browser_process_identities = managed_browser_process_identities()?;
    let inherited = takeover
        .blockers
        .iter()
        .filter(|blocker| replacement_must_preserve_takeover_blocker(&blocker.code))
        .map(|blocker| RuntimeReplacementBlocker {
            code: blocker.code.clone(),
            message: blocker.message.clone(),
        })
        .collect::<Vec<_>>();
    let mut plan = build_runtime_replacement_plan(
        policy,
        takeover.ingress_revision,
        takeover.selected_backend,
        takeover.selected_process_identity,
        takeover.census,
        &browser_process_identities,
    )?;
    plan.blockers.extend(inherited);
    plan.blockers.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.message.cmp(&right.message))
    });
    plan.blockers.dedup();
    if policy == RuntimeReplacementPolicy::FullShutdown && !plan.blockers.is_empty() {
        plan.disposition = RuntimeReplacementDisposition::Blocked;
    }
    plan.plan_digest = digest_plan(&plan)?;
    Ok(plan)
}

fn managed_browser_process_identities() -> Result<BTreeMap<String, RecordedProcessIdentity>, String>
{
    use crate::native::service_store::JsonServiceStateStore;

    let path = JsonServiceStateStore::default_path()?;
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => {
            return Err(format!(
                "runtime_replacement_service_state_read_failed:{}:{error}",
                path.display()
            ))
        }
    };
    let state = crate::native::service_state_migration::read_service_state(&raw)
        .map_err(|error| format!("runtime_replacement_service_state_invalid:{error}"))?;
    Ok(state
        .browser_process_identities
        .into_iter()
        .map(|(browser_id, identity)| (browser_id, identity.process_identity))
        .collect())
}

fn replacement_must_preserve_takeover_blocker(code: &str) -> bool {
    matches!(
        code,
        "blocked_boot_epoch"
            | "blocked_runtime_topology"
            | "blocked_active_ingress_transaction"
            | "blocked_selected_process_identity"
            | "blocked_selected_binary_mismatch"
            | "blocked_supervisor_other_main_pid"
            | "blocked_unrelated_port_owner"
            | "blocked_active_admission_drain"
            | "blocked_active_workstation_transaction"
            | "blocked_active_takeover_transaction"
    )
}

fn build_runtime_replacement_plan(
    policy: RuntimeReplacementPolicy,
    ingress_revision: u64,
    selected_backend: RuntimeHostBackend,
    selected_process_identity: Option<RecordedProcessIdentity>,
    census: StableRuntimeCensus,
    browser_process_identities: &BTreeMap<String, RecordedProcessIdentity>,
) -> Result<RuntimeReplacementPlan, String> {
    let mut browsers = census
        .records
        .iter()
        .map(|record| {
            let mut session_names = record.session_names.clone();
            session_names.sort();
            session_names.dedup();
            RuntimeReplacementBrowser {
                logical_browser_id: record.logical_browser_id.clone(),
                session_names,
                profile_identity_digest: record.profile_identity_digest.clone(),
                classification: record.classification,
                process_identity: browser_process_identities
                    .get(&record.logical_browser_id)
                    .cloned(),
                close_required: matches!(
                    record.classification,
                    RuntimeClassification::CooperativeLiveOwner
                        | RuntimeClassification::OrphanAdoptable
                ),
            }
        })
        .collect::<Vec<_>>();
    browsers.sort_by(|left, right| left.logical_browser_id.cmp(&right.logical_browser_id));

    let live_state_will_end = browsers.iter().any(|browser| browser.close_required);
    let mut blockers = Vec::new();
    if policy == RuntimeReplacementPolicy::FullShutdown {
        if selected_backend.topology != RuntimeHostTopology::SingleHost {
            push_blocker(
                &mut blockers,
                "blocked_runtime_topology",
                "Full shutdown requires one selected single-host runtime.",
            );
        }
        match selected_process_identity.as_ref() {
            Some(identity)
                if identity.pid == selected_backend.pid
                    && identity
                        .executable_path
                        .as_deref()
                        .is_some_and(|path| !path.trim().is_empty()) => {}
            _ => push_blocker(
                &mut blockers,
                "blocked_selected_process_identity",
                "The selected runtime host lacks an exact executable and process-start identity.",
            ),
        }
        if !census.activation_allowed {
            push_blocker(
                &mut blockers,
                "blocked_ambiguous_census",
                "The stable runtime census does not authorize activation.",
            );
        }
        for record in &census.records {
            if matches!(
                record.classification,
                RuntimeClassification::CooperativeLiveOwner
                    | RuntimeClassification::OrphanAdoptable
            ) && !browser_process_identities.contains_key(&record.logical_browser_id)
            {
                push_blocker(
                    &mut blockers,
                    "blocked_browser_process_identity_missing",
                    &format!(
                        "Browser {} lacks an exact process identity for forced shutdown.",
                        record.logical_browser_id
                    ),
                );
            }
            match record.classification {
                RuntimeClassification::OrphanAdoptable => {}
                RuntimeClassification::ConflictingOwner
                | RuntimeClassification::InsufficientEvidence => push_blocker(
                    &mut blockers,
                    "blocked_ambiguous_runtime_owner",
                    &format!(
                        "Browser {} does not have unambiguous package ownership.",
                        record.logical_browser_id
                    ),
                ),
                RuntimeClassification::CooperativeLiveOwner
                | RuntimeClassification::ManualPreserveOnly
                | RuntimeClassification::IdleDaemon
                | RuntimeClassification::StaleMetadata
                | RuntimeClassification::ExternalObserved => {}
            }
        }
    }

    blockers.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.message.cmp(&right.message))
    });
    blockers.dedup();
    let disposition = match policy {
        RuntimeReplacementPolicy::Preserve => RuntimeReplacementDisposition::PreserveContinuity,
        RuntimeReplacementPolicy::FullShutdown if blockers.is_empty() => {
            RuntimeReplacementDisposition::ReadyForFullShutdown
        }
        RuntimeReplacementPolicy::FullShutdown => RuntimeReplacementDisposition::Blocked,
    };
    let mut plan = RuntimeReplacementPlan {
        schema_version: PLAN_SCHEMA_VERSION.to_string(),
        plan_digest: String::new(),
        policy,
        disposition,
        blockers,
        ingress_revision,
        selected_backend,
        selected_process_identity,
        census_digest: census.digest,
        browsers,
        profiles_preserved: true,
        live_state_will_end,
    };
    plan.plan_digest = digest_plan(&plan)?;
    Ok(plan)
}

fn push_blocker(blockers: &mut Vec<RuntimeReplacementBlocker>, code: &str, message: &str) {
    blockers.push(RuntimeReplacementBlocker {
        code: code.to_string(),
        message: message.to_string(),
    });
}

fn digest_plan(plan: &RuntimeReplacementPlan) -> Result<String, String> {
    let mut unsigned = plan.clone();
    unsigned.plan_digest.clear();
    serde_json::to_vec(&unsigned)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| format!("runtime_replacement_plan_serialize_failed:{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_adoption::{
        RuntimeCensusRecord, RuntimeDisposition, RUNTIME_ADOPTION_SCHEMA_VERSION,
    };
    use crate::runtime_host_ingress::RuntimeHostTopology;
    use std::path::PathBuf;

    fn observation() -> (
        u64,
        RuntimeHostBackend,
        Option<RecordedProcessIdentity>,
        StableRuntimeCensus,
    ) {
        (
            7,
            RuntimeHostBackend {
                topology: RuntimeHostTopology::SingleHost,
                generation_id: "generation-one".to_string(),
                socket_dir: PathBuf::from("/run/user/1000/agent-browser"),
                binary_sha256: "a".repeat(64),
                host_id: "runtime-host:41".to_string(),
                pid: 41,
                socket_identity: "unix:one".to_string(),
            },
            Some(RecordedProcessIdentity {
                pid: 41,
                start_token: "start-41".to_string(),
                executable_path: Some("/opt/agent-browser".to_string()),
                browser_family: None,
            }),
            StableRuntimeCensus {
                schema_version: RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
                digest: "b".repeat(64),
                registry_revision: 11,
                activation_allowed: true,
                records: vec![RuntimeCensusRecord {
                    logical_browser_id: "session:research".to_string(),
                    session_names: vec!["research".to_string()],
                    profile_identity_digest: "c".repeat(64),
                    observed_sources: Vec::new(),
                    classification: RuntimeClassification::CooperativeLiveOwner,
                    disposition: RuntimeDisposition::CooperativeTransfer,
                    reason_codes: vec!["live_owned_browser".to_string()],
                }],
            },
        )
    }

    fn browser_identities() -> BTreeMap<String, RecordedProcessIdentity> {
        BTreeMap::from([(
            "session:research".to_string(),
            RecordedProcessIdentity {
                pid: 84,
                start_token: "browser-start-84".to_string(),
                executable_path: Some("/opt/chrome".to_string()),
                browser_family: Some("chrome".to_string()),
            },
        )])
    }

    fn authorization(plan: &RuntimeReplacementPlan) -> RuntimeReplacementAuthorization {
        authorize_reviewed_full_shutdown(
            plan,
            &plan.plan_digest,
            "operator:test",
            ProfileIdentityAssurance::Operator,
            &[
                ProfilePermission::LifecycleManage,
                ProfilePermission::FullShutdown,
            ],
            "2026-09-02T20:00:00Z",
        )
        .unwrap()
    }

    #[test]
    fn full_shutdown_plan_is_deterministic_and_preserves_profile_state() {
        let (revision, backend, identity, census) = observation();
        let first = build_runtime_replacement_plan(
            RuntimeReplacementPolicy::FullShutdown,
            revision,
            backend.clone(),
            identity.clone(),
            census.clone(),
            &browser_identities(),
        )
        .unwrap();
        let second = build_runtime_replacement_plan(
            RuntimeReplacementPolicy::FullShutdown,
            revision,
            backend,
            identity,
            census,
            &browser_identities(),
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.plan_digest.len(), 64);
        assert_eq!(
            first.disposition,
            RuntimeReplacementDisposition::ReadyForFullShutdown
        );
        assert!(first.profiles_preserved);
        assert!(first.live_state_will_end);
        assert_eq!(first.browsers.len(), 1);
        assert!(first.browsers[0].close_required);
        let payload = serde_json::to_string(&first).unwrap();
        assert!(!payload.contains("user-data"));
        assert!(!payload.contains("research.gov"));
    }

    #[test]
    fn upgrade_transaction_rejects_replacement_policy_or_plan_drift() {
        let (revision, backend, identity, census) = observation();
        let plan = build_runtime_replacement_plan(
            RuntimeReplacementPolicy::FullShutdown,
            revision,
            backend,
            identity,
            census,
            &browser_identities(),
        )
        .unwrap();
        let mut transaction = crate::runtime_adoption::UpgradeTransaction {
            schema_version: crate::runtime_adoption::RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
            transaction_id: "upgrade-one".to_string(),
            requested_by: "test".to_string(),
            old_generation_id: Some("old".to_string()),
            candidate_generation_id: "candidate".to_string(),
            candidate_binary_sha256: "d".repeat(64),
            candidate_support_manifest_sha256: "e".repeat(64),
            runtime_census_digest: None,
            runtime_migrations: Vec::new(),
            runtime_handoffs: Vec::new(),
            runtime_host_convergence: None,
            service_state_migration: None,
            state: crate::runtime_adoption::UpgradeTransactionState::Planned,
            revision: 0,
            checkpoints: Vec::new(),
            dashboard_validation_summary: None,
            presentation_validation_summary: None,
            terminal_result: None,
            stop_reason: None,
            successor_fields: std::collections::BTreeMap::new(),
        };

        bind_upgrade_transaction(&mut transaction, &plan).unwrap();
        bind_upgrade_transaction(&mut transaction, &plan).unwrap();
        let authorization = authorization(&plan);
        bind_full_shutdown_authorization(&mut transaction, &plan, &authorization).unwrap();
        bind_full_shutdown_authorization(&mut transaction, &plan, &authorization).unwrap();
        assert_eq!(
            plan_from_upgrade_transaction(&transaction).unwrap(),
            Some(plan.clone())
        );
        assert_eq!(
            authorization_from_upgrade_transaction(&transaction).unwrap(),
            authorization
        );

        let mut changed = plan;
        changed.policy = RuntimeReplacementPolicy::Preserve;
        assert_eq!(
            bind_upgrade_transaction(&mut transaction, &changed).unwrap_err(),
            "runtime_replacement_transaction_plan_changed"
        );
    }

    #[test]
    fn full_shutdown_authorization_requires_operator_permission_and_exact_plan() {
        let (revision, backend, identity, census) = observation();
        let plan = build_runtime_replacement_plan(
            RuntimeReplacementPolicy::FullShutdown,
            revision,
            backend,
            identity,
            census,
            &browser_identities(),
        )
        .unwrap();

        assert_eq!(
            authorize_reviewed_full_shutdown(
                &plan,
                &plan.plan_digest,
                "client:self-declared",
                ProfileIdentityAssurance::SelfDeclared,
                &[
                    ProfilePermission::LifecycleManage,
                    ProfilePermission::FullShutdown
                ],
                "2026-09-02T20:00:00Z",
            )
            .unwrap_err(),
            "runtime_replacement_operator_authorization_required"
        );
        assert_eq!(
            authorize_reviewed_full_shutdown(
                &plan,
                &plan.plan_digest,
                "operator:test",
                ProfileIdentityAssurance::Operator,
                &[ProfilePermission::FullShutdown],
                "2026-09-02T20:00:00Z",
            )
            .unwrap_err(),
            "runtime_replacement_operator_authorization_required"
        );
        assert_eq!(
            authorize_reviewed_full_shutdown(
                &plan,
                &"0".repeat(64),
                "operator:test",
                ProfileIdentityAssurance::Operator,
                &[
                    ProfilePermission::LifecycleManage,
                    ProfilePermission::FullShutdown
                ],
                "2026-09-02T20:00:00Z",
            )
            .unwrap_err(),
            "runtime_replacement_authorization_plan_mismatch"
        );
    }

    #[test]
    fn full_shutdown_forces_exact_browser_when_old_runtime_refuses_close() {
        struct FakeEffects {
            calls: Vec<String>,
            census: StableRuntimeCensus,
            source_running: bool,
            browser_running: bool,
        }

        impl RuntimeReplacementEffects for FakeEffects {
            fn checkpoint(
                &mut self,
                receipt: &RuntimeReplacementEffectReceipt,
            ) -> Result<(), String> {
                self.calls.push(format!("checkpoint:{:?}", receipt.state));
                Ok(())
            }

            fn close_session(&mut self, session_name: &str) -> Result<(), String> {
                self.calls.push(format!("close:{session_name}"));
                Err("old runtime refused close".to_string())
            }

            fn browser_is_running(
                &mut self,
                identity: &RecordedProcessIdentity,
            ) -> Result<bool, String> {
                self.calls.push(format!("browser-running:{}", identity.pid));
                Ok(self.browser_running)
            }

            fn force_close_browser(
                &mut self,
                identity: &RecordedProcessIdentity,
            ) -> Result<(), String> {
                self.calls.push(format!("force-browser:{}", identity.pid));
                self.browser_running = false;
                Ok(())
            }

            fn collect_stable_census(&mut self) -> Result<StableRuntimeCensus, String> {
                self.calls.push("census".to_string());
                Ok(self.census.clone())
            }

            fn source_is_running(&mut self) -> Result<bool, String> {
                self.calls.push("source-running".to_string());
                Ok(self.source_running)
            }

            fn retire_source(&mut self) -> Result<(), String> {
                self.calls.push("retire-source".to_string());
                self.source_running = false;
                Ok(())
            }
        }

        let (revision, backend, identity, census) = observation();
        let plan = build_runtime_replacement_plan(
            RuntimeReplacementPolicy::FullShutdown,
            revision,
            backend,
            identity,
            census,
            &browser_identities(),
        )
        .unwrap();
        let final_census = StableRuntimeCensus {
            schema_version: RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
            digest: "f".repeat(64),
            registry_revision: 12,
            activation_allowed: true,
            records: Vec::new(),
        };
        let mut effects = FakeEffects {
            calls: Vec::new(),
            census: final_census,
            source_running: true,
            browser_running: true,
        };

        let authorization = authorization(&plan);
        let receipt =
            execute_full_shutdown_with(&plan, &authorization, None, &mut effects).unwrap();
        assert_eq!(receipt.state, RuntimeReplacementEffectState::SourceAbsent);
        assert_eq!(receipt.closed_sessions, vec!["research"]);
        assert!(receipt.source_exit_proven);
        assert!(receipt.profiles_preserved);
        assert_eq!(receipt.forced_browser_ids, vec!["session:research"]);
        assert_eq!(
            effects.calls,
            vec![
                "checkpoint:Planned",
                "checkpoint:BrowsersClosing",
                "close:research",
                "force-browser:84",
                "browser-running:84",
                "checkpoint:BrowsersClosing",
                "census",
                "checkpoint:BrowsersClosed",
                "source-running",
                "checkpoint:SourceRetiring",
                "retire-source",
                "source-running",
                "census",
                "checkpoint:SourceAbsent",
            ]
        );

        effects.calls.clear();
        let replayed =
            execute_full_shutdown_with(&plan, &authorization, Some(receipt.clone()), &mut effects)
                .expect("source-absent replay should remain idempotent");
        assert_eq!(replayed, receipt);
        assert_eq!(effects.calls, vec!["source-running", "census"]);
    }
}
