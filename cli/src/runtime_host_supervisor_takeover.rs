//! Transactional transfer of one browserless runtime host into systemd custody.
//!
//! This module owns the recovery decision and effects. Callers provide only an
//! exact plan digest; they never choose a PID, port, socket, signal, or browser.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::process_identity::{
    BootEpochStatus, ProcessObservation, RecordedProcessIdentity, VerifiedProcessSignal,
    VerifiedProcessTermination,
};
use crate::runtime_adoption::{RuntimeClassification, StableRuntimeCensus};
use crate::runtime_host_ingress::{
    RuntimeHostBackend, RuntimeHostIngressRepository, RuntimeHostTopology,
};
use crate::session_supervisor::RuntimeHostSupervisorObservation;

const PLAN_SCHEMA_VERSION: &str = "agent-browser.runtime-host-supervisor-takeover-plan.v1";
const TRANSACTION_SCHEMA_VERSION: &str = "agent-browser.runtime-host-supervisor-takeover.v1";
const OUTCOME_SCHEMA_VERSION: &str = "agent-browser.runtime-host-supervisor-takeover-outcome.v1";
const SOURCE_EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const REPLACEMENT_READY_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SupervisorTakeoverDisposition {
    AlreadySupervised,
    ReadyForTakeover,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SupervisorTakeoverBlocker {
    pub(crate) code: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SupervisorTakeoverPlan {
    pub(crate) schema_version: String,
    pub(crate) plan_digest: String,
    pub(crate) disposition: SupervisorTakeoverDisposition,
    pub(crate) blockers: Vec<SupervisorTakeoverBlocker>,
    pub(crate) ingress_revision: u64,
    pub(crate) boot_epoch_status: String,
    pub(crate) selected_backend: RuntimeHostBackend,
    pub(crate) selected_process_identity: Option<RecordedProcessIdentity>,
    pub(crate) supervisor: RuntimeHostSupervisorObservation,
    pub(crate) census: StableRuntimeCensus,
    pub(crate) selected_listener_ports: Vec<u16>,
    pub(crate) p147_capability_ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SupervisorTakeoverState {
    Planned,
    CensusStable,
    AdmissionDraining,
    SourceRetiring,
    SourceAbsent,
    SupervisorStarting,
    ReplacementReady,
    IngressAdopted,
    Accepted,
    ClosedZeroEffect,
    OperatorRecoveryRequired,
}

impl SupervisorTakeoverState {
    fn is_terminal(self) -> bool {
        matches!(self, Self::Accepted | Self::ClosedZeroEffect)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SupervisorTakeoverTransaction {
    schema_version: String,
    transaction_id: String,
    revision: u64,
    state: SupervisorTakeoverState,
    plan_digest: String,
    source_backend: RuntimeHostBackend,
    source_process_identity: RecordedProcessIdentity,
    supervisor_manifest_digest: String,
    created_at: String,
    updated_at: String,
    replacement_pid: Option<u32>,
    accepted_ingress_revision: Option<u64>,
    failure: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent_admission_transaction_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SupervisorTakeoverOutcome {
    pub(crate) schema_version: String,
    pub(crate) state: String,
    pub(crate) transaction_id: Option<String>,
    pub(crate) transaction_revision: Option<u64>,
    pub(crate) plan_digest: String,
    pub(crate) source_pid: u32,
    pub(crate) replacement_pid: Option<u32>,
    pub(crate) ingress_revision: u64,
    pub(crate) browser_launched: bool,
}

pub(crate) fn plan_supervisor_takeover() -> Result<SupervisorTakeoverPlan, String> {
    plan_supervisor_takeover_with_admission(None)
}

fn plan_supervisor_takeover_with_admission(
    parent_admission_transaction_id: Option<&str>,
) -> Result<SupervisorTakeoverPlan, String> {
    let repository =
        RuntimeHostIngressRepository::new(RuntimeHostIngressRepository::default_path());
    let registry = repository.load()?;
    let selected = registry.selected_backend().clone();
    let supervisor = crate::session_supervisor::runtime_host_supervisor_observation()?;
    let census = crate::workstation_install::collect_stable_host_runtime_census()?;
    let identity = read_selected_process_identity(&selected).ok();
    let listener_ports = listener_ports_for_pid(selected.pid).unwrap_or_default();
    let mut blockers = Vec::new();

    if registry.boot_epoch_status() != BootEpochStatus::Current {
        push_blocker(
            &mut blockers,
            "blocked_boot_epoch",
            "Selected runtime-host evidence is not bound to the current boot.",
        );
    }
    if selected.topology != RuntimeHostTopology::SingleHost {
        push_blocker(
            &mut blockers,
            "blocked_runtime_topology",
            "Supervisor takeover requires one selected single-host runtime.",
        );
    }
    if registry.active_transaction_id.is_some() || registry.candidate_backend().is_some() {
        push_blocker(
            &mut blockers,
            "blocked_active_ingress_transaction",
            "A runtime-host ingress transaction is already active.",
        );
    }

    match identity.as_ref() {
        Some(recorded) => validate_selected_process(&selected, recorded, &mut blockers),
        None => push_blocker(
            &mut blockers,
            "blocked_selected_process_identity",
            "The selected runtime host has no readable exact process identity.",
        ),
    }

    let already_supervised = supervisor.active_state == "active"
        && supervisor.sub_state == "running"
        && supervisor.main_pid == Some(selected.pid);

    validate_supervisor(&selected, &supervisor, already_supervised, &mut blockers);
    validate_runtime_conflicts(
        &selected,
        &supervisor,
        &census,
        &listener_ports,
        already_supervised,
        parent_admission_transaction_id.is_some(),
        &mut blockers,
    );
    if let Some(identity) = identity.as_ref() {
        let listener_inventory =
            crate::install::daemon_listener_inventory(identity.executable_path.as_deref());
        validate_authoritative_listener_inventory(&selected, &listener_inventory, &mut blockers);
    }

    if let Some(blocker) = active_coordination_blocker(parent_admission_transaction_id)? {
        blockers.push(blocker);
    }

    let p147_capability_ready = parent_admission_transaction_id.is_some()
        || current_executable_sha256().is_ok_and(|digest| digest == selected.binary_sha256);
    if !p147_capability_ready {
        push_blocker(
            &mut blockers,
            "blocked_p147_capability_missing",
            "The invoking executable does not match the selected P147-capable generation.",
        );
    }

    let disposition = if !blockers.is_empty() {
        SupervisorTakeoverDisposition::Blocked
    } else if already_supervised {
        SupervisorTakeoverDisposition::AlreadySupervised
    } else {
        SupervisorTakeoverDisposition::ReadyForTakeover
    };
    let mut plan = SupervisorTakeoverPlan {
        schema_version: PLAN_SCHEMA_VERSION.to_string(),
        plan_digest: String::new(),
        disposition,
        blockers,
        ingress_revision: registry.revision,
        boot_epoch_status: boot_epoch_label(registry.boot_epoch_status()).to_string(),
        selected_backend: selected,
        selected_process_identity: identity,
        supervisor,
        census,
        selected_listener_ports: listener_ports.into_iter().collect(),
        p147_capability_ready,
    };
    plan.plan_digest = digest_json(&plan)?;
    Ok(plan)
}

pub(crate) fn apply_supervisor_takeover(
    expected_plan_digest: &str,
) -> Result<SupervisorTakeoverOutcome, String> {
    let initial = plan_supervisor_takeover()?;
    apply_supervisor_takeover_from_plan(initial, expected_plan_digest, None)
}

fn apply_supervisor_takeover_from_plan(
    initial: SupervisorTakeoverPlan,
    expected_plan_digest: &str,
    parent_admission_transaction_id: Option<&str>,
) -> Result<SupervisorTakeoverOutcome, String> {
    require_plan_digest(&initial, expected_plan_digest)?;
    if initial.disposition == SupervisorTakeoverDisposition::AlreadySupervised {
        return Ok(outcome_without_transaction(&initial));
    }
    require_ready_plan(&initial)?;

    let _lock = acquire_takeover_lock()?;
    let plan = plan_supervisor_takeover_with_admission(parent_admission_transaction_id)?;
    require_plan_digest(&plan, expected_plan_digest)?;
    require_ready_plan(&plan)?;
    let source_identity = plan
        .selected_process_identity
        .clone()
        .ok_or_else(|| "blocked_selected_process_identity".to_string())?;
    let mut transaction = SupervisorTakeoverTransaction {
        schema_version: TRANSACTION_SCHEMA_VERSION.to_string(),
        transaction_id: format!("runtime-host-takeover-{}", uuid::Uuid::new_v4()),
        revision: 1,
        state: SupervisorTakeoverState::Planned,
        plan_digest: plan.plan_digest.clone(),
        source_backend: plan.selected_backend.clone(),
        source_process_identity: source_identity.clone(),
        supervisor_manifest_digest: digest_json(&plan.supervisor.manifests)?,
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
        replacement_pid: None,
        accepted_ingress_revision: None,
        failure: None,
        parent_admission_transaction_id: parent_admission_transaction_id.map(str::to_string),
    };
    write_transaction(&transaction)?;

    let effect_result = execute_takeover(&plan, &source_identity, &mut transaction);
    if let Err(error) = effect_result {
        transaction.failure = Some(error.clone());
        if matches!(
            transaction.state,
            SupervisorTakeoverState::Planned
                | SupervisorTakeoverState::CensusStable
                | SupervisorTakeoverState::AdmissionDraining
        ) {
            transaction.state = SupervisorTakeoverState::ClosedZeroEffect;
            transaction.revision = transaction.revision.saturating_add(1);
            transaction.updated_at = current_timestamp();
            write_transaction(&transaction)?;
            clear_takeover_admission_drain(&transaction)?;
        } else {
            transaction.state = SupervisorTakeoverState::OperatorRecoveryRequired;
            transaction.revision = transaction.revision.saturating_add(1);
            transaction.updated_at = current_timestamp();
            write_transaction(&transaction)?;
            write_admission_drain(&transaction)?;
        }
        return Err(format!(
            "runtime_host_supervisor_takeover_failed:{}:{}",
            transaction.transaction_id, error
        ));
    }

    Ok(SupervisorTakeoverOutcome {
        schema_version: OUTCOME_SCHEMA_VERSION.to_string(),
        state: "accepted".to_string(),
        transaction_id: Some(transaction.transaction_id),
        transaction_revision: Some(transaction.revision),
        plan_digest: plan.plan_digest,
        source_pid: plan.selected_backend.pid,
        replacement_pid: transaction.replacement_pid,
        ingress_revision: transaction
            .accepted_ingress_revision
            .unwrap_or(plan.ingress_revision),
        browser_launched: false,
    })
}

/// Complete the accepted workstation upgrade by placing the selected runtime
/// host under the rewritten user supervisor, then prove the resulting
/// supervisor and runtime topology from a fresh census.
pub(crate) fn ensure_selected_runtime_host_supervised_with_admission(
    parent_admission_transaction_id: Option<&str>,
) -> Result<SupervisorTakeoverOutcome, String> {
    if let Some(parent_id) = parent_admission_transaction_id {
        retire_superseded_runtime_hosts_for_accepted_upgrade(parent_id)?;
    }
    let plan = plan_supervisor_takeover_with_admission(parent_admission_transaction_id)?;
    let outcome = match plan.disposition {
        SupervisorTakeoverDisposition::AlreadySupervised => outcome_without_transaction(&plan),
        SupervisorTakeoverDisposition::ReadyForTakeover => {
            let plan_digest = plan.plan_digest.clone();
            apply_supervisor_takeover_from_plan(
                plan,
                &plan_digest,
                parent_admission_transaction_id,
            )?
        }
        SupervisorTakeoverDisposition::Blocked => {
            require_ready_plan(&plan)?;
            unreachable!("blocked supervisor takeover plan was accepted")
        }
    };
    let verified = plan_supervisor_takeover_with_admission(parent_admission_transaction_id)?;
    if verified.disposition != SupervisorTakeoverDisposition::AlreadySupervised {
        return Err(format!(
            "runtime_host_supervisor_post_upgrade_verification_failed:{:?}",
            verified.blockers
        ));
    }
    Ok(outcome)
}

/// Retire exact non-selected production runtime hosts while an accepted
/// workstation transaction owns admission. Runtime-host shutdown leaves
/// retained browser processes alive; the selected host re-adopts their
/// Service State routes after the supervisor restart.
#[cfg(target_os = "linux")]
fn retire_superseded_runtime_hosts_for_accepted_upgrade(parent_id: &str) -> Result<(), String> {
    let drain_path = crate::runtime_adoption::runtime_admission_drain_path()?;
    let drain: crate::runtime_adoption::RuntimeAdmissionDrain = serde_json::from_slice(
        &fs::read(&drain_path)
            .map_err(|error| format!("runtime_admission_drain_unreadable:{error}"))?,
    )
    .map_err(|error| format!("runtime_admission_drain_invalid:{error}"))?;
    if drain.transaction_id != parent_id {
        return Err("superseded_runtime_retirement_admission_owner_changed".to_string());
    }

    let repository =
        RuntimeHostIngressRepository::new(RuntimeHostIngressRepository::default_path());
    let registry = repository.load()?;
    let selected = registry.selected_backend();
    let namespace = selected
        .socket_dir
        .parent()
        .ok_or_else(|| "selected_runtime_socket_namespace_missing".to_string())?;
    let mut allowed_hashes = BTreeSet::from([selected.binary_sha256.clone()]);
    if let Some(fallback) = registry.fallback_backend() {
        allowed_hashes.insert(fallback.binary_sha256.clone());
    }
    allowed_hashes.insert(current_executable_sha256()?);
    let invoking_path = std::env::current_exe()
        .map_err(|error| format!("current_executable_unavailable:{error}"))?
        .canonicalize()
        .map_err(|error| format!("current_executable_unavailable:{error}"))?;

    reconcile_absent_runtime_host_authority_records(namespace, &selected.socket_dir)?;

    let inventory = crate::install::daemon_listener_inventory(None);
    if inventory
        .get("available")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return Err("superseded_runtime_listener_inventory_unavailable".to_string());
    }
    let mut retired = BTreeSet::new();
    for listener in inventory
        .get("listeners")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(pid) = listener
            .get("pid")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
        else {
            continue;
        };
        let Some(socket_path) = listener
            .get("socketPath")
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from)
        else {
            continue;
        };
        if pid == selected.pid
            || socket_path.file_name().and_then(|value| value.to_str()) != Some("runtime-host.sock")
            || socket_path.parent().and_then(Path::parent) != Some(namespace)
            || !retired.insert(pid)
        {
            continue;
        }
        let socket_dir = socket_path
            .parent()
            .ok_or_else(|| "superseded_runtime_socket_directory_missing".to_string())?;
        require_runtime_host_process_environment(pid, socket_dir)?;
        if runtime_host_has_live_browser_child(pid)? {
            return Err(format!("superseded_runtime_retains_live_browser:{pid}"));
        }
        let observed = match crate::process_identity::observe_process(pid) {
            ProcessObservation::Observed(observed) => observed,
            ProcessObservation::Missing => continue,
            ProcessObservation::Failed { reason } => {
                return Err(format!(
                    "superseded_runtime_process_unobservable:{pid}:{reason}"
                ));
            }
        };
        let start_token = observed
            .start_token
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("superseded_runtime_start_token_missing:{pid}"))?;
        let executable_hash = sha256_file(Path::new(&format!("/proc/{pid}/exe")))?;
        let replaced_invoking_path = observed
            .executable_path
            .as_deref()
            .and_then(|path| path.strip_suffix(" (deleted)"))
            .is_some_and(|path| Path::new(path) == invoking_path);
        if !allowed_hashes.contains(&executable_hash) && !replaced_invoking_path {
            return Err(format!("superseded_runtime_binary_unrecognized:{pid}"));
        }
        let identity = RecordedProcessIdentity {
            pid,
            start_token,
            executable_path: observed.executable_path,
            browser_family: observed.browser_family,
        };
        let Some(process) = VerifiedProcessTermination::open(&identity)? else {
            continue;
        };
        process.signal(VerifiedProcessSignal::Terminate)?;
        wait_for_process_exit(&process, SOURCE_EXIT_TIMEOUT)?;
        if process.is_running()? {
            process.signal(VerifiedProcessSignal::Kill)?;
            wait_for_process_exit(&process, SOURCE_EXIT_TIMEOUT)?;
        }
        if process.is_running()? {
            return Err(format!("superseded_runtime_exit_timeout:{pid}"));
        }
    }
    Ok(())
}

/// Remove only runtime-host authority artifacts whose exact recorded process
/// identity is absent. Session engine and handoff artifacts remain available
/// for browser transfer and incident diagnosis. Invalid or unreadable identity
/// records are retained for explicit diagnosis rather than treated as proof of
/// absence.
#[cfg(target_os = "linux")]
fn reconcile_absent_runtime_host_authority_records(
    namespace: &Path,
    selected_socket_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    let entries = match fs::read_dir(namespace) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "runtime_host_namespace_unreadable:{}:{error}",
                namespace.display()
            ));
        }
    };
    let mut reconciled = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "runtime_host_namespace_entry_unreadable:{}:{error}",
                namespace.display()
            )
        })?;
        let socket_dir = entry.path();
        if socket_dir == selected_socket_dir || !socket_dir.is_dir() {
            continue;
        }
        let identity_path = socket_dir.join("runtime-host.identity.json");
        let identity: RecordedProcessIdentity = match fs::read(&identity_path)
            .ok()
            .and_then(|body| serde_json::from_slice(&body).ok())
        {
            Some(identity) => identity,
            None => continue,
        };
        if VerifiedProcessTermination::open(&identity)?.is_some() {
            continue;
        }
        remove_absent_runtime_host_authority_artifacts(&socket_dir)?;
        reconciled.push(socket_dir);
    }
    Ok(reconciled)
}

