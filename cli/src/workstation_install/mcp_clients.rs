//! Installed stdio MCP clients participate in generation convergence. The host
//! owns their pipes; retirement ends old connections rather than spawning an
//! unattached replacement. Only exact user-owned installed processes qualify.

use crate::process_identity::{
    RecordedProcessIdentity, VerifiedProcessSignal, VerifiedProcessTermination,
};
use serde::Serialize;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Client {
    pub(crate) identity: RecordedProcessIdentity,
    pub(crate) generation_id: String,
    pub(crate) stale: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Inventory {
    pub(crate) schema_version: &'static str,
    pub(crate) state: &'static str,
    pub(crate) clients: Vec<Client>,
    pub(crate) stale_count: usize,
}

pub(crate) fn inventory(root: &Path) -> Result<Inventory, String> {
    if !cfg!(target_os = "linux") {
        return Ok(report("unsupported_platform", Vec::new()));
    }
    let selector = root.join(".local/lib/agent-browser/current");
    let generations = root.join(".local/lib/agent-browser/generations");
    let selected = match std::fs::symlink_metadata(&selector) {
        Ok(_) => Some(
            std::fs::canonicalize(selector.join("bin/agent-browser"))
                .map_err(|e| format!("mcp_selected_executable_unproven:{e}"))?,
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("mcp_selection_unproven:{e}")),
    };
    if selected.is_none() && !generations.try_exists().map_err(|e| e.to_string())? {
        return Ok(report("not_installed", Vec::new()));
    }
    let generations =
        std::fs::canonicalize(generations).map_err(|e| format!("mcp_generations_unproven:{e}"))?;
    if selected
        .as_ref()
        .is_some_and(|path| installed_generation(&generations, path).is_none())
    {
        return Err("mcp_selected_executable_outside_installation".to_string());
    }
    // After a failed first install removes selection, every installed client is
    // stale. Never hide candidate clients merely because rollback has no selector.
    let clients = scan(root, selected.as_deref().unwrap_or(Path::new("")))?;
    let state = if clients.iter().any(|c| c.stale) {
        "stale"
    } else if selected.is_some() {
        "ready"
    } else {
        "not_installed"
    };
    Ok(report(state, clients))
}

fn report(state: &'static str, clients: Vec<Client>) -> Inventory {
    Inventory {
        schema_version: "agent-browser.installed-mcp-clients.v1",
        state,
        stale_count: clients.iter().filter(|c| c.stale).count(),
        clients,
    }
}

#[cfg(target_os = "linux")]
fn scan(root: &Path, selected: &Path) -> Result<Vec<Client>, String> {
    use std::os::unix::fs::MetadataExt;
    let generations = std::fs::canonicalize(root.join(".local/lib/agent-browser/generations"))
        .map_err(|e| format!("mcp_generations_unproven:{e}"))?;
    let mut clients = Vec::new();
    for entry in
        std::fs::read_dir("/proc").map_err(|e| format!("mcp_process_inventory_unavailable:{e}"))?
    {
        let entry = entry.map_err(|e| format!("mcp_process_inventory_unavailable:{e}"))?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let proc = entry.path();
        let Ok(metadata) = proc.metadata() else {
            continue;
        };
        if metadata.uid() != unsafe { libc::geteuid() } {
            continue;
        }
        let Ok(executable) = std::fs::read_link(proc.join("exe")) else {
            continue;
        };
        let generation = installed_generation(&generations, &executable).or_else(|| {
            (without_deleted_suffix(&executable) == root.join(".local/bin/agent-browser"))
                .then(|| "legacy_mutable".to_string())
        });
        let Some(generation) = generation else {
            continue;
        };
        let cmd = match std::fs::read(proc.join("cmdline")) {
            Ok(cmd) => cmd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("mcp_command_unproven:{pid}:{e}")),
        };
        if !is_mcp_command(&cmd) {
            continue;
        }
        let environment = match std::fs::read(proc.join("environ")) {
            Ok(environment) => environment,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("mcp_environment_unproven:{pid}:{e}")),
        };
        if !in_installation(root, &environment) {
            continue;
        }
        let Some(identity) =
            crate::process_identity::capture_process_identity(pid, Some(&executable), None)
        else {
            if matches!(
                crate::process_identity::observe_process(pid),
                crate::process_identity::ProcessObservation::Missing
            ) {
                continue;
            }
            return Err(format!("mcp_process_identity_unproven:{pid}"));
        };
        clients.push(Client {
            identity,
            generation_id: generation,
            stale: executable != selected,
        });
    }
    clients.sort_by_key(|c| c.identity.pid);
    Ok(clients)
}

