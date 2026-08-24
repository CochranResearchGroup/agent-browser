//! Source-free Linux workstation installation.
//!
//! The workstation installer stages the release binary, support assets,
//! manifests, and rendered unit templates in a sealed generation before
//! atomically selecting it. Stable command and unit links resolve through that
//! selector without relying on a repository checkout or package manager at
//! runtime. Host provisioning stops with a resumable status when a fresh login
//! is required. Runtime reconciliation consumes the selected generation and
//! activates services only after canonical route projection and final doctor
//! readiness. Real-host apply requires a stable two-round runtime census before
//! unit quiescence or payload staging. Preflight also requires enough free disk
//! capacity before sudo or payload mutation begins. Fresh install and upgrade
//! share one durable transaction: census precedes candidate staging, host gates
//! precede admission drain, runtime ownership is receipted before selector
//! commit, and failures reverse to the prior selected generation when proven.
//! A first upgrade from the legacy mutable layout imports and seals the exact
//! installed binary, support tree, and unit files as the rollback generation
//! before converting stable entrypoints to generation-backed symlinks. Unit
//! types introduced after that legacy install remain inert until candidate
//! selection. On Linux, exact daemon identities are reconciled after that
//! controlled relocation only when the process start token and imported binary
//! digest still match. A schema-v1 live daemon without an owner record can
//! bootstrap the first receipted owner only from the explicit census reason
//! and only after exact daemon revocation. Historical presentation identifiers
//! remain scoped to their browser owner during census joins.
//! Failed reconciliation restores the exact prior active state of managed user
//! units and writes a private diagnostic receipt.
//! Operator recovery closes an exact retained admission drain only after the
//! old selector, candidate process absence, dashboard route, and stable census
//! prove that the failed transaction preserved its rollback generation.

use serde::Serialize;
use serde_json::Value;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{exit, Child, Command, Output, Stdio};

const INSTALL_SCHEMA_VERSION: &str = "agent-browser.workstation-install.v1";
const DEFAULT_DASHBOARD_PORT: u16 = 4848;
const DEFAULT_GUACAMOLE_PORT: u16 = 8092;
const MIN_WORKSTATION_FREE_DISK_BYTES: u64 = 6 * 1024 * 1024 * 1024;
// The shadow dashboard synchronously bootstraps its candidate service lane before
// binding. That bounded recovery may itself consume 20 seconds, so the installer
// must leave additional time for process startup and runtime-manifest hashing.
const DASHBOARD_CANDIDATE_START_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);
const DASHBOARD_PRESENTATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
const DASHBOARD_CANDIDATE_POLL_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(250);
const POST_COMMIT_DOCTOR_ATTEMPTS: usize = 4;
const POST_COMMIT_DOCTOR_RETRY_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);
const LEGACY_DAEMON_EXIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const LEGACY_DAEMON_EXIT_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(25);
// A reconcile may run as agent-browser-runtime-interlock.service, so stopping
// that service here would terminate the active reconciler before reactivation.
const WORKSTATION_RECONCILE_QUIESCE_UNITS: [&str; 3] = [
    "agent-browser-dashboard-backend.service",
    "agent-browser-runtime-interlock.timer",
    "agent-browser-guacamole-postgres-backup.timer",
];
const WORKSTATION_GENERATION_UNITS: [&str; 6] = [
    "agent-browser-dashboard-backend.service",
    "agent-browser-dashboard.service",
    "agent-browser-runtime-interlock.service",
    "agent-browser-runtime-interlock.timer",
    "agent-browser-guacamole-postgres-backup.service",
    "agent-browser-guacamole-postgres-backup.timer",
];
const GUACAMOLE_COMPOSE: &str = include_str!("../assets/workstation/guacamole/compose.yml");
const GUACAMOLE_ENVIRONMENT_EXAMPLE: &str =
    include_str!("../assets/workstation/guacamole/environment.example");
const GUACAMOLE_SCHEMA_GENERATOR: &str =
    include_str!("../assets/workstation/guacamole/generate-initdb.sh");
/// Rehydrates the sealed Guacamole template into container-local writable
/// storage before the upstream entrypoint installs generated extensions.
const GUACAMOLE_START_WRAPPER: &str =
    include_str!("../assets/workstation/guacamole/start-guacamole.sh");
const GUACAMOLE_BUNDLE_MANIFEST: &str =
    include_str!("../assets/workstation/guacamole/manifest.json");
const GUACAMOLE_INITDB: &str = include_str!("../assets/workstation/guacamole/init/001-initdb.sql");
const GUACAMOLE_DEFAULTS_EXTENSION_MANIFEST: &str =
    include_str!("../assets/workstation/guacamole/extensions/guac-manifest.json");
const GUACAMOLE_DEFAULTS_EXTENSION_SCRIPT: &str =
    include_str!("../assets/workstation/guacamole/extensions/agent-browser-defaults.js");
const ROUTE_POOL_READINESS_SCRIPT: &str =
    include_str!("../../scripts/smoke-rdp-guac-route-pool-readiness.js");
const RDP_GATEWAY_READINESS_SCRIPT: &str =
    include_str!("../../scripts/smoke-rdp-gateway-readiness.js");
const INSPECT_ROUTE_DISPLAYS_SCRIPT: &str =
    include_str!("../../scripts/inspect-rdp-route-displays.js");
const OPEN_ROUTE_DISPLAYS_SCRIPT: &str =
    include_str!("../../scripts/open-rdp-guac-route-displays.js");
const ROUTE_DISPLAY_SELECTION_SCRIPT: &str =
    include_str!("../../scripts/lib/rdp-route-display-selection.js");
const ROUTE_INVENTORY_SCRIPT: &str = include_str!("../../scripts/lib/rdp-route-inventory.js");
const ROUTE_USER_POOL_SCRIPT: &str = include_str!("../../scripts/lib/rdp-route-user-pool.py");
const ENSURE_POSTGRES_SCRIPT: &str = include_str!("../../scripts/ensure-rdp-guac-postgres.sh");
const POSTGRES_DURABILITY_SCRIPT: &str =
    include_str!("../../scripts/guacamole-postgres-durability.sh");
const SYNC_ROUTE_POOL_SCRIPT: &str =
    include_str!("../../scripts/sync-rdp-guac-route-specific-user-pool.sh");
const GRANT_ROUTE_DISPLAY_ACCESS_SCRIPT: &str =
    include_str!("../../scripts/grant-rdp-route-display-access.sh");
const CONTROLLER_PACKAGE_JSON: &str = "{\n  \"private\": true,\n  \"type\": \"module\"\n}\n";
const CONTROLLER_ASSETS: [(&str, &str, bool); 12] = [
    (
        "scripts/smoke-rdp-guac-route-pool-readiness.js",
        ROUTE_POOL_READINESS_SCRIPT,
        true,
    ),
    (
        "scripts/smoke-rdp-gateway-readiness.js",
        RDP_GATEWAY_READINESS_SCRIPT,
        true,
    ),
    (
        "scripts/inspect-rdp-route-displays.js",
        INSPECT_ROUTE_DISPLAYS_SCRIPT,
        true,
    ),
    (
        "scripts/open-rdp-guac-route-displays.js",
        OPEN_ROUTE_DISPLAYS_SCRIPT,
        true,
    ),
    (
        "scripts/ensure-rdp-guac-postgres.sh",
        ENSURE_POSTGRES_SCRIPT,
        true,
    ),
    (
        "scripts/guacamole-postgres-durability.sh",
        POSTGRES_DURABILITY_SCRIPT,
        true,
    ),
    (
        "scripts/sync-rdp-guac-route-specific-user-pool.sh",
        SYNC_ROUTE_POOL_SCRIPT,
        true,
    ),
    (
        "scripts/grant-rdp-route-display-access.sh",
        GRANT_ROUTE_DISPLAY_ACCESS_SCRIPT,
        true,
    ),
    (
        "scripts/lib/rdp-route-display-selection.js",
        ROUTE_DISPLAY_SELECTION_SCRIPT,
        false,
    ),
    (
        "scripts/lib/rdp-route-inventory.js",
        ROUTE_INVENTORY_SCRIPT,
        false,
    ),
    (
        "scripts/lib/rdp-route-user-pool.py",
        ROUTE_USER_POOL_SCRIPT,
        false,
    ),
    ("package.json", CONTROLLER_PACKAGE_JSON, false),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallMode {
    DryRun,
    Apply,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkstationInstallArgs {
    mode: InstallMode,
    json: bool,
    dashboard_port: u16,
    guacamole_port: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkstationPaths {
    root: String,
    binary: String,
    support_dir: String,
    unit_dir: String,
    guacamole_state_dir: String,
    guacamole_secret_file: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkstationInstallReport {
    schema_version: &'static str,
    success: bool,
    complete: bool,
    state: &'static str,
    mode: &'static str,
    mutated: bool,
    ready: bool,
    version: &'static str,
    dashboard_port: u16,
    guacamole_port: u16,
    host_plan: HostPlan,
    paths: WorkstationPaths,
    phases: Vec<&'static str>,
    host_prepared: bool,
    session_refresh_required: bool,
    runtime_census_transaction: Option<String>,
    reconcile_receipt: Option<String>,
    next_action: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HostPlan {
    supported: bool,
    fixture_root: bool,
    architecture: String,
    operating_system: String,
    missing_commands: Vec<String>,
    available_disk_bytes: Option<u64>,
    minimum_disk_bytes: u64,
    disk_space_ready: bool,
    effective_groups: Vec<String>,
    actions: Vec<&'static str>,
}

pub fn run_workstation_command(args: &[String]) {
    let json = args.iter().any(|arg| arg == "--json");
    let operation = args
        .iter()
        .position(|arg| arg == "workstation")
        .and_then(|index| args.get(index + 1))
        .map(String::as_str);
    match operation {
        Some("reconcile") => run_workstation_reconcile(json),
        Some("backup") => run_workstation_backup(json),
        Some("status") => run_workstation_upgrade_status(json),
        Some("recover") => run_workstation_upgrade_recover(args, json),
        Some("finalize") => run_workstation_upgrade_finalize(json),
        Some("gc") => run_workstation_generation_gc(args, json),
        _ => run_workstation_install(args),
    }
}

fn run_workstation_upgrade_recover(args: &[String], json: bool) {
    let result = (|| {
        if !cfg!(target_os = "linux") {
            return Err("workstation upgrade recovery is only supported on Linux".to_string());
        }
        let transaction_id = parse_recovery_transaction_id(args)?;
        let root = workstation_root()?;
        let _lock = WorkstationLock::acquire(&root)?;
        recover_operator_required_upgrade_for_root(
            &root,
            &transaction_id,
            env::var_os("AGENT_BROWSER_WORKSTATION_ROOT").is_some(),
        )
    })();
    match result {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        ),
        Ok(report) => println!(
            "Workstation transaction {} was recovered to the preserved generation; runtime admission is open.",
            report
                .get("transactionId")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        Err(error) => fail(&error, json),
    }
}

fn parse_recovery_transaction_id(args: &[String]) -> Result<String, String> {
    let mut transaction_id = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "install" | "workstation" | "recover" | "--json" => {}
            "--transaction-id" => {
                if transaction_id.is_some() {
                    return Err("--transaction-id may be specified only once".to_string());
                }
                index += 1;
                transaction_id = Some(
                    args.get(index)
                        .filter(|value| valid_upgrade_transaction_id(value))
                        .cloned()
                        .ok_or_else(|| {
                            "--transaction-id requires an exact upgrade transaction ID".to_string()
                        })?,
                );
            }
            unknown => return Err(format!("Unknown workstation recovery argument: {unknown}")),
        }
        index += 1;
    }
    transaction_id.ok_or_else(|| "workstation recovery requires --transaction-id <id>".to_string())
}

fn valid_upgrade_transaction_id(value: &str) -> bool {
    value.starts_with("upgrade-")
        && value.len() > "upgrade-".len()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

/// Closes one exact failed transaction only after proving that it preserved
/// the sealed old generation and no candidate executable or dashboard ingress
/// retains effect authority. Pre-admission census blocks have no drain to own;
/// every later recovery state must still own the matching drain.
fn recover_operator_required_upgrade_for_root(
    root: &Path,
    expected_transaction_id: &str,
    isolated_root: bool,
) -> Result<Value, String> {
    use crate::runtime_adoption::UpgradeTransactionState;

    if !valid_upgrade_transaction_id(expected_transaction_id) {
        return Err("workstation_recovery_transaction_id_invalid".to_string());
    }
    let paths = install_paths(root);
    let adoption_root = root.join(".agent-browser/runtime-adoption");
    let drain_path = adoption_root.join("admission-drain.json");
    let transaction_path = adoption_root
        .join("transactions")
        .join(format!("{expected_transaction_id}.json"));
    let mut transaction: crate::runtime_adoption::UpgradeTransaction =
        serde_json::from_slice(&fs::read(&transaction_path).map_err(display_io(
            "read operator recovery transaction",
            &transaction_path,
        ))?)
        .map_err(|error| format!("Operator recovery transaction is invalid: {error}"))?;
    if transaction.transaction_id != expected_transaction_id {
        return Err("workstation_recovery_transaction_evidence_mismatch".to_string());
    }
    let admission_drain_present = drain_path.is_file();
    if admission_drain_present {
        let drain: Value = serde_json::from_slice(
            &fs::read(&drain_path)
                .map_err(display_io("read runtime admission drain", &drain_path))?,
        )
        .map_err(|error| format!("Runtime admission drain is invalid: {error}"))?;
        let drain_transaction_id = drain
            .get("transactionId")
            .and_then(Value::as_str)
            .filter(|value| valid_upgrade_transaction_id(value))
            .ok_or_else(|| "runtime_admission_drain_transaction_invalid".to_string())?;
        if drain_transaction_id != expected_transaction_id {
            return Err("workstation_recovery_transaction_does_not_own_admission".to_string());
        }
        if drain.get("candidateGenerationId").and_then(Value::as_str)
            != Some(transaction.candidate_generation_id.as_str())
            || drain
                .get("transactionRevision")
                .and_then(Value::as_u64)
                .is_none_or(|revision| revision > transaction.revision)
        {
            return Err("workstation_recovery_transaction_evidence_mismatch".to_string());
        }
    }
    let operator_recovery_checkpoint_present = transaction
        .checkpoints
        .iter()
        .any(|checkpoint| checkpoint.name == "operator_recovery_verified_old_generation");
    let pre_admission_recovery_checkpoint_present = transaction
        .checkpoints
        .iter()
        .any(|checkpoint| checkpoint.name == "pre_admission_census_block_recovered");
    let pre_admission_inflight_block = transaction.state
        == UpgradeTransactionState::BlockedInflightEffect
        && transaction.stop_reason.as_deref() == Some("installer_lock_busy")
        && !admission_drain_present;
    let recoverable = match transaction.state {
        UpgradeTransactionState::OperatorRecoveryRequired => admission_drain_present,
        UpgradeTransactionState::BlockedAmbiguousRuntime => !admission_drain_present,
        UpgradeTransactionState::BlockedInflightEffect => pre_admission_inflight_block,
        UpgradeTransactionState::FailedPreservedOldGeneration => {
            operator_recovery_checkpoint_present || pre_admission_recovery_checkpoint_present
        }
        _ => false,
    };
    if !recoverable {
        return Err(format!(
            "workstation_recovery_requires_operator_recovery_state:{:?}",
            transaction.state
        ));
    }
    let old_generation_id = transaction
        .old_generation_id
        .as_deref()
        .ok_or_else(|| "workstation_recovery_old_generation_missing".to_string())?
        .to_string();
    if selected_generation_id(&paths).as_deref() != Some(old_generation_id.as_str()) {
        return Err("workstation_recovery_old_generation_not_selected".to_string());
    }
    validate_sealed_generation_tree(&paths.generations_dir.join(&old_generation_id))?;

    if pre_admission_inflight_block {
        transaction.stop_reason = Some("pre_admission_inflight_block_recovered".to_string());
        transaction.terminal_result = Some("old_generation_preserved".to_string());
        persist_upgrade_transition(
            &transaction_path,
            &mut transaction,
            UpgradeTransactionState::FailedPreservedOldGeneration,
            "pre_admission_inflight_block_recovered",
        )?;
        return Ok(serde_json::json!({
            "schemaVersion": "agent-browser.workstation-upgrade-recovery.v1",
            "success": true,
            "changed": true,
            "transactionId": transaction.transaction_id,
            "state": transaction.state,
            "selectedGenerationId": old_generation_id,
            "candidatePayloadStaged": false,
            "runtimeCensusStable": true,
            "admissionDrainPresent": false,
            "admissionDraining": false,
        }));
    }

    let dashboard_ingress_path = env::var_os("AGENT_BROWSER_DASHBOARD_INGRESS_STATE")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".agent-browser/dashboard-ingress.json"));
    let dashboard_ingress =
        crate::dashboard_ingress::dashboard_ingress_status_for_path(&dashboard_ingress_path);
    let candidate_selected = dashboard_ingress
        .pointer("/selectedBackend/generationId")
        .and_then(Value::as_str)
        == Some(transaction.candidate_generation_id.as_str());
    let candidate_staged = dashboard_ingress
        .pointer("/candidateBackend/generationId")
        .and_then(Value::as_str)
        == Some(transaction.candidate_generation_id.as_str());
    let candidate_fallback = dashboard_ingress
        .pointer("/fallbackBackend/generationId")
        .and_then(Value::as_str)
        == Some(transaction.candidate_generation_id.as_str());
    if candidate_selected || candidate_staged {
        return Err("workstation_recovery_candidate_dashboard_still_routed".to_string());
    }
    if candidate_fallback {
        let repository =
            crate::dashboard_ingress::DashboardIngressRepository::new(&dashboard_ingress_path);
        let registry = repository.load()?;
        repository.retire_fallback(registry.revision, &transaction.candidate_generation_id)?;
        if !isolated_root {
            let command_env = workstation_command_env(&paths);
            restart_stable_dashboard_ingress(&paths, &paths.support_dir, &command_env)?;
        }
    }
    let runtime_host_ingress_path =
        crate::runtime_host_ingress::RuntimeHostIngressRepository::default_path();
    if runtime_host_ingress_path.is_file() {
        let repository = crate::runtime_host_ingress::RuntimeHostIngressRepository::new(
            runtime_host_ingress_path,
        );
        let ingress = repository.load()?;
        if ingress.selected_backend().generation_id == transaction.candidate_generation_id
            || ingress.candidate_backend().is_some_and(|candidate| {
                candidate.generation_id == transaction.candidate_generation_id
            })
        {
            return Err("workstation_recovery_candidate_runtime_host_still_routed".to_string());
        }
    }

    let mut live_process_references = std::collections::BTreeMap::new();
    collect_process_generation_references(&paths, &mut live_process_references);
    if live_process_references.contains_key(&transaction.candidate_generation_id)
        && operator_recovery_can_stop_rolled_back_candidate_host(&transaction)
    {
        stop_candidate_runtime_host(&paths, &transaction)?;
        live_process_references.clear();
        collect_process_generation_references(&paths, &mut live_process_references);
    }
    if live_process_references.contains_key(&transaction.candidate_generation_id) {
        return Err("workstation_recovery_candidate_process_still_live".to_string());
    }

    let census = if isolated_root {
        isolated_runtime_census()
    } else {
        collect_stable_runtime_census_with(
            crate::runtime_adoption::collect_host_runtime_census_round,
        )
    }
    .map_err(|error| format!("workstation_recovery_runtime_census_unstable:{error}"))?;
    if !census.activation_allowed {
        return Err("workstation_recovery_runtime_census_ambiguous".to_string());
    }

    let pre_admission_census_block =
        transaction.state == UpgradeTransactionState::BlockedAmbiguousRuntime;
    let changed = matches!(
        transaction.state,
        UpgradeTransactionState::OperatorRecoveryRequired
            | UpgradeTransactionState::BlockedAmbiguousRuntime
    );
    if changed {
        let checkpoint = if pre_admission_census_block {
            "pre_admission_census_block_recovered"
        } else {
            "operator_recovery_verified_old_generation"
        };
        transaction.stop_reason = Some(checkpoint.to_string());
        transaction.terminal_result = Some("old_generation_preserved".to_string());
        persist_upgrade_transition(
            &transaction_path,
            &mut transaction,
            UpgradeTransactionState::FailedPreservedOldGeneration,
            checkpoint,
        )?;
    }
    clear_admission_drain(&drain_path)?;
    Ok(serde_json::json!({
        "schemaVersion": "agent-browser.workstation-upgrade-recovery.v1",
        "success": true,
        "changed": changed,
        "transactionId": transaction.transaction_id,
        "state": transaction.state,
        "selectedGenerationId": old_generation_id,
        "candidateProcessAbsent": true,
        "candidateDashboardRouteAbsent": true,
        "candidateRuntimeHostRouteAbsent": true,
        "runtimeCensusStable": true,
        "admissionDrainPresent": admission_drain_present,
        "admissionDraining": false,
    }))
}

fn operator_recovery_can_stop_rolled_back_candidate_host(
    transaction: &crate::runtime_adoption::UpgradeTransaction,
) -> bool {
    use crate::runtime_adoption::{RuntimeLaneTransferState, UpgradeTransactionState};

    transaction.state == UpgradeTransactionState::OperatorRecoveryRequired
        && transaction
            .runtime_host_convergence
            .as_ref()
            .is_some_and(|convergence| {
                convergence.candidate_host.is_some()
                    && !convergence.lanes.is_empty()
                    && convergence.lanes.iter().all(|lane| {
                        lane.state == RuntimeLaneTransferState::RolledBack
                            && lane.rollback_owner_generation.is_some_and(|rollback| {
                                lane.owner_generation_after
                                    .is_some_and(|candidate| rollback > candidate)
                            })
                            && lane
                                .rollback_receipt_id
                                .as_deref()
                                .is_some_and(|receipt| !receipt.trim().is_empty())
                    })
            })
}

fn run_workstation_upgrade_status(json: bool) {
    let result = workstation_upgrade_status_json();
    match result {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        ),
        Ok(report) => {
            let selected = report
                .get("selectedGenerationId")
                .and_then(Value::as_str)
                .unwrap_or("none");
            let state = report
                .pointer("/latestTransaction/state")
                .and_then(Value::as_str)
                .unwrap_or("none");
            println!("Selected generation: {selected}");
            println!("Latest transaction: {state}");
        }
        Err(error) => fail(&error, json),
    }
}

fn run_workstation_upgrade_finalize(json: bool) {
    let result = (|| {
        if !cfg!(target_os = "linux") {
            return Err("workstation upgrade finalization is only supported on Linux".to_string());
        }
        let root = workstation_root()?;
        let _lock = WorkstationLock::acquire(&root)?;
        finalize_accepted_upgrade_for_root(&root)
    })();
    match result {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        ),
        Ok(report) => println!(
            "Workstation upgrade {} is finalized; rollback generation is now eligible for reviewed GC.",
            report
                .get("transactionId")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        Err(error) => fail(&error, json),
    }
}

fn finalize_accepted_upgrade_for_root(root: &Path) -> Result<Value, String> {
    use crate::runtime_adoption::UpgradeTransactionState;

    if root
        .join(".agent-browser/runtime-adoption/admission-drain.json")
        .is_file()
    {
        return Err("workstation_upgrade_finalize_blocked_by_admission_drain".to_string());
    }
    let transaction_dir = root.join(".agent-browser/runtime-adoption/transactions");
    let Some((path, mut transaction)) = latest_upgrade_transaction_entry(&transaction_dir)? else {
        return Err("workstation_upgrade_finalize_requires_a_transaction".to_string());
    };
    if transaction.state == UpgradeTransactionState::OldGenerationRetirable {
        return Ok(serde_json::json!({
            "schemaVersion": "agent-browser.workstation-upgrade-finalize.v1",
            "success": true,
            "transactionId": transaction.transaction_id,
            "state": transaction.state,
            "changed": false,
        }));
    }
    if transaction.state != UpgradeTransactionState::Accepted {
        return Err(format!(
            "workstation_upgrade_finalize_requires_accepted_transaction:{}",
            serde_json::to_value(transaction.state)
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_else(|| "unknown".to_string())
        ));
    }
    let status = workstation_upgrade_status_for_root(root)?;
    if status.get("ready").and_then(Value::as_bool) != Some(true) {
        return Err("workstation_upgrade_finalize_requires_all_readiness_axes".to_string());
    }
    persist_upgrade_transition(
        &path,
        &mut transaction,
        UpgradeTransactionState::OldGenerationRetirable,
        "old_generation_retirable_after_review",
    )?;
    Ok(serde_json::json!({
        "schemaVersion": "agent-browser.workstation-upgrade-finalize.v1",
        "success": true,
        "transactionId": transaction.transaction_id,
        "state": transaction.state,
        "changed": true,
    }))
}

pub(crate) fn workstation_upgrade_status_json() -> Result<Value, String> {
    let root = workstation_root()?;
    workstation_upgrade_status_for_root(&root)
}

fn workstation_upgrade_status_for_root(root: &Path) -> Result<Value, String> {
    let paths = install_paths(root);
    let transaction_dir = root.join(".agent-browser/runtime-adoption/transactions");
    let latest = latest_upgrade_transaction(&transaction_dir)?;
    let active_drain = root
        .join(".agent-browser/runtime-adoption/admission-drain.json")
        .is_file();
    let selected_generation = selected_generation_id(&paths);
    let dashboard_ingress_path = env::var_os("AGENT_BROWSER_DASHBOARD_INGRESS_STATE")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".agent-browser/dashboard-ingress.json"));
    let dashboard_ingress =
        crate::dashboard_ingress::dashboard_ingress_status_for_path(&dashboard_ingress_path);
    let readiness = workstation_upgrade_readiness(
        &paths,
        selected_generation.as_deref(),
        latest.as_ref(),
        active_drain,
        &dashboard_ingress,
    );
    let summary = latest.as_ref().map(|transaction| {
        serde_json::json!({
            "transactionId": transaction.transaction_id,
            "state": transaction.state,
            "revision": transaction.revision,
            "oldGenerationId": transaction.old_generation_id,
            "candidateGenerationId": transaction.candidate_generation_id,
            "runtimeMigrations": transaction.runtime_migrations.iter().map(|migration| serde_json::json!({
                "logicalBrowserId": migration.logical_browser_id,
                "classification": migration.classification,
                "disposition": migration.disposition,
                "receipted": migration.adoption_receipt_id.is_some(),
                "reasonCodes": migration.reason_codes,
            })).collect::<Vec<_>>(),
            "dashboardValidationSummary": transaction.dashboard_validation_summary,
            "presentationValidationSummary": transaction.presentation_validation_summary,
            "terminalResult": transaction.terminal_result,
            "stopReason": transaction.stop_reason,
        })
    });
    Ok(serde_json::json!({
        "schemaVersion": "agent-browser.workstation-upgrade-status.v1",
        "success": true,
        "ready": readiness.get("ready").cloned().unwrap_or(Value::Bool(false)),
        "selectedGenerationId": selected_generation,
        "admissionDraining": active_drain,
        "readiness": readiness,
        "dashboardIngress": dashboard_ingress,
        "latestTransaction": summary,
    }))
}

fn workstation_upgrade_readiness(
    paths: &InstallPaths,
    selected_generation_id: Option<&str>,
    transaction: Option<&crate::runtime_adoption::UpgradeTransaction>,
    admission_draining: bool,
    dashboard_ingress: &Value,
) -> Value {
    use crate::runtime_adoption::UpgradeTransactionState;

    let generation_ready = |generation_id: &str| {
        validate_sealed_generation_tree(&paths.generations_dir.join(generation_id)).is_ok()
    };
    let payload_ready = selected_generation_id.is_some_and(generation_ready);
    let dashboard_ingress_ready = dashboard_ingress
        .get("dashboardIngressReady")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let operator_journey_ready = dashboard_ingress
        .get("operatorJourneyReady")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let (upgrade_state, selected_generation_ready, runtime_convergence_ready, rollback_ready) =
        if let Some(transaction) = transaction {
            let candidate_selected =
                selected_generation_id == Some(transaction.candidate_generation_id.as_str());
            let old_selected = selected_generation_id == transaction.old_generation_id.as_deref();
            let expects_candidate = matches!(
                transaction.state,
                UpgradeTransactionState::GenerationCommitted
                    | UpgradeTransactionState::PostCommitValidating
                    | UpgradeTransactionState::Accepted
                    | UpgradeTransactionState::OldGenerationRetirable
            );
            let expects_old = matches!(
                transaction.state,
                UpgradeTransactionState::Planned
                    | UpgradeTransactionState::CandidateStaged
                    | UpgradeTransactionState::CandidatePreflightReady
                    | UpgradeTransactionState::CensusStable
                    | UpgradeTransactionState::AdmissionDraining
                    | UpgradeTransactionState::RuntimesTransferring
                    | UpgradeTransactionState::PresentationsRebinding
                    | UpgradeTransactionState::CandidateReady
                    | UpgradeTransactionState::BlockedAmbiguousRuntime
                    | UpgradeTransactionState::BlockedInflightEffect
                    | UpgradeTransactionState::BlockedCandidateIncompatible
                    | UpgradeTransactionState::RollbackBeforeCommit
                    | UpgradeTransactionState::FailedPreservedOldGeneration
            );
            let selected_ready = (expects_candidate && candidate_selected && payload_ready)
                || (expects_old
                    && old_selected
                    && transaction
                        .old_generation_id
                        .as_deref()
                        .is_none_or(generation_ready));
            let runtime_ready =
                crate::runtime_adoption::upgrade_runtime_preservation_proven(transaction)
                    && !matches!(
                        transaction.state,
                        UpgradeTransactionState::BlockedAmbiguousRuntime
                            | UpgradeTransactionState::BlockedInflightEffect
                            | UpgradeTransactionState::BlockedCandidateIncompatible
                            | UpgradeTransactionState::OperatorRecoveryRequired
                            | UpgradeTransactionState::FailedEffectUncertain
                    );
            // Finalization explicitly relinquishes rollback authority, so
            // reviewed GC may remove the old payload without degrading the
            // selected runtime's readiness.
            let rollback_ready = transaction.state
                == UpgradeTransactionState::OldGenerationRetirable
                || transaction
                    .old_generation_id
                    .as_deref()
                    .is_none_or(generation_ready);
            (
                serde_json::to_value(transaction.state)
                    .unwrap_or_else(|_| Value::String("unknown".to_string())),
                selected_ready,
                runtime_ready,
                rollback_ready,
            )
        } else {
            (
                Value::String("none".to_string()),
                payload_ready,
                payload_ready,
                true,
            )
        };
    let transaction_terminal = transaction.is_none_or(|transaction| {
        matches!(
            transaction.state,
            UpgradeTransactionState::Accepted | UpgradeTransactionState::OldGenerationRetirable
        )
    });
    let ready = payload_ready
        && selected_generation_ready
        && runtime_convergence_ready
        && dashboard_ingress_ready
        && operator_journey_ready
        && rollback_ready
        && transaction_terminal
        && !admission_draining;

    serde_json::json!({
        "payloadReady": payload_ready,
        "selectedGenerationReady": selected_generation_ready,
        "runtimeConvergenceReady": runtime_convergence_ready,
        "upgradeTransactionState": upgrade_state,
        "dashboardIngressReady": dashboard_ingress_ready,
        "operatorJourneyReady": operator_journey_ready,
        "rollbackReady": rollback_ready,
        "ready": ready,
    })
}

fn latest_upgrade_transaction(
    transaction_dir: &Path,
) -> Result<Option<crate::runtime_adoption::UpgradeTransaction>, String> {
    latest_upgrade_transaction_entry(transaction_dir)
        .map(|entry| entry.map(|(_, transaction)| transaction))
}

fn latest_upgrade_transaction_entry(
    transaction_dir: &Path,
) -> Result<Option<(PathBuf, crate::runtime_adoption::UpgradeTransaction)>, String> {
    let entries = match fs::read_dir(transaction_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Unable to read runtime transaction directory {}: {error}",
                transaction_dir.display()
            ));
        }
    };
    let mut candidates = Vec::new();
    for entry in entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
    {
        let path = entry.path();
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .map_err(display_io("inspect runtime transaction", &path))?;
        let body = fs::read(&path).map_err(display_io("read runtime transaction", &path))?;
        let transaction: crate::runtime_adoption::UpgradeTransaction =
            serde_json::from_slice(&body).map_err(|error| {
                format!("Runtime transaction {} is invalid: {error}", path.display())
            })?;
        let planned_at = transaction
            .checkpoints
            .iter()
            .find(|checkpoint| checkpoint.name == "transaction_planned")
            .or_else(|| transaction.checkpoints.first())
            .and_then(|checkpoint| {
                chrono::DateTime::parse_from_rfc3339(&checkpoint.recorded_at).ok()
            })
            .map(|timestamp| timestamp.timestamp_millis() as i128 * 1_000_000)
            .unwrap_or_else(|| {
                modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos() as i128)
                    .unwrap_or(i128::MIN)
            });
        candidates.push((planned_at, modified, path, transaction));
    }
    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| right.2.cmp(&left.2))
    });
    let Some((_, _, path, transaction)) = candidates.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some((path, transaction)))
}

fn run_workstation_generation_gc(args: &[String], json: bool) {
    let result = (|| {
        let mode = if args.iter().any(|arg| arg == "--apply") {
            if args.iter().any(|arg| arg == "--dry-run") {
                return Err("workstation gc modes are mutually exclusive".to_string());
            }
            InstallMode::Apply
        } else if args.iter().any(|arg| arg == "--dry-run") {
            InstallMode::DryRun
        } else {
            return Err(
                "Choose exactly one of --dry-run or --apply for workstation gc".to_string(),
            );
        };
        let root = workstation_root()?;
        let paths = install_paths(&root);
        let _lock = if mode == InstallMode::Apply {
            Some(WorkstationLock::acquire(&root)?)
        } else {
            None
        };
        workstation_generation_gc_locked(&root, &paths, mode)
    })();
    match result {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        ),
        Ok(report) => println!(
            "Generation GC {}: {} candidate(s), {} removed.",
            report
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            report
                .get("candidates")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0),
            report
                .get("removed")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0)
        ),
        Err(error) => fail(&error, json),
    }
}

fn workstation_generation_gc_locked(
    root: &Path,
    paths: &InstallPaths,
    mode: InstallMode,
) -> Result<Value, String> {
    if mode == InstallMode::Apply
        && root
            .join(".agent-browser/runtime-adoption/admission-drain.json")
            .exists()
    {
        return Err("generation_gc_blocked_by_active_admission_drain".to_string());
    }
    let retention = generation_retention_plan(root, paths, mode == InstallMode::Apply)?;
    let finalizable_transaction_ids = retention.finalizable_transaction_ids.clone();
    let references = retention.references;
    let mut retained = Vec::new();
    let mut candidates = Vec::new();
    let entries = match fs::read_dir(&paths.generations_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(serde_json::json!({
                "schemaVersion": "agent-browser.workstation-generation-gc.v1",
                "success": true,
                "mode": if mode == InstallMode::Apply { "apply" } else { "dry-run" },
                "selectedGenerationId": selected_generation_id(paths),
                "previousHealthyGenerationId": retention.previous_healthy_generation_id,
                "finalizableTransactionIds": finalizable_transaction_ids.clone(),
                "finalizedTransactionIds": if mode == InstallMode::Apply {
                    finalizable_transaction_ids.clone()
                } else {
                    Vec::<String>::new()
                },
                "candidates": [],
                "retained": [],
                "removed": [],
            }));
        }
        Err(error) => {
            return Err(format!(
                "Unable to read runtime generations {}: {error}",
                paths.generations_dir.display()
            ));
        }
    };
    for entry in entries.filter_map(Result::ok) {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let generation_id = entry.file_name().to_string_lossy().to_string();
        if let Some(reasons) = references.get(&generation_id) {
            retained.push(serde_json::json!({
                "generationId": generation_id,
                "reasonCodes": reasons,
            }));
        } else {
            validate_sealed_generation_tree(&entry.path())?;
            candidates.push(generation_id);
        }
    }
    candidates.sort();
    retained.sort_by(|left, right| {
        left.get("generationId")
            .and_then(Value::as_str)
            .cmp(&right.get("generationId").and_then(Value::as_str))
    });
    let mut removed = Vec::new();
    if mode == InstallMode::Apply {
        for generation_id in &candidates {
            let generation_path = paths.generations_dir.join(generation_id);
            remove_generation_tree(&generation_path)?;
            removed.push(generation_id.clone());
        }
    }
    Ok(serde_json::json!({
        "schemaVersion": "agent-browser.workstation-generation-gc.v1",
        "success": true,
        "mode": if mode == InstallMode::Apply { "apply" } else { "dry-run" },
        "selectedGenerationId": selected_generation_id(paths),
        "previousHealthyGenerationId": retention.previous_healthy_generation_id,
        "finalizableTransactionIds": finalizable_transaction_ids.clone(),
        "finalizedTransactionIds": if mode == InstallMode::Apply {
            finalizable_transaction_ids.clone()
        } else {
            Vec::<String>::new()
        },
        "candidates": candidates,
        "retained": retained,
        "removed": removed,
    }))
}

#[cfg(test)]
fn generation_references(
    root: &Path,
    paths: &InstallPaths,
) -> Result<std::collections::BTreeMap<String, Vec<String>>, String> {
    Ok(generation_retention_plan(root, paths, false)?.references)
}

fn generation_retention_plan(
    root: &Path,
    paths: &InstallPaths,
    finalize_eligible: bool,
) -> Result<crate::runtime_retention::GenerationRetentionPlan, String> {
    let selected = selected_generation_id(paths);
    let transaction_dir = root.join(".agent-browser/runtime-adoption/transactions");
    let mut transactions = Vec::new();
    let mut transaction_paths = std::collections::BTreeMap::new();
    if let Ok(entries) = fs::read_dir(&transaction_dir) {
        for entry in entries.filter_map(Result::ok).filter(|entry| {
            entry.path().extension().and_then(|value| value.to_str()) == Some("json")
        }) {
            let path = entry.path();
            let transaction: crate::runtime_adoption::UpgradeTransaction =
                serde_json::from_slice(&fs::read(&path).map_err(display_io(
                    "read runtime transaction for generation gc",
                    &path,
                ))?)
                .map_err(|error| format!("Runtime transaction blocks generation gc: {error}"))?;
            transaction_paths.insert(transaction.transaction_id.clone(), path);
            transactions.push(transaction);
        }
    }
    let plan = crate::runtime_retention::plan_generation_retention(
        selected.as_deref(),
        &transactions,
        chrono::Utc::now(),
    );
    if finalize_eligible {
        for transaction_id in &plan.finalizable_transaction_ids {
            let transaction = transactions
                .iter_mut()
                .find(|transaction| &transaction.transaction_id == transaction_id)
                .ok_or_else(|| "retention_transaction_disappeared".to_string())?;
            let expected_revision = transaction.revision;
            crate::runtime_adoption::transition_upgrade_transaction(
                transaction,
                expected_revision,
                crate::runtime_adoption::UpgradeTransactionState::OldGenerationRetirable,
                "retention_window_elapsed",
                &runtime_adoption_timestamp(),
            )?;
            let path = transaction_paths
                .get(transaction_id)
                .ok_or_else(|| "retention_transaction_path_missing".to_string())?;
            write_private_json_atomic(path, transaction)?;
        }
    }
    let mut references = plan.references.clone();
    collect_process_generation_references(paths, &mut references);
    collect_supervisor_generation_references(root, paths, &mut references)?;
    for reasons in references.values_mut() {
        reasons.sort();
        reasons.dedup();
    }
    Ok(crate::runtime_retention::GenerationRetentionPlan { references, ..plan })
}

fn collect_process_generation_references(
    paths: &InstallPaths,
    references: &mut std::collections::BTreeMap<String, Vec<String>>,
) {
    collect_process_generation_references_from(Path::new("/proc"), paths, references);
}

/// Adds references for exact process executables rooted in an immutable
/// generation. The proc root is injectable so the cleanup safety boundary is
/// deterministic without starting or signaling a process in tests.
fn collect_process_generation_references_from(
    proc_root: &Path,
    paths: &InstallPaths,
    references: &mut std::collections::BTreeMap<String, Vec<String>>,
) {
    let Ok(entries) = fs::read_dir(proc_root) else {
        return;
    };
    for entry in entries.filter_map(Result::ok).filter(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .bytes()
            .all(|byte| byte.is_ascii_digit())
    }) {
        let Ok(executable) = fs::read_link(entry.path().join("exe")) else {
            continue;
        };
        if let Some(generation) = generation_id_from_path(&executable, &paths.generations_dir) {
            references
                .entry(generation)
                .or_default()
                .push("live_process".to_string());
        }
    }
}

fn collect_supervisor_generation_references(
    root: &Path,
    paths: &InstallPaths,
    references: &mut std::collections::BTreeMap<String, Vec<String>>,
) -> Result<(), String> {
    let manifests = root.join(".config/agent-browser/session-supervisors");
    let entries = match fs::read_dir(&manifests) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Unable to read session supervisor manifests {}: {error}",
                manifests.display()
            ));
        }
    };
    for entry in entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
    {
        let path = entry.path();
        let manifest: Value = serde_json::from_slice(
            &fs::read(&path).map_err(display_io("read session supervisor manifest", &path))?,
        )
        .map_err(|error| format!("Session supervisor manifest blocks generation gc: {error}"))?;
        if let Some(executable) = manifest.get("executablePath").and_then(Value::as_str) {
            if let Some(generation) =
                generation_id_from_path(Path::new(executable), &paths.generations_dir)
            {
                references
                    .entry(generation)
                    .or_default()
                    .push("session_supervisor".to_string());
            }
        }
    }
    Ok(())
}

fn generation_id_from_path(path: &Path, generations_dir: &Path) -> Option<String> {
    path.strip_prefix(generations_dir)
        .ok()
        .and_then(|relative| relative.components().next())
        .map(|component| component.as_os_str().to_string_lossy().to_string())
}

fn run_workstation_install(args: &[String]) {
    let parsed = match parse_workstation_install_args(args) {
        Ok(parsed) => parsed,
        Err(error) => fail(&error, args.iter().any(|arg| arg == "--json")),
    };
    if !cfg!(target_os = "linux") {
        fail(
            "agent-browser install workstation is only supported on Linux",
            parsed.json,
        );
    }

    let root = match workstation_root() {
        Ok(root) => root,
        Err(error) => fail(&error, parsed.json),
    };
    let mut paths = install_paths(&root);
    let mut phases = vec!["plan-validated"];
    let isolated_root = env::var_os("AGENT_BROWSER_WORKSTATION_ROOT").is_some();
    let host_plan = build_host_plan(isolated_root, &root);
    if !host_plan.disk_space_ready {
        let mut error = format!(
                "workstation installation requires at least {} bytes of free disk space; {} bytes are available",
                host_plan.minimum_disk_bytes,
                host_plan.available_disk_bytes.unwrap_or(0)
            );
        if parsed.mode == InstallMode::Apply {
            if let Ok(path) = record_blocked_upgrade_transaction(
                &root,
                &paths,
                &parsed,
                crate::runtime_adoption::UpgradeTransactionState::BlockedCandidateIncompatible,
                "host_disk_precondition_failed",
            ) {
                error.push_str(&format!("; transaction: {}", path.display()));
            }
        }
        fail(&error, parsed.json);
    }
    if !host_plan.supported {
        let mut error = "workstation installation requires Ubuntu 24.04 x86_64 with apt-get, apt-cache, bash, sudo, and systemctl".to_string();
        if parsed.mode == InstallMode::Apply {
            if let Ok(path) = record_blocked_upgrade_transaction(
                &root,
                &paths,
                &parsed,
                crate::runtime_adoption::UpgradeTransactionState::BlockedCandidateIncompatible,
                "host_platform_precondition_failed",
            ) {
                error.push_str(&format!("; transaction: {}", path.display()));
            }
        }
        fail(&error, parsed.json);
    }
    let _install_lock = if parsed.mode == InstallMode::Apply {
        match WorkstationLock::acquire(&root) {
            Ok(lock) => Some(lock),
            Err(error) => {
                let message = record_blocked_upgrade_transaction(
                    &root,
                    &paths,
                    &parsed,
                    crate::runtime_adoption::UpgradeTransactionState::BlockedInflightEffect,
                    "installer_lock_busy",
                )
                .map(|path| format!("{error}; transaction: {}", path.display()))
                .unwrap_or(error);
                fail(&message, parsed.json)
            }
        }
    } else {
        None
    };
    let mut apply_quiesced_user_units = None;
    let mut runtime_census_transaction = None;
    let mut host_prepared = false;
    let mut session_refresh_required = false;
    let mut reconcile_receipt = None;
    let mut workstation_ready = false;
    let mut next_action =
        "workstation substrate provisioning is required before service activation".to_string();
    let mut prepared_payload = if parsed.mode == InstallMode::Apply {
        let prepared = match prepare_payload_transaction(&root, &paths, &parsed, isolated_root) {
            Ok(prepared) => prepared,
            Err(error) => fail(&error, parsed.json),
        };
        runtime_census_transaction = Some(prepared.transaction_path.display().to_string());
        phases.extend([
            "payload-staged",
            "units-staged",
            "candidate-preflight-ready",
            "runtime-census-stable",
        ]);
        Some(prepared)
    } else {
        None
    };

    if parsed.mode == InstallMode::Apply && !isolated_root {
        if let Err(error) = crate::install::install_remote_view_privileges(true, parsed.json) {
            if let Some(prepared) = prepared_payload.as_mut() {
                let _ = rollback_prepared_payload_transaction(
                    &paths,
                    prepared,
                    false,
                    "host_dependency_precondition_failed",
                );
            }
            fail_with_user_unit_restoration(
                &error,
                parsed.json,
                &paths,
                apply_quiesced_user_units.as_ref(),
            );
        }
        phases.push("host-dependencies-prepared");
        host_prepared = true;
        session_refresh_required =
            !current_process_has_group("agent-browser") || !current_process_has_group("docker");
        if session_refresh_required {
            if let Some(prepared) = prepared_payload.as_mut() {
                if let Err(error) = rollback_prepared_payload_transaction(
                    &paths,
                    prepared,
                    false,
                    "operator_login_refresh_required",
                ) {
                    fail(&error, parsed.json);
                }
            }
            next_action =
                "log out and back in or reboot, then rerun workstation installation".to_string();
        }
    }

    if parsed.mode == InstallMode::Apply && !session_refresh_required {
        let prepared = prepared_payload
            .as_mut()
            .expect("apply always prepares one payload transaction");
        if !isolated_root {
            if let Err(error) =
                prepare_dashboard_candidate_for_transaction(&root, &parsed, prepared)
            {
                let rollback = rollback_prepared_payload_transaction(
                    &paths,
                    prepared,
                    false,
                    "candidate_dashboard_shadow_failed",
                );
                fail(
                    &rollback
                        .err()
                        .map_or(error.clone(), |rollback| format!("{error}; {rollback}")),
                    parsed.json,
                );
            }
            phases.push("candidate-dashboard-shadow-ready");
        }
        if let Err(error) = activate_prepared_payload_transaction(prepared, &paths, isolated_root) {
            let rollback_error = rollback_prepared_payload_transaction(
                &paths,
                prepared,
                false,
                "candidate_activation_failed",
            )
            .err();
            fail(
                &rollback_error.map_or(error.clone(), |rollback| format!("{error}; {rollback}")),
                parsed.json,
            );
        }
        phases.extend([
            "admission-draining",
            "runtimes-transferred",
            "presentations-rebound",
            "candidate-ready",
        ]);
        if !isolated_root {
            if let Err(error) =
                wait_for_dashboard_candidate_commit(prepared, DASHBOARD_PRESENTATION_TIMEOUT)
            {
                let rollback = rollback_prepared_payload_transaction(
                    &paths,
                    prepared,
                    false,
                    "candidate_dashboard_presentation_unproven",
                );
                fail(
                    &rollback
                        .err()
                        .map_or(error.clone(), |rollback| format!("{error}; {rollback}")),
                    parsed.json,
                );
            }
            phases.push("candidate-presentation-receipted");
        }
        if !isolated_root {
            match quiesce_existing_user_units(&paths) {
                Ok(quiesced) => apply_quiesced_user_units = Some(quiesced),
                Err(error) => {
                    let _ = rollback_prepared_payload_transaction(
                        &paths,
                        prepared,
                        false,
                        "user_unit_quiesce_failed",
                    );
                    fail(&error, parsed.json);
                }
            }
        }
        if let Err(error) = commit_prepared_payload_transaction(&paths, &parsed, prepared) {
            let _ = rollback_prepared_payload_transaction(
                &paths,
                prepared,
                false,
                "generation_commit_failed",
            );
            fail_with_user_unit_restoration(
                &error,
                parsed.json,
                &paths,
                apply_quiesced_user_units.as_ref(),
            );
        }
        phases.push("payload-committed");
        paths = install_paths(&root);
        if let Err(error) = begin_post_commit_validation(prepared) {
            let rollback = rollback_prepared_payload_transaction(
                &paths,
                prepared,
                true,
                "post_commit_validation_transition_failed",
            );
            fail(
                &rollback
                    .err()
                    .map_or(error.clone(), |rollback| format!("{error}; {rollback}")),
                parsed.json,
            );
        }
        phases.push("post-commit-validating");

        let validation = if !isolated_root {
            let transitional_source_sessions = permitted_stale_source_sessions(
                &prepared.runtime_handoffs,
                &prepared.transaction.runtime_migrations,
            );
            let reconcile = match reconcile_workstation_locked_for_upgrade(
                &root,
                &paths,
                Some(&prepared.transaction),
                &transitional_source_sessions,
            ) {
                Ok(reconcile) => reconcile,
                Err(error) => {
                    let rollback = rollback_prepared_payload_transaction(
                        &paths,
                        prepared,
                        true,
                        "post_commit_reconciliation_failed",
                    );
                    let message = rollback
                        .err()
                        .map_or(error.clone(), |rollback| format!("{error}; {rollback}"));
                    fail_with_user_unit_restoration(
                        &message,
                        parsed.json,
                        &paths,
                        apply_quiesced_user_units.as_ref(),
                    );
                }
            };
            phases.push("workstation-reconciled");
            workstation_ready = true;
            reconcile_receipt = Some(reconcile.receipt_path);
            next_action =
                "run agent-browser install doctor --json and review the installed workstation state"
                    .to_string();
            if let Err(error) = promote_dashboard_candidate_to_managed_backend(&parsed, prepared) {
                let rollback = rollback_prepared_payload_transaction(
                    &paths,
                    prepared,
                    true,
                    "managed_dashboard_promotion_failed",
                );
                let message = rollback
                    .err()
                    .map_or(error.clone(), |rollback| format!("{error}; {rollback}"));
                fail_with_user_unit_restoration(
                    &message,
                    parsed.json,
                    &paths,
                    apply_quiesced_user_units.as_ref(),
                );
            }
            phases.push("candidate-dashboard-managed");
            match validate_post_commit_transaction(&root, &paths, prepared) {
                Ok(validation) => validation,
                Err(error) => {
                    let rollback = rollback_prepared_payload_transaction(
                        &paths,
                        prepared,
                        true,
                        "post_commit_readiness_unproven",
                    );
                    let message = rollback
                        .err()
                        .map_or(error.clone(), |rollback| format!("{error}; {rollback}"));
                    fail_with_user_unit_restoration(
                        &message,
                        parsed.json,
                        &paths,
                        apply_quiesced_user_units.as_ref(),
                    );
                }
            }
        } else {
            isolated_post_commit_validation(&paths, prepared)
                .unwrap_or_else(|error| fail(&error, parsed.json))
        };
        if let Err(error) = accept_prepared_payload_transaction(prepared, validation) {
            if prepared.transaction.state
                == crate::runtime_adoption::UpgradeTransactionState::OperatorRecoveryRequired
            {
                fail(&error, parsed.json);
            }
            let rollback = rollback_prepared_payload_transaction(
                &paths,
                prepared,
                true,
                "transaction_acceptance_failed",
            );
            fail(
                &rollback
                    .err()
                    .map_or(error.clone(), |rollback| format!("{error}; {rollback}")),
                parsed.json,
            );
        }
        if !isolated_root {
            if let Err(error) =
                crate::session_supervisor::rebind_supervisors_after_accepted_upgrade(&paths.binary)
            {
                fail(
                    &format!(
                        "workstation upgrade was accepted but supervisor rebinding failed: {error}"
                    ),
                    parsed.json,
                );
            }
            phases.push("session-supervisors-rebound");
        }
    }

    let mutated = parsed.mode == InstallMode::Apply;

    let report = WorkstationInstallReport {
        schema_version: INSTALL_SCHEMA_VERSION,
        success: !session_refresh_required,
        complete: workstation_ready,
        state: if workstation_ready {
            "ready"
        } else if session_refresh_required {
            "relogin_required"
        } else if parsed.mode == InstallMode::Apply {
            "payload_installed"
        } else {
            "planned"
        },
        mode: match parsed.mode {
            InstallMode::DryRun => "dry-run",
            InstallMode::Apply => "apply",
        },
        mutated,
        ready: workstation_ready,
        version: env!("CARGO_PKG_VERSION"),
        dashboard_port: parsed.dashboard_port,
        guacamole_port: parsed.guacamole_port,
        host_plan,
        paths: WorkstationPaths {
            root: root.display().to_string(),
            binary: paths.binary.display().to_string(),
            support_dir: paths.support_dir.display().to_string(),
            unit_dir: paths.unit_dir.display().to_string(),
            guacamole_state_dir: paths.guacamole_state_dir.display().to_string(),
            guacamole_secret_file: paths.guacamole_secret_file.display().to_string(),
        },
        phases,
        host_prepared,
        session_refresh_required,
        runtime_census_transaction,
        reconcile_receipt,
        next_action,
    };

    if parsed.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        );
    } else {
        println!(
            "Workstation install {} complete for agent-browser {}.",
            report.mode, report.version
        );
        println!("  Binary: {}", report.paths.binary);
        println!("  Support: {}", report.paths.support_dir);
        println!("  Units: {}", report.paths.unit_dir);
        println!("  Ready: {}", if report.ready { "yes" } else { "no" });
        println!("  Next: {}", report.next_action);
    }
    if session_refresh_required {
        exit(75);
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReconcileStep {
    name: &'static str,
    success: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkstationReconcileReport {
    schema_version: &'static str,
    success: bool,
    version: &'static str,
    steps: Vec<ReconcileStep>,
    route_pool: Vec<Value>,
    receipt_path: String,
}

fn run_workstation_reconcile(json: bool) {
    match reconcile_runtime_maintenance() {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        ),
        Ok(report) => println!(
            "Runtime reconciliation {}: {}.",
            report
                .get("state")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            report
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or("no summary")
        ),
        Err(error) => fail(&error, json),
    }
}

fn reconcile_runtime_maintenance() -> Result<Value, String> {
    if !cfg!(target_os = "linux") {
        return Err("runtime reconciliation is only supported on Linux".to_string());
    }
    let root = workstation_root()?;
    let paths = install_paths(&root);
    let receipt_path = root.join(".agent-browser/convergence/runtime-monitor.json");
    let previous = read_runtime_monitor_receipt(&receipt_path)?;
    let now = runtime_monitor_epoch_seconds();
    let previous_failures = previous
        .as_ref()
        .and_then(|value| value.get("consecutiveFailures"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let next_eligible = previous
        .as_ref()
        .and_then(|value| value.get("nextEligibleAtEpochSeconds"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if next_eligible > now {
        let report = serde_json::json!({
            "schemaVersion": "agent-browser.runtime-monitor.v1",
            "success": true,
            "state": "backoff",
            "skipped": true,
            "observedAt": runtime_adoption_timestamp(),
            "observedAtEpochSeconds": now,
            "consecutiveFailures": previous_failures,
            "nextEligibleAtEpochSeconds": next_eligible,
            "incident": previous.as_ref().and_then(|value| value.get("incident")).cloned(),
            "summary": "effect backoff remains active",
            "receiptPath": receipt_path.display().to_string(),
        });
        write_private_json_atomic(&receipt_path, &report)?;
        return Ok(report);
    }

    let started_at = runtime_adoption_timestamp();
    let result: Result<Value, String> = (|| {
        let _lock = WorkstationLock::acquire(&root)?;
        require_installed_payload(&paths)?;
        ensure_route_users(&paths, &workstation_command_env(&paths))?;
        use crate::native::service_store::ServiceStateRepository;
        let repository =
            crate::native::service_store::LockedServiceStateRepository::default_json()?;
        let service_effects = repository.mutate(|state| {
            let process_gc = crate::native::service_resources::service_gc_unattended_response(state);
            if process_gc.get("applied").and_then(Value::as_bool) != Some(true) {
                return Err(format!(
                    "unattended_process_gc_failed:{}",
                    process_gc
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("candidate_effect_failed")
                ));
            }
            let retained_state = crate::native::service_retained_state::service_commands::automatically_prune_retained_service_state(state, true);
            let profile_errors = retained_state
                .pointer("/removed/orphanedProfileErrors")
                .and_then(Value::as_object)
                .map(|errors| errors.len())
                .unwrap_or(0);
            if profile_errors != 0 {
                return Err(format!(
                    "unattended_profile_retention_failed:{profile_errors}"
                ));
            }
            let resources = crate::native::service_resources::service_resources_response(state);
            let lifecycle_record_count = state
                .runtime_owner_registry
                .lifecycle_records
                .len();
            let missing_cleanup_obligation_count = state
                .browsers
                .keys()
                .filter(|browser_id| {
                    state.browser_process_identities.contains_key(*browser_id)
                        && !state
                            .runtime_owner_registry
                            .lifecycle_records
                            .contains_key(*browser_id)
                })
                .count();
            Ok(serde_json::json!({
                "processGc": process_gc,
                "retainedState": retained_state,
                "resources": resources,
                "cleanupObligations": {
                    "trackedCount": lifecycle_record_count,
                    "missingCount": missing_cleanup_obligation_count,
                },
            }))
        })?;
        let generation_gc = workstation_generation_gc_locked(&root, &paths, InstallMode::Apply)?;
        Ok(serde_json::json!({
            "routeUsers": {
                "reconciled": true,
                "credentialContract": "non_pam_sha512",
            },
            "service": service_effects,
            "generations": generation_gc,
        }))
    })();

    match result {
        Ok(effects) => {
            let report = serde_json::json!({
                "schemaVersion": "agent-browser.runtime-monitor.v1",
                "success": true,
                "state": "healthy",
                "skipped": false,
                "startedAt": started_at,
                "completedAt": runtime_adoption_timestamp(),
                "observedAtEpochSeconds": runtime_monitor_epoch_seconds(),
                "consecutiveFailures": 0,
                "nextEligibleAtEpochSeconds": Value::Null,
                "incident": Value::Null,
                "effects": effects,
                "summary": "unattended reconciliation and retention completed",
                "receiptPath": receipt_path.display().to_string(),
            });
            write_private_json_atomic(&receipt_path, &report)?;
            Ok(report)
        }
        Err(error) if runtime_monitor_blocked_by_active_upgrade(&root, &error) => {
            let report = serde_json::json!({
                "schemaVersion": "agent-browser.runtime-monitor.v1",
                "success": true,
                "state": "healthy",
                "skipped": true,
                "startedAt": started_at,
                "completedAt": runtime_adoption_timestamp(),
                "observedAtEpochSeconds": runtime_monitor_epoch_seconds(),
                "consecutiveFailures": 0,
                "nextEligibleAtEpochSeconds": Value::Null,
                "incident": Value::Null,
                "effects": {
                    "activeUpgrade": {
                        "state": "in_progress",
                        "maintenanceOwner": "workstation_install",
                    },
                },
                "summary": "active workstation upgrade owns reconciliation; unattended pass skipped",
                "receiptPath": receipt_path.display().to_string(),
            });
            write_private_json_atomic(&receipt_path, &report)?;
            Ok(report)
        }
        Err(error) => {
            let consecutive_failures = previous_failures.saturating_add(1);
            let backoff_seconds = runtime_monitor_backoff_seconds(consecutive_failures);
            let next_eligible_at = now.saturating_add(backoff_seconds);
            let incident = (consecutive_failures >= 3).then(|| serde_json::json!({
                "type": "runtime_reconciliation_repeated_failure",
                "severity": "error",
                "state": "active",
                "failureCount": consecutive_failures,
                "reason": error.clone(),
                "recommendedAction": "Inspect the runtime monitor receipt and install doctor before retrying effects.",
            }));
            let report = serde_json::json!({
                "schemaVersion": "agent-browser.runtime-monitor.v1",
                "success": false,
                "state": if incident.is_some() { "incident" } else { "degraded" },
                "skipped": false,
                "startedAt": started_at,
                "completedAt": runtime_adoption_timestamp(),
                "observedAtEpochSeconds": runtime_monitor_epoch_seconds(),
                "consecutiveFailures": consecutive_failures,
                "nextEligibleAtEpochSeconds": next_eligible_at,
                "backoffSeconds": backoff_seconds,
                "incident": incident,
                "error": error,
                "summary": "unattended reconciliation failed and entered bounded backoff",
                "receiptPath": receipt_path.display().to_string(),
            });
            write_private_json_atomic(&receipt_path, &report)?;
            Err(format!(
                "runtime reconciliation failed; receipt: {}",
                receipt_path.display()
            ))
        }
    }
}

fn read_runtime_monitor_receipt(path: &Path) -> Result<Option<Value>, String> {
    match fs::read(path) {
        Ok(body) => serde_json::from_slice(&body).map(Some).map_err(|error| {
            format!(
                "Invalid runtime monitor receipt {}: {error}",
                path.display()
            )
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "Unable to read runtime monitor receipt {}: {error}",
            path.display()
        )),
    }
}

fn runtime_monitor_epoch_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn runtime_monitor_backoff_seconds(consecutive_failures: u64) -> u64 {
    let exponent = consecutive_failures.saturating_sub(1).min(3) as u32;
    300_u64.saturating_mul(2_u64.saturating_pow(exponent))
}

fn runtime_monitor_blocked_by_active_upgrade(root: &Path, error: &str) -> bool {
    error.starts_with("workstation reconciliation is already active:")
        && root
            .join(".agent-browser/runtime-adoption/admission-drain.json")
            .is_file()
}

pub(crate) fn runtime_monitor_status_json() -> Value {
    let root = match workstation_root() {
        Ok(root) => root,
        Err(error) => {
            return serde_json::json!({
                "schemaVersion": "agent-browser.runtime-monitor-status.v1",
                "ready": false,
                "state": "unavailable",
                "error": error,
            });
        }
    };
    let receipt_path = root.join(".agent-browser/convergence/runtime-monitor.json");
    let now = runtime_monitor_epoch_seconds();
    match read_runtime_monitor_receipt(&receipt_path) {
        Ok(Some(receipt)) => {
            let observed_at = receipt
                .get("observedAtEpochSeconds")
                .and_then(Value::as_u64);
            let age_seconds = observed_at.map(|observed| now.saturating_sub(observed));
            let fresh = age_seconds.is_some_and(|age| age <= 900);
            let healthy = receipt.get("state").and_then(Value::as_str) == Some("healthy")
                && receipt.get("success").and_then(Value::as_bool) == Some(true);
            serde_json::json!({
                "schemaVersion": "agent-browser.runtime-monitor-status.v1",
                "ready": fresh && healthy,
                "state": if !fresh { "stale" } else { receipt.get("state").and_then(Value::as_str).unwrap_or("unknown") },
                "fresh": fresh,
                "ageSeconds": age_seconds,
                "maximumAgeSeconds": 900,
                "receiptPath": receipt_path,
                "receipt": receipt,
            })
        }
        Ok(None) => {
            let paths = install_paths(&root);
            let generation_age = fs::symlink_metadata(&paths.current_selector)
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| std::time::SystemTime::now().duration_since(modified).ok())
                .map(|age| age.as_secs());
            let grace = generation_age.is_some_and(|age| age <= 900);
            serde_json::json!({
                "schemaVersion": "agent-browser.runtime-monitor-status.v1",
                "ready": grace,
                "state": if grace { "bootstrap_grace" } else { "missing" },
                "fresh": false,
                "ageSeconds": Value::Null,
                "maximumAgeSeconds": 900,
                "receiptPath": receipt_path,
            })
        }
        Err(error) => serde_json::json!({
            "schemaVersion": "agent-browser.runtime-monitor-status.v1",
            "ready": false,
            "state": "invalid",
            "fresh": false,
            "maximumAgeSeconds": 900,
            "receiptPath": receipt_path,
            "error": error,
        }),
    }
}

#[cfg(test)]
fn workstation_reconcile_failure_receipt(error: &str) -> Value {
    let recorded_at_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    serde_json::json!({
        "schemaVersion": "agent-browser.workstation-reconcile-failure.v1",
        "success": false,
        "version": env!("CARGO_PKG_VERSION"),
        "recordedAtUnixMs": recorded_at_unix_ms,
        "error": error,
    })
}

fn reconcile_workstation_locked_for_upgrade(
    root: &Path,
    paths: &InstallPaths,
    expected_upgrade: Option<&crate::runtime_adoption::UpgradeTransaction>,
    transitional_source_sessions: &[String],
) -> Result<WorkstationReconcileReport, String> {
    require_installed_payload(paths)?;
    require_effective_groups()?;
    let support_root = &paths.support_dir;
    let mut command_env = workstation_command_env(paths);
    if let Some(expected_upgrade) = expected_upgrade {
        command_env.push((
            crate::runtime_adoption::RUNTIME_ADMISSION_TRANSACTION_ID_ENV.to_string(),
            expected_upgrade.transaction_id.clone(),
        ));
        command_env.push((
            crate::runtime_adoption::RUNTIME_ADMISSION_TRANSACTION_REVISION_ENV.to_string(),
            expected_upgrade.revision.to_string(),
        ));
    }
    let quiesced_user_units = quiesce_existing_user_units(paths)?;
    let reconcile_result = reconcile_workstation_after_quiesce(
        root,
        paths,
        support_root,
        command_env.clone(),
        expected_upgrade,
        transitional_source_sessions,
    );
    complete_reconcile_with_unit_restore(reconcile_result, || {
        restore_previously_active_user_units(
            paths,
            support_root,
            &command_env,
            &quiesced_user_units,
        )
    })
}

fn reconcile_workstation_after_quiesce(
    root: &Path,
    paths: &InstallPaths,
    support_root: &Path,
    command_env: Vec<(String, String)>,
    expected_upgrade: Option<&crate::runtime_adoption::UpgradeTransaction>,
    transitional_source_sessions: &[String],
) -> Result<WorkstationReconcileReport, String> {
    let scripts_dir = support_root.join("scripts");
    let guacamole_dir = support_root.join("guacamole");
    let guacamole_env = paths.guacamole_state_dir.join(".env");
    let mut steps = vec![ReconcileStep {
        name: "existing-user-units-quiesced",
        success: true,
    }];
    inject_failure("existing-user-units-quiesced")?;

    run_required(
        paths
            .binary
            .to_str()
            .ok_or_else(|| "invalid installed agent-browser path".to_string())?,
        &["install"],
        support_root,
        &command_env,
        false,
        "install Chrome for Testing",
    )?;
    steps.push(ReconcileStep {
        name: "chrome-ready",
        success: true,
    });

    let volume_inspect = run_status(
        "docker",
        &["volume", "inspect", "agent-browser-guacamole-postgres-data"],
        support_root,
        &command_env,
        false,
    );
    if volume_inspect.is_err() {
        run_required(
            "docker",
            &["volume", "create", "agent-browser-guacamole-postgres-data"],
            support_root,
            &command_env,
            false,
            "create PostgreSQL named volume",
        )?;
    }
    steps.push(ReconcileStep {
        name: "postgres-volume-ready",
        success: true,
    });

    reconcile_postgres_password_from_retained_container(paths, support_root, &command_env)?;
    steps.push(ReconcileStep {
        name: "postgres-credentials-aligned",
        success: true,
    });

    let retained_compose_project =
        retained_postgres_compose_project_name(support_root, &command_env)?;
    let compose_args = guacamole_compose_args(
        &guacamole_env,
        &paths.guacamole_secret_file,
        &guacamole_dir.join("compose.yml"),
        retained_compose_project.as_deref(),
    );
    run_required_owned(
        "docker",
        &compose_args,
        support_root,
        &command_env,
        false,
        "start pinned Guacamole stack",
    )?;
    steps.push(ReconcileStep {
        name: "guacamole-stack-ready",
        success: true,
    });

    let durability_script = scripts_dir.join("guacamole-postgres-durability.sh");
    run_required(
        "bash",
        &[
            durability_script
                .to_str()
                .ok_or_else(|| "invalid installed durability script path".to_string())?,
            "record-identity",
        ],
        support_root,
        &command_env,
        false,
        "record PostgreSQL continuity identity",
    )?;
    run_required(
        "bash",
        &[
            durability_script
                .to_str()
                .ok_or_else(|| "invalid installed durability script path".to_string())?,
            "status",
        ],
        support_root,
        &command_env,
        false,
        "verify PostgreSQL continuity",
    )?;
    steps.push(ReconcileStep {
        name: "postgres-continuity-ready",
        success: true,
    });

    ensure_route_users(paths, &command_env)?;
    steps.push(ReconcileStep {
        name: "route-users-ready",
        success: true,
    });

    let header_user = env::var("USER").unwrap_or_else(|_| "agent-browser".to_string());
    let guacamole_port = env_file_value(&guacamole_env, "AGENT_BROWSER_GUACAMOLE_HTTP_PORT")
        .unwrap_or_else(|| DEFAULT_GUACAMOLE_PORT.to_string());
    ensure_guacamole_header_user(&header_user, &guacamole_port, support_root, &command_env)?;
    steps.push(ReconcileStep {
        name: "guacamole-header-user-ready",
        success: true,
    });

    run_required(
        "bash",
        &[scripts_dir
            .join("sync-rdp-guac-route-specific-user-pool.sh")
            .to_str()
            .ok_or_else(|| "invalid installed route sync path".to_string())?],
        support_root,
        &command_env,
        true,
        "synchronize canonical Guacamole route records",
    )?;
    steps.push(ReconcileStep {
        name: "route-records-ready",
        success: true,
    });

    run_required(
        "node",
        &[scripts_dir
            .join("open-rdp-guac-route-displays.js")
            .to_str()
            .ok_or_else(|| "invalid installed route opener path".to_string())?],
        support_root,
        &command_env,
        true,
        "open canonical Guacamole route displays",
    )?;
    steps.push(ReconcileStep {
        name: "route-displays-opened",
        success: true,
    });

    run_required(
        "bash",
        &[
            scripts_dir
                .join("grant-rdp-route-display-access.sh")
                .to_str()
                .ok_or_else(|| "invalid installed display grant path".to_string())?,
            "--apply",
        ],
        support_root,
        &command_env,
        true,
        "grant operator route display access",
    )?;
    steps.push(ReconcileStep {
        name: "route-display-access-ready",
        success: true,
    });

    let route_pool = route_readiness(&scripts_dir, support_root, &command_env)?;
    reconcile_authoritative_route_pool(&paths.binary, &route_pool, support_root, &command_env)?;
    steps.push(ReconcileStep {
        name: "authoritative-route-pool-projected",
        success: true,
    });

    update_canonical_runtime_env(root, &route_pool)?;
    activate_user_units(paths, support_root, &command_env)?;
    steps.push(ReconcileStep {
        name: "user-services-active",
        success: true,
    });
    if expected_upgrade.is_some() {
        restart_stable_dashboard_ingress(paths, support_root, &command_env)?;
        steps.push(ReconcileStep {
            name: "stable-dashboard-ingress-refreshed",
            success: true,
        });
    }

    let final_route_pool = route_readiness(&scripts_dir, support_root, &command_env)?;
    reconcile_authoritative_route_pool(
        &paths.binary,
        &final_route_pool,
        support_root,
        &command_env,
    )?;
    steps.push(ReconcileStep {
        name: "final-route-pool-projected",
        success: true,
    });

    verify_final_doctors(
        paths,
        support_root,
        &command_env,
        expected_upgrade,
        transitional_source_sessions,
    )?;
    steps.push(ReconcileStep {
        name: "final-doctors-ready",
        success: true,
    });

    let receipt_path = root.join(".agent-browser/convergence/workstation-latest.json");
    let report = WorkstationReconcileReport {
        schema_version: "agent-browser.workstation-reconcile.v1",
        success: true,
        version: env!("CARGO_PKG_VERSION"),
        steps,
        route_pool: final_route_pool,
        receipt_path: receipt_path.display().to_string(),
    };
    write_private_json(&receipt_path, &report)?;
    Ok(report)
}

fn ensure_guacamole_header_user(
    header_user: &str,
    guacamole_port: &str,
    support_root: &Path,
    command_env: &[(String, String)],
) -> Result<(), String> {
    if header_user.is_empty()
        || !header_user
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._@-".contains(character))
    {
        return Err("Guacamole header user contains unsupported characters".to_string());
    }

    run_required_owned(
        "curl",
        &[
            "--fail".to_string(),
            "--silent".to_string(),
            "--show-error".to_string(),
            "--connect-timeout".to_string(),
            "5".to_string(),
            "--max-time".to_string(),
            "10".to_string(),
            "--retry".to_string(),
            "30".to_string(),
            "--retry-delay".to_string(),
            "3".to_string(),
            "--retry-max-time".to_string(),
            "180".to_string(),
            "--retry-all-errors".to_string(),
            "--output".to_string(),
            "/dev/null".to_string(),
            format!("http://127.0.0.1:{guacamole_port}/guacamole/"),
        ],
        support_root,
        command_env,
        false,
        "wait for the local Guacamole application",
    )?;

    // Header authentication may successfully create the PostgreSQL account
    // while an overlapping first-start request returns a duplicate-key 500.
    // Treat the database postcondition as authoritative, not the HTTP status.
    let token_request = run_status_owned(
        "curl",
        &[
            "--fail".to_string(),
            "--silent".to_string(),
            "--show-error".to_string(),
            "--connect-timeout".to_string(),
            "5".to_string(),
            "--max-time".to_string(),
            "30".to_string(),
            "--request".to_string(),
            "POST".to_string(),
            "--header".to_string(),
            format!("Remote-User: {header_user}"),
            "--header".to_string(),
            "Content-Type: application/x-www-form-urlencoded".to_string(),
            "--data".to_string(),
            String::new(),
            "--output".to_string(),
            "/dev/null".to_string(),
            format!("http://127.0.0.1:{guacamole_port}/guacamole/api/tokens"),
        ],
        support_root,
        command_env,
        true,
    );

    let query = format!(
        r#"PGPASSWORD="$POSTGRES_PASSWORD" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc "SELECT COUNT(*) FROM guacamole_entity e JOIN guacamole_user u ON u.entity_id = e.entity_id WHERE e.type::text = \$\$USER\$\$ AND e.name = \$\${header_user}\$\$;""#
    );
    let entity_output = run_status(
        "docker",
        &[
            "exec",
            "agent-browser-guacamole-postgres",
            "sh",
            "-c",
            &query,
        ],
        support_root,
        command_env,
        false,
    )
    .map_err(|error| format!("verify the local Guacamole header user: {error}"))?;
    if guacamole_header_user_ready(&entity_output.stdout) {
        return Ok(());
    }
    if let Err(error) = token_request {
        return Err(format!("create the local Guacamole header user: {error}"));
    }
    Err(
        "Guacamole header authentication returned without materializing its database user"
            .to_string(),
    )
}

fn guacamole_header_user_ready(stdout: &[u8]) -> bool {
    String::from_utf8_lossy(stdout).trim() == "1"
}

fn run_workstation_backup(json: bool) {
    let result = (|| {
        let root = workstation_root()?;
        let paths = install_paths(&root);
        require_installed_payload(&paths)?;
        let support_root = &paths.support_dir;
        let script = support_root.join("scripts/guacamole-postgres-durability.sh");
        let command_env = workstation_command_env(&paths);
        run_required(
            "bash",
            &[
                script
                    .to_str()
                    .ok_or_else(|| "invalid installed backup script path".to_string())?,
                "backup",
            ],
            support_root,
            &command_env,
            true,
            "back up Guacamole PostgreSQL",
        )?;
        Ok::<_, String>(serde_json::json!({
            "schemaVersion": "agent-browser.workstation-backup.v1",
            "success": true,
            "version": env!("CARGO_PKG_VERSION")
        }))
    })();
    match result {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        ),
        Ok(_) => println!("Guacamole PostgreSQL backup complete."),
        Err(error) => fail(&error, json),
    }
}

fn build_host_plan(fixture_root: bool, root: &Path) -> HostPlan {
    let effective_groups = current_process_groups();
    if fixture_root {
        return HostPlan {
            supported: true,
            fixture_root: true,
            architecture: env::consts::ARCH.to_string(),
            operating_system: "isolated-fixture".to_string(),
            missing_commands: Vec::new(),
            available_disk_bytes: None,
            minimum_disk_bytes: MIN_WORKSTATION_FREE_DISK_BYTES,
            disk_space_ready: true,
            effective_groups,
            actions: workstation_host_actions(),
        };
    }

    let operating_system = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let ubuntu_2404 = operating_system
        .lines()
        .any(|line| line.trim() == "ID=ubuntu")
        && operating_system.lines().any(|line| {
            line.trim()
                .strip_prefix("VERSION_ID=")
                .map(|value| value.trim_matches('"') == "24.04")
                .unwrap_or(false)
        });
    let required_commands = ["apt-get", "apt-cache", "bash", "sudo", "systemctl"];
    let missing_commands = required_commands
        .iter()
        .filter(|command| !command_exists(command))
        .map(|command| (*command).to_string())
        .collect::<Vec<_>>();
    let available_disk_bytes = available_disk_bytes(root);
    let disk_space_ready = workstation_disk_space_ready(available_disk_bytes);
    HostPlan {
        supported: env::consts::ARCH == "x86_64"
            && ubuntu_2404
            && missing_commands.is_empty()
            && disk_space_ready,
        fixture_root: false,
        architecture: env::consts::ARCH.to_string(),
        operating_system: if ubuntu_2404 {
            "ubuntu-24.04".to_string()
        } else {
            "unsupported".to_string()
        },
        missing_commands,
        available_disk_bytes,
        minimum_disk_bytes: MIN_WORKSTATION_FREE_DISK_BYTES,
        disk_space_ready,
        effective_groups,
        actions: workstation_host_actions(),
    }
}

fn workstation_host_actions() -> Vec<&'static str> {
    vec![
        "validate package candidates and no-removal simulation",
        "require at least 6 GiB of free disk capacity before mutation",
        "authorize sudo exactly once when host changes are required",
        "install browser and remote-view host dependencies",
        "install and verify the narrow privileged helper",
        "configure agent-browser and docker groups",
        "enable user lingering plus Docker and XRDP services",
        "stop for a fresh login when group membership is not effective",
    ]
}

fn workstation_disk_space_ready(available_disk_bytes: Option<u64>) -> bool {
    available_disk_bytes
        .map(|available| available >= MIN_WORKSTATION_FREE_DISK_BYTES)
        .unwrap_or(false)
}

#[cfg(target_family = "unix")]
fn available_disk_bytes(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;

    let path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stats = MaybeUninit::<libc::statvfs>::uninit();
    if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
        return None;
    }
    let stats = unsafe { stats.assume_init() };
    Some(statvfs_available_bytes(stats.f_bavail, stats.f_frsize))
}

#[cfg(target_family = "unix")]
fn statvfs_available_bytes<Blocks, Bytes>(available_blocks: Blocks, fragment_bytes: Bytes) -> u64
where
    Blocks: Into<u64>,
    Bytes: Into<u64>,
{
    available_blocks
        .into()
        .saturating_mul(fragment_bytes.into())
}

#[cfg(not(target_family = "unix"))]
fn available_disk_bytes(_path: &Path) -> Option<u64> {
    None
}

fn command_exists(command: &str) -> bool {
    env::var_os("PATH")
        .map(|path| env::split_paths(&path).any(|directory| directory.join(command).is_file()))
        .unwrap_or(false)
}

fn current_process_groups() -> Vec<String> {
    Command::new("id")
        .arg("-nG")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .split_whitespace()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn current_process_has_group(group: &str) -> bool {
    current_process_groups()
        .iter()
        .any(|candidate| candidate == group)
}

fn require_effective_groups() -> Result<(), String> {
    let missing = ["agent-browser", "docker"]
        .iter()
        .filter(|group| !current_process_has_group(group))
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "workstation reconciliation requires a fresh login with effective groups: {}",
        missing.join(", ")
    ))
}

fn require_installed_payload(paths: &InstallPaths) -> Result<(), String> {
    validate_selected_generation_if_present(paths)?;
    let required = [
        paths.binary.clone(),
        paths.support_dir.join("manifest.json"),
        paths.support_dir.join("guacamole/compose.yml"),
        paths
            .support_dir
            .join("scripts/smoke-rdp-guac-route-pool-readiness.js"),
        paths.guacamole_secret_file.clone(),
    ];
    let missing = required
        .iter()
        .filter(|path| !path.is_file())
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "workstation payload is incomplete; missing: {}",
        missing.join(", ")
    ))
}

fn validate_selected_generation_if_present(paths: &InstallPaths) -> Result<(), String> {
    match fs::symlink_metadata(&paths.current_selector) {
        Ok(_) => validate_generation_install_preconditions(paths),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "Unable to inspect current runtime generation selector {}: {error}",
            paths.current_selector.display()
        )),
    }
}

fn workstation_command_env(paths: &InstallPaths) -> Vec<(String, String)> {
    let path = format!(
        "{}:{}",
        paths
            .binary
            .parent()
            .unwrap_or(Path::new("/usr/local/bin"))
            .display(),
        env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".to_string())
    );
    vec![
        ("PATH".to_string(), path),
        (
            "AGENT_BROWSER_BIN".to_string(),
            paths.binary.display().to_string(),
        ),
        (
            "AGENT_BROWSER_ROUTE_DISPLAY_AGENT_BROWSER_CMD".to_string(),
            paths.binary.display().to_string(),
        ),
        (
            "AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT".to_string(),
            paths.support_dir.join("scripts").display().to_string(),
        ),
        (
            "AGENT_BROWSER_GUACAMOLE_DIR".to_string(),
            paths.support_dir.join("guacamole").display().to_string(),
        ),
        (
            "AGENT_BROWSER_GUACAMOLE_SECRET_FILE".to_string(),
            paths.guacamole_secret_file.display().to_string(),
        ),
        (
            "AGENT_BROWSER_RDP_ROUTE_POOL_JSON".to_string(),
            String::new(),
        ),
    ]
}

fn apply_command_environment(command: &mut Command, command_env: &[(String, String)]) {
    for (key, value) in command_env {
        command.env(key, value);
    }
}

fn run_status(
    command: &str,
    args: &[&str],
    current_dir: &Path,
    command_env: &[(String, String)],
    sensitive: bool,
) -> Result<Output, String> {
    let output = run_observed(command, args, current_dir, command_env)?;
    if output.status.success() {
        return Ok(output);
    }
    if sensitive {
        return Err(format!(
            "Sensitive workstation helper {command} failed with status {}; output was redacted",
            output.status
        ));
    }
    Err(format!(
        "Workstation command {command} failed with status {}: {}{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn run_observed(
    command: &str,
    args: &[&str],
    current_dir: &Path,
    command_env: &[(String, String)],
) -> Result<Output, String> {
    let mut process = Command::new(command);
    process
        .args(args)
        .current_dir(current_dir)
        .stdin(Stdio::null());
    apply_command_environment(&mut process, command_env);
    process
        .output()
        .map_err(|error| format!("Unable to run installed workstation command {command}: {error}"))
}

fn run_required(
    command: &str,
    args: &[&str],
    current_dir: &Path,
    command_env: &[(String, String)],
    sensitive: bool,
    label: &str,
) -> Result<(), String> {
    run_status(command, args, current_dir, command_env, sensitive)
        .map(|_| ())
        .map_err(|error| format!("{label}: {error}"))
}

fn run_required_owned(
    command: &str,
    args: &[String],
    current_dir: &Path,
    command_env: &[(String, String)],
    sensitive: bool,
    label: &str,
) -> Result<(), String> {
    let borrowed = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_required(
        command,
        &borrowed,
        current_dir,
        command_env,
        sensitive,
        label,
    )
}

fn run_status_owned(
    command: &str,
    args: &[String],
    current_dir: &Path,
    command_env: &[(String, String)],
    sensitive: bool,
) -> Result<Output, String> {
    let borrowed = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_status(command, &borrowed, current_dir, command_env, sensitive)
}

fn secret_values(secret_file: &Path) -> Result<std::collections::HashMap<String, String>, String> {
    let metadata = fs::metadata(secret_file).map_err(display_io(
        "inspect protected Guacamole secrets",
        secret_file,
    ))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(format!(
                "Guacamole secret file permissions are broader than 0600: {}",
                secret_file.display()
            ));
        }
    }
    let contents = fs::read_to_string(secret_file)
        .map_err(display_io("read protected Guacamole secrets", secret_file))?;
    Ok(contents
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect())
}

/// Keeps a retained PostgreSQL cluster and the protected Compose environment
/// on the same credential before any Guacamole container can be recreated.
/// PostgreSQL ignores `POSTGRES_PASSWORD` when its data directory already
/// exists, so the retained container value must win over a drifted file.
fn reconcile_postgres_password_from_retained_container(
    paths: &InstallPaths,
    support_root: &Path,
    command_env: &[(String, String)],
) -> Result<bool, String> {
    let container_name = "agent-browser-guacamole-postgres";
    if !retained_postgres_container_exists(support_root, command_env)? {
        return Ok(false);
    }

    let inspect = run_status(
        "docker",
        &[
            "container",
            "inspect",
            "--format",
            "{{range .Config.Env}}{{println .}}{{end}}",
            container_name,
        ],
        support_root,
        command_env,
        true,
    )?;
    let password = container_environment_value(&inspect.stdout, "POSTGRES_PASSWORD")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Retained Guacamole PostgreSQL container has no usable POSTGRES_PASSWORD".to_string()
        })?;
    reconcile_protected_secret_value(&paths.guacamole_secret_file, "POSTGRES_PASSWORD", &password)
}

fn retained_postgres_container_exists(
    support_root: &Path,
    command_env: &[(String, String)],
) -> Result<bool, String> {
    let container_list = run_status(
        "docker",
        &[
            "container",
            "ls",
            "--all",
            "--filter",
            "name=^/agent-browser-guacamole-postgres$",
            "--format",
            "{{.Names}}",
        ],
        support_root,
        command_env,
        false,
    )?;
    Ok(String::from_utf8_lossy(&container_list.stdout)
        .lines()
        .any(|name| name.trim() == "agent-browser-guacamole-postgres"))
}

fn retained_postgres_compose_project_name(
    support_root: &Path,
    command_env: &[(String, String)],
) -> Result<Option<String>, String> {
    if !retained_postgres_container_exists(support_root, command_env)? {
        return Ok(None);
    }
    let output = run_status(
        "docker",
        &[
            "container",
            "inspect",
            "--format",
            "{{index .Config.Labels \"com.docker.compose.project\"}}",
            "agent-browser-guacamole-postgres",
        ],
        support_root,
        command_env,
        false,
    )?;
    let project = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let valid = project
        .chars()
        .next()
        .map(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        .unwrap_or(false)
        && project.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '-'
                || character == '_'
        });
    if !valid {
        return Err(
            "Retained Guacamole PostgreSQL container has no usable Compose project label"
                .to_string(),
        );
    }
    Ok(Some(project))
}

fn guacamole_compose_args(
    environment_file: &Path,
    secret_file: &Path,
    compose_file: &Path,
    retained_project: Option<&str>,
) -> Vec<String> {
    let mut args = vec!["compose".to_string()];
    if let Some(project) = retained_project {
        args.extend(["--project-name".to_string(), project.to_string()]);
    }
    args.extend([
        "--env-file".to_string(),
        environment_file.display().to_string(),
        "--env-file".to_string(),
        secret_file.display().to_string(),
        "-f".to_string(),
        compose_file.display().to_string(),
        "up".to_string(),
        "-d".to_string(),
        "--wait".to_string(),
    ]);
    args
}

fn container_environment_value(output: &[u8], key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    String::from_utf8_lossy(output)
        .lines()
        .find_map(|line| line.strip_prefix(&prefix).map(str::to_string))
}

fn reconcile_protected_secret_value(
    secret_file: &Path,
    key: &str,
    value: &str,
) -> Result<bool, String> {
    if value.contains('\n') || value.contains('\r') {
        return Err(format!(
            "Protected Guacamole value for {key} contains a newline"
        ));
    }
    let contents = fs::read_to_string(secret_file)
        .map_err(display_io("read protected Guacamole secrets", secret_file))?;
    let mut lines = contents.lines().map(str::to_string).collect::<Vec<_>>();
    let matches = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            line.split_once('=')
                .filter(|(candidate, _)| candidate.trim() == key)
                .map(|(_, current)| (index, current.to_string()))
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!(
            "Protected Guacamole secret file must contain exactly one {key} entry: {}",
            secret_file.display()
        ));
    }
    if matches[0].1 == value {
        set_private_file(secret_file)?;
        return Ok(false);
    }
    lines[matches[0].0] = format!("{key}={value}");
    fs::write(secret_file, format!("{}\n", lines.join("\n"))).map_err(display_io(
        "reconcile protected Guacamole secrets",
        secret_file,
    ))?;
    set_private_file(secret_file)?;
    Ok(true)
}

fn ensure_route_users(
    paths: &InstallPaths,
    command_env: &[(String, String)],
) -> Result<(), String> {
    let values = secret_values(&paths.guacamole_secret_file)?;
    let helper = "/usr/local/libexec/agent-browser/agent-browser-privileged-helper";
    for (user_key, password_key, expected_user) in [
        (
            "XRDP_AGENT_BROWSER_ROUTE_A_USERNAME",
            "XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD",
            "agent-browser-rdp-a",
        ),
        (
            "XRDP_AGENT_BROWSER_ROUTE_B_USERNAME",
            "XRDP_AGENT_BROWSER_ROUTE_B_PASSWORD",
            "agent-browser-rdp-b",
        ),
    ] {
        let user = values
            .get(user_key)
            .ok_or_else(|| format!("Missing required route username key {user_key}"))?;
        let password = values
            .get(password_key)
            .ok_or_else(|| format!("Missing required route password key {password_key}"))?;
        if user != expected_user || password.is_empty() {
            return Err(format!(
                "Route credential identity is invalid for {expected_user}"
            ));
        }
        let mut process = Command::new("sudo");
        process
            .args(["-n", helper, "ensure-rdp-route-user", "--user", user])
            .current_dir(&paths.support_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        apply_command_environment(&mut process, command_env);
        let mut child = process
            .spawn()
            .map_err(|error| format!("Unable to start protected route-user helper: {error}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(password.as_bytes())
                .and_then(|()| stdin.write_all(b"\n"))
                .map_err(|error| {
                    format!("Unable to provide protected route credential: {error}")
                })?;
        }
        let status = child
            .wait()
            .map_err(|error| format!("Unable to wait for protected route-user helper: {error}"))?;
        if !status.success() {
            return Err(format!(
                "Protected route-user helper failed for {expected_user}; output was redacted"
            ));
        }
    }
    // XRDP resolves PAM users and their session startup file at login time.
    // Restarting sesman here is unnecessary and makes any live route desktop
    // unreachable because the replacement sesman cannot adopt the surviving
    // Xorg session owned by its predecessor.
    Ok(())
}

fn route_readiness(
    scripts_dir: &Path,
    support_root: &Path,
    command_env: &[(String, String)],
) -> Result<Vec<Value>, String> {
    let script = scripts_dir.join("smoke-rdp-guac-route-pool-readiness.js");
    let output = run_status(
        "node",
        &[
            script
                .to_str()
                .ok_or_else(|| "invalid installed route-readiness path".to_string())?,
            "--report-only",
        ],
        support_root,
        command_env,
        false,
    )?;
    let payload: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Route-readiness JSON parse failed: {error}"))?;
    if payload.get("success").and_then(Value::as_bool) != Some(true) {
        return Err("Route readiness did not report success".to_string());
    }
    let route_pool = payload
        .get("routePoolJson")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "Route readiness did not return routePoolJson".to_string())?;
    validate_canonical_route_pool(&route_pool)?;
    Ok(route_pool)
}

fn validate_canonical_route_pool(route_pool: &[Value]) -> Result<(), String> {
    if route_pool.len() < 2 {
        return Err(format!(
            "Canonical route pool must contain at least two routes, found {}",
            route_pool.len()
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut route_ids = std::collections::BTreeSet::new();
    let mut displays = std::collections::BTreeSet::new();
    for (index, route) in route_pool.iter().enumerate() {
        let id = route
            .get("id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("Canonical route at index {index} is missing an id"))?;
        let route_id = route
            .get("routeId")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("Canonical route {id} is missing a routeId"))?;
        let display = route
            .pointer("/target/displayName")
            .and_then(Value::as_str)
            .filter(|display| !display.is_empty())
            .ok_or_else(|| format!("{id} is missing a selected route display"))?;
        if !ids.insert(id) {
            return Err(format!("Canonical route pool contains duplicate id {id}"));
        }
        if !route_ids.insert(route_id) {
            return Err(format!(
                "Canonical route pool contains duplicate routeId {route_id}"
            ));
        }
        if !displays.insert(display) {
            return Err(format!(
                "Canonical route pool resolved multiple routes to display {display}"
            ));
        }
    }
    Ok(())
}

fn reconcile_authoritative_route_pool(
    binary: &Path,
    route_pool: &[Value],
    support_root: &Path,
    command_env: &[(String, String)],
) -> Result<(), String> {
    let route_json = serde_json::to_string(route_pool)
        .map_err(|error| format!("Unable to serialize authoritative route pool: {error}"))?;
    let reconcile_session = workstation_reconcile_session(command_env);
    let mut reconcile_env = command_env.to_vec();
    reconcile_env.push((
        "AGENT_BROWSER_IDLE_TIMEOUT_MS".to_string(),
        "30000".to_string(),
    ));
    let output = run_status(
        binary
            .to_str()
            .ok_or_else(|| "invalid installed agent-browser path".to_string())?,
        &[
            "--json",
            "--session",
            &reconcile_session,
            "service",
            "reconcile",
            "--authoritative-route-pool-json",
            &route_json,
        ],
        support_root,
        &reconcile_env,
        false,
    )?;
    let payload: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Service reconcile JSON parse failed: {error}"))?;
    validate_service_reconcile_payload(&payload)
}

fn workstation_reconcile_session(command_env: &[(String, String)]) -> String {
    let transaction_id = command_env.iter().find_map(|(key, value)| {
        (key == crate::runtime_adoption::RUNTIME_ADMISSION_TRANSACTION_ID_ENV)
            .then_some(value.as_str())
    });
    transaction_id.map_or_else(
        || "workstation-reconcile".to_string(),
        |transaction_id| {
            let digest = workstation_bytes_sha256(transaction_id.as_bytes());
            format!("workstation-reconcile-{}", &digest[..12])
        },
    )
}

fn validate_service_reconcile_payload(payload: &Value) -> Result<(), String> {
    if payload.get("success").and_then(Value::as_bool) != Some(true) {
        return Err("Service reconcile rejected the authoritative route pool".to_string());
    }
    let conflicts = payload
        .pointer("/data/routePoolRefresh/skippedActiveConflictEntryIds")
        .or_else(|| payload.pointer("/routePoolRefresh/skippedActiveConflictEntryIds"))
        .and_then(Value::as_array);
    if conflicts.is_some_and(|entries| !entries.is_empty()) {
        return Err("Service reconcile preserved an active conflicting route entry".to_string());
    }
    Ok(())
}

fn update_canonical_runtime_env(root: &Path, route_pool: &[Value]) -> Result<(), String> {
    let route_environment =
        crate::native::presentation_inventory::runtime_route_environment(route_pool)?;
    let header_user = env::var("USER").unwrap_or_else(|_| "agent-browser".to_string());
    let mut values = vec![
        ("AGENT_BROWSER_REMOTE_VIEW_PROVIDER", "rdp_gateway"),
        (
            "AGENT_BROWSER_REMOTE_VIEW_URL",
            &route_environment.remote_view_url,
        ),
        ("AGENT_BROWSER_GUACAMOLE_HEADER_USER", &header_user),
        (
            "AGENT_BROWSER_RDP_ROUTE_POOL_JSON",
            &route_environment.route_pool_json,
        ),
    ];
    values.extend(route_environment.legacy_display_env_values()?);
    upsert_env_values(&root.join(".agent-browser/.env"), &values)
}

fn env_file_value(path: &Path, key: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()?
        .lines()
        .filter_map(|line| line.split_once('='))
        .find(|(candidate, _)| candidate.trim() == key)
        .map(|(_, value)| value.trim().trim_matches('"').to_string())
}

fn upsert_env_values(path: &Path, values: &[(&str, &str)]) -> Result<(), String> {
    let mut lines = fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    for (key, value) in values {
        if value.contains('\n') || value.contains('\r') {
            return Err(format!("Environment value for {key} contains a newline"));
        }
        let replacement = format!("{key}={value}");
        if let Some(line) = lines.iter_mut().find(|line| {
            line.split_once('=')
                .map(|(candidate, _)| candidate.trim() == *key)
                .unwrap_or(false)
        }) {
            *line = replacement;
        } else {
            lines.push(replacement);
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(display_io("create agent environment directory", parent))?;
    }
    fs::write(path, format!("{}\n", lines.join("\n")))
        .map_err(display_io("write canonical agent environment", path))
}

fn activate_user_units(
    paths: &InstallPaths,
    support_root: &Path,
    command_env: &[(String, String)],
) -> Result<(), String> {
    run_required(
        "systemctl",
        &["--user", "daemon-reload"],
        support_root,
        command_env,
        false,
        "reload user service units",
    )?;
    reset_failed_user_unit_if_failed(
        "agent-browser-runtime-interlock.service",
        support_root,
        command_env,
    )?;
    run_required(
        "systemctl",
        &[
            "--user",
            "enable",
            "--now",
            "agent-browser-dashboard-backend.service",
            "agent-browser-dashboard.service",
            "agent-browser-runtime-interlock.timer",
            "agent-browser-guacamole-postgres-backup.timer",
        ],
        support_root,
        command_env,
        false,
        "activate workstation user services",
    )?;
    for unit in [
        "agent-browser-dashboard-backend.service",
        "agent-browser-dashboard.service",
        "agent-browser-runtime-interlock.timer",
        "agent-browser-guacamole-postgres-backup.timer",
    ] {
        run_required(
            "systemctl",
            &["--user", "is-active", "--quiet", unit],
            &paths.support_dir,
            command_env,
            false,
            "verify workstation user service",
        )?;
    }
    Ok(())
}

/// The stable ingress is intentionally excluded from the ordinary reconcile
/// quiesce set. During an upgrade its unit link changes generations, so reload
/// and restart it only after the candidate selector and authenticated shadow
/// route are committed.
fn restart_stable_dashboard_ingress(
    paths: &InstallPaths,
    support_root: &Path,
    command_env: &[(String, String)],
) -> Result<(), String> {
    run_required(
        "systemctl",
        &["--user", "daemon-reload"],
        support_root,
        command_env,
        false,
        "reload stable dashboard ingress generation",
    )?;
    run_required(
        "systemctl",
        &[
            "--user",
            "restart",
            "agent-browser-dashboard-backend.service",
        ],
        support_root,
        command_env,
        false,
        "restart dashboard backend on selected generation",
    )?;
    run_required(
        "systemctl",
        &["--user", "restart", "agent-browser-dashboard.service"],
        support_root,
        command_env,
        false,
        "restart stable dashboard ingress on selected generation",
    )?;
    run_required(
        "systemctl",
        &[
            "--user",
            "is-active",
            "--quiet",
            "agent-browser-dashboard-backend.service",
        ],
        &paths.support_dir,
        command_env,
        false,
        "verify refreshed dashboard backend",
    )?;
    run_required(
        "systemctl",
        &[
            "--user",
            "is-active",
            "--quiet",
            "agent-browser-dashboard.service",
        ],
        &paths.support_dir,
        command_env,
        false,
        "verify refreshed stable dashboard ingress",
    )
}

fn reset_failed_user_unit_if_failed(
    unit: &str,
    support_root: &Path,
    command_env: &[(String, String)],
) -> Result<(), String> {
    let failed_state = run_observed(
        "systemctl",
        &["--user", "is-failed", unit],
        support_root,
        command_env,
    )
    .map_err(|error| format!("inspect a prior bounded interlock failure: {error}"))?;
    if !systemd_unit_is_failed(&failed_state.stdout)? {
        return Ok(());
    }
    let reset_result = run_required(
        "systemctl",
        &["--user", "reset-failed", unit],
        support_root,
        command_env,
        false,
        "clear a prior bounded interlock failure",
    );
    if reset_result.is_ok() {
        return Ok(());
    }

    let current_state = run_observed(
        "systemctl",
        &["--user", "is-failed", unit],
        support_root,
        command_env,
    )
    .map_err(|error| format!("verify a prior bounded interlock failure cleared: {error}"))?;
    if systemd_unit_is_failed(&current_state.stdout)? {
        reset_result
    } else {
        Ok(())
    }
}

fn systemd_unit_is_failed(stdout: &[u8]) -> Result<bool, String> {
    match String::from_utf8_lossy(stdout).trim() {
        "failed" => Ok(true),
        "active" | "activating" | "deactivating" | "inactive" | "maintenance" | "reloading"
        | "unknown" => Ok(false),
        state => Err(format!(
            "inspect a prior bounded interlock failure: unexpected systemd state '{state}'"
        )),
    }
}

/// Snapshots and stops installed user units that could race with workstation
/// reconciliation. Success activates the canonical unit set; failure restores
/// every pre-existing unit to its exact prior active state.
#[derive(Debug, Clone, PartialEq, Eq)]
struct QuiescedUserUnits {
    prior_states: Vec<(&'static str, bool)>,
}

impl QuiescedUserUnits {
    #[cfg(test)]
    fn from_states(states: impl IntoIterator<Item = (&'static str, bool)>) -> Self {
        Self {
            prior_states: states.into_iter().collect(),
        }
    }

    fn units_to_start(&self) -> Vec<&'static str> {
        self.prior_states
            .iter()
            .filter_map(|(unit, active)| active.then_some(*unit))
            .collect()
    }

    fn units_to_stop(&self) -> Vec<&'static str> {
        self.prior_states
            .iter()
            .filter_map(|(unit, active)| (!active).then_some(*unit))
            .collect()
    }
}

fn quiesce_existing_user_units(paths: &InstallPaths) -> Result<QuiescedUserUnits, String> {
    let existing_units = WORKSTATION_RECONCILE_QUIESCE_UNITS
        .into_iter()
        .filter(|unit| paths.unit_dir.join(unit).is_file())
        .collect::<Vec<_>>();
    let mut prior_states = Vec::new();
    for unit in &existing_units {
        let status = run_observed(
            "systemctl",
            &["--user", "is-active", "--quiet", unit],
            &paths.root,
            &[],
        )
        .map_err(|error| format!("inspect workstation user unit {unit}: {error}"))?;
        prior_states.push((*unit, status.status.success()));
    }
    let quiesced = QuiescedUserUnits { prior_states };

    if !WORKSTATION_RECONCILE_QUIESCE_UNITS
        .iter()
        .any(|unit| paths.unit_dir.join(unit).is_file())
    {
        return Ok(quiesced);
    }

    run_required(
        "systemctl",
        &["--user", "daemon-reload"],
        &paths.root,
        &[],
        false,
        "reload existing workstation user units before reconciliation",
    )?;
    let mut args = vec!["--user", "stop"];
    args.extend(WORKSTATION_RECONCILE_QUIESCE_UNITS);
    let stopped = run_required(
        "systemctl",
        &args,
        &paths.root,
        &[],
        false,
        "quiesce existing workstation user units before reconciliation",
    );
    if let Err(error) = stopped {
        return match restore_previously_active_user_units(paths, &paths.root, &[], &quiesced) {
            Ok(()) => Err(format!(
                "{error}; previously active workstation user units were restored"
            )),
            Err(restore_error) => Err(format!(
                "{error}; failed to restore previously active workstation user units: {restore_error}"
            )),
        };
    }
    Ok(quiesced)
}

fn restore_previously_active_user_units(
    paths: &InstallPaths,
    support_root: &Path,
    command_env: &[(String, String)],
    quiesced: &QuiescedUserUnits,
) -> Result<(), String> {
    if quiesced.prior_states.is_empty() {
        return Ok(());
    }
    let mut errors = Vec::new();
    if let Err(error) = run_required(
        "systemctl",
        &["--user", "daemon-reload"],
        support_root,
        command_env,
        false,
        "reload workstation user units before failure restoration",
    ) {
        errors.push(error);
    }
    let units_to_stop = quiesced.units_to_stop();
    if !units_to_stop.is_empty() {
        let mut args = vec!["--user", "stop"];
        args.extend(units_to_stop.iter().copied());
        if let Err(error) = run_required(
            "systemctl",
            &args,
            support_root,
            command_env,
            false,
            "restore previously inactive workstation user units after reconciliation failure",
        ) {
            errors.push(error);
        }
    }
    let units_to_start = quiesced.units_to_start();
    if !units_to_start.is_empty() {
        let mut args = vec!["--user", "start"];
        args.extend(units_to_start.iter().copied());
        if let Err(error) = run_required(
            "systemctl",
            &args,
            support_root,
            command_env,
            false,
            "restore previously active workstation user units after reconciliation failure",
        ) {
            errors.push(error);
        }
    }
    for (unit, expected_active) in &quiesced.prior_states {
        match run_observed(
            "systemctl",
            &["--user", "is-active", "--quiet", unit],
            &paths.root,
            command_env,
        ) {
            Ok(status) if status.status.success() == *expected_active => {}
            Ok(_) => errors.push(format!(
                "restored workstation user unit {unit} did not return to its prior state"
            )),
            Err(error) => errors.push(format!(
                "verify restored workstation user unit {unit}: {error}"
            )),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn complete_reconcile_with_unit_restore<T>(
    reconcile_result: Result<T, String>,
    restore: impl FnOnce() -> Result<(), String>,
) -> Result<T, String> {
    match reconcile_result {
        Ok(value) => Ok(value),
        Err(error) => match restore() {
            Ok(()) => Err(format!(
                "{error}; previously active workstation user units were restored"
            )),
            Err(restore_error) => Err(format!(
                "{error}; failed to restore previously active workstation user units: {restore_error}"
            )),
        },
    }
}

fn fail_with_user_unit_restoration(
    error: &str,
    json: bool,
    paths: &InstallPaths,
    quiesced: Option<&QuiescedUserUnits>,
) -> ! {
    let restored_error = match quiesced {
        Some(quiesced) => {
            complete_reconcile_with_unit_restore::<()>(Err(error.to_string()), || {
                restore_previously_active_user_units(paths, &paths.root, &[], quiesced)
            })
            .unwrap_err()
        }
        None => error.to_string(),
    };
    fail(&restored_error, json)
}

fn verify_final_doctors(
    paths: &InstallPaths,
    support_root: &Path,
    command_env: &[(String, String)],
    expected_upgrade: Option<&crate::runtime_adoption::UpgradeTransaction>,
    transitional_source_sessions: &[String],
) -> Result<(), String> {
    for (label, args, readiness_pointer) in [
        (
            "install doctor",
            vec!["install", "doctor", "--json"],
            "/success",
        ),
        (
            "remote-view doctor",
            vec!["doctor", "remote-view", "--json"],
            "/data/remoteControl/ready",
        ),
    ] {
        let attempts = if expected_upgrade.is_some() {
            POST_COMMIT_DOCTOR_ATTEMPTS
        } else {
            1
        };
        let mut last_failure = None;
        for attempt in 0..attempts {
            let output = run_observed(
                paths
                    .binary
                    .to_str()
                    .ok_or_else(|| "invalid installed agent-browser path".to_string())?,
                &args,
                support_root,
                command_env,
            )?;
            let payload: Value = serde_json::from_slice(&output.stdout)
                .map_err(|error| format!("{label} JSON parse failed: {error}"))?;
            let ready = if label == "install doctor" {
                expected_upgrade.map_or_else(
                    || install_doctor_reports_workstation_ready(&payload),
                    |transaction| {
                        install_doctor_reports_expected_upgrade_ready(
                            &payload,
                            transaction,
                            transitional_source_sessions,
                        )
                    },
                )
            } else if let Some(transaction) = expected_upgrade {
                output.status.success()
                    && remote_view_doctor_reports_expected_upgrade_ready(
                        &payload,
                        transaction,
                        transitional_source_sessions,
                    )
            } else {
                final_doctor_reports_ready(
                    label,
                    &payload,
                    output.status.success(),
                    readiness_pointer,
                )
            };
            if ready {
                last_failure = None;
                break;
            }
            last_failure = Some((output.status.to_string(), doctor_issue_codes(&payload)));
            if attempt + 1 < attempts {
                std::thread::sleep(POST_COMMIT_DOCTOR_RETRY_INTERVAL);
            }
        }
        if let Some((status, issue_codes)) = last_failure {
            return Err(format!(
                "{label} did not report ready after {attempts} attempt(s) (status {status}; issueCodes={issue_codes})"
            ));
        }
    }
    Ok(())
}

fn doctor_issue_codes(payload: &Value) -> String {
    let issues = payload
        .pointer("/data/issues")
        .or_else(|| payload.pointer("/data/install/data/issues"))
        .and_then(Value::as_array);
    let mut codes = issues
        .into_iter()
        .flatten()
        .filter_map(|issue| issue.get("code").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<Vec<_>>();
    codes.sort();
    codes.dedup();
    if codes.is_empty() {
        "none".to_string()
    } else {
        codes.join(",")
    }
}

fn final_doctor_reports_ready(
    label: &str,
    payload: &Value,
    command_succeeded: bool,
    readiness_pointer: &str,
) -> bool {
    if label == "install doctor" {
        install_doctor_reports_workstation_ready(payload)
    } else {
        command_succeeded
            && payload.pointer(readiness_pointer).and_then(Value::as_bool) == Some(true)
    }
}

/// Accepts a globally degraded install doctor only when every reported issue
/// is known not to affect workstation route readiness. Duplicate profile
/// pressure remains visible to operators, while an inactive optional session
/// supervisor may retain stale executable provenance without taking the
/// dashboard, RDP route, or Guacamole interlock down. Active supervisor drift
/// and every other doctor issue remain blocking.
fn install_doctor_reports_workstation_ready(payload: &Value) -> bool {
    if payload.get("success").and_then(Value::as_bool) == Some(true) {
        return true;
    }

    let Some(data) = payload.get("data") else {
        return false;
    };
    let Some(issues) = data.get("issues").and_then(Value::as_array) else {
        return false;
    };
    if issues.is_empty() {
        return false;
    }

    install_doctor_issues_are_advisory(data, issues)
}

fn install_doctor_issues_are_advisory(data: &Value, issues: &[Value]) -> bool {
    let session_supervisors = data.get("sessionSupervisors").unwrap_or(&Value::Null);
    let inactive_supervisors = session_supervisors
        .get("sessions")
        .and_then(Value::as_array)
        .is_some_and(|sessions| {
            sessions.iter().all(|session| {
                session.get("activeState").and_then(Value::as_str) == Some("inactive")
            })
        });
    let supervisor_issues = session_supervisors
        .get("issues")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    issues.iter().all(|issue| {
        issue.get("code").and_then(Value::as_str) == Some("service_duplicate_profile_pressure")
            || issue.get("code").and_then(Value::as_str)
                == Some("active_runtime_manual_preservation")
            || (inactive_supervisors && supervisor_issues.contains(issue))
    })
}

/// During the transaction's own post-commit validation, install doctor must
/// remain globally non-ready because the transaction and admission drain are
/// still active. The installer may consume the component evidence only when
/// the only additional blockers are the exact expected transaction at the
/// exact `post_commit_validating` revision, the runtime monitor startup gap
/// created by this installer quiescing and reactivating its units, and
/// reviewed external process pressure when the service proves that it has no
/// GC candidates. This does not weaken ordinary doctor.
fn install_doctor_reports_expected_upgrade_ready(
    payload: &Value,
    expected: &crate::runtime_adoption::UpgradeTransaction,
    transitional_source_sessions: &[String],
) -> bool {
    use crate::runtime_adoption::UpgradeTransactionState;

    if expected.state != UpgradeTransactionState::PostCommitValidating {
        return false;
    }
    let Some(data) = payload.get("data") else {
        return false;
    };
    let Some(upgrade) = data.pointer("/liveDashboardRuntime/workstationUpgrade") else {
        return false;
    };
    if upgrade
        .pointer("/latestTransaction/transactionId")
        .and_then(Value::as_str)
        != Some(expected.transaction_id.as_str())
        || upgrade
            .pointer("/latestTransaction/state")
            .and_then(Value::as_str)
            != Some("post_commit_validating")
        || upgrade.get("admissionDraining").and_then(Value::as_bool) != Some(true)
        || upgrade.get("selectedGenerationId").and_then(Value::as_str)
            != Some(expected.candidate_generation_id.as_str())
    {
        return false;
    }

    let Some(issues) = data.get("issues").and_then(Value::as_array) else {
        return false;
    };
    let transaction_issue_count = issues
        .iter()
        .filter(|issue| {
            issue.get("code").and_then(Value::as_str)
                == Some("workstation_upgrade_transaction_not_terminal")
        })
        .count();
    let shadow_dashboard_transition_ready =
        expected_upgrade_shadow_dashboard_transition_ready(upgrade, expected);
    let supervisor_transition_ready = expected_upgrade_supervisor_transition_ready(data, expected);
    let remaining_issues = issues
        .iter()
        .filter(|issue| {
            let code = issue.get("code").and_then(Value::as_str);
            if code == Some("workstation_upgrade_transaction_not_terminal") {
                return false;
            }
            if code == Some("runtime_monitor_not_ready") {
                return false;
            }
            if code == Some("runtime_pressure_ownership_unknown")
                && expected_upgrade_runtime_pressure_is_review_only(data)
            {
                return false;
            }
            if code == Some("dashboard_runtime_stale_or_unreadable")
                && shadow_dashboard_transition_ready
            {
                return false;
            }
            if code == Some("executable_drift") && supervisor_transition_ready {
                return false;
            }
            !(code == Some("active_runtime_stale_executable")
                && issue
                    .get("session")
                    .and_then(Value::as_str)
                    .is_some_and(|session| {
                        transitional_source_sessions
                            .iter()
                            .any(|expected| expected == session)
                    }))
        })
        .cloned()
        .collect::<Vec<_>>();
    transaction_issue_count == 1 && install_doctor_issues_are_advisory(data, &remaining_issues)
}

fn expected_upgrade_supervisor_transition_ready(
    data: &Value,
    expected: &crate::runtime_adoption::UpgradeTransaction,
) -> bool {
    let Some(old_generation) = expected.old_generation_id.as_deref() else {
        return false;
    };
    let expected_suffix = format!("/generations/{old_generation}/bin/agent-browser");
    let supervisors = data
        .pointer("/sessionSupervisors/sessions")
        .and_then(Value::as_array);
    let supervisor_issues = data
        .pointer("/sessionSupervisors/issues")
        .and_then(Value::as_array);
    let runtime_hosts = data
        .pointer("/runtimeMultiplicity/runtimeHosts")
        .and_then(Value::as_array);
    supervisors.is_some_and(|sessions| {
        !sessions.is_empty()
            && sessions.iter().all(|session| {
                session
                    .pointer("/manifest/executablePath")
                    .and_then(Value::as_str)
                    .is_some_and(|path| path.ends_with(&expected_suffix))
            })
    }) && supervisor_issues.is_some_and(|issues| {
        !issues.is_empty()
            && issues
                .iter()
                .all(|issue| issue.get("code").and_then(Value::as_str) == Some("executable_drift"))
    }) && runtime_hosts.is_some_and(|hosts| {
        hosts.len() == 1
            && hosts[0].get("generationId").and_then(Value::as_str)
                == Some(expected.candidate_generation_id.as_str())
    }) && data
        .pointer("/runtimeMultiplicity/legacyDaemons")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
}

fn expected_upgrade_runtime_pressure_is_review_only(data: &Value) -> bool {
    let resources = data.get("serviceResources").unwrap_or(&Value::Null);
    resources.get("available").and_then(Value::as_bool) == Some(true)
        && resources.get("candidateCount").and_then(Value::as_u64) == Some(0)
        && resources
            .get("readinessImpactingCandidates")
            .and_then(Value::as_u64)
            == Some(0)
}

fn expected_upgrade_shadow_dashboard_transition_ready(
    upgrade: &Value,
    expected: &crate::runtime_adoption::UpgradeTransaction,
) -> bool {
    let Some(ingress) = upgrade.get("dashboardIngress") else {
        return false;
    };
    ingress
        .get("dashboardIngressReady")
        .and_then(Value::as_bool)
        == Some(true)
        && ingress.get("operatorJourneyReady").and_then(Value::as_bool) == Some(true)
        && ingress
            .pointer("/selectedBackend/generationId")
            .and_then(Value::as_str)
            == Some(expected.candidate_generation_id.as_str())
        && ingress
            .pointer("/presentationReceipt/dashboardDeploymentGeneration")
            .and_then(Value::as_str)
            == Some(expected.candidate_generation_id.as_str())
        && ingress
            .pointer("/presentationReceipt/state")
            .and_then(Value::as_str)
            == Some("ready")
        && ingress
            .pointer("/presentationReceipt/receiptId")
            .and_then(Value::as_str)
            .is_some_and(|receipt| !receipt.trim().is_empty())
}

/// Remote-view doctor derives `remoteControl.ready` from install-doctor
/// readiness plus its route, gateway, display, and launch axes. During the
/// reversible handoff window, accept only the exact embedded transaction-aware
/// install result while every independent remote-control axis is already true.
fn remote_view_doctor_reports_expected_upgrade_ready(
    payload: &Value,
    expected: &crate::runtime_adoption::UpgradeTransaction,
    transitional_source_sessions: &[String],
) -> bool {
    let Some(install) = payload.pointer("/data/install/data") else {
        return false;
    };
    if !install_doctor_reports_expected_upgrade_ready(
        install,
        expected,
        transitional_source_sessions,
    ) {
        return false;
    }
    let Some(remote_control) = payload.pointer("/data/remoteControl") else {
        return false;
    };
    [
        "rdpGatewayReady",
        "privateDisplayAllocatorReady",
        "routePoolReady",
        "routeUrlReady",
        "routeDisplayReady",
        "routeDisplayAccessReady",
        "browserLaunchReady",
    ]
    .iter()
    .all(|axis| remote_control.get(*axis).and_then(Value::as_bool) == Some(true))
}

fn write_private_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(display_io("create receipt directory", parent))?;
        set_private_directory(parent)?;
    }
    let body = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Unable to serialize workstation receipt: {error}"))?;
    fs::write(path, body).map_err(display_io("write workstation receipt", path))?;
    set_private_file(path)
}

const MAX_RUNTIME_CENSUS_ROUNDS: usize = 8;

fn collect_stable_runtime_census_with(
    mut collect_round: impl FnMut() -> Result<crate::runtime_adoption::RuntimeCensusRound, String>,
) -> Result<crate::runtime_adoption::StableRuntimeCensus, String> {
    use crate::runtime_adoption::build_stable_runtime_census;

    let mut previous = collect_round()?;
    let mut latest = None;
    for _ in 1..MAX_RUNTIME_CENSUS_ROUNDS {
        let current = collect_round()?;
        let census = build_stable_runtime_census(&previous, &current)?;
        let changed_during_classification = census.records.iter().any(|record| {
            record
                .reason_codes
                .iter()
                .any(|reason| reason == "census_changed_during_classification")
        });
        if !changed_during_classification {
            return Ok(census);
        }
        previous = current;
        latest = Some(census);
    }
    latest.ok_or_else(|| "runtime census did not collect two rounds".to_string())
}

#[cfg(test)]
fn require_stable_runtime_census_with(
    root: &Path,
    paths: &InstallPaths,
    args: &WorkstationInstallArgs,
    mut collect_round: impl FnMut() -> Result<crate::runtime_adoption::RuntimeCensusRound, String>,
) -> Result<PathBuf, String> {
    use crate::runtime_adoption::{
        persist_runtime_census, UpgradeCheckpoint, UpgradeTransaction, UpgradeTransactionState,
        RUNTIME_ADOPTION_SCHEMA_VERSION,
    };

    let current_exe = env::current_exe()
        .map_err(|error| format!("Unable to resolve candidate executable: {error}"))?;
    let binary_sha256 = workstation_file_sha256(&current_exe)?;
    let rendered_units = render_units(
        &paths.binary.display().to_string(),
        &paths.current_selector.join("support"),
        &paths.guacamole_secret_file,
        args.dashboard_port,
    );
    let support_manifest = render_manifest(args, &binary_sha256, &rendered_units);
    let support_manifest_sha256 = workstation_bytes_sha256(support_manifest.as_bytes());
    let candidate_generation_id = format!(
        "{}-{}-{}",
        env!("CARGO_PKG_VERSION"),
        &binary_sha256[..12],
        &support_manifest_sha256[..12]
    );
    let transaction_id = format!("upgrade-{}", uuid::Uuid::new_v4());
    let transaction_path = root
        .join(".agent-browser/runtime-adoption/transactions")
        .join(format!("{transaction_id}.json"));
    let recorded_at = runtime_adoption_timestamp();
    let mut transaction = UpgradeTransaction {
        schema_version: RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
        transaction_id,
        requested_by: "workstation-installer".to_string(),
        old_generation_id: selected_generation_id(paths),
        candidate_generation_id,
        candidate_binary_sha256: binary_sha256,
        candidate_support_manifest_sha256: support_manifest_sha256,
        runtime_census_digest: None,
        runtime_migrations: Vec::new(),
        runtime_host_convergence: None,
        state: UpgradeTransactionState::Planned,
        revision: 0,
        checkpoints: vec![UpgradeCheckpoint {
            name: "census_planned".to_string(),
            transaction_revision: 0,
            recorded_at: recorded_at.clone(),
        }],
        dashboard_validation_summary: None,
        presentation_validation_summary: None,
        terminal_result: None,
        stop_reason: None,
    };

    let census_result = collect_stable_runtime_census_with(&mut collect_round);
    let census = match census_result {
        Ok(census) => census,
        Err(error) => {
            block_incomplete_runtime_census(&mut transaction, &recorded_at);
            write_private_json_atomic(&transaction_path, &transaction)?;
            return Err(format!(
                "runtime census is incomplete; payload was not changed; transaction {}: {error}",
                transaction_path.display()
            ));
        }
    };
    persist_runtime_census(&mut transaction, &census, &recorded_at);
    write_private_json_atomic(&transaction_path, &transaction)?;
    if !census.activation_allowed {
        return Err(format!(
            "runtime census is ambiguous; payload was not changed; inspect transaction {}",
            transaction_path.display()
        ));
    }
    Ok(transaction_path)
}

fn block_incomplete_runtime_census(
    transaction: &mut crate::runtime_adoption::UpgradeTransaction,
    recorded_at: &str,
) {
    transaction.revision = transaction.revision.saturating_add(1);
    transaction.state = crate::runtime_adoption::UpgradeTransactionState::BlockedAmbiguousRuntime;
    transaction.stop_reason = Some("runtime_census_incomplete".to_string());
    transaction
        .checkpoints
        .push(crate::runtime_adoption::UpgradeCheckpoint {
            name: "census_blocked_incomplete".to_string(),
            transaction_revision: transaction.revision,
            recorded_at: recorded_at.to_string(),
        });
}

fn selected_generation_id(paths: &InstallPaths) -> Option<String> {
    fs::read_link(&paths.current_selector)
        .ok()
        .and_then(|target| {
            target
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
}

fn runtime_adoption_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "timestamp-unavailable".to_string())
}

fn write_private_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(display_io("create receipt directory", parent))?;
        set_private_directory(parent)?;
    }
    let staged = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let body = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Unable to serialize runtime adoption transaction: {error}"))?;
    fs::write(&staged, body).map_err(display_io("stage runtime adoption transaction", &staged))?;
    set_private_file(&staged)?;
    fs::rename(&staged, path).map_err(display_io("commit runtime adoption transaction", path))
}

fn candidate_generation_identity(
    paths: &InstallPaths,
    args: &WorkstationInstallArgs,
) -> Result<(String, String, String), String> {
    let current_exe = env::current_exe()
        .map_err(|error| format!("Unable to resolve candidate executable: {error}"))?;
    let binary_sha256 = workstation_file_sha256(&current_exe)?;
    let rendered_units = render_units(
        &paths.binary.display().to_string(),
        &paths.current_selector.join("support"),
        &paths.guacamole_secret_file,
        args.dashboard_port,
    );
    let support_manifest = render_manifest(args, &binary_sha256, &rendered_units);
    let support_manifest_sha256 = workstation_bytes_sha256(support_manifest.as_bytes());
    let generation_id = format!(
        "{}-{}-{}",
        env!("CARGO_PKG_VERSION"),
        &binary_sha256[..12],
        &support_manifest_sha256[..12]
    );
    Ok((generation_id, binary_sha256, support_manifest_sha256))
}

fn new_upgrade_transaction(
    paths: &InstallPaths,
    candidate_generation_id: String,
    candidate_binary_sha256: String,
    candidate_support_manifest_sha256: String,
) -> crate::runtime_adoption::UpgradeTransaction {
    use crate::runtime_adoption::{
        RuntimeHostConvergenceRecord, UpgradeCheckpoint, UpgradeTransaction,
        UpgradeTransactionState, RUNTIME_ADOPTION_SCHEMA_VERSION,
    };

    let recorded_at = runtime_adoption_timestamp();
    let deadline = time::OffsetDateTime::now_utc() + time::Duration::minutes(10);
    let deadline_at = deadline
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "timestamp-unavailable".to_string());
    UpgradeTransaction {
        schema_version: RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
        transaction_id: format!("upgrade-{}", uuid::Uuid::new_v4()),
        requested_by: "workstation-installer".to_string(),
        old_generation_id: selected_generation_id(paths),
        candidate_generation_id,
        candidate_binary_sha256,
        candidate_support_manifest_sha256,
        runtime_census_digest: None,
        runtime_migrations: Vec::new(),
        runtime_host_convergence: Some(RuntimeHostConvergenceRecord {
            schema_version: "agent-browser.runtime-host-convergence.v1".to_string(),
            deadline_at,
            deadline_unix_seconds: deadline.unix_timestamp(),
            queue_transfer_policy: "drain_then_commit".to_string(),
            old_host: None,
            candidate_host: None,
            lanes: Vec::new(),
        }),
        state: UpgradeTransactionState::Planned,
        revision: 0,
        checkpoints: vec![UpgradeCheckpoint {
            name: "transaction_planned".to_string(),
            transaction_revision: 0,
            recorded_at,
        }],
        dashboard_validation_summary: None,
        presentation_validation_summary: None,
        terminal_result: None,
        stop_reason: None,
    }
}

fn record_blocked_upgrade_transaction(
    root: &Path,
    paths: &InstallPaths,
    args: &WorkstationInstallArgs,
    state: crate::runtime_adoption::UpgradeTransactionState,
    stop_reason: &str,
) -> Result<PathBuf, String> {
    let (generation_id, binary_sha256, support_manifest_sha256) =
        candidate_generation_identity(paths, args)?;
    let mut transaction =
        new_upgrade_transaction(paths, generation_id, binary_sha256, support_manifest_sha256);
    transaction.stop_reason = Some(stop_reason.to_string());
    let path = transaction_path(root, &transaction.transaction_id);
    write_private_json_atomic(&path, &transaction)?;
    persist_upgrade_transition(&path, &mut transaction, state, stop_reason)?;
    Ok(path)
}

fn transaction_path(root: &Path, transaction_id: &str) -> PathBuf {
    root.join(".agent-browser/runtime-adoption/transactions")
        .join(format!("{transaction_id}.json"))
}

fn persist_upgrade_transition(
    path: &Path,
    transaction: &mut crate::runtime_adoption::UpgradeTransaction,
    next_state: crate::runtime_adoption::UpgradeTransactionState,
    checkpoint_name: &str,
) -> Result<(), String> {
    let revision = transaction.revision;
    crate::runtime_adoption::transition_upgrade_transaction(
        transaction,
        revision,
        next_state,
        checkpoint_name,
        &runtime_adoption_timestamp(),
    )?;
    write_private_json_atomic(path, transaction)
}

fn persist_admission_drain(
    path: &Path,
    transaction: &crate::runtime_adoption::UpgradeTransaction,
) -> Result<(), String> {
    write_private_json_atomic(
        path,
        &crate::runtime_adoption::RuntimeAdmissionDrain {
            schema_version: crate::runtime_adoption::RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
            transaction_id: transaction.transaction_id.clone(),
            candidate_generation_id: transaction.candidate_generation_id.clone(),
            transaction_revision: transaction.revision,
            recorded_at: runtime_adoption_timestamp(),
        },
    )
}

fn clear_admission_drain(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "Unable to close runtime admission drain {}: {error}",
            path.display()
        )),
    }
}

fn isolated_runtime_census() -> Result<crate::runtime_adoption::StableRuntimeCensus, String> {
    let round = crate::runtime_adoption::collect_runtime_census_round(
        0,
        crate::runtime_adoption::runtime_census_sources()
            .into_iter()
            .map(
                |source| crate::runtime_adoption::RuntimeCensusSourceSnapshot {
                    source,
                    source_revision: "isolated-fixture".to_string(),
                    logical_browser_ids: Vec::new(),
                },
            )
            .collect(),
        Vec::new(),
    )?;
    crate::runtime_adoption::build_stable_runtime_census(&round, &round)
}

fn prepare_payload_transaction(
    root: &Path,
    paths: &InstallPaths,
    args: &WorkstationInstallArgs,
    isolated_root: bool,
) -> Result<PreparedPayloadTransaction, String> {
    use crate::runtime_adoption::{persist_runtime_census, UpgradeTransactionState};

    let (generation_id, binary_sha256, support_manifest_sha256) =
        candidate_generation_identity(paths, args)?;
    let mut transaction =
        new_upgrade_transaction(paths, generation_id, binary_sha256, support_manifest_sha256);
    let transaction_path = transaction_path(root, &transaction.transaction_id);
    write_private_json_atomic(&transaction_path, &transaction)?;
    let admission_drain_path = root
        .join(".agent-browser/runtime-adoption")
        .join("admission-drain.json");
    if admission_drain_path.exists() {
        transaction.stop_reason = Some("existing_admission_drain".to_string());
        persist_upgrade_transition(
            &transaction_path,
            &mut transaction,
            UpgradeTransactionState::BlockedInflightEffect,
            "blocked_existing_admission_drain",
        )?;
        return Err(format!(
            "an earlier workstation transaction still owns runtime admission; selected generation was not changed; inspect transaction status before retrying: {}",
            transaction_path.display()
        ));
    }

    if !isolated_root {
        if let Err(error) = reconcile_selected_legacy_daemon_identities(paths) {
            block_incomplete_runtime_census(&mut transaction, &runtime_adoption_timestamp());
            transaction.stop_reason =
                Some("legacy_daemon_identity_reconciliation_failed".to_string());
            write_private_json_atomic(&transaction_path, &transaction)?;
            return Err(format!(
                "legacy daemon identity reconciliation failed before runtime census; payload and selected generation were not changed; transaction {}: {error}",
                transaction_path.display()
            ));
        }
    }

    let census = if isolated_root {
        isolated_runtime_census()
    } else {
        collect_stable_runtime_census_with(
            crate::runtime_adoption::collect_host_runtime_census_round,
        )
    };
    let census = match census {
        Ok(census) => census,
        Err(error) => {
            block_incomplete_runtime_census(&mut transaction, &runtime_adoption_timestamp());
            write_private_json_atomic(&transaction_path, &transaction)?;
            return Err(format!(
                "runtime census is incomplete; payload and selected generation were not changed; transaction {}: {error}",
                transaction_path.display()
            ));
        }
    };
    if !census.activation_allowed {
        persist_runtime_census(&mut transaction, &census, &runtime_adoption_timestamp());
        write_private_json_atomic(&transaction_path, &transaction)?;
        return Err(format!(
            "runtime census is ambiguous; payload and selected generation were not changed; inspect transaction {}",
            transaction_path.display()
        ));
    }

    if transaction.old_generation_id.is_none() && legacy_mutable_payload_present(paths) {
        let legacy_generation_id = match migrate_legacy_payload_to_generation(paths) {
            Ok(generation_id) => generation_id,
            Err(error) => {
                transaction.stop_reason = Some("legacy_generation_migration_failed".to_string());
                let _ = persist_upgrade_transition(
                    &transaction_path,
                    &mut transaction,
                    UpgradeTransactionState::RollbackBeforeCommit,
                    "rollback_before_commit",
                );
                transaction.terminal_result = Some("legacy_payload_preserved".to_string());
                let _ = persist_upgrade_transition(
                    &transaction_path,
                    &mut transaction,
                    UpgradeTransactionState::FailedPreservedOldGeneration,
                    "failed_preserved_old_generation",
                );
                return Err(format!(
                    "{error}; transaction: {}",
                    transaction_path.display()
                ));
            }
        };
        transaction.old_generation_id = Some(legacy_generation_id);
        write_private_json_atomic(&transaction_path, &transaction)?;
    }

    let staged = match stage_payload_generation(paths, args) {
        Ok(staged) => staged,
        Err(error) => {
            transaction.stop_reason = Some("candidate_staging_failed".to_string());
            let _ = persist_upgrade_transition(
                &transaction_path,
                &mut transaction,
                UpgradeTransactionState::RollbackBeforeCommit,
                "rollback_before_commit",
            );
            transaction.terminal_result = Some("old_generation_preserved".to_string());
            let _ = persist_upgrade_transition(
                &transaction_path,
                &mut transaction,
                UpgradeTransactionState::FailedPreservedOldGeneration,
                "failed_preserved_old_generation",
            );
            return Err(format!(
                "{error}; transaction: {}",
                transaction_path.display()
            ));
        }
    };
    if staged.generation_id != transaction.candidate_generation_id
        || staged.binary_sha256 != transaction.candidate_binary_sha256
        || staged.support_manifest_sha256 != transaction.candidate_support_manifest_sha256
    {
        return Err("candidate_generation_identity_changed_during_staging".to_string());
    }
    persist_upgrade_transition(
        &transaction_path,
        &mut transaction,
        UpgradeTransactionState::CandidateStaged,
        "candidate_staged",
    )?;
    persist_upgrade_transition(
        &transaction_path,
        &mut transaction,
        UpgradeTransactionState::CandidatePreflightReady,
        "candidate_preflight_ready",
    )?;
    persist_runtime_census(&mut transaction, &census, &runtime_adoption_timestamp());
    write_private_json_atomic(&transaction_path, &transaction)?;

    Ok(PreparedPayloadTransaction {
        staged,
        transaction_path,
        transaction,
        previous_selector: fs::read_link(&paths.current_selector).ok(),
        admission_drain_path,
        runtime_handoffs: Vec::new(),
        dashboard_candidate: None,
    })
}

fn activate_prepared_payload_transaction(
    prepared: &mut PreparedPayloadTransaction,
    paths: &InstallPaths,
    isolated_root: bool,
) -> Result<(), String> {
    use crate::runtime_adoption::UpgradeTransactionState;

    crate::runtime_adoption::require_runtime_host_convergence_deadline(&prepared.transaction)?;
    persist_upgrade_transition(
        &prepared.transaction_path,
        &mut prepared.transaction,
        UpgradeTransactionState::AdmissionDraining,
        "admission_draining",
    )?;
    persist_admission_drain(&prepared.admission_drain_path, &prepared.transaction)?;
    persist_upgrade_transition(
        &prepared.transaction_path,
        &mut prepared.transaction,
        UpgradeTransactionState::RuntimesTransferring,
        "runtimes_transferring",
    )?;
    persist_admission_drain(&prepared.admission_drain_path, &prepared.transaction)?;
    if candidate_runtime_host_stage_required(isolated_root, 0) {
        capture_selected_runtime_host_before_transfer(&mut prepared.transaction)?;
        let transfer_evidence = transfer_discovered_runtimes(
            paths,
            &prepared.staged,
            &prepared.transaction.transaction_id,
            &mut prepared.transaction.runtime_migrations,
            &mut prepared.runtime_handoffs,
        )?;
        let candidate_socket_dir =
            candidate_runtime_host_socket_dir(&prepared.transaction.transaction_id)?;
        let (host_identity, candidate_backend) = capture_runtime_host_identity(
            &candidate_socket_dir,
            &prepared.transaction.candidate_generation_id,
            &prepared.transaction.candidate_binary_sha256,
            true,
        )?;
        crate::runtime_adoption::record_runtime_host_identity(
            &mut prepared.transaction,
            true,
            host_identity,
        )?;
        for evidence in transfer_evidence {
            let session_names = prepared
                .transaction
                .runtime_migrations
                .iter()
                .find(|migration| migration.logical_browser_id == evidence.logical_browser_id)
                .map(|migration| migration.session_names.clone())
                .ok_or_else(|| {
                    format!(
                        "runtime_transfer_migration_disappeared:{}",
                        evidence.logical_browser_id
                    )
                })?;
            for session_name in session_names {
                crate::runtime_adoption::record_runtime_lane_observation(
                    &mut prepared.transaction,
                    &session_name,
                    &evidence.candidate_session,
                    evidence.receipt.previous_owner_generation,
                    &evidence.receipt.receipt_id,
                )?;
                crate::runtime_adoption::commit_runtime_lane_transfer(
                    &mut prepared.transaction,
                    &session_name,
                    evidence.receipt.candidate_owner_generation,
                    0,
                    &evidence.receipt.receipt_id,
                )?;
            }
        }
        stage_candidate_runtime_host_ingress(paths, &mut prepared.transaction, candidate_backend)?;
        write_private_json_atomic(&prepared.transaction_path, &prepared.transaction)?;
    }
    crate::runtime_adoption::require_runtime_host_convergence_deadline(&prepared.transaction)?;
    persist_upgrade_transition(
        &prepared.transaction_path,
        &mut prepared.transaction,
        UpgradeTransactionState::PresentationsRebinding,
        "presentations_rebinding",
    )?;
    prepared.transaction.presentation_validation_summary =
        Some(if prepared.transaction.runtime_migrations.is_empty() {
            "no_live_presentations".to_string()
        } else {
            "runtime_presentations_preserved_for_candidate".to_string()
        });
    write_private_json_atomic(&prepared.transaction_path, &prepared.transaction)?;
    persist_upgrade_transition(
        &prepared.transaction_path,
        &mut prepared.transaction,
        UpgradeTransactionState::CandidateReady,
        "candidate_ready",
    )?;
    persist_admission_drain(&prepared.admission_drain_path, &prepared.transaction)
}

fn candidate_runtime_host_stage_required(
    isolated_root: bool,
    _transferable_lane_count: usize,
) -> bool {
    !isolated_root
}

fn transfer_discovered_runtimes(
    paths: &InstallPaths,
    staged: &StagedWorkstationGeneration,
    transaction_id: &str,
    migrations: &mut [crate::runtime_adoption::RuntimeMigrationRecord],
    handoffs: &mut Vec<PreparedRuntimeHandoff>,
) -> Result<Vec<RuntimeTransferEvidence>, String> {
    use crate::native::service_store::{JsonServiceStateStore, ServiceStateStore};
    use crate::runtime_adoption::{BrowserAdoptionMode, RuntimeClassification, RuntimeDisposition};

    let service_state =
        JsonServiceStateStore::new(JsonServiceStateStore::default_path()?).load()?;
    let old_binary = paths.current_selector.join("bin/agent-browser");
    let candidate_binary = staged.generation_path.join("bin/agent-browser");
    let mut transfer_evidence = Vec::new();
    for migration in migrations {
        let source_session = resolve_runtime_source_session(&service_state, migration)?;
        match migration.disposition {
            RuntimeDisposition::CooperativeTransfer => {
                let Some(source_session) = source_session else {
                    preserve_runtime_without_live_source(migration);
                    continue;
                };
                if !crate::connection::daemon_ready(&source_session) {
                    preserve_runtime_without_live_source(migration);
                    continue;
                }
                let (source_session, prepared, retired_aliases) = match
                    prepare_runtime_handoff_with_alias_fallback(
                        &old_binary,
                        &service_state,
                        migration,
                        &source_session,
                    )
                {
                    Ok(prepared) => prepared,
                    Err((failed_session, error))
                        if matches!(
                            error.kind,
                            RuntimeTransactionCommandFailureKind::ProtocolUnavailable
                                | RuntimeTransactionCommandFailureKind::LegacyTransferredOwnerRejected
                        ) =>
                    {
                        let legacy_transferred_owner_rejected = error.kind
                            == RuntimeTransactionCommandFailureKind::LegacyTransferredOwnerRejected;
                        let evidence = adopt_runtime_via_verified_orphan_fallback(
                            paths,
                            &candidate_binary,
                            transaction_id,
                            &service_state,
                            migration,
                            handoffs,
                            &failed_session,
                            legacy_transferred_owner_rejected,
                            false,
                        )?;
                        transfer_evidence.push(evidence);
                        continue;
                    }
                    Err((_failed_session, error)) => return Err(error.message),
                };
                if !retired_aliases.is_empty() {
                    let reason = "browserless_source_alias_daemon_retired";
                    if !migration.reason_codes.iter().any(|value| value == reason) {
                        migration.reason_codes.push(reason.to_string());
                    }
                }
                if runtime_handoff_prepare_response_kind(&prepared)
                    == RuntimeHandoffPrepareResponseKind::LegacyBrowser
                {
                    let evidence = adopt_runtime_via_verified_orphan_fallback(
                        paths,
                        &candidate_binary,
                        transaction_id,
                        &service_state,
                        migration,
                        handoffs,
                        &source_session,
                        false,
                        true,
                    )?;
                    transfer_evidence.push(evidence);
                    continue;
                }
                let candidate_session = prepared
                    .pointer("/data/candidateSessionName")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| {
                        format!("runtime_transfer_candidate_session_missing:{source_session}")
                    })?
                    .to_string();
                let candidate_socket_dir = candidate_runtime_host_socket_dir(transaction_id)?;
                stage_candidate_runtime_handoff_descriptor(
                    &crate::connection::get_socket_dir(),
                    &candidate_socket_dir,
                    &source_session,
                    &prepared,
                )?;
                handoffs.push(PreparedRuntimeHandoff {
                    source_session: source_session.clone(),
                    candidate_session: candidate_session.clone(),
                    source_process_identity: crate::connection::load_daemon_process_identity(
                        &source_session,
                    )
                    .ok(),
                    mode: BrowserAdoptionMode::CooperativeTransfer,
                    committed: false,
                    source_finalized: false,
                    irreversible_source_revocation: false,
                });
                let handoff_index = handoffs.len() - 1;
                let resumed = match run_candidate_agent_json(
                    &candidate_binary,
                    &candidate_session,
                    transaction_id,
                    &["handoff", "resume", "--source-session", &source_session],
                ) {
                    Ok(resumed) => resumed,
                    Err(error) => {
                        let _ = run_agent_json(&old_binary, &source_session, &["handoff", "abort"]);
                        return Err(error);
                    }
                };
                handoffs[handoff_index].committed = true;
                let receipt = owner_transfer_receipt(&resumed)?;
                migration.adoption_receipt_id = Some(receipt.receipt_id.clone());
                transfer_evidence.push(RuntimeTransferEvidence {
                    logical_browser_id: migration.logical_browser_id.clone(),
                    candidate_session,
                    receipt,
                });
            }
            RuntimeDisposition::OrphanAdoption => {
                let source_session = source_session.ok_or_else(|| {
                    format!(
                        "runtime_orphan_source_session_missing:{}",
                        migration.logical_browser_id
                    )
                })?;
                if !runtime_source_session_is_bound(
                    &service_state,
                    &migration.logical_browser_id,
                    &source_session,
                ) {
                    return Err(format!(
                        "runtime_orphan_logical_browser_not_session_bound:{}",
                        migration.logical_browser_id
                    ));
                }
                if migration.logical_browser_id != format!("session:{source_session}") {
                    migration
                        .reason_codes
                        .push("prior_transaction_session_alias_rebound".to_string());
                }
                if runtime_orphan_owner_requires_fencing(&service_state, migration) {
                    let evidence = adopt_runtime_via_verified_orphan_fallback(
                        paths,
                        &candidate_binary,
                        transaction_id,
                        &service_state,
                        migration,
                        handoffs,
                        &source_session,
                        false,
                        false,
                    )?;
                    transfer_evidence.push(evidence);
                    continue;
                }
                let candidate_session = orphan_candidate_session(migration, transaction_id);
                handoffs.push(PreparedRuntimeHandoff {
                    source_session: source_session.clone(),
                    candidate_session: candidate_session.clone(),
                    source_process_identity: crate::connection::load_daemon_process_identity(
                        &source_session,
                    )
                    .ok(),
                    mode: BrowserAdoptionMode::OrphanAdoption,
                    committed: false,
                    source_finalized: false,
                    irreversible_source_revocation: false,
                });
                let handoff_index = handoffs.len() - 1;
                let resumed = run_candidate_agent_json(
                    &candidate_binary,
                    &candidate_session,
                    transaction_id,
                    &[
                        "handoff",
                        "resume",
                        "--source-session",
                        &source_session,
                        "--logical-browser-id",
                        &migration.logical_browser_id,
                    ],
                )?;
                handoffs[handoff_index].committed = true;
                let receipt = owner_transfer_receipt(&resumed)?;
                migration.adoption_receipt_id = Some(receipt.receipt_id.clone());
                transfer_evidence.push(RuntimeTransferEvidence {
                    logical_browser_id: migration.logical_browser_id.clone(),
                    candidate_session,
                    receipt,
                });
            }
            RuntimeDisposition::ManualPreservation => {}
            RuntimeDisposition::RetiredIdle => {
                if migration.classification == RuntimeClassification::IdleDaemon {
                    let source_session = source_session.ok_or_else(|| {
                        format!(
                            "idle_daemon_source_session_missing:{}",
                            migration.logical_browser_id
                        )
                    })?;
                    let reason = match retire_idle_runtime(&source_session)? {
                        IdleRuntimeRetirement::SharedLaneDeferred => {
                            "idle_shared_lane_deferred_to_host_cutover"
                        }
                        IdleRuntimeRetirement::StandaloneDaemonRetired => "idle_daemon_retired",
                    };
                    migration.reason_codes.push(reason.to_string());
                }
            }
            RuntimeDisposition::RejectedAmbiguity => {
                return Err(format!(
                    "runtime_transfer_rejected_ambiguity:{}",
                    migration.logical_browser_id
                ));
            }
        }
    }
    Ok(transfer_evidence)
}

fn runtime_orphan_owner_requires_fencing(
    service_state: &crate::native::service_model::ServiceState,
    migration: &crate::runtime_adoption::RuntimeMigrationRecord,
) -> bool {
    service_state
        .runtime_owner_registry
        .owner(&migration.profile_identity_digest)
        .is_some_and(|owner| {
            owner.state != crate::runtime_owner_transfer::ProfileOwnerState::Orphaned
        })
}

fn preserve_runtime_without_live_source(
    migration: &mut crate::runtime_adoption::RuntimeMigrationRecord,
) {
    migration.classification = crate::runtime_adoption::RuntimeClassification::ManualPreserveOnly;
    migration.disposition = crate::runtime_adoption::RuntimeDisposition::ManualPreservation;
    let reason = "verified_browser_without_live_source_session_preserved";
    if !migration.reason_codes.iter().any(|value| value == reason) {
        migration.reason_codes.push(reason.to_string());
    }
}

/// Stops a census-proven browserless daemon while the admission drain prevents
/// new effect work. The recorded process identity prevents PID-reuse signals.
/// A Linux daemon whose executable was replaced after launch is reconciled
/// only when its PID, start token, original path, and recorded digest match the
/// live deleted proc inode exactly.
/// A daemon that does not complete graceful shutdown within the bounded grace
/// period is force-stopped through the same verified process handle. No browser
/// process is signaled or closed.
fn retire_idle_daemon(session: &str) -> Result<(), String> {
    retire_idle_runtime(session).map(|_| ())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdleRuntimeRetirement {
    SharedLaneDeferred,
    StandaloneDaemonRetired,
}

/// An idle lane on the selected shared host does not own the host process.
/// Leave it in the old host until generation cutover retires that host once.
/// Only a standalone daemon has process-level retirement authority here.
fn retire_idle_runtime(session: &str) -> Result<IdleRuntimeRetirement, String> {
    if crate::runtime_host::endpoint_key(session) == crate::runtime_host::RUNTIME_HOST_ENDPOINT_KEY
    {
        return Ok(IdleRuntimeRetirement::SharedLaneDeferred);
    }

    let mut identity = crate::connection::load_daemon_process_identity(session)?;
    let process = match crate::process_identity::VerifiedProcessTermination::open(&identity) {
        Ok(process) => process,
        Err(original_error) => {
            let Some(reconciled) = reconcile_deleted_idle_daemon_identity(session, &identity)?
            else {
                return Err(original_error);
            };
            identity = reconciled;
            crate::process_identity::VerifiedProcessTermination::open(&identity)?
        }
    };
    let Some(process) = process else {
        if crate::connection::daemon_ready(session) {
            return Err(format!("idle_daemon_identity_changed:{session}"));
        }
        return Ok(IdleRuntimeRetirement::StandaloneDaemonRetired);
    };
    retire_verified_idle_daemon_process(
        &process,
        LEGACY_DAEMON_EXIT_TIMEOUT,
        LEGACY_DAEMON_EXIT_TIMEOUT,
    )?;
    if crate::connection::daemon_ready(session) {
        return Err(format!("idle_daemon_session_rebound:{session}"));
    }
    Ok(IdleRuntimeRetirement::StandaloneDaemonRetired)
}

#[cfg(target_os = "linux")]
fn reconcile_deleted_idle_daemon_identity(
    session: &str,
    recorded: &crate::process_identity::RecordedProcessIdentity,
) -> Result<Option<crate::process_identity::RecordedProcessIdentity>, String> {
    use crate::process_identity::ProcessObservation;

    let observed = match crate::process_identity::observe_process(recorded.pid) {
        ProcessObservation::Observed(observed) => observed,
        ProcessObservation::Missing | ProcessObservation::Failed { .. } => return Ok(None),
    };
    let recorded_sha_path = crate::connection::get_socket_dir().join(format!("{session}.sha256"));
    let recorded_sha256 = fs::read_to_string(&recorded_sha_path).map_err(display_io(
        "read idle daemon executable digest",
        &recorded_sha_path,
    ))?;
    let proc_executable = PathBuf::from(format!("/proc/{}/exe", recorded.pid));
    let observed_sha256 = workstation_file_sha256(&proc_executable)?;
    let Some(reconciled) = reconciled_deleted_idle_daemon_identity(
        recorded,
        &observed,
        recorded_sha256.trim(),
        &observed_sha256,
    ) else {
        return Ok(None);
    };
    crate::connection::write_daemon_process_identity(session, &reconciled)?;
    Ok(Some(reconciled))
}

#[cfg(not(target_os = "linux"))]
fn reconcile_deleted_idle_daemon_identity(
    _session: &str,
    _recorded: &crate::process_identity::RecordedProcessIdentity,
) -> Result<Option<crate::process_identity::RecordedProcessIdentity>, String> {
    Ok(None)
}

fn reconciled_deleted_idle_daemon_identity(
    recorded: &crate::process_identity::RecordedProcessIdentity,
    observed: &crate::process_identity::ObservedProcessIdentity,
    recorded_sha256: &str,
    observed_sha256: &str,
) -> Option<crate::process_identity::RecordedProcessIdentity> {
    let recorded_executable = recorded.executable_path.as_deref()?;
    let observed_executable = observed.executable_path.as_deref()?;
    let relocated = observed_executable.strip_suffix(" (deleted)")?;
    if recorded.pid != observed.pid
        || observed.start_token.as_deref() != Some(recorded.start_token.as_str())
        || relocated != recorded_executable
        || recorded_sha256.len() != 64
        || recorded_sha256 != observed_sha256
        || recorded.browser_family != observed.browser_family
    {
        return None;
    }
    let mut reconciled = recorded.clone();
    reconciled.executable_path = Some(observed_executable.to_string());
    Some(reconciled)
}

fn retire_verified_idle_daemon_process(
    process: &crate::process_identity::VerifiedProcessTermination,
    graceful_timeout: std::time::Duration,
    forced_timeout: std::time::Duration,
) -> Result<(), String> {
    process.signal(crate::process_identity::VerifiedProcessSignal::Terminate)?;
    let deadline = std::time::Instant::now() + graceful_timeout;
    while process.is_running()? {
        if std::time::Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(LEGACY_DAEMON_EXIT_POLL_INTERVAL);
    }
    if !process.is_running()? {
        return Ok(());
    }
    process.signal(crate::process_identity::VerifiedProcessSignal::Kill)?;
    let deadline = std::time::Instant::now() + forced_timeout;
    while process.is_running()? {
        if std::time::Instant::now() >= deadline {
            return Err("idle_daemon_forced_exit_timeout".to_string());
        }
        std::thread::sleep(LEGACY_DAEMON_EXIT_POLL_INTERVAL);
    }
    Ok(())
}

/// Derive a stable session within one transaction without allowing a failed
/// transaction's temporary candidate daemon to become the next transaction's owner.
fn orphan_candidate_session(
    migration: &crate::runtime_adoption::RuntimeMigrationRecord,
    transaction_id: &str,
) -> String {
    let identity = format!("{}\0{transaction_id}", migration.logical_browser_id);
    format!(
        "orphan-{}",
        &workstation_bytes_sha256(identity.as_bytes())[..16]
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeHandoffPrepareResponseKind {
    NoBrowser,
    LegacyBrowser,
    Cooperative,
    Invalid,
}

fn runtime_handoff_prepare_response_kind(payload: &Value) -> RuntimeHandoffPrepareResponseKind {
    let data = payload.get("data").unwrap_or(payload);
    let prepared = data.get("prepared").and_then(Value::as_bool);
    let browser_present = data.get("browserPresent").and_then(Value::as_bool);
    let candidate_session = data
        .get("candidateSessionName")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty());
    if prepared == Some(false) || browser_present == Some(false) {
        RuntimeHandoffPrepareResponseKind::NoBrowser
    } else if prepared == Some(true) && browser_present == Some(true) && candidate_session {
        RuntimeHandoffPrepareResponseKind::Cooperative
    } else if prepared == Some(true) && browser_present == Some(true) {
        RuntimeHandoffPrepareResponseKind::LegacyBrowser
    } else {
        RuntimeHandoffPrepareResponseKind::Invalid
    }
}

fn alternate_runtime_source_sessions(
    service_state: &crate::native::service_model::ServiceState,
    logical_browser_id: &str,
    primary_session: &str,
) -> Vec<String> {
    service_state
        .sessions
        .values()
        .filter(|session| {
            session.id != primary_session
                && session
                    .browser_ids
                    .iter()
                    .any(|browser_id| browser_id == logical_browser_id)
        })
        .map(|session| session.id.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn prepare_runtime_handoff_with_alias_fallback(
    old_binary: &Path,
    service_state: &crate::native::service_model::ServiceState,
    migration: &crate::runtime_adoption::RuntimeMigrationRecord,
    primary_session: &str,
) -> Result<(String, Value, Vec<String>), (String, RuntimeTransactionCommandFailure)> {
    let mut candidates = vec![primary_session.to_string()];
    candidates.extend(
        alternate_runtime_source_sessions(
            service_state,
            &migration.logical_browser_id,
            primary_session,
        )
        .into_iter()
        .filter(|session| crate::connection::daemon_ready(session)),
    );
    prepare_runtime_handoff_candidates(
        &migration.logical_browser_id,
        primary_session,
        candidates,
        |session| {
            clear_exact_reversed_source_handoff_retry(service_state, session).map_err(
                |message| RuntimeTransactionCommandFailure {
                    kind: RuntimeTransactionCommandFailureKind::CommandFailed,
                    message,
                },
            )?;
            run_agent_json_detailed(old_binary, session, &["handoff", "prepare"])
        },
        retire_idle_daemon,
    )
}

fn clear_exact_reversed_source_handoff_retry(
    service_state: &crate::native::service_model::ServiceState,
    source_session: &str,
) -> Result<bool, String> {
    if !crate::validation::is_valid_session_name(source_session) {
        return Err(crate::validation::session_name_error(source_session));
    }
    let path = crate::connection::get_socket_dir().join(format!("{source_session}.handoff.json"));
    let descriptor_bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "runtime_handoff_prepare_retry_read_failed:{source_session}:{error}"
            ));
        }
    };
    let descriptor: crate::native::action_runtime::runtime::RuntimeHandoffDescriptor =
        serde_json::from_slice(&descriptor_bytes).map_err(|error| {
            format!("runtime_handoff_prepare_retry_invalid:{source_session}:{error}")
        })?;
    let Some(proposal) = descriptor.owner_transfer.as_ref() else {
        return Ok(false);
    };
    let Some(current_owner) = service_state
        .runtime_owner_registry
        .owner(&proposal.request.profile_identity_digest)
    else {
        return Ok(false);
    };
    if current_owner.pending_transfer.as_ref() == Some(proposal)
        || !crate::native::action_runtime::runtime::reversed_handoff_retry_matches_current_owner(
            &descriptor.session_name,
            proposal,
            current_owner,
        )
    {
        return Ok(false);
    }
    fs::remove_file(&path).map_err(|error| {
        format!("runtime_handoff_prepare_stale_retry_cleanup_failed:{source_session}:{error}")
    })?;
    Ok(true)
}

fn prepare_runtime_handoff_candidates<Prepare, Retire>(
    logical_browser_id: &str,
    primary_session: &str,
    candidates: Vec<String>,
    mut prepare: Prepare,
    mut retire: Retire,
) -> Result<(String, Value, Vec<String>), (String, RuntimeTransactionCommandFailure)>
where
    Prepare: FnMut(&str) -> Result<Value, RuntimeTransactionCommandFailure>,
    Retire: FnMut(&str) -> Result<(), String>,
{
    let mut retired_aliases = Vec::new();
    let mut selected: Option<(String, Value)> = None;
    for session in candidates {
        match prepare(&session) {
            Ok(payload) => match runtime_handoff_prepare_response_kind(&payload) {
                RuntimeHandoffPrepareResponseKind::NoBrowser => {
                    if let Err(message) = retire(&session) {
                        return Err((
                            session,
                            RuntimeTransactionCommandFailure {
                                kind: RuntimeTransactionCommandFailureKind::CommandFailed,
                                message,
                            },
                        ));
                    }
                    retired_aliases.push(session);
                }
                RuntimeHandoffPrepareResponseKind::LegacyBrowser
                | RuntimeHandoffPrepareResponseKind::Cooperative => {
                    if let Some((selected_session, _)) = &selected {
                        return Err((
                            session.clone(),
                            RuntimeTransactionCommandFailure {
                                kind: RuntimeTransactionCommandFailureKind::CommandFailed,
                                message: format!(
                                    "runtime_handoff_alternate_browser_conflict:{logical_browser_id}:{selected_session}:{session}"
                                ),
                            },
                        ));
                    }
                    selected = Some((session, payload));
                }
                RuntimeHandoffPrepareResponseKind::Invalid => {
                    return Err((
                        session.clone(),
                        RuntimeTransactionCommandFailure {
                            kind: RuntimeTransactionCommandFailureKind::CommandFailed,
                            message: format!("runtime_handoff_prepare_response_invalid:{session}"),
                        },
                    ));
                }
            },
            Err(error)
                if matches!(
                    error.kind,
                    RuntimeTransactionCommandFailureKind::ObservationOnlyAlias
                        | RuntimeTransactionCommandFailureKind::LegacyTransferredOwnerRejected
                        | RuntimeTransactionCommandFailureKind::BrowserUnavailableAlias
                ) && selected.is_some()
                    && session != primary_session =>
            {
                if let Err(message) = retire(&session) {
                    return Err((
                        session,
                        RuntimeTransactionCommandFailure {
                            kind: RuntimeTransactionCommandFailureKind::CommandFailed,
                            message,
                        },
                    ));
                }
                retired_aliases.push(session);
            }
            Err(error) => return Err((session, error)),
        }
    }
    if let Some((session, payload)) = selected {
        return Ok((session, payload, retired_aliases));
    }
    Err((
        primary_session.to_string(),
        RuntimeTransactionCommandFailure {
            kind: RuntimeTransactionCommandFailureKind::CommandFailed,
            message: format!(
                "runtime_transfer_source_browser_missing:{}",
                logical_browser_id
            ),
        },
    ))
}

#[allow(clippy::too_many_arguments)]
fn adopt_runtime_via_verified_orphan_fallback(
    paths: &InstallPaths,
    candidate_binary: &Path,
    transaction_id: &str,
    service_state: &crate::native::service_model::ServiceState,
    migration: &mut crate::runtime_adoption::RuntimeMigrationRecord,
    handoffs: &mut Vec<PreparedRuntimeHandoff>,
    source_session: &str,
    legacy_transferred_owner_rejected: bool,
    legacy_prepare_v1: bool,
) -> Result<RuntimeTransferEvidence, String> {
    use crate::runtime_adoption::{BrowserAdoptionMode, RuntimeDisposition};

    if legacy_transferred_owner_rejected {
        if !legacy_transferred_owner_prepare_rejection_can_fallback(
            service_state,
            migration,
            source_session,
        ) {
            return Err("runtime_owner_current_evidence_mismatch: transferred owner fallback evidence is incomplete".to_string());
        }
    } else if !runtime_source_session_is_bound(
        service_state,
        &migration.logical_browser_id,
        source_session,
    ) {
        return Err(format!(
            "runtime_orphan_logical_browser_not_session_bound:{}",
            migration.logical_browser_id
        ));
    }
    let candidate_session = orphan_candidate_session(migration, transaction_id);
    let expected_owner = service_state
        .runtime_owner_registry
        .owner(&migration.profile_identity_digest)
        .cloned();
    if expected_owner.is_none()
        && !legacy_ownerless_orphan_bootstrap_allowed(
            migration,
            legacy_transferred_owner_rejected,
            legacy_prepare_v1,
        )
    {
        return Err(format!(
            "legacy_daemon_owner_missing:{}",
            migration.logical_browser_id
        ));
    }
    handoffs.push(PreparedRuntimeHandoff {
        source_session: source_session.to_string(),
        candidate_session: candidate_session.clone(),
        source_process_identity: crate::connection::load_daemon_process_identity(source_session)
            .ok(),
        mode: BrowserAdoptionMode::OrphanAdoption,
        committed: false,
        source_finalized: false,
        irreversible_source_revocation: false,
    });
    let handoff_index = handoffs.len() - 1;
    let revocation = revoke_legacy_daemon_effect_authority(
        paths,
        source_session,
        expected_owner.as_ref(),
        &mut handoffs[handoff_index].irreversible_source_revocation,
    );
    if let Err(error) = revocation {
        if handoffs[handoff_index].irreversible_source_revocation {
            migration.disposition = RuntimeDisposition::OrphanAdoption;
            let reason = "legacy_daemon_effect_authority_revocation_incomplete";
            if !migration.reason_codes.iter().any(|value| value == reason) {
                migration.reason_codes.push(reason.to_string());
            }
        }
        return Err(error);
    }
    migration.disposition = RuntimeDisposition::OrphanAdoption;
    let reason = if legacy_transferred_owner_rejected {
        "legacy_transferred_owner_prepare_rejected_effect_authority_revoked"
    } else if legacy_prepare_v1 {
        "legacy_daemon_protocol_v1_effect_authority_revoked"
    } else {
        "legacy_daemon_protocol_unavailable_effect_authority_revoked"
    };
    if !migration.reason_codes.iter().any(|value| value == reason) {
        migration.reason_codes.push(reason.to_string());
    }
    let resumed = run_candidate_agent_json(
        candidate_binary,
        &candidate_session,
        transaction_id,
        &[
            "handoff",
            "resume",
            "--source-session",
            source_session,
            "--logical-browser-id",
            &migration.logical_browser_id,
        ],
    )?;
    handoffs[handoff_index].committed = true;
    let receipt = owner_transfer_receipt(&resumed)?;
    migration.adoption_receipt_id = Some(receipt.receipt_id.clone());
    Ok(RuntimeTransferEvidence {
        logical_browser_id: migration.logical_browser_id.clone(),
        candidate_session,
        receipt,
    })
}

fn legacy_ownerless_orphan_bootstrap_allowed(
    migration: &crate::runtime_adoption::RuntimeMigrationRecord,
    legacy_transferred_owner_rejected: bool,
    legacy_prepare_v1: bool,
) -> bool {
    !legacy_transferred_owner_rejected
        && legacy_prepare_v1
        && migration
            .reason_codes
            .iter()
            .any(|reason| reason == "cooperative_owner_registration_required")
}

fn runtime_source_session_is_bound(
    service_state: &crate::native::service_model::ServiceState,
    logical_browser_id: &str,
    source_session: &str,
) -> bool {
    // A failed transaction may leave its transaction-scoped candidate session
    // as the retained browser alias after the owner registry has rolled back.
    // The service session's exact browser binding authorizes a new candidate
    // to re-adopt that browser without reusing the prior candidate daemon.
    logical_browser_id == format!("session:{source_session}")
        || service_state
            .sessions
            .get(source_session)
            .is_some_and(|session| {
                session
                    .browser_ids
                    .iter()
                    .any(|browser_id| browser_id == logical_browser_id)
            })
        || service_state.remote_view_handoffs.values().any(|handoff| {
            handoff.state == "ready"
                && handoff.browser_id.as_deref() == Some(logical_browser_id)
                && handoff.session_name.as_deref() == Some(source_session)
                && handoff
                    .presentation_receipt
                    .as_ref()
                    .is_some_and(|receipt| {
                        receipt.state == "ready"
                            && receipt.logical_browser_id == logical_browser_id
                            && service_state
                                .runtime_owner_registry
                                .owners
                                .values()
                                .any(|owner| {
                                    owner.state
                                        == crate::runtime_owner_transfer::ProfileOwnerState::Ready
                                        && owner.browser_id == logical_browser_id
                                        && owner.daemon_session_route == source_session
                                        && Some(owner.owner_generation)
                                            == receipt.daemon_owner_generation
                                        && receipt.process_instance_digest.as_deref()
                                            == Some(owner.process_instance_digest.as_str())
                                })
                    })
        })
}

/// Revokes one legacy daemon only after its recorded process identity proves it
/// matches a binary hash from a sealed retained generation. The browser
/// process is not signaled; the candidate must independently prove and adopt
/// that orphan.
fn revoke_legacy_daemon_effect_authority(
    paths: &InstallPaths,
    source_session: &str,
    expected_owner: Option<&crate::runtime_owner_transfer::ProfileOwner>,
    source_authority_unavailable: &mut bool,
) -> Result<(), String> {
    let identity = legacy_daemon_identity_for_revocation(
        source_session,
        crate::connection::load_daemon_process_identity,
        crate::connection::daemon_ready,
    )?;
    if let Some(identity) = identity {
        let authorized_binary_hashes =
            authorized_runtime_generation_binary_hashes(&paths.generations_dir)?;
        revoke_verified_legacy_daemon_process(
            &identity,
            &authorized_binary_hashes,
            LEGACY_DAEMON_EXIT_TIMEOUT,
            source_authority_unavailable,
        )?;
    } else {
        // No route can accept effects and no process identity exists to signal.
        // The owner-generation transition below is the effect-authority fence;
        // any stale in-memory binding will fail its next registry check.
        *source_authority_unavailable = true;
    }

    let deadline = std::time::Instant::now() + LEGACY_DAEMON_EXIT_TIMEOUT;
    while crate::connection::daemon_ready(source_session) {
        if std::time::Instant::now() >= deadline {
            return Err(format!(
                "legacy_daemon_effect_authority_still_reachable:{source_session}"
            ));
        }
        std::thread::sleep(LEGACY_DAEMON_EXIT_POLL_INTERVAL);
    }
    std::thread::sleep(std::time::Duration::from_millis(150));
    if crate::connection::daemon_ready(source_session) {
        return Err(format!(
            "legacy_daemon_effect_authority_reappeared:{source_session}"
        ));
    }
    if let Some(expected_owner) = expected_owner {
        let repository =
            crate::native::service_store::LockedServiceStateRepository::default_json()?;
        crate::native::runtime_lifecycle::RuntimeLifecycleAuthority::new(&repository)
            .revoke_legacy_owner(
                &expected_owner.profile_identity_digest,
                &expected_owner.browser_id,
                &expected_owner.daemon_session_route,
                &expected_owner.owner_id,
                expected_owner.owner_generation,
            )?;
    }
    Ok(())
}

fn legacy_daemon_identity_for_revocation(
    session: &str,
    load_identity: impl FnOnce(&str) -> Result<crate::process_identity::RecordedProcessIdentity, String>,
    route_ready: impl FnOnce(&str) -> bool,
) -> Result<Option<crate::process_identity::RecordedProcessIdentity>, String> {
    match load_identity(session) {
        Ok(identity) => Ok(Some(identity)),
        Err(_error) if !route_ready(session) => Ok(None),
        Err(error) => Err(format!(
            "legacy_daemon_identity_unavailable_while_reachable:{session}:{error}"
        )),
    }
}

fn revoke_verified_legacy_daemon_process(
    identity: &crate::process_identity::RecordedProcessIdentity,
    authorized_binary_hashes: &std::collections::BTreeSet<String>,
    exit_timeout: std::time::Duration,
    source_authority_unavailable: &mut bool,
) -> Result<(), String> {
    let recorded_executable = identity
        .executable_path
        .as_deref()
        .ok_or_else(|| "legacy_daemon_executable_identity_unavailable".to_string())?;
    let recorded = Path::new(recorded_executable)
        .canonicalize()
        .map_err(|error| format!("legacy_daemon_recorded_binary_unavailable:{error}"))?;
    let recorded_hash = workstation_file_sha256(&recorded)?;
    if !authorized_binary_hashes.contains(&recorded_hash) {
        return Err("legacy_daemon_executable_provenance_mismatch".to_string());
    }

    let Some(process) = crate::process_identity::VerifiedProcessTermination::open(identity)? else {
        *source_authority_unavailable = true;
        return Ok(());
    };
    process.signal(crate::process_identity::VerifiedProcessSignal::Kill)?;
    *source_authority_unavailable = true;
    let deadline = std::time::Instant::now() + exit_timeout;
    while process.is_running()? {
        if std::time::Instant::now() >= deadline {
            return Err("legacy_daemon_exact_process_exit_timeout".to_string());
        }
        std::thread::sleep(LEGACY_DAEMON_EXIT_POLL_INTERVAL);
    }
    Ok(())
}

fn authorized_runtime_generation_binary_hashes(
    generations_dir: &Path,
) -> Result<std::collections::BTreeSet<String>, String> {
    let mut hashes = std::collections::BTreeSet::new();
    for entry in fs::read_dir(generations_dir).map_err(display_io(
        "read installed runtime generations",
        generations_dir,
    ))? {
        let entry = entry.map_err(|error| {
            format!("Unable to read installed runtime generation entry: {error}")
        })?;
        let generation_path = entry.path();
        if !generation_path.is_dir() {
            continue;
        }
        let generation_id = entry.file_name().to_string_lossy().into_owned();
        let manifest_path = generation_path.join("generation.json");
        let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).map_err(
            display_io("read installed runtime generation manifest", &manifest_path),
        )?)
        .map_err(|error| {
            format!(
                "Installed runtime generation manifest is invalid for '{generation_id}': {error}"
            )
        })?;
        let declared_generation = manifest
            .get("generationId")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("installed runtime generation id is missing:{generation_id}"))?;
        let declared_hash = manifest
            .get("binarySha256")
            .and_then(Value::as_str)
            .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or_else(|| {
                format!("installed runtime generation binary hash is invalid:{generation_id}")
            })?;
        if declared_generation != generation_id {
            return Err(format!(
                "installed runtime generation manifest identity mismatch:{generation_id}"
            ));
        }
        validate_sealed_generation_tree(&generation_path)?;
        let actual_hash = workstation_file_sha256(&generation_path.join("bin/agent-browser"))?;
        if actual_hash != declared_hash {
            return Err(format!(
                "installed runtime generation binary hash mismatch:{generation_id}"
            ));
        }
        hashes.insert(actual_hash);
    }
    if hashes.is_empty() {
        return Err("installed runtime generation provenance is unavailable".to_string());
    }
    Ok(hashes)
}

fn resolve_runtime_source_session(
    service_state: &crate::native::service_model::ServiceState,
    migration: &crate::runtime_adoption::RuntimeMigrationRecord,
) -> Result<Option<String>, String> {
    resolve_runtime_source_session_with_probe(service_state, migration, |session| {
        crate::connection::daemon_ready(session)
    })
}

fn resolve_runtime_source_session_with_probe(
    service_state: &crate::native::service_model::ServiceState,
    migration: &crate::runtime_adoption::RuntimeMigrationRecord,
    session_ready: impl Fn(&str) -> bool,
) -> Result<Option<String>, String> {
    let mut candidates = std::collections::BTreeSet::new();
    if let Some(owner) = service_state
        .runtime_owner_registry
        .owner(&migration.profile_identity_digest)
    {
        if owner.state != crate::runtime_owner_transfer::ProfileOwnerState::Orphaned {
            candidates.insert(owner.daemon_session_route.clone());
        }
    }
    if let Some(browser) = service_state.browsers.get(&migration.logical_browser_id) {
        candidates.extend(browser.active_session_ids.iter().cloned());
    }
    candidates.retain(|value| !value.trim().is_empty());
    if candidates.is_empty() {
        candidates.extend(
            migration
                .session_names
                .iter()
                .filter(|session| !session.trim().is_empty())
                .cloned(),
        );
    }
    if candidates.is_empty() {
        if let Some(session) = migration.logical_browser_id.strip_prefix("session:") {
            let session_is_unbound_or_matches =
                service_state.sessions.get(session).is_none_or(|record| {
                    record
                        .browser_ids
                        .iter()
                        .any(|browser_id| browser_id == &migration.logical_browser_id)
                });
            if session_is_unbound_or_matches {
                candidates.insert(session.to_string());
            }
        }
    }
    let live_candidates = candidates
        .iter()
        .filter(|session| session_ready(session))
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if !live_candidates.is_empty() {
        candidates = live_candidates;
    }
    if candidates.len() > 1 {
        return Err(format!(
            "runtime_transfer_source_session_ambiguous:{}",
            migration.logical_browser_id
        ));
    }
    Ok(candidates.into_iter().next())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeTransactionCommandFailureKind {
    ProtocolUnavailable,
    LegacyTransferredOwnerRejected,
    ObservationOnlyAlias,
    BrowserUnavailableAlias,
    CommandFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeTransactionCommandFailure {
    kind: RuntimeTransactionCommandFailureKind,
    message: String,
}

fn runtime_transaction_failure_kind(
    payload: Option<&Value>,
    diagnostic: &str,
    command_args: &[&str],
) -> RuntimeTransactionCommandFailureKind {
    let handoff_command = command_args.first().copied() == Some("handoff");
    let typed_unknown = payload
        .and_then(|value| value.get("type"))
        .and_then(Value::as_str)
        .is_some_and(|value| matches!(value, "unknown_command" | "unknown_subcommand"));
    let normalized = diagnostic.to_ascii_lowercase();
    let textual_unknown = handoff_command
        && (normalized.contains("unknown command") || normalized.contains("unknown subcommand"))
        && normalized.contains("handoff");
    let legacy_transferred_owner_rejected = command_args == ["handoff", "prepare"]
        && normalized.contains("runtime_owner_current_evidence_mismatch:");
    let observation_only_alias = command_args == ["handoff", "prepare"]
        && normalized.contains("runtime_owner_observation_only:");
    let browser_unavailable_alias =
        command_args == ["handoff", "prepare"] && normalized.contains("browser pid is unavailable");
    if handoff_command && (typed_unknown || textual_unknown) {
        RuntimeTransactionCommandFailureKind::ProtocolUnavailable
    } else if legacy_transferred_owner_rejected {
        RuntimeTransactionCommandFailureKind::LegacyTransferredOwnerRejected
    } else if observation_only_alias {
        RuntimeTransactionCommandFailureKind::ObservationOnlyAlias
    } else if browser_unavailable_alias {
        RuntimeTransactionCommandFailureKind::BrowserUnavailableAlias
    } else {
        RuntimeTransactionCommandFailureKind::CommandFailed
    }
}

fn legacy_transferred_owner_prepare_rejection_can_fallback(
    service_state: &crate::native::service_model::ServiceState,
    migration: &crate::runtime_adoption::RuntimeMigrationRecord,
    source_session: &str,
) -> bool {
    service_state
        .runtime_owner_registry
        .owner(&migration.profile_identity_digest)
        .is_some_and(|owner| {
            owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Ready
                && owner.pending_transfer.is_none()
                && owner.owner_generation > 1
                && owner.browser_id == migration.logical_browser_id
                && owner.daemon_session_route == source_session
                && owner.browser_id != format!("session:{source_session}")
        })
}

fn run_agent_json_detailed(
    binary: &Path,
    session: &str,
    command_args: &[&str],
) -> Result<Value, RuntimeTransactionCommandFailure> {
    run_agent_json_detailed_in_socket_dir(binary, session, command_args, None)
}

fn run_agent_json_detailed_in_socket_dir(
    binary: &Path,
    session: &str,
    command_args: &[&str],
    runtime: Option<(&Path, bool)>,
) -> Result<Value, RuntimeTransactionCommandFailure> {
    let mut command = Command::new(binary);
    command
        .args(["--json", "--session", session])
        .args(command_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some((socket_dir, runtime_host)) = runtime {
        command
            .env(
                crate::runtime_host::RUNTIME_HOST_ENV,
                if runtime_host { "1" } else { "0" },
            )
            .env("AGENT_BROWSER_SOCKET_DIR", socket_dir);
    }
    let output = command
        .output()
        .map_err(|error| RuntimeTransactionCommandFailure {
            kind: RuntimeTransactionCommandFailureKind::CommandFailed,
            message: format!(
                "Unable to run runtime transaction client {} for session '{session}': {error}",
                binary.display()
            ),
        })?;
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let payload: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let diagnostic = format!("{stdout}\n{stderr}");
        RuntimeTransactionCommandFailure {
            kind: runtime_transaction_failure_kind(None, &diagnostic, command_args),
            message: format!(
                "Runtime transaction client returned invalid JSON for session '{session}': {error}; {stderr}"
            ),
        }
    })?;
    if !output.status.success() || payload.get("success").and_then(Value::as_bool) != Some(true) {
        let message = payload
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or(stderr);
        return Err(RuntimeTransactionCommandFailure {
            kind: runtime_transaction_failure_kind(Some(&payload), &message, command_args),
            message: format!(
                "Runtime transaction command failed for session '{session}': {message}"
            ),
        });
    }
    Ok(payload)
}

fn candidate_runtime_host_socket_dir(transaction_id: &str) -> Result<PathBuf, String> {
    if transaction_id.trim().is_empty()
        || !transaction_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("candidate_runtime_host_transaction_id_invalid".to_string());
    }
    let runtime_root = if let Some(root) = env::var_os("AGENT_BROWSER_WORKSTATION_ROOT") {
        PathBuf::from(root).join(".agent-browser/runtime-hosts")
    } else if let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        runtime_dir.join("agent-browser/runtime-hosts")
    } else {
        dirs::home_dir()
            .ok_or_else(|| "Unable to determine home directory".to_string())?
            .join(".agent-browser/runtime-hosts")
    };
    Ok(candidate_runtime_host_socket_dir_in(
        &runtime_root,
        transaction_id,
    ))
}

fn candidate_runtime_host_socket_dir_in(runtime_root: &Path, transaction_id: &str) -> PathBuf {
    let digest = workstation_bytes_sha256(transaction_id.as_bytes());
    runtime_root.join(&digest[..16])
}

fn capture_runtime_host_identity(
    socket_dir: &Path,
    generation_id: &str,
    binary_sha256: &str,
    observation_only: bool,
) -> Result<
    (
        crate::runtime_adoption::RuntimeHostIdentityEvidence,
        crate::runtime_host_ingress::RuntimeHostBackend,
    ),
    String,
> {
    let manifest_path = socket_dir.join("runtime-host.json");
    let identity_path = socket_dir.join("runtime-host.identity.json");
    let sha_path = socket_dir.join("runtime-host.sha256");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let (manifest, identity) = loop {
        let manifest = fs::read(&manifest_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        let identity = fs::read(&identity_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        if let (Some(manifest), Some(identity)) = (manifest, identity) {
            break (manifest, identity);
        }
        if std::time::Instant::now() >= deadline {
            return Err(format!(
                "runtime_host_executable_identity_not_ready: socketDir={} manifestReady={} identityReady={}",
                socket_dir.display()
                , manifest_path.is_file()
                , identity_path.is_file()
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    };
    let manifest: crate::runtime_host::RuntimeHostManifest = manifest;
    let identity: crate::process_identity::RecordedProcessIdentity = identity;
    let executable_sha256 = manifest.executable_generation.clone();
    if executable_sha256 != binary_sha256 {
        return Err("runtime_host_executable_identity_mismatch".to_string());
    }
    if let Ok(sidecar_sha256) = fs::read_to_string(&sha_path) {
        let sidecar_sha256 = sidecar_sha256.trim();
        if sidecar_sha256 != "pending" && sidecar_sha256 != executable_sha256 {
            return Err("runtime_host_executable_identity_sidecar_mismatch".to_string());
        }
    }
    if manifest.schema_version != "agent-browser.runtime-host.v1"
        || manifest.pid != identity.pid
        || manifest.executable_generation != executable_sha256
        || manifest.socket_identity.trim().is_empty()
        || identity.start_token.trim().is_empty()
    {
        return Err("runtime_host_identity_inconsistent".to_string());
    }
    let evidence = crate::runtime_adoption::RuntimeHostIdentityEvidence {
        endpoint_key: crate::runtime_host::RUNTIME_HOST_ENDPOINT_KEY.to_string(),
        generation_id: generation_id.to_string(),
        binary_sha256: executable_sha256.clone(),
        pid: identity.pid,
        process_start_token: identity.start_token,
        socket_identity: manifest.socket_identity.clone(),
        observation_only,
    };
    let backend = crate::runtime_host_ingress::RuntimeHostBackend {
        topology: crate::runtime_host_ingress::RuntimeHostTopology::SingleHost,
        generation_id: generation_id.to_string(),
        socket_dir: socket_dir.to_path_buf(),
        binary_sha256: executable_sha256,
        host_id: manifest.host_id,
        pid: manifest.pid,
        socket_identity: manifest.socket_identity,
    };
    Ok((evidence, backend))
}

fn stage_candidate_runtime_host_ingress(
    paths: &InstallPaths,
    transaction: &mut crate::runtime_adoption::UpgradeTransaction,
    candidate: crate::runtime_host_ingress::RuntimeHostBackend,
) -> Result<(), String> {
    let repository = crate::runtime_host_ingress::RuntimeHostIngressRepository::new(
        crate::runtime_host_ingress::RuntimeHostIngressRepository::default_path(),
    );
    let registry = match repository.load() {
        Ok(registry) => registry,
        Err(error) if error.contains("Unable to read runtime host ingress state") => {
            let old_generation = transaction
                .old_generation_id
                .as_deref()
                .ok_or_else(|| "runtime_host_ingress_old_generation_missing".to_string())?;
            let old_socket_dir = crate::connection::get_socket_dir();
            let old_sha_path = old_socket_dir.join("runtime-host.sha256");
            if old_sha_path.is_file() {
                let old_sha = fs::read_to_string(&old_sha_path)
                    .map_err(display_io("read old runtime host hash", &old_sha_path))?;
                let (old_identity, old_backend) = capture_runtime_host_identity(
                    &old_socket_dir,
                    old_generation,
                    old_sha.trim(),
                    false,
                )?;
                crate::runtime_adoption::record_runtime_host_identity(
                    transaction,
                    false,
                    old_identity,
                )?;
                repository.initialize(old_backend)?
            } else {
                let old_binary = paths.current_selector.join("bin/agent-browser");
                let legacy_backend = crate::runtime_host_ingress::RuntimeHostBackend {
                    topology: crate::runtime_host_ingress::RuntimeHostTopology::LegacyPerSession,
                    generation_id: old_generation.to_string(),
                    socket_dir: old_socket_dir,
                    binary_sha256: workstation_file_sha256(&old_binary)?,
                    host_id: format!(
                        "legacy-runtime-set:{}",
                        transaction
                            .runtime_census_digest
                            .as_deref()
                            .unwrap_or("unknown")
                    ),
                    pid: 0,
                    socket_identity: "legacy-per-session-endpoints".to_string(),
                };
                repository.initialize(legacy_backend)?
            }
        }
        Err(error) => return Err(error),
    };
    let old_generation = transaction
        .old_generation_id
        .as_deref()
        .ok_or_else(|| "runtime_host_ingress_old_generation_missing".to_string())?;
    let registry = if registry.selected_backend().generation_id != old_generation {
        let selected = registry.selected_backend();
        let fallback_matches = registry
            .fallback_backend()
            .is_some_and(|fallback| fallback.generation_id == old_generation);
        let selected_is_missing = matches!(
            crate::process_identity::observe_process(selected.pid),
            crate::process_identity::ProcessObservation::Missing
        );
        if registry.active_transaction_id.is_none()
            && registry.candidate_backend().is_none()
            && fallback_matches
            && selected_is_missing
        {
            repository.recover_dead_selected_backend(
                registry.revision,
                &selected.generation_id,
                selected.pid,
                old_generation,
            )?
        } else {
            return Err("runtime_host_ingress_selected_generation_drift".to_string());
        }
    } else {
        registry
    };
    if selected_runtime_host_capture_required(
        registry.selected_backend().topology,
        matches!(
            crate::process_identity::observe_process(registry.selected_backend().pid),
            crate::process_identity::ProcessObservation::Missing
        ),
        transaction
            .runtime_host_convergence
            .as_ref()
            .is_some_and(|convergence| convergence.old_host.is_some()),
    ) {
        return Err(
            "runtime_host_ingress_selected_identity_not_captured_before_transfer".to_string(),
        );
    }
    repository.stage_candidate(registry.revision, &transaction.transaction_id, candidate)?;
    Ok(())
}

fn selected_runtime_host_capture_required(
    topology: crate::runtime_host_ingress::RuntimeHostTopology,
    selected_host_absent: bool,
    old_host_present: bool,
) -> bool {
    topology == crate::runtime_host_ingress::RuntimeHostTopology::SingleHost
        && !selected_host_absent
        && !old_host_present
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedRuntimeHostCaptureAction {
    Skip,
    CaptureExact,
    RefreshIdentity,
}

fn selected_runtime_host_capture_action(
    topology: crate::runtime_host_ingress::RuntimeHostTopology,
    selected_host_absent: bool,
    old_host_present: bool,
) -> SelectedRuntimeHostCaptureAction {
    if old_host_present || topology != crate::runtime_host_ingress::RuntimeHostTopology::SingleHost
    {
        SelectedRuntimeHostCaptureAction::Skip
    } else if selected_host_absent {
        SelectedRuntimeHostCaptureAction::RefreshIdentity
    } else {
        SelectedRuntimeHostCaptureAction::CaptureExact
    }
}

fn capture_selected_runtime_host_before_transfer(
    transaction: &mut crate::runtime_adoption::UpgradeTransaction,
) -> Result<(), String> {
    let repository = crate::runtime_host_ingress::RuntimeHostIngressRepository::new(
        crate::runtime_host_ingress::RuntimeHostIngressRepository::default_path(),
    );
    let registry = match repository.load() {
        Ok(registry) => registry,
        Err(error) if error.contains("Unable to read runtime host ingress state") => return Ok(()),
        Err(error) => return Err(error),
    };
    let selected = registry.selected_backend();
    let old_host_present = transaction
        .runtime_host_convergence
        .as_ref()
        .is_some_and(|convergence| convergence.old_host.is_some());
    let selected_host_absent = match crate::process_identity::observe_process(selected.pid) {
        crate::process_identity::ProcessObservation::Missing => true,
        crate::process_identity::ProcessObservation::Observed(_) => false,
        crate::process_identity::ProcessObservation::Failed { reason } => {
            return Err(format!(
                "runtime_host_ingress_selected_process_observation_failed: {reason}"
            ))
        }
    };
    let capture_action = selected_runtime_host_capture_action(
        selected.topology,
        selected_host_absent,
        old_host_present,
    );
    if capture_action == SelectedRuntimeHostCaptureAction::Skip {
        return Ok(());
    }
    if capture_action == SelectedRuntimeHostCaptureAction::RefreshIdentity {
        let expected_selected = selected.clone();
        let (old_identity, observed_backend) = capture_runtime_host_identity(
            &selected.socket_dir,
            &selected.generation_id,
            &selected.binary_sha256,
            false,
        )?;
        repository.refresh_selected_backend_identity(
            registry.revision,
            &expected_selected,
            observed_backend,
        )?;
        return crate::runtime_adoption::record_runtime_host_identity(
            transaction,
            false,
            old_identity,
        );
    }
    let (old_identity, observed_backend) = capture_runtime_host_identity(
        &selected.socket_dir,
        &selected.generation_id,
        &selected.binary_sha256,
        false,
    )?;
    if &observed_backend != selected {
        return Err("runtime_host_ingress_selected_identity_changed".to_string());
    }
    crate::runtime_adoption::record_runtime_host_identity(transaction, false, old_identity)
}

fn run_candidate_agent_json(
    binary: &Path,
    session: &str,
    transaction_id: &str,
    command_args: &[&str],
) -> Result<Value, String> {
    let socket_dir = candidate_runtime_host_socket_dir(transaction_id)?;
    run_candidate_agent_json_in_socket_dir(binary, session, &socket_dir, command_args)
}

fn run_candidate_agent_json_in_socket_dir(
    binary: &Path,
    session: &str,
    socket_dir: &Path,
    command_args: &[&str],
) -> Result<Value, String> {
    fs::create_dir_all(socket_dir).map_err(display_io(
        "create candidate runtime host socket directory",
        socket_dir,
    ))?;
    run_agent_json_detailed_in_socket_dir(binary, session, command_args, Some((socket_dir, true)))
        .map_err(|error| error.message)
}

fn stage_candidate_runtime_handoff_descriptor(
    source_socket_dir: &Path,
    candidate_socket_dir: &Path,
    source_session: &str,
    prepared: &Value,
) -> Result<PathBuf, String> {
    if !crate::validation::is_valid_session_name(source_session) {
        return Err(crate::validation::session_name_error(source_session));
    }
    let expected_name = format!("{source_session}.handoff.json");
    let expected_source = source_socket_dir.join(&expected_name);
    let reported_source = prepared
        .pointer("/data/handoffPath")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| format!("runtime_handoff_prepare_path_missing:{source_session}"))?;
    let expected_source = fs::canonicalize(&expected_source).map_err(display_io(
        "resolve prepared runtime handoff descriptor",
        &expected_source,
    ))?;
    let reported_source = fs::canonicalize(&reported_source).map_err(display_io(
        "resolve reported runtime handoff descriptor",
        &reported_source,
    ))?;
    if reported_source != expected_source {
        return Err(format!(
            "runtime_handoff_prepare_path_mismatch:{source_session}"
        ));
    }
    let descriptor_bytes = fs::read(&reported_source).map_err(display_io(
        "read prepared runtime handoff descriptor",
        &reported_source,
    ))?;
    let descriptor: Value = serde_json::from_slice(&descriptor_bytes).map_err(|error| {
        format!("runtime_handoff_prepare_descriptor_invalid:{source_session}:{error}")
    })?;
    if descriptor.get("schemaVersion").and_then(Value::as_u64) != Some(2)
        || descriptor.get("sessionName").and_then(Value::as_str) != Some(source_session)
    {
        return Err(format!(
            "runtime_handoff_prepare_descriptor_identity_mismatch:{source_session}"
        ));
    }
    let candidate_session = prepared
        .pointer("/data/candidateSessionName")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("runtime_transfer_candidate_session_missing:{source_session}"))?;
    if descriptor
        .pointer("/ownerTransfer/request/candidateDaemonSessionRoute")
        .and_then(Value::as_str)
        != Some(candidate_session)
    {
        return Err(format!(
            "runtime_handoff_prepare_candidate_identity_mismatch:{source_session}"
        ));
    }
    fs::create_dir_all(candidate_socket_dir).map_err(display_io(
        "create candidate runtime host socket directory",
        candidate_socket_dir,
    ))?;
    let candidate_path = candidate_socket_dir.join(expected_name);
    write_private_json_atomic(&candidate_path, &descriptor)?;
    Ok(candidate_path)
}

fn clear_candidate_runtime_handoff_descriptor(
    transaction_id: &str,
    source_session: &str,
) -> Result<(), String> {
    let path = candidate_runtime_host_socket_dir(transaction_id)?
        .join(format!("{source_session}.handoff.json"));
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "Unable to remove candidate runtime handoff descriptor {}: {error}",
            path.display()
        )),
    }
}

fn run_agent_json(binary: &Path, session: &str, command_args: &[&str]) -> Result<Value, String> {
    run_agent_json_detailed(binary, session, command_args).map_err(|error| error.message)
}

#[derive(Debug)]
struct RuntimeTransferEvidence {
    logical_browser_id: String,
    candidate_session: String,
    receipt: crate::runtime_owner_transfer::OwnerTransferReceipt,
}

fn owner_transfer_receipt(
    payload: &Value,
) -> Result<crate::runtime_owner_transfer::OwnerTransferReceipt, String> {
    let receipt = payload
        .pointer("/data/ownerTransferReceipt")
        .cloned()
        .ok_or_else(|| "runtime_transfer_receipt_missing".to_string())?;
    serde_json::from_value(receipt)
        .map_err(|error| format!("runtime_transfer_receipt_invalid: {error}"))
}

fn commit_prepared_payload_transaction(
    paths: &InstallPaths,
    args: &WorkstationInstallArgs,
    prepared: &mut PreparedPayloadTransaction,
) -> Result<(), String> {
    crate::runtime_adoption::require_runtime_host_convergence_deadline(&prepared.transaction)?;
    commit_candidate_runtime_host_ingress(&prepared.transaction)?;
    if let Err(error) = commit_staged_payload_generation(paths, args, &prepared.staged) {
        let rollback = rollback_runtime_host_ingress(&prepared.transaction);
        return Err(rollback.err().map_or(error.clone(), |rollback| {
            format!("{error}; runtime host ingress rollback failed: {rollback}")
        }));
    }
    persist_upgrade_transition(
        &prepared.transaction_path,
        &mut prepared.transaction,
        crate::runtime_adoption::UpgradeTransactionState::GenerationCommitted,
        "generation_committed",
    )?;
    persist_admission_drain(&prepared.admission_drain_path, &prepared.transaction)
}

fn commit_candidate_runtime_host_ingress(
    transaction: &crate::runtime_adoption::UpgradeTransaction,
) -> Result<(), String> {
    if transaction
        .runtime_host_convergence
        .as_ref()
        .and_then(|convergence| convergence.candidate_host.as_ref())
        .is_none()
    {
        return Ok(());
    }
    let repository = crate::runtime_host_ingress::RuntimeHostIngressRepository::new(
        crate::runtime_host_ingress::RuntimeHostIngressRepository::default_path(),
    );
    let registry = repository.load()?;
    if registry.selected_backend().generation_id == transaction.candidate_generation_id {
        return Ok(());
    }
    if registry
        .candidate_backend()
        .is_none_or(|candidate| candidate.generation_id != transaction.candidate_generation_id)
    {
        return Err("runtime host ingress candidate changed before commit".to_string());
    }
    repository.commit_candidate(
        registry.revision,
        &transaction.transaction_id,
        &transaction.candidate_generation_id,
    )?;
    Ok(())
}

fn rollback_runtime_host_ingress(
    transaction: &crate::runtime_adoption::UpgradeTransaction,
) -> Result<(), String> {
    if transaction
        .runtime_host_convergence
        .as_ref()
        .and_then(|convergence| convergence.candidate_host.as_ref())
        .is_none()
    {
        return Ok(());
    }
    let path = crate::runtime_host_ingress::RuntimeHostIngressRepository::default_path();
    if !path.is_file() {
        return Ok(());
    }
    let repository = crate::runtime_host_ingress::RuntimeHostIngressRepository::new(path);
    let registry = repository.load()?;
    if registry.selected_backend().generation_id == transaction.candidate_generation_id
        && registry.fallback_backend().is_none()
    {
        return Err("runtime host ingress rollback backend is missing".to_string());
    }
    if registry.active_transaction_id.as_deref() == Some(&transaction.transaction_id)
        || registry.selected_backend().generation_id == transaction.candidate_generation_id
    {
        repository.rollback(
            registry.revision,
            &transaction.transaction_id,
            &transaction.candidate_generation_id,
        )?;
    }
    Ok(())
}

fn begin_post_commit_validation(prepared: &mut PreparedPayloadTransaction) -> Result<(), String> {
    persist_upgrade_transition(
        &prepared.transaction_path,
        &mut prepared.transaction,
        crate::runtime_adoption::UpgradeTransactionState::PostCommitValidating,
        "post_commit_validating",
    )?;
    persist_admission_drain(&prepared.admission_drain_path, &prepared.transaction)
}

/// Starts and stages the generation-specific shadow dashboard before payload
/// selection. The stable ingress remains bound to its prior selected backend.
fn prepare_dashboard_candidate_for_transaction(
    root: &Path,
    args: &WorkstationInstallArgs,
    prepared: &mut PreparedPayloadTransaction,
) -> Result<(), String> {
    let shadow_port = args
        .dashboard_port
        .checked_add(2)
        .ok_or_else(|| "dashboard candidate shadow port is unavailable".to_string())?;
    let candidate_binary = prepared.staged.generation_path.join("bin/agent-browser");
    let backend = crate::dashboard_ingress::DashboardBackend::new(
        prepared.transaction.candidate_generation_id.clone(),
        shadow_port,
        crate::dashboard_ingress::dashboard_runtime_manifest_sha256_for_executable(
            &candidate_binary,
        )?,
    );
    let runtime_socket_dir =
        candidate_runtime_host_socket_dir(&prepared.transaction.transaction_id)?;
    let child = candidate_dashboard_command(
        &candidate_binary,
        shadow_port,
        &prepared.transaction.candidate_generation_id,
        &runtime_socket_dir,
    )
        .spawn()
        .map_err(|error| {
            format!(
                "Unable to start candidate dashboard generation {} on shadow port {shadow_port}: {error}",
                prepared.transaction.candidate_generation_id
            )
        })?;
    let ingress_path = env::var_os("AGENT_BROWSER_DASHBOARD_INGRESS_STATE")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".agent-browser/dashboard-ingress.json"));
    prepared.dashboard_candidate = Some(PreparedDashboardCandidate {
        child: Some(child),
        backend: backend.clone(),
        ingress_path: ingress_path.clone(),
        staged_revision: 0,
    });

    wait_for_dashboard_backend(&backend, prepared, DASHBOARD_CANDIDATE_START_TIMEOUT)?;
    let repository = crate::dashboard_ingress::DashboardIngressRepository::new(&ingress_path);
    let registry = if ingress_path.is_file() {
        repository.load()?
    } else {
        repository.initialize(crate::dashboard_ingress::DashboardBackend::new(
            "bootstrap-unselected",
            args.dashboard_port.saturating_add(1),
            "unselected",
        ))?
    };
    let staged = repository.stage_candidate(registry.revision, backend)?;
    prepared
        .dashboard_candidate
        .as_mut()
        .ok_or_else(|| "candidate dashboard process custody is missing".to_string())?
        .staged_revision = staged.revision;
    Ok(())
}

fn candidate_dashboard_command(
    candidate_binary: &Path,
    shadow_port: u16,
    generation_id: &str,
    runtime_socket_dir: &Path,
) -> Command {
    let mut command = Command::new(candidate_binary);
    // The shadow dashboard owns the transaction-scoped socket directory and
    // must bootstrap its candidate runtime host before admission begins.
    // Omitting backend-only mode lets the ordinary dashboard startup path
    // create that one service lane without exposing the shadow as ingress.
    command
        .env("AGENT_BROWSER_DASHBOARD", "1")
        .env("AGENT_BROWSER_DASHBOARD_PORT", shadow_port.to_string())
        .env("AGENT_BROWSER_DASHBOARD_GENERATION", generation_id)
        .env("AGENT_BROWSER_SOCKET_DIR", runtime_socket_dir)
        .env(crate::runtime_host::RUNTIME_HOST_ENV, "1")
        .env_remove("AGENT_BROWSER_DASHBOARD_INGRESS")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn wait_for_dashboard_backend(
    backend: &crate::dashboard_ingress::DashboardBackend,
    prepared: &mut PreparedPayloadTransaction,
    timeout: std::time::Duration,
) -> Result<(), String> {
    let started = std::time::Instant::now();
    loop {
        if crate::dashboard_ingress::validate_dashboard_backend(backend).is_ok() {
            return Ok(());
        }
        let candidate = prepared
            .dashboard_candidate
            .as_mut()
            .ok_or_else(|| "candidate dashboard process custody is missing".to_string())?;
        if let Some(status) = candidate
            .child
            .as_mut()
            .and_then(|child| child.try_wait().ok().flatten())
        {
            return Err(format!(
                "candidate dashboard generation {} exited before validation with {status}",
                candidate.backend.generation_id
            ));
        }
        if started.elapsed() >= timeout {
            return Err(format!(
                "candidate dashboard generation {} did not become ready on port {}",
                backend.generation_id, backend.port
            ));
        }
        std::thread::sleep(DASHBOARD_CANDIDATE_POLL_INTERVAL);
    }
}

/// Waits after runtime transfer for an independently authenticated journey to
/// commit the staged shadow backend. The installer never fabricates evidence.
fn wait_for_dashboard_candidate_commit(
    prepared: &mut PreparedPayloadTransaction,
    timeout: std::time::Duration,
) -> Result<(), String> {
    let candidate = prepared
        .dashboard_candidate
        .as_ref()
        .ok_or_else(|| "candidate dashboard process custody is missing".to_string())?;
    let repository =
        crate::dashboard_ingress::DashboardIngressRepository::new(&candidate.ingress_path);
    let generation_id = candidate.backend.generation_id.clone();
    let port = candidate.backend.port;
    let manifest_sha256 = candidate.backend.runtime_manifest_sha256.clone();
    let staged_revision = candidate.staged_revision;
    let started = std::time::Instant::now();
    loop {
        let registry = repository.load()?;
        let committed = registry.selected_backend().generation_id == generation_id
            && registry.last_presentation_receipt().is_some_and(|receipt| {
                receipt.state == crate::runtime_adoption::PresentationState::Ready
                    && receipt.dashboard_deployment_generation == generation_id
            });
        if committed {
            return Ok(());
        }
        if registry
            .candidate_backend()
            .is_none_or(|candidate| candidate.generation_id != generation_id)
        {
            return Err("dashboard candidate selection changed before proof commit".to_string());
        }
        if let Some(status) = prepared
            .dashboard_candidate
            .as_mut()
            .and_then(|candidate| candidate.child.as_mut())
            .and_then(|child| child.try_wait().ok().flatten())
        {
            return Err(format!(
                "candidate dashboard generation {generation_id} exited before presentation commit with {status}"
            ));
        }
        if started.elapsed() >= timeout {
            return Err(format!(
                "candidate dashboard presentation was not committed within {} seconds; validate the authenticated candidate on 127.0.0.1:{port}, then run `agent-browser dashboard ingress commit --expected-revision {staged_revision} --evidence <presentation-evidence.json>`; generation={generation_id}; manifestSha256={manifest_sha256}",
                timeout.as_secs()
            ));
        }
        std::thread::sleep(DASHBOARD_CANDIDATE_POLL_INTERVAL);
    }
}

/// Moves ingress from the proven shadow process to the managed candidate unit
/// only after the latter serves the identical runtime manifest.
fn promote_dashboard_candidate_to_managed_backend(
    args: &WorkstationInstallArgs,
    prepared: &mut PreparedPayloadTransaction,
) -> Result<(), String> {
    let managed_port = args
        .dashboard_port
        .checked_add(1)
        .ok_or_else(|| "managed dashboard backend port is unavailable".to_string())?;
    let candidate = prepared
        .dashboard_candidate
        .as_ref()
        .ok_or_else(|| "candidate dashboard process custody is missing".to_string())?;
    let managed_backend = crate::dashboard_ingress::DashboardBackend::new(
        candidate.backend.generation_id.clone(),
        managed_port,
        candidate.backend.runtime_manifest_sha256.clone(),
    );
    wait_for_dashboard_backend(
        &managed_backend,
        prepared,
        DASHBOARD_CANDIDATE_START_TIMEOUT,
    )?;

    let candidate = prepared
        .dashboard_candidate
        .as_mut()
        .ok_or_else(|| "candidate dashboard process custody is missing".to_string())?;
    let repository =
        crate::dashboard_ingress::DashboardIngressRepository::new(&candidate.ingress_path);
    let registry = repository.load()?;
    let receipt = registry
        .last_presentation_receipt()
        .filter(|receipt| {
            receipt.dashboard_deployment_generation == candidate.backend.generation_id
                && receipt.state == crate::runtime_adoption::PresentationState::Ready
        })
        .ok_or_else(|| "candidate dashboard presentation receipt disappeared".to_string())?
        .clone();
    let staged = repository.stage_candidate(registry.revision, managed_backend)?;
    repository.commit_candidate(
        staged.revision,
        crate::dashboard_ingress::CandidateOperatorJourney::ready(
            crate::dashboard_ingress::PresentationEvidence::from_ready_receipt(&receipt)?,
        ),
    )?;
    stop_prepared_dashboard_candidate(prepared)
}

fn isolated_post_commit_validation(
    paths: &InstallPaths,
    prepared: &PreparedPayloadTransaction,
) -> Result<PostCommitValidationReceipt, String> {
    use crate::runtime_adoption::UpgradeTransactionState;

    if prepared.transaction.state != UpgradeTransactionState::PostCommitValidating
        || selected_generation_id(paths).as_deref()
            != Some(prepared.transaction.candidate_generation_id.as_str())
        || validate_sealed_generation_tree(&prepared.staged.generation_path).is_err()
        || !crate::runtime_adoption::upgrade_runtime_preservation_proven(&prepared.transaction)
    {
        return Err("isolated_post_commit_validation_unproven".to_string());
    }
    Ok(PostCommitValidationReceipt {
        dashboard_summary: "isolated_source_fixture_payload_validated".to_string(),
        presentation_summary: "isolated_source_fixture_has_no_live_presentations".to_string(),
    })
}

fn validate_post_commit_transaction(
    root: &Path,
    paths: &InstallPaths,
    prepared: &PreparedPayloadTransaction,
) -> Result<PostCommitValidationReceipt, String> {
    use crate::runtime_adoption::UpgradeTransactionState;

    if prepared.transaction.state != UpgradeTransactionState::PostCommitValidating {
        return Err("post_commit_transaction_not_validating".to_string());
    }
    let status = workstation_upgrade_status_for_root(root)?;
    if status
        .pointer("/latestTransaction/transactionId")
        .and_then(Value::as_str)
        != Some(prepared.transaction.transaction_id.as_str())
        || status.get("selectedGenerationId").and_then(Value::as_str)
            != Some(prepared.transaction.candidate_generation_id.as_str())
        || selected_generation_id(paths).as_deref()
            != Some(prepared.transaction.candidate_generation_id.as_str())
    {
        return Err("post_commit_selected_generation_unproven".to_string());
    }
    for axis in [
        "payloadReady",
        "selectedGenerationReady",
        "runtimeConvergenceReady",
        "dashboardIngressReady",
        "operatorJourneyReady",
        "rollbackReady",
    ] {
        if status
            .pointer(&format!("/readiness/{axis}"))
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err(format!("post_commit_readiness_axis_not_ready:{axis}"));
        }
    }
    let dashboard_generation = status
        .pointer("/dashboardIngress/selectedBackend/generationId")
        .and_then(Value::as_str);
    let receipt_generation = status
        .pointer("/dashboardIngress/presentationReceipt/dashboardDeploymentGeneration")
        .and_then(Value::as_str);
    let receipt_state = status
        .pointer("/dashboardIngress/presentationReceipt/state")
        .and_then(Value::as_str);
    let receipt_id = status
        .pointer("/dashboardIngress/presentationReceipt/receiptId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    if dashboard_generation != Some(prepared.transaction.candidate_generation_id.as_str())
        || receipt_generation != Some(prepared.transaction.candidate_generation_id.as_str())
        || receipt_state != Some("ready")
        || receipt_id.is_none()
    {
        return Err("post_commit_candidate_operator_journey_unproven".to_string());
    }
    let receipt_id = receipt_id.expect("receipt presence checked");
    Ok(PostCommitValidationReceipt {
        dashboard_summary: format!("candidate_dashboard_generation_receipted:{receipt_id}"),
        presentation_summary: format!("authenticated_operator_journey_receipted:{receipt_id}"),
    })
}

fn accept_prepared_payload_transaction(
    prepared: &mut PreparedPayloadTransaction,
    validation: PostCommitValidationReceipt,
) -> Result<(), String> {
    use crate::runtime_adoption::UpgradeTransactionState;

    if prepared.transaction.state != UpgradeTransactionState::PostCommitValidating {
        return Err("transaction_acceptance_without_post_commit_validation".to_string());
    }
    finalize_runtime_handoffs(prepared)?;
    prepared.transaction.dashboard_validation_summary = Some(validation.dashboard_summary);
    prepared.transaction.presentation_validation_summary = Some(validation.presentation_summary);
    prepared.transaction.terminal_result = Some("accepted".to_string());
    write_private_json_atomic(&prepared.transaction_path, &prepared.transaction)?;
    persist_upgrade_transition(
        &prepared.transaction_path,
        &mut prepared.transaction,
        UpgradeTransactionState::Accepted,
        "accepted",
    )?;
    if let Err(error) = clear_admission_drain(&prepared.admission_drain_path) {
        prepared.transaction.stop_reason = Some("accepted_admission_drain_not_cleared".to_string());
        persist_upgrade_transition(
            &prepared.transaction_path,
            &mut prepared.transaction,
            UpgradeTransactionState::OperatorRecoveryRequired,
            "operator_recovery_required",
        )?;
        return Err(error);
    }
    Ok(())
}

fn finalize_runtime_handoffs(prepared: &mut PreparedPayloadTransaction) -> Result<(), String> {
    if !prepared
        .runtime_handoffs
        .iter()
        .any(PreparedRuntimeHandoff::should_finalize_source)
    {
        return Ok(());
    }
    let transaction_client = prepared.staged.generation_path.join("bin/agent-browser");
    let repository = crate::runtime_host_ingress::RuntimeHostIngressRepository::new(
        crate::runtime_host_ingress::RuntimeHostIngressRepository::default_path(),
    );
    let registry = repository.load()?;
    let source_backend = registry
        .fallback_backend()
        .ok_or_else(|| "runtime source ingress backend is missing before finalize".to_string())?
        .clone();
    let source_is_runtime_host =
        source_backend.topology == crate::runtime_host_ingress::RuntimeHostTopology::SingleHost;
    for handoff in &mut prepared.runtime_handoffs {
        if handoff.should_finalize_source() {
            run_agent_json_detailed_in_socket_dir(
                &transaction_client,
                &handoff.source_session,
                &["handoff", "finalize"],
                Some((&source_backend.socket_dir, source_is_runtime_host)),
            )
            .map_err(|error| error.message)?;
            handoff.source_finalized = true;
        }
    }
    if source_is_runtime_host {
        retire_finalized_source_runtime_host(&prepared.transaction, &source_backend)?;
    }
    prove_finalized_source_exit(&prepared.transaction, &prepared.runtime_handoffs)?;
    for handoff in prepared
        .runtime_handoffs
        .iter()
        .filter(|handoff| handoff.source_finalized)
    {
        crate::runtime_adoption::finalize_runtime_lane_transfer(
            &mut prepared.transaction,
            &handoff.candidate_session,
        )?;
        clear_candidate_runtime_handoff_descriptor(
            &prepared.transaction.transaction_id,
            &handoff.source_session,
        )?;
    }
    Ok(())
}

/// A shared runtime host owns several lanes, so finalizing one lane must not
/// stop its process. Once every transferred browser lane has been finalized,
/// generation cutover retires the old host exactly once. Idle lanes are host
/// bookkeeping and do not retain process-level authority.
fn retire_finalized_source_runtime_host(
    transaction: &crate::runtime_adoption::UpgradeTransaction,
    source_backend: &crate::runtime_host_ingress::RuntimeHostBackend,
) -> Result<(), String> {
    let convergence = transaction
        .runtime_host_convergence
        .as_ref()
        .ok_or_else(|| "runtime_host_convergence_missing".to_string())?;
    let evidence = convergence
        .old_host
        .as_ref()
        .ok_or_else(|| "runtime_source_host_identity_missing".to_string())?;
    if source_backend.generation_id != evidence.generation_id
        || source_backend.binary_sha256 != evidence.binary_sha256
        || source_backend.pid != evidence.pid
        || source_backend.socket_identity != evidence.socket_identity
    {
        return Err("runtime_source_host_backend_identity_changed_before_stop".to_string());
    }
    let identity_path = source_backend.socket_dir.join("runtime-host.identity.json");
    let identity: crate::process_identity::RecordedProcessIdentity =
        serde_json::from_slice(&fs::read(&identity_path).map_err(display_io(
            "read source runtime host identity",
            &identity_path,
        ))?)
        .map_err(|error| format!("runtime_source_host_identity_invalid: {error}"))?;
    if identity.pid != evidence.pid || identity.start_token != evidence.process_start_token {
        return Err("runtime_source_host_process_identity_changed_before_stop".to_string());
    }
    if wait_for_recorded_process_exit(&identity, std::time::Duration::from_secs(5))? {
        return Ok(());
    }
    let Some(process) = crate::process_identity::VerifiedProcessTermination::open(&identity)?
    else {
        return Ok(());
    };
    process.signal(crate::process_identity::VerifiedProcessSignal::Terminate)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while process.is_running()? && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    if process.is_running()? {
        process.signal(crate::process_identity::VerifiedProcessSignal::Kill)?;
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while process.is_running()? && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    if process.is_running()? {
        return Err("runtime_source_host_exit_timeout".to_string());
    }
    Ok(())
}

fn wait_for_recorded_process_exit(
    identity: &crate::process_identity::RecordedProcessIdentity,
    timeout: std::time::Duration,
) -> Result<bool, String> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if !crate::process_identity::recorded_process_is_running(identity)? {
            return Ok(true);
        }
        if std::time::Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

fn prove_finalized_source_exit(
    transaction: &crate::runtime_adoption::UpgradeTransaction,
    handoffs: &[PreparedRuntimeHandoff],
) -> Result<(), String> {
    crate::runtime_adoption::require_runtime_host_convergence_deadline(transaction)?;
    let deadline_unix_seconds = transaction
        .runtime_host_convergence
        .as_ref()
        .map(|convergence| convergence.deadline_unix_seconds)
        .ok_or_else(|| "runtime_host_convergence_missing".to_string())?;
    let mut seen = std::collections::BTreeSet::new();
    for handoff in handoffs.iter().filter(|handoff| handoff.source_finalized) {
        let identity = handoff.source_process_identity.as_ref().ok_or_else(|| {
            format!(
                "runtime_source_process_identity_missing:{}",
                handoff.source_session
            )
        })?;
        if !seen.insert((identity.pid, identity.start_token.clone())) {
            continue;
        }
        let local_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while crate::process_identity::recorded_process_is_running(identity)? {
            if time::OffsetDateTime::now_utc().unix_timestamp() >= deadline_unix_seconds {
                return Err(format!(
                    "runtime_source_exit_deadline_expired:{}",
                    handoff.source_session
                ));
            }
            if std::time::Instant::now() >= local_deadline {
                return Err(format!(
                    "runtime_source_exit_not_observed:{}",
                    handoff.source_session
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }
    Ok(())
}

fn rollback_prepared_payload_transaction(
    paths: &InstallPaths,
    prepared: &mut PreparedPayloadTransaction,
    after_generation_commit: bool,
    stop_reason: &str,
) -> Result<(), String> {
    use crate::runtime_adoption::UpgradeTransactionState;

    prepared.transaction.stop_reason = Some(stop_reason.to_string());
    let rollback_state = if after_generation_commit {
        UpgradeTransactionState::RollbackAfterCommit
    } else {
        UpgradeTransactionState::RollbackBeforeCommit
    };
    persist_upgrade_transition(
        &prepared.transaction_path,
        &mut prepared.transaction,
        rollback_state,
        if after_generation_commit {
            "rollback_after_commit"
        } else {
            "rollback_before_commit"
        },
    )?;

    let selector_result = if after_generation_commit {
        restore_generation_selector(paths, prepared.previous_selector.as_deref())
    } else {
        Ok(())
    };
    let dashboard_result = rollback_dashboard_candidate_for_transaction(
        &paths.root,
        &prepared.transaction.candidate_generation_id,
    );
    let dashboard_process_result = stop_prepared_dashboard_candidate(prepared);
    let handoff_result = rollback_runtime_handoffs(prepared);
    let candidate_host_result = if handoff_result.is_ok() {
        stop_candidate_runtime_host(paths, &prepared.transaction)
    } else {
        Err("candidate runtime host retained because handoff rollback failed".to_string())
    };
    let runtime_host_ingress_result = rollback_runtime_host_ingress(&prepared.transaction);
    let dashboard_ingress_result = if after_generation_commit
        && selector_result.is_ok()
        && dashboard_result.is_ok()
        && dirs::home_dir().as_deref() == Some(paths.root.as_path())
    {
        let command_env = workstation_command_env(paths);
        restart_stable_dashboard_ingress(paths, &paths.support_dir, &command_env)
    } else {
        Ok(())
    };
    if selector_result.is_ok()
        && dashboard_result.is_ok()
        && dashboard_process_result.is_ok()
        && handoff_result.is_ok()
        && candidate_host_result.is_ok()
        && runtime_host_ingress_result.is_ok()
        && dashboard_ingress_result.is_ok()
    {
        prepared.transaction.terminal_result = Some("old_generation_preserved".to_string());
        clear_admission_drain(&prepared.admission_drain_path)?;
        persist_upgrade_transition(
            &prepared.transaction_path,
            &mut prepared.transaction,
            UpgradeTransactionState::FailedPreservedOldGeneration,
            "failed_preserved_old_generation",
        )
    } else {
        prepared.transaction.terminal_result = Some("operator_recovery_required".to_string());
        persist_upgrade_transition(
            &prepared.transaction_path,
            &mut prepared.transaction,
            UpgradeTransactionState::OperatorRecoveryRequired,
            "operator_recovery_required",
        )?;
        Err(format!(
            "runtime transaction rollback requires operator recovery: selector={}, dashboard={}, dashboardProcess={}, handoffs={}, candidateHost={}, runtimeHostIngress={}, dashboardIngress={}",
            selector_result
                .err()
                .unwrap_or_else(|| "restored".to_string()),
            dashboard_result
                .err()
                .unwrap_or_else(|| "restored".to_string()),
            dashboard_process_result
                .err()
                .unwrap_or_else(|| "stopped".to_string()),
            handoff_result
                .err()
                .unwrap_or_else(|| "restored".to_string()),
            candidate_host_result
                .err()
                .unwrap_or_else(|| "stopped".to_string()),
            runtime_host_ingress_result
                .err()
                .unwrap_or_else(|| "restored".to_string()),
            dashboard_ingress_result
                .err()
                .unwrap_or_else(|| "refreshed".to_string())
        ))
    }
}

fn stop_candidate_runtime_host(
    paths: &InstallPaths,
    transaction: &crate::runtime_adoption::UpgradeTransaction,
) -> Result<(), String> {
    let evidence = transaction
        .runtime_host_convergence
        .as_ref()
        .and_then(|convergence| convergence.candidate_host.as_ref());
    let socket_dir = candidate_runtime_host_socket_dir(&transaction.transaction_id)?;
    stop_candidate_runtime_host_in(paths, transaction, evidence, &socket_dir)
}

fn stop_candidate_runtime_host_in(
    paths: &InstallPaths,
    transaction: &crate::runtime_adoption::UpgradeTransaction,
    evidence: Option<&crate::runtime_adoption::RuntimeHostIdentityEvidence>,
    socket_dir: &Path,
) -> Result<(), String> {
    let identity_path = socket_dir.join("runtime-host.identity.json");
    if !identity_path.is_file() {
        return Ok(());
    }
    let identity: crate::process_identity::RecordedProcessIdentity =
        serde_json::from_slice(&fs::read(&identity_path).map_err(display_io(
            "read candidate runtime host identity",
            &identity_path,
        ))?)
        .map_err(|error| format!("candidate_runtime_host_identity_invalid: {error}"))?;
    let expected_executable = paths
        .generations_dir
        .join(&transaction.candidate_generation_id)
        .join("bin/agent-browser");
    if identity.executable_path.as_deref() != expected_executable.to_str() {
        return Err("candidate_runtime_host_executable_changed_before_stop".to_string());
    }
    if evidence.is_some_and(|evidence| {
        identity.pid != evidence.pid || identity.start_token != evidence.process_start_token
    }) {
        return Err("candidate_runtime_host_identity_changed_before_stop".to_string());
    }
    let Some(process) = crate::process_identity::VerifiedProcessTermination::open(&identity)?
    else {
        fs::remove_dir_all(socket_dir).map_err(display_io(
            "remove stopped candidate runtime host directory",
            socket_dir,
        ))?;
        return Ok(());
    };
    process.signal(crate::process_identity::VerifiedProcessSignal::Terminate)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while process.is_running()? && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    if process.is_running()? {
        process.signal(crate::process_identity::VerifiedProcessSignal::Kill)?;
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while process.is_running()? && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    if process.is_running()? {
        return Err("candidate_runtime_host_exit_timeout".to_string());
    }
    fs::remove_dir_all(socket_dir).map_err(display_io(
        "remove stopped candidate runtime host directory",
        socket_dir,
    ))?;
    Ok(())
}

fn stop_prepared_dashboard_candidate(
    prepared: &mut PreparedPayloadTransaction,
) -> Result<(), String> {
    let Some(candidate) = prepared.dashboard_candidate.as_mut() else {
        return Ok(());
    };
    let Some(mut child) = candidate.child.take() else {
        return Ok(());
    };
    match child.kill() {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => {}
        Err(error) => {
            return Err(format!(
                "Unable to stop candidate dashboard shadow backend: {error}"
            ))
        }
    }
    child
        .wait()
        .map(|_| ())
        .map_err(|error| format!("Unable to reap candidate dashboard shadow backend: {error}"))
}

fn rollback_dashboard_candidate_for_transaction(
    root: &Path,
    candidate_generation_id: &str,
) -> Result<(), String> {
    let state_path = env::var_os("AGENT_BROWSER_DASHBOARD_INGRESS_STATE")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".agent-browser/dashboard-ingress.json"));
    if !state_path.is_file() {
        return Ok(());
    }
    let repository = crate::dashboard_ingress::DashboardIngressRepository::new(&state_path);
    let registry = repository.load()?;
    if registry.selected_backend().generation_id == candidate_generation_id {
        let rolled_back =
            repository.rollback_selected_candidate(registry.revision, candidate_generation_id)?;
        if rolled_back
            .fallback_backend()
            .is_some_and(|fallback| fallback.generation_id == candidate_generation_id)
        {
            repository.retire_fallback(rolled_back.revision, candidate_generation_id)?;
        }
        Ok(())
    } else if registry
        .candidate_backend()
        .is_some_and(|candidate| candidate.generation_id == candidate_generation_id)
    {
        repository
            .rollback_candidate(registry.revision, candidate_generation_id)
            .map(|_| ())
    } else {
        Ok(())
    }
}

fn rollback_runtime_handoffs(prepared: &mut PreparedPayloadTransaction) -> Result<(), String> {
    let candidate_binary = prepared.staged.generation_path.join("bin/agent-browser");
    let mut failures = Vec::new();
    for index in (0..prepared.runtime_handoffs.len()).rev() {
        let handoff = &prepared.runtime_handoffs[index];
        let source_session = handoff.source_session.clone();
        let candidate_session = handoff.candidate_session.clone();
        let committed = handoff.committed;
        let requires_recovery = handoff.rollback_requires_operator_recovery();
        let source_finalized = handoff.source_finalized;
        let irreversible_source_revocation = handoff.irreversible_source_revocation;
        if committed {
            match run_candidate_agent_json(
                &candidate_binary,
                &candidate_session,
                &prepared.transaction.transaction_id,
                &["handoff", "rollback", "--source-session", &source_session],
            ) {
                Ok(payload) => match owner_transfer_receipt(&payload).and_then(|receipt| {
                    crate::runtime_adoption::rollback_runtime_lane_transfer(
                        &mut prepared.transaction,
                        &source_session,
                        &candidate_session,
                        receipt.previous_owner_generation,
                        receipt.candidate_owner_generation,
                        &receipt.receipt_id,
                    )
                }) {
                    Ok(()) => {}
                    Err(error) => failures.push(error),
                },
                Err(error) => failures.push(error),
            }
        }
        if let Err(error) = clear_candidate_runtime_handoff_descriptor(
            &prepared.transaction.transaction_id,
            &source_session,
        ) {
            failures.push(error);
        }
        if requires_recovery {
            if source_finalized {
                failures.push(format!(
                    "runtime_handoff_source_already_finalized:{}",
                    source_session
                ));
            }
            if irreversible_source_revocation {
                failures.push(format!(
                    "legacy_daemon_effect_authority_was_revoked:{}",
                    source_session
                ));
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

struct WorkstationLock {
    path: PathBuf,
}

impl WorkstationLock {
    fn acquire(root: &Path) -> Result<Self, String> {
        let path = root.join(".agent-browser/convergence/workstation.lock");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(display_io("create convergence lock directory", parent))?;
        }
        if path.exists() {
            let stale = fs::read_to_string(&path)
                .ok()
                .and_then(|value| value.trim().parse::<i32>().ok())
                .map(workstation_lock_pid_is_stale)
                .unwrap_or(false);
            if stale {
                fs::remove_file(&path)
                    .map_err(display_io("remove stale convergence lock", &path))?;
            } else {
                return Err(format!(
                    "workstation reconciliation is already active: {}",
                    path.display()
                ));
            }
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(display_io("acquire workstation convergence lock", &path))?;
        writeln!(file, "{}", std::process::id())
            .map_err(|error| format!("Unable to write workstation lock: {error}"))?;
        Ok(Self { path })
    }
}

#[cfg(unix)]
fn workstation_lock_pid_is_stale(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) != 0 }
}

#[cfg(not(unix))]
fn workstation_lock_pid_is_stale(_pid: i32) -> bool {
    false
}

impl Drop for WorkstationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn parse_workstation_install_args(args: &[String]) -> Result<WorkstationInstallArgs, String> {
    let mut mode = None;
    let mut json = false;
    let mut dashboard_port = DEFAULT_DASHBOARD_PORT;
    let mut guacamole_port = DEFAULT_GUACAMOLE_PORT;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "install" | "workstation" => {}
            "--dry-run" => set_mode(&mut mode, InstallMode::DryRun)?,
            "--apply" => set_mode(&mut mode, InstallMode::Apply)?,
            "--json" => json = true,
            "--dashboard-port" => {
                index += 1;
                dashboard_port = parse_port(args.get(index), "--dashboard-port")?;
            }
            "--guacamole-port" => {
                index += 1;
                guacamole_port = parse_port(args.get(index), "--guacamole-port")?;
            }
            "--help" | "-h" => {
                return Err(workstation_usage().to_string());
            }
            unknown => return Err(format!("Unknown workstation install argument: {unknown}")),
        }
        index += 1;
    }

    let mode = mode.ok_or_else(|| {
        "Choose exactly one of --dry-run or --apply for workstation installation".to_string()
    })?;
    let dashboard_backend_port = dashboard_port.checked_add(1).ok_or_else(|| {
        "--dashboard-port must leave the next TCP port available for the dashboard backend"
            .to_string()
    })?;
    let dashboard_shadow_port = dashboard_port.checked_add(2).ok_or_else(|| {
        "--dashboard-port must leave the next two TCP ports available for dashboard backends"
            .to_string()
    })?;
    if dashboard_port == guacamole_port || dashboard_backend_port == guacamole_port {
        return Err(
            "dashboard ingress, dashboard backend, and Guacamole ports must be distinct"
                .to_string(),
        );
    }
    if dashboard_shadow_port == guacamole_port {
        return Err(
            "dashboard ingress, dashboard backends, and Guacamole ports must be distinct"
                .to_string(),
        );
    }
    Ok(WorkstationInstallArgs {
        mode,
        json,
        dashboard_port,
        guacamole_port,
    })
}

fn set_mode(mode: &mut Option<InstallMode>, requested: InstallMode) -> Result<(), String> {
    if let Some(current) = mode {
        if *current != requested {
            return Err("--dry-run and --apply are mutually exclusive".to_string());
        }
        return Err("The workstation install mode may be specified only once".to_string());
    }
    *mode = Some(requested);
    Ok(())
}

fn parse_port(value: Option<&String>, flag: &str) -> Result<u16, String> {
    value
        .ok_or_else(|| format!("{flag} requires a port"))?
        .parse::<u16>()
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| format!("{flag} must be an integer from 1 through 65535"))
}

fn workstation_usage() -> &'static str {
    "Usage: agent-browser install workstation <--dry-run|--apply> [--json] [--dashboard-port <port>] [--guacamole-port <port>]\n       agent-browser install workstation status [--json]\n       agent-browser install workstation recover --transaction-id <id> [--json]\n       agent-browser install workstation finalize [--json]\n       agent-browser install workstation gc <--dry-run|--apply> [--json]"
}

#[derive(Debug)]
struct InstallPaths {
    root: PathBuf,
    binary: PathBuf,
    generations_dir: PathBuf,
    current_selector: PathBuf,
    legacy_support_dir: PathBuf,
    support_dir: PathBuf,
    unit_dir: PathBuf,
    guacamole_state_dir: PathBuf,
    guacamole_secret_file: PathBuf,
}

#[derive(Debug)]
struct StagedWorkstationGeneration {
    generation_id: String,
    generation_path: PathBuf,
    binary_sha256: String,
    support_manifest_sha256: String,
    rendered_units: Vec<(&'static str, String)>,
}

#[derive(Debug)]
struct PreparedPayloadTransaction {
    staged: StagedWorkstationGeneration,
    transaction_path: PathBuf,
    transaction: crate::runtime_adoption::UpgradeTransaction,
    previous_selector: Option<PathBuf>,
    admission_drain_path: PathBuf,
    runtime_handoffs: Vec<PreparedRuntimeHandoff>,
    dashboard_candidate: Option<PreparedDashboardCandidate>,
}

#[derive(Debug)]
struct PreparedDashboardCandidate {
    child: Option<Child>,
    backend: crate::dashboard_ingress::DashboardBackend,
    ingress_path: PathBuf,
    staged_revision: u64,
}

impl Drop for PreparedDashboardCandidate {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[derive(Debug)]
struct PreparedRuntimeHandoff {
    source_session: String,
    candidate_session: String,
    source_process_identity: Option<crate::process_identity::RecordedProcessIdentity>,
    mode: crate::runtime_adoption::BrowserAdoptionMode,
    committed: bool,
    source_finalized: bool,
    irreversible_source_revocation: bool,
}

impl PreparedRuntimeHandoff {
    fn should_finalize_source(&self) -> bool {
        self.committed
            && self.mode == crate::runtime_adoption::BrowserAdoptionMode::CooperativeTransfer
    }

    fn rollback_requires_operator_recovery(&self) -> bool {
        self.source_finalized || self.irreversible_source_revocation
    }
}

fn permitted_stale_source_sessions(
    runtime_handoffs: &[PreparedRuntimeHandoff],
    runtime_migrations: &[crate::runtime_adoption::RuntimeMigrationRecord],
) -> Vec<String> {
    let mut sessions = runtime_handoffs
        .iter()
        .filter(|handoff| handoff.committed && !handoff.source_finalized)
        .map(|handoff| handoff.source_session.clone())
        .chain(
            runtime_migrations
                .iter()
                .filter(|migration| {
                    migration.disposition
                        == crate::runtime_adoption::RuntimeDisposition::ManualPreservation
                })
                .filter_map(|migration| {
                    migration
                        .logical_browser_id
                        .strip_prefix("session:")
                        .map(str::to_string)
                }),
        )
        .collect::<Vec<_>>();
    sessions.sort();
    sessions.dedup();
    sessions
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PostCommitValidationReceipt {
    dashboard_summary: String,
    presentation_summary: String,
}

fn workstation_root() -> Result<PathBuf, String> {
    if let Some(root) = env::var_os("AGENT_BROWSER_WORKSTATION_ROOT") {
        let path = PathBuf::from(root);
        if !path.is_absolute() {
            return Err("AGENT_BROWSER_WORKSTATION_ROOT must be an absolute path".to_string());
        }
        return Ok(path);
    }
    dirs::home_dir().ok_or_else(|| "Unable to resolve the current home directory".to_string())
}

fn install_paths(root: &Path) -> InstallPaths {
    let store_dir = root.join(".local/lib/agent-browser");
    let generations_dir = store_dir.join("generations");
    let current_selector = store_dir.join("current");
    let legacy_support_dir = store_dir.join(env!("CARGO_PKG_VERSION"));
    let support_dir = if fs::symlink_metadata(&current_selector).is_ok() {
        current_selector.join("support")
    } else {
        legacy_support_dir.clone()
    };
    InstallPaths {
        root: root.to_path_buf(),
        binary: root.join(".local/bin/agent-browser"),
        generations_dir,
        current_selector,
        legacy_support_dir,
        support_dir,
        unit_dir: root.join(".config/systemd/user"),
        guacamole_state_dir: root.join(".agent-browser/guacamole"),
        guacamole_secret_file: root.join(".agent-browser/guacamole/secrets/guacamole.env"),
    }
}

fn stage_payload_generation(
    paths: &InstallPaths,
    args: &WorkstationInstallArgs,
) -> Result<StagedWorkstationGeneration, String> {
    validate_generation_install_preconditions(paths)?;
    let staging = paths
        .root
        .join(".agent-browser/install-staging")
        .join(format!(
            "{}-{}",
            env!("CARGO_PKG_VERSION"),
            uuid::Uuid::new_v4()
        ));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(display_io("clear install staging", &staging))?;
    }

    let result = (|| {
        let staged_binary = staging.join("bin/agent-browser");
        let staged_support = staging.join("support");
        let staged_units = staging.join("units");
        fs::create_dir_all(&staged_support)
            .map_err(display_io("create support staging", &staged_support))?;
        fs::create_dir_all(&staged_units)
            .map_err(display_io("create unit staging", &staged_units))?;
        if let Some(parent) = staged_binary.parent() {
            fs::create_dir_all(parent).map_err(display_io("create binary staging", parent))?;
        }

        let current_exe = env::current_exe()
            .map_err(|error| format!("Unable to resolve current executable: {error}"))?;
        fs::copy(&current_exe, &staged_binary)
            .map_err(display_io("stage agent-browser executable", &staged_binary))?;
        set_executable(&staged_binary)?;
        inject_failure("binary-staged")?;

        let final_binary = paths.binary.display().to_string();
        let selected_support_dir = paths.current_selector.join("support");
        let rendered_units = render_units(
            &final_binary,
            &selected_support_dir,
            &paths.guacamole_secret_file,
            args.dashboard_port,
        );
        let binary_sha256 = workstation_file_sha256(&staged_binary)?;
        let manifest = render_manifest(args, &binary_sha256, &rendered_units);
        fs::write(staged_support.join("manifest.json"), manifest)
            .map_err(display_io("stage workstation manifest", &staged_support))?;
        fs::write(
            staged_support.join("README.txt"),
            "Versioned agent-browser workstation support assets.\n",
        )
        .map_err(display_io("stage support readme", &staged_support))?;
        materialize_guacamole_assets(&staged_support)?;
        materialize_controller_assets(&staged_support)?;
        inject_failure("support-staged")?;

        for (name, content) in &rendered_units {
            fs::write(staged_units.join(name), content)
                .map_err(display_io("stage systemd user unit", &staged_units))?;
        }

        inject_failure("units-staged")?;

        let support_manifest_sha256 =
            workstation_file_sha256(&staged_support.join("manifest.json"))?;
        let generation_id = format!(
            "{}-{}-{}",
            env!("CARGO_PKG_VERSION"),
            &binary_sha256[..12],
            &support_manifest_sha256[..12]
        );
        let generation_path = paths.generations_dir.join(&generation_id);
        let generation_manifest = serde_json::to_string_pretty(&serde_json::json!({
            "schemaVersion": "agent-browser.runtime-generation.v1",
            "generationId": generation_id,
            "packageVersion": env!("CARGO_PKG_VERSION"),
            "binarySha256": binary_sha256.clone(),
            "supportManifestSha256": support_manifest_sha256.clone(),
            "controllerCompatibilityVersion": 1,
            "schemaCompatibilityVersion": 1,
            "immutableInstallationPath": generation_path,
        }))
        .expect("runtime generation manifest must serialize");
        fs::write(staging.join("generation.json"), generation_manifest)
            .map_err(display_io("stage runtime generation manifest", &staging))?;
        preflight_staged_generation(
            &staging,
            &binary_sha256,
            &support_manifest_sha256,
            &rendered_units,
        )?;
        inject_failure("generation-preflight-ready")?;

        commit_immutable_generation(&staging, &generation_path)?;
        inject_failure("generation-staged")?;
        Ok(StagedWorkstationGeneration {
            generation_id,
            generation_path,
            binary_sha256,
            support_manifest_sha256,
            rendered_units,
        })
    })();
    let _ = remove_generation_tree(&staging);
    result
}

fn commit_staged_payload_generation(
    paths: &InstallPaths,
    args: &WorkstationInstallArgs,
    staged: &StagedWorkstationGeneration,
) -> Result<(), String> {
    validate_sealed_generation_tree(&staged.generation_path)?;
    ensure_workstation_state(paths, args)?;
    let created_links = prepare_stable_generation_links(paths, &staged.rendered_units)?;
    if let Err(error) = select_generation(paths, &staged.generation_id) {
        remove_created_links(&created_links);
        return Err(error);
    }
    Ok(())
}

fn legacy_mutable_payload_present(paths: &InstallPaths) -> bool {
    let legacy_units = WORKSTATION_RECONCILE_QUIESCE_UNITS
        .iter()
        .any(|unit| paths.unit_dir.join(unit).exists());
    paths.binary.exists() || paths.legacy_support_dir.exists() || legacy_units
}

fn migrate_legacy_payload_to_generation(paths: &InstallPaths) -> Result<String, String> {
    if fs::symlink_metadata(&paths.current_selector).is_ok() {
        return selected_generation_id(paths)
            .ok_or_else(|| "current runtime generation selector is invalid".to_string());
    }
    let required = [
        paths.binary.clone(),
        paths.legacy_support_dir.join("manifest.json"),
    ];
    let missing = required
        .iter()
        .filter(|path| !path.is_file())
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "legacy workstation payload is incomplete; missing: {}",
            missing.join(", ")
        ));
    }

    let staging = paths
        .root
        .join(".agent-browser/install-staging")
        .join(format!("legacy-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        let staged_binary = staging.join("bin/agent-browser");
        let staged_support = staging.join("support");
        let staged_units = staging.join("units");
        fs::create_dir_all(staged_binary.parent().expect("staged binary parent"))
            .map_err(display_io("create legacy binary staging", &staging))?;
        fs::create_dir_all(&staged_units)
            .map_err(display_io("create legacy unit staging", &staged_units))?;
        fs::copy(&paths.binary, &staged_binary).map_err(display_io(
            "stage legacy agent-browser executable",
            &staged_binary,
        ))?;
        set_executable(&staged_binary)?;
        copy_legacy_generation_tree(&paths.legacy_support_dir, &staged_support)?;

        let mut units = Vec::with_capacity(WORKSTATION_GENERATION_UNITS.len());
        for unit in WORKSTATION_GENERATION_UNITS {
            let source = paths.unit_dir.join(unit);
            let content = match fs::read_to_string(&source) {
                Ok(content) => content,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    inactive_legacy_generation_unit(unit)
                }
                Err(error) => {
                    return Err(format!(
                        "Unable to read legacy systemd user unit {}: {error}",
                        source.display()
                    ));
                }
            };
            fs::write(staged_units.join(unit), &content)
                .map_err(display_io("stage legacy systemd user unit", &staged_units))?;
            units.push((unit, content));
        }

        let binary_sha256 = workstation_file_sha256(&staged_binary)?;
        let support_manifest_sha256 =
            workstation_file_sha256(&staged_support.join("manifest.json"))?;
        let generation_id = format!(
            "{}-{}-{}",
            env!("CARGO_PKG_VERSION"),
            &binary_sha256[..12],
            &support_manifest_sha256[..12]
        );
        let generation_path = paths.generations_dir.join(&generation_id);
        let generation_manifest = serde_json::to_string_pretty(&serde_json::json!({
            "schemaVersion": "agent-browser.runtime-generation.v1",
            "generationId": generation_id,
            "packageVersion": env!("CARGO_PKG_VERSION"),
            "binarySha256": binary_sha256,
            "supportManifestSha256": support_manifest_sha256,
            "controllerCompatibilityVersion": 1,
            "schemaCompatibilityVersion": 1,
            "immutableInstallationPath": generation_path,
            "importedFromLegacyPayload": true,
        }))
        .expect("legacy runtime generation manifest must serialize");
        fs::write(staging.join("generation.json"), generation_manifest).map_err(display_io(
            "stage legacy runtime generation manifest",
            &staging,
        ))?;
        preflight_staged_generation(&staging, &binary_sha256, &support_manifest_sha256, &units)?;
        commit_immutable_generation(&staging, &generation_path)?;
        select_generation(paths, &generation_id)?;
        replace_legacy_payload_with_stable_links(paths, &units)?;
        reconcile_relocated_legacy_daemon_identities(paths, &binary_sha256)?;
        Ok(generation_id)
    })();
    let _ = remove_generation_tree(&staging);
    result
}

fn reconcile_selected_legacy_daemon_identities(paths: &InstallPaths) -> Result<(), String> {
    let Some(generation_id) = selected_generation_id(paths) else {
        return Ok(());
    };
    let generation = paths.generations_dir.join(generation_id);
    let manifest_path = generation.join("generation.json");
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).map_err(display_io(
        "read selected runtime generation manifest",
        &manifest_path,
    ))?)
    .map_err(|error| {
        format!(
            "Unable to parse selected runtime generation manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    if manifest
        .get("importedFromLegacyPayload")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Ok(());
    }
    let binary_sha256 = manifest
        .get("binarySha256")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "selected imported legacy generation manifest lacks binarySha256: {}",
                manifest_path.display()
            )
        })?;
    reconcile_relocated_legacy_daemon_identities(paths, binary_sha256)
}

#[cfg(target_os = "linux")]
fn reconcile_relocated_legacy_daemon_identities(
    paths: &InstallPaths,
    imported_binary_sha256: &str,
) -> Result<(), String> {
    use crate::process_identity::ProcessObservation;

    let socket_dir = crate::connection::get_socket_dir();
    let entries = match fs::read_dir(&socket_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Unable to read daemon identity directory {}: {error}",
                socket_dir.display()
            ));
        }
    };
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "Unable to read daemon identity entry in {}: {error}",
                socket_dir.display()
            )
        })?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        let Some(session) = file_name.strip_suffix(".identity.json") else {
            continue;
        };
        let recorded = match crate::connection::load_daemon_process_identity(session) {
            Ok(recorded) => recorded,
            Err(_) => continue,
        };
        if recorded.executable_path.as_deref() != paths.binary.to_str() {
            continue;
        }
        let observed = match crate::process_identity::observe_process(recorded.pid) {
            ProcessObservation::Observed(observed) => observed,
            ProcessObservation::Missing | ProcessObservation::Failed { .. } => continue,
        };
        if !relocated_legacy_daemon_observation_matches(paths, &recorded, &observed) {
            continue;
        }
        let proc_executable = PathBuf::from(format!("/proc/{}/exe", recorded.pid));
        let observed_binary_sha256 = workstation_file_sha256(&proc_executable)?;
        let Some(reconciled) = reconciled_legacy_daemon_identity(
            paths,
            &recorded,
            &observed,
            imported_binary_sha256,
            &observed_binary_sha256,
        ) else {
            continue;
        };
        let verified = crate::process_identity::VerifiedProcessTermination::open(&reconciled)?;
        if verified.is_none() {
            continue;
        }
        crate::connection::write_daemon_process_identity(session, &reconciled)?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn reconcile_relocated_legacy_daemon_identities(
    _paths: &InstallPaths,
    _imported_binary_sha256: &str,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn reconciled_legacy_daemon_identity(
    paths: &InstallPaths,
    recorded: &crate::process_identity::RecordedProcessIdentity,
    observed: &crate::process_identity::ObservedProcessIdentity,
    imported_binary_sha256: &str,
    observed_binary_sha256: &str,
) -> Option<crate::process_identity::RecordedProcessIdentity> {
    if !relocated_legacy_daemon_observation_matches(paths, recorded, observed)
        || imported_binary_sha256 != observed_binary_sha256
    {
        return None;
    }
    let mut reconciled = recorded.clone();
    reconciled.executable_path = observed.executable_path.clone();
    Some(reconciled)
}

#[cfg(target_os = "linux")]
fn relocated_legacy_daemon_observation_matches(
    paths: &InstallPaths,
    recorded: &crate::process_identity::RecordedProcessIdentity,
    observed: &crate::process_identity::ObservedProcessIdentity,
) -> bool {
    let Some(observed_executable) = observed.executable_path.as_deref() else {
        return false;
    };
    let Some(legacy_parent) = paths.binary.parent() else {
        return false;
    };
    let Some(binary_name) = paths.binary.file_name() else {
        return false;
    };
    let legacy_prefix = format!(".{}.legacy-", binary_name.to_string_lossy());
    let relocated = observed_executable
        .strip_suffix(" (deleted)")
        .unwrap_or(observed_executable);
    let relocated = Path::new(relocated);
    recorded.pid == observed.pid
        && observed.start_token.as_deref() == Some(recorded.start_token.as_str())
        && recorded.executable_path.as_deref() == paths.binary.to_str()
        && relocated.parent() == Some(legacy_parent)
        && relocated
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(&legacy_prefix))
}

fn inactive_legacy_generation_unit(unit: &str) -> String {
    format!(
        "[Unit]\nDescription=Inactive placeholder for {unit} in imported legacy generation\nConditionPathExists=/dev/null/agent-browser-legacy-unit-unavailable\n\n[Service]\nType=oneshot\nExecStart=/usr/bin/false\n"
    )
}

fn copy_legacy_generation_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(display_io("inspect legacy workstation payload", source))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "legacy workstation payload contains an unsupported symlink: {}",
            source.display()
        ));
    }
    if metadata.is_dir() {
        fs::create_dir_all(destination).map_err(display_io(
            "create legacy generation directory",
            destination,
        ))?;
        for entry in
            fs::read_dir(source).map_err(display_io("read legacy workstation payload", source))?
        {
            let entry =
                entry.map_err(|error| format!("Unable to read legacy payload entry: {error}"))?;
            copy_legacy_generation_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if metadata.is_file() {
        fs::copy(source, destination)
            .map_err(display_io("copy legacy workstation payload", destination))?;
        return Ok(());
    }
    Err(format!(
        "legacy workstation payload contains an unsupported entry: {}",
        source.display()
    ))
}

fn replace_legacy_payload_with_stable_links(
    paths: &InstallPaths,
    units: &[(&'static str, String)],
) -> Result<(), String> {
    let mut replacements = vec![(
        paths.binary.clone(),
        PathBuf::from("../lib/agent-browser/current/bin/agent-browser"),
    )];
    replacements.extend(units.iter().map(|(name, _)| {
        (
            paths.unit_dir.join(name),
            PathBuf::from("../../../.local/lib/agent-browser/current/units").join(name),
        )
    }));
    let mut backups = Vec::<(PathBuf, Option<PathBuf>)>::new();
    for (path, target) in replacements {
        let backup = path.with_file_name(format!(
            ".{}.legacy-{}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("payload"),
            uuid::Uuid::new_v4()
        ));
        let backup = match fs::rename(&path, &backup) {
            Ok(()) => Some(backup),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                rollback_legacy_link_replacements(paths, &backups);
                return Err(format!(
                    "Unable to preserve legacy payload entry {}: {error}",
                    path.display()
                ));
            }
        };
        if let Err(error) = atomic_symlink(&target, &path) {
            if let Some(backup) = backup.as_ref() {
                let _ = fs::rename(backup, &path);
            }
            rollback_legacy_link_replacements(paths, &backups);
            return Err(error);
        }
        backups.push((path, backup));
    }
    for (_, backup) in backups {
        if let Some(backup) = backup {
            let _ = fs::remove_file(backup);
        }
    }
    Ok(())
}

fn rollback_legacy_link_replacements(paths: &InstallPaths, backups: &[(PathBuf, Option<PathBuf>)]) {
    for (path, backup) in backups.iter().rev() {
        let _ = fs::remove_file(path);
        if let Some(backup) = backup {
            let _ = fs::rename(backup, path);
        }
    }
    let _ = restore_generation_selector(paths, None);
}

fn validate_generation_install_preconditions(paths: &InstallPaths) -> Result<(), String> {
    match fs::symlink_metadata(&paths.current_selector) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let selected_target = fs::read_link(&paths.current_selector).map_err(display_io(
                "read current runtime generation selector",
                &paths.current_selector,
            ))?;
            let generation_id = selected_target
                .strip_prefix("generations")
                .ok()
                .filter(|relative| relative.components().count() == 1)
                .ok_or_else(|| {
                    format!(
                        "current runtime generation selector has an invalid target: {}",
                        selected_target.display()
                    )
                })?;
            let selected = paths.generations_dir.join(generation_id);
            for required in [
                selected.join("generation.json"),
                selected.join("bin/agent-browser"),
                selected.join("support/manifest.json"),
            ] {
                if !required.is_file() {
                    return Err(format!(
                        "current runtime generation is incomplete; missing {}",
                        required.display()
                    ));
                }
            }
            for unit in WORKSTATION_GENERATION_UNITS {
                let required = selected.join("units").join(unit);
                if !required.is_file() {
                    return Err(format!(
                        "current runtime generation is incomplete; missing {}",
                        required.display()
                    ));
                }
            }
            validate_sealed_generation_tree(&selected)?;
        }
        Ok(_) => {
            return Err(format!(
                "current runtime generation selector is not a symlink: {}",
                paths.current_selector.display()
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if legacy_mutable_payload_present(paths) {
                return Err(
                    "legacy mutable workstation payload requires transactional generation migration before apply"
                        .to_string(),
                );
            }
        }
        Err(error) => {
            return Err(format!(
                "Unable to inspect current runtime generation selector {}: {error}",
                paths.current_selector.display()
            ));
        }
    }
    Ok(())
}

fn preflight_staged_generation(
    staging: &Path,
    expected_binary_sha256: &str,
    expected_support_manifest_sha256: &str,
    units: &[(&'static str, String)],
) -> Result<(), String> {
    let binary = staging.join("bin/agent-browser");
    let support_manifest = staging.join("support/manifest.json");
    if workstation_file_sha256(&binary)? != expected_binary_sha256 {
        return Err("staged runtime generation binary hash mismatch".to_string());
    }
    if workstation_file_sha256(&support_manifest)? != expected_support_manifest_sha256 {
        return Err("staged runtime generation support manifest hash mismatch".to_string());
    }
    for (name, expected) in units {
        let path = staging.join("units").join(name);
        let actual = fs::read_to_string(&path)
            .map_err(display_io("read staged runtime generation unit", &path))?;
        if actual != *expected {
            return Err(format!(
                "staged runtime generation unit does not match rendered content: {name}"
            ));
        }
    }
    if !staging.join("generation.json").is_file() {
        return Err("staged runtime generation manifest is missing".to_string());
    }
    Ok(())
}

fn commit_immutable_generation(staging: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        validate_sealed_generation_tree(destination)?;
        if !directory_tree_contents_match(staging, destination)? {
            return Err(format!(
                "immutable runtime generation already exists with different content: {}",
                destination.display()
            ));
        }
        remove_generation_tree(staging)?;
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(display_io("create runtime generation store", parent))?;
    }
    fs::rename(staging, destination).map_err(display_io(
        "commit immutable runtime generation",
        destination,
    ))?;
    if let Err(error) = seal_generation_tree(destination) {
        let cleanup = remove_generation_tree(destination);
        return match cleanup {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(format!(
                "{error}; additionally failed to remove the unsealed runtime generation: {cleanup_error}"
            )),
        };
    }
    Ok(())
}

fn seal_generation_tree(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(display_io("inspect staged runtime generation entry", path))?;
    if metadata.is_dir() {
        for entry in fs::read_dir(path)
            .map_err(display_io("read staged runtime generation directory", path))?
        {
            let entry = entry
                .map_err(|error| format!("Unable to read staged generation entry: {error}"))?;
            seal_generation_tree(&entry.path())?;
        }
    }
    if !metadata.file_type().is_symlink() {
        let mut permissions = metadata.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(permissions.mode() & !0o222);
        }
        #[cfg(not(unix))]
        permissions.set_readonly(true);
        fs::set_permissions(path, permissions)
            .map_err(display_io("seal immutable runtime generation entry", path))?;
    }
    Ok(())
}

fn validate_sealed_generation_tree(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(display_io(
        "inspect immutable runtime generation entry",
        path,
    ))?;
    #[cfg(unix)]
    let writable = {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o222 != 0
    };
    #[cfg(not(unix))]
    let writable = !metadata.permissions().readonly();
    if !metadata.file_type().is_symlink() && writable {
        return Err(format!(
            "immutable runtime generation entry is writable: {}",
            path.display()
        ));
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(display_io(
            "read immutable runtime generation directory",
            path,
        ))? {
            let entry = entry
                .map_err(|error| format!("Unable to read runtime generation entry: {error}"))?;
            validate_sealed_generation_tree(&entry.path())?;
        }
    }
    Ok(())
}

fn remove_generation_tree(path: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Unable to inspect staged runtime generation {}: {error}",
                path.display()
            ));
        }
    };
    if metadata.is_dir() {
        let mut permissions = metadata.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(permissions.mode() | 0o700);
        }
        #[cfg(not(unix))]
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions).map_err(display_io(
            "unseal staged runtime generation directory",
            path,
        ))?;
        for entry in fs::read_dir(path).map_err(display_io(
            "read staged runtime generation for cleanup",
            path,
        ))? {
            let entry = entry
                .map_err(|error| format!("Unable to read staged generation entry: {error}"))?;
            if entry
                .file_type()
                .map_err(|error| format!("Unable to inspect staged generation entry: {error}"))?
                .is_dir()
            {
                remove_generation_tree(&entry.path())?;
            }
        }
        fs::remove_dir_all(path).map_err(display_io("remove staged runtime generation", path))
    } else {
        fs::remove_file(path).map_err(display_io("remove staged runtime generation file", path))
    }
}

fn prepare_stable_generation_links(
    paths: &InstallPaths,
    units: &[(&'static str, String)],
) -> Result<Vec<PathBuf>, String> {
    let mut created = Vec::new();
    let binary_target = PathBuf::from("../lib/agent-browser/current/bin/agent-browser");
    match ensure_stable_symlink(&paths.binary, &binary_target) {
        Ok(true) => created.push(paths.binary.clone()),
        Ok(false) => {}
        Err(error) => return Err(error),
    }
    for (name, _) in units {
        let link = paths.unit_dir.join(name);
        let target = PathBuf::from("../../../.local/lib/agent-browser/current/units").join(name);
        match ensure_stable_symlink(&link, &target) {
            Ok(true) => created.push(link),
            Ok(false) => {}
            Err(error) => {
                remove_created_links(&created);
                return Err(error);
            }
        }
    }
    Ok(created)
}

fn ensure_stable_symlink(link: &Path, target: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(link) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let existing = fs::read_link(link)
                .map_err(display_io("read stable runtime generation link", link))?;
            if existing == target || stable_symlink_targets_equivalent(link, &existing, target) {
                return Ok(false);
            }
            Err(format!(
                "stable runtime generation link has unexpected target {}: {}",
                link.display(),
                existing.display()
            ))
        }
        Ok(_) => Err(format!(
            "stable runtime generation link is occupied by mutable payload: {}",
            link.display()
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            atomic_symlink(target, link)?;
            Ok(true)
        }
        Err(error) => Err(format!(
            "Unable to inspect stable runtime generation link {}: {error}",
            link.display()
        )),
    }
}

fn stable_symlink_targets_equivalent(link: &Path, left: &Path, right: &Path) -> bool {
    let Some(parent) = link.parent() else {
        return false;
    };
    let resolve = |target: &Path| {
        let path = if target.is_absolute() {
            target.to_path_buf()
        } else {
            parent.join(target)
        };
        lexically_normalized_absolute_path(&path)
    };
    resolve(left).is_some_and(|left| resolve(right).as_ref() == Some(&left))
}

fn lexically_normalized_absolute_path(path: &Path) -> Option<PathBuf> {
    use std::path::Component;

    if !path.is_absolute() {
        return None;
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::Normal(value) => normalized.push(value),
        }
    }
    Some(normalized)
}

fn atomic_symlink(target: &Path, destination: &Path) -> Result<(), String> {
    let parent = destination.parent().ok_or_else(|| {
        format!(
            "runtime generation link has no parent: {}",
            destination.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(display_io(
        "create runtime generation link directory",
        parent,
    ))?;
    let temporary = parent.join(format!(
        ".{}.{}-tmp",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("generation-link"),
        uuid::Uuid::new_v4()
    ));
    create_generation_symlink(target, &temporary).map_err(display_io(
        "stage atomic runtime generation link",
        &temporary,
    ))?;
    if let Err(error) = fs::rename(&temporary, destination) {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "Unable to commit atomic runtime generation link {}: {error}",
            destination.display()
        ));
    }
    Ok(())
}

fn select_generation(paths: &InstallPaths, generation_id: &str) -> Result<(), String> {
    let destination = paths.generations_dir.join(generation_id);
    if !destination.is_dir() {
        return Err(format!(
            "cannot select missing runtime generation: {}",
            destination.display()
        ));
    }
    let previous = match fs::symlink_metadata(&paths.current_selector) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Some(fs::read_link(&paths.current_selector).map_err(display_io(
                "read previous runtime generation selector",
                &paths.current_selector,
            ))?)
        }
        Ok(_) => {
            return Err(format!(
                "current runtime generation selector is not a symlink: {}",
                paths.current_selector.display()
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "Unable to inspect previous runtime generation selector {}: {error}",
                paths.current_selector.display()
            ));
        }
    };
    let selected_target = PathBuf::from("generations").join(generation_id);
    let parent = paths
        .current_selector
        .parent()
        .expect("current generation selector always has a parent");
    fs::create_dir_all(parent).map_err(display_io(
        "create runtime generation selector directory",
        parent,
    ))?;
    let temporary = parent.join(format!(".current.{}-tmp", uuid::Uuid::new_v4()));
    create_generation_symlink(&selected_target, &temporary).map_err(display_io(
        "stage current runtime generation selector",
        &temporary,
    ))?;
    if let Err(error) = inject_failure("selector-staged") {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    fs::rename(&temporary, &paths.current_selector).map_err(display_io(
        "commit current runtime generation selector",
        &paths.current_selector,
    ))?;
    if let Err(error) = inject_failure("selector-committed") {
        restore_generation_selector(paths, previous.as_deref())?;
        return Err(error);
    }
    Ok(())
}

#[cfg(unix)]
fn create_generation_symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn create_generation_symlink(_target: &Path, _link: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "workstation runtime generation links are only supported on Unix",
    ))
}

fn restore_generation_selector(
    paths: &InstallPaths,
    previous: Option<&Path>,
) -> Result<(), String> {
    if let Some(previous) = previous {
        atomic_symlink(previous, &paths.current_selector)
    } else {
        match fs::remove_file(&paths.current_selector) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!(
                "Unable to roll back current runtime generation selector {}: {error}",
                paths.current_selector.display()
            )),
        }
    }
}

fn remove_created_links(paths: &[PathBuf]) {
    for path in paths.iter().rev() {
        let _ = fs::remove_file(path);
    }
}

fn materialize_guacamole_assets(staged_support: &Path) -> Result<(), String> {
    let guacamole_dir = staged_support.join("guacamole");
    let init_dir = guacamole_dir.join("init");
    fs::create_dir_all(&init_dir)
        .map_err(display_io("create Guacamole asset staging", &init_dir))?;
    let assets = [
        ("compose.yml", GUACAMOLE_COMPOSE, false),
        ("environment.example", GUACAMOLE_ENVIRONMENT_EXAMPLE, false),
        ("generate-initdb.sh", GUACAMOLE_SCHEMA_GENERATOR, true),
        ("start-guacamole.sh", GUACAMOLE_START_WRAPPER, true),
        ("manifest.json", GUACAMOLE_BUNDLE_MANIFEST, false),
        ("init/001-initdb.sql", GUACAMOLE_INITDB, false),
        (
            "extensions/guac-manifest.json",
            GUACAMOLE_DEFAULTS_EXTENSION_MANIFEST,
            false,
        ),
        (
            "extensions/agent-browser-defaults.js",
            GUACAMOLE_DEFAULTS_EXTENSION_SCRIPT,
            false,
        ),
    ];
    for (relative, content, executable) in assets {
        let destination = guacamole_dir.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(display_io("create Guacamole asset staging", parent))?;
        }
        fs::write(&destination, content)
            .map_err(display_io("stage Guacamole support asset", &destination))?;
        if executable {
            set_executable(&destination)?;
        }
    }
    materialize_guacamole_defaults_extension(&guacamole_dir)?;
    Ok(())
}

/// Packages the browser-local Guacamole defaults migration as a standard
/// extension JAR. Guacamole loads the JavaScript before Angular bootstraps,
/// allowing the migration to set text input without modifying the pinned
/// upstream image.
fn materialize_guacamole_defaults_extension(guacamole_dir: &Path) -> Result<(), String> {
    let extension_dir = guacamole_dir.join("extensions");
    fs::create_dir_all(&extension_dir).map_err(display_io(
        "create Guacamole extension staging",
        &extension_dir,
    ))?;

    let cursor = io::Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, content) in [
        ("guac-manifest.json", GUACAMOLE_DEFAULTS_EXTENSION_MANIFEST),
        (
            "agent-browser-defaults.js",
            GUACAMOLE_DEFAULTS_EXTENSION_SCRIPT,
        ),
    ] {
        writer.start_file(name, options).map_err(|error| {
            format!("Unable to start Guacamole extension entry {name}: {error}")
        })?;
        writer.write_all(content.as_bytes()).map_err(|error| {
            format!("Unable to write Guacamole extension entry {name}: {error}")
        })?;
    }
    let archive = writer
        .finish()
        .map_err(|error| format!("Unable to finish Guacamole defaults extension: {error}"))?
        .into_inner();
    let destination = extension_dir.join("agent-browser-defaults.jar");
    fs::write(&destination, archive).map_err(display_io(
        "stage Guacamole defaults extension",
        &destination,
    ))
}

fn materialize_controller_assets(staged_support: &Path) -> Result<(), String> {
    for (relative, content, executable) in CONTROLLER_ASSETS {
        let destination = staged_support.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(display_io("create controller asset staging", parent))?;
        }
        fs::write(&destination, content).map_err(display_io(
            "stage workstation controller asset",
            &destination,
        ))?;
        if executable {
            set_executable(&destination)?;
        }
    }
    Ok(())
}

fn ensure_workstation_state(
    paths: &InstallPaths,
    args: &WorkstationInstallArgs,
) -> Result<(), String> {
    let secrets_dir = paths.guacamole_state_dir.join("secrets");
    for directory in [
        &paths.guacamole_state_dir,
        &secrets_dir,
        &paths.guacamole_state_dir.join("state"),
        &paths.guacamole_state_dir.join("backups"),
    ] {
        fs::create_dir_all(directory)
            .map_err(display_io("create Guacamole state directory", directory))?;
        set_private_directory(directory)?;
    }

    let environment_file = paths.guacamole_state_dir.join(".env");
    fs::write(
        &environment_file,
        format!(
            "AGENT_BROWSER_GUACAMOLE_HTTP_PORT={}\n",
            args.guacamole_port
        ),
    )
    .map_err(display_io(
        "write Guacamole listener environment",
        &environment_file,
    ))?;

    ensure_secret_values(&paths.guacamole_secret_file)?;
    set_private_file(&paths.guacamole_secret_file)?;
    Ok(())
}

fn ensure_secret_values(secret_file: &Path) -> Result<(), String> {
    let mut contents = fs::read_to_string(secret_file).unwrap_or_default();
    let required = [
        (
            "POSTGRES_PASSWORD",
            format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ),
        ),
        (
            "XRDP_AGENT_BROWSER_ROUTE_A_USERNAME",
            "agent-browser-rdp-a".to_string(),
        ),
        (
            "XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD",
            format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ),
        ),
        (
            "XRDP_AGENT_BROWSER_ROUTE_B_USERNAME",
            "agent-browser-rdp-b".to_string(),
        ),
        (
            "XRDP_AGENT_BROWSER_ROUTE_B_PASSWORD",
            format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ),
        ),
    ];
    for (key, value) in required {
        let present = contents.lines().any(|line| {
            line.split_once('=')
                .map(|(candidate, _)| candidate.trim() == key)
                .unwrap_or(false)
        });
        if !present {
            if !contents.is_empty() && !contents.ends_with('\n') {
                contents.push('\n');
            }
            contents.push_str(&format!("{key}={value}\n"));
        }
    }
    fs::write(secret_file, contents)
        .map_err(display_io("write protected Guacamole secrets", secret_file))
}

fn render_manifest(
    args: &WorkstationInstallArgs,
    binary_sha256: &str,
    units: &[(&'static str, String)],
) -> String {
    let guacamole_bundle: serde_json::Value = serde_json::from_str(GUACAMOLE_BUNDLE_MANIFEST)
        .expect("embedded Guacamole bundle manifest must be valid JSON");
    let controller_assets = CONTROLLER_ASSETS
        .iter()
        .map(|(path, content, _)| {
            serde_json::json!({
                "path": path,
                "sha256": workstation_bytes_sha256(content.as_bytes()),
            })
        })
        .collect::<Vec<_>>();
    let unit_assets = units
        .iter()
        .map(|(name, content)| {
            serde_json::json!({
                "name": name,
                "sha256": workstation_bytes_sha256(content.as_bytes()),
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&serde_json::json!({
        "schemaVersion": "agent-browser.workstation-payload.v1",
        "version": env!("CARGO_PKG_VERSION"),
        "dashboardPort": args.dashboard_port,
        "guacamolePort": args.guacamole_port,
        "runtimeController": "installed-binary",
        "sourceCheckoutRequired": false,
        "binary": {
            "sha256": binary_sha256,
        },
        "controllerAssets": {
            "files": controller_assets,
        },
        "units": unit_assets,
        "guacamoleBundleManifestSha256": workstation_bytes_sha256(
            GUACAMOLE_BUNDLE_MANIFEST.as_bytes()
        ),
        "guacamoleBundle": guacamole_bundle,
    }))
    .expect("static workstation manifest must serialize")
}

fn workstation_bytes_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    hex::encode(Sha256::digest(bytes))
}

fn workstation_file_sha256(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};

    let mut file = fs::File::open(path).map_err(display_io("open file for hashing", path))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(display_io("read file for hashing", path))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn render_units(
    binary: &str,
    support_dir: &Path,
    guacamole_secret_file: &Path,
    dashboard_port: u16,
) -> Vec<(&'static str, String)> {
    let script_root = support_dir.join("scripts");
    let guacamole_dir = support_dir.join("guacamole");
    let dashboard_backend_port = dashboard_port
        .checked_add(1)
        .expect("validated workstation dashboard port must reserve a backend port");
    let runtime_environment = format!(
        "EnvironmentFile=-%h/.agent-browser/.env\nEnvironment=AGENT_BROWSER_BIN={binary}\nEnvironment=AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT={}\nEnvironment=AGENT_BROWSER_GUACAMOLE_DIR={}\nEnvironment=AGENT_BROWSER_GUACAMOLE_SECRET_FILE={}\n",
        script_root.display(),
        guacamole_dir.display(),
        guacamole_secret_file.display()
    );
    vec![
        (
            "agent-browser-dashboard-backend.service",
            format!(
                "[Unit]\nDescription=agent-browser dashboard generation backend\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nEnvironmentFile=-%h/.agent-browser/.env\nEnvironment=AGENT_BROWSER_DASHBOARD=1\nEnvironment=AGENT_BROWSER_DASHBOARD_BACKEND_ONLY=1\nEnvironment=AGENT_BROWSER_DASHBOARD_PORT={dashboard_backend_port}\nExecStart={binary}\nRestart=on-failure\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n"
            ),
        ),
        (
            "agent-browser-dashboard.service",
            format!(
                "[Unit]\nDescription=agent-browser stable dashboard ingress\nAfter=agent-browser-dashboard-backend.service network-online.target\nWants=agent-browser-dashboard-backend.service network-online.target\n\n[Service]\nType=simple\nEnvironmentFile=-%h/.agent-browser/.env\nEnvironment=AGENT_BROWSER_DASHBOARD_INGRESS=1\nEnvironment=AGENT_BROWSER_DASHBOARD_PORT={dashboard_port}\nEnvironment=AGENT_BROWSER_DASHBOARD_BACKEND_PORT={dashboard_backend_port}\nExecStart={binary}\nRestart=on-failure\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n"
            ),
        ),
        (
            "agent-browser-runtime-interlock.service",
            format!(
                "[Unit]\nDescription=agent-browser runtime health interlock\nAfter=agent-browser-dashboard.service network-online.target\nWants=agent-browser-dashboard.service network-online.target\n\n[Service]\nType=oneshot\n{runtime_environment}ExecStart={binary} install workstation reconcile --json\nTimeoutStartSec=15min\n"
            ),
        ),
        (
            "agent-browser-runtime-interlock.timer",
            "[Unit]\nDescription=Periodically reconcile agent-browser runtime health\n\n[Timer]\nOnActiveSec=5min\nOnUnitInactiveSec=5min\nAccuracySec=5s\nPersistent=true\nUnit=agent-browser-runtime-interlock.service\n\n[Install]\nWantedBy=timers.target\n".to_string(),
        ),
        (
            "agent-browser-guacamole-postgres-backup.service",
            format!(
                "[Unit]\nDescription=Back up agent-browser Guacamole PostgreSQL\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=oneshot\n{runtime_environment}ExecStart={binary} install workstation backup --json\nTimeoutStartSec=10min\n"
            ),
        ),
        (
            "agent-browser-guacamole-postgres-backup.timer",
            "[Unit]\nDescription=Daily agent-browser Guacamole PostgreSQL backup\n\n[Timer]\nOnCalendar=daily\nRandomizedDelaySec=15min\nPersistent=true\nUnit=agent-browser-guacamole-postgres-backup.service\n\n[Install]\nWantedBy=timers.target\n".to_string(),
        ),
    ]
}

fn inject_failure(phase: &str) -> Result<(), String> {
    if env::var("AGENT_BROWSER_WORKSTATION_FAIL_AFTER").as_deref() == Ok(phase) {
        return Err(format!(
            "Injected workstation install failure after {phase}"
        ));
    }
    Ok(())
}

fn directory_tree_contents_match(left: &Path, right: &Path) -> Result<bool, String> {
    let left_metadata =
        fs::symlink_metadata(left).map_err(display_io("inspect staged directory", left))?;
    let right_metadata =
        fs::symlink_metadata(right).map_err(display_io("inspect installed directory", right))?;
    if !left_metadata.is_dir() || !right_metadata.is_dir() {
        return Ok(false);
    }

    let mut left_entries = directory_entry_names(left)?;
    let mut right_entries = directory_entry_names(right)?;
    left_entries.sort();
    right_entries.sort();
    if left_entries != right_entries {
        return Ok(false);
    }

    for name in left_entries {
        let left_entry = left.join(&name);
        let right_entry = right.join(&name);
        let left_type = fs::symlink_metadata(&left_entry)
            .map_err(display_io("inspect staged support entry", &left_entry))?
            .file_type();
        let right_type = fs::symlink_metadata(&right_entry)
            .map_err(display_io("inspect installed support entry", &right_entry))?
            .file_type();
        if left_type.is_dir() && right_type.is_dir() {
            if !directory_tree_contents_match(&left_entry, &right_entry)? {
                return Ok(false);
            }
        } else if left_type.is_file() && right_type.is_file() {
            if !file_contents_match(&left_entry, &right_entry)? {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }
    Ok(true)
}

fn directory_entry_names(path: &Path) -> Result<Vec<std::ffi::OsString>, String> {
    fs::read_dir(path)
        .map_err(display_io("read workstation directory", path))?
        .map(|entry| {
            entry
                .map(|entry| entry.file_name())
                .map_err(|error| format!("Unable to read entry in {}: {error}", path.display()))
        })
        .collect()
}

fn file_contents_match(left: &Path, right: &Path) -> Result<bool, String> {
    let left_metadata =
        fs::symlink_metadata(left).map_err(display_io("inspect staged file", left))?;
    let right_metadata =
        fs::symlink_metadata(right).map_err(display_io("inspect installed file", right))?;
    if !left_metadata.is_file()
        || !right_metadata.is_file()
        || left_metadata.len() != right_metadata.len()
    {
        return Ok(false);
    }

    let mut left_file = fs::File::open(left).map_err(display_io("open staged file", left))?;
    let mut right_file = fs::File::open(right).map_err(display_io("open installed file", right))?;
    let mut left_buffer = [0_u8; 64 * 1024];
    let mut right_buffer = [0_u8; 64 * 1024];
    loop {
        let left_count = left_file
            .read(&mut left_buffer)
            .map_err(display_io("read staged file", left))?;
        let right_count = right_file
            .read(&mut right_buffer)
            .map_err(display_io("read installed file", right))?;
        if left_count != right_count || left_buffer[..left_count] != right_buffer[..right_count] {
            return Ok(false);
        }
        if left_count == 0 {
            return Ok(true);
        }
    }
}

fn display_io<'a>(action: &'static str, path: &'a Path) -> impl FnOnce(io::Error) -> String + 'a {
    move |error| format!("Unable to {action} {}: {error}", path.display())
}

fn set_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .map_err(display_io("set executable permissions on", path))?;
    }
    Ok(())
}

fn set_private_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(display_io("set private directory permissions on", path))?;
    }
    Ok(())
}

fn set_private_file(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(display_io("set private file permissions on", path))?;
    }
    Ok(())
}

fn fail(message: &str, json: bool) -> ! {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "success": false,
                "error": message,
            }))
            .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        );
    } else {
        eprintln!("{message}");
    }
    exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;

    #[test]
    fn shared_runtime_host_idle_lanes_do_not_retire_the_host_process() {
        let root = env::temp_dir().join(format!(
            "agent-browser-shared-idle-lane-retirement-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let guard = EnvGuard::new(&[
            crate::runtime_host::RUNTIME_HOST_ENV,
            "AGENT_BROWSER_SOCKET_DIR",
        ]);
        guard.set(crate::runtime_host::RUNTIME_HOST_ENV, "1");
        guard.set("AGENT_BROWSER_SOCKET_DIR", root.to_str().unwrap());

        assert_eq!(
            retire_idle_runtime("shared-idle-alpha").unwrap(),
            IdleRuntimeRetirement::SharedLaneDeferred
        );
        assert_eq!(
            retire_idle_runtime("shared-idle-beta").unwrap(),
            IdleRuntimeRetirement::SharedLaneDeferred
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn finalized_shared_runtime_host_is_retired_once_by_exact_identity() {
        let root = env::temp_dir().join(format!(
            "agent-browser-finalized-shared-host-retirement-{}",
            uuid::Uuid::new_v4()
        ));
        let socket_dir = root.join("runtime-host");
        fs::create_dir_all(&socket_dir).unwrap();
        let mut child = Command::new("sleep").arg("60").spawn().unwrap();
        let identity = crate::process_identity::capture_process_identity(child.id(), None, None)
            .expect("capture fixture process identity");
        write_private_json_atomic(&socket_dir.join("runtime-host.identity.json"), &identity)
            .unwrap();

        let generation_id = "generation-old";
        let binary_sha256 = "a".repeat(64);
        let socket_identity = "unix:fixture:old-host";
        let backend = crate::runtime_host_ingress::RuntimeHostBackend {
            topology: crate::runtime_host_ingress::RuntimeHostTopology::SingleHost,
            generation_id: generation_id.to_string(),
            socket_dir: socket_dir.clone(),
            binary_sha256: binary_sha256.clone(),
            host_id: "runtime-host:fixture".to_string(),
            pid: identity.pid,
            socket_identity: socket_identity.to_string(),
        };
        let paths = install_paths(&root);
        let mut transaction = new_upgrade_transaction(
            &paths,
            "generation-candidate".to_string(),
            "b".repeat(64),
            "c".repeat(64),
        );
        transaction.runtime_host_convergence =
            Some(crate::runtime_adoption::RuntimeHostConvergenceRecord {
                schema_version: "agent-browser.runtime-host-convergence.v1".to_string(),
                deadline_at: runtime_adoption_timestamp(),
                deadline_unix_seconds: time::OffsetDateTime::now_utc().unix_timestamp() + 30,
                queue_transfer_policy: "drain_then_commit".to_string(),
                old_host: Some(crate::runtime_adoption::RuntimeHostIdentityEvidence {
                    endpoint_key: crate::runtime_host::RUNTIME_HOST_ENDPOINT_KEY.to_string(),
                    generation_id: generation_id.to_string(),
                    binary_sha256,
                    pid: identity.pid,
                    process_start_token: identity.start_token.clone(),
                    socket_identity: socket_identity.to_string(),
                    observation_only: false,
                }),
                candidate_host: None,
                lanes: Vec::new(),
            });

        retire_finalized_source_runtime_host(&transaction, &backend).unwrap();
        assert!(child.wait().unwrap().code().is_none());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn runtime_monitor_treats_upgrade_owned_lock_contention_as_a_skip() {
        let root = env::temp_dir().join(format!(
            "agent-browser-runtime-monitor-upgrade-lock-{}",
            uuid::Uuid::new_v4()
        ));
        let drain = root.join(".agent-browser/runtime-adoption/admission-drain.json");
        fs::create_dir_all(drain.parent().unwrap()).unwrap();
        fs::write(&drain, b"{}").unwrap();

        assert!(runtime_monitor_blocked_by_active_upgrade(
            &root,
            "workstation reconciliation is already active: fixture"
        ));
        assert!(!runtime_monitor_blocked_by_active_upgrade(
            &root,
            "unattended_process_gc_failed:fixture"
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn runtime_monitor_backoff_is_bounded_and_exponential() {
        assert_eq!(runtime_monitor_backoff_seconds(1), 300);
        assert_eq!(runtime_monitor_backoff_seconds(2), 600);
        assert_eq!(runtime_monitor_backoff_seconds(3), 1200);
        assert_eq!(runtime_monitor_backoff_seconds(4), 2400);
        assert_eq!(runtime_monitor_backoff_seconds(99), 2400);
    }

    #[test]
    fn latest_transaction_uses_planned_time_instead_of_mutable_file_time() {
        let root = env::temp_dir().join(format!(
            "agent-browser-transaction-ordering-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        let transaction_dir = root.join(".agent-browser/runtime-adoption/transactions");
        fs::create_dir_all(&transaction_dir).unwrap();

        let mut newer = new_upgrade_transaction(
            &paths,
            "candidate-new".to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        newer.transaction_id = "upgrade-new".to_string();
        newer.checkpoints[0].recorded_at = "2026-08-21T16:00:00Z".to_string();
        fs::write(
            transaction_dir.join("upgrade-new.json"),
            serde_json::to_vec_pretty(&newer).unwrap(),
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));

        let mut older = new_upgrade_transaction(
            &paths,
            "candidate-old".to_string(),
            "c".repeat(64),
            "d".repeat(64),
        );
        older.transaction_id = "upgrade-old".to_string();
        older.checkpoints[0].recorded_at = "2026-08-17T16:00:00Z".to_string();
        fs::write(
            transaction_dir.join("upgrade-old.json"),
            serde_json::to_vec_pretty(&older).unwrap(),
        )
        .unwrap();

        let latest = latest_upgrade_transaction(&transaction_dir)
            .unwrap()
            .unwrap();
        assert_eq!(latest.transaction_id, "upgrade-new");

        fs::remove_dir_all(root).unwrap();
    }

    fn runtime_migration(
        logical_browser_id: &str,
    ) -> crate::runtime_adoption::RuntimeMigrationRecord {
        crate::runtime_adoption::RuntimeMigrationRecord {
            logical_browser_id: logical_browser_id.to_string(),
            session_names: Vec::new(),
            profile_identity_digest: "profile-digest".to_string(),
            classification: crate::runtime_adoption::RuntimeClassification::ExternalObserved,
            disposition: crate::runtime_adoption::RuntimeDisposition::ManualPreservation,
            adoption_receipt_id: None,
            reason_codes: vec!["external_owner_preserved".to_string()],
        }
    }

    #[test]
    fn orphan_adoption_fences_a_non_orphaned_registry_owner_first() {
        use crate::runtime_owner_transfer::{ProfileOwner, ProfileOwnerState};

        let mut migration = runtime_migration("session:browser-a");
        migration.profile_identity_digest = "profile-a".to_string();
        let mut state = crate::native::service_model::ServiceState::default();
        let mut owner = ProfileOwner {
            owner_id: "owner-a".to_string(),
            profile_identity_digest: migration.profile_identity_digest.clone(),
            state: ProfileOwnerState::Ready,
            owner_generation: 4,
            browser_id: migration.logical_browser_id.clone(),
            daemon_session_route: "session-a".to_string(),
            process_instance_digest: "process-a".to_string(),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: "cdp-a".to_string(),
            target_set_digest: "targets-a".to_string(),
            pending_transfer: None,
            last_transition: None,
        };
        state
            .runtime_owner_registry
            .owners
            .insert(migration.profile_identity_digest.clone(), owner.clone());

        assert!(runtime_orphan_owner_requires_fencing(&state, &migration));
        owner.state = ProfileOwnerState::Orphaned;
        state
            .runtime_owner_registry
            .owners
            .insert(migration.profile_identity_digest.clone(), owner);
        assert!(!runtime_orphan_owner_requires_fencing(&state, &migration));
        state.runtime_owner_registry.owners.clear();
        assert!(!runtime_orphan_owner_requires_fencing(&state, &migration));
    }

    #[test]
    fn transferred_owner_route_supersedes_legacy_logical_session_alias() {
        let logical_browser_id = "session:p116-alpha";
        let candidate_session = "handoff-candidate";
        let mut service_state = crate::native::service_model::ServiceState::default();
        service_state.runtime_owner_registry =
            crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
                crate::runtime_owner_transfer::ProfileOwner {
                    owner_id: "owner-candidate".to_string(),
                    profile_identity_digest: "profile-digest".to_string(),
                    state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
                    owner_generation: 2,
                    browser_id: logical_browser_id.to_string(),
                    daemon_session_route: candidate_session.to_string(),
                    process_instance_digest: "process-digest".to_string(),
                    browser_family: "chrome".to_string(),
                    cdp_endpoint_identity_digest: "cdp-digest".to_string(),
                    target_set_digest: "target-digest".to_string(),
                    pending_transfer: None,
                    last_transition: None,
                },
            );
        service_state.browsers.insert(
            logical_browser_id.to_string(),
            crate::native::service_model::BrowserProcess {
                active_session_ids: vec![candidate_session.to_string()],
                ..Default::default()
            },
        );

        let source =
            resolve_runtime_source_session(&service_state, &runtime_migration(logical_browser_id))
                .unwrap();

        assert_eq!(source.as_deref(), Some(candidate_session));
    }

    #[test]
    fn transferred_owner_route_still_rejects_conflicting_active_session() {
        let logical_browser_id = "session:p116-alpha";
        let mut service_state = crate::native::service_model::ServiceState::default();
        service_state.runtime_owner_registry =
            crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
                crate::runtime_owner_transfer::ProfileOwner {
                    owner_id: "owner-candidate".to_string(),
                    profile_identity_digest: "profile-digest".to_string(),
                    state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
                    owner_generation: 2,
                    browser_id: logical_browser_id.to_string(),
                    daemon_session_route: "handoff-candidate".to_string(),
                    process_instance_digest: "process-digest".to_string(),
                    browser_family: "chrome".to_string(),
                    cdp_endpoint_identity_digest: "cdp-digest".to_string(),
                    target_set_digest: "target-digest".to_string(),
                    pending_transfer: None,
                    last_transition: None,
                },
            );
        service_state.browsers.insert(
            logical_browser_id.to_string(),
            crate::native::service_model::BrowserProcess {
                active_session_ids: vec!["different-session".to_string()],
                ..Default::default()
            },
        );

        let error =
            resolve_runtime_source_session(&service_state, &runtime_migration(logical_browser_id))
                .unwrap_err();

        assert_eq!(
            error,
            "runtime_transfer_source_session_ambiguous:session:p116-alpha"
        );
    }

    #[test]
    fn orphan_owner_placeholder_route_defers_to_the_bound_browser_session() {
        let logical_browser_id = "session:p116-alpha";
        let mut service_state = crate::native::service_model::ServiceState::default();
        service_state.runtime_owner_registry =
            crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
                crate::runtime_owner_transfer::ProfileOwner {
                    owner_id: "owner-orphan".to_string(),
                    profile_identity_digest: "profile-digest".to_string(),
                    state: crate::runtime_owner_transfer::ProfileOwnerState::Orphaned,
                    owner_generation: 2,
                    browser_id: logical_browser_id.to_string(),
                    daemon_session_route: "orphan-observation".to_string(),
                    process_instance_digest: "process-digest".to_string(),
                    browser_family: "chrome".to_string(),
                    cdp_endpoint_identity_digest: "cdp-digest".to_string(),
                    target_set_digest: "target-digest".to_string(),
                    pending_transfer: None,
                    last_transition: None,
                },
            );
        service_state.browsers.insert(
            logical_browser_id.to_string(),
            crate::native::service_model::BrowserProcess {
                active_session_ids: vec!["orphan-verified".to_string()],
                ..Default::default()
            },
        );

        let source = resolve_runtime_source_session_with_probe(
            &service_state,
            &runtime_migration(logical_browser_id),
            |_| false,
        )
        .unwrap();

        assert_eq!(source.as_deref(), Some("orphan-verified"));
    }

    #[test]
    fn rebound_logical_session_is_not_selected_as_an_orphan_source() {
        let logical_browser_id = "session:p116-alpha";
        let mut service_state = crate::native::service_model::ServiceState::default();
        service_state.sessions.insert(
            "p116-alpha".to_string(),
            crate::native::service_model::BrowserSession {
                id: "p116-alpha".to_string(),
                browser_ids: vec!["browser-current".to_string()],
                ..Default::default()
            },
        );

        let source = resolve_runtime_source_session_with_probe(
            &service_state,
            &runtime_migration(logical_browser_id),
            |_| true,
        )
        .unwrap();

        assert_eq!(source, None);
    }

    #[test]
    fn observed_idle_daemon_uses_its_census_bound_session() {
        let service_state = crate::native::service_model::ServiceState::default();
        let mut migration = runtime_migration("observed-idle-daemon");
        migration.session_names = vec!["idle-source".to_string()];

        let source =
            resolve_runtime_source_session_with_probe(&service_state, &migration, |_| true)
                .unwrap();

        assert_eq!(source.as_deref(), Some("idle-source"));
    }

    #[test]
    fn browser_without_a_live_cooperative_source_is_preserved() {
        let mut migration = runtime_migration("session:preserved-browser");
        migration.classification =
            crate::runtime_adoption::RuntimeClassification::CooperativeLiveOwner;
        migration.disposition = crate::runtime_adoption::RuntimeDisposition::CooperativeTransfer;

        preserve_runtime_without_live_source(&mut migration);

        assert_eq!(
            migration.classification,
            crate::runtime_adoption::RuntimeClassification::ManualPreserveOnly
        );
        assert_eq!(
            migration.disposition,
            crate::runtime_adoption::RuntimeDisposition::ManualPreservation
        );
        assert!(migration
            .reason_codes
            .contains(&"verified_browser_without_live_source_session_preserved".to_string()));
    }

    #[test]
    fn live_owner_route_supersedes_a_retained_inactive_browser_alias() {
        let logical_browser_id = "session:p116-alpha";
        let mut service_state = crate::native::service_model::ServiceState::default();
        service_state.runtime_owner_registry =
            crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
                crate::runtime_owner_transfer::ProfileOwner {
                    owner_id: "owner-current".to_string(),
                    profile_identity_digest: "profile-digest".to_string(),
                    state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
                    owner_generation: 8,
                    browser_id: logical_browser_id.to_string(),
                    daemon_session_route: "p116-alpha-recovery".to_string(),
                    process_instance_digest: "process-digest".to_string(),
                    browser_family: "chrome".to_string(),
                    cdp_endpoint_identity_digest: "cdp-digest".to_string(),
                    target_set_digest: "target-digest".to_string(),
                    pending_transfer: None,
                    last_transition: None,
                },
            );
        service_state.browsers.insert(
            logical_browser_id.to_string(),
            crate::native::service_model::BrowserProcess {
                active_session_ids: vec!["retained-candidate-alias".to_string()],
                ..Default::default()
            },
        );

        let source = resolve_runtime_source_session_with_probe(
            &service_state,
            &runtime_migration(logical_browser_id),
            |session| session == "p116-alpha-recovery",
        )
        .unwrap();

        assert_eq!(source.as_deref(), Some("p116-alpha-recovery"));
    }

    #[test]
    fn orphan_candidate_sessions_are_scoped_to_the_upgrade_transaction() {
        let migration = runtime_migration("session:p116-alpha");
        let first = orphan_candidate_session(&migration, "upgrade-first");

        assert_eq!(first, orphan_candidate_session(&migration, "upgrade-first"));
        assert_ne!(first, orphan_candidate_session(&migration, "upgrade-retry"));
        assert_ne!(
            first,
            orphan_candidate_session(&runtime_migration("session:p116-beta"), "upgrade-first")
        );
        assert!(first.starts_with("orphan-"));
        assert_eq!(first.len(), "orphan-".len() + 16);
    }

    #[test]
    fn candidate_runtime_host_socket_path_is_bounded_for_orphan_adoption() {
        let runtime_root = Path::new("/run/user/1000/agent-browser/runtime-hosts");
        let transaction_id = "upgrade-719d8fd6-057f-4526-a7c7-1031b36ac46b";
        let socket_dir = candidate_runtime_host_socket_dir_in(runtime_root, transaction_id);
        let orphan_session = orphan_candidate_session(
            &runtime_migration("session:last30days-x-upgrade-live-20260820"),
            transaction_id,
        );
        let socket_path = socket_dir.join(format!("{orphan_session}.sock"));

        assert_eq!(
            socket_dir,
            candidate_runtime_host_socket_dir_in(runtime_root, transaction_id)
        );
        assert_ne!(
            socket_dir,
            candidate_runtime_host_socket_dir_in(runtime_root, "upgrade-retry")
        );
        assert!(socket_path.as_os_str().len() <= 103, "{socket_path:?}");
    }

    #[test]
    fn candidate_identity_uses_the_synchronous_manifest_while_sha_sidecar_is_pending() {
        let fixture = env::temp_dir().join(format!(
            "agent-browser-candidate-identity-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&fixture).unwrap();
        let binary_sha256 = workstation_bytes_sha256(b"candidate-binary");
        fs::write(
            fixture.join("runtime-host.json"),
            serde_json::to_vec(&crate::runtime_host::RuntimeHostManifest {
                schema_version: "agent-browser.runtime-host.v1".to_string(),
                host_id: "runtime-host:4100".to_string(),
                pid: 4100,
                executable_generation: binary_sha256.clone(),
                socket_identity: "unix:fixture-runtime-host".to_string(),
                authentication_record: "runtime-host.token".to_string(),
                max_lanes: crate::runtime_host::DEFAULT_MAX_RUNTIME_LANES,
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(
            fixture.join("runtime-host.identity.json"),
            serde_json::to_vec(&crate::process_identity::RecordedProcessIdentity {
                pid: 4100,
                start_token: "linux:boot:4100".to_string(),
                executable_path: Some("/opt/agent-browser".to_string()),
                browser_family: None,
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(fixture.join("runtime-host.sha256"), "pending").unwrap();

        let (identity, backend) =
            capture_runtime_host_identity(&fixture, "candidate-generation", &binary_sha256, true)
                .expect("the synchronous manifest is the executable identity authority");

        assert_eq!(identity.binary_sha256, binary_sha256);
        assert_eq!(backend.pid, 4100);
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn cooperative_handoff_descriptor_is_staged_for_the_candidate_host() {
        let fixture = env::temp_dir().join(format!(
            "agent-browser-candidate-handoff-stage-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let source_dir = fixture.join("selected-host");
        let candidate_dir = fixture.join("candidate-host");
        fs::create_dir_all(&source_dir).unwrap();
        let source_session = "source-owner";
        let candidate_session = "handoff-candidate";
        let source_path = source_dir.join(format!("{source_session}.handoff.json"));
        fs::write(
            &source_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schemaVersion": 2,
                "sessionName": source_session,
                "ownerTransfer": {
                    "request": {
                        "candidateDaemonSessionRoute": candidate_session
                    }
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let prepared = serde_json::json!({
            "data": {
                "handoffPath": source_path,
                "candidateSessionName": candidate_session
            }
        });

        let staged = stage_candidate_runtime_handoff_descriptor(
            &source_dir,
            &candidate_dir,
            source_session,
            &prepared,
        )
        .expect("candidate host must receive the exact prepared descriptor");

        assert_eq!(
            staged,
            candidate_dir.join(format!("{source_session}.handoff.json"))
        );
        let staged_descriptor: Value = serde_json::from_slice(&fs::read(staged).unwrap()).unwrap();
        assert_eq!(
            staged_descriptor.get("sessionName").and_then(Value::as_str),
            Some(source_session)
        );
        let mismatched_candidate = serde_json::json!({
            "data": {
                "handoffPath": source_path,
                "candidateSessionName": "different-candidate"
            }
        });
        assert!(stage_candidate_runtime_handoff_descriptor(
            &source_dir,
            &candidate_dir,
            source_session,
            &mismatched_candidate,
        )
        .is_err());
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn selected_single_host_is_captured_before_runtime_transfer() {
        assert!(selected_runtime_host_capture_required(
            crate::runtime_host_ingress::RuntimeHostTopology::SingleHost,
            false,
            false,
        ));
        assert!(!selected_runtime_host_capture_required(
            crate::runtime_host_ingress::RuntimeHostTopology::SingleHost,
            false,
            true,
        ));
        assert!(!selected_runtime_host_capture_required(
            crate::runtime_host_ingress::RuntimeHostTopology::LegacyPerSession,
            false,
            false,
        ));
        assert!(!selected_runtime_host_capture_required(
            crate::runtime_host_ingress::RuntimeHostTopology::SingleHost,
            true,
            false,
        ));
        assert_eq!(
            selected_runtime_host_capture_action(
                crate::runtime_host_ingress::RuntimeHostTopology::SingleHost,
                false,
                false,
            ),
            SelectedRuntimeHostCaptureAction::CaptureExact,
        );
        assert_eq!(
            selected_runtime_host_capture_action(
                crate::runtime_host_ingress::RuntimeHostTopology::SingleHost,
                true,
                false,
            ),
            SelectedRuntimeHostCaptureAction::RefreshIdentity,
        );
        assert_eq!(
            selected_runtime_host_capture_action(
                crate::runtime_host_ingress::RuntimeHostTopology::SingleHost,
                true,
                true,
            ),
            SelectedRuntimeHostCaptureAction::Skip,
        );
    }

    #[test]
    fn real_host_stages_candidate_runtime_host_even_without_transferable_lanes() {
        assert!(candidate_runtime_host_stage_required(false, 0));
        assert!(candidate_runtime_host_stage_required(false, 3));
        assert!(!candidate_runtime_host_stage_required(true, 0));
    }

    #[cfg(unix)]
    #[test]
    fn rollback_stops_candidate_host_started_before_identity_capture() {
        let root = env::temp_dir().join(format!(
            "agent-browser-candidate-host-rollback-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        let candidate_generation_id = "generation-candidate";
        let executable = paths
            .generations_dir
            .join(candidate_generation_id)
            .join("bin/agent-browser");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::copy("/bin/sleep", &executable).unwrap();
        let mut child = Command::new(&executable).arg("30").spawn().unwrap();
        let identity =
            crate::process_identity::capture_process_identity(child.id(), Some(&executable), None)
                .unwrap();
        let transaction = new_upgrade_transaction(
            &paths,
            candidate_generation_id.to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        assert!(transaction
            .runtime_host_convergence
            .as_ref()
            .unwrap()
            .candidate_host
            .is_none());
        let socket_dir = root.join("candidate-host-socket");
        fs::create_dir_all(&socket_dir).unwrap();
        fs::write(
            socket_dir.join("runtime-host.identity.json"),
            serde_json::to_vec(&identity).unwrap(),
        )
        .unwrap();

        stop_candidate_runtime_host_in(&paths, &transaction, None, &socket_dir).unwrap();

        assert!(!socket_dir.exists());
        assert!(!crate::process_identity::process_exists(child.id()));
        let _ = child.wait();
        remove_generation_tree(
            &paths
                .generations_dir
                .join(transaction.candidate_generation_id),
        )
        .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn prior_transaction_orphan_alias_remains_bound_to_its_exact_browser() {
        let logical_browser_id = "session:p116-alpha";
        let prior_candidate = "orphan-prior-transaction";
        let mut service_state = crate::native::service_model::ServiceState::default();
        service_state.sessions.insert(
            prior_candidate.to_string(),
            crate::native::service_model::BrowserSession {
                id: prior_candidate.to_string(),
                browser_ids: vec![logical_browser_id.to_string()],
                ..Default::default()
            },
        );

        assert!(runtime_source_session_is_bound(
            &service_state,
            logical_browser_id,
            prior_candidate
        ));
        assert!(!runtime_source_session_is_bound(
            &service_state,
            "session:p116-beta",
            prior_candidate
        ));
    }

    #[test]
    fn ready_durable_handoff_binds_a_recovered_orphan_session() {
        use crate::native::service_model::{
            DurableHandoffPresentationReceipt, RemoteViewHandoff, ViewStreamProvider,
        };
        use crate::runtime_owner_transfer::{ProfileOwner, ProfileOwnerState};

        let logical_browser_id = "session:p117-recovered";
        let source_session = "orphan-recovered-session";
        let process_digest = "2".repeat(64);
        let profile_digest = "1".repeat(64);
        let mut service_state = crate::native::service_model::ServiceState::default();
        service_state.runtime_owner_registry.owners.insert(
            profile_digest.clone(),
            ProfileOwner {
                owner_id: "owner-recovered".to_string(),
                profile_identity_digest: profile_digest,
                state: ProfileOwnerState::Ready,
                owner_generation: 9,
                browser_id: logical_browser_id.to_string(),
                daemon_session_route: source_session.to_string(),
                process_instance_digest: process_digest.clone(),
                browser_family: "chrome".to_string(),
                cdp_endpoint_identity_digest: "3".repeat(64),
                target_set_digest: "4".repeat(64),
                pending_transfer: None,
                last_transition: None,
            },
        );
        service_state.remote_view_handoffs.insert(
            "r-recovered".to_string(),
            RemoteViewHandoff {
                id: "r-recovered".to_string(),
                state: "ready".to_string(),
                browser_id: Some(logical_browser_id.to_string()),
                session_name: Some(source_session.to_string()),
                presentation_receipt: Some(DurableHandoffPresentationReceipt {
                    schema_version: "agent-browser.durable-handoff-presentation.v1".to_string(),
                    generation: 3,
                    dashboard_deployment_generation: "generation-a".to_string(),
                    logical_browser_id: logical_browser_id.to_string(),
                    daemon_owner_generation: Some(9),
                    process_instance_digest: Some(process_digest),
                    target_id: "target-a".to_string(),
                    required_stream_provider: ViewStreamProvider::RdpGateway,
                    observed_stream_provider: ViewStreamProvider::RdpGateway,
                    route_id: "route-a".to_string(),
                    display_allocation_id: "display-a".to_string(),
                    observed_at: "2026-08-21T00:00:00Z".to_string(),
                    state: "ready".to_string(),
                }),
                ..Default::default()
            },
        );

        assert!(runtime_source_session_is_bound(
            &service_state,
            logical_browser_id,
            source_session
        ));
        service_state
            .remote_view_handoffs
            .get_mut("r-recovered")
            .unwrap()
            .presentation_receipt
            .as_mut()
            .unwrap()
            .daemon_owner_generation = Some(10);
        assert!(!runtime_source_session_is_bound(
            &service_state,
            logical_browser_id,
            source_session
        ));
    }

    #[test]
    fn parses_explicit_dry_run() {
        let args = vec![
            "install".to_string(),
            "workstation".to_string(),
            "--dry-run".to_string(),
            "--json".to_string(),
            "--dashboard-port".to_string(),
            "4949".to_string(),
        ];
        let parsed = parse_workstation_install_args(&args).unwrap();
        assert_eq!(parsed.mode, InstallMode::DryRun);
        assert!(parsed.json);
        assert_eq!(parsed.dashboard_port, 4949);
        assert_eq!(parsed.guacamole_port, DEFAULT_GUACAMOLE_PORT);
    }

    #[test]
    fn recovery_requires_one_exact_transaction_id() {
        let args = vec![
            "install".to_string(),
            "workstation".to_string(),
            "recover".to_string(),
            "--transaction-id".to_string(),
            "upgrade-fixture-42".to_string(),
            "--json".to_string(),
        ];
        assert_eq!(
            parse_recovery_transaction_id(&args).unwrap(),
            "upgrade-fixture-42"
        );
        assert!(parse_recovery_transaction_id(&args[..3])
            .unwrap_err()
            .contains("requires --transaction-id"));

        let mut traversal = args;
        traversal[4] = "upgrade-../../state".to_string();
        assert!(parse_recovery_transaction_id(&traversal).is_err());
    }

    #[test]
    fn requires_exactly_one_mode() {
        let missing = vec!["install".to_string(), "workstation".to_string()];
        assert!(parse_workstation_install_args(&missing)
            .unwrap_err()
            .contains("Choose exactly one"));

        let conflicting = vec![
            "install".to_string(),
            "workstation".to_string(),
            "--dry-run".to_string(),
            "--apply".to_string(),
        ];
        assert!(parse_workstation_install_args(&conflicting)
            .unwrap_err()
            .contains("mutually exclusive"));
    }

    #[test]
    fn candidate_dashboard_timeout_covers_service_lane_recovery() {
        assert!(DASHBOARD_CANDIDATE_START_TIMEOUT >= std::time::Duration::from_secs(30));
    }

    #[test]
    fn dashboard_port_must_reserve_a_distinct_backend_port() {
        let exhausted = vec![
            "install".to_string(),
            "workstation".to_string(),
            "--dry-run".to_string(),
            "--dashboard-port".to_string(),
            u16::MAX.to_string(),
        ];
        assert!(parse_workstation_install_args(&exhausted)
            .unwrap_err()
            .contains("next TCP port"));

        let shadow_exhausted = vec![
            "install".to_string(),
            "workstation".to_string(),
            "--dry-run".to_string(),
            "--dashboard-port".to_string(),
            (u16::MAX - 1).to_string(),
        ];
        assert!(parse_workstation_install_args(&shadow_exhausted)
            .unwrap_err()
            .contains("next two TCP ports"));

        let collision = vec![
            "install".to_string(),
            "workstation".to_string(),
            "--dry-run".to_string(),
            "--dashboard-port".to_string(),
            (DEFAULT_GUACAMOLE_PORT - 1).to_string(),
        ];
        assert!(parse_workstation_install_args(&collision)
            .unwrap_err()
            .contains("must be distinct"));

        let shadow_collision = vec![
            "install".to_string(),
            "workstation".to_string(),
            "--dry-run".to_string(),
            "--dashboard-port".to_string(),
            (DEFAULT_GUACAMOLE_PORT - 2).to_string(),
        ];
        assert!(parse_workstation_install_args(&shadow_collision)
            .unwrap_err()
            .contains("must be distinct"));
    }

    #[test]
    fn installed_units_are_source_free() {
        let units = render_units(
            "/home/test/.local/bin/agent-browser",
            Path::new("/home/test/.local/lib/agent-browser/0.28.0"),
            Path::new("/home/test/.agent-browser/guacamole/secrets/guacamole.env"),
            4848,
        );
        for (name, body) in units {
            assert!(!body.contains("pnpm"));
            assert!(!body.contains("WorkingDirectory="));
            assert!(!body.contains("workspace.local"));
            if name == "agent-browser-dashboard.service" {
                assert!(body.contains("EnvironmentFile=-%h/.agent-browser/.env"));
            }
            if name == "agent-browser-runtime-interlock.timer" {
                assert!(body.contains("OnActiveSec=5min"));
                assert!(!body.contains("OnBootSec="));
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn stable_generation_link_accepts_equivalent_absolute_target() {
        use std::os::unix::fs::symlink;

        let root = env::temp_dir().join(format!(
            "agent-browser-equivalent-stable-link-{}",
            uuid::Uuid::new_v4()
        ));
        let link_dir = root.join("links");
        let target = root.join("current/units/example.service");
        let link = link_dir.join("example.service");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::create_dir_all(&link_dir).unwrap();
        fs::write(&target, b"unit\n").unwrap();
        symlink(&target, &link).unwrap();

        assert!(
            !ensure_stable_symlink(&link, Path::new("../current/units/example.service")).unwrap()
        );

        fs::remove_file(&link).unwrap();
        let unexpected = root.join("other/example.service");
        fs::create_dir_all(unexpected.parent().unwrap()).unwrap();
        fs::write(&unexpected, b"other unit\n").unwrap();
        symlink(&unexpected, &link).unwrap();
        assert!(
            ensure_stable_symlink(&link, Path::new("../current/units/example.service")).is_err()
        );

        fs::remove_file(&link).unwrap();
        fs::remove_file(unexpected).unwrap();
        fs::remove_file(target).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manifest_records_binary_owned_runtime() {
        let args = WorkstationInstallArgs {
            mode: InstallMode::Apply,
            json: true,
            dashboard_port: 4848,
            guacamole_port: 8092,
        };
        let units = render_units(
            "/tmp/agent-browser",
            Path::new("/tmp/support"),
            Path::new("/tmp/secrets"),
            args.dashboard_port,
        );
        let manifest = render_manifest(&args, "fixture-binary-sha256", &units);
        assert!(manifest.contains(r#""runtimeController": "installed-binary""#));
        assert!(manifest.contains(r#""sourceCheckoutRequired": false"#));
        assert!(manifest.contains(r#""sha256": "fixture-binary-sha256""#));
        assert!(manifest.contains(r#""controllerAssets""#));
        assert!(manifest.contains(r#""guacamoleBundleManifestSha256""#));
        assert!(manifest.contains(r#""agent-browser-runtime-interlock.timer""#));
    }

    #[test]
    fn legacy_mutable_payload_migrates_to_a_sealed_rollback_generation() {
        let root = env::temp_dir().join(format!(
            "agent-browser-legacy-generation-migration-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        fs::create_dir_all(paths.binary.parent().unwrap()).unwrap();
        fs::create_dir_all(&paths.legacy_support_dir).unwrap();
        fs::create_dir_all(&paths.unit_dir).unwrap();
        fs::write(&paths.binary, b"legacy-binary").unwrap();
        set_executable(&paths.binary).unwrap();
        fs::write(paths.legacy_support_dir.join("manifest.json"), b"{}\n").unwrap();
        fs::write(paths.legacy_support_dir.join("README.txt"), b"legacy\n").unwrap();
        for unit in WORKSTATION_GENERATION_UNITS {
            if unit == "agent-browser-dashboard-backend.service" {
                continue;
            }
            fs::write(paths.unit_dir.join(unit), format!("legacy {unit}\n")).unwrap();
        }

        let generation_id = migrate_legacy_payload_to_generation(&paths).unwrap();
        let generation = paths.generations_dir.join(&generation_id);
        assert_eq!(
            fs::read_link(&paths.current_selector).unwrap(),
            PathBuf::from("generations").join(&generation_id)
        );
        assert_eq!(
            fs::read_link(&paths.binary).unwrap(),
            PathBuf::from("../lib/agent-browser/current/bin/agent-browser")
        );
        for unit in WORKSTATION_GENERATION_UNITS {
            assert_eq!(
                fs::read_link(paths.unit_dir.join(unit)).unwrap(),
                PathBuf::from("../../../.local/lib/agent-browser/current/units").join(unit)
            );
        }
        assert_eq!(
            fs::read(generation.join("bin/agent-browser")).unwrap(),
            b"legacy-binary"
        );
        assert!(generation.join("support/README.txt").is_file());
        assert!(fs::read_to_string(
            generation.join("units/agent-browser-dashboard-backend.service")
        )
        .unwrap()
        .contains("Inactive placeholder"));
        validate_generation_install_preconditions(&paths).unwrap();
        validate_sealed_generation_tree(&generation).unwrap();

        remove_generation_tree(&generation).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn operator_recovery_closes_only_a_verified_matching_drain() {
        let root = env::temp_dir().join(format!(
            "agent-browser-operator-recovery-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        fs::create_dir_all(paths.binary.parent().unwrap()).unwrap();
        fs::create_dir_all(&paths.legacy_support_dir).unwrap();
        fs::create_dir_all(&paths.unit_dir).unwrap();
        fs::write(&paths.binary, b"legacy-binary").unwrap();
        set_executable(&paths.binary).unwrap();
        fs::write(paths.legacy_support_dir.join("manifest.json"), b"{}\n").unwrap();
        for unit in WORKSTATION_GENERATION_UNITS {
            if unit != "agent-browser-dashboard-backend.service" {
                fs::write(paths.unit_dir.join(unit), format!("legacy {unit}\n")).unwrap();
            }
        }
        let old_generation_id = migrate_legacy_payload_to_generation(&paths).unwrap();
        let mut transaction = new_upgrade_transaction(
            &paths,
            "generation-candidate".to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        transaction.old_generation_id = Some(old_generation_id.clone());
        transaction.state =
            crate::runtime_adoption::UpgradeTransactionState::OperatorRecoveryRequired;
        transaction.revision = 7;
        transaction.stop_reason = Some("candidate_dashboard_presentation_unproven".to_string());
        transaction.terminal_result = Some("operator_recovery_required".to_string());
        let transaction_path = transaction_path(&root, &transaction.transaction_id);
        write_private_json_atomic(&transaction_path, &transaction).unwrap();
        let drain_path = root.join(".agent-browser/runtime-adoption/admission-drain.json");
        persist_admission_drain(&drain_path, &transaction).unwrap();

        let report =
            recover_operator_required_upgrade_for_root(&root, &transaction.transaction_id, true)
                .unwrap();

        assert_eq!(report["changed"], true);
        assert_eq!(report["selectedGenerationId"], old_generation_id);
        assert!(!drain_path.exists());
        let recovered: crate::runtime_adoption::UpgradeTransaction =
            serde_json::from_slice(&fs::read(transaction_path).unwrap()).unwrap();
        assert_eq!(
            recovered.state,
            crate::runtime_adoption::UpgradeTransactionState::FailedPreservedOldGeneration
        );
        assert_eq!(
            recovered.terminal_result.as_deref(),
            Some("old_generation_preserved")
        );
        assert!(recovered
            .checkpoints
            .iter()
            .any(|checkpoint| checkpoint.name == "operator_recovery_verified_old_generation"));

        remove_generation_tree(&paths.generations_dir.join(old_generation_id)).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn operator_recovery_stops_only_a_receipted_rolled_back_candidate_host() {
        use crate::runtime_adoption::{
            RuntimeHostConvergenceRecord, RuntimeHostIdentityEvidence, RuntimeLaneTransferRecord,
            RuntimeLaneTransferState, UpgradeTransactionState,
        };

        let root = PathBuf::from("/tmp/agent-browser-rolled-back-host-fixture");
        let paths = install_paths(&root);
        let mut transaction = new_upgrade_transaction(
            &paths,
            "generation-candidate".to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        transaction.state = UpgradeTransactionState::OperatorRecoveryRequired;
        transaction.runtime_host_convergence = Some(RuntimeHostConvergenceRecord {
            schema_version: "agent-browser.runtime-host-convergence.v1".to_string(),
            deadline_at: "2026-08-22T00:00:00Z".to_string(),
            deadline_unix_seconds: 0,
            queue_transfer_policy: "drain_then_commit".to_string(),
            old_host: None,
            candidate_host: Some(RuntimeHostIdentityEvidence {
                endpoint_key: "runtime-host".to_string(),
                generation_id: "generation-candidate".to_string(),
                binary_sha256: "a".repeat(64),
                pid: 41,
                process_start_token: "linux:boot:41".to_string(),
                socket_identity: "unix:1:41".to_string(),
                observation_only: true,
            }),
            lanes: vec![RuntimeLaneTransferRecord {
                session_name: "source".to_string(),
                candidate_session_name: Some("candidate".to_string()),
                source_generation_id: Some("generation-old".to_string()),
                candidate_generation_id: "generation-candidate".to_string(),
                state: RuntimeLaneTransferState::RolledBack,
                owner_generation_before: Some(4),
                owner_generation_after: Some(5),
                rollback_owner_generation: Some(6),
                observation_receipt_id: Some("observation".to_string()),
                commit_receipt_id: Some("commit".to_string()),
                rollback_receipt_id: Some("rollback".to_string()),
                queued_work_count: 0,
            }],
        });
        assert!(operator_recovery_can_stop_rolled_back_candidate_host(
            &transaction
        ));

        transaction.runtime_host_convergence.as_mut().unwrap().lanes[0].rollback_receipt_id = None;
        assert!(!operator_recovery_can_stop_rolled_back_candidate_host(
            &transaction
        ));
    }

    #[cfg(unix)]
    #[test]
    fn finalized_runtime_host_grace_observes_self_exit_before_pidfd_fallback() {
        let mut child = Command::new("sh")
            .args(["-c", "sleep 0.05"])
            .spawn()
            .unwrap();
        let executable = PathBuf::from(format!("/proc/{}/exe", child.id()))
            .canonicalize()
            .unwrap();
        let identity =
            crate::process_identity::capture_process_identity(child.id(), Some(&executable), None)
                .unwrap();

        assert!(
            wait_for_recorded_process_exit(&identity, std::time::Duration::from_secs(1)).unwrap()
        );
        child.wait().unwrap();
    }

    #[test]
    fn post_commit_supervisor_drift_requires_one_candidate_host_and_old_manifests() {
        let root = PathBuf::from("/tmp/agent-browser-supervisor-transition-fixture");
        let paths = install_paths(&root);
        let mut transaction = new_upgrade_transaction(
            &paths,
            "generation-candidate".to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        transaction.old_generation_id = Some("generation-old".to_string());
        let ready = serde_json::json!({
            "sessionSupervisors": {
                "sessions": [{
                    "manifest": {
                        "executablePath": "/home/test/generations/generation-old/bin/agent-browser"
                    }
                }],
                "issues": [{"code": "executable_drift"}]
            },
            "runtimeMultiplicity": {
                "runtimeHosts": [{"generationId": "generation-candidate"}],
                "legacyDaemons": []
            }
        });
        assert!(expected_upgrade_supervisor_transition_ready(
            &ready,
            &transaction
        ));

        let mut duplicate = ready.clone();
        duplicate["runtimeMultiplicity"]["runtimeHosts"] = serde_json::json!([
            {"generationId": "generation-candidate"},
            {"generationId": "generation-old"}
        ]);
        assert!(!expected_upgrade_supervisor_transition_ready(
            &duplicate,
            &transaction
        ));
    }

    #[cfg(unix)]
    #[test]
    fn operator_recovery_closes_verified_pre_admission_census_block_without_drain() {
        let root = env::temp_dir().join(format!(
            "agent-browser-pre-admission-recovery-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        fs::create_dir_all(paths.binary.parent().unwrap()).unwrap();
        fs::create_dir_all(&paths.legacy_support_dir).unwrap();
        fs::create_dir_all(&paths.unit_dir).unwrap();
        fs::write(&paths.binary, b"legacy-binary").unwrap();
        set_executable(&paths.binary).unwrap();
        fs::write(paths.legacy_support_dir.join("manifest.json"), b"{}\n").unwrap();
        for unit in WORKSTATION_GENERATION_UNITS {
            if unit != "agent-browser-dashboard-backend.service" {
                fs::write(paths.unit_dir.join(unit), format!("legacy {unit}\n")).unwrap();
            }
        }
        let old_generation_id = migrate_legacy_payload_to_generation(&paths).unwrap();
        let mut transaction = new_upgrade_transaction(
            &paths,
            "generation-candidate".to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        transaction.old_generation_id = Some(old_generation_id.clone());
        transaction.state =
            crate::runtime_adoption::UpgradeTransactionState::BlockedAmbiguousRuntime;
        transaction.revision = 1;
        transaction.stop_reason = Some("runtime_census_ambiguous".to_string());
        let transaction_path = transaction_path(&root, &transaction.transaction_id);
        write_private_json_atomic(&transaction_path, &transaction).unwrap();

        let report =
            recover_operator_required_upgrade_for_root(&root, &transaction.transaction_id, true)
                .unwrap();

        assert_eq!(report["changed"], true);
        assert_eq!(report["selectedGenerationId"], old_generation_id);
        assert_eq!(report["admissionDrainPresent"], false);
        let recovered: crate::runtime_adoption::UpgradeTransaction =
            serde_json::from_slice(&fs::read(transaction_path).unwrap()).unwrap();
        assert_eq!(
            recovered.state,
            crate::runtime_adoption::UpgradeTransactionState::FailedPreservedOldGeneration
        );
        assert_eq!(
            recovered.terminal_result.as_deref(),
            Some("old_generation_preserved")
        );
        assert!(recovered
            .checkpoints
            .iter()
            .any(|checkpoint| { checkpoint.name == "pre_admission_census_block_recovered" }));

        remove_generation_tree(&paths.generations_dir.join(old_generation_id)).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn relocated_legacy_daemon_identity_requires_exact_start_path_and_binary_digest() {
        let root = PathBuf::from("/tmp/agent-browser-legacy-daemon-identity-fixture");
        let paths = install_paths(&root);
        let recorded = crate::process_identity::RecordedProcessIdentity {
            pid: 41,
            start_token: "linux:boot:100".to_string(),
            executable_path: Some(paths.binary.display().to_string()),
            browser_family: None,
        };
        let relocated = paths
            .binary
            .parent()
            .unwrap()
            .join(".agent-browser.legacy-fixture");
        let observed = crate::process_identity::ObservedProcessIdentity {
            pid: 41,
            start_token: Some("linux:boot:100".to_string()),
            executable_path: Some(format!("{} (deleted)", relocated.display())),
            browser_family: None,
            command_line: Some(vec![paths.binary.display().to_string()]),
        };

        let reconciled = reconciled_legacy_daemon_identity(
            &paths,
            &recorded,
            &observed,
            "matching-sha",
            "matching-sha",
        )
        .unwrap();
        assert_eq!(reconciled.executable_path, observed.executable_path);

        let mut wrong_start = observed.clone();
        wrong_start.start_token = Some("linux:boot:101".to_string());
        assert!(reconciled_legacy_daemon_identity(
            &paths,
            &recorded,
            &wrong_start,
            "matching-sha",
            "matching-sha",
        )
        .is_none());
        assert!(reconciled_legacy_daemon_identity(
            &paths,
            &recorded,
            &observed,
            "matching-sha",
            "different-sha",
        )
        .is_none());

        let mut wrong_path = observed;
        wrong_path.executable_path = Some("/tmp/unrelated-agent-browser (deleted)".to_string());
        assert!(reconciled_legacy_daemon_identity(
            &paths,
            &recorded,
            &wrong_path,
            "matching-sha",
            "matching-sha",
        )
        .is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn deleted_idle_daemon_identity_requires_exact_start_path_family_and_binary_digest() {
        let recorded = crate::process_identity::RecordedProcessIdentity {
            pid: 73587,
            start_token: "linux:boot:44070287".to_string(),
            executable_path: Some("/workspace/cli/target/release/agent-browser".to_string()),
            browser_family: None,
        };
        let observed = crate::process_identity::ObservedProcessIdentity {
            pid: 73587,
            start_token: Some("linux:boot:44070287".to_string()),
            executable_path: Some(
                "/workspace/cli/target/release/agent-browser (deleted)".to_string(),
            ),
            browser_family: None,
            command_line: None,
        };
        let digest = "a".repeat(64);

        let reconciled =
            reconciled_deleted_idle_daemon_identity(&recorded, &observed, &digest, &digest)
                .expect("exact deleted idle daemon identity should reconcile");
        assert_eq!(
            reconciled.executable_path.as_deref(),
            Some("/workspace/cli/target/release/agent-browser (deleted)")
        );

        let mut wrong_start = observed.clone();
        wrong_start.start_token = Some("linux:boot:other".to_string());
        assert!(
            reconciled_deleted_idle_daemon_identity(&recorded, &wrong_start, &digest, &digest,)
                .is_none()
        );
        assert!(reconciled_deleted_idle_daemon_identity(
            &recorded,
            &observed,
            &digest,
            &"b".repeat(64),
        )
        .is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn verified_idle_daemon_retirement_escalates_after_grace_timeout() {
        use std::io::BufRead;

        let mut child = Command::new("sh")
            .args(["-c", "trap '' TERM; printf 'ready\\n'; while :; do :; done"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut ready = String::new();
        std::io::BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        assert_eq!(ready, "ready\n");
        let identity = crate::process_identity::capture_process_identity(child.id(), None, None)
            .expect("fixture process identity");
        let process = crate::process_identity::VerifiedProcessTermination::open(&identity)
            .unwrap()
            .expect("fixture process must be live");

        retire_verified_idle_daemon_process(
            &process,
            std::time::Duration::from_millis(25),
            std::time::Duration::from_secs(1),
        )
        .unwrap();
        assert!(!process.is_running().unwrap());
        child.wait().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn generation_gc_retains_live_process_supervisor_and_unclosed_transaction_references() {
        use std::os::unix::fs::symlink;

        let root = env::temp_dir().join(format!(
            "agent-browser-generation-gc-references-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        fs::create_dir_all(&paths.generations_dir).unwrap();
        fs::create_dir_all(paths.current_selector.parent().unwrap()).unwrap();
        symlink(
            PathBuf::from("generations").join("selected-generation"),
            &paths.current_selector,
        )
        .unwrap();

        let mut transaction = new_upgrade_transaction(
            &paths,
            "transaction-candidate".to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        transaction.old_generation_id = Some("transaction-old".to_string());
        transaction.state = crate::runtime_adoption::UpgradeTransactionState::Accepted;
        write_private_json_atomic(
            &transaction_path(&root, &transaction.transaction_id),
            &transaction,
        )
        .unwrap();

        let supervisor_dir = root.join(".config/agent-browser/session-supervisors");
        fs::create_dir_all(&supervisor_dir).unwrap();
        write_private_json_atomic(
            &supervisor_dir.join("supervisor.json"),
            &serde_json::json!({
                "executablePath": paths
                    .generations_dir
                    .join("supervisor-generation/bin/agent-browser")
            }),
        )
        .unwrap();

        let fake_proc = root.join("proc");
        let process_dir = fake_proc.join("4242");
        fs::create_dir_all(&process_dir).unwrap();
        symlink(
            paths
                .generations_dir
                .join("live-process-generation/bin/agent-browser"),
            process_dir.join("exe"),
        )
        .unwrap();

        let mut references = generation_references(&root, &paths).unwrap();
        collect_process_generation_references_from(&fake_proc, &paths, &mut references);
        for reasons in references.values_mut() {
            reasons.sort();
            reasons.dedup();
        }

        assert_eq!(
            references["selected-generation"],
            vec!["selected_generation"]
        );
        assert_eq!(
            references["transaction-old"],
            vec!["transaction_old_generation"]
        );
        assert_eq!(
            references["transaction-candidate"],
            vec!["transaction_candidate_generation"]
        );
        assert_eq!(
            references["supervisor-generation"],
            vec!["session_supervisor"]
        );
        assert_eq!(references["live-process-generation"], vec!["live_process"]);

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn accepted_transaction_history_no_longer_pins_retired_generations() {
        use std::os::unix::fs::symlink;

        let root = env::temp_dir().join(format!(
            "agent-browser-p117-accepted-generation-history-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        fs::create_dir_all(&paths.generations_dir).unwrap();
        fs::create_dir_all(paths.current_selector.parent().unwrap()).unwrap();
        symlink(
            PathBuf::from("generations").join("selected-generation"),
            &paths.current_selector,
        )
        .unwrap();

        let mut transaction = new_upgrade_transaction(
            &paths,
            "accepted-candidate".to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        transaction.old_generation_id = Some("accepted-old".to_string());
        transaction.state = crate::runtime_adoption::UpgradeTransactionState::Accepted;
        transaction.dashboard_validation_summary = Some("ready".to_string());
        transaction.presentation_validation_summary = Some("ready".to_string());
        transaction.terminal_result = Some("accepted".to_string());
        transaction.checkpoints[0].recorded_at = "2026-08-01T00:00:00Z".to_string();
        write_private_json_atomic(
            &transaction_path(&root, &transaction.transaction_id),
            &transaction,
        )
        .unwrap();

        let references = generation_references(&root, &paths).unwrap();
        assert!(!references.contains_key("accepted-old"));
        assert!(!references.contains_key("accepted-candidate"));
        assert_eq!(
            references["selected-generation"],
            vec!["selected_generation"]
        );

        let plan = generation_retention_plan(&root, &paths, true).unwrap();
        assert_eq!(plan.finalizable_transaction_ids.len(), 1);
        let finalized: crate::runtime_adoption::UpgradeTransaction = serde_json::from_slice(
            &fs::read(transaction_path(&root, &transaction.transaction_id)).unwrap(),
        )
        .unwrap();
        assert_eq!(
            finalized.state,
            crate::runtime_adoption::UpgradeTransactionState::OldGenerationRetirable
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stable_census_receipt_commits_before_any_payload_path_exists() {
        let root = env::temp_dir().join(format!(
            "agent-browser-census-gate-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let paths = install_paths(&root);
        let args = WorkstationInstallArgs {
            mode: InstallMode::Apply,
            json: true,
            dashboard_port: 4848,
            guacamole_port: 8092,
        };
        let round = crate::runtime_adoption::collect_runtime_census_round(
            11,
            crate::runtime_adoption::runtime_census_sources()
                .into_iter()
                .map(
                    |source| crate::runtime_adoption::RuntimeCensusSourceSnapshot {
                        source,
                        source_revision: format!("{source:?}-stable"),
                        logical_browser_ids: Vec::new(),
                    },
                )
                .collect(),
            Vec::new(),
        )
        .unwrap();
        let mut calls = 0usize;
        let receipt = require_stable_runtime_census_with(&root, &paths, &args, || {
            calls += 1;
            Ok(round.clone())
        })
        .unwrap();

        assert_eq!(calls, 2);
        assert!(receipt.is_file());
        assert!(!paths.generations_dir.exists());
        assert!(!paths.current_selector.exists());
        assert!(!paths.binary.exists());
        let transaction: Value = serde_json::from_slice(&fs::read(&receipt).unwrap()).unwrap();
        assert_eq!(transaction["state"], "census_stable");
        assert_eq!(transaction["runtimeMigrations"], serde_json::json!([]));
        assert_eq!(
            transaction["runtimeCensusDigest"].as_str().unwrap().len(),
            64
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&receipt).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn runtime_census_requires_an_adjacent_stable_pair_within_the_bounded_window() {
        fn round(registry_revision: u64) -> crate::runtime_adoption::RuntimeCensusRound {
            use crate::runtime_adoption::{
                EvidenceAgreement, RuntimeCensusCandidate, RuntimeCensusSource,
                RuntimeCensusSourceSnapshot, RuntimeEvidenceSummary,
            };

            let runtime_id = "browser-a".to_string();
            crate::runtime_adoption::collect_runtime_census_round(
                registry_revision,
                crate::runtime_adoption::runtime_census_sources()
                    .into_iter()
                    .map(|source| RuntimeCensusSourceSnapshot {
                        source,
                        source_revision: format!("{source:?}-stable"),
                        logical_browser_ids: (source == RuntimeCensusSource::ServiceBrowserRecords)
                            .then(|| vec![runtime_id.clone()])
                            .unwrap_or_default(),
                    })
                    .collect(),
                vec![RuntimeCensusCandidate {
                    logical_browser_id: runtime_id,
                    session_names: Vec::new(),
                    profile_identity_digest: "a".repeat(64),
                    observation_digest: "b".repeat(64),
                    observed_sources: vec![RuntimeCensusSource::ServiceBrowserRecords],
                    evidence: RuntimeEvidenceSummary {
                        observation_rounds_agree: true,
                        registry_revision_stable: true,
                        manual_browser: true,
                        metadata_present: true,
                        profile_identity: EvidenceAgreement::Match,
                        ..RuntimeEvidenceSummary::default()
                    },
                }],
            )
            .unwrap()
        }

        let mut converging = std::collections::VecDeque::from([round(11), round(12), round(12)]);
        let converged = collect_stable_runtime_census_with(|| {
            converging
                .pop_front()
                .ok_or_else(|| "test census exhausted".to_string())
        })
        .unwrap();
        assert!(converged.activation_allowed);
        assert!(converging.is_empty());

        let mut late_convergence = (11..18)
            .map(round)
            .chain(std::iter::once(round(17)))
            .collect::<std::collections::VecDeque<_>>();
        let converged = collect_stable_runtime_census_with(|| {
            late_convergence
                .pop_front()
                .ok_or_else(|| "test census exhausted".to_string())
        })
        .unwrap();
        assert!(converged.activation_allowed);
        assert!(late_convergence.is_empty());

        let mut changing = (11..19)
            .map(round)
            .collect::<std::collections::VecDeque<_>>();
        let blocked = collect_stable_runtime_census_with(|| {
            changing
                .pop_front()
                .ok_or_else(|| "test census exhausted".to_string())
        })
        .unwrap();
        assert!(!blocked.activation_allowed);
        assert!(changing.is_empty());
        assert!(blocked.records[0]
            .reason_codes
            .contains(&"census_changed_during_classification".to_string()));
    }

    #[test]
    fn blocked_precondition_records_one_transaction_without_staging_payload() {
        let root = env::temp_dir().join(format!(
            "agent-browser-blocked-upgrade-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let paths = install_paths(&root);
        let args = WorkstationInstallArgs {
            mode: InstallMode::Apply,
            json: true,
            dashboard_port: 4848,
            guacamole_port: 8092,
        };
        let receipt = record_blocked_upgrade_transaction(
            &root,
            &paths,
            &args,
            crate::runtime_adoption::UpgradeTransactionState::BlockedInflightEffect,
            "installer_lock_busy",
        )
        .unwrap();
        let transaction: crate::runtime_adoption::UpgradeTransaction =
            serde_json::from_slice(&fs::read(receipt).unwrap()).unwrap();

        assert_eq!(
            transaction.state,
            crate::runtime_adoption::UpgradeTransactionState::BlockedInflightEffect
        );
        assert_eq!(
            transaction.stop_reason.as_deref(),
            Some("installer_lock_busy")
        );
        let revision = transaction.revision;
        let mut recovered = transaction.clone();
        recovered.terminal_result = Some("old_generation_preserved".to_string());
        crate::runtime_adoption::transition_upgrade_transaction(
            &mut recovered,
            revision,
            crate::runtime_adoption::UpgradeTransactionState::FailedPreservedOldGeneration,
            "pre_admission_inflight_block_recovered",
            "2026-08-21T21:45:00Z",
        )
        .expect("a pre-admission lock collision must have a terminal recovery transition");
        assert_eq!(
            recovered.state,
            crate::runtime_adoption::UpgradeTransactionState::FailedPreservedOldGeneration
        );
        assert!(!paths.generations_dir.exists());
        assert!(!paths.current_selector.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn upgrade_readiness_reports_all_seven_axes_and_never_accepts_an_active_transaction() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let root = env::temp_dir().join(format!(
            "agent-browser-upgrade-readiness-axes-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        let generation_id = "generation-ready";
        let generation_root = paths.generations_dir.join(generation_id);
        fs::create_dir_all(&generation_root).unwrap();
        fs::set_permissions(&generation_root, fs::Permissions::from_mode(0o555)).unwrap();
        fs::create_dir_all(paths.current_selector.parent().unwrap()).unwrap();
        symlink(
            PathBuf::from("generations").join(generation_id),
            &paths.current_selector,
        )
        .unwrap();
        let mut transaction = new_upgrade_transaction(
            &paths,
            generation_id.to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        transaction.runtime_census_digest = Some("c".repeat(64));
        transaction.state = crate::runtime_adoption::UpgradeTransactionState::PostCommitValidating;
        let ingress = serde_json::json!({
            "dashboardIngressReady": true,
            "operatorJourneyReady": true,
        });

        let active = workstation_upgrade_readiness(
            &paths,
            Some(generation_id),
            Some(&transaction),
            true,
            &ingress,
        );
        for axis in [
            "payloadReady",
            "selectedGenerationReady",
            "runtimeConvergenceReady",
            "dashboardIngressReady",
            "operatorJourneyReady",
            "rollbackReady",
        ] {
            assert_eq!(active[axis], true, "{axis}");
        }
        assert_eq!(active["upgradeTransactionState"], "post_commit_validating");
        assert_eq!(active["ready"], false);

        transaction.state = crate::runtime_adoption::UpgradeTransactionState::Accepted;
        let accepted = workstation_upgrade_readiness(
            &paths,
            Some(generation_id),
            Some(&transaction),
            false,
            &ingress,
        );
        assert_eq!(accepted["ready"], true);

        transaction.state =
            crate::runtime_adoption::UpgradeTransactionState::OldGenerationRetirable;
        transaction.old_generation_id = Some("generation-already-collected".to_string());
        let finalized = workstation_upgrade_readiness(
            &paths,
            Some(generation_id),
            Some(&transaction),
            false,
            &ingress,
        );
        assert_eq!(finalized["rollbackReady"], true);
        assert_eq!(finalized["ready"], true);
        fs::set_permissions(&generation_root, fs::Permissions::from_mode(0o755)).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn post_commit_validation_requires_a_matching_live_candidate_presentation_receipt() {
        use std::net::TcpListener;
        use std::os::unix::fs::{symlink, PermissionsExt};

        let root = env::temp_dir().join(format!(
            "agent-browser-post-commit-presentation-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        let old_generation = "generation-old";
        let candidate_generation = "generation-candidate";
        let old_root = paths.generations_dir.join(old_generation);
        let candidate_root = paths.generations_dir.join(candidate_generation);
        fs::create_dir_all(&old_root).unwrap();
        fs::create_dir_all(&candidate_root).unwrap();
        fs::set_permissions(&old_root, fs::Permissions::from_mode(0o555)).unwrap();
        fs::set_permissions(&candidate_root, fs::Permissions::from_mode(0o555)).unwrap();
        fs::create_dir_all(paths.current_selector.parent().unwrap()).unwrap();
        symlink(
            PathBuf::from("generations").join(old_generation),
            &paths.current_selector,
        )
        .unwrap();

        let mut transaction = new_upgrade_transaction(
            &paths,
            candidate_generation.to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        transaction.runtime_census_digest = Some("c".repeat(64));
        transaction.state = crate::runtime_adoption::UpgradeTransactionState::PostCommitValidating;
        transaction.revision = 6;
        let transaction_path = transaction_path(&root, &transaction.transaction_id);
        write_private_json_atomic(&transaction_path, &transaction).unwrap();
        let admission_drain_path = root
            .join(".agent-browser/runtime-adoption")
            .join("admission-drain.json");
        persist_admission_drain(&admission_drain_path, &transaction).unwrap();
        fs::remove_file(&paths.current_selector).unwrap();
        symlink(
            PathBuf::from("generations").join(candidate_generation),
            &paths.current_selector,
        )
        .unwrap();

        let manifest = serde_json::json!({
            "schemaVersion": "agent-browser.runtime-manifest.v1",
        });
        let manifest_body = manifest.to_string();
        let manifest_sha256 = workstation_bytes_sha256(manifest_body.as_bytes());
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let candidate_port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                manifest_body.len(),
                manifest_body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let ingress_path = root.join(".agent-browser/dashboard-ingress.json");
        let repository = crate::dashboard_ingress::DashboardIngressRepository::new(&ingress_path);
        repository
            .initialize(crate::dashboard_ingress::DashboardBackend::new(
                old_generation,
                candidate_port.saturating_add(1),
                "old-manifest",
            ))
            .unwrap();
        let staged = repository
            .stage_candidate(
                1,
                crate::dashboard_ingress::DashboardBackend::new(
                    candidate_generation,
                    candidate_port,
                    manifest_sha256,
                ),
            )
            .unwrap();
        let receipt_id = "presentation-receipt-candidate";
        repository
            .commit_candidate(
                staged.revision,
                crate::dashboard_ingress::CandidateOperatorJourney::ready(
                    crate::dashboard_ingress::PresentationEvidence {
                        receipt_id: receipt_id.to_string(),
                        dashboard_deployment_generation: candidate_generation.to_string(),
                        coordinator_generation: candidate_generation.to_string(),
                        daemon_generation: candidate_generation.to_string(),
                        logical_browser_id: "browser-fixture".to_string(),
                        process_instance_digest: "d".repeat(64),
                        selected_target_generation: 9,
                        selected_target_identity_digest: "e".repeat(64),
                        required_stream_provider: "rdp".to_string(),
                        observed_stream_provider: "rdp".to_string(),
                        display_allocation_id: "display-fixture".to_string(),
                        geometry_epoch: "geometry-fixture".to_string(),
                        route_generation: 4,
                        guacamole_connection_generation: Some(5),
                        authenticated_ingress_probe_at: runtime_adoption_timestamp(),
                        operator_surface_load_result: "ready".to_string(),
                    },
                ),
            )
            .unwrap();
        server.join().unwrap();

        let mut prepared = PreparedPayloadTransaction {
            staged: StagedWorkstationGeneration {
                generation_id: candidate_generation.to_string(),
                generation_path: candidate_root.clone(),
                binary_sha256: "a".repeat(64),
                support_manifest_sha256: "b".repeat(64),
                rendered_units: Vec::new(),
            },
            transaction_path,
            transaction,
            previous_selector: Some(PathBuf::from("generations").join(old_generation)),
            admission_drain_path,
            runtime_handoffs: Vec::new(),
            dashboard_candidate: None,
        };

        let validation = validate_post_commit_transaction(&root, &paths, &prepared).unwrap();
        assert_eq!(
            validation.dashboard_summary,
            format!("candidate_dashboard_generation_receipted:{receipt_id}")
        );
        assert_eq!(
            validation.presentation_summary,
            format!("authenticated_operator_journey_receipted:{receipt_id}")
        );
        accept_prepared_payload_transaction(&mut prepared, validation).unwrap();
        let finalized = finalize_accepted_upgrade_for_root(&root).unwrap();
        assert_eq!(finalized["changed"], true);
        assert_eq!(finalized["state"], "old_generation_retirable");
        let latest =
            latest_upgrade_transaction(&root.join(".agent-browser/runtime-adoption/transactions"))
                .unwrap()
                .unwrap();
        assert_eq!(
            latest.state,
            crate::runtime_adoption::UpgradeTransactionState::OldGenerationRetirable
        );

        fs::set_permissions(&old_root, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&candidate_root, fs::Permissions::from_mode(0o755)).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn candidate_dashboard_wait_observes_one_concurrent_authenticated_commit() {
        use std::net::TcpListener;

        let root = env::temp_dir().join(format!(
            "agent-browser-candidate-dashboard-wait-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        let generation_id = "generation-candidate";
        let manifest = serde_json::json!({
            "schemaVersion": "agent-browser.runtime-manifest.v1",
        });
        let manifest_body = manifest.to_string();
        let manifest_sha256 = workstation_bytes_sha256(manifest_body.as_bytes());
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                manifest_body.len(),
                manifest_body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let ingress_path = root.join(".agent-browser/dashboard-ingress.json");
        let repository = crate::dashboard_ingress::DashboardIngressRepository::new(&ingress_path);
        let initial = repository
            .initialize(crate::dashboard_ingress::DashboardBackend::new(
                "generation-old",
                port.saturating_add(1),
                "old-manifest",
            ))
            .unwrap();
        let backend =
            crate::dashboard_ingress::DashboardBackend::new(generation_id, port, manifest_sha256);
        let staged = repository
            .stage_candidate(initial.revision, backend.clone())
            .unwrap();
        let staged_revision = staged.revision;
        let commit_path = ingress_path.clone();
        let committer = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            crate::dashboard_ingress::DashboardIngressRepository::new(commit_path)
                .commit_candidate(
                    staged_revision,
                    crate::dashboard_ingress::CandidateOperatorJourney::ready(
                        crate::dashboard_ingress::PresentationEvidence {
                            receipt_id: "presentation-concurrent".to_string(),
                            dashboard_deployment_generation: generation_id.to_string(),
                            coordinator_generation: generation_id.to_string(),
                            daemon_generation: generation_id.to_string(),
                            logical_browser_id: "browser-fixture".to_string(),
                            process_instance_digest: "d".repeat(64),
                            selected_target_generation: 3,
                            selected_target_identity_digest: "e".repeat(64),
                            required_stream_provider: "rdp".to_string(),
                            observed_stream_provider: "rdp".to_string(),
                            display_allocation_id: "display-fixture".to_string(),
                            geometry_epoch: "geometry-fixture".to_string(),
                            route_generation: 2,
                            guacamole_connection_generation: Some(1),
                            authenticated_ingress_probe_at: runtime_adoption_timestamp(),
                            operator_surface_load_result: "ready".to_string(),
                        },
                    ),
                )
                .unwrap();
        });
        let transaction = new_upgrade_transaction(
            &paths,
            generation_id.to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        let mut prepared = PreparedPayloadTransaction {
            staged: StagedWorkstationGeneration {
                generation_id: generation_id.to_string(),
                generation_path: paths.generations_dir.join(generation_id),
                binary_sha256: "a".repeat(64),
                support_manifest_sha256: "b".repeat(64),
                rendered_units: Vec::new(),
            },
            transaction_path: transaction_path(&root, &transaction.transaction_id),
            transaction,
            previous_selector: None,
            admission_drain_path: root.join("admission-drain.json"),
            runtime_handoffs: Vec::new(),
            dashboard_candidate: Some(PreparedDashboardCandidate {
                child: None,
                backend,
                ingress_path: ingress_path.clone(),
                staged_revision,
            }),
        };

        wait_for_dashboard_candidate_commit(&mut prepared, std::time::Duration::from_secs(2))
            .unwrap();
        committer.join().unwrap();
        server.join().unwrap();
        let selected = repository.load().unwrap();
        assert_eq!(selected.selected_backend().generation_id, generation_id);
        fs::remove_file(ingress_path.with_extension("json.lock")).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn candidate_dashboard_targets_the_transaction_runtime_host() {
        let binary = Path::new("/opt/agent-browser/candidate/bin/agent-browser");
        let socket_dir = Path::new("/run/user/1000/agent-browser/runtime-hosts/transaction-a");
        let command = candidate_dashboard_command(binary, 4850, "generation-candidate", socket_dir);
        let env = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.map(|value| value.to_string_lossy().into_owned()),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();

        assert_eq!(
            env.get("AGENT_BROWSER_SOCKET_DIR").and_then(Clone::clone),
            Some(socket_dir.display().to_string())
        );
        assert_eq!(
            env.get("AGENT_BROWSER_DASHBOARD_GENERATION")
                .and_then(Clone::clone),
            Some("generation-candidate".to_string())
        );
        assert_eq!(
            env.get(crate::runtime_host::RUNTIME_HOST_ENV)
                .and_then(Clone::clone),
            Some("1".to_string())
        );
        assert_eq!(env.get("AGENT_BROWSER_DASHBOARD_BACKEND_ONLY"), None);
    }

    #[test]
    fn installed_command_environment_clears_ambient_route_pool() {
        let paths = install_paths(Path::new("/tmp/workstation-command-env"));
        let command_env = workstation_command_env(&paths);
        assert!(command_env.iter().any(|(key, value)| {
            key == "AGENT_BROWSER_RDP_ROUTE_POOL_JSON" && value.is_empty()
        }));
    }

    #[test]
    fn canonical_route_pool_accepts_dynamic_distinct_displays() {
        let route_pool = vec![
            serde_json::json!({
                "id": "guacamole-rdp-a",
                "routeId": "guacamole:1",
                "target": {"displayName": ":10"}
            }),
            serde_json::json!({
                "id": "guacamole-rdp-b",
                "routeId": "guacamole:2",
                "target": {"displayName": ":11"}
            }),
        ];
        validate_canonical_route_pool(&route_pool).unwrap();
    }

    #[test]
    fn canonical_route_pool_accepts_four_opaque_distinct_routes() {
        let route_pool = (0..4)
            .map(|index| {
                serde_json::json!({
                    "id": format!("route-slot-{index}"),
                    "routeId": format!("guacamole:{}", 20 + index),
                    "target": {"displayName": format!(":{}", 30 + index)}
                })
            })
            .collect::<Vec<_>>();

        validate_canonical_route_pool(&route_pool).unwrap();
    }

    #[test]
    fn canonical_route_pool_rejects_route_or_display_drift() {
        let duplicate_route = vec![
            serde_json::json!({
                "id": "guacamole-rdp-a",
                "routeId": "guacamole:2",
                "target": {"displayName": ":10"}
            }),
            serde_json::json!({
                "id": "guacamole-rdp-b",
                "routeId": "guacamole:2",
                "target": {"displayName": ":11"}
            }),
        ];
        assert!(validate_canonical_route_pool(&duplicate_route)
            .unwrap_err()
            .contains("duplicate routeId"));

        let collapsed_display = vec![
            serde_json::json!({
                "id": "guacamole-rdp-a",
                "routeId": "guacamole:1",
                "target": {"displayName": ":10"}
            }),
            serde_json::json!({
                "id": "guacamole-rdp-b",
                "routeId": "guacamole:2",
                "target": {"displayName": ":10"}
            }),
        ];
        assert!(validate_canonical_route_pool(&collapsed_display)
            .unwrap_err()
            .contains("multiple routes"));
    }

    #[test]
    fn service_reconcile_rejects_active_route_conflicts() {
        let accepted = serde_json::json!({
            "success": true,
            "data": {
                "routePoolRefresh": {
                    "skippedActiveConflictEntryIds": []
                }
            }
        });
        validate_service_reconcile_payload(&accepted).unwrap();

        let conflict = serde_json::json!({
            "success": true,
            "data": {
                "routePoolRefresh": {
                    "skippedActiveConflictEntryIds": ["legacy-route"]
                }
            }
        });
        assert!(validate_service_reconcile_payload(&conflict)
            .unwrap_err()
            .contains("active conflicting route"));

        let rejected = serde_json::json!({"success": false});
        assert!(validate_service_reconcile_payload(&rejected)
            .unwrap_err()
            .contains("rejected"));
    }

    #[test]
    fn service_reconcile_uses_a_transaction_scoped_daemon_session() {
        let transaction_env = vec![(
            crate::runtime_adoption::RUNTIME_ADMISSION_TRANSACTION_ID_ENV.to_string(),
            "upgrade-test-a".to_string(),
        )];
        let same_transaction_env = transaction_env.clone();
        let other_transaction_env = vec![(
            crate::runtime_adoption::RUNTIME_ADMISSION_TRANSACTION_ID_ENV.to_string(),
            "upgrade-test-b".to_string(),
        )];

        let session = workstation_reconcile_session(&transaction_env);
        assert!(session.starts_with("workstation-reconcile-"));
        assert_eq!(
            session,
            workstation_reconcile_session(&same_transaction_env)
        );
        assert_ne!(
            session,
            workstation_reconcile_session(&other_transaction_env)
        );
        assert_eq!(workstation_reconcile_session(&[]), "workstation-reconcile");
    }

    #[test]
    fn protected_secrets_are_private_and_idempotent() {
        let root = env::temp_dir().join(format!(
            "agent-browser-workstation-secret-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let secret_file = root.join("guacamole.env");

        ensure_secret_values(&secret_file).unwrap();
        set_private_file(&secret_file).unwrap();
        let first = fs::read_to_string(&secret_file).unwrap();
        ensure_secret_values(&secret_file).unwrap();
        let second = fs::read_to_string(&secret_file).unwrap();

        assert_eq!(first, second);
        for key in [
            "POSTGRES_PASSWORD",
            "XRDP_AGENT_BROWSER_ROUTE_A_USERNAME",
            "XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD",
            "XRDP_AGENT_BROWSER_ROUTE_B_USERNAME",
            "XRDP_AGENT_BROWSER_ROUTE_B_PASSWORD",
        ] {
            assert_eq!(
                second
                    .lines()
                    .filter(|line| line.starts_with(&format!("{key}=")))
                    .count(),
                1
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&secret_file).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn retained_postgres_password_replaces_drifted_protected_secret() {
        let root = env::temp_dir().join(format!(
            "agent-browser-workstation-postgres-secret-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let secret_file = root.join("guacamole.env");
        fs::write(
            &secret_file,
            "POSTGRES_PASSWORD=stale\nXRDP_AGENT_BROWSER_ROUTE_A_USERNAME=preserved\n",
        )
        .unwrap();
        set_private_file(&secret_file).unwrap();

        assert!(reconcile_protected_secret_value(
            &secret_file,
            "POSTGRES_PASSWORD",
            "retained-container-password",
        )
        .unwrap());
        assert!(!reconcile_protected_secret_value(
            &secret_file,
            "POSTGRES_PASSWORD",
            "retained-container-password",
        )
        .unwrap());

        let contents = fs::read_to_string(&secret_file).unwrap();
        assert!(contents.contains("POSTGRES_PASSWORD=retained-container-password\n"));
        assert!(contents.contains("XRDP_AGENT_BROWSER_ROUTE_A_USERNAME=preserved\n"));
        assert!(!contents.contains("POSTGRES_PASSWORD=stale\n"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&secret_file).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn retained_compose_project_is_used_for_reconcile() {
        let args = guacamole_compose_args(
            Path::new("/state/.env"),
            Path::new("/state/secrets/guacamole.env"),
            Path::new("/support/guacamole/compose.yml"),
            Some("guacamole"),
        );
        assert_eq!(&args[..3], ["compose", "--project-name", "guacamole"]);
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--env-file", "/state/.env"]));
        assert!(args.ends_with(&["up".to_string(), "-d".to_string(), "--wait".to_string()]));

        let fresh = guacamole_compose_args(
            Path::new("/state/.env"),
            Path::new("/state/secrets/guacamole.env"),
            Path::new("/support/guacamole/compose.yml"),
            None,
        );
        assert_eq!(fresh.first().map(String::as_str), Some("compose"));
        assert!(!fresh.iter().any(|value| value == "--project-name"));
    }

    #[test]
    fn guacamole_defaults_extension_packages_text_input_migration() {
        let root = env::temp_dir().join(format!(
            "agent-browser-guacamole-defaults-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();

        materialize_guacamole_defaults_extension(&root).unwrap();
        let bundle = fs::File::open(root.join("extensions/agent-browser-defaults.jar")).unwrap();
        let mut archive = zip::ZipArchive::new(bundle).unwrap();
        let mut manifest = String::new();
        archive
            .by_name("guac-manifest.json")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        assert!(manifest.contains(r#""namespace": "agent-browser-defaults""#));
        drop(archive);

        let bundle = fs::File::open(root.join("extensions/agent-browser-defaults.jar")).unwrap();
        let mut archive = zip::ZipArchive::new(bundle).unwrap();
        let mut script = String::new();
        archive
            .by_name("agent-browser-defaults.js")
            .unwrap()
            .read_to_string(&mut script)
            .unwrap();
        assert!(script.contains("preferences.inputMethod = 'text'"));
        assert!(script.contains("AGENT_BROWSER_GUAC_DEFAULTS_VERSION"));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn materialized_guacamole_bundle_keeps_defaults_extension_sources() {
        let root = env::temp_dir().join(format!(
            "agent-browser-guacamole-bundle-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();

        materialize_guacamole_assets(&root).unwrap();
        for relative in [
            "guacamole/start-guacamole.sh",
            "guacamole/extensions/guac-manifest.json",
            "guacamole/extensions/agent-browser-defaults.js",
            "guacamole/extensions/agent-browser-defaults.jar",
        ] {
            assert!(root.join(relative).is_file(), "missing {relative}");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_ne!(
                fs::metadata(root.join("guacamole/start-guacamole.sh"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o111,
                0
            );
        }

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn workstation_disk_preflight_fails_closed_below_minimum() {
        assert!(!workstation_disk_space_ready(None));
        assert!(!workstation_disk_space_ready(Some(
            MIN_WORKSTATION_FREE_DISK_BYTES - 1
        )));
        assert!(workstation_disk_space_ready(Some(
            MIN_WORKSTATION_FREE_DISK_BYTES
        )));
    }

    #[cfg(target_family = "unix")]
    #[test]
    fn statvfs_available_bytes_accepts_platform_integer_widths_and_saturates() {
        assert_eq!(statvfs_available_bytes(3_u32, 4096_u64), 12_288);
        assert_eq!(statvfs_available_bytes(u64::MAX, 2_u32), u64::MAX);
    }

    #[test]
    fn guacamole_header_user_postcondition_requires_exactly_one_database_user() {
        assert!(guacamole_header_user_ready(b" 1\n"));
        assert!(!guacamole_header_user_ready(b"0\n"));
        assert!(!guacamole_header_user_ready(b"2\n"));
        assert!(!guacamole_header_user_ready(b""));
    }

    #[test]
    fn systemd_reset_only_targets_a_failed_unit() {
        assert_eq!(systemd_unit_is_failed(b"failed\n"), Ok(true));
        assert_eq!(systemd_unit_is_failed(b"inactive\n"), Ok(false));
        assert_eq!(systemd_unit_is_failed(b"unknown\n"), Ok(false));
        assert!(systemd_unit_is_failed(b"").is_err());
    }

    #[test]
    fn systemd_reconcile_quiesce_set_preserves_its_running_service() {
        assert!(WORKSTATION_RECONCILE_QUIESCE_UNITS
            .contains(&"agent-browser-dashboard-backend.service"));
        assert!(!WORKSTATION_RECONCILE_QUIESCE_UNITS.contains(&"agent-browser-dashboard.service"));
        assert!(
            WORKSTATION_RECONCILE_QUIESCE_UNITS.contains(&"agent-browser-runtime-interlock.timer")
        );
        assert!(WORKSTATION_RECONCILE_QUIESCE_UNITS
            .contains(&"agent-browser-guacamole-postgres-backup.timer"));
        assert!(!WORKSTATION_RECONCILE_QUIESCE_UNITS
            .contains(&"agent-browser-runtime-interlock.service"));
    }

    #[test]
    fn reconcile_failure_restores_previously_active_user_units() {
        let restored = std::cell::Cell::new(false);
        let result = complete_reconcile_with_unit_restore::<()>(
            Err("route readiness failed".to_string()),
            || {
                restored.set(true);
                Ok(())
            },
        );

        assert!(restored.get());
        let error = result.unwrap_err();
        assert!(error.contains("route readiness failed"));
        assert!(error.contains("previously active workstation user units were restored"));
    }

    #[test]
    fn reconcile_failure_reports_restoration_failure_without_hiding_original_error() {
        let result = complete_reconcile_with_unit_restore::<()>(
            Err("route readiness failed".to_string()),
            || Err("systemctl start failed".to_string()),
        );

        let error = result.unwrap_err();
        assert!(error.contains("route readiness failed"));
        assert!(error.contains("failed to restore previously active workstation user units"));
        assert!(error.contains("systemctl start failed"));
    }

    #[test]
    fn reconciliation_restoration_returns_units_to_their_exact_prior_state() {
        let snapshot = QuiescedUserUnits::from_states([
            ("agent-browser-dashboard-backend.service", true),
            ("agent-browser-runtime-interlock.timer", true),
            ("agent-browser-guacamole-postgres-backup.timer", false),
        ]);

        assert_eq!(
            snapshot.units_to_start(),
            vec![
                "agent-browser-dashboard-backend.service",
                "agent-browser-runtime-interlock.timer"
            ]
        );
        assert_eq!(
            snapshot.units_to_stop(),
            vec!["agent-browser-guacamole-postgres-backup.timer"]
        );
    }

    #[test]
    fn reconciliation_failure_receipt_is_durable_and_diagnostic() {
        let receipt = workstation_reconcile_failure_receipt("route readiness failed");

        assert_eq!(
            receipt["schemaVersion"],
            "agent-browser.workstation-reconcile-failure.v1"
        );
        assert_eq!(receipt["success"], false);
        assert_eq!(receipt["error"], "route readiness failed");
        assert_eq!(receipt["version"], env!("CARGO_PKG_VERSION"));
        assert!(receipt["recordedAtUnixMs"].as_u64().unwrap() > 0);
    }

    #[test]
    fn workstation_reconcile_accepts_only_proven_advisory_install_doctor_issues() {
        let advisory = serde_json::json!({
            "success": false,
            "data": {
                "issues": [
                    {"code": "service_duplicate_profile_pressure"},
                    {"code": "executable_drift", "severity": "warning"}
                ],
                "sessionSupervisors": {
                    "sessions": [{"activeState": "inactive"}],
                    "issues": [{"code": "executable_drift", "severity": "warning"}]
                }
            }
        });
        assert!(final_doctor_reports_ready(
            "install doctor",
            &advisory,
            false,
            "/success"
        ));

        let active_supervisor = serde_json::json!({
            "success": false,
            "data": {
                "issues": [{"code": "executable_drift", "severity": "warning"}],
                "sessionSupervisors": {
                    "sessions": [{"activeState": "active"}],
                    "issues": [{"code": "executable_drift", "severity": "warning"}]
                }
            }
        });
        assert!(!install_doctor_reports_workstation_ready(
            &active_supervisor
        ));

        let route_blocker = serde_json::json!({
            "success": false,
            "data": {
                "issues": [{"code": "dashboard_runtime_stale_or_unreadable"}],
                "sessionSupervisors": {"sessions": [], "issues": []}
            }
        });
        assert!(!install_doctor_reports_workstation_ready(&route_blocker));

        let remote_view_ready = serde_json::json!({
            "data": {"remoteControl": {"ready": true}}
        });
        assert!(!final_doctor_reports_ready(
            "remote-view doctor",
            &remote_view_ready,
            false,
            "/data/remoteControl/ready"
        ));
    }

    #[test]
    fn doctor_failure_diagnostics_report_only_sorted_issue_codes() {
        let payload = serde_json::json!({
            "data": {
                "issues": [
                    {"code": "workstation_upgrade_transaction_not_terminal", "message": "private detail"},
                    {"code": "dashboard_runtime_stale_or_unreadable", "path": "/private/path"},
                    {"code": "dashboard_runtime_stale_or_unreadable"}
                ]
            }
        });

        assert_eq!(
            doctor_issue_codes(&payload),
            "dashboard_runtime_stale_or_unreadable,workstation_upgrade_transaction_not_terminal"
        );
    }

    #[test]
    fn transaction_validation_accepts_only_its_exact_inflight_doctor_blocker() {
        let root = env::temp_dir().join(format!(
            "agent-browser-upgrade-doctor-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        let mut transaction = new_upgrade_transaction(
            &paths,
            "generation-new".to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        transaction.state = crate::runtime_adoption::UpgradeTransactionState::PostCommitValidating;
        let report = serde_json::json!({
            "success": false,
            "data": {
                "issues": [
                    {
                        "code": "workstation_upgrade_transaction_not_terminal",
                        "state": "post_commit_validating",
                    },
                    {"code": "runtime_monitor_not_ready"},
                    {"code": "runtime_pressure_ownership_unknown"},
                    {"code": "service_duplicate_profile_pressure"},
                ],
                "serviceResources": {
                    "available": true,
                    "candidateCount": 0,
                    "readinessImpactingCandidates": 0,
                },
                "sessionSupervisors": {"sessions": [], "issues": []},
                "liveDashboardRuntime": {
                    "workstationUpgrade": {
                        "selectedGenerationId": "generation-new",
                        "admissionDraining": true,
                        "latestTransaction": {
                            "transactionId": transaction.transaction_id.clone(),
                            "state": "post_commit_validating",
                        }
                    }
                }
            }
        });

        assert!(install_doctor_reports_expected_upgrade_ready(
            &report,
            &transaction,
            &[]
        ));
        assert!(!install_doctor_reports_workstation_ready(&report));

        let mut reclaimable = report.clone();
        reclaimable["data"]["serviceResources"]["candidateCount"] = Value::from(1);
        assert!(!install_doctor_reports_expected_upgrade_ready(
            &reclaimable,
            &transaction,
            &[]
        ));

        let mut wrong = report;
        wrong["data"]["liveDashboardRuntime"]["workstationUpgrade"]["latestTransaction"]
            ["transactionId"] = Value::String("another-transaction".to_string());
        assert!(!install_doctor_reports_expected_upgrade_ready(
            &wrong,
            &transaction,
            &[]
        ));
    }

    #[test]
    fn transaction_validation_allows_only_exact_transitional_source_sessions() {
        let root = env::temp_dir().join(format!(
            "agent-browser-upgrade-source-doctor-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        let mut transaction = new_upgrade_transaction(
            &paths,
            "generation-new".to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        transaction.state = crate::runtime_adoption::UpgradeTransactionState::PostCommitValidating;
        let report = serde_json::json!({
            "success": false,
            "data": {
                "issues": [
                    {"code": "workstation_upgrade_transaction_not_terminal"},
                    {"code": "active_runtime_stale_executable", "session": "source-owner"},
                ],
                "sessionSupervisors": {"sessions": [], "issues": []},
                "liveDashboardRuntime": {
                    "workstationUpgrade": {
                        "selectedGenerationId": "generation-new",
                        "admissionDraining": true,
                        "latestTransaction": {
                            "transactionId": transaction.transaction_id.clone(),
                            "state": "post_commit_validating",
                        }
                    }
                }
            }
        });

        assert!(install_doctor_reports_expected_upgrade_ready(
            &report,
            &transaction,
            &["source-owner".to_string()]
        ));
        assert!(!install_doctor_reports_expected_upgrade_ready(
            &report,
            &transaction,
            &["another-source".to_string()]
        ));

        let remote_report = serde_json::json!({
            "data": {
                "install": {"data": report},
                "remoteControl": {
                    "ready": false,
                    "rdpGatewayReady": true,
                    "privateDisplayAllocatorReady": true,
                    "routePoolReady": true,
                    "routeUrlReady": true,
                    "routeDisplayReady": true,
                    "routeDisplayAccessReady": true,
                    "browserLaunchReady": true,
                }
            }
        });
        assert!(remote_view_doctor_reports_expected_upgrade_ready(
            &remote_report,
            &transaction,
            &["source-owner".to_string()]
        ));
        let mut route_blocked = remote_report;
        route_blocked["data"]["remoteControl"]["routePoolReady"] = Value::Bool(false);
        assert!(!remote_view_doctor_reports_expected_upgrade_ready(
            &route_blocked,
            &transaction,
            &["source-owner".to_string()]
        ));
    }

    #[test]
    fn transaction_validation_includes_only_exact_manual_preservation_sessions() {
        let handoffs = vec![PreparedRuntimeHandoff {
            source_session: "cooperative-source".to_string(),
            candidate_session: "cooperative-candidate".to_string(),
            source_process_identity: None,
            mode: crate::runtime_adoption::BrowserAdoptionMode::CooperativeTransfer,
            committed: true,
            source_finalized: false,
            irreversible_source_revocation: false,
        }];
        let migrations = vec![
            runtime_migration("session:manual-source"),
            runtime_migration("browser-without-session-prefix"),
        ];

        assert_eq!(
            permitted_stale_source_sessions(&handoffs, &migrations),
            vec![
                "cooperative-source".to_string(),
                "manual-source".to_string(),
            ]
        );

        let finalized_handoffs = vec![PreparedRuntimeHandoff {
            source_finalized: true,
            ..handoffs.into_iter().next().unwrap()
        }];
        assert_eq!(
            permitted_stale_source_sessions(&finalized_handoffs, &migrations),
            vec!["manual-source".to_string()]
        );
    }

    #[test]
    fn transaction_validation_accepts_only_the_proven_shadow_dashboard_transition() {
        let root = env::temp_dir().join(format!(
            "agent-browser-upgrade-shadow-dashboard-doctor-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = install_paths(&root);
        let mut transaction = new_upgrade_transaction(
            &paths,
            "generation-new".to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        transaction.state = crate::runtime_adoption::UpgradeTransactionState::PostCommitValidating;
        let report = serde_json::json!({
            "success": false,
            "data": {
                "issues": [
                    {"code": "workstation_upgrade_transaction_not_terminal"},
                    {"code": "dashboard_runtime_stale_or_unreadable"},
                ],
                "sessionSupervisors": {"sessions": [], "issues": []},
                "liveDashboardRuntime": {
                    "workstationUpgrade": {
                        "selectedGenerationId": "generation-new",
                        "admissionDraining": true,
                        "dashboardIngress": {
                            "dashboardIngressReady": true,
                            "operatorJourneyReady": true,
                            "selectedBackend": {"generationId": "generation-new"},
                            "presentationReceipt": {
                                "dashboardDeploymentGeneration": "generation-new",
                                "state": "ready",
                                "receiptId": "presentation-ready",
                            }
                        },
                        "latestTransaction": {
                            "transactionId": transaction.transaction_id.clone(),
                            "state": "post_commit_validating",
                        }
                    }
                }
            }
        });

        assert!(install_doctor_reports_expected_upgrade_ready(
            &report,
            &transaction,
            &[]
        ));

        let mut journey_missing = report.clone();
        journey_missing["data"]["liveDashboardRuntime"]["workstationUpgrade"]["dashboardIngress"]
            ["operatorJourneyReady"] = Value::Bool(false);
        assert!(!install_doctor_reports_expected_upgrade_ready(
            &journey_missing,
            &transaction,
            &[]
        ));

        let mut wrong_generation = report;
        wrong_generation["data"]["liveDashboardRuntime"]["workstationUpgrade"]
            ["dashboardIngress"]["selectedBackend"]["generationId"] =
            Value::String("another-generation".to_string());
        assert!(!install_doctor_reports_expected_upgrade_ready(
            &wrong_generation,
            &transaction,
            &[]
        ));
    }

    #[test]
    fn runtime_transaction_failure_classifies_only_handoff_protocol_absence() {
        let typed = serde_json::json!({"type": "unknown_command"});
        assert_eq!(
            runtime_transaction_failure_kind(Some(&typed), "", &["handoff", "prepare"]),
            RuntimeTransactionCommandFailureKind::ProtocolUnavailable
        );
        assert_eq!(
            runtime_transaction_failure_kind(
                None,
                "Unknown subcommand 'handoff'",
                &["handoff", "prepare"]
            ),
            RuntimeTransactionCommandFailureKind::ProtocolUnavailable
        );
        assert_eq!(
            runtime_transaction_failure_kind(Some(&typed), "", &["service", "resource", "list"]),
            RuntimeTransactionCommandFailureKind::CommandFailed
        );
        let runtime_failure = serde_json::json!({"type": "cdp_error"});
        assert_eq!(
            runtime_transaction_failure_kind(
                Some(&runtime_failure),
                "handoff target verification failed",
                &["handoff", "prepare"]
            ),
            RuntimeTransactionCommandFailureKind::CommandFailed
        );
        assert_eq!(
            runtime_transaction_failure_kind(
                Some(&runtime_failure),
                "runtime_owner_current_evidence_mismatch: existing profile owner does not match the preparing daemon",
                &["handoff", "prepare"]
            ),
            RuntimeTransactionCommandFailureKind::LegacyTransferredOwnerRejected
        );
        assert_eq!(
            runtime_transaction_failure_kind(
                Some(&runtime_failure),
                "runtime_owner_current_evidence_mismatch: existing profile owner does not match the preparing daemon",
                &["handoff", "resume"]
            ),
            RuntimeTransactionCommandFailureKind::CommandFailed
        );
        assert_eq!(
            runtime_transaction_failure_kind(
                Some(&runtime_failure),
                "runtime_owner_observation_only: candidate cannot issue browser effects before owner compare-and-swap",
                &["handoff", "prepare"]
            ),
            RuntimeTransactionCommandFailureKind::ObservationOnlyAlias
        );
        assert_eq!(
            runtime_transaction_failure_kind(
                Some(&runtime_failure),
                "runtime_owner_observation_only: candidate cannot issue browser effects before owner compare-and-swap",
                &["handoff", "resume"]
            ),
            RuntimeTransactionCommandFailureKind::CommandFailed
        );
        assert_eq!(
            runtime_transaction_failure_kind(
                Some(&runtime_failure),
                "Cannot prepare runtime handoff for session 'historical-alias': browser PID is unavailable",
                &["handoff", "prepare"]
            ),
            RuntimeTransactionCommandFailureKind::BrowserUnavailableAlias
        );
        assert_eq!(
            runtime_transaction_failure_kind(
                Some(&runtime_failure),
                "browser PID is unavailable",
                &["handoff", "resume"]
            ),
            RuntimeTransactionCommandFailureKind::CommandFailed
        );
    }

    #[test]
    fn only_an_exact_transferred_owner_can_use_legacy_prepare_rejection_fallback() {
        use crate::native::service_model::ServiceState;
        use crate::runtime_adoption::{
            RuntimeClassification, RuntimeDisposition, RuntimeMigrationRecord,
        };
        use crate::runtime_owner_transfer::{ProfileOwner, ProfileOwnerState};

        let migration = RuntimeMigrationRecord {
            logical_browser_id: "session:logical-browser".to_string(),
            session_names: vec!["logical-browser".to_string()],
            profile_identity_digest: "profile-digest".to_string(),
            classification: RuntimeClassification::CooperativeLiveOwner,
            disposition: RuntimeDisposition::CooperativeTransfer,
            adoption_receipt_id: None,
            reason_codes: Vec::new(),
        };
        let owner = ProfileOwner {
            owner_id: "owner-transfer-issued".to_string(),
            profile_identity_digest: migration.profile_identity_digest.clone(),
            state: ProfileOwnerState::Ready,
            owner_generation: 2,
            browser_id: migration.logical_browser_id.clone(),
            daemon_session_route: "handoff-candidate".to_string(),
            process_instance_digest: "process-digest".to_string(),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: "cdp-digest".to_string(),
            target_set_digest: "targets-digest".to_string(),
            pending_transfer: None,
            last_transition: None,
        };
        let mut service_state = ServiceState::default();
        service_state
            .runtime_owner_registry
            .owners
            .insert(migration.profile_identity_digest.clone(), owner.clone());

        assert!(legacy_transferred_owner_prepare_rejection_can_fallback(
            &service_state,
            &migration,
            "handoff-candidate",
        ));
        assert!(!legacy_transferred_owner_prepare_rejection_can_fallback(
            &service_state,
            &migration,
            "different-route",
        ));

        let mut generation_one_state = service_state;
        generation_one_state
            .runtime_owner_registry
            .owners
            .get_mut(&migration.profile_identity_digest)
            .unwrap()
            .owner_generation = 1;
        assert!(!legacy_transferred_owner_prepare_rejection_can_fallback(
            &generation_one_state,
            &migration,
            "handoff-candidate",
        ));
    }

    #[test]
    fn ownerless_bootstrap_requires_legacy_v1_and_explicit_census_reason() {
        let mut migration = runtime_migration("session:legacy-ownerless");
        migration.classification =
            crate::runtime_adoption::RuntimeClassification::CooperativeLiveOwner;
        migration.disposition = crate::runtime_adoption::RuntimeDisposition::CooperativeTransfer;
        migration.reason_codes = vec!["cooperative_owner_registration_required".to_string()];

        assert!(legacy_ownerless_orphan_bootstrap_allowed(
            &migration, false, true
        ));
        assert!(!legacy_ownerless_orphan_bootstrap_allowed(
            &migration, false, false
        ));
        assert!(!legacy_ownerless_orphan_bootstrap_allowed(
            &migration, true, true
        ));
        migration.reason_codes = vec!["cooperative_owner_verified".to_string()];
        assert!(!legacy_ownerless_orphan_bootstrap_allowed(
            &migration, false, true
        ));
    }

    #[test]
    fn legacy_prepare_payload_distinguishes_browserless_aliases_from_retained_browsers() {
        let browserless = serde_json::json!({
            "success": true,
            "data": {"prepared": false, "browserPresent": false, "sessionName": "alias"}
        });
        assert_eq!(
            runtime_handoff_prepare_response_kind(&browserless),
            RuntimeHandoffPrepareResponseKind::NoBrowser
        );

        let legacy = serde_json::json!({
            "success": true,
            "data": {"prepared": true, "browserPresent": true, "sessionName": "legacy"}
        });
        assert_eq!(
            runtime_handoff_prepare_response_kind(&legacy),
            RuntimeHandoffPrepareResponseKind::LegacyBrowser
        );

        let cooperative = serde_json::json!({
            "success": true,
            "data": {
                "prepared": true,
                "browserPresent": true,
                "sessionName": "current",
                "candidateSessionName": "candidate"
            }
        });
        assert_eq!(
            runtime_handoff_prepare_response_kind(&cooperative),
            RuntimeHandoffPrepareResponseKind::Cooperative
        );
    }

    #[test]
    fn legacy_browser_alias_candidates_include_all_sessions_bound_to_the_logical_browser() {
        use crate::native::service_model::{BrowserSession, ServiceState};

        let mut service_state = ServiceState::default();
        for session_id in ["p116-beta", "p116-beta-daemon"] {
            service_state.sessions.insert(
                session_id.to_string(),
                BrowserSession {
                    id: session_id.to_string(),
                    browser_ids: vec!["session:p116-beta".to_string()],
                    ..BrowserSession::default()
                },
            );
        }

        assert_eq!(
            alternate_runtime_source_sessions(&service_state, "session:p116-beta", "p116-beta",),
            vec!["p116-beta-daemon".to_string()]
        );
    }

    #[test]
    fn runtime_handoff_prepare_retires_browserless_alias_after_selecting_primary() {
        let candidates = vec!["primary".to_string(), "historical-alias".to_string()];
        let mut prepared_sessions = Vec::new();
        let mut retired_sessions = Vec::new();

        let (selected_session, payload, retired_aliases) = prepare_runtime_handoff_candidates(
            "session:logical-browser",
            "primary",
            candidates,
            |session| {
                prepared_sessions.push(session.to_string());
                Ok(if session == "primary" {
                    serde_json::json!({
                        "data": {
                            "prepared": true,
                            "browserPresent": true,
                            "candidateSessionName": "candidate"
                        }
                    })
                } else {
                    serde_json::json!({
                        "data": {"prepared": false, "browserPresent": false}
                    })
                })
            },
            |session| {
                retired_sessions.push(session.to_string());
                Ok(())
            },
        )
        .expect("one browser-bearing session should be selected");

        assert_eq!(selected_session, "primary");
        assert_eq!(
            payload.pointer("/data/candidateSessionName"),
            Some(&serde_json::json!("candidate"))
        );
        assert_eq!(prepared_sessions, vec!["primary", "historical-alias"]);
        assert_eq!(retired_sessions, vec!["historical-alias"]);
        assert_eq!(retired_aliases, vec!["historical-alias"]);
    }

    #[test]
    fn runtime_handoff_prepare_retires_legacy_browser_unavailable_alias_after_owner_selection() {
        let candidates = vec![
            "current-owner-route".to_string(),
            "historical-alias".to_string(),
        ];
        let mut retired_sessions = Vec::new();

        let (selected_session, _, retired_aliases) = prepare_runtime_handoff_candidates(
            "session:last30days-facebook--last30days-facebook",
            "current-owner-route",
            candidates,
            |session| {
                if session == "current-owner-route" {
                    Ok(serde_json::json!({
                        "data": {
                            "prepared": true,
                            "browserPresent": true,
                            "candidateSessionName": "candidate"
                        }
                    }))
                } else {
                    Err(RuntimeTransactionCommandFailure {
                        kind: RuntimeTransactionCommandFailureKind::BrowserUnavailableAlias,
                        message: "Cannot prepare runtime handoff: browser PID is unavailable"
                            .to_string(),
                    })
                }
            },
            |session| {
                retired_sessions.push(session.to_string());
                Ok(())
            },
        )
        .expect("the current owner route must supersede a browserless historical alias");

        assert_eq!(selected_session, "current-owner-route");
        assert_eq!(retired_sessions, vec!["historical-alias"]);
        assert_eq!(retired_aliases, vec!["historical-alias"]);
    }

    #[test]
    fn runtime_handoff_prepare_rejects_browser_unavailable_primary() {
        let (_, error) = prepare_runtime_handoff_candidates(
            "session:logical-browser",
            "owner-route",
            vec!["owner-route".to_string()],
            |_| {
                Err(RuntimeTransactionCommandFailure {
                    kind: RuntimeTransactionCommandFailureKind::BrowserUnavailableAlias,
                    message: "browser PID is unavailable".to_string(),
                })
            },
            |_| Ok(()),
        )
        .expect_err("the authoritative source route must have a browser PID");

        assert_eq!(
            error.kind,
            RuntimeTransactionCommandFailureKind::BrowserUnavailableAlias
        );
    }

    #[test]
    fn runtime_handoff_prepare_rejects_second_browser_bearing_alias() {
        let candidates = vec!["primary".to_string(), "conflicting-alias".to_string()];

        let (_, error) = prepare_runtime_handoff_candidates(
            "session:logical-browser",
            "primary",
            candidates,
            |session| {
                Ok(serde_json::json!({
                    "data": {
                        "prepared": true,
                        "browserPresent": true,
                        "candidateSessionName": format!("candidate-{session}")
                    }
                }))
            },
            |_| Ok(()),
        )
        .expect_err("multiple browser-bearing aliases must fail closed");

        assert_eq!(
            error.message,
            "runtime_handoff_alternate_browser_conflict:session:logical-browser:primary:conflicting-alias"
        );
    }

    #[test]
    fn runtime_handoff_prepare_retires_observation_only_alias_after_owner_selection() {
        let candidates = vec!["owner-route".to_string(), "stale-alias".to_string()];
        let mut retired_sessions = Vec::new();

        let (selected_session, _, retired_aliases) = prepare_runtime_handoff_candidates(
            "session:logical-browser",
            "owner-route",
            candidates,
            |session| {
                if session == "owner-route" {
                    Ok(serde_json::json!({
                        "data": {
                            "prepared": true,
                            "browserPresent": true,
                            "candidateSessionName": "candidate"
                        }
                    }))
                } else {
                    Err(RuntimeTransactionCommandFailure {
                        kind: RuntimeTransactionCommandFailureKind::ObservationOnlyAlias,
                        message: "runtime_owner_observation_only".to_string(),
                    })
                }
            },
            |session| {
                retired_sessions.push(session.to_string());
                Ok(())
            },
        )
        .expect("an observation-only non-owner alias should be retired");

        assert_eq!(selected_session, "owner-route");
        assert_eq!(retired_sessions, vec!["stale-alias"]);
        assert_eq!(retired_aliases, vec!["stale-alias"]);
    }

    #[test]
    fn runtime_handoff_prepare_retires_rejected_transferred_alias_after_owner_selection() {
        let candidates = vec!["owner-route".to_string(), "transferred-alias".to_string()];
        let mut retired_sessions = Vec::new();

        let (selected_session, _, retired_aliases) = prepare_runtime_handoff_candidates(
            "session:logical-browser",
            "owner-route",
            candidates,
            |session| {
                if session == "owner-route" {
                    Ok(serde_json::json!({
                        "data": {
                            "prepared": true,
                            "browserPresent": true,
                            "candidateSessionName": "candidate"
                        }
                    }))
                } else {
                    Err(RuntimeTransactionCommandFailure {
                        kind: RuntimeTransactionCommandFailureKind::LegacyTransferredOwnerRejected,
                        message: "runtime_owner_current_evidence_mismatch".to_string(),
                    })
                }
            },
            |session| {
                retired_sessions.push(session.to_string());
                Ok(())
            },
        )
        .expect("a rejected non-owner transfer alias should be retired");

        assert_eq!(selected_session, "owner-route");
        assert_eq!(retired_sessions, vec!["transferred-alias"]);
        assert_eq!(retired_aliases, vec!["transferred-alias"]);
    }

    #[test]
    fn runtime_handoff_prepare_rejects_observation_only_primary() {
        let (_, error) = prepare_runtime_handoff_candidates(
            "session:logical-browser",
            "owner-route",
            vec!["owner-route".to_string()],
            |_| {
                Err(RuntimeTransactionCommandFailure {
                    kind: RuntimeTransactionCommandFailureKind::ObservationOnlyAlias,
                    message: "runtime_owner_observation_only".to_string(),
                })
            },
            |_| Ok(()),
        )
        .expect_err("the authoritative source route must remain effect-capable");

        assert_eq!(
            error.kind,
            RuntimeTransactionCommandFailureKind::ObservationOnlyAlias
        );
    }

    #[test]
    fn orphan_handoffs_skip_source_finalize_and_legacy_revocation_requires_recovery() {
        use crate::runtime_adoption::BrowserAdoptionMode;

        let cooperative = PreparedRuntimeHandoff {
            source_session: "source".to_string(),
            candidate_session: "candidate".to_string(),
            source_process_identity: None,
            mode: BrowserAdoptionMode::CooperativeTransfer,
            committed: true,
            source_finalized: false,
            irreversible_source_revocation: false,
        };
        assert!(cooperative.should_finalize_source());
        assert!(!cooperative.rollback_requires_operator_recovery());

        let finalized = PreparedRuntimeHandoff {
            source_finalized: true,
            ..cooperative
        };
        assert!(finalized.rollback_requires_operator_recovery());

        let orphan = PreparedRuntimeHandoff {
            mode: BrowserAdoptionMode::OrphanAdoption,
            source_finalized: false,
            ..finalized
        };
        assert!(!orphan.should_finalize_source());
        assert!(!orphan.rollback_requires_operator_recovery());

        let legacy = PreparedRuntimeHandoff {
            irreversible_source_revocation: true,
            ..orphan
        };
        assert!(!legacy.should_finalize_source());
        assert!(legacy.rollback_requires_operator_recovery());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn legacy_daemon_revocation_kills_only_the_exact_recorded_process() {
        let root = env::temp_dir().join(format!(
            "agent-browser-legacy-daemon-revocation-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let old_binary = root.join("old-agent-browser");
        let wrong_binary = root.join("wrong-agent-browser");
        let browser_binary = root.join("retained-browser");
        fs::copy("/bin/sleep", &old_binary).unwrap();
        fs::copy("/bin/true", &wrong_binary).unwrap();
        fs::copy("/bin/sleep", &browser_binary).unwrap();

        let mut daemon = Command::new(&old_binary).arg("30").spawn().unwrap();
        let daemon_identity = (0..50)
            .find_map(|_| {
                let identity = crate::process_identity::capture_process_identity(
                    daemon.id(),
                    Some(&old_binary),
                    None,
                );
                if identity.is_none() {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                identity
            })
            .expect("copied daemon fixture should reach its expected executable identity");
        let mut browser = Command::new(&browser_binary).arg("30").spawn().unwrap();

        let mut source_authority_unavailable = false;
        let wrong_hashes =
            std::collections::BTreeSet::from([workstation_file_sha256(&wrong_binary).unwrap()]);
        let mismatch = revoke_verified_legacy_daemon_process(
            &daemon_identity,
            &wrong_hashes,
            std::time::Duration::from_secs(1),
            &mut source_authority_unavailable,
        )
        .unwrap_err();
        assert_eq!(mismatch, "legacy_daemon_executable_provenance_mismatch");
        assert!(!source_authority_unavailable);
        assert!(daemon.try_wait().unwrap().is_none());
        assert!(browser.try_wait().unwrap().is_none());

        let authorized_hashes =
            std::collections::BTreeSet::from([workstation_file_sha256(&old_binary).unwrap()]);
        revoke_verified_legacy_daemon_process(
            &daemon_identity,
            &authorized_hashes,
            std::time::Duration::from_secs(2),
            &mut source_authority_unavailable,
        )
        .unwrap();
        assert!(source_authority_unavailable);
        assert!(daemon.wait().unwrap().code().is_none());
        assert!(browser.try_wait().unwrap().is_none());

        browser.kill().unwrap();
        browser.wait().unwrap();
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn identityless_legacy_daemon_can_be_registry_fenced_only_when_unreachable() {
        let missing = |_session: &str| {
            Err::<crate::process_identity::RecordedProcessIdentity, _>(
                "daemon process identity is unavailable".to_string(),
            )
        };

        assert!(
            legacy_daemon_identity_for_revocation("stale-lane", missing, |_| false)
                .unwrap()
                .is_none()
        );

        let error =
            legacy_daemon_identity_for_revocation("live-lane", missing, |_| true).unwrap_err();
        assert!(error.starts_with("legacy_daemon_identity_unavailable_while_reachable:live-lane:"));
    }

    #[test]
    fn retained_generation_hash_authorizes_an_identical_daemon_binary_copy() {
        let root = env::temp_dir().join(format!(
            "agent-browser-retained-generation-provenance-{}",
            uuid::Uuid::new_v4()
        ));
        let generations_dir = root.join("generations");
        let generation_id = "generation-a";
        let generation = generations_dir.join(generation_id);
        let binary = generation.join("bin/agent-browser");
        let copied_binary = root.join("workspace-agent-browser");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::copy("/bin/true", &binary).unwrap();
        fs::copy(&binary, &copied_binary).unwrap();
        let binary_sha256 = workstation_file_sha256(&binary).unwrap();
        fs::write(
            generation.join("generation.json"),
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": "agent-browser.runtime-generation.v1",
                "generationId": generation_id,
                "binarySha256": binary_sha256,
            }))
            .unwrap(),
        )
        .unwrap();
        seal_generation_tree(&generation).unwrap();

        let hashes = authorized_runtime_generation_binary_hashes(&generations_dir).unwrap();
        assert!(hashes.contains(&workstation_file_sha256(&copied_binary).unwrap()));

        remove_generation_tree(&generation).unwrap();
        fs::remove_dir_all(&root).unwrap();
    }
}
