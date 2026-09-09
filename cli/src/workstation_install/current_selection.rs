//! Read-only current-installation proof independent of an older failed upgrade.

use super::InstallPaths;
#[cfg(target_os = "linux")]
use super::{
    capture_runtime_host_identity, validate_sealed_generation_tree, workstation_file_sha256,
};
use crate::dashboard_ingress::DashboardBackend;
#[cfg(target_os = "linux")]
use crate::process_identity::{BootEpochStatus, ProcessObservation};
#[cfg(target_os = "linux")]
use crate::runtime_host_ingress::{RuntimeHostIngressRepository, RuntimeHostTopology};
use serde_json::Value;
#[cfg(target_os = "linux")]
use std::fs;

pub(super) fn validate(
    paths: &InstallPaths,
    generation_id: &str,
    ingress: &Value,
) -> Result<(), String> {
    validate_with_dashboard_probe(
        paths,
        generation_id,
        ingress,
        crate::dashboard_ingress::validate_dashboard_backend,
    )
}

#[cfg(not(target_os = "linux"))]
fn validate_with_dashboard_probe(
    _paths: &InstallPaths,
    _generation_id: &str,
    _ingress: &Value,
    _probe: impl FnOnce(&DashboardBackend) -> Result<(), String>,
) -> Result<(), String> {
    Err("current_selection_platform_proof_unavailable".to_string())
}