#[cfg(target_os = "linux")]
fn remove_absent_runtime_host_authority_artifacts(socket_dir: &Path) -> Result<(), String> {
    const AUTHORITY_FILES: [&str; 7] = [
        "runtime-host.identity.json",
        "runtime-host.json",
        "runtime-host.pid",
        "runtime-host.sha256",
        "runtime-host.sock",
        "runtime-host.token",
        "runtime-host.version",
    ];
    for name in AUTHORITY_FILES {
        let path = socket_dir.join(name);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "absent_runtime_host_authority_cleanup_failed:{}:{error}",
                    path.display()
                ));
            }
        }
    }
    for entry in fs::read_dir(socket_dir).map_err(|error| {
        format!(
            "absent_runtime_host_directory_unreadable:{}:{error}",
            socket_dir.display()
        )
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "absent_runtime_host_directory_entry_unreadable:{}:{error}",
                socket_dir.display()
            )
        })?;
        if entry.path().extension().and_then(|value| value.to_str()) == Some("stream") {
            fs::remove_file(entry.path()).map_err(|error| {
                format!(
                    "absent_runtime_host_stream_cleanup_failed:{}:{error}",
                    entry.path().display()
                )
            })?;
        }
    }
    match fs::remove_dir(socket_dir) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
        Err(error) => {
            return Err(format!(
                "absent_runtime_host_directory_cleanup_failed:{}:{error}",
                socket_dir.display()
            ));
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn runtime_host_has_live_browser_child(runtime_pid: u32) -> Result<bool, String> {
    use crate::native::service_store::{JsonServiceStateStore, ServiceStateStore};

    let state = JsonServiceStateStore::new(JsonServiceStateStore::default_path()?).load()?;
    for browser_pid in state.browsers.values().filter_map(|browser| browser.pid) {
        let stat = match fs::read_to_string(format!("/proc/{browser_pid}/stat")) {
            Ok(stat) => stat,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!(
                    "superseded_runtime_browser_parent_unreadable:{browser_pid}:{error}"
                ));
            }
        };
        let Some(after_name) = stat.rsplit_once(") ").map(|(_, value)| value) else {
            return Err(format!(
                "superseded_runtime_browser_stat_invalid:{browser_pid}"
            ));
        };
        let parent_pid = after_name
            .split_whitespace()
            .nth(1)
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| format!("superseded_runtime_browser_stat_invalid:{browser_pid}"))?;
        if parent_pid == runtime_pid {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(target_os = "linux")]