#[cfg(not(target_os = "linux"))]
fn scan(_root: &Path, _selected: &Path) -> Result<Vec<Client>, String> {
    Err("installed_mcp_inventory_unsupported_platform".to_string())
}

fn without_deleted_suffix(executable: &Path) -> &Path {
    executable
        .to_str()
        .and_then(|path| path.strip_suffix(" (deleted)"))
        .map(Path::new)
        .unwrap_or(executable)
}

fn installed_generation(generations: &Path, executable: &Path) -> Option<String> {
    let relative = without_deleted_suffix(executable)
        .strip_prefix(generations)
        .ok()?;
    let parts = relative.components().collect::<Vec<_>>();
    if parts.len() != 3 || parts[1].as_os_str() != "bin" || parts[2].as_os_str() != "agent-browser"
    {
        return None;
    }
    Some(parts[0].as_os_str().to_str()?.to_string())
}

#[cfg(target_os = "linux")]
fn is_mcp_command(command: &[u8]) -> bool {
    // Reuse the CLI's pure flag normalization, including flags after `serve`.
    // A command argument containing MCP-looking text is not an MCP process.
    let args = command
        .split(|b| *b == 0)
        .filter(|arg| !arg.is_empty())
        .skip(1)
        .map(|arg| std::str::from_utf8(arg).map(str::to_owned))
        .collect::<Result<Vec<_>, _>>();
    args.is_ok_and(|args| crate::flags::clean_args(&args) == ["mcp", "serve"])
}

#[cfg(target_os = "linux")]
fn in_installation(root: &Path, environment: &[u8]) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let value = |key: &[u8]| {
        environment
            .split(|b| *b == 0)
            .find_map(|entry| entry.strip_prefix(key))
            .map(|value| std::path::PathBuf::from(std::ffi::OsStr::from_bytes(value)))
    };
    let same_path = |a: &Path, b: &Path| {
        a == b
            || std::fs::canonicalize(a)
                .ok()
                .zip(std::fs::canonicalize(b).ok())
                .is_some_and(|(a, b)| a == b)
    };
    if value(b"AGENT_BROWSER_WORKSTATION_ROOT=").is_some_and(|actual| !same_path(&actual, root)) {
        return false;
    }
    // A client can use installed bytes with a disposable HOME. Executable
    // location alone never grants ownership over that separate runtime namespace.
    let app_home = value(b"AGENT_BROWSER_HOME=")
        .or_else(|| value(b"HOME=").map(|home| home.join(".agent-browser")));
    app_home.is_some_and(|actual| same_path(&actual, &root.join(".agent-browser")))
}

/// Persist the exact target list before signaling and the verified outcome
/// afterward. No physical transaction lock spans shutdown or waiting.
pub(crate) fn retire(
    root: &Path,
    mut persist: impl FnMut(&serde_json::Value) -> Result<(), String>,
) -> Result<(), String> {
    let before = inventory(root)?;
    let targets = before
        .clients
        .into_iter()
        .filter(|c| c.stale)
        .collect::<Vec<_>>();
    let mut receipt = serde_json::json!({
        "schemaVersion": "agent-browser.mcp-client-retirement.v1",
        "attemptId": uuid::Uuid::new_v4().to_string(),
        "state": "planned", "targets": targets, "reconnectRequired": !targets.is_empty(),
    });
    if targets.is_empty() {
        receipt["state"] = "ready".into();
        persist(&receipt)?;
        return Ok(());
    }
    persist(&receipt)?;
    let mut handles = Vec::new();
    for client in &targets {
        receipt["state"] = "retiring".into();
        receipt["signalingPid"] = client.identity.pid.into();
        persist(&receipt)?;
        if let Some(process) = VerifiedProcessTermination::open(&client.identity)? {
            process.signal(VerifiedProcessSignal::Terminate)?;
            handles.push(process);
        }
    }
    wait(&handles, Duration::from_secs(5))?;
    for process in &handles {
        if process.is_running()? {
            process.signal(VerifiedProcessSignal::Kill)?;
        }
    }
    wait(&handles, Duration::from_secs(5))?;
    if handles.iter().any(|p| p.is_running().unwrap_or(true)) {
        receipt["state"] = "exit_unproven".into();
        persist(&receipt)?;
        return Err("installed_mcp_client_exit_unproven".to_string());
    }
    let after = inventory(root)?;
    receipt["state"] = if after.stale_count == 0 {
        "retired"
    } else {
        "stale_clients_remain"
    }
    .into();
    receipt["after"] = serde_json::to_value(&after).map_err(|e| e.to_string())?;
    persist(&receipt)?;
    if after.stale_count != 0 {
        return Err("installed_mcp_clients_remain_after_retirement".to_string());
    }
    Ok(())
}

