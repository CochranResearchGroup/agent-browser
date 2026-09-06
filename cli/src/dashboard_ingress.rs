//! Stable dashboard ingress and generation-specific backend selection.
//!
//! The public listener resolves the selected backend for each connection,
//! retains the previously accepted backend for bounded fallback, and changes
//! selection only after a manifest-bound authenticated presentation proof.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream as StdTcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

pub(crate) const DASHBOARD_INGRESS_SCHEMA_VERSION: &str = "agent-browser.dashboard-ingress.v1";
const DASHBOARD_INGRESS_FIRST_RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);
// PrimaryTask::ready bounds startup at 16 seconds; retain five seconds for delivery.
const DASHBOARD_INGRESS_PRIMARY_FIRST_RESPONSE_TIMEOUT: Duration = Duration::from_secs(21);
const DASHBOARD_INGRESS_SERVICE_STATUS_FIRST_RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);
const DASHBOARD_INGRESS_HANDOFF_FIRST_RESPONSE_TIMEOUT: Duration = Duration::from_secs(65);
const DASHBOARD_INGRESS_DEFAULT_SERVICE_JOB_TIMEOUT: Duration = Duration::from_secs(30);
const DASHBOARD_INGRESS_SERVICE_RESPONSE_GRACE: Duration = Duration::from_secs(5);
const DASHBOARD_INGRESS_MAX_SERVICE_FIRST_RESPONSE_TIMEOUT: Duration = Duration::from_secs(125);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DashboardBackend {
    pub(crate) generation_id: String,
    pub(crate) port: u16,
    pub(crate) runtime_manifest_sha256: String,
}

impl DashboardBackend {
    pub(crate) fn new(
        generation_id: impl Into<String>,
        port: u16,
        runtime_manifest_sha256: impl Into<String>,
    ) -> Self {
        Self {
            generation_id: generation_id.into(),
            port,
            runtime_manifest_sha256: runtime_manifest_sha256.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateOperatorJourney {
    generation_id: String,
    authenticated: bool,
    runtime_manifest_valid: bool,
    operator_surface_ready: bool,
    evidence: Option<PresentationEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PresentationEvidence {
    pub(crate) receipt_id: String,
    pub(crate) dashboard_deployment_generation: String,
    pub(crate) coordinator_generation: String,
    pub(crate) daemon_generation: String,
    pub(crate) logical_browser_id: String,
    pub(crate) process_instance_digest: String,
    pub(crate) selected_target_generation: u64,
    pub(crate) selected_target_identity_digest: String,
    pub(crate) required_stream_provider: String,
    pub(crate) observed_stream_provider: String,
    pub(crate) display_allocation_id: String,
    pub(crate) geometry_epoch: String,
    pub(crate) route_generation: u64,
    pub(crate) guacamole_connection_generation: Option<u64>,
    pub(crate) authenticated_ingress_probe_at: String,
    pub(crate) operator_surface_load_result: String,
}

impl PresentationEvidence {
    pub(crate) fn from_ready_receipt(
        receipt: &crate::runtime_adoption::PresentationReceipt,
    ) -> Result<Self, String> {
        if receipt.state != crate::runtime_adoption::PresentationState::Ready
            || !receipt.reason_codes.is_empty()
        {
            return Err("dashboard presentation receipt is not ready".to_string());
        }
        Ok(Self {
            receipt_id: receipt.receipt_id.clone(),
            dashboard_deployment_generation: receipt.dashboard_deployment_generation.clone(),
            coordinator_generation: receipt.coordinator_generation.clone(),
            daemon_generation: receipt.daemon_generation.clone(),
            logical_browser_id: receipt.logical_browser_id.clone(),
            process_instance_digest: receipt.process_instance_digest.clone(),
            selected_target_generation: receipt.selected_target_generation,
            selected_target_identity_digest: receipt.selected_target_identity_digest.clone(),
            required_stream_provider: receipt.required_stream_provider.clone(),
            observed_stream_provider: receipt.required_stream_provider.clone(),
            display_allocation_id: receipt.display_allocation_id.clone(),
            geometry_epoch: receipt.geometry_epoch.clone(),
            route_generation: receipt.route_generation,
            guacamole_connection_generation: receipt.guacamole_connection_generation,
            authenticated_ingress_probe_at: receipt.authenticated_ingress_probe_at.clone(),
            operator_surface_load_result: receipt.operator_surface_load_result.clone(),
        })
    }
}

impl CandidateOperatorJourney {
    #[cfg(test)]
    fn blocked(generation_id: impl Into<String>) -> Self {
        Self {
            generation_id: generation_id.into(),
            authenticated: false,
            runtime_manifest_valid: false,
            operator_surface_ready: false,
            evidence: None,
        }
    }

    pub(crate) fn ready(evidence: PresentationEvidence) -> Self {
        Self {
            generation_id: evidence.dashboard_deployment_generation.clone(),
            authenticated: true,
            runtime_manifest_valid: true,
            operator_surface_ready: evidence.operator_surface_load_result == "ready",
            evidence: Some(evidence),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DashboardIngressRegistry {
    pub(crate) schema_version: String,
    pub(crate) revision: u64,
    selected_backend: DashboardBackend,
    candidate_backend: Option<DashboardBackend>,
    #[serde(default)]
    fallback_backend: Option<DashboardBackend>,
    #[serde(default)]
    rollback_backend: Option<DashboardBackend>,
    #[serde(default)]
    last_presentation_receipt: Option<crate::runtime_adoption::PresentationReceipt>,
    #[serde(default)]
    fallback_presentation_receipt: Option<crate::runtime_adoption::PresentationReceipt>,
    #[serde(default)]
    rollback_presentation_receipt: Option<crate::runtime_adoption::PresentationReceipt>,
}

impl DashboardIngressRegistry {
    pub(crate) fn new(selected_backend: DashboardBackend) -> Self {
        Self {
            schema_version: DASHBOARD_INGRESS_SCHEMA_VERSION.to_string(),
            revision: 1,
            selected_backend,
            candidate_backend: None,
            fallback_backend: None,
            rollback_backend: None,
            last_presentation_receipt: None,
            fallback_presentation_receipt: None,
            rollback_presentation_receipt: None,
        }
    }

    pub(crate) fn selected_backend(&self) -> &DashboardBackend {
        &self.selected_backend
    }

    pub(crate) fn candidate_backend(&self) -> Option<&DashboardBackend> {
        self.candidate_backend.as_ref()
    }

    pub(crate) fn fallback_backend(&self) -> Option<&DashboardBackend> {
        self.fallback_backend.as_ref()
    }

    pub(crate) fn last_presentation_receipt(
        &self,
    ) -> Option<&crate::runtime_adoption::PresentationReceipt> {
        self.last_presentation_receipt.as_ref()
    }

    pub(crate) fn stage_candidate(&mut self, candidate: DashboardBackend) -> Result<(), String> {
        if candidate.generation_id.trim().is_empty()
            || candidate.port == 0
            || candidate.runtime_manifest_sha256.trim().is_empty()
        {
            return Err("dashboard candidate identity is incomplete".to_string());
        }
        if candidate == self.selected_backend {
            return Ok(());
        }
        if self.candidate_backend.as_ref() == Some(&candidate) {
            return Ok(());
        }
        self.candidate_backend = Some(candidate);
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    pub(crate) fn commit_candidate(
        &mut self,
        journey: CandidateOperatorJourney,
    ) -> Result<crate::runtime_adoption::PresentationReceipt, String> {
        let candidate = self
            .candidate_backend
            .as_ref()
            .ok_or_else(|| "dashboard candidate is not staged".to_string())?;
        if journey.generation_id != candidate.generation_id
            || !journey.authenticated
            || !journey.runtime_manifest_valid
            || !journey.operator_surface_ready
        {
            return Err(
                "dashboard candidate operator journey is not ready for selection".to_string(),
            );
        }
        let evidence = journey
            .evidence
            .ok_or_else(|| "dashboard candidate presentation evidence is missing".to_string())?;
        if evidence.required_stream_provider != evidence.observed_stream_provider {
            return Err("dashboard candidate presented the wrong stream provider".to_string());
        }
        for (label, value) in [
            ("receipt ID", evidence.receipt_id.as_str()),
            (
                "coordinator generation",
                evidence.coordinator_generation.as_str(),
            ),
            ("daemon generation", evidence.daemon_generation.as_str()),
            ("logical browser ID", evidence.logical_browser_id.as_str()),
            (
                "process identity",
                evidence.process_instance_digest.as_str(),
            ),
            (
                "selected target identity",
                evidence.selected_target_identity_digest.as_str(),
            ),
            (
                "stream provider",
                evidence.required_stream_provider.as_str(),
            ),
            (
                "display allocation",
                evidence.display_allocation_id.as_str(),
            ),
            ("geometry epoch", evidence.geometry_epoch.as_str()),
            (
                "authenticated ingress probe time",
                evidence.authenticated_ingress_probe_at.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(format!("dashboard candidate {label} is missing"));
            }
        }
        let receipt = crate::runtime_adoption::PresentationReceipt {
            schema_version: crate::runtime_adoption::RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
            receipt_id: evidence.receipt_id,
            dashboard_deployment_generation: evidence.dashboard_deployment_generation,
            coordinator_generation: evidence.coordinator_generation,
            daemon_generation: evidence.daemon_generation,
            logical_browser_id: evidence.logical_browser_id,
            process_instance_digest: evidence.process_instance_digest,
            selected_target_generation: evidence.selected_target_generation,
            selected_target_identity_digest: evidence.selected_target_identity_digest,
            required_stream_provider: evidence.required_stream_provider,
            display_allocation_id: evidence.display_allocation_id,
            geometry_epoch: evidence.geometry_epoch,
            route_generation: evidence.route_generation,
            guacamole_connection_generation: evidence.guacamole_connection_generation,
            authenticated_ingress_probe_at: evidence.authenticated_ingress_probe_at,
            operator_surface_load_result: evidence.operator_surface_load_result,
            state: crate::runtime_adoption::PresentationState::Ready,
            reason_codes: Vec::new(),
        };
        let prior = std::mem::replace(&mut self.selected_backend, candidate.clone());
        if prior.generation_id != candidate.generation_id {
            self.rollback_backend = Some(prior.clone());
            self.rollback_presentation_receipt = self.last_presentation_receipt.clone();
        }
        self.fallback_backend = Some(prior);
        self.candidate_backend = None;
        self.fallback_presentation_receipt = self.last_presentation_receipt.take();
        self.last_presentation_receipt = Some(receipt.clone());
        self.revision = self.revision.saturating_add(1);
        Ok(receipt)
    }

    fn rollback_selected_candidate(&mut self, expected_generation: &str) -> Result<(), String> {
        if self.selected_backend.generation_id != expected_generation {
            return Err("dashboard selected generation changed before rollback".to_string());
        }
        let rollback = self
            .rollback_backend
            .take()
            .or_else(|| self.fallback_backend.take())
            .ok_or_else(|| "dashboard rollback backend is missing".to_string())?;
        let failed_candidate = std::mem::replace(&mut self.selected_backend, rollback);
        self.fallback_backend = Some(failed_candidate);
        self.candidate_backend = None;
        let failed_candidate_receipt = self.last_presentation_receipt.take();
        self.last_presentation_receipt = self
            .rollback_presentation_receipt
            .take()
            .or_else(|| self.fallback_presentation_receipt.take());
        self.fallback_presentation_receipt = failed_candidate_receipt;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DashboardIngressRepository {
    path: PathBuf,
}

impl DashboardIngressRepository {
    /// Opens a registry repository without reading or mutating its state.
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Returns the private registry path used by dashboard ingress commands.
    pub(crate) fn default_path() -> PathBuf {
        std::env::var_os("AGENT_BROWSER_DASHBOARD_INGRESS_STATE")
            .map(PathBuf::from)
            .or_else(|| {
                dirs::home_dir()
                    .map(|home| home.join(".agent-browser").join("dashboard-ingress.json"))
            })
            .unwrap_or_else(|| std::env::temp_dir().join("agent-browser-dashboard-ingress.json"))
    }

    pub(crate) fn load(&self) -> Result<DashboardIngressRegistry, String> {
        // Writers publish a complete registry with an atomic rename. Readers
        // therefore consume either the prior committed file or the new one and
        // must not contend on the exclusive compare-and-swap writer lock.
        load_registry(&self.path)
    }

    pub(crate) fn initialize(
        &self,
        selected_backend: DashboardBackend,
    ) -> Result<DashboardIngressRegistry, String> {
        let _lock = acquire_ingress_lock(&self.path)?;
        if self.path.exists() {
            return load_registry(&self.path);
        }
        let registry = DashboardIngressRegistry::new(selected_backend);
        write_registry_atomic(&self.path, &registry)?;
        Ok(registry)
    }

    pub(crate) fn stage_candidate(
        &self,
        expected_revision: u64,
        candidate: DashboardBackend,
    ) -> Result<DashboardIngressRegistry, String> {
        self.mutate(expected_revision, |registry| {
            registry.stage_candidate(candidate)
        })
    }

    pub(crate) fn commit_candidate(
        &self,
        expected_revision: u64,
        journey: CandidateOperatorJourney,
    ) -> Result<crate::runtime_adoption::PresentationReceipt, String> {
        let _lock = acquire_ingress_lock(&self.path)?;
        let mut registry = load_registry(&self.path)?;
        require_revision(&registry, expected_revision)?;
        let candidate = registry
            .candidate_backend()
            .ok_or_else(|| "dashboard candidate is not staged".to_string())?;
        validate_dashboard_backend(candidate)?;
        let receipt = registry.commit_candidate(journey)?;
        write_registry_atomic(&self.path, &registry)?;
        Ok(receipt)
    }

    pub(crate) fn rollback_candidate(
        &self,
        expected_revision: u64,
        expected_generation: &str,
    ) -> Result<DashboardIngressRegistry, String> {
        self.mutate(expected_revision, |registry| {
            let Some(candidate) = registry.candidate_backend() else {
                return Ok(());
            };
            if candidate.generation_id != expected_generation {
                return Err("dashboard candidate generation changed before rollback".to_string());
            }
            registry.candidate_backend = None;
            registry.revision = registry.revision.saturating_add(1);
            Ok(())
        })
    }

    pub(crate) fn rollback_selected_candidate(
        &self,
        expected_revision: u64,
        expected_generation: &str,
    ) -> Result<DashboardIngressRegistry, String> {
        self.mutate(expected_revision, |registry| {
            registry.rollback_selected_candidate(expected_generation)
        })
    }

    pub(crate) fn retire_fallback(
        &self,
        expected_revision: u64,
        expected_generation: &str,
    ) -> Result<DashboardIngressRegistry, String> {
        self.mutate(expected_revision, |registry| {
            let Some(fallback) = registry.fallback_backend() else {
                return Ok(());
            };
            if fallback.generation_id != expected_generation {
                return Err("dashboard fallback generation changed before retirement".to_string());
            }
            registry.fallback_backend = None;
            registry.fallback_presentation_receipt = None;
            registry.revision = registry.revision.saturating_add(1);
            Ok(())
        })
    }

    fn mutate(
        &self,
        expected_revision: u64,
        mutator: impl FnOnce(&mut DashboardIngressRegistry) -> Result<(), String>,
    ) -> Result<DashboardIngressRegistry, String> {
        let _lock = acquire_ingress_lock(&self.path)?;
        let mut registry = load_registry(&self.path)?;
        require_revision(&registry, expected_revision)?;
        mutator(&mut registry)?;
        write_registry_atomic(&self.path, &registry)?;
        Ok(registry)
    }
}

fn require_revision(
    registry: &DashboardIngressRegistry,
    expected_revision: u64,
) -> Result<(), String> {
    if registry.revision != expected_revision {
        return Err(format!(
            "dashboard ingress revision changed: expected {expected_revision}, current {}",
            registry.revision
        ));
    }
    Ok(())
}

fn load_registry(path: &Path) -> Result<DashboardIngressRegistry, String> {
    let body = fs::read(path).map_err(|error| {
        format!(
            "Unable to read dashboard ingress state {}: {error}",
            path.display()
        )
    })?;
    let registry: DashboardIngressRegistry = serde_json::from_slice(&body).map_err(|error| {
        format!(
            "Unable to parse dashboard ingress state {}: {error}",
            path.display()
        )
    })?;
    if registry.schema_version != DASHBOARD_INGRESS_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported dashboard ingress schema: {}",
            registry.schema_version
        ));
    }
    Ok(registry)
}

fn write_registry_atomic(path: &Path, registry: &DashboardIngressRegistry) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "dashboard ingress state path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Unable to create dashboard ingress directory {}: {error}",
            parent.display()
        )
    })?;
    let staged = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let body = serde_json::to_vec_pretty(registry)
        .map_err(|error| format!("Unable to serialize dashboard ingress state: {error}"))?;
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staged)
            .map_err(|error| {
                format!(
                    "Unable to stage dashboard ingress state {}: {error}",
                    staged.display()
                )
            })?;
        file.write_all(&body)
            .and_then(|_| file.sync_all())
            .map_err(|error| {
                format!(
                    "Unable to persist dashboard ingress state {}: {error}",
                    staged.display()
                )
            })?;
        set_private_file(&staged)?;
        fs::rename(&staged, path).map_err(|error| {
            format!(
                "Unable to commit dashboard ingress state {}: {error}",
                path.display()
            )
        })?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    result
}

fn acquire_ingress_lock(state_path: &Path) -> Result<File, String> {
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Unable to create dashboard ingress directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let lock_path = state_path.with_extension("json.lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|error| {
            format!(
                "Unable to open dashboard ingress lock {}: {error}",
                lock_path.display()
            )
        })?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(lock),
            Err(std::fs::TryLockError::WouldBlock) if Instant::now() >= deadline => {
                return Err("dashboard_ingress_lock_timeout".to_string());
            }
            Err(std::fs::TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(2)),
            Err(error) => return Err(format!("Unable to lock dashboard ingress state: {error}")),
        }
    }
}