fn require_runtime_host_process_environment(pid: u32, socket_dir: &Path) -> Result<(), String> {
    let bytes = fs::read(format!("/proc/{pid}/environ"))
        .map_err(|error| format!("superseded_runtime_environment_unreadable:{pid}:{error}"))?;
    let entries = bytes
        .split(|byte| *byte == 0)
        .filter_map(|entry| std::str::from_utf8(entry).ok())
        .collect::<Vec<_>>();
    let runtime_host = entries
        .iter()
        .any(|entry| *entry == format!("{}=1", crate::runtime_host::RUNTIME_HOST_ENV));
    let expected_socket = format!("AGENT_BROWSER_SOCKET_DIR={}", socket_dir.display());
    let socket_matches = entries.iter().any(|entry| *entry == expected_socket);
    if runtime_host && socket_matches {
        Ok(())
    } else {
        Err(format!("superseded_runtime_environment_mismatch:{pid}"))
    }
}

#[cfg(not(target_os = "linux"))]
fn retire_superseded_runtime_hosts_for_accepted_upgrade(_parent_id: &str) -> Result<(), String> {
    Ok(())
}

pub(crate) fn resume_supervisor_takeover(
    transaction_id: &str,
    expected_revision: u64,
) -> Result<SupervisorTakeoverOutcome, String> {
    let _lock = acquire_takeover_lock()?;
    let mut transaction = read_transaction()?;
    if transaction.transaction_id != transaction_id || transaction.revision != expected_revision {
        return Err(format!(
            "runtime_host_supervisor_takeover_revision_changed:transaction={}:revision={}",
            transaction.transaction_id, transaction.revision
        ));
    }
    if transaction.state == SupervisorTakeoverState::Accepted {
        return Ok(outcome_from_transaction(&transaction));
    }
    if transaction.state != SupervisorTakeoverState::OperatorRecoveryRequired {
        return Err(format!(
            "runtime_host_supervisor_takeover_not_resumable:{:?}",
            transaction.state
        ));
    }
    if crate::process_identity::recorded_process_is_running(&transaction.source_process_identity)? {
        return Err("runtime_host_supervisor_takeover_source_still_live".to_string());
    }
    let supervisor = crate::session_supervisor::runtime_host_supervisor_observation()?;
    if digest_json(&supervisor.manifests)? != transaction.supervisor_manifest_digest {
        return Err("runtime_host_supervisor_takeover_manifest_changed".to_string());
    }
    let drain_path = crate::runtime_adoption::runtime_admission_drain_path()?;
    let drain: crate::runtime_adoption::RuntimeAdmissionDrain = serde_json::from_slice(
        &fs::read(&drain_path)
            .map_err(|error| format!("runtime_admission_drain_unreadable:{error}"))?,
    )
    .map_err(|error| format!("runtime_admission_drain_invalid:{error}"))?;
    if drain.transaction_id != takeover_admission_owner_id(&transaction) {
        return Err("runtime_admission_drain_owner_changed".to_string());
    }

    let recovery = (|| -> Result<(u32, u64), String> {
        if supervisor.active_state != "active" || supervisor.main_pid.is_none() {
            crate::session_supervisor::start_runtime_host_supervisor_once()?;
        }
        wait_for_replacement(&transaction.source_backend)
    })();
    match recovery {
        Ok((replacement_pid, ingress_revision)) => {
            transaction.replacement_pid = Some(replacement_pid);
            advance_transaction(&mut transaction, SupervisorTakeoverState::ReplacementReady)?;
            transaction.accepted_ingress_revision = Some(ingress_revision);
            advance_transaction(&mut transaction, SupervisorTakeoverState::IngressAdopted)?;
            clear_takeover_admission_drain(&transaction)?;
            advance_transaction(&mut transaction, SupervisorTakeoverState::Accepted)?;
            Ok(outcome_from_transaction(&transaction))
        }
        Err(error) => {
            transaction.failure = Some(error.clone());
            transaction.revision = transaction.revision.saturating_add(1);
            transaction.updated_at = current_timestamp();
            write_transaction(&transaction)?;
            write_admission_drain(&transaction)?;
            Err(format!(
                "runtime_host_supervisor_takeover_resume_failed:{}:{error}",
                transaction.transaction_id
            ))
        }
    }
}