#[cfg(target_os = "linux")]
fn validate_with_dashboard_probe(
    paths: &InstallPaths,
    generation_id: &str,
    ingress: &Value,
    probe: impl FnOnce(&DashboardBackend) -> Result<(), String>,
) -> Result<(), String> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let io = |_: std::io::Error| "current_selection_evidence_unavailable".to_string();
    let generation = paths.generations_dir.join(generation_id);
    let binary = generation.join("bin/agent-browser");
    if fs::canonicalize(&paths.current_selector).map_err(io)?
        != fs::canonicalize(&generation).map_err(io)?
        || fs::canonicalize(&paths.binary).map_err(io)? != fs::canonicalize(&binary).map_err(io)?
    {
        return Err("current_selection_selector_mismatch".to_string());
    }
    validate_sealed_generation_tree(&generation)
        .map_err(|_| "current_selection_payload_not_sealed".to_string())?;
    let manifest: Value =
        serde_json::from_slice(&fs::read(generation.join("generation.json")).map_err(io)?)
            .map_err(|_| "current_selection_manifest_invalid".to_string())?;
    let binary_sha = workstation_file_sha256(&binary)
        .map_err(|_| "current_selection_binary_unreadable".to_string())?;
    let support_sha = workstation_file_sha256(&generation.join("support/manifest.json"))
        .map_err(|_| "current_selection_support_manifest_unreadable".to_string())?;
    if manifest["schemaVersion"] != "agent-browser.runtime-generation.v1"
        || manifest["generationId"] != generation_id
        || manifest["binarySha256"] != binary_sha
        || manifest["supportManifestSha256"] != support_sha
    {
        return Err("current_selection_payload_identity_mismatch".to_string());
    }
    let dashboard: DashboardBackend = serde_json::from_value(ingress["selectedBackend"].clone())
        .map_err(|_| "current_selection_dashboard_missing".to_string())?;
    // Ordinary dashboards may retain a package-level label. Bind the actual
    // executable manifest rather than equating that label with the install ID.
    if dashboard.runtime_manifest_sha256
        != crate::dashboard_ingress::dashboard_runtime_manifest_sha256_for_executable(&binary)
            .map_err(|_| "current_selection_dashboard_manifest_unavailable".to_string())?
        || ingress["operatorJourneyReady"] != true
        || ingress["presentationReceipt"]["state"] != "ready"
        || ingress["presentationReceipt"]["dashboardDeploymentGeneration"]
            != dashboard.generation_id
        || ingress["presentationReceipt"]["receiptId"]
            .as_str()
            .is_none_or(|id| id.trim().is_empty())
    {
        return Err("current_selection_dashboard_identity_unproven".to_string());
    }
    probe(&dashboard).map_err(|_| "current_selection_dashboard_probe_failed".to_string())?;
    let repository = RuntimeHostIngressRepository::new(
        paths.root.join(".agent-browser/runtime-host-ingress.json"),
    );
    let host_registry = repository
        .load()
        .map_err(|_| "current_selection_host_registry_unavailable".to_string())?;
    let host = host_registry.selected_backend();
    if host_registry.boot_epoch_status() != BootEpochStatus::Current
        || host_registry.active_transaction_id.is_some()
        || host_registry.candidate_backend().is_some()
        || host_registry.fallback_backend().is_some()
        || host.topology != RuntimeHostTopology::SingleHost
        || host.generation_id != generation_id
        || host.binary_sha256 != binary_sha
    {
        return Err("current_selection_host_ingress_unsettled".to_string());
    }
    let (identity, observed_backend) =
        capture_runtime_host_identity(&host.socket_dir, generation_id, &binary_sha, true).map_err(
            |error| {
                error
                    .split(':')
                    .next()
                    .filter(|code| code.starts_with("runtime_host_"))
                    .unwrap_or("current_selection_host_identity_unavailable")
                    .to_string()
            },
        )?;
    let observed = match crate::process_identity::observe_process(host.pid) {
        ProcessObservation::Observed(observed) => observed,
        _ => return Err("current_selection_host_process_unavailable".to_string()),
    };
    if &observed_backend != host
        || observed.start_token.as_deref() != Some(identity.process_start_token.as_str())
        || observed.executable_path.as_deref() != binary.to_str()
        || workstation_file_sha256(&std::path::PathBuf::from(format!("/proc/{}/exe", host.pid)))
            .map_err(|_| "current_selection_live_executable_unreadable".to_string())?
            != binary_sha
    {
        return Err("current_selection_host_process_changed".to_string());
    }
    let socket = fs::metadata(host.socket_dir.join("runtime-host.sock")).map_err(io)?;
    if !socket.file_type().is_socket()
        || host.socket_identity != format!("unix:{}:{}", socket.dev(), socket.ino())
    {
        return Err("current_selection_host_socket_changed".to_string());
    }
    if repository
        .load()
        .map_err(|_| "current_selection_host_registry_unavailable".to_string())?
        != host_registry
        || fs::canonicalize(&paths.current_selector).map_err(io)?
            != fs::canonicalize(&generation).map_err(io)?
        || crate::process_identity::observe_process(host.pid)
            != ProcessObservation::Observed(observed)
    {
        return Err("current_selection_changed_during_observation".to_string());
    }
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::runtime_host_ingress::{RuntimeHostBackend, RuntimeHostIngressRegistry};
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::os::unix::fs::{symlink, MetadataExt};
    use std::os::unix::net::UnixListener;
    use std::process::{Child, Command};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    struct Fixture {
        root: std::path::PathBuf,
        child: Child,
        _socket: UnixListener,
        stop: Arc<AtomicBool>,
        port: u16,
        server: Option<std::thread::JoinHandle<()>>,
        ingress: Value,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            self.stop.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", self.port));
            if let Some(server) = self.server.take() {
                let _ = server.join();
            }
            let _ = super::super::remove_generation_tree(&self.root);
        }
    }

    fn fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!("ab-current-{}", uuid::Uuid::new_v4()));
        let paths = super::super::install_paths(&root);
        let generation = paths.generations_dir.join("generation-current");
        let binary = generation.join("bin/agent-browser");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::create_dir_all(generation.join("support")).unwrap();
        fs::copy("/bin/sleep", &binary).unwrap();
        fs::write(generation.join("support/manifest.json"), b"{}").unwrap();
        let sha = workstation_file_sha256(&binary).unwrap();
        fs::write(generation.join("generation.json"), serde_json::to_vec(&serde_json::json!({
            "schemaVersion": "agent-browser.runtime-generation.v1", "generationId": "generation-current",
            "binarySha256": sha, "supportManifestSha256": workstation_file_sha256(&generation.join("support/manifest.json")).unwrap(),
        })).unwrap()).unwrap();
        super::super::seal_generation_tree(&generation).unwrap();
        fs::create_dir_all(paths.binary.parent().unwrap()).unwrap();
        symlink(&generation, &paths.current_selector).unwrap();
        symlink(&binary, &paths.binary).unwrap();
        let child = Command::new(&binary).arg("120").spawn().unwrap();
        let ProcessObservation::Observed(observed) =
            crate::process_identity::observe_process(child.id())
        else {
            panic!("fixture process missing")
        };
        let sockets = root.join("sockets");
        fs::create_dir_all(&sockets).unwrap();
        let socket = UnixListener::bind(sockets.join("runtime-host.sock")).unwrap();
        let meta = fs::metadata(sockets.join("runtime-host.sock")).unwrap();
        let socket_identity = format!("unix:{}:{}", meta.dev(), meta.ino());
        let host = RuntimeHostBackend {
            topology: RuntimeHostTopology::SingleHost,
            generation_id: "generation-current".to_string(),
            socket_dir: sockets.clone(),
            binary_sha256: sha.clone(),
            host_id: "fixture-host".to_string(),
            pid: child.id(),
            socket_identity: socket_identity.clone(),
        };
        RuntimeHostIngressRepository::new(root.join(".agent-browser/runtime-host-ingress.json"))
            .initialize(host)
            .unwrap();
        fs::write(sockets.join("runtime-host.json"), serde_json::to_vec(&serde_json::json!({
            "schemaVersion": "agent-browser.runtime-host.v1", "hostId": "fixture-host", "pid": child.id(),
            "executableGeneration": sha, "socketIdentity": socket_identity, "authenticationRecord": "fixture-token", "maxLanes": 1,
        })).unwrap()).unwrap();
        fs::write(
            sockets.join("runtime-host.identity.json"),
            serde_json::to_vec(&crate::process_identity::RecordedProcessIdentity {
                pid: child.id(),
                start_token: observed.start_token.unwrap(),
                executable_path: observed.executable_path,
                browser_family: None,
            })
            .unwrap(),
        )
        .unwrap();
        let manifest =
            crate::native::stream::runtime_manifest_json_for_executable(&binary).unwrap();
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let server = std::thread::spawn(move || {
            while let Ok((mut stream, _)) = listener.accept() {
                if thread_stop.load(Ordering::SeqCst) {
                    break;
                }
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .unwrap();
                let mut request = [0; 1024];
                stream.read(&mut request).unwrap();
                let body = manifest.to_string();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        let ingress = serde_json::json!({
            "dashboardIngressReady": true, "operatorJourneyReady": true,
            "selectedBackend": DashboardBackend::new("dashboard-package-label", port, crate::dashboard_ingress::dashboard_runtime_manifest_sha256_for_executable(&binary).unwrap()),
            "presentationReceipt": {"state":"ready", "receiptId":"fixture-authenticated", "dashboardDeploymentGeneration":"dashboard-package-label"},
        });
        Fixture {
            root,
            child,
            _socket: socket,
            stop,
            port,
            server: Some(server),
            ingress,
        }
    }

    #[test]
    fn unrelated_terminal_history_requires_current_payload_host_and_dashboard_proof() {
        let mut fixture = fixture();
        let paths = super::super::install_paths(&fixture.root);
        let mut transaction = super::super::new_upgrade_transaction(
            &paths,
            "failed-candidate".to_string(),
            "a".repeat(64),
            "b".repeat(64),
        );
        transaction.old_generation_id = Some("historical-old".to_string());
        transaction.state =
            crate::runtime_adoption::UpgradeTransactionState::FailedPreservedOldGeneration;
        let old = paths.generations_dir.join("historical-old");
        fs::create_dir_all(&old).unwrap();
        super::super::seal_generation_tree(&old).unwrap();
        let original_transaction = serde_json::to_value(&transaction).unwrap();
        let readiness = |transaction: &crate::runtime_adoption::UpgradeTransaction,
                         ingress: &Value,
                         draining| {
            super::super::workstation_upgrade_readiness(
                &paths,
                Some("generation-current"),
                Some(transaction),
                draining,
                ingress,
            )
        };
        let ready = readiness(&transaction, &fixture.ingress, false);
        assert_eq!(ready["selectedGenerationReady"], true, "{ready}");
        assert_eq!(ready["ready"], true);
        assert_eq!(
            ready["upgradeTransactionState"],
            "failed_preserved_old_generation"
        );
        assert_eq!(
            serde_json::to_value(&transaction).unwrap(),
            original_transaction
        );
        assert_eq!(
            readiness(&transaction, &fixture.ingress, true)["ready"],
            false
        );
        transaction.state = crate::runtime_adoption::UpgradeTransactionState::FailedEffectUncertain;
        assert_eq!(
            readiness(&transaction, &fixture.ingress, false)["selectedGenerationReady"],
            false
        );
        transaction.state = crate::runtime_adoption::UpgradeTransactionState::CandidateReady;
        assert_eq!(
            readiness(&transaction, &fixture.ingress, false)["ready"],
            false
        );
        transaction.state =
            crate::runtime_adoption::UpgradeTransactionState::FailedPreservedOldGeneration;
        let mut wrong_dashboard = fixture.ingress.clone();
        wrong_dashboard["selectedBackend"]["runtimeManifestSha256"] =
            Value::String("wrong".to_string());
        assert_eq!(
            readiness(&transaction, &wrong_dashboard, false)["selectedGenerationReady"],
            false
        );
        let mut missing_acceptance = fixture.ingress.clone();
        missing_acceptance["presentationReceipt"] = Value::Null;
        assert_eq!(
            readiness(&transaction, &missing_acceptance, false)["selectedGenerationReady"],
            false
        );
        let manifest_path = paths
            .generations_dir
            .join("generation-current/generation.json");
        let original_manifest = fs::read(&manifest_path).unwrap();
        let mut wrong_payload: Value = serde_json::from_slice(&original_manifest).unwrap();
        wrong_payload["binarySha256"] = Value::String("wrong-payload".to_string());
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&manifest_path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&manifest_path, serde_json::to_vec(&wrong_payload).unwrap()).unwrap();
        super::super::seal_generation_tree(&manifest_path).unwrap();
        let refused = readiness(&transaction, &fixture.ingress, false);
        assert_eq!(
            refused["currentSelectionEvidence"]["error"],
            "current_selection_payload_identity_mismatch"
        );
        fs::set_permissions(&manifest_path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&manifest_path, original_manifest).unwrap();
        super::super::seal_generation_tree(&manifest_path).unwrap();
        let host_path = fixture
            .root
            .join(".agent-browser/runtime-host-ingress.json");
        let registry: RuntimeHostIngressRegistry =
            serde_json::from_slice(&fs::read(&host_path).unwrap()).unwrap();
        let mut prior_boot = registry.clone();
        prior_boot.boot_epoch = Some("linux:prior-boot".to_string());
        fs::write(&host_path, serde_json::to_vec(&prior_boot).unwrap()).unwrap();
        assert_eq!(
            readiness(&transaction, &fixture.ingress, false)["selectedGenerationReady"],
            false
        );
        fs::write(&host_path, serde_json::to_vec(&registry).unwrap()).unwrap();
        fixture.child.kill().unwrap();
        fixture.child.wait().unwrap();
        assert_eq!(
            readiness(&transaction, &fixture.ingress, false)["selectedGenerationReady"],
            false
        );
    }
}
