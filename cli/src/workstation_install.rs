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
//! capacity before sudo or payload mutation begins. Failed reconciliation restores the
//! exact prior active state of managed user units and writes a private
//! diagnostic receipt.

use serde::Serialize;
use serde_json::Value;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Output, Stdio};

const INSTALL_SCHEMA_VERSION: &str = "agent-browser.workstation-install.v1";
const DEFAULT_DASHBOARD_PORT: u16 = 4848;
const DEFAULT_GUACAMOLE_PORT: u16 = 8092;
const MIN_WORKSTATION_FREE_DISK_BYTES: u64 = 6 * 1024 * 1024 * 1024;
// A reconcile may run as agent-browser-runtime-interlock.service, so stopping
// that service here would terminate the active reconciler before reactivation.
const WORKSTATION_RECONCILE_QUIESCE_UNITS: [&str; 3] = [
    "agent-browser-dashboard.service",
    "agent-browser-runtime-interlock.timer",
    "agent-browser-guacamole-postgres-backup.timer",
];
const WORKSTATION_GENERATION_UNITS: [&str; 5] = [
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
const ENSURE_POSTGRES_SCRIPT: &str = include_str!("../../scripts/ensure-rdp-guac-postgres.sh");
const POSTGRES_DURABILITY_SCRIPT: &str =
    include_str!("../../scripts/guacamole-postgres-durability.sh");
const SYNC_ROUTE_POOL_SCRIPT: &str =
    include_str!("../../scripts/sync-rdp-guac-route-specific-user-pool.sh");
const GRANT_ROUTE_DISPLAY_ACCESS_SCRIPT: &str =
    include_str!("../../scripts/grant-rdp-route-display-access.sh");
const CONTROLLER_PACKAGE_JSON: &str = "{\n  \"private\": true,\n  \"type\": \"module\"\n}\n";
const CONTROLLER_ASSETS: [(&str, &str, bool); 10] = [
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
        _ => run_workstation_install(args),
    }
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
        fail(
            &format!(
                "workstation installation requires at least {} bytes of free disk space; {} bytes are available",
                host_plan.minimum_disk_bytes,
                host_plan.available_disk_bytes.unwrap_or(0)
            ),
            parsed.json,
        );
    }
    if !host_plan.supported {
        fail(
            "workstation installation requires Ubuntu 24.04 x86_64 with apt-get, apt-cache, bash, sudo, and systemctl",
            parsed.json,
        );
    }
    let _install_lock = if parsed.mode == InstallMode::Apply {
        match WorkstationLock::acquire(&root) {
            Ok(lock) => Some(lock),
            Err(error) => fail(&error, parsed.json),
        }
    } else {
        None
    };
    let mut apply_quiesced_user_units = None;
    let mut runtime_census_transaction = None;

    if parsed.mode == InstallMode::Apply && !isolated_root {
        match require_stable_runtime_census(&root, &paths, &parsed) {
            Ok(path) => runtime_census_transaction = Some(path.display().to_string()),
            Err(error) => fail(&error, parsed.json),
        }
        phases.push("runtime-census-stable");
    }

    let mutated = if parsed.mode == InstallMode::Apply {
        if !isolated_root {
            match quiesce_existing_user_units(&paths) {
                Ok(quiesced) => apply_quiesced_user_units = Some(quiesced),
                Err(error) => fail(&error, parsed.json),
            }
        }
        match materialize_payload(&paths, &parsed) {
            Ok(()) => {
                phases.extend(["payload-staged", "units-staged", "payload-committed"]);
                paths = install_paths(&root);
                true
            }
            Err(error) => fail_with_user_unit_restoration(
                &error,
                parsed.json,
                &paths,
                apply_quiesced_user_units.as_ref(),
            ),
        }
    } else {
        false
    };
    let mut host_prepared = false;
    let mut session_refresh_required = false;
    let mut reconcile_receipt = None;
    let mut workstation_ready = false;
    let mut next_action =
        "workstation substrate provisioning is required before service activation".to_string();
    if parsed.mode == InstallMode::Apply && !isolated_root {
        if let Err(error) = crate::install::install_remote_view_privileges(true, parsed.json) {
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
        next_action = if session_refresh_required {
            "log out and back in or reboot, then rerun workstation installation".to_string()
        } else {
            let reconcile = match reconcile_workstation_locked(&root, &paths) {
                Ok(reconcile) => reconcile,
                Err(error) => fail_with_user_unit_restoration(
                    &error,
                    parsed.json,
                    &paths,
                    apply_quiesced_user_units.as_ref(),
                ),
            };
            phases.push("workstation-reconciled");
            workstation_ready = true;
            reconcile_receipt = Some(reconcile.receipt_path);
            "run agent-browser install doctor --json and review the installed workstation state"
                .to_string()
        };
    }
    if session_refresh_required {
        if let Some(quiesced) = apply_quiesced_user_units.as_ref() {
            if let Err(error) =
                restore_previously_active_user_units(&paths, &paths.root, &[], quiesced)
            {
                fail(
                    &format!(
                        "workstation installation requires a fresh login; failed to restore previously active workstation user units: {error}"
                    ),
                    parsed.json,
                );
            }
        }
    }

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
    match reconcile_workstation() {
        Ok(report) => {
            print_reconcile_report(&report, json);
        }
        Err(error) => fail(&error, json),
    }
}