fn execute_takeover(
    plan: &SupervisorTakeoverPlan,
    source_identity: &RecordedProcessIdentity,
    transaction: &mut SupervisorTakeoverTransaction,
) -> Result<(), String> {
    advance_transaction(transaction, SupervisorTakeoverState::CensusStable)?;
    write_admission_drain(transaction)?;
    advance_transaction(transaction, SupervisorTakeoverState::AdmissionDraining)?;

    revalidate_source(
        plan,
        source_identity,
        takeover_admission_owner_id(transaction),
        transaction.parent_admission_transaction_id.is_some(),
    )?;
    let process = VerifiedProcessTermination::open(source_identity)?
        .ok_or_else(|| "blocked_selected_process_missing_before_signal".to_string())?;
    revalidate_source(
        plan,
        source_identity,
        takeover_admission_owner_id(transaction),
        transaction.parent_admission_transaction_id.is_some(),
    )?;
    advance_transaction(transaction, SupervisorTakeoverState::SourceRetiring)?;

    process.signal(VerifiedProcessSignal::Terminate)?;
    wait_for_process_exit(&process, SOURCE_EXIT_TIMEOUT)?;
    if process.is_running()? {
        process.signal(VerifiedProcessSignal::Kill)?;
        wait_for_process_exit(&process, SOURCE_EXIT_TIMEOUT)?;
    }
    if process.is_running()? {
        return Err("source_exit_timeout".to_string());
    }
    advance_transaction(transaction, SupervisorTakeoverState::SourceAbsent)?;

    crate::session_supervisor::start_runtime_host_supervisor_once()?;
    advance_transaction(transaction, SupervisorTakeoverState::SupervisorStarting)?;
    let (replacement_pid, ingress_revision) = wait_for_replacement(&plan.selected_backend)?;
    transaction.replacement_pid = Some(replacement_pid);
    advance_transaction(transaction, SupervisorTakeoverState::ReplacementReady)?;
    transaction.accepted_ingress_revision = Some(ingress_revision);
    advance_transaction(transaction, SupervisorTakeoverState::IngressAdopted)?;
    clear_takeover_admission_drain(transaction)?;
    advance_transaction(transaction, SupervisorTakeoverState::Accepted)
}

fn validate_selected_process(
    selected: &RuntimeHostBackend,
    recorded: &RecordedProcessIdentity,
    blockers: &mut Vec<SupervisorTakeoverBlocker>,
) {
    if recorded.pid != selected.pid || recorded.start_token.trim().is_empty() {
        push_blocker(
            blockers,
            "blocked_selected_process_identity",
            "The selected PID and recorded process identity disagree.",
        );
        return;
    }
    match crate::process_identity::observe_process(recorded.pid) {
        ProcessObservation::Observed(observed)
            if observed.start_token.as_deref() == Some(recorded.start_token.as_str()) => {}
        ProcessObservation::Observed(_) => push_blocker(
            blockers,
            "blocked_identity_changed",
            "The selected PID now names another process instance.",
        ),
        ProcessObservation::Missing => push_blocker(
            blockers,
            "blocked_selected_process_missing",
            "The selected process is absent; P147 restart adoption should reconcile it.",
        ),
        ProcessObservation::Failed { .. } => push_blocker(
            blockers,
            "blocked_process_observation_failed",
            "The selected process could not be observed safely.",
        ),
    }
    let proc_executable = PathBuf::from(format!("/proc/{}/exe", selected.pid));
    if sha256_file(&proc_executable).as_deref() != Ok(selected.binary_sha256.as_str()) {
        push_blocker(
            blockers,
            "blocked_selected_binary_mismatch",
            "The selected process executable digest does not match ingress.",
        );
    }
}

fn validate_supervisor(
    selected: &RuntimeHostBackend,
    supervisor: &RuntimeHostSupervisorObservation,
    already_supervised: bool,
    blockers: &mut Vec<SupervisorTakeoverBlocker>,
) {
    if supervisor.load_state != "loaded" {
        push_blocker(
            blockers,
            "blocked_supervisor_not_loaded",
            "The shared runtime-host unit is not loaded.",
        );
    }
    if !matches!(
        supervisor.unit_file_state.as_str(),
        "enabled" | "enabled-runtime" | "static"
    ) {
        push_blocker(
            blockers,
            "blocked_supervisor_not_enabled",
            "The shared runtime-host unit is not enabled.",
        );
    }
    if supervisor.manifests.is_empty() {
        push_blocker(
            blockers,
            "blocked_supervisor_empty",
            "No supervised lane manifests are configured.",
        );
    }
    // The installer may invoke takeover from its build artifact after it has
    // copied the same bytes into the immutable selected generation. P147
    // validates the invoking digest separately, so path equality here would
    // reject an exact candidate solely because its pathname differs.
    if supervisor
        .manifests
        .iter()
        .any(|manifest| manifest.executable_sha256 != selected.binary_sha256)
    {
        push_blocker(
            blockers,
            "blocked_supervisor_binary_mismatch",
            "Supervisor manifests do not match the selected executable generation.",
        );
    }
    if !already_supervised && supervisor.main_pid.is_some() {
        push_blocker(
            blockers,
            "blocked_supervisor_other_main_pid",
            "The shared supervisor already owns another live process.",
        );
    }
}