#[cfg(unix)]
fn set_private_file(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|error| {
        format!(
            "Unable to protect dashboard ingress state {}: {error}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn set_private_file(_path: &Path) -> Result<(), String> {
    Ok(())
}

pub(crate) fn current_dashboard_backend(port: u16) -> DashboardBackend {
    let generation_id = std::env::var("AGENT_BROWSER_DASHBOARD_GENERATION")
        .unwrap_or_else(|_| format!("dashboard-{}", env!("CARGO_PKG_VERSION")));
    DashboardBackend::new(generation_id, port, dashboard_runtime_manifest_sha256())
}

pub(crate) fn dashboard_runtime_manifest_sha256() -> String {
    let manifest = crate::native::stream::runtime_manifest_json();
    format!("{:x}", Sha256::digest(manifest.to_string().as_bytes()))
}

pub(crate) fn dashboard_runtime_manifest_sha256_for_executable(
    executable: &Path,
) -> Result<String, String> {
    let manifest = crate::native::stream::runtime_manifest_json_for_executable(executable)?;
    Ok(format!(
        "{:x}",
        Sha256::digest(manifest.to_string().as_bytes())
    ))
}

/// Projects public-safe selector and presentation readiness for status and doctor.
pub(crate) fn dashboard_ingress_status_json() -> serde_json::Value {
    dashboard_ingress_status_for_path(&DashboardIngressRepository::default_path())
}

/// Projects a specific private ingress repository without falling back to the
/// ambient user-scoped path. Workstation fixtures and transaction validation
/// use this seam so an isolated root cannot inspect or mutate live state.
pub(crate) fn dashboard_ingress_status_for_path(path: &Path) -> serde_json::Value {
    let repository = DashboardIngressRepository::new(path);
    match repository.load() {
        Ok(registry) => {
            let receipt_ready = registry.last_presentation_receipt().is_some_and(|receipt| {
                receipt.state == crate::runtime_adoption::PresentationState::Ready
                    && receipt.dashboard_deployment_generation
                        == registry.selected_backend().generation_id
            });
            serde_json::json!({
                "schemaVersion": registry.schema_version,
                "revision": registry.revision,
                "dashboardIngressReady": true,
                "operatorJourneyReady": receipt_ready,
                "state": if registry.candidate_backend().is_some() || !receipt_ready {
                    "converging"
                } else {
                    "ready"
                },
                "selectedBackend": registry.selected_backend(),
                "candidateBackend": registry.candidate_backend(),
                "fallbackBackend": registry.fallback_backend(),
                "presentationReceipt": registry.last_presentation_receipt(),
            })
        }
        Err(error) => serde_json::json!({
            "schemaVersion": DASHBOARD_INGRESS_SCHEMA_VERSION,
            "dashboardIngressReady": false,
            "operatorJourneyReady": false,
            "state": "blocked",
            "error": error,
        }),
    }
}

/// Returns the generation serving the authenticated dashboard request.
///
/// A shadow candidate must bind its pre-commit presentation receipt to its
/// own generation, while an ordinary backend can fall back to the generation
/// selected behind the stable ingress.
pub(crate) fn selected_dashboard_generation() -> Result<String, String> {
    if let Some(generation_id) =
        dashboard_generation_override(std::env::var("AGENT_BROWSER_DASHBOARD_GENERATION"))
    {
        return Ok(generation_id);
    }
    let repository = DashboardIngressRepository::new(DashboardIngressRepository::default_path());
    Ok(repository.load()?.selected_backend().generation_id.clone())
}

fn dashboard_generation_override(configured: Result<String, std::env::VarError>) -> Option<String> {
    configured
        .ok()
        .filter(|generation_id| !generation_id.trim().is_empty())
}

pub(crate) fn stage_dashboard_candidate(
    expected_revision: u64,
    candidate: DashboardBackend,
) -> Result<serde_json::Value, String> {
    validate_dashboard_backend(&candidate)?;
    let repository = DashboardIngressRepository::new(DashboardIngressRepository::default_path());
    let registry = repository.stage_candidate(expected_revision, candidate)?;
    Ok(serde_json::json!({
        "revision": registry.revision,
        "selectedBackend": registry.selected_backend(),
        "candidateBackend": registry.candidate_backend(),
        "fallbackBackend": registry.fallback_backend(),
        "state": "converging",
    }))
}

pub(crate) fn commit_dashboard_candidate(
    expected_revision: u64,
    evidence: PresentationEvidence,
) -> Result<serde_json::Value, String> {
    let repository = DashboardIngressRepository::new(DashboardIngressRepository::default_path());
    let registry = repository.load()?;
    require_revision(&registry, expected_revision)?;
    let candidate = registry
        .candidate_backend()
        .ok_or_else(|| "dashboard candidate is not staged".to_string())?;
    validate_dashboard_backend(candidate)?;
    let receipt = repository
        .commit_candidate(expected_revision, CandidateOperatorJourney::ready(evidence))?;
    let registry = repository.load()?;
    Ok(serde_json::json!({
        "revision": registry.revision,
        "selectedBackend": registry.selected_backend(),
        "candidateBackend": registry.candidate_backend(),
        "fallbackBackend": registry.fallback_backend(),
        "presentationReceipt": receipt,
        "state": "ready",
    }))
}

/// Selects a staged candidate from the candidate dashboard's durable handoff
/// receipt after rechecking the exact runtime owner, route, display, target,
/// provider, and deployment generation recorded by service state.
pub(crate) fn commit_dashboard_candidate_from_handoff(
    expected_revision: u64,
    handoff_id: &str,
) -> Result<serde_json::Value, String> {
    use crate::native::service_store::{JsonServiceStateStore, ServiceStateStore};

    let repository = DashboardIngressRepository::new(DashboardIngressRepository::default_path());
    let registry = repository.load()?;
    require_revision(&registry, expected_revision)?;
    let state = JsonServiceStateStore::new(JsonServiceStateStore::default_path()?).load()?;
    let evidence = presentation_evidence_from_durable_handoff(&registry, &state, handoff_id)?;
    commit_dashboard_candidate(expected_revision, evidence)
}

/// Commits the staged ingress candidate after that exact dashboard generation
/// has completed an authenticated, ready durable-handoff resolution.
///
/// Requests served by the selected or a stale dashboard generation are a
/// no-op. The staged candidate may commit only from service-state evidence
/// that passes the same owner, route, display, target, provider, and
/// generation checks as the explicit CLI commit path.
pub(crate) fn commit_authenticated_dashboard_candidate_from_handoff(
    dashboard_generation: &str,
    handoff_id: &str,
) -> Result<bool, String> {
    commit_authenticated_dashboard_candidate_from_handoff_at_paths(
        &DashboardIngressRepository::default_path(),
        &crate::native::service_store::JsonServiceStateStore::default_path()?,
        dashboard_generation,
        handoff_id,
    )
}

fn commit_authenticated_dashboard_candidate_from_handoff_at_paths(
    ingress_path: &Path,
    service_state_path: &Path,
    dashboard_generation: &str,
    handoff_id: &str,
) -> Result<bool, String> {
    use crate::native::service_store::{JsonServiceStateStore, ServiceStateStore};

    let repository = DashboardIngressRepository::new(ingress_path);
    let registry = repository.load()?;
    let state = JsonServiceStateStore::new(service_state_path).load()?;
    let Some(evidence) = authenticated_candidate_handoff_evidence(
        &registry,
        &state,
        dashboard_generation,
        handoff_id,
    )?
    else {
        return Ok(false);
    };
    repository.commit_candidate(registry.revision, CandidateOperatorJourney::ready(evidence))?;
    Ok(true)
}

fn authenticated_candidate_handoff_evidence(
    registry: &DashboardIngressRegistry,
    state: &crate::native::service_model::ServiceState,
    dashboard_generation: &str,
    handoff_id: &str,
) -> Result<Option<PresentationEvidence>, String> {
    let candidate_matches = registry
        .candidate_backend()
        .is_some_and(|candidate| candidate.generation_id == dashboard_generation);
    if !candidate_matches {
        return Ok(None);
    }
    presentation_evidence_from_durable_handoff(registry, state, handoff_id).map(Some)
}

fn presentation_evidence_from_durable_handoff(
    registry: &DashboardIngressRegistry,
    state: &crate::native::service_model::ServiceState,
    handoff_id: &str,
) -> Result<PresentationEvidence, String> {
    use crate::native::service_model::ViewStreamProvider;

    let candidate = registry
        .candidate_backend()
        .ok_or_else(|| "dashboard candidate is not staged".to_string())?;
    let handoff = state
        .remote_view_handoffs
        .get(handoff_id)
        .filter(|handoff| handoff.state == "ready")
        .ok_or_else(|| "dashboard candidate durable handoff is not ready".to_string())?;
    let receipt = handoff
        .presentation_receipt
        .as_ref()
        .filter(|receipt| receipt.state == "ready" && receipt.generation > 0)
        .ok_or_else(|| {
            "dashboard candidate durable presentation receipt is not ready".to_string()
        })?;
    if receipt.dashboard_deployment_generation != candidate.generation_id
        || handoff.browser_id.as_deref() != Some(receipt.logical_browser_id.as_str())
        || handoff.target_id.as_deref() != Some(receipt.target_id.as_str())
        || handoff.last_route_id.as_deref() != Some(receipt.route_id.as_str())
        || handoff.last_display_allocation_id.as_deref()
            != Some(receipt.display_allocation_id.as_str())
        || receipt.required_stream_provider != receipt.observed_stream_provider
        || receipt.observed_at.trim().is_empty()
        || receipt.target_id.trim().is_empty()
    {
        return Err("dashboard candidate durable presentation evidence changed".to_string());
    }
    let owner_generation = receipt
        .daemon_owner_generation
        .filter(|generation| *generation > 0)
        .ok_or_else(|| "dashboard candidate durable receipt lacks owner generation".to_string())?;
    let process_instance_digest = receipt
        .process_instance_digest
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "dashboard candidate durable receipt lacks process identity".to_string())?;
    let handoff_session = handoff
        .session_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "dashboard candidate durable handoff lacks a session".to_string())?;
    let owner = state
        .runtime_owner_registry
        .owners
        .values()
        .find(|owner| {
            owner.browser_id == receipt.logical_browser_id
                && owner.daemon_session_route == handoff_session
        })
        .ok_or_else(|| "dashboard candidate durable handoff owner is unavailable".to_string())?;
    if owner.owner_generation != owner_generation
        || owner.process_instance_digest != process_instance_digest
    {
        return Err("dashboard candidate durable handoff owner changed".to_string());
    }
    let route = state
        .remote_view_routes
        .get(&receipt.route_id)
        .filter(|route| route.state == "ready")
        .ok_or_else(|| "dashboard candidate durable handoff route is not ready".to_string())?;
    if route.provider != receipt.observed_stream_provider
        || route.display_allocation_id.as_deref() != Some(receipt.display_allocation_id.as_str())
        || route.browser_id.as_deref() != Some(receipt.logical_browser_id.as_str())
        || route.session_id.as_deref() != Some(handoff_session)
    {
        return Err("dashboard candidate durable handoff route changed".to_string());
    }
    let display = state
        .display_allocations
        .get(&receipt.display_allocation_id)
        .filter(|display| display.state == "ready")
        .ok_or_else(|| "dashboard candidate durable handoff display is not ready".to_string())?;
    if display.owner_browser_id.as_deref() != Some(receipt.logical_browser_id.as_str())
        || display.owner_session_id.as_deref() != Some(handoff_session)
        || !display
            .route_ids
            .iter()
            .any(|route_id| route_id == &receipt.route_id)
    {
        return Err("dashboard candidate durable handoff display changed".to_string());
    }

    let required_stream_provider = serde_json::to_value(receipt.required_stream_provider)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .ok_or_else(|| "dashboard candidate stream provider is invalid".to_string())?;
    let receipt_body = serde_json::to_vec(receipt)
        .map_err(|error| format!("Unable to encode durable presentation receipt: {error}"))?;
    let display_body = serde_json::to_vec(display)
        .map_err(|error| format!("Unable to encode durable display evidence: {error}"))?;
    let receipt_id = format!(
        "durable-handoff-{}",
        hex_sha256(&[&receipt_body, candidate.runtime_manifest_sha256.as_bytes()].concat())
    );
    let geometry_epoch = format!("display-{}", hex_sha256(&display_body));
    let selected_target_identity_digest = hex_sha256(receipt.target_id.as_bytes());

    Ok(PresentationEvidence {
        receipt_id,
        dashboard_deployment_generation: candidate.generation_id.clone(),
        coordinator_generation: candidate.generation_id.clone(),
        daemon_generation: format!("owner-generation-{owner_generation}"),
        logical_browser_id: receipt.logical_browser_id.clone(),
        process_instance_digest: process_instance_digest.to_string(),
        selected_target_generation: receipt.generation,
        selected_target_identity_digest,
        required_stream_provider: required_stream_provider.clone(),
        observed_stream_provider: required_stream_provider,
        display_allocation_id: receipt.display_allocation_id.clone(),
        geometry_epoch,
        route_generation: receipt.generation,
        guacamole_connection_generation: (receipt.required_stream_provider
            == ViewStreamProvider::RdpGateway)
            .then_some(receipt.generation),
        authenticated_ingress_probe_at: receipt.observed_at.clone(),
        operator_surface_load_result: "ready".to_string(),
    })
}