fn reconcile_workstation() -> Result<WorkstationReconcileReport, String> {
    if !cfg!(target_os = "linux") {
        return Err("workstation reconciliation is only supported on Linux".to_string());
    }
    let root = workstation_root()?;
    let paths = install_paths(&root);
    let _lock = WorkstationLock::acquire(&root)?;
    match reconcile_workstation_locked(&root, &paths) {
        Ok(report) => Ok(report),
        Err(error) => {
            let receipt_path =
                root.join(".agent-browser/convergence/workstation-last-failure.json");
            let receipt = workstation_reconcile_failure_receipt(&error);
            match write_private_json(&receipt_path, &receipt) {
                Ok(()) => Err(format!(
                    "{error}; failure receipt: {}",
                    receipt_path.display()
                )),
                Err(receipt_error) => Err(format!(
                    "{error}; failed to write workstation reconciliation failure receipt: {receipt_error}"
                )),
            }
        }
    }
}

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

fn reconcile_workstation_locked(
    root: &Path,
    paths: &InstallPaths,
) -> Result<WorkstationReconcileReport, String> {
    require_installed_payload(paths)?;
    require_effective_groups()?;
    let support_root = &paths.support_dir;
    let command_env = workstation_command_env(paths);
    let quiesced_user_units = quiesce_existing_user_units(paths)?;
    let reconcile_result =
        reconcile_workstation_after_quiesce(root, paths, support_root, command_env.clone());
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

    verify_final_doctors(paths, support_root, &command_env)?;
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

fn print_reconcile_report(report: &WorkstationReconcileReport, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(report)
                .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        );
    } else {
        println!("Workstation reconciliation complete.");
        println!("  Version: {}", report.version);
        println!("  Routes: {}", report.route_pool.len());
        println!("  Receipt: {}", report.receipt_path);
    }
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
    if route_pool.len() != 2 {
        return Err(format!(
            "Canonical route pool must contain exactly two routes, found {}",
            route_pool.len()
        ));
    }
    let expected = [
        ("guacamole-rdp-a", "guacamole:1"),
        ("guacamole-rdp-b", "guacamole:2"),
    ];
    let mut displays = Vec::new();
    for (id, route_id) in expected {
        let route = route_pool
            .iter()
            .find(|route| route.get("id").and_then(Value::as_str) == Some(id))
            .ok_or_else(|| format!("Canonical route pool is missing {id}"))?;
        if route.get("routeId").and_then(Value::as_str) != Some(route_id) {
            return Err(format!(
                "{id} did not resolve to canonical route {route_id}"
            ));
        }
        let display = route
            .pointer("/target/displayName")
            .and_then(Value::as_str)
            .filter(|display| !display.is_empty())
            .ok_or_else(|| format!("{id} is missing a selected route display"))?;
        displays.push(display.to_string());
    }
    if displays[0] == displays[1] {
        return Err("Canonical route pool resolved both routes to one display".to_string());
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
    let output = run_status(
        binary
            .to_str()
            .ok_or_else(|| "invalid installed agent-browser path".to_string())?,
        &[
            "--json",
            "service",
            "reconcile",
            "--authoritative-route-pool-json",
            &route_json,
        ],
        support_root,
        command_env,
        false,
    )?;
    let payload: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Service reconcile JSON parse failed: {error}"))?;
    validate_service_reconcile_payload(&payload)
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
    let route_a = route_pool
        .iter()
        .find(|route| route.get("id").and_then(Value::as_str) == Some("guacamole-rdp-a"))
        .ok_or_else(|| "Canonical route A is missing".to_string())?;
    let route_b = route_pool
        .iter()
        .find(|route| route.get("id").and_then(Value::as_str) == Some("guacamole-rdp-b"))
        .ok_or_else(|| "Canonical route B is missing".to_string())?;
    let route_url = route_a
        .pointer("/routeDescriptor/localEmbedUrl")
        .or_else(|| route_a.get("frameUrl"))
        .and_then(Value::as_str)
        .ok_or_else(|| "Canonical route A is missing a local operator URL".to_string())?;
    let display_a = route_a
        .pointer("/target/displayName")
        .and_then(Value::as_str)
        .ok_or_else(|| "Canonical route A is missing a display".to_string())?;
    let display_b = route_b
        .pointer("/target/displayName")
        .and_then(Value::as_str)
        .ok_or_else(|| "Canonical route B is missing a display".to_string())?;
    let header_user = env::var("USER").unwrap_or_else(|_| "agent-browser".to_string());
    upsert_env_values(
        &root.join(".agent-browser/.env"),
        &[
            ("AGENT_BROWSER_REMOTE_VIEW_PROVIDER", "rdp_gateway"),
            ("AGENT_BROWSER_REMOTE_VIEW_URL", route_url),
            ("AGENT_BROWSER_GUACAMOLE_HEADER_USER", &header_user),
            ("AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME", display_a),
            ("AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME", display_b),
        ],
    )
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
        let ready =
            final_doctor_reports_ready(label, &payload, output.status.success(), readiness_pointer);
        if !ready {
            return Err(format!(
                "{label} did not report ready (status {})",
                output.status
            ));
        }
    }
    Ok(())
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
            || (inactive_supervisors && supervisor_issues.contains(issue))
    })
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