fn validate_runtime_conflicts(
    selected: &RuntimeHostBackend,
    supervisor: &RuntimeHostSupervisorObservation,
    census: &StableRuntimeCensus,
    listener_ports: &BTreeSet<u16>,
    already_supervised: bool,
    accepted_upgrade_transition: bool,
    blockers: &mut Vec<SupervisorTakeoverBlocker>,
) {
    if !already_supervised && !accepted_upgrade_transition && !browserless_census_is_safe(census) {
        let live = census
            .records
            .iter()
            .filter(|record| {
                !matches!(
                    record.classification,
                    RuntimeClassification::IdleDaemon
                        | RuntimeClassification::StaleMetadata
                        | RuntimeClassification::ExternalObserved
                        | RuntimeClassification::ManualPreserveOnly
                )
            })
            .map(|record| record.logical_browser_id.as_str())
            .collect::<Vec<_>>()
            .join(",");
        push_blocker(
            blockers,
            "blocked_live_owned_browser",
            &format!("Runtime census did not prove browserless ownership: {live}"),
        );
    }
    for manifest in &supervisor.manifests {
        if supervisor
            .reachable_stream_ports
            .contains(&manifest.stream_port)
            && !listener_ports.contains(&manifest.stream_port)
        {
            push_blocker(
                blockers,
                "blocked_unrelated_port_owner",
                &format!(
                    "Configured stream port {} is not proven to be owned by selected PID {}.",
                    manifest.stream_port, selected.pid
                ),
            );
        }
    }
}

fn validate_authoritative_listener_inventory(
    selected: &RuntimeHostBackend,
    inventory: &serde_json::Value,
    blockers: &mut Vec<SupervisorTakeoverBlocker>,
) {
    if inventory
        .get("available")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        push_blocker(
            blockers,
            "blocked_listener_inventory_unavailable",
            "The production daemon listener census is unavailable.",
        );
        return;
    }
    let listeners = inventory
        .get("listeners")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let exact = listeners.len() == 1
        && listeners[0].get("pid").and_then(serde_json::Value::as_u64)
            == Some(u64::from(selected.pid))
        && listeners[0]
            .get("socketPath")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|path| path.ends_with("/runtime-host.sock"))
        && listeners[0]
            .get("socketIdentity")
            .and_then(serde_json::Value::as_str)
            == Some(selected.socket_identity.as_str());
    if !exact {
        push_blocker(
            blockers,
            "blocked_runtime_listener_multiplicity",
            "Production must have exactly one authoritative runtime-host listener bound to the selected process and socket identity.",
        );
    }
}

