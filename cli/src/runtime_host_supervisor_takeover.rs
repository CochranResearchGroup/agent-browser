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
        &mut blockers,
    );

    if let Some(blocker) = active_coordination_blocker()? {
        blockers.push(blocker);
    }

    let p147_capability_ready =
        current_executable_sha256().is_ok_and(|digest| digest == selected.binary_sha256);
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
    require_plan_digest(&initial, expected_plan_digest)?;
    if initial.disposition == SupervisorTakeoverDisposition::AlreadySupervised {
        return Ok(outcome_without_transaction(&initial));
    }
    require_ready_plan(&initial)?;

    let _lock = acquire_takeover_lock()?;
    let plan = plan_supervisor_takeover()?;
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
            clear_owned_admission_drain(&transaction.transaction_id)?;
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
    if drain.transaction_id != transaction.transaction_id {
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
            clear_owned_admission_drain(&transaction.transaction_id)?;
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

    revalidate_source(plan, source_identity, &transaction.transaction_id)?;
    let process = VerifiedProcessTermination::open(source_identity)?
        .ok_or_else(|| "blocked_selected_process_missing_before_signal".to_string())?;
    revalidate_source(plan, source_identity, &transaction.transaction_id)?;
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
    clear_owned_admission_drain(&transaction.transaction_id)?;
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
    if !supervisor.executable_matches
        || supervisor
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
    blockers: &mut Vec<SupervisorTakeoverBlocker>,
) {
    if already_supervised {
        return;
    }
    if !browserless_census_is_safe(census) {
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
    if !browserless_census_is_safe(&census) {
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
                    && supervisor.executable_matches
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

fn active_coordination_blocker() -> Result<Option<SupervisorTakeoverBlocker>, String> {
    let drain_path = crate::runtime_adoption::runtime_admission_drain_path()?;
    if drain_path.exists() {
        return Ok(Some(SupervisorTakeoverBlocker {
            code: "blocked_active_admission_drain".to_string(),
            message: "Another runtime transaction owns the admission drain.".to_string(),
        }));
    }
    if let Some(home) = dirs::home_dir() {
        if home
            .join(".agent-browser/convergence/workstation.lock")
            .exists()
        {
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
        if existing.transaction_id != transaction.transaction_id {
            return Err("blocked_active_admission_drain".to_string());
        }
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
            &mut blockers,
        );
        assert!(blockers.is_empty(), "free configured port must be safe");

        validate_runtime_conflicts(
            &selected,
            &supervisor(vec![39717]),
            &safe_census,
            &BTreeSet::new(),
            false,
            &mut blockers,
        );
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].code, "blocked_unrelated_port_owner");
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
}
