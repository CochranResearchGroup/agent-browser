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
    last_presentation_receipt: Option<crate::runtime_adoption::PresentationReceipt>,
}

impl DashboardIngressRegistry {
    pub(crate) fn new(selected_backend: DashboardBackend) -> Self {
        Self {
            schema_version: DASHBOARD_INGRESS_SCHEMA_VERSION.to_string(),
            revision: 1,
            selected_backend,
            candidate_backend: None,
            fallback_backend: None,
            last_presentation_receipt: None,
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
        self.fallback_backend = Some(prior);
        self.candidate_backend = None;
        self.last_presentation_receipt = Some(receipt.clone());
        self.revision = self.revision.saturating_add(1);
        Ok(receipt)
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
        let _lock = acquire_ingress_lock(&self.path)?;
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
    let manifest = crate::native::stream::runtime_manifest_json();
    let manifest_sha256 = format!("{:x}", Sha256::digest(manifest.to_string().as_bytes()));
    let generation_id = std::env::var("AGENT_BROWSER_DASHBOARD_GENERATION")
        .unwrap_or_else(|_| format!("dashboard-{}", env!("CARGO_PKG_VERSION")));
    DashboardBackend::new(generation_id, port, manifest_sha256)
}

/// Projects public-safe selector and presentation readiness for status and doctor.
pub(crate) fn dashboard_ingress_status_json() -> serde_json::Value {
    let repository = DashboardIngressRepository::new(DashboardIngressRepository::default_path());
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
            let registry = match repository.load() {
                Ok(registry) => registry,
                Err(error) => {
                    write_ingress_unavailable(&mut client, "registry_unavailable", &error).await;
                    return;
                }
            };
            proxy_ingress_request(&mut client, &registry).await;
        });
    }
}

async fn proxy_ingress_request(client: &mut TcpStream, registry: &DashboardIngressRegistry) {
    let request = match read_initial_http_request(client).await {
        Ok(request) => request,
        Err(error) => {
            write_ingress_unavailable(client, "invalid_ingress_request", &error).await;
            return;
        }
    };
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
    let mut attempts = vec![registry.selected_backend()];
    if retry_safe {
        if let Some(fallback) = registry.fallback_backend() {
            attempts.push(fallback);
        }
    }
    let mut failures = Vec::new();
    for backend in attempts {
        match attempt_dashboard_backend(backend, &backend_request).await {
            Ok((mut connection, first_response)) => {
                if client.write_all(&first_response).await.is_ok() {
                    proxy_ingress_connection(client, &mut connection).await;
                }
                return;
            }
            Err(error) => failures.push(format!("{}: {error}", backend.generation_id)),
        }
    }
    write_ingress_unavailable(
        client,
        "selected_backend_unavailable",
        &format!(
            "selected dashboard generation {} is converging: {}",
            registry.selected_backend().generation_id,
            failures.join("; ")
        ),
    )
    .await;
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

fn validate_dashboard_backend(backend: &DashboardBackend) -> Result<(), String> {
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
) -> Result<(TcpStream, Vec<u8>), String> {
    let mut connection = timeout(
        Duration::from_secs(2),
        TcpStream::connect(("127.0.0.1", backend.port)),
    )
    .await
    .map_err(|_| "connect timed out".to_string())?
    .map_err(|error| format!("connect failed: {error}"))?;
    connection
        .write_all(request)
        .await
        .map_err(|error| format!("request write failed: {error}"))?;
    let mut first_response = vec![0_u8; 8192];
    let count = timeout(Duration::from_secs(2), connection.read(&mut first_response))
        .await
        .map_err(|_| "first response byte timed out".to_string())?
        .map_err(|error| format!("response read failed: {error}"))?;
    if count == 0 {
        return Err("backend closed before returning a response".to_string());
    }
    first_response.truncate(count);
    Ok((connection, first_response))
}

async fn proxy_ingress_connection(client: &mut TcpStream, backend: &mut TcpStream) {
    let _ = tokio::io::copy_bidirectional(client, backend).await;
    let _ = client.shutdown().await;
    let _ = backend.shutdown().await;
}

async fn write_ingress_unavailable(client: &mut TcpStream, code: &str, message: &str) {
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