fn browserless_census_is_safe(census: &StableRuntimeCensus) -> bool {
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

fn revalidate_source(
    plan: &SupervisorTakeoverPlan,
    source_identity: &RecordedProcessIdentity,
    transaction_id: &str,
    accepted_upgrade_transition: bool,
) -> Result<(), String> {
    let registry =
        RuntimeHostIngressRepository::new(RuntimeHostIngressRepository::default_path()).load()?;
    if registry.revision != plan.ingress_revision
        || registry.selected_backend() != &plan.selected_backend
        || registry.active_transaction_id.is_some()
        || registry.candidate_backend().is_some()
        || registry.boot_epoch_status() != BootEpochStatus::Current
    {
        return Err("blocked_ingress_changed_before_signal".to_string());
    }
    let recorded = read_selected_process_identity(&plan.selected_backend)?;
    if &recorded != source_identity
        || !crate::process_identity::recorded_process_is_running(source_identity)?
    {
        return Err("blocked_identity_changed_before_signal".to_string());
    }
    let census = crate::workstation_install::collect_stable_host_runtime_census()?;
    if !census.activation_allowed
        || (!accepted_upgrade_transition && !browserless_census_is_safe(&census))
    {
        return Err("blocked_runtime_census_changed_before_signal".to_string());
    }
    let ports = listener_ports_for_pid(plan.selected_backend.pid)?;
    let supervisor = crate::session_supervisor::runtime_host_supervisor_observation()?;
    if digest_json(&supervisor.manifests)? != digest_json(&plan.supervisor.manifests)? {
        return Err("blocked_supervisor_manifest_changed_before_signal".to_string());
    }
    if supervisor.manifests.iter().any(|manifest| {
        supervisor
            .reachable_stream_ports
            .contains(&manifest.stream_port)
            && !ports.contains(&manifest.stream_port)
    }) {
        return Err("blocked_listener_identity_changed_before_signal".to_string());
    }
    let drain: crate::runtime_adoption::RuntimeAdmissionDrain = serde_json::from_slice(
        &fs::read(crate::runtime_adoption::runtime_admission_drain_path()?)
            .map_err(|error| format!("runtime_admission_drain_unreadable:{error}"))?,
    )
    .map_err(|error| format!("runtime_admission_drain_invalid:{error}"))?;
    if drain.transaction_id != transaction_id {
        return Err("blocked_admission_drain_changed_before_signal".to_string());
    }
    Ok(())
}

fn wait_for_replacement(source_backend: &RuntimeHostBackend) -> Result<(u32, u64), String> {
    let deadline = Instant::now() + REPLACEMENT_READY_TIMEOUT;
    while Instant::now() < deadline {
        let supervisor = crate::session_supervisor::runtime_host_supervisor_observation();
        let registry =
            RuntimeHostIngressRepository::new(RuntimeHostIngressRepository::default_path()).load();
        if let (Ok(supervisor), Ok(registry)) = (supervisor, registry) {
            if let Some(pid) = supervisor.main_pid {
                let selected = registry.selected_backend();
                let ports_ready = supervisor.manifests.iter().all(|manifest| {
                    supervisor
                        .reachable_stream_ports
                        .contains(&manifest.stream_port)
                });
                if pid != source_backend.pid
                    && supervisor.active_state == "active"
                    && supervisor.sub_state == "running"
                    && supervisor
                        .manifests
                        .iter()
                        .all(|manifest| manifest.executable_sha256 == selected.binary_sha256)
                    && ports_ready
                    && selected.pid == pid
                    && selected.generation_id == source_backend.generation_id
                    && selected.binary_sha256 == source_backend.binary_sha256
                    && registry.boot_epoch_status() == BootEpochStatus::Current
                    && registry.active_transaction_id.is_none()
                    && registry.candidate_backend().is_none()
                {
                    return Ok((pid, registry.revision));
                }
            }
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    Err("replacement_readiness_or_ingress_adoption_failed".to_string())
}

fn active_coordination_blocker(
    parent_admission_transaction_id: Option<&str>,
) -> Result<Option<SupervisorTakeoverBlocker>, String> {
    let drain_path = crate::runtime_adoption::runtime_admission_drain_path()?;
    if drain_path.exists() {
        let drain: crate::runtime_adoption::RuntimeAdmissionDrain = serde_json::from_slice(
            &fs::read(&drain_path)
                .map_err(|error| format!("runtime_admission_drain_unreadable:{error}"))?,
        )
        .map_err(|error| format!("runtime_admission_drain_invalid:{error}"))?;
        if admission_drain_conflicts(&drain, parent_admission_transaction_id) {
            return Ok(Some(SupervisorTakeoverBlocker {
                code: "blocked_active_admission_drain".to_string(),
                message: "Another runtime transaction owns the admission drain.".to_string(),
            }));
        }
    }
    if let Some(home) = dirs::home_dir() {
        let workstation_lock = home.join(".agent-browser/convergence/workstation.lock");
        if workstation_transaction_is_foreign(&workstation_lock, std::process::id()) {
            return Ok(Some(SupervisorTakeoverBlocker {
                code: "blocked_active_workstation_transaction".to_string(),
                message: "A workstation convergence transaction is active.".to_string(),
            }));
        }
    }
    let path = transaction_path()?;
    if path.exists() {
        let transaction: SupervisorTakeoverTransaction = serde_json::from_slice(
            &fs::read(&path)
                .map_err(|error| format!("supervisor_takeover_transaction_unreadable:{error}"))?,
        )
        .map_err(|error| format!("supervisor_takeover_transaction_invalid:{error}"))?;
        if !transaction.state.is_terminal() {
            return Ok(Some(SupervisorTakeoverBlocker {
                code: "blocked_active_takeover_transaction".to_string(),
                message: format!(
                    "Supervisor takeover {} requires recovery at revision {}.",
                    transaction.transaction_id, transaction.revision
                ),
            }));
        }
    }
    Ok(None)
}

fn admission_drain_conflicts(
    drain: &crate::runtime_adoption::RuntimeAdmissionDrain,
    parent_admission_transaction_id: Option<&str>,
) -> bool {
    parent_admission_transaction_id != Some(drain.transaction_id.as_str())
}

fn workstation_transaction_is_foreign(path: &Path, current_pid: u32) -> bool {
    if !path.exists() {
        return false;
    }
    fs::read_to_string(path)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        != Some(current_pid)
}

fn listener_ports_for_pid(pid: u32) -> Result<BTreeSet<u16>, String> {
    let mut socket_inodes = BTreeSet::new();
    let fd_dir = PathBuf::from(format!("/proc/{pid}/fd"));
    for entry in fs::read_dir(&fd_dir)
        .map_err(|error| format!("runtime_host_listener_fd_observation_failed:{error}"))?
    {
        let entry = entry.map_err(|error| format!("runtime_host_listener_fd_invalid:{error}"))?;
        let target = match fs::read_link(entry.path()) {
            Ok(target) => target,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!("runtime_host_listener_fd_target_failed:{error}"));
            }
        };
        let target = target.to_string_lossy();
        if let Some(inode) = target
            .strip_prefix("socket:[")
            .and_then(|value| value.strip_suffix(']'))
        {
            socket_inodes.insert(inode.to_string());
        }
    }
    let mut ports = BTreeSet::new();
    for table in ["/proc/net/tcp", "/proc/net/tcp6"] {
        let body = fs::read_to_string(table)
            .map_err(|error| format!("runtime_host_listener_table_unreadable:{table}:{error}"))?;
        for line in body.lines().skip(1) {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() < 10 || fields[3] != "0A" || !socket_inodes.contains(fields[9]) {
                continue;
            }
            let Some(port_hex) = fields[1].rsplit(':').next() else {
                continue;
            };
            if let Ok(port) = u16::from_str_radix(port_hex, 16) {
                ports.insert(port);
            }
        }
    }
    Ok(ports)
}

fn read_selected_process_identity(
    selected: &RuntimeHostBackend,
) -> Result<RecordedProcessIdentity, String> {
    let path = selected.socket_dir.join("runtime-host.identity.json");
    serde_json::from_slice(
        &fs::read(&path)
            .map_err(|error| format!("selected_runtime_host_identity_unreadable:{error}"))?,
    )
    .map_err(|error| format!("selected_runtime_host_identity_invalid:{error}"))
}

fn current_executable_sha256() -> Result<String, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("current_executable_unavailable:{error}"))?;
    sha256_file(&executable)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| {
        format!(
            "runtime_host_executable_unreadable:{}:{error}",
            path.display()
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("runtime_host_executable_hash_failed:{error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn digest_json(value: &impl Serialize) -> Result<String, String> {
    let body = serde_json::to_vec(value)
        .map_err(|error| format!("supervisor_takeover_digest_encoding_failed:{error}"))?;
    Ok(format!("{:x}", Sha256::digest(body)))
}

fn require_plan_digest(
    plan: &SupervisorTakeoverPlan,
    expected_plan_digest: &str,
) -> Result<(), String> {
    if plan.plan_digest == expected_plan_digest.to_ascii_lowercase() {
        Ok(())
    } else {
        Err(format!(
            "runtime_host_supervisor_takeover_plan_changed:expected={expected_plan_digest}:current={}",
            plan.plan_digest
        ))
    }
}

fn require_ready_plan(plan: &SupervisorTakeoverPlan) -> Result<(), String> {
    if plan.disposition == SupervisorTakeoverDisposition::ReadyForTakeover {
        Ok(())
    } else {
        Err(format!(
            "runtime_host_supervisor_takeover_blocked:{}",
            plan.blockers
                .iter()
                .map(|blocker| blocker.code.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ))
    }
}

fn outcome_without_transaction(plan: &SupervisorTakeoverPlan) -> SupervisorTakeoverOutcome {
    SupervisorTakeoverOutcome {
        schema_version: OUTCOME_SCHEMA_VERSION.to_string(),
        state: "already_supervised".to_string(),
        transaction_id: None,
        transaction_revision: None,
        plan_digest: plan.plan_digest.clone(),
        source_pid: plan.selected_backend.pid,
        replacement_pid: Some(plan.selected_backend.pid),
        ingress_revision: plan.ingress_revision,
        browser_launched: false,
    }
}

fn outcome_from_transaction(
    transaction: &SupervisorTakeoverTransaction,
) -> SupervisorTakeoverOutcome {
    SupervisorTakeoverOutcome {
        schema_version: OUTCOME_SCHEMA_VERSION.to_string(),
        state: "accepted".to_string(),
        transaction_id: Some(transaction.transaction_id.clone()),
        transaction_revision: Some(transaction.revision),
        plan_digest: transaction.plan_digest.clone(),
        source_pid: transaction.source_backend.pid,
        replacement_pid: transaction.replacement_pid,
        ingress_revision: transaction.accepted_ingress_revision.unwrap_or_default(),
        browser_launched: false,
    }
}

fn push_blocker(blockers: &mut Vec<SupervisorTakeoverBlocker>, code: &str, message: &str) {
    if blockers.iter().any(|blocker| blocker.code == code) {
        return;
    }
    blockers.push(SupervisorTakeoverBlocker {
        code: code.to_string(),
        message: message.to_string(),
    });
}

fn boot_epoch_label(status: BootEpochStatus) -> &'static str {
    match status {
        BootEpochStatus::Current => "current",
        BootEpochStatus::Prior => "prior",
        BootEpochStatus::Missing => "missing",
        BootEpochStatus::Unavailable => "unavailable",
    }
}

fn transaction_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("AGENT_BROWSER_RUNTIME_HOST_SUPERVISOR_TAKEOVER_STATE") {
        return Ok(PathBuf::from(path));
    }
    dirs::home_dir()
        .map(|home| {
            home.join(".agent-browser/runtime-adoption")
                .join("supervisor-takeover.json")
        })
        .ok_or_else(|| "supervisor_takeover_home_unavailable".to_string())
}

fn acquire_takeover_lock() -> Result<File, String> {
    let path = transaction_path()?.with_extension("json.lock");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("supervisor_takeover_lock_directory_failed:{error}"))?;
        set_private_directory(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|error| format!("supervisor_takeover_lock_open_failed:{error}"))?;
    file.try_lock()
        .map_err(|error| format!("supervisor_takeover_lock_unavailable:{error}"))?;
    Ok(file)
}

fn write_transaction(transaction: &SupervisorTakeoverTransaction) -> Result<(), String> {
    write_private_json_atomic(&transaction_path()?, transaction)
}

fn read_transaction() -> Result<SupervisorTakeoverTransaction, String> {
    let path = transaction_path()?;
    let transaction: SupervisorTakeoverTransaction = serde_json::from_slice(
        &fs::read(&path)
            .map_err(|error| format!("supervisor_takeover_transaction_unreadable:{error}"))?,
    )
    .map_err(|error| format!("supervisor_takeover_transaction_invalid:{error}"))?;
    if transaction.schema_version != TRANSACTION_SCHEMA_VERSION {
        return Err("supervisor_takeover_transaction_schema_unsupported".to_string());
    }
    Ok(transaction)
}