fn require_stable_runtime_census(
    root: &Path,
    paths: &InstallPaths,
    args: &WorkstationInstallArgs,
) -> Result<PathBuf, String> {
    require_stable_runtime_census_with(
        root,
        paths,
        args,
        crate::runtime_adoption::collect_host_runtime_census_round,
    )
}

fn require_stable_runtime_census_with(
    root: &Path,
    paths: &InstallPaths,
    args: &WorkstationInstallArgs,
    mut collect_round: impl FnMut() -> Result<crate::runtime_adoption::RuntimeCensusRound, String>,
) -> Result<PathBuf, String> {
    use crate::runtime_adoption::{
        build_stable_runtime_census, persist_runtime_census, UpgradeCheckpoint, UpgradeTransaction,
        UpgradeTransactionState, RUNTIME_ADOPTION_SCHEMA_VERSION,
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

    let census_result = collect_round().and_then(|first| {
        collect_round().and_then(|second| build_stable_runtime_census(&first, &second))
    });
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
    "Usage: agent-browser install workstation <--dry-run|--apply> [--json] [--dashboard-port <port>] [--guacamole-port <port>]"
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

fn materialize_payload(paths: &InstallPaths, args: &WorkstationInstallArgs) -> Result<(), String> {
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
            "binarySha256": binary_sha256,
            "supportManifestSha256": support_manifest_sha256,
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
        ensure_workstation_state(paths, args)?;
        let created_links = prepare_stable_generation_links(paths, &rendered_units)?;
        if let Err(error) = select_generation(paths, &generation_id) {
            remove_created_links(&created_links);
            return Err(error);
        }
        Ok(())
    })();
    let _ = remove_generation_tree(&staging);
    result
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
            let legacy_units = WORKSTATION_RECONCILE_QUIESCE_UNITS
                .iter()
                .any(|unit| paths.unit_dir.join(unit).exists());
            if paths.binary.exists() || paths.legacy_support_dir.exists() || legacy_units {
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
            if existing == target {
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
    let runtime_environment = format!(
        "EnvironmentFile=-%h/.agent-browser/.env\nEnvironment=AGENT_BROWSER_BIN={binary}\nEnvironment=AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT={}\nEnvironment=AGENT_BROWSER_GUACAMOLE_DIR={}\nEnvironment=AGENT_BROWSER_GUACAMOLE_SECRET_FILE={}\n",
        script_root.display(),
        guacamole_dir.display(),
        guacamole_secret_file.display()
    );
    vec![
        (
            "agent-browser-dashboard.service",
            format!(
                "[Unit]\nDescription=agent-browser dashboard\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nEnvironmentFile=-%h/.agent-browser/.env\nEnvironment=AGENT_BROWSER_DASHBOARD=1\nEnvironment=AGENT_BROWSER_DASHBOARD_PORT={dashboard_port}\nExecStart={binary}\nRestart=on-failure\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n"
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
    fn canonical_route_pool_rejects_route_or_display_drift() {
        let wrong_route = vec![
            serde_json::json!({
                "id": "guacamole-rdp-a",
                "routeId": "guacamole:4",
                "target": {"displayName": ":10"}
            }),
            serde_json::json!({
                "id": "guacamole-rdp-b",
                "routeId": "guacamole:2",
                "target": {"displayName": ":11"}
            }),
        ];
        assert!(validate_canonical_route_pool(&wrong_route)
            .unwrap_err()
            .contains("guacamole:1"));

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
            .contains("one display"));
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
            "guacamole/extensions/guac-manifest.json",
            "guacamole/extensions/agent-browser-defaults.js",
            "guacamole/extensions/agent-browser-defaults.jar",
        ] {
            assert!(root.join(relative).is_file(), "missing {relative}");
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
        assert!(WORKSTATION_RECONCILE_QUIESCE_UNITS.contains(&"agent-browser-dashboard.service"));
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
            ("agent-browser-dashboard.service", true),
            ("agent-browser-runtime-interlock.timer", true),
            ("agent-browser-guacamole-postgres-backup.timer", false),
        ]);

        assert_eq!(
            snapshot.units_to_start(),
            vec![
                "agent-browser-dashboard.service",
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
}