/// Reports whether an existing durable handoff identifies one current browser,
/// target, and runtime owner that a staged candidate may adopt. Replaceable
/// presentation infrastructure and a generation-bound receipt are deliberately
/// excluded here because only the staged candidate can reacquire and prove
/// them. The strict candidate proof remains a separate commit prerequisite.
pub(crate) fn candidate_presentation_bootstrap_prerequisite(
    state: &crate::native::service_model::ServiceState,
) -> serde_json::Value {
    let mut eligible_handoff_ids = Vec::new();
    let mut blocker_counts = std::collections::BTreeMap::<&'static str, usize>::new();
    for handoff in state.remote_view_handoffs.values() {
        let owner_session =
            crate::native::remote_view_handoff::remote_view_handoff_ready_owner_session(
                state, handoff,
            )
            .or_else(|| {
                crate::native::remote_view_handoff::remote_view_handoff_live_owner_session(
                    state, handoff,
                )
            })
            .or_else(|| {
                crate::native::remote_view_handoff::remote_view_handoff_recoverable_pending_owner_session(
                    state, handoff,
                )
            });
        let blocker = if handoff.state != "ready" {
            Some("handoff_not_ready")
        } else if owner_session.is_none() {
            Some("current_owner_unproven")
        } else {
            None
        };
        if let Some(blocker) = blocker {
            *blocker_counts.entry(blocker).or_default() += 1;
        } else {
            eligible_handoff_ids.push(handoff.id.clone());
        }
    }
    let ready = !eligible_handoff_ids.is_empty();
    serde_json::json!({
        "schemaVersion": "agent-browser.candidate-presentation-prerequisite.v1",
        "proofPhase": "bootstrap",
        "ready": ready,
        "eligibleHandoffCount": eligible_handoff_ids.len(),
        "eligibleHandoffIds": eligible_handoff_ids,
        "candidateProofRequiredAfterStaging": true,
        "blockerCounts": blocker_counts,
        "nextAction": if ready {
            "stage_candidate_then_resolve_eligible_handoff"
        } else {
            "reconcile_one_adoptable_current_handoff_before_candidate_staging"
        },
    })
}

/// Reports whether a staged candidate has satisfied the independently
/// authenticated shadow-dashboard journey required for commit. The projection
/// contains only opaque handoff ids and blocker classes.
#[cfg(test)]
pub(crate) fn candidate_presentation_prerequisite(
    state: &crate::native::service_model::ServiceState,
) -> serde_json::Value {
    let mut eligible_handoff_ids = Vec::new();
    let mut blocker_counts = std::collections::BTreeMap::<&'static str, usize>::new();
    for handoff in state.remote_view_handoffs.values() {
        let owner_session =
            crate::native::remote_view_handoff::remote_view_handoff_ready_owner_session(
                state, handoff,
            );
        let owner = owner_session.as_deref().and_then(|owner_session| {
            state.runtime_owner_registry.owners.values().find(|owner| {
                owner.browser_id.as_str() == handoff.browser_id.as_deref().unwrap_or_default()
                    && owner.daemon_session_route == owner_session
            })
        });
        let route = handoff
            .last_route_id
            .as_deref()
            .and_then(|route_id| state.remote_view_routes.get(route_id));
        let display = handoff
            .last_display_allocation_id
            .as_deref()
            .and_then(|display_id| state.display_allocations.get(display_id));
        let blocker = if handoff.state != "ready" {
            Some("handoff_not_ready")
        } else if route.filter(|route| route.state == "ready").is_none() {
            Some("route_not_ready")
        } else if owner_session.is_none() {
            Some("current_owner_unproven")
        } else if handoff
            .presentation_receipt
            .as_ref()
            .filter(|receipt| receipt.state == "ready" && receipt.generation > 0)
            .is_none()
        {
            Some("presentation_receipt_unready")
        } else if handoff
            .presentation_receipt
            .as_ref()
            .is_some_and(|receipt| {
                receipt.logical_browser_id.as_str()
                    != handoff.browser_id.as_deref().unwrap_or_default()
                    || receipt.target_id.as_str()
                        != handoff.target_id.as_deref().unwrap_or_default()
                    || receipt.route_id.as_str()
                        != handoff.last_route_id.as_deref().unwrap_or_default()
                    || receipt.display_allocation_id.as_str()
                        != handoff
                            .last_display_allocation_id
                            .as_deref()
                            .unwrap_or_default()
                    || Some(receipt.required_stream_provider) != handoff.view_stream_provider
                    || receipt.observed_stream_provider != receipt.required_stream_provider
                    || receipt.observed_at.trim().is_empty()
                    || owner.is_none_or(|owner| {
                        Some(owner.owner_generation) != receipt.daemon_owner_generation
                            || Some(owner.process_instance_digest.as_str())
                                != receipt.process_instance_digest.as_deref()
                    })
            })
        {
            Some("presentation_receipt_changed")
        } else if route.is_some_and(|route| {
            route.browser_id.as_deref() != handoff.browser_id.as_deref()
                || route.session_id.as_deref() != owner_session.as_deref()
                || route.display_allocation_id.as_deref()
                    != handoff.last_display_allocation_id.as_deref()
                || Some(route.provider) != handoff.view_stream_provider
        }) {
            Some("route_changed")
        } else if display.filter(|display| display.state == "ready").is_none() {
            Some("display_not_ready")
        } else if display.is_some_and(|display| {
            display.owner_browser_id.as_deref() != handoff.browser_id.as_deref()
                || display.owner_session_id.as_deref() != owner_session.as_deref()
                || handoff.last_route_id.as_ref().is_none_or(|route_id| {
                    !display
                        .route_ids
                        .iter()
                        .any(|candidate| candidate == route_id)
                })
        }) {
            Some("display_changed")
        } else {
            None
        };
        if let Some(blocker) = blocker {
            *blocker_counts.entry(blocker).or_default() += 1;
        } else {
            eligible_handoff_ids.push(handoff.id.clone());
        }
    }
    let ready = !eligible_handoff_ids.is_empty();
    serde_json::json!({
        "schemaVersion": "agent-browser.candidate-presentation-prerequisite.v1",
        "proofPhase": "candidate_authenticated",
        "ready": ready,
        "eligibleHandoffCount": eligible_handoff_ids.len(),
        "eligibleHandoffIds": eligible_handoff_ids,
        "blockerCounts": blocker_counts,
        "nextAction": if ready {
            "resolve_eligible_handoff_through_authenticated_candidate"
        } else {
            "reconcile_exact_current_handoff_or_create_fresh_presentation_handoff"
        },
    })
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn rollback_dashboard_candidate(
    expected_revision: u64,
    generation_id: &str,
) -> Result<serde_json::Value, String> {
    let repository = DashboardIngressRepository::new(DashboardIngressRepository::default_path());
    let registry = repository.rollback_candidate(expected_revision, generation_id)?;
    Ok(serde_json::json!({
        "revision": registry.revision,
        "selectedBackend": registry.selected_backend(),
        "candidateBackend": registry.candidate_backend(),
        "fallbackBackend": registry.fallback_backend(),
        "state": if registry.last_presentation_receipt().is_some() { "ready" } else { "converging" },
    }))
}

/// Runs the stable public listener without owning a generation-specific backend.
pub(crate) async fn run_dashboard_ingress_server(public_port: u16, fallback_backend_port: u16) {
    let repository = DashboardIngressRepository::new(DashboardIngressRepository::default_path());
    if let Err(error) = repository.initialize(current_dashboard_backend(fallback_backend_port)) {
        eprintln!("Failed to initialize dashboard ingress: {error}");
        return;
    }
    let listener = match TcpListener::bind(("127.0.0.1", public_port)).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Failed to bind dashboard ingress on 127.0.0.1:{public_port}: {error}");
            return;
        }
    };
    if std::env::var_os("AGENT_BROWSER_DASHBOARD_INGRESS_SKIP_SERVICE_BACKEND").is_none() {
        tokio::spawn(crate::native::stream::ensure_dashboard_service_backend());
    }
    loop {
        let Ok((mut client, _)) = listener.accept().await else {
            break;
        };
        let repository = repository.clone();
        tokio::spawn(async move {
            let registry = match load_ingress_registry_for_request(repository).await {
                Ok(registry) => registry,
                Err(error) => {
                    write_ingress_unavailable(
                        &mut client,
                        "registry_unavailable",
                        &error,
                        "dashboard_load",
                        None,
                    )
                    .await;
                    return;
                }
            };
            proxy_ingress_request(&mut client, &registry).await;
        });
    }
}