fn advance_transaction(
    transaction: &mut SupervisorTakeoverTransaction,
    state: SupervisorTakeoverState,
) -> Result<(), String> {
    transaction.revision = transaction
        .revision
        .checked_add(1)
        .ok_or_else(|| "supervisor_takeover_revision_exhausted".to_string())?;
    transaction.state = state;
    transaction.updated_at = current_timestamp();
    write_transaction(transaction)?;
    let drain_path = crate::runtime_adoption::runtime_admission_drain_path()?;
    if drain_path.exists() {
        write_admission_drain(transaction)?;
    }
    Ok(())
}

fn write_admission_drain(transaction: &SupervisorTakeoverTransaction) -> Result<(), String> {
    let path = crate::runtime_adoption::runtime_admission_drain_path()?;
    if path.exists() {
        let existing: crate::runtime_adoption::RuntimeAdmissionDrain = serde_json::from_slice(
            &fs::read(&path)
                .map_err(|error| format!("runtime_admission_drain_unreadable:{error}"))?,
        )
        .map_err(|error| format!("runtime_admission_drain_invalid:{error}"))?;
        if existing.transaction_id != takeover_admission_owner_id(transaction) {
            return Err("blocked_active_admission_drain".to_string());
        }
        if transaction.parent_admission_transaction_id.is_some() {
            return Ok(());
        }
    } else if transaction.parent_admission_transaction_id.is_some() {
        return Err("parent_admission_drain_missing".to_string());
    }
    write_private_json_atomic(
        &path,
        &crate::runtime_adoption::RuntimeAdmissionDrain {
            schema_version: crate::runtime_adoption::RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
            transaction_id: transaction.transaction_id.clone(),
            candidate_generation_id: transaction.source_backend.generation_id.clone(),
            transaction_revision: transaction.revision,
            recorded_at: current_timestamp(),
        },
    )
}

fn takeover_admission_owner_id(transaction: &SupervisorTakeoverTransaction) -> &str {
    transaction
        .parent_admission_transaction_id
        .as_deref()
        .unwrap_or(transaction.transaction_id.as_str())
}

fn clear_takeover_admission_drain(
    transaction: &SupervisorTakeoverTransaction,
) -> Result<(), String> {
    if transaction.parent_admission_transaction_id.is_some() {
        return Ok(());
    }
    clear_owned_admission_drain(&transaction.transaction_id)
}

fn clear_owned_admission_drain(transaction_id: &str) -> Result<(), String> {
    let path = crate::runtime_adoption::runtime_admission_drain_path()?;
    if !path.exists() {
        return Ok(());
    }
    let existing: crate::runtime_adoption::RuntimeAdmissionDrain = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("runtime_admission_drain_unreadable:{error}"))?,
    )
    .map_err(|error| format!("runtime_admission_drain_invalid:{error}"))?;
    if existing.transaction_id != transaction_id {
        return Err("runtime_admission_drain_owner_changed".to_string());
    }
    fs::remove_file(&path).map_err(|error| format!("runtime_admission_drain_clear_failed:{error}"))
}

fn write_private_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "supervisor_takeover_state_parent_missing".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("supervisor_takeover_state_directory_failed:{error}"))?;
    set_private_directory(parent)?;
    let staged = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let body = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("supervisor_takeover_state_encoding_failed:{error}"))?;
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staged)
            .map_err(|error| format!("supervisor_takeover_state_stage_failed:{error}"))?;
        file.write_all(&body)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("supervisor_takeover_state_persist_failed:{error}"))?;
        set_private_file(&staged)?;
        fs::rename(&staged, path)
            .map_err(|error| format!("supervisor_takeover_state_commit_failed:{error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    result
}

fn wait_for_process_exit(
    process: &VerifiedProcessTermination,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while process.is_running()? && Instant::now() < deadline {
        std::thread::sleep(POLL_INTERVAL);
    }
    Ok(())
}

fn current_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string()
        })
}

#[cfg(unix)]
fn set_private_file(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("supervisor_takeover_state_permissions_failed:{error}"))
}

#[cfg(not(unix))]
fn set_private_file(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn set_private_directory(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("supervisor_takeover_directory_permissions_failed:{error}"))
}