fn wait(processes: &[VerifiedProcessTermination], grace: Duration) -> Result<(), String> {
    let deadline = Instant::now() + grace;
    while Instant::now() < deadline {
        let running = processes
            .iter()
            .map(|p| p.is_running())
            .collect::<Result<Vec<_>, _>>()?;
        if running.iter().all(|p| !p) {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::fs;
    use std::process::{Child, Command, Stdio};

    struct Fixture {
        root: std::path::PathBuf,
        children: Vec<Child>,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "agent-browser-mcp-convergence-{}",
                uuid::Uuid::new_v4()
            ));
            let generations = root.join(".local/lib/agent-browser/generations");
            for generation in ["old", "current"] {
                let bin = generations.join(generation).join("bin");
                fs::create_dir_all(&bin).unwrap();
                fs::copy("/bin/bash", bin.join("agent-browser")).unwrap();
            }
            fs::write(root.join("mcp"), "if [ \"$FIXTURE_STUBBORN\" = 1 ]; then trap '' TERM; fi\n: > \"$FIXTURE_READY\"\nread -r -t 60 unused\n").unwrap();
            std::os::unix::fs::symlink(
                "generations/current",
                root.join(".local/lib/agent-browser/current"),
            )
            .unwrap();
            Self {
                root,
                children: Vec::new(),
            }
        }
        fn spawn(&mut self, generation: &str, foreign_home: bool, stubborn: bool) -> u32 {
            let ready = self.root.join(uuid::Uuid::new_v4().to_string());
            let child = Command::new(
                self.root
                    .join(".local/lib/agent-browser/generations")
                    .join(generation)
                    .join("bin/agent-browser"),
            )
            .args(["mcp", "serve"])
            .current_dir(&self.root)
            .env("FIXTURE_READY", &ready)
            .env("FIXTURE_STUBBORN", if stubborn { "1" } else { "0" })
            .env(
                "AGENT_BROWSER_HOME",
                if foreign_home {
                    self.root.join("development")
                } else {
                    self.root.join(".agent-browser")
                },
            )
            .env("AGENT_BROWSER_WORKSTATION_ROOT", &self.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
            let pid = child.id();
            self.children.push(child);
            let deadline = Instant::now() + Duration::from_secs(5);
            while !ready.exists() {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(10));
            }
            pid
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            for child in &mut self.children {
                let _ = child.kill();
                let _ = child.wait();
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn old_mcp_clients_are_retired_current_and_development_clients_survive() {
        let mut fixture = Fixture::new();
        let old = fixture.spawn("old", false, false);
        let current = fixture.spawn("current", false, false);
        let foreign = fixture.spawn("old", true, false);
        let before = inventory(&fixture.root).unwrap();
        assert_eq!(before.state, "stale");
        assert_eq!(before.stale_count, 1);
        assert_eq!(before.clients.len(), 2);
        assert_eq!(
            before
                .clients
                .iter()
                .find(|c| c.stale)
                .unwrap()
                .identity
                .pid,
            old
        );
        let mut receipts = Vec::new();
        retire(&fixture.root, |receipt| {
            receipts.push(receipt.clone());
            Ok(())
        })
        .unwrap();
        assert_eq!(receipts.first().unwrap()["state"], "planned");
        assert_eq!(receipts.last().unwrap()["state"], "retired");
        assert!(fixture.children[0].try_wait().unwrap().is_some());
        for pid in [current, foreign] {
            assert!(fixture
                .children
                .iter_mut()
                .find(|c| c.id() == pid)
                .unwrap()
                .try_wait()
                .unwrap()
                .is_none());
        }
        assert_eq!(inventory(&fixture.root).unwrap().state, "ready");
        retire(&fixture.root, |receipt| {
            assert_eq!(receipt["state"], "ready");
            assert!(receipt["targets"].as_array().unwrap().is_empty());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn failed_receipt_prevents_signals_and_exact_stubborn_client_escalates() {
        let mut fixture = Fixture::new();
        fixture.spawn("old", false, true);
        assert_eq!(
            retire(&fixture.root, |_| Err("receipt_unavailable".into())).unwrap_err(),
            "receipt_unavailable"
        );
        assert!(fixture.children[0].try_wait().unwrap().is_none());
        retire(&fixture.root, |_| Ok(())).unwrap();
        assert!(fixture.children[0].try_wait().unwrap().is_some());
    }

    #[test]
    fn deleted_old_executable_remains_a_retirement_target() {
        let mut fixture = Fixture::new();
        fixture.spawn("old", false, false);
        fs::remove_file(
            fixture
                .root
                .join(".local/lib/agent-browser/generations/old/bin/agent-browser"),
        )
        .unwrap();
        assert_eq!(inventory(&fixture.root).unwrap().stale_count, 1);
        retire(&fixture.root, |_| Ok(())).unwrap();
        assert!(fixture.children[0].try_wait().unwrap().is_some());
    }

    #[test]
    fn installer_records_retirement_before_validation_and_replays_acceptance() {
        use super::super::*;
        let mut fixture = Fixture::new();
        fixture.spawn("old", false, false);
        fixture.spawn("current", false, false);
        let paths = install_paths(&fixture.root);
        let mut transaction =
            new_upgrade_transaction(&paths, "current".into(), "a".repeat(64), "b".repeat(64));
        transaction.state = crate::runtime_adoption::UpgradeTransactionState::GenerationCommitted;
        let path = transaction_path(&fixture.root, &transaction.transaction_id);
        let mut prepared = PreparedPayloadTransaction {
            staged: StagedWorkstationGeneration {
                generation_id: "current".into(),
                generation_path: paths.generations_dir.join("current"),
                binary_sha256: "a".repeat(64),
                support_manifest_sha256: "b".repeat(64),
                rendered_units: Vec::new(),
            },
            transaction_path: path.clone(),
            transaction,
            previous_selector: None,
            admission_drain_path: fixture.root.join("drain.json"),
            runtime_handoffs: Vec::new(),
            dashboard_candidate: None,
        };
        begin_post_commit_validation(&paths, &mut prepared).unwrap();
        assert!(fixture.children[0].try_wait().unwrap().is_some());
        let persisted: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(persisted["mcpClientRetirement"]["state"], "retired");
        assert_eq!(
            persisted["mcpClientRetirement"]["targets"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        accept_prepared_payload_transaction(
            &paths,
            &mut prepared,
            PostCommitValidationReceipt {
                dashboard_summary: "fixture".into(),
                presentation_summary: "fixture".into(),
            },
            None,
        )
        .unwrap();
        assert_eq!(
            prepared.transaction.terminal_result.as_deref(),
            Some("accepted")
        );
        assert_eq!(
            prepared.transaction.successor_fields["mcpClientRetirement"]["state"],
            "ready"
        );
        assert_eq!(
            prepared.transaction.successor_fields["mcpClientRetirement"]["reconnectRequired"],
            true
        );
        assert_eq!(
            prepared.transaction.successor_fields["mcpClientRetirementHistory"][0]["state"],
            "retired"
        );
        assert!(fixture.children[1].try_wait().unwrap().is_none());
    }

    #[test]
    fn forward_only_failure_converges_mcp_clients_on_selected_candidate() {
        use super::super::*;
        let mut fixture = Fixture::new();
        fixture.spawn("old", false, false);
        fixture.spawn("current", false, false);
        let paths = install_paths(&fixture.root);
        let mut transaction =
            new_upgrade_transaction(&paths, "current".into(), "a".repeat(64), "b".repeat(64));
        transaction.state = crate::runtime_adoption::UpgradeTransactionState::PostCommitValidating;
        let mut prepared = PreparedPayloadTransaction {
            staged: StagedWorkstationGeneration {
                generation_id: "current".into(),
                generation_path: paths.generations_dir.join("current"),
                binary_sha256: "a".repeat(64),
                support_manifest_sha256: "b".repeat(64),
                rendered_units: Vec::new(),
            },
            transaction_path: transaction_path(&fixture.root, &transaction.transaction_id),
            transaction,
            previous_selector: Some("generations/old".into()),
            admission_drain_path: fixture.root.join("drain.json"),
            runtime_handoffs: Vec::new(),
            dashboard_candidate: None,
        };
        let error =
            rollback_prepared_payload_transaction(&paths, &mut prepared, true, "fixture_failure")
                .unwrap_err();
        assert!(error.contains("install_transaction_forward_only"));
        assert_eq!(selected_generation_id(&paths).as_deref(), Some("current"));
        assert!(fixture.children[0].try_wait().unwrap().is_some());
        assert!(fixture.children[1].try_wait().unwrap().is_none());
        assert_eq!(
            prepared.transaction.successor_fields["mcpClientRetirement"]["state"],
            "retired"
        );
        assert_eq!(inventory(&fixture.root).unwrap().stale_count, 0);
    }

    #[test]
    fn failed_first_install_cannot_hide_clients_by_removing_selection() {
        let mut fixture = Fixture::new();
        fixture.spawn("current", false, false);
        fs::remove_file(fixture.root.join(".local/lib/agent-browser/current")).unwrap();
        assert_eq!(inventory(&fixture.root).unwrap().stale_count, 1);
        retire(&fixture.root, |_| Ok(())).unwrap();
        assert!(fixture.children[0].try_wait().unwrap().is_some());
    }

    #[test]
    fn changed_process_birth_cannot_be_signaled() {
        let mut fixture = Fixture::new();
        fixture.spawn("old", false, false);
        let mut identity = inventory(&fixture.root).unwrap().clients[0]
            .identity
            .clone();
        identity.start_token.push_str("changed");
        assert!(VerifiedProcessTermination::open(&identity).is_err());
        assert!(fixture.children[0].try_wait().unwrap().is_none());
    }

    #[test]
    fn runtime_home_is_positive_installation_ownership_evidence() {
        let root = Path::new("/synthetic/operator");
        assert!(in_installation(root, b"HOME=/synthetic/operator\0"));
        assert!(!in_installation(root, b"HOME=/synthetic/development\0"));
        assert!(in_installation(
            root,
            b"HOME=/synthetic/development\0AGENT_BROWSER_HOME=/synthetic/operator/.agent-browser\0"
        ));
        assert!(!in_installation(
            root,
            b"HOME=/synthetic/operator\0AGENT_BROWSER_WORKSTATION_ROOT=/synthetic/development\0"
        ));
        assert!(!in_installation(root, b"PATH=/usr/bin\0"));
    }

    #[test]
    fn exact_role_and_generation_boundaries_and_broken_selection() {
        assert!(is_mcp_command(
            b"agent-browser\0--session\0one\0mcp\0serve\0"
        ));
        assert!(!is_mcp_command(b"agent-browser\0eval\0mcp serve\0"));
        assert!(is_mcp_command(
            b"agent-browser\0mcp\0serve\0--session\0one\0--json\0"
        ));
        assert!(!is_mcp_command(b"agent-browser\0eval\0mcp\0serve\0"));
        assert!(!is_mcp_command(b"agent-browser\0mcp\0serve\0other\0"));
        assert!(installed_generation(
            Path::new("/install/generations"),
            Path::new("/install/generations/old/bin/agent-browser")
        )
        .is_some());
        assert!(installed_generation(
            Path::new("/install/generations"),
            Path::new("/install/generations/old/bin/foreign")
        )
        .is_none());
        assert!(installed_generation(
            Path::new("/install/generations"),
            Path::new("/install-dev/generations/old/bin/agent-browser")
        )
        .is_none());
        let fixture = Fixture::new();
        fs::remove_file(fixture.root.join(".local/lib/agent-browser/current")).unwrap();
        std::os::unix::fs::symlink(
            "generations/missing",
            fixture.root.join(".local/lib/agent-browser/current"),
        )
        .unwrap();
        assert!(inventory(&fixture.root).is_err());
    }
}