async fn load_ingress_registry_for_request(
    repository: DashboardIngressRepository,
) -> Result<DashboardIngressRegistry, String> {
    tokio::task::spawn_blocking(move || repository.load())
        .await
        .map_err(|error| format!("dashboard ingress registry reader failed: {error}"))?
}

async fn proxy_ingress_request(client: &mut TcpStream, registry: &DashboardIngressRegistry) {
    let request = match read_initial_http_request(client).await {
        Ok(request) => request,
        Err(error) => {
            write_ingress_unavailable(
                client,
                "invalid_ingress_request",
                &error,
                "dashboard_load",
                None,
            )
            .await;
            return;
        }
    };
    let (request_method, request_path, request_action) =
        dashboard_ingress_request_identity(&request);
    let retry_safe = request.starts_with(b"GET ")
        || request.starts_with(b"HEAD ")
        || request.starts_with(b"OPTIONS ");
    let websocket_upgrade = String::from_utf8_lossy(&request)
        .lines()
        .any(|line| line.eq_ignore_ascii_case("upgrade: websocket"));
    let backend_request = if websocket_upgrade {
        request
    } else {
        request_with_connection_close(&request)
    };
    let first_response_timeout = dashboard_ingress_first_response_timeout(&backend_request);
    let mut attempts = vec![registry.selected_backend()];
    if retry_safe {
        if let Some(fallback) = registry.fallback_backend() {
            attempts.push(fallback);
        }
    }
    let mut failures = Vec::new();
    let mut failure_stages = Vec::new();
    let mut mutation_outcome_unknown = false;
    for backend in attempts {
        match attempt_dashboard_backend(backend, &backend_request, first_response_timeout).await {
            Ok((mut connection, first_response)) => {
                if client.write_all(&first_response).await.is_ok() {
                    proxy_ingress_connection(client, &mut connection).await;
                }
                return;
            }
            Err(error) => {
                mutation_outcome_unknown |= !retry_safe && error.request_may_have_been_delivered();
                failure_stages.push(error.failure_stage());
                failures.push(format!("{}: {}", backend.generation_id, error.message()));
            }
        }
    }
    if mutation_outcome_unknown {
        write_ingress_mutation_outcome_unknown(
            client,
            &format!(
                "selected dashboard generation {} accepted the request connection but did not return a bounded response: {}",
                registry.selected_backend().generation_id,
                failures.join("; ")
            ),
        )
        .await;
        return;
    }
    write_ingress_unavailable(
        client,
        "selected_backend_unavailable",
        &format!(
            "selected dashboard generation {} is converging: {}",
            registry.selected_backend().generation_id,
            failures.join("; ")
        ),
        &request_action,
        Some(serde_json::json!({
            "requestMethod": request_method,
            "requestPath": request_path,
            "retrySafe": retry_safe,
            "selectedBackendGeneration": registry.selected_backend().generation_id,
            "fallbackAttempted": registry.fallback_backend().is_some() && retry_safe,
            "backendFailureStages": failure_stages,
            "firstResponseTimeoutMs": first_response_timeout.as_millis(),
        })),
    )
    .await;
}

enum DashboardBackendAttemptError {
    BeforeDelivery(String),
    AfterDelivery(String),
}

impl DashboardBackendAttemptError {
    fn request_may_have_been_delivered(&self) -> bool {
        matches!(self, Self::AfterDelivery(_))
    }

    fn message(&self) -> &str {
        match self {
            Self::BeforeDelivery(message) | Self::AfterDelivery(message) => message,
        }
    }

    fn failure_stage(&self) -> &'static str {
        match self {
            Self::BeforeDelivery(_) => "before_delivery",
            Self::AfterDelivery(message) if message == "first response byte timed out" => {
                "first_response_timeout"
            }
            Self::AfterDelivery(_) => "after_delivery",
        }
    }
}

fn dashboard_ingress_request_identity(request: &[u8]) -> (String, String, String) {
    let request_line = String::from_utf8_lossy(request)
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    let mut fields = request_line.split_whitespace();
    let method = fields.next().unwrap_or("UNKNOWN").to_string();
    let raw_path = fields
        .next()
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/");
    let (path, action) = match raw_path {
        "/api/service/resources" => (raw_path.to_string(), "service_resources".to_string()),
        "/api/service/status" => (raw_path.to_string(), "service_status".to_string()),
        "/api/service/contracts" => (raw_path.to_string(), "service_contracts".to_string()),
        "/api/service/browser-capability-registry" => (
            raw_path.to_string(),
            "service_browser_capability_registry".to_string(),
        ),
        "/api/service/request" => (raw_path.to_string(), "service_request".to_string()),
        "/api/session-tabs" => (raw_path.to_string(), "session_tabs".to_string()),
        "/api/sessions" => (raw_path.to_string(), "sessions_read".to_string()),
        "/api/models" => (raw_path.to_string(), "models_read".to_string()),
        "/api/runtime/health" => (raw_path.to_string(), "runtime_health".to_string()),
        "/api/dashboard-auth/status" => (raw_path.to_string(), "dashboard_auth_status".to_string()),
        "/api/chat/status" => (raw_path.to_string(), "chat_status".to_string()),
        "/api/runtime/manifest" => (raw_path.to_string(), "runtime_manifest".to_string()),
        path if path.starts_with("/remote-view/") => (
            "/remote-view/<redacted>".to_string(),
            "remote_view_handoff_load".to_string(),
        ),
        path if path.starts_with("/guacamole") => (
            "/guacamole/<redacted>".to_string(),
            "guacamole_load".to_string(),
        ),
        _ => (
            "/dashboard/<route>".to_string(),
            "dashboard_load".to_string(),
        ),
    };
    (method, path, action)
}

/// Service requests may commit a mutation before the backend writes its first
/// response byte. Keep ingress attached through the worker's bounded job
/// timeout plus a small response grace so a committed request is not reported
/// as a retryable backend failure.
fn dashboard_ingress_first_response_timeout(request: &[u8]) -> Duration {
    if request.starts_with(b"GET ")
        || request.starts_with(b"HEAD ")
        || request.starts_with(b"OPTIONS ")
    {
        // Idempotent dashboard reads can queue behind a large live projection
        // while the host is under admitted pressure. They are safe to retry,
        // but must not be declared unavailable before the selected backend's
        // own bounded read budget can finish.
        return DASHBOARD_INGRESS_SERVICE_STATUS_FIRST_RESPONSE_TIMEOUT;
    }
    if request.starts_with(b"POST /api/guacamole-primary-claim ") {
        return DASHBOARD_INGRESS_PRIMARY_FIRST_RESPONSE_TIMEOUT;
    }
    if !request.starts_with(b"POST /api/service/request ") {
        return DASHBOARD_INGRESS_FIRST_RESPONSE_TIMEOUT;
    }
    let Some(body_offset) = request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
    else {
        return DASHBOARD_INGRESS_FIRST_RESPONSE_TIMEOUT;
    };
    let Some(payload) = serde_json::from_slice::<serde_json::Value>(&request[body_offset..]).ok()
    else {
        return DASHBOARD_INGRESS_FIRST_RESPONSE_TIMEOUT;
    };
    let requested_timeout = payload
        .get("jobTimeoutMs")
        .and_then(serde_json::Value::as_u64)
        .filter(|timeout_ms| *timeout_ms > 0)
        .map(Duration::from_millis)
        .unwrap_or(DASHBOARD_INGRESS_DEFAULT_SERVICE_JOB_TIMEOUT);
    let bounded_timeout = requested_timeout
        .saturating_add(DASHBOARD_INGRESS_SERVICE_RESPONSE_GRACE)
        .min(DASHBOARD_INGRESS_MAX_SERVICE_FIRST_RESPONSE_TIMEOUT);
    if payload.get("action").and_then(serde_json::Value::as_str)
        == Some("service_remote_view_handoff_resolve")
    {
        bounded_timeout.max(DASHBOARD_INGRESS_HANDOFF_FIRST_RESPONSE_TIMEOUT)
    } else {
        bounded_timeout
    }
}

fn request_with_connection_close(request: &[u8]) -> Vec<u8> {
    let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
        return request.to_vec();
    };
    let mut rewritten = Vec::with_capacity(request.len() + 19);
    rewritten.extend_from_slice(&request[..header_end]);
    rewritten.extend_from_slice(b"\r\nConnection: close");
    rewritten.extend_from_slice(&request[header_end..]);
    rewritten
}

pub(crate) fn validate_dashboard_backend(backend: &DashboardBackend) -> Result<(), String> {
    let address = SocketAddr::from(([127, 0, 0, 1], backend.port));
    let mut stream =
        StdTcpStream::connect_timeout(&address, Duration::from_secs(2)).map_err(|error| {
            format!(
                "dashboard candidate generation {} is unavailable: {error}",
                backend.generation_id
            )
        })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .and_then(|()| stream.set_write_timeout(Some(Duration::from_secs(2))))
        .map_err(|error| format!("Unable to configure dashboard candidate probe: {error}"))?;
    stream
        .write_all(
            b"GET /api/runtime/manifest HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        )
        .map_err(|error| format!("Unable to probe dashboard candidate: {error}"))?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| format!("Unable to read dashboard candidate manifest: {error}"))?;
    let separator = b"\r\n\r\n";
    let body_offset = response
        .windows(separator.len())
        .position(|window| window == separator)
        .map(|index| index + separator.len())
        .ok_or_else(|| "dashboard candidate returned an invalid HTTP response".to_string())?;
    let headers = String::from_utf8_lossy(&response[..body_offset]);
    if !headers.starts_with("HTTP/1.1 200") && !headers.starts_with("HTTP/1.0 200") {
        return Err(format!(
            "dashboard candidate manifest probe failed: {}",
            headers.lines().next().unwrap_or("missing status")
        ));
    }
    let manifest: serde_json::Value = serde_json::from_slice(&response[body_offset..])
        .map_err(|error| format!("dashboard candidate manifest is invalid JSON: {error}"))?;
    if manifest
        .get("schemaVersion")
        .and_then(serde_json::Value::as_str)
        != Some("agent-browser.runtime-manifest.v1")
    {
        return Err("dashboard candidate runtime manifest schema is invalid".to_string());
    }
    let observed_sha256 = format!("{:x}", Sha256::digest(manifest.to_string().as_bytes()));
    if observed_sha256 != backend.runtime_manifest_sha256 {
        return Err(format!(
            "dashboard candidate manifest changed: expected {}, observed {observed_sha256}",
            backend.runtime_manifest_sha256
        ));
    }
    Ok(())
}