#[cfg(not(unix))]
fn set_private_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_adoption::{RuntimeCensusRecord, RuntimeDisposition};
    use crate::session_supervisor::{SessionSupervisorManifest, SessionSupervisorProvenance};

    fn census(classifications: &[RuntimeClassification]) -> StableRuntimeCensus {
        StableRuntimeCensus {
            schema_version: crate::runtime_adoption::RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
            digest: "a".repeat(64),
            registry_revision: 1,
            activation_allowed: true,
            records: classifications
                .iter()
                .enumerate()
                .map(|(index, classification)| RuntimeCensusRecord {
                    logical_browser_id: format!("runtime-{index}"),
                    session_names: vec![format!("session-{index}")],
                    profile_identity_digest: "b".repeat(64),
                    observed_sources: Vec::new(),
                    classification: *classification,
                    disposition: RuntimeDisposition::ManualPreservation,
                    reason_codes: Vec::new(),
                })
                .collect(),
        }
    }

    fn selected_backend(socket_dir: PathBuf) -> RuntimeHostBackend {
        RuntimeHostBackend {
            topology: RuntimeHostTopology::SingleHost,
            generation_id: "generation-one".to_string(),
            socket_dir,
            binary_sha256: "a".repeat(64),
            host_id: "runtime-host:41".to_string(),
            pid: 41,
            socket_identity: "unix:one".to_string(),
        }
    }

    fn supervisor(reachable_stream_ports: Vec<u16>) -> RuntimeHostSupervisorObservation {
        RuntimeHostSupervisorObservation {
            unit: "agent-browser-runtime-host.service".to_string(),
            manifests: vec![SessionSupervisorManifest {
                schema_version: "agent-browser.session-supervisor.v1".to_string(),
                session: "fixture".to_string(),
                executable_path: "/fixture/agent-browser".to_string(),
                executable_sha256: "a".repeat(64),
                stream_port: 39717,
                runtime_profile: None,
                service_config_path: None,
                provenance: SessionSupervisorProvenance {
                    package_version: "0.28.0".to_string(),
                    installed_at: "2026-09-01T00:00:00Z".to_string(),
                    installed_by: "takeover regression".to_string(),
                },
            }],
            load_state: "loaded".to_string(),
            unit_file_state: "enabled".to_string(),
            active_state: "inactive".to_string(),
            sub_state: "dead".to_string(),
            result: "success".to_string(),
            restart_count: 0,
            main_pid: None,
            executable_matches: true,
            reachable_stream_ports,
        }
    }

    #[test]
    fn free_manifest_port_is_safe_but_reachable_non_selected_port_blocks() {
        let socket_dir = std::env::temp_dir().join(format!(
            "agent-browser-takeover-free-port-{}",
            uuid::Uuid::new_v4()
        ));
        let selected = selected_backend(socket_dir);
        let safe_census = census(&[]);
        let mut blockers = Vec::new();
        validate_runtime_conflicts(
            &selected,
            &supervisor(Vec::new()),
            &safe_census,
            &BTreeSet::new(),
            false,
            false,
            &mut blockers,
        );
        assert!(blockers.is_empty(), "free configured port must be safe");

        validate_runtime_conflicts(
            &selected,
            &supervisor(vec![39717]),
            &safe_census,
            &BTreeSet::new(),
            false,
            false,
            &mut blockers,
        );
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].code, "blocked_unrelated_port_owner");
    }

    #[test]
    fn already_supervised_runtime_still_rejects_unrelated_configured_port_owner() {
        let selected = selected_backend(PathBuf::from("/tmp/runtime-host"));
        let mut blockers = Vec::new();

        validate_runtime_conflicts(
            &selected,
            &supervisor(vec![39717]),
            &census(&[RuntimeClassification::CooperativeLiveOwner]),
            &BTreeSet::new(),
            true,
            false,
            &mut blockers,
        );

        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].code, "blocked_unrelated_port_owner");
    }

    #[test]
    fn accepted_upgrade_can_supervise_finalized_retained_browsers() {
        let selected = selected_backend(PathBuf::from("/tmp/runtime-host"));
        let mut blockers = Vec::new();

        validate_runtime_conflicts(
            &selected,
            &supervisor(Vec::new()),
            &census(&[
                RuntimeClassification::CooperativeLiveOwner,
                RuntimeClassification::ManualPreserveOnly,
            ]),
            &BTreeSet::new(),
            false,
            true,
            &mut blockers,
        );

        assert!(blockers.is_empty());
    }

    #[test]
    fn supervisor_manifest_digest_matches_selected_even_when_invoker_path_differs() {
        let selected = selected_backend(PathBuf::from("/tmp/runtime-host"));
        let mut observation = supervisor(Vec::new());
        observation.executable_matches = false;
        let mut blockers = Vec::new();

        validate_supervisor(&selected, &observation, false, &mut blockers);

        assert!(blockers.is_empty());
    }

    #[test]
    fn authoritative_listener_validation_rejects_multiple_runtime_hosts() {
        let selected = selected_backend(PathBuf::from("/tmp/runtime-host"));
        let exact = serde_json::json!({
            "available": true,
            "listeners": [{
                "pid": selected.pid,
                "socketPath": "/run/user/1000/agent-browser/runtime-hosts/current/runtime-host.sock",
                "socketIdentity": selected.socket_identity,
            }]
        });
        let mut blockers = Vec::new();
        validate_authoritative_listener_inventory(&selected, &exact, &mut blockers);
        assert!(blockers.is_empty());

        let multiple = serde_json::json!({
            "available": true,
            "listeners": [
                {
                    "pid": selected.pid,
                    "socketPath": "/run/user/1000/agent-browser/runtime-hosts/current/runtime-host.sock",
                    "socketIdentity": selected.socket_identity,
                },
                {
                    "pid": selected.pid + 1,
                    "socketPath": "/run/user/1000/agent-browser/runtime-hosts/old/runtime-host.sock",
                    "socketIdentity": "unix:old",
                }
            ]
        });
        validate_authoritative_listener_inventory(&selected, &multiple, &mut blockers);
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].code, "blocked_runtime_listener_multiplicity");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn accepted_upgrade_reconciles_only_proven_absent_runtime_host_authority() {
        let namespace = std::env::temp_dir().join(format!(
            "agent-browser-absent-runtime-authority-{}",
            uuid::Uuid::new_v4()
        ));
        let selected = namespace.join("selected");
        let absent = namespace.join("absent");
        let unknown = namespace.join("unknown");
        fs::create_dir_all(&selected).unwrap();
        fs::create_dir_all(&absent).unwrap();
        fs::create_dir_all(&unknown).unwrap();

        let absent_identity = RecordedProcessIdentity {
            pid: u32::MAX,
            start_token: "proven-absent".to_string(),
            executable_path: Some("/missing/agent-browser".to_string()),
            browser_family: None,
        };
        fs::write(
            absent.join("runtime-host.identity.json"),
            serde_json::to_vec(&absent_identity).unwrap(),
        )
        .unwrap();
        for name in [
            "runtime-host.json",
            "runtime-host.pid",
            "runtime-host.sha256",
            "runtime-host.sock",
            "runtime-host.token",
            "runtime-host.version",
            "fixture.stream",
        ] {
            fs::write(absent.join(name), "stale").unwrap();
        }
        fs::write(absent.join("retained.engine"), "preserve").unwrap();
        fs::write(unknown.join("runtime-host.identity.json"), "invalid").unwrap();
        fs::write(selected.join("runtime-host.identity.json"), "selected").unwrap();

        let reconciled =
            reconcile_absent_runtime_host_authority_records(&namespace, &selected).unwrap();

        assert_eq!(reconciled, vec![absent.clone()]);
        assert!(absent.join("retained.engine").is_file());
        assert!(!absent.join("runtime-host.identity.json").exists());
        assert!(!absent.join("runtime-host.sock").exists());
        assert!(!absent.join("fixture.stream").exists());
        assert!(unknown.join("runtime-host.identity.json").is_file());
        assert!(selected.join("runtime-host.identity.json").is_file());

        fs::remove_dir_all(namespace).unwrap();
    }

    #[test]
    fn nested_takeover_accepts_only_its_parent_admission_drain() {
        let drain = crate::runtime_adoption::RuntimeAdmissionDrain {
            schema_version: crate::runtime_adoption::RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
            transaction_id: "workstation-parent".to_string(),
            candidate_generation_id: "candidate".to_string(),
            transaction_revision: 12,
            recorded_at: "2026-09-09T00:00:00Z".to_string(),
        };

        assert!(!admission_drain_conflicts(
            &drain,
            Some("workstation-parent")
        ));
        assert!(admission_drain_conflicts(
            &drain,
            Some("another-transaction")
        ));
        assert!(admission_drain_conflicts(&drain, None));
    }

    #[test]
    fn browserless_policy_accepts_only_non_owned_runtime_classes() {
        let safe = [
            RuntimeClassification::IdleDaemon,
            RuntimeClassification::StaleMetadata,
            RuntimeClassification::ExternalObserved,
            RuntimeClassification::ManualPreserveOnly,
        ];
        assert!(browserless_census_is_safe(&census(&safe)));
        for classification in [
            RuntimeClassification::CooperativeLiveOwner,
            RuntimeClassification::OrphanAdoptable,
            RuntimeClassification::ConflictingOwner,
            RuntimeClassification::InsufficientEvidence,
        ] {
            assert!(!browserless_census_is_safe(&census(&[classification])));
        }
        let mut ambiguous = census(&safe);
        ambiguous.activation_allowed = false;
        assert!(!browserless_census_is_safe(&ambiguous));
    }

    #[test]
    fn plan_digest_changes_when_effect_relevant_evidence_changes() {
        #[derive(Serialize)]
        struct Evidence<'a> {
            pid: u32,
            socket_identity: &'a str,
        }
        let first = digest_json(&Evidence {
            pid: 41,
            socket_identity: "unix:one",
        })
        .unwrap();
        let second = digest_json(&Evidence {
            pid: 42,
            socket_identity: "unix:one",
        })
        .unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn workstation_lock_owned_by_this_installer_allows_supervisor_completion() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-takeover-workstation-lock-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let lock = root.join("workstation.lock");
        fs::write(&lock, format!("{}\n", std::process::id())).unwrap();

        assert!(!workstation_transaction_is_foreign(
            &lock,
            std::process::id()
        ));
        assert!(workstation_transaction_is_foreign(
            &lock,
            std::process::id().saturating_add(1)
        ));
        fs::write(&lock, "unknown-owner\n").unwrap();
        assert!(workstation_transaction_is_foreign(
            &lock,
            std::process::id()
        ));

        fs::remove_dir_all(root).unwrap();
    }
}