async fn read_initial_http_request(client: &mut TcpStream) -> Result<Vec<u8>, String> {
    const MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;
    let mut request = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut required_length = None;
    loop {
        let count = timeout(Duration::from_secs(2), client.read(&mut buffer))
            .await
            .map_err(|_| "dashboard ingress request timed out".to_string())?
            .map_err(|error| format!("Unable to read dashboard ingress request: {error}"))?;
        if count == 0 {
            return Err("dashboard ingress request closed before headers completed".to_string());
        }
        request.extend_from_slice(&buffer[..count]);
        if request.len() > MAX_REQUEST_BYTES {
            return Err("dashboard ingress request exceeds 16 MiB".to_string());
        }
        if required_length.is_none() {
            if let Some(header_end) = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|index| index + 4)
            {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                });
                required_length = Some(header_end.saturating_add(content_length.unwrap_or(0)));
            }
        }
        if required_length.is_some_and(|length| request.len() >= length) {
            return Ok(request);
        }
    }
}

async fn attempt_dashboard_backend(
    backend: &DashboardBackend,
    request: &[u8],
    first_response_timeout: Duration,
) -> Result<(TcpStream, Vec<u8>), DashboardBackendAttemptError> {
    let mut connection = timeout(
        Duration::from_secs(2),
        TcpStream::connect(("127.0.0.1", backend.port)),
    )
    .await
    .map_err(|_| DashboardBackendAttemptError::BeforeDelivery("connect timed out".to_string()))?
    .map_err(|error| {
        DashboardBackendAttemptError::BeforeDelivery(format!("connect failed: {error}"))
    })?;
    connection.write_all(request).await.map_err(|error| {
        DashboardBackendAttemptError::AfterDelivery(format!("request write failed: {error}"))
    })?;
    let mut first_response = vec![0_u8; 8192];
    let count = timeout(first_response_timeout, connection.read(&mut first_response))
        .await
        .map_err(|_| {
            DashboardBackendAttemptError::AfterDelivery("first response byte timed out".to_string())
        })?
        .map_err(|error| {
            DashboardBackendAttemptError::AfterDelivery(format!("response read failed: {error}"))
        })?;
    if count == 0 {
        return Err(DashboardBackendAttemptError::AfterDelivery(
            "backend closed before returning a response".to_string(),
        ));
    }
    first_response.truncate(count);
    Ok((connection, first_response))
}

async fn write_ingress_mutation_outcome_unknown(client: &mut TcpStream, message: &str) {
    let body = serde_json::json!({
        "success": false,
        "error": "mutation_outcome_unknown",
        "data": {
            "dashboardIngressReady": true,
            "operatorJourneyReady": false,
            "state": "outcome_unknown",
            "retrySafe": false,
            "message": message,
        }
    })
    .to_string();
    let response = format!(
        "HTTP/1.1 502 Bad Gateway\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = client.write_all(response.as_bytes()).await;
    let _ = client.shutdown().await;
}

async fn proxy_ingress_connection(client: &mut TcpStream, backend: &mut TcpStream) {
    let _ = tokio::io::copy_bidirectional(client, backend).await;
    let _ = client.shutdown().await;
    let _ = backend.shutdown().await;
}

async fn write_ingress_unavailable(
    client: &mut TcpStream,
    code: &str,
    message: &str,
    action: &str,
    details: Option<serde_json::Value>,
) {
    crate::native::service_failure_journal::append_service_failure_best_effort(
        &dashboard_ingress_failure_record(code, action, details),
    );
    let body = serde_json::json!({
        "success": false,
        "error": code,
        "data": {
            "dashboardIngressReady": true,
            "operatorJourneyReady": false,
            "state": "converging",
            "message": message,
            "retryAfterMs": 1000,
        }
    })
    .to_string();
    let response = format!(
        "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json; charset=utf-8\r\nRetry-After: 1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(), body
    );
    let _ = client.write_all(response.as_bytes()).await;
}

fn dashboard_ingress_failure_record(
    code: &str,
    action: &str,
    details: Option<serde_json::Value>,
) -> crate::native::service_failure_journal::ServiceFailureRecord {
    let record = crate::native::service_failure_journal::ServiceFailureRecord::new(
        crate::native::service_failure_journal::ServiceFailureCategory::DashboardAction,
        "dashboard_ingress",
        "request_proxy",
        code,
        "Stable dashboard ingress could not serve the request.",
    )
    .with_action(action);
    match details {
        Some(details) => record.with_details(details),
        None => record,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener as StdTcpListener;
    use std::thread;

    fn temp_registry_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "agent-browser-dashboard-ingress-{label}-{}-{}.json",
            std::process::id(),
            uuid::Uuid::new_v4()
        ))
    }

    fn ready_evidence(generation_id: &str) -> PresentationEvidence {
        PresentationEvidence {
            receipt_id: "presentation-1".to_string(),
            dashboard_deployment_generation: generation_id.to_string(),
            coordinator_generation: "coordinator-new".to_string(),
            daemon_generation: "daemon-new".to_string(),
            logical_browser_id: "browser-1".to_string(),
            process_instance_digest: "process-digest".to_string(),
            selected_target_generation: 9,
            selected_target_identity_digest: "target-digest".to_string(),
            required_stream_provider: "rdp_gateway".to_string(),
            observed_stream_provider: "rdp_gateway".to_string(),
            display_allocation_id: "display-1".to_string(),
            geometry_epoch: "geometry-3".to_string(),
            route_generation: 12,
            guacamole_connection_generation: Some(4),
            authenticated_ingress_probe_at: "2026-08-15T12:00:00Z".to_string(),
            operator_surface_load_result: "ready".to_string(),
        }
    }

    #[test]
    fn ingress_unavailable_response_has_a_postmortem_failure_record() {
        let record = dashboard_ingress_failure_record(
            "selected_backend_unavailable",
            "service_resources",
            Some(serde_json::json!({
                "requestMethod": "GET",
                "requestPath": "/api/service/resources",
                "firstResponseTimeoutMs": 10_000,
            })),
        );

        assert_eq!(
            record.category,
            crate::native::service_failure_journal::ServiceFailureCategory::DashboardAction
        );
        assert_eq!(record.source, "dashboard_ingress");
        assert_eq!(record.stage, "request_proxy");
        assert_eq!(record.code, "selected_backend_unavailable");
        assert_eq!(record.action.as_deref(), Some("service_resources"));
        assert_eq!(
            record.details.as_ref().unwrap()["requestPath"],
            "/api/service/resources"
        );
        assert_eq!(
            record.details.as_ref().unwrap()["firstResponseTimeoutMs"],
            10_000
        );
    }

    #[test]
    fn ingress_failure_request_identity_redacts_handoff_and_guacamole_paths() {
        assert_eq!(
            dashboard_ingress_request_identity(
                b"GET /remote-view/private-handoff?token=secret HTTP/1.1\r\n\r\n"
            ),
            (
                "GET".to_string(),
                "/remote-view/<redacted>".to_string(),
                "remote_view_handoff_load".to_string(),
            )
        );
        assert_eq!(
            dashboard_ingress_request_identity(
                b"GET /guacamole/api/session/data/private-token HTTP/1.1\r\n\r\n"
            ),
            (
                "GET".to_string(),
                "/guacamole/<redacted>".to_string(),
                "guacamole_load".to_string(),
            )
        );
    }

    #[test]
    fn shadow_dashboard_generation_overrides_the_precommit_selected_backend() {
        assert_eq!(
            dashboard_generation_override(Ok("generation-candidate".to_string())),
            Some("generation-candidate".to_string())
        );
        assert_eq!(dashboard_generation_override(Ok("   ".to_string())), None);
        assert_eq!(
            dashboard_generation_override(Err(std::env::VarError::NotPresent)),
            None
        );
    }

    #[test]
    fn candidate_failure_preserves_the_selected_dashboard_backend() {
        let old = DashboardBackend::new("generation-old", 4850, "old-manifest");
        let candidate = DashboardBackend::new("generation-new", 4851, "new-manifest");
        let mut registry = DashboardIngressRegistry::new(old.clone());

        registry.stage_candidate(candidate).unwrap();
        assert_eq!(registry.selected_backend(), &old);
        assert!(registry
            .commit_candidate(CandidateOperatorJourney::blocked("generation-new"))
            .is_err());
        assert_eq!(registry.selected_backend(), &old);
    }

    #[test]
    fn ready_operator_journey_commits_the_candidate_and_derives_a_presentation_receipt() {
        let old = DashboardBackend::new("generation-old", 4850, "old-manifest");
        let candidate = DashboardBackend::new("generation-new", 4851, "new-manifest");
        let mut registry = DashboardIngressRegistry::new(old);
        registry.stage_candidate(candidate.clone()).unwrap();

        let receipt = registry
            .commit_candidate(CandidateOperatorJourney::ready(ready_evidence(
                &candidate.generation_id,
            )))
            .unwrap();

        assert_eq!(registry.selected_backend(), &candidate);
        assert_eq!(
            receipt.state,
            crate::runtime_adoption::PresentationState::Ready
        );
        assert_eq!(receipt.dashboard_deployment_generation, "generation-new");
        assert_eq!(receipt.required_stream_provider, "rdp_gateway");
        assert!(receipt.reason_codes.is_empty());
        assert_eq!(registry.last_presentation_receipt(), Some(&receipt));
        assert_eq!(
            registry.fallback_backend().unwrap().generation_id,
            "generation-old"
        );
    }

    #[test]
    fn durable_handoff_receipt_derives_candidate_evidence_from_current_authority() {
        use crate::native::service_model::{
            DisplayAllocation, DurableHandoffPresentationReceipt, RemoteViewHandoff,
            RemoteViewRoute, ServiceState, ViewStreamProvider,
        };
        use crate::runtime_owner_transfer::{
            ProfileOwner, ProfileOwnerState, RuntimeOwnerRegistry,
        };

        let mut registry = DashboardIngressRegistry::new(DashboardBackend::new(
            "generation-old",
            4849,
            "old-manifest",
        ));
        registry
            .stage_candidate(DashboardBackend::new(
                "generation-new",
                4850,
                "candidate-manifest",
            ))
            .unwrap();
        let mut state = ServiceState {
            runtime_owner_registry: RuntimeOwnerRegistry::from_owner(ProfileOwner {
                owner_id: "owner-new".to_string(),
                profile_identity_digest: "profile-digest".to_string(),
                state: ProfileOwnerState::Ready,
                owner_generation: 4,
                browser_id: "browser-1".to_string(),
                daemon_session_route: "candidate-session".to_string(),
                process_instance_digest: "process-digest".to_string(),
                browser_family: "chrome".to_string(),
                cdp_endpoint_identity_digest: "cdp-digest".to_string(),
                target_set_digest: "target-set-digest".to_string(),
                pending_transfer: None,
                last_transition: None,
            }),
            ..ServiceState::default()
        };
        state.remote_view_handoffs.insert(
            "r1".to_string(),
            RemoteViewHandoff {
                id: "r1".to_string(),
                state: "ready".to_string(),
                browser_id: Some("browser-1".to_string()),
                session_name: Some("candidate-session".to_string()),
                target_id: Some("target-1".to_string()),
                last_route_id: Some("route-1".to_string()),
                last_display_allocation_id: Some("display-1".to_string()),
                presentation_receipt: Some(DurableHandoffPresentationReceipt {
                    schema_version: "agent-browser.durable-handoff-presentation.v1".to_string(),
                    generation: 3,
                    dashboard_deployment_generation: "generation-new".to_string(),
                    logical_browser_id: "browser-1".to_string(),
                    daemon_owner_generation: Some(4),
                    process_instance_digest: Some("process-digest".to_string()),
                    target_id: "target-1".to_string(),
                    required_stream_provider: ViewStreamProvider::RdpGateway,
                    observed_stream_provider: ViewStreamProvider::RdpGateway,
                    route_id: "route-1".to_string(),
                    display_allocation_id: "display-1".to_string(),
                    observed_at: "2026-08-17T18:30:00Z".to_string(),
                    state: "ready".to_string(),
                }),
                ..RemoteViewHandoff::default()
            },
        );
        state.remote_view_routes.insert(
            "route-1".to_string(),
            RemoteViewRoute {
                id: "route-1".to_string(),
                provider: ViewStreamProvider::RdpGateway,
                display_allocation_id: Some("display-1".to_string()),
                browser_id: Some("browser-1".to_string()),
                session_id: Some("candidate-session".to_string()),
                state: "ready".to_string(),
                ..RemoteViewRoute::default()
            },
        );
        state.display_allocations.insert(
            "display-1".to_string(),
            DisplayAllocation {
                id: "display-1".to_string(),
                owner_browser_id: Some("browser-1".to_string()),
                owner_session_id: Some("candidate-session".to_string()),
                state: "ready".to_string(),
                route_ids: vec!["route-1".to_string()],
                ..DisplayAllocation::default()
            },
        );

        let evidence = presentation_evidence_from_durable_handoff(&registry, &state, "r1").unwrap();

        assert_eq!(evidence.dashboard_deployment_generation, "generation-new");
        assert_eq!(evidence.daemon_generation, "owner-generation-4");
        assert_eq!(evidence.selected_target_generation, 3);
        assert_eq!(evidence.required_stream_provider, "rdp_gateway");
        assert!(evidence.receipt_id.starts_with("durable-handoff-"));
        assert!(authenticated_candidate_handoff_evidence(
            &registry,
            &state,
            "generation-new",
            "r1",
        )
        .unwrap()
        .is_some());
        assert!(authenticated_candidate_handoff_evidence(
            &registry,
            &state,
            "generation-old",
            "r1",
        )
        .unwrap()
        .is_none());

        let fixture_root = std::env::temp_dir().join(format!(
            "agent-browser-authenticated-candidate-commit-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&fixture_root).unwrap();
        let ingress_path = fixture_root.join("dashboard-ingress.json");
        let service_state_path = fixture_root.join("service-state.json");
        let listener = StdTcpListener::bind(("127.0.0.1", 0)).unwrap();
        let candidate_port = listener.local_addr().unwrap().port();
        let manifest = serde_json::json!({
            "schemaVersion": "agent-browser.runtime-manifest.v1",
            "generationId": "generation-new",
        });
        let manifest_body = manifest.to_string();
        let manifest_sha256 = format!("{:x}", Sha256::digest(manifest_body.as_bytes()));
        let manifest_server = thread::spawn(move || {
            let (mut connection, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = connection.read(&mut request).unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                manifest_body.len(),
                manifest_body
            );
            connection.write_all(response.as_bytes()).unwrap();
        });
        let repository = DashboardIngressRepository::new(&ingress_path);
        let initial = repository
            .initialize(DashboardBackend::new(
                "generation-old",
                candidate_port.saturating_add(1),
                "old-manifest",
            ))
            .unwrap();
        repository
            .stage_candidate(
                initial.revision,
                DashboardBackend::new("generation-new", candidate_port, manifest_sha256),
            )
            .unwrap();
        use crate::native::service_store::ServiceStateStore;
        crate::native::service_store::JsonServiceStateStore::new(&service_state_path)
            .save(&state)
            .unwrap();

        assert!(
            commit_authenticated_dashboard_candidate_from_handoff_at_paths(
                &ingress_path,
                &service_state_path,
                "generation-new",
                "r1",
            )
            .unwrap()
        );
        manifest_server.join().unwrap();
        let selected = repository.load().unwrap();
        assert_eq!(selected.selected_backend().generation_id, "generation-new");
        assert_eq!(
            selected
                .last_presentation_receipt()
                .unwrap()
                .dashboard_deployment_generation,
            "generation-new"
        );
        fs::remove_dir_all(fixture_root).unwrap();

        state.remote_view_routes.get_mut("route-1").unwrap().state = "orphaned".to_string();
        assert_eq!(
            presentation_evidence_from_durable_handoff(&registry, &state, "r1").unwrap_err(),
            "dashboard candidate durable handoff route is not ready"
        );
    }

    #[test]
    fn candidate_presentation_prerequisite_blocks_released_legacy_handoff_before_effects() {
        use crate::native::service_model::{
            DisplayAllocation, RemoteViewHandoff, RemoteViewRoute, ServiceState, ViewStreamProvider,
        };

        let state = ServiceState {
            remote_view_handoffs: std::collections::BTreeMap::from([(
                "r1".to_string(),
                RemoteViewHandoff {
                    id: "r1".to_string(),
                    state: "ready".to_string(),
                    browser_id: Some("browser-1".to_string()),
                    session_name: Some("owner-session".to_string()),
                    target_id: Some("target-1".to_string()),
                    last_route_id: Some("route-1".to_string()),
                    last_display_allocation_id: Some("display-1".to_string()),
                    view_stream_provider: Some(ViewStreamProvider::RdpGateway),
                    ..RemoteViewHandoff::default()
                },
            )]),
            remote_view_routes: std::collections::BTreeMap::from([(
                "route-1".to_string(),
                RemoteViewRoute {
                    id: "route-1".to_string(),
                    state: "released".to_string(),
                    browser_id: Some("browser-1".to_string()),
                    session_id: Some("owner-session".to_string()),
                    display_allocation_id: Some("display-1".to_string()),
                    provider: ViewStreamProvider::RdpGateway,
                    ..RemoteViewRoute::default()
                },
            )]),
            display_allocations: std::collections::BTreeMap::from([(
                "display-1".to_string(),
                DisplayAllocation {
                    id: "display-1".to_string(),
                    state: "released".to_string(),
                    owner_browser_id: Some("browser-1".to_string()),
                    owner_session_id: Some("owner-session".to_string()),
                    ..DisplayAllocation::default()
                },
            )]),
            ..ServiceState::default()
        };

        let prerequisite = candidate_presentation_prerequisite(&state);

        assert_eq!(prerequisite["ready"], false);
        assert_eq!(prerequisite["eligibleHandoffCount"], 0);
        assert_eq!(prerequisite["blockerCounts"]["route_not_ready"], 1);
        assert_eq!(
            prerequisite["nextAction"],
            "reconcile_exact_current_handoff_or_create_fresh_presentation_handoff"
        );
    }

    #[test]
    fn candidate_presentation_prerequisite_requires_current_browser_process_and_target_owner() {
        use crate::native::service_model::{
            DisplayAllocation, RemoteViewHandoff, RemoteViewRoute, ServiceState, ViewStreamProvider,
        };

        let state = ServiceState {
            remote_view_handoffs: std::collections::BTreeMap::from([(
                "r1".to_string(),
                RemoteViewHandoff {
                    id: "r1".to_string(),
                    state: "ready".to_string(),
                    browser_id: Some("browser-1".to_string()),
                    session_name: Some("owner-session".to_string()),
                    target_id: Some("target-1".to_string()),
                    last_route_id: Some("route-1".to_string()),
                    last_display_allocation_id: Some("display-1".to_string()),
                    view_stream_provider: Some(ViewStreamProvider::RdpGateway),
                    ..RemoteViewHandoff::default()
                },
            )]),
            remote_view_routes: std::collections::BTreeMap::from([(
                "route-1".to_string(),
                RemoteViewRoute {
                    id: "route-1".to_string(),
                    state: "ready".to_string(),
                    browser_id: Some("browser-1".to_string()),
                    session_id: Some("owner-session".to_string()),
                    display_allocation_id: Some("display-1".to_string()),
                    provider: ViewStreamProvider::RdpGateway,
                    ..RemoteViewRoute::default()
                },
            )]),
            display_allocations: std::collections::BTreeMap::from([(
                "display-1".to_string(),
                DisplayAllocation {
                    id: "display-1".to_string(),
                    state: "ready".to_string(),
                    owner_browser_id: Some("browser-1".to_string()),
                    owner_session_id: Some("owner-session".to_string()),
                    route_ids: vec!["route-1".to_string()],
                    ..DisplayAllocation::default()
                },
            )]),
            ..ServiceState::default()
        };

        let prerequisite = candidate_presentation_prerequisite(&state);

        assert_eq!(prerequisite["ready"], false);
        assert_eq!(prerequisite["eligibleHandoffCount"], 0);
        assert_eq!(prerequisite["blockerCounts"]["current_owner_unproven"], 1);
    }

    fn exact_candidate_presentation_state() -> crate::native::service_model::ServiceState {
        use crate::native::service_model::{
            BrowserHealth, BrowserProcess, DisplayAllocation, DurableHandoffPresentationReceipt,
            RemoteViewHandoff, RemoteViewRoute, ServiceBrowserProcessIdentity, ServiceState,
            ServiceTabHandle, ViewStreamProvider,
        };
        use crate::process_identity::RecordedProcessIdentity;
        use crate::runtime_owner_transfer::{
            ProfileOwner, ProfileOwnerState, RuntimeOwnerRegistry,
        };

        let process_identity = RecordedProcessIdentity {
            pid: 4242,
            start_token: "linux:boot:4242".to_string(),
            executable_path: Some("/opt/chrome".to_string()),
            browser_family: Some("chrome".to_string()),
        };
        let process_digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&process_identity).unwrap())
        );
        ServiceState {
            remote_view_handoffs: std::collections::BTreeMap::from([(
                "r1".to_string(),
                RemoteViewHandoff {
                    id: "r1".to_string(),
                    state: "ready".to_string(),
                    browser_id: Some("browser-1".to_string()),
                    session_name: Some("owner-session".to_string()),
                    target_id: Some("target-1".to_string()),
                    last_route_id: Some("route-1".to_string()),
                    last_display_allocation_id: Some("display-1".to_string()),
                    view_stream_provider: Some(ViewStreamProvider::RdpGateway),
                    presentation_receipt: Some(DurableHandoffPresentationReceipt {
                        schema_version: "agent-browser.durable-handoff-presentation.v1".to_string(),
                        generation: 3,
                        dashboard_deployment_generation: "generation-old".to_string(),
                        logical_browser_id: "browser-1".to_string(),
                        daemon_owner_generation: Some(4),
                        process_instance_digest: Some(process_digest.clone()),
                        target_id: "target-1".to_string(),
                        required_stream_provider: ViewStreamProvider::RdpGateway,
                        observed_stream_provider: ViewStreamProvider::RdpGateway,
                        route_id: "route-1".to_string(),
                        display_allocation_id: "display-1".to_string(),
                        observed_at: "2026-08-27T12:00:00Z".to_string(),
                        state: "ready".to_string(),
                    }),
                    ..RemoteViewHandoff::default()
                },
            )]),
            remote_view_routes: std::collections::BTreeMap::from([(
                "route-1".to_string(),
                RemoteViewRoute {
                    id: "route-1".to_string(),
                    state: "ready".to_string(),
                    browser_id: Some("browser-1".to_string()),
                    session_id: Some("owner-session".to_string()),
                    display_allocation_id: Some("display-1".to_string()),
                    provider: ViewStreamProvider::RdpGateway,
                    ..RemoteViewRoute::default()
                },
            )]),
            display_allocations: std::collections::BTreeMap::from([(
                "display-1".to_string(),
                DisplayAllocation {
                    id: "display-1".to_string(),
                    state: "ready".to_string(),
                    owner_browser_id: Some("browser-1".to_string()),
                    owner_session_id: Some("owner-session".to_string()),
                    route_ids: vec!["route-1".to_string()],
                    ..DisplayAllocation::default()
                },
            )]),
            browsers: std::collections::BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::Ready,
                    pid: Some(4242),
                    active_session_ids: vec!["owner-session".to_string()],
                    tab_handles: vec![ServiceTabHandle {
                        browser_id: "browser-1".to_string(),
                        session_name: Some("owner-session".to_string()),
                        target_id: Some("target-1".to_string()),
                        valid: true,
                        ..ServiceTabHandle::default()
                    }],
                    ..BrowserProcess::default()
                },
            )]),
            browser_process_identities: std::collections::BTreeMap::from([(
                "browser-1".to_string(),
                ServiceBrowserProcessIdentity {
                    process_identity,
                    user_data_dir: None,
                    runtime_profile: Some("profile-1".to_string()),
                },
            )]),
            runtime_owner_registry: RuntimeOwnerRegistry::from_owner(ProfileOwner {
                owner_id: "owner-1".to_string(),
                profile_identity_digest: "profile-digest".to_string(),
                state: ProfileOwnerState::Ready,
                owner_generation: 4,
                browser_id: "browser-1".to_string(),
                daemon_session_route: "owner-session".to_string(),
                process_instance_digest: process_digest,
                browser_family: "chrome".to_string(),
                cdp_endpoint_identity_digest: "cdp-digest".to_string(),
                target_set_digest: "target-digest".to_string(),
                pending_transfer: None,
                last_transition: None,
            }),
            ..ServiceState::default()
        }
    }

    #[test]
    fn candidate_presentation_prerequisite_rejects_cross_browser_route() {
        let mut state = exact_candidate_presentation_state();
        state
            .remote_view_routes
            .get_mut("route-1")
            .unwrap()
            .browser_id = Some("browser-other".to_string());

        let prerequisite = candidate_presentation_prerequisite(&state);

        assert_eq!(prerequisite["ready"], false);
        assert_eq!(prerequisite["blockerCounts"]["route_changed"], 1);
    }

    #[test]
    fn candidate_presentation_prerequisite_rejects_cross_browser_display() {
        let mut state = exact_candidate_presentation_state();
        state
            .display_allocations
            .get_mut("display-1")
            .unwrap()
            .owner_browser_id = Some("browser-other".to_string());

        let prerequisite = candidate_presentation_prerequisite(&state);

        assert_eq!(prerequisite["ready"], false);
        assert_eq!(prerequisite["blockerCounts"]["display_changed"], 1);
    }

    #[test]
    fn candidate_presentation_prerequisite_accepts_one_exact_current_handoff() {
        let state = exact_candidate_presentation_state();

        let prerequisite = candidate_presentation_prerequisite(&state);

        assert_eq!(prerequisite["ready"], true);
        assert_eq!(prerequisite["eligibleHandoffCount"], 1);
        assert_eq!(
            prerequisite["eligibleHandoffIds"],
            serde_json::json!(["r1"])
        );
        assert_eq!(
            prerequisite["nextAction"],
            "resolve_eligible_handoff_through_authenticated_candidate"
        );
    }

    #[test]
    fn candidate_presentation_prerequisite_requires_ready_current_receipt() {
        let mut state = exact_candidate_presentation_state();
        state
            .remote_view_handoffs
            .get_mut("r1")
            .unwrap()
            .presentation_receipt = None;

        let prerequisite = candidate_presentation_prerequisite(&state);

        assert_eq!(prerequisite["ready"], false);
        assert_eq!(
            prerequisite["blockerCounts"]["presentation_receipt_unready"],
            1
        );
    }

    #[test]
    fn candidate_presentation_bootstrap_accepts_adoptable_handoff_without_old_receipt_or_route() {
        let mut state = exact_candidate_presentation_state();
        let handoff = state.remote_view_handoffs.get_mut("r1").unwrap();
        handoff.presentation_receipt = None;
        state.remote_view_routes.get_mut("route-1").unwrap().state = "orphaned".to_string();
        state
            .display_allocations
            .get_mut("display-1")
            .unwrap()
            .state = "orphaned".to_string();

        let prerequisite = candidate_presentation_bootstrap_prerequisite(&state);

        assert_eq!(prerequisite["proofPhase"], "bootstrap");
        assert_eq!(prerequisite["ready"], true);
        assert_eq!(prerequisite["eligibleHandoffCount"], 1);
        assert_eq!(
            prerequisite["eligibleHandoffIds"],
            serde_json::json!(["r1"])
        );
        assert_eq!(prerequisite["candidateProofRequiredAfterStaging"], true);
        assert_eq!(
            prerequisite["nextAction"],
            "stage_candidate_then_resolve_eligible_handoff"
        );
    }

    #[test]
    fn candidate_presentation_bootstrap_accepts_exact_recoverable_precommit_transfer() {
        use crate::runtime_adoption::BrowserAdoptionMode;
        use crate::runtime_owner_transfer::{OwnerTransferProposal, OwnerTransferRequest};

        let mut state = exact_candidate_presentation_state();
        let owner = state
            .runtime_owner_registry
            .owners
            .values_mut()
            .next()
            .unwrap();
        owner.pending_transfer = Some(OwnerTransferProposal {
            request: OwnerTransferRequest {
                mode: BrowserAdoptionMode::CooperativeTransfer,
                logical_browser_id: owner.browser_id.clone(),
                profile_identity_digest: owner.profile_identity_digest.clone(),
                expected_owner_id: Some(owner.owner_id.clone()),
                expected_owner_generation: owner.owner_generation,
                candidate_owner_id: "candidate-owner".to_string(),
                candidate_daemon_session_route: "handoff-candidate".to_string(),
                process_instance_digest: owner.process_instance_digest.clone(),
                browser_family: owner.browser_family.clone(),
                cdp_endpoint_identity_digest: owner.cdp_endpoint_identity_digest.clone(),
                target_set_digest: owner.target_set_digest.clone(),
                selected_target_identity_digest: "selected-target-digest".to_string(),
                transfer_nonce_digest: "transfer-nonce-digest".to_string(),
            },
            previous_owner_generation: owner.owner_generation,
            candidate_owner_generation: owner.owner_generation + 1,
            candidate_effect_capable: false,
        });

        let prerequisite = candidate_presentation_bootstrap_prerequisite(&state);

        assert_eq!(prerequisite["ready"], true);
        assert_eq!(
            prerequisite["eligibleHandoffIds"],
            serde_json::json!(["r1"])
        );
    }

    #[test]
    fn candidate_presentation_bootstrap_rejects_handoff_without_current_owner() {
        let mut state = exact_candidate_presentation_state();
        state.runtime_owner_registry.owners.clear();

        let prerequisite = candidate_presentation_bootstrap_prerequisite(&state);

        assert_eq!(prerequisite["ready"], false);
        assert_eq!(prerequisite["eligibleHandoffCount"], 0);
        assert_eq!(prerequisite["blockerCounts"]["current_owner_unproven"], 1);
    }

    fn serve_runtime_targets_once(target_id: &str) -> (u16, thread::JoinHandle<()>) {
        use std::io::{Read, Write};

        let listener = StdTcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let target_id = target_id.to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            let body = serde_json::json!([{
                "id": target_id,
                "type": "page",
                "title": "retained page",
                "url": "about:blank"
            }])
            .to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        (port, server)
    }

    #[test]
    fn candidate_presentation_bootstrap_recovers_from_stale_projection_with_exact_live_evidence() {
        let mut state = exact_candidate_presentation_state();
        let process_identity =
            crate::process_identity::capture_process_identity(std::process::id(), None, None)
                .unwrap();
        let process_digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&process_identity).unwrap())
        );
        state
            .browser_process_identities
            .get_mut("browser-1")
            .unwrap()
            .process_identity = process_identity;
        let owner = state
            .runtime_owner_registry
            .owners
            .values_mut()
            .next()
            .unwrap();
        owner.process_instance_digest = process_digest;
        let (port, server) = serve_runtime_targets_once("target-1");
        let browser = state.browsers.get_mut("browser-1").unwrap();
        browser.pid = Some(std::process::id());
        browser.cdp_endpoint = Some(format!("ws://127.0.0.1:{port}/devtools/browser/retained"));
        browser.health = crate::native::service_model::BrowserHealth::CdpDisconnected;
        browser.tab_handles[0].valid = false;

        assert!(
            crate::native::remote_view_handoff::remote_view_handoff_ready_owner_session(
                &state,
                state.remote_view_handoffs.get("r1").unwrap()
            )
            .is_none()
        );

        let prerequisite = candidate_presentation_bootstrap_prerequisite(&state);
        server.join().unwrap();

        assert_eq!(prerequisite["ready"], true);
        assert_eq!(prerequisite["eligibleHandoffCount"], 1);
        assert_eq!(
            prerequisite["eligibleHandoffIds"],
            serde_json::json!(["r1"])
        );
    }

    #[test]
    fn candidate_presentation_bootstrap_rejects_stale_projection_when_live_target_differs() {
        let mut state = exact_candidate_presentation_state();
        let process_identity =
            crate::process_identity::capture_process_identity(std::process::id(), None, None)
                .unwrap();
        let process_digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&process_identity).unwrap())
        );
        state
            .browser_process_identities
            .get_mut("browser-1")
            .unwrap()
            .process_identity = process_identity;
        state
            .runtime_owner_registry
            .owners
            .values_mut()
            .next()
            .unwrap()
            .process_instance_digest = process_digest;
        let (port, server) = serve_runtime_targets_once("different-target");
        let browser = state.browsers.get_mut("browser-1").unwrap();
        browser.pid = Some(std::process::id());
        browser.cdp_endpoint = Some(format!("ws://127.0.0.1:{port}/devtools/browser/retained"));
        browser.health = crate::native::service_model::BrowserHealth::CdpDisconnected;
        browser.tab_handles[0].valid = false;

        let prerequisite = candidate_presentation_bootstrap_prerequisite(&state);
        server.join().unwrap();

        assert_eq!(prerequisite["ready"], false);
        assert_eq!(prerequisite["eligibleHandoffCount"], 0);
        assert_eq!(prerequisite["blockerCounts"]["current_owner_unproven"], 1);
    }

    #[test]
    fn post_commit_rollback_restores_the_authenticated_old_generation_receipt() {
        let bootstrap = DashboardBackend::new("generation-bootstrap", 4849, "bootstrap-manifest");
        let old = DashboardBackend::new("generation-old", 4850, "old-manifest");
        let candidate = DashboardBackend::new("generation-new", 4851, "new-manifest");
        let managed_candidate = DashboardBackend::new("generation-new", 4852, "new-manifest");
        let mut registry = DashboardIngressRegistry::new(bootstrap);
        registry.stage_candidate(old.clone()).unwrap();
        let mut old_evidence = ready_evidence(&old.generation_id);
        old_evidence.receipt_id = "presentation-old".to_string();
        let old_receipt = registry
            .commit_candidate(CandidateOperatorJourney::ready(old_evidence))
            .unwrap();
        registry.stage_candidate(candidate.clone()).unwrap();
        let mut candidate_evidence = ready_evidence(&candidate.generation_id);
        candidate_evidence.receipt_id = "presentation-new".to_string();
        registry
            .commit_candidate(CandidateOperatorJourney::ready(candidate_evidence))
            .unwrap();
        registry.stage_candidate(managed_candidate.clone()).unwrap();
        let mut managed_evidence = ready_evidence(&managed_candidate.generation_id);
        managed_evidence.receipt_id = "presentation-new-managed".to_string();
        registry
            .commit_candidate(CandidateOperatorJourney::ready(managed_evidence))
            .unwrap();

        registry
            .rollback_selected_candidate(&managed_candidate.generation_id)
            .unwrap();

        assert_eq!(registry.selected_backend(), &old);
        assert_eq!(registry.last_presentation_receipt(), Some(&old_receipt));
        assert!(registry.candidate_backend().is_none());
        assert_eq!(registry.fallback_backend(), Some(&managed_candidate));
    }

    #[test]
    fn recovery_retires_only_the_exact_failed_fallback_generation() {
        let path = temp_registry_path("retire-fallback");
        let repository = DashboardIngressRepository::new(&path);
        let old = DashboardBackend::new("generation-old", 4850, "old-manifest");
        let candidate = DashboardBackend::new("generation-new", 4851, "new-manifest");
        let mut registry = DashboardIngressRegistry::new(old.clone());
        registry.stage_candidate(candidate.clone()).unwrap();
        registry
            .commit_candidate(CandidateOperatorJourney::ready(ready_evidence(
                &candidate.generation_id,
            )))
            .unwrap();
        registry
            .rollback_selected_candidate(&candidate.generation_id)
            .unwrap();
        write_registry_atomic(&path, &registry).unwrap();
        let rolled_back = repository.load().unwrap();

        assert_eq!(rolled_back.selected_backend(), &old);
        assert_eq!(rolled_back.fallback_backend(), Some(&candidate));
        assert!(repository
            .retire_fallback(rolled_back.revision, "generation-other")
            .unwrap_err()
            .contains("fallback generation changed"));

        let retired = repository
            .retire_fallback(rolled_back.revision, &candidate.generation_id)
            .unwrap();
        assert!(retired.fallback_backend().is_none());
        assert!(retired.fallback_presentation_receipt.is_none());
        assert_eq!(retired.selected_backend(), &old);
        let _ = fs::remove_file(path.with_extension("json.lock"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn repository_rejects_a_candidate_that_failed_after_staging() {
        let path = temp_registry_path("failed-candidate");
        let repository = DashboardIngressRepository::new(&path);
        let old = DashboardBackend::new("generation-old", 4850, "old-manifest");
        let probe = StdTcpListener::bind(("127.0.0.1", 0)).unwrap();
        let unavailable_port = probe.local_addr().unwrap().port();
        drop(probe);
        let candidate = DashboardBackend::new("generation-new", unavailable_port, "new-manifest");

        let initial = repository.initialize(old.clone()).unwrap();
        let staged = repository
            .stage_candidate(initial.revision, candidate)
            .unwrap();
        let error = repository
            .commit_candidate(
                staged.revision,
                CandidateOperatorJourney::ready(ready_evidence("generation-new")),
            )
            .unwrap_err();

        assert!(error.contains("unavailable"));
        let preserved = repository.load().unwrap();
        assert_eq!(preserved.selected_backend(), &old);
        assert!(preserved.fallback_backend().is_none());
        let _ = fs::remove_file(path.with_extension("json.lock"));
        let _ = fs::remove_file(path);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ingress_request_registry_load_does_not_block_on_writer_lock() {
        let path = temp_registry_path("request-load-writer-lock");
        let repository = DashboardIngressRepository::new(&path);
        repository
            .initialize(DashboardBackend::new("generation-1", 4850, "manifest-1"))
            .unwrap();
        let writer_lock = acquire_ingress_lock(&path).unwrap();

        let loaded = tokio::time::timeout(
            Duration::from_millis(100),
            load_ingress_registry_for_request(repository.clone()),
        )
        .await;

        drop(writer_lock);
        assert!(
            matches!(loaded, Ok(Ok(registry)) if registry.selected_backend().generation_id == "generation-1"),
            "ingress request reads must use the last atomically committed registry without blocking the async runtime"
        );
        let _ = fs::remove_file(path.with_extension("json.lock"));
        let _ = fs::remove_file(path);
    }

    #[tokio::test]
    async fn selected_backend_failure_serves_a_safe_request_from_the_committed_fallback() {
        let fallback_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let fallback_port = fallback_listener.local_addr().unwrap().port();
        let unused = StdTcpListener::bind(("127.0.0.1", 0)).unwrap();
        let unavailable_port = unused.local_addr().unwrap().port();
        drop(unused);
        let old = DashboardBackend::new("generation-old", fallback_port, "old-manifest");
        let candidate = DashboardBackend::new("generation-new", unavailable_port, "new-manifest");
        let mut registry = DashboardIngressRegistry::new(old);
        registry.stage_candidate(candidate).unwrap();
        registry
            .commit_candidate(CandidateOperatorJourney::ready(ready_evidence(
                "generation-new",
            )))
            .unwrap();
        let fallback = tokio::spawn(async move {
            let (mut connection, _) = fallback_listener.accept().await.unwrap();
            let request = read_initial_http_request(&mut connection).await.unwrap();
            assert!(request.starts_with(b"GET /api/runtime/manifest "));
            connection
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\nfallback",
                )
                .await
                .unwrap();
        });
        let ingress_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let ingress_port = ingress_listener.local_addr().unwrap().port();
        let ingress = tokio::spawn(async move {
            let (mut connection, _) = ingress_listener.accept().await.unwrap();
            proxy_ingress_request(&mut connection, &registry).await;
        });
        let mut client = TcpStream::connect(("127.0.0.1", ingress_port))
            .await
            .unwrap();
        client
            .write_all(b"GET /api/runtime/manifest HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        client.shutdown().await.unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();

        fallback.await.unwrap();
        ingress.await.unwrap();
        assert!(response.ends_with(b"fallback"));
    }

    #[tokio::test]
    async fn selected_backend_failure_does_not_replay_a_mutation_to_fallback() {
        let fallback_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let fallback_port = fallback_listener.local_addr().unwrap().port();
        let unused = StdTcpListener::bind(("127.0.0.1", 0)).unwrap();
        let unavailable_port = unused.local_addr().unwrap().port();
        drop(unused);
        let old = DashboardBackend::new("generation-old", fallback_port, "old-manifest");
        let candidate = DashboardBackend::new("generation-new", unavailable_port, "new-manifest");
        let mut registry = DashboardIngressRegistry::new(old);
        registry.stage_candidate(candidate).unwrap();
        registry
            .commit_candidate(CandidateOperatorJourney::ready(ready_evidence(
                "generation-new",
            )))
            .unwrap();
        let ingress_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let ingress_port = ingress_listener.local_addr().unwrap().port();
        let ingress = tokio::spawn(async move {
            let (mut connection, _) = ingress_listener.accept().await.unwrap();
            proxy_ingress_request(&mut connection, &registry).await;
        });
        let mut client = TcpStream::connect(("127.0.0.1", ingress_port))
            .await
            .unwrap();
        client
            .write_all(
                b"POST /api/service/request HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n\r\n{}",
            )
            .await
            .unwrap();
        client.shutdown().await.unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();

        ingress.await.unwrap();
        assert!(String::from_utf8_lossy(&response).starts_with("HTTP/1.1 503"));
        assert!(
            timeout(Duration::from_millis(50), fallback_listener.accept())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn delivered_mutation_without_backend_response_is_not_reported_retryable() {
        let backend_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let backend_port = backend_listener.local_addr().unwrap().port();
        let registry = DashboardIngressRegistry::new(DashboardBackend::new(
            "generation-selected",
            backend_port,
            "selected-manifest",
        ));
        let backend = tokio::spawn(async move {
            let (mut connection, _) = backend_listener.accept().await.unwrap();
            let request = read_initial_http_request(&mut connection).await.unwrap();
            assert!(request.starts_with(b"POST /api/service/request "));
        });
        let ingress_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let ingress_port = ingress_listener.local_addr().unwrap().port();
        let ingress = tokio::spawn(async move {
            let (mut connection, _) = ingress_listener.accept().await.unwrap();
            proxy_ingress_request(&mut connection, &registry).await;
        });
        let mut client = TcpStream::connect(("127.0.0.1", ingress_port))
            .await
            .unwrap();
        client
            .write_all(
                b"POST /api/service/request HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n\r\n{}",
            )
            .await
            .unwrap();
        client.shutdown().await.unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();

        backend.await.unwrap();
        ingress.await.unwrap();
        let response = String::from_utf8(response).unwrap();
        assert!(response.starts_with("HTTP/1.1 502"));
        assert!(response.contains("mutation_outcome_unknown"));
        assert!(response.contains("\"retrySafe\":false"));
        assert!(!response.contains("Retry-After"));
    }

    #[tokio::test]
    async fn committed_service_mutation_waits_for_its_bounded_backend_response() {
        let backend_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let backend_port = backend_listener.local_addr().unwrap().port();
        let registry = DashboardIngressRegistry::new(DashboardBackend::new(
            "generation-selected",
            backend_port,
            "selected-manifest",
        ));
        let backend = tokio::spawn(async move {
            let (mut connection, _) = backend_listener.accept().await.unwrap();
            let request = read_initial_http_request(&mut connection).await.unwrap();
            assert!(request.starts_with(b"POST /api/service/request "));
            tokio::time::sleep(Duration::from_millis(2_100)).await;
            connection
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\ncommitted",
                )
                .await
                .unwrap();
        });
        let ingress_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let ingress_port = ingress_listener.local_addr().unwrap().port();
        let ingress = tokio::spawn(async move {
            let (mut connection, _) = ingress_listener.accept().await.unwrap();
            proxy_ingress_request(&mut connection, &registry).await;
        });
        let body = serde_json::json!({
            "action": "tab_new",
            "params": {"url": "about:blank"},
            "jobTimeoutMs": 30_000,
        })
        .to_string();
        let request = format!(
            "POST /api/service/request HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let mut client = TcpStream::connect(("127.0.0.1", ingress_port))
            .await
            .unwrap();
        client.write_all(request.as_bytes()).await.unwrap();
        client.shutdown().await.unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();

        backend.await.unwrap();
        ingress.await.unwrap();
        assert!(response.ends_with(b"committed"));
    }

    #[test]
    fn pressure_sensitive_dashboard_reads_get_a_tolerant_first_response_timeout() {
        assert_eq!(
            dashboard_ingress_first_response_timeout(
                b"GET /api/service/status HTTP/1.1\r\nHost: localhost\r\n\r\n"
            ),
            Duration::from_secs(10)
        );
        assert_eq!(
            dashboard_ingress_first_response_timeout(
                b"GET /api/service/resources HTTP/1.1\r\nHost: localhost\r\n\r\n"
            ),
            Duration::from_secs(10)
        );
        assert_eq!(
            dashboard_ingress_first_response_timeout(
                b"GET /api/service/browser-capability-registry HTTP/1.1\r\nHost: localhost\r\n\r\n"
            ),
            Duration::from_secs(10)
        );
        assert_eq!(
            dashboard_ingress_first_response_timeout(
                b"GET /api/session-tabs?port=9222 HTTP/1.1\r\nHost: localhost\r\n\r\n"
            ),
            Duration::from_secs(10)
        );
        assert_eq!(
            dashboard_ingress_first_response_timeout(
                b"GET /api/models HTTP/1.1\r\nHost: localhost\r\n\r\n"
            ),
            Duration::from_secs(10)
        );
        assert_eq!(
            dashboard_ingress_first_response_timeout(
                b"HEAD /favicon.ico HTTP/1.1\r\nHost: localhost\r\n\r\n"
            ),
            Duration::from_secs(10)
        );
        assert_eq!(
            dashboard_ingress_first_response_timeout(
                b"GET /api/runtime/manifest HTTP/1.1\r\nHost: localhost\r\n\r\n"
            ),
            DASHBOARD_INGRESS_SERVICE_STATUS_FIRST_RESPONSE_TIMEOUT
        );
    }

    #[tokio::test]
    async fn presentation_startup_waits_for_the_bounded_backend_response() {
        for (path, payload) in [
            (
                "/api/service/request",
                serde_json::json!({
                    "action": "service_remote_view_handoff_resolve",
                    "params": {"handoffId": "handoff-a"},
                    "jobTimeoutMs": 90_000,
                }),
            ),
            (
                "/api/guacamole-primary-claim",
                serde_json::json!({
                    "operation": "ensure", "routeId": "route-a", "connectionId": "1",
                }),
            ),
        ] {
            let backend_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            let backend_port = backend_listener.local_addr().unwrap().port();
            let registry = DashboardIngressRegistry::new(DashboardBackend::new(
                "generation-selected",
                backend_port,
                "selected-manifest",
            ));
            let backend = tokio::spawn(async move {
                let (mut connection, _) = backend_listener.accept().await.unwrap();
                let request = read_initial_http_request(&mut connection).await.unwrap();
                assert!(request.starts_with(format!("POST {path} ").as_bytes()));
                tokio::time::sleep(Duration::from_millis(2_100)).await;
                let _ = connection
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nready",
                    )
                    .await;
            });
            let ingress_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            let ingress_port = ingress_listener.local_addr().unwrap().port();
            let ingress = tokio::spawn(async move {
                let (mut connection, _) = ingress_listener.accept().await.unwrap();
                proxy_ingress_request(&mut connection, &registry).await;
            });
            let body = payload.to_string();
            let request = format!(
                "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let mut client = TcpStream::connect(("127.0.0.1", ingress_port))
                .await
                .unwrap();
            client.write_all(request.as_bytes()).await.unwrap();
            client.shutdown().await.unwrap();
            let mut response = Vec::new();
            client.read_to_end(&mut response).await.unwrap();

            backend.await.unwrap();
            ingress.await.unwrap();
            assert!(
                response.ends_with(b"ready"),
                "{path}: {}",
                String::from_utf8_lossy(&response)
            );
        }
    }

    #[test]
    fn candidate_manifest_probe_binds_the_staged_generation_to_live_bytes() {
        let listener = StdTcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let manifest = serde_json::json!({
            "schemaVersion": "agent-browser.runtime-manifest.v1",
            "packageVersion": "0.28.0"
        });
        let body = manifest.to_string();
        let expected_sha256 = format!("{:x}", Sha256::digest(body.as_bytes()));
        let server = thread::spawn(move || {
            let (mut connection, _) = listener.accept().unwrap();
            let mut request = [0_u8; 512];
            let _ = connection.read(&mut request).unwrap();
            write!(
                connection,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        validate_dashboard_backend(&DashboardBackend::new(
            "generation-new",
            port,
            expected_sha256,
        ))
        .unwrap();
        server.join().unwrap();
    }

    #[test]
    fn repository_compare_and_swap_persists_one_selected_backend() {
        let path = temp_registry_path("cas");
        let repository = DashboardIngressRepository::new(&path);
        let old = DashboardBackend::new("generation-old", 4850, "old-manifest");
        let candidate = DashboardBackend::new("generation-new", 4851, "new-manifest");

        let initial = repository.initialize(old.clone()).unwrap();
        let staged = repository
            .stage_candidate(initial.revision, candidate.clone())
            .unwrap();
        assert_eq!(staged.selected_backend(), &old);
        assert_eq!(staged.candidate_backend(), Some(&candidate));
        assert!(repository
            .rollback_candidate(initial.revision, "generation-new")
            .unwrap_err()
            .contains("revision changed"));

        let rolled_back = repository
            .rollback_candidate(staged.revision, "generation-new")
            .unwrap();
        assert_eq!(rolled_back.selected_backend(), &old);
        assert!(rolled_back.candidate_backend().is_none());
        assert_eq!(repository.load().unwrap(), rolled_back);

        let _ = fs::remove_file(path.with_extension("json.lock"));
        let _ = fs::remove_file(path);
    }
}
