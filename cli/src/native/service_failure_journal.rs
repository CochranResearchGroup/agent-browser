//! Append-only, privacy-bounded failure evidence for service postmortems.
//!
//! The operational service event collection is intentionally bounded. This
//! journal is separate so pruning or recovery of `state.json` cannot erase a
//! failure occurrence. Callers must pass identifiers and summaries, never
//! credentials, page content, request headers, or raw operator URLs.

use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const FAILURE_JOURNAL_FILENAME: &str = "failure-journal.jsonl";
const MAX_TEXT_BYTES: usize = 1_024;
const MAX_DETAILS_BYTES: usize = 8_192;
const FAILURE_JOURNAL_QUEUE_CAPACITY: usize = 256;

#[derive(Default)]
struct FailureJournalDeliveryCounters {
    write_failures: AtomicU64,
    queue_overloads: AtomicU64,
    delivery_failures: AtomicU64,
}

type FailureJournalSink =
    Arc<dyn Fn(&ServiceFailureRecord) -> Result<(), String> + Send + Sync + 'static>;

enum FailureJournalCommand {
    Append(Box<ServiceFailureRecord>),
    Flush(SyncSender<()>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureJournalEnqueueResult {
    Accepted,
    Backpressured,
    Unavailable,
}

#[derive(Clone)]
struct FailureJournalDispatcher {
    sender: SyncSender<FailureJournalCommand>,
    counters: Arc<FailureJournalDeliveryCounters>,
}

impl FailureJournalDispatcher {
    fn start(
        capacity: usize,
        sink: FailureJournalSink,
        counters: Arc<FailureJournalDeliveryCounters>,
    ) -> Result<Self, String> {
        let (sender, receiver) = sync_channel::<FailureJournalCommand>(capacity.max(1));
        let worker_counters = counters.clone();
        thread::Builder::new()
            .name("agent-browser-failure-journal".to_string())
            .spawn(move || {
                while let Ok(command) = receiver.recv() {
                    match command {
                        FailureJournalCommand::Append(record) => {
                            let delivered =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    sink(&record)
                                }));
                            if !matches!(delivered, Ok(Ok(()))) {
                                worker_counters
                                    .write_failures
                                    .fetch_add(1, Ordering::Relaxed);
                                worker_counters
                                    .delivery_failures
                                    .fetch_add(1, Ordering::Relaxed);
                                eprintln!(
                                    "agent_browser_failure_journal_delivery event=write_failed"
                                );
                            }
                        }
                        FailureJournalCommand::Flush(acknowledge) => {
                            let _ = acknowledge.send(());
                        }
                    }
                }
            })
            .map_err(|error| format!("failed to start service failure journal worker: {error}"))?;
        Ok(Self { sender, counters })
    }

    fn enqueue(&self, record: &ServiceFailureRecord) -> FailureJournalEnqueueResult {
        match self
            .sender
            .try_send(FailureJournalCommand::Append(Box::new(record.clone())))
        {
            Ok(()) => FailureJournalEnqueueResult::Accepted,
            Err(TrySendError::Full(FailureJournalCommand::Append(record))) => {
                self.counters
                    .queue_overloads
                    .fetch_add(1, Ordering::Relaxed);
                eprintln!("agent_browser_failure_journal_delivery event=queue_backpressure");
                match self.sender.send(FailureJournalCommand::Append(record)) {
                    Ok(()) => FailureJournalEnqueueResult::Backpressured,
                    Err(_) => {
                        self.counters.write_failures.fetch_add(1, Ordering::Relaxed);
                        self.counters
                            .delivery_failures
                            .fetch_add(1, Ordering::Relaxed);
                        eprintln!(
                            "agent_browser_failure_journal_delivery event=worker_unavailable"
                        );
                        FailureJournalEnqueueResult::Unavailable
                    }
                }
            }
            Err(TrySendError::Disconnected(_)) => {
                self.counters.write_failures.fetch_add(1, Ordering::Relaxed);
                self.counters
                    .delivery_failures
                    .fetch_add(1, Ordering::Relaxed);
                eprintln!("agent_browser_failure_journal_delivery event=worker_unavailable");
                FailureJournalEnqueueResult::Unavailable
            }
            Err(TrySendError::Full(FailureJournalCommand::Flush(_))) => unreachable!(),
        }
    }

    fn flush(&self) -> Result<(), String> {
        let (acknowledge, acknowledged) = sync_channel(0);
        self.sender
            .send(FailureJournalCommand::Flush(acknowledge))
            .map_err(|_| "service failure journal worker is unavailable".to_string())?;
        acknowledged
            .recv()
            .map_err(|_| "service failure journal worker stopped before flush".to_string())
    }
}

fn flush_failure_journal_best_effort() {
    if let Ok(dispatcher) = failure_journal_dispatcher() {
        if dispatcher.flush().is_err() {
            let counters = failure_journal_delivery_counters();
            counters.write_failures.fetch_add(1, Ordering::Relaxed);
            counters.delivery_failures.fetch_add(1, Ordering::Relaxed);
            eprintln!("agent_browser_failure_journal_delivery event=flush_failed");
        }
    }
}

fn failure_journal_delivery_counters() -> &'static Arc<FailureJournalDeliveryCounters> {
    static COUNTERS: OnceLock<Arc<FailureJournalDeliveryCounters>> = OnceLock::new();
    COUNTERS.get_or_init(|| Arc::new(FailureJournalDeliveryCounters::default()))
}

fn failure_journal_dispatcher() -> &'static Result<FailureJournalDispatcher, String> {
    static DISPATCHER: OnceLock<Result<FailureJournalDispatcher, String>> = OnceLock::new();
    DISPATCHER.get_or_init(|| {
        let counters = failure_journal_delivery_counters().clone();
        FailureJournalDispatcher::start(
            FAILURE_JOURNAL_QUEUE_CAPACITY,
            Arc::new(|record| append_service_failure(record).map(|_| ())),
            counters,
        )
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceFailureCategory {
    BrowserLaunch,
    GuacamoleLoad,
    HandoffLink,
    CdpStream,
    DashboardAction,
    ServiceAction,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceFailureReferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_environment_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_lane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff_id_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceFailureRecord {
    pub schema_version: String,
    pub occurrence_id: String,
    pub occurred_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_epoch: Option<String>,
    pub runtime_environment: String,
    pub category: ServiceFailureCategory,
    pub source: String,
    pub stage: String,
    pub code: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    pub references: ServiceFailureReferences,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

/// Restricted report accepted from an authenticated dashboard client when the
/// failure is visible only outside the service process.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClientFailureObservation {
    pub category: ServiceFailureCategory,
    pub stage: String,
    pub code: String,
    pub summary: String,
    pub action: Option<String>,
    pub observation_id: String,
    pub browser_id: Option<String>,
    pub profile_id: Option<String>,
    pub session_id: Option<String>,
    pub route_id: Option<String>,
    pub display_id: Option<String>,
    pub handoff_id_hash: Option<String>,
    pub stream_provider: Option<String>,
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceFailureJournalReadback {
    pub schema_version: String,
    pub records: Vec<ServiceFailureRecord>,
    pub malformed_line_count: u64,
    pub write_failure_count: u64,
    pub delivery_overload_count: u64,
    pub delivery_failure_count: u64,
}

impl ServiceFailureRecord {
    pub fn new(
        category: ServiceFailureCategory,
        source: impl Into<String>,
        stage: impl Into<String>,
        code: impl Into<String>,
        summary: impl AsRef<str>,
    ) -> Self {
        Self {
            schema_version: "agent-browser.service-failure-record.v1".to_string(),
            occurrence_id: uuid::Uuid::new_v4().to_string(),
            occurred_at: Utc::now().to_rfc3339(),
            boot_epoch: crate::process_identity::current_boot_epoch(),
            runtime_environment: std::env::var("AGENT_BROWSER_RUNTIME_ENVIRONMENT")
                .ok()
                .filter(|value| matches!(value.as_str(), "development" | "production"))
                .unwrap_or_else(|| "production".to_string()),
            category,
            source: bounded_token(source.into(), "unknown"),
            stage: bounded_token(stage.into(), "unknown"),
            code: bounded_token(code.into(), "unknown_failure"),
            summary: redact_summary(summary.as_ref()),
            action: None,
            references: ServiceFailureReferences::default(),
            details: None,
        }
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(bounded_token(action.into(), "unknown"));
        self
    }

    pub fn with_references(mut self, references: ServiceFailureReferences) -> Self {
        self.references = ServiceFailureReferences {
            runtime_environment_id: safe_reference(references.runtime_environment_id),
            runtime_lane_id: safe_reference(references.runtime_lane_id),
            request_id: safe_reference(references.request_id),
            job_id: safe_reference(references.job_id),
            trace_id: safe_reference(references.trace_id),
            browser_id: safe_reference(references.browser_id),
            profile_id: safe_reference(references.profile_id),
            session_id: safe_reference(references.session_id),
            route_id: safe_reference(references.route_id),
            display_id: safe_reference(references.display_id),
            handoff_id_hash: references
                .handoff_id_hash
                .and_then(|value| bounded_hash(value).ok()),
        };
        self
    }

    pub fn with_details(mut self, details: Value) -> Self {
        if details.is_object()
            && serde_json::to_vec(&details).is_ok_and(|encoded| encoded.len() <= MAX_DETAILS_BYTES)
        {
            self.details = Some(details);
        }
        self
    }
}

/// Append one record to the default user-scoped journal.
///
/// This is deliberately best-effort at call sites: logging failure must not
/// hide or replace the primary product failure being reported.
pub fn append_service_failure(record: &ServiceFailureRecord) -> Result<PathBuf, String> {
    let path = default_failure_journal_path()?;
    append_service_failure_at(&path, record)?;
    Ok(path)
}

pub fn append_service_failure_best_effort(record: &ServiceFailureRecord) {
    let occurrence_id = serde_json::to_string(&record.occurrence_id)
        .unwrap_or_else(|_| "\"unavailable\"".to_string());
    eprintln!(
        "agent_browser_service_failure event=observed occurrence_id={} category={:?}",
        occurrence_id, record.category
    );
    match failure_journal_dispatcher() {
        Ok(dispatcher) => {
            let _ = dispatcher.enqueue(record);
        }
        Err(_) => {
            let counters = failure_journal_delivery_counters();
            counters.write_failures.fetch_add(1, Ordering::Relaxed);
            counters.delivery_failures.fetch_add(1, Ordering::Relaxed);
            eprintln!("agent_browser_failure_journal_delivery event=worker_start_failed");
        }
    }
}

pub fn failure_journal_write_failure_count() -> u64 {
    failure_journal_delivery_counters()
        .write_failures
        .load(Ordering::Relaxed)
}

pub fn failure_journal_delivery_overload_count() -> u64 {
    failure_journal_delivery_counters()
        .queue_overloads
        .load(Ordering::Relaxed)
}

pub fn failure_journal_delivery_failure_count() -> u64 {
    failure_journal_delivery_counters()
        .delivery_failures
        .load(Ordering::Relaxed)
}

pub fn read_service_failures(limit: usize) -> Result<ServiceFailureJournalReadback, String> {
    flush_failure_journal_best_effort();
    let path = default_failure_journal_path()?;
    read_service_failures_at(&path, limit)
}

pub fn read_service_failures_at(
    path: &Path,
    limit: usize,
) -> Result<ServiceFailureJournalReadback, String> {
    let limit = limit.clamp(1, 1_000);
    if !path.exists() {
        return Ok(ServiceFailureJournalReadback {
            schema_version: "agent-browser.service-failure-journal-readback.v1".to_string(),
            records: Vec::new(),
            malformed_line_count: 0,
            write_failure_count: failure_journal_write_failure_count(),
            delivery_overload_count: failure_journal_delivery_overload_count(),
            delivery_failure_count: failure_journal_delivery_failure_count(),
        });
    }
    let file = OpenOptions::new().read(true).open(path).map_err(|error| {
        format!(
            "failed to open service failure journal {}: {error}",
            path.display()
        )
    })?;
    file.lock_shared().map_err(|error| {
        format!(
            "failed to read-lock service failure journal {}: {error}",
            path.display()
        )
    })?;
    let mut records = VecDeque::with_capacity(limit);
    let mut malformed_line_count = 0_u64;
    for line in BufReader::new(file).lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => {
                malformed_line_count += 1;
                continue;
            }
        };
        match serde_json::from_str::<ServiceFailureRecord>(&line) {
            Ok(record) => {
                if records.len() == limit {
                    records.pop_front();
                }
                records.push_back(record);
            }
            Err(_) => malformed_line_count += 1,
        }
    }
    Ok(ServiceFailureJournalReadback {
        schema_version: "agent-browser.service-failure-journal-readback.v1".to_string(),
        records: records.into(),
        malformed_line_count,
        write_failure_count: failure_journal_write_failure_count(),
        delivery_overload_count: failure_journal_delivery_overload_count(),
        delivery_failure_count: failure_journal_delivery_failure_count(),
    })
}

pub fn record_client_failure_observation(
    body: &str,
    authenticated_actor: &str,
) -> Result<ServiceFailureRecord, String> {
    let record = build_client_failure_observation(body, authenticated_actor)?;
    append_service_failure(&record)?;
    Ok(record)
}

fn build_client_failure_observation(
    body: &str,
    authenticated_actor: &str,
) -> Result<ServiceFailureRecord, String> {
    let observation: ClientFailureObservation = serde_json::from_str(body)
        .map_err(|error| format!("invalid failure observation: {error}"))?;
    if !matches!(
        observation.category,
        ServiceFailureCategory::GuacamoleLoad
            | ServiceFailureCategory::HandoffLink
            | ServiceFailureCategory::CdpStream
            | ServiceFailureCategory::DashboardAction
    ) {
        return Err("client failure category is not observable by the dashboard".to_string());
    }
    for (name, value, max_len) in [
        ("stage", observation.stage.as_str(), 128),
        ("code", observation.code.as_str(), 128),
        ("summary", observation.summary.as_str(), MAX_TEXT_BYTES),
        ("observationId", observation.observation_id.as_str(), 256),
    ] {
        if value.trim().is_empty() || value.len() > max_len {
            return Err(format!("failure observation {name} is empty or too long"));
        }
    }
    if observation
        .action
        .as_deref()
        .is_some_and(|action| action.trim().is_empty() || action.len() > 128)
    {
        return Err("failure observation action is empty or too long".to_string());
    }
    let record = ServiceFailureRecord::new(
        observation.category,
        "authenticated_dashboard_client",
        observation.stage,
        observation.code,
        observation.summary,
    )
    .with_references(ServiceFailureReferences {
        browser_id: observation.browser_id.map(bounded_identifier).transpose()?,
        profile_id: observation.profile_id.map(bounded_identifier).transpose()?,
        session_id: observation.session_id.map(bounded_identifier).transpose()?,
        route_id: observation.route_id.map(bounded_identifier).transpose()?,
        display_id: observation.display_id.map(bounded_identifier).transpose()?,
        handoff_id_hash: observation.handoff_id_hash.map(bounded_hash).transpose()?,
        ..ServiceFailureReferences::default()
    })
    .with_details(serde_json::json!({
        "observationId": bounded_identifier(observation.observation_id)?,
        "authenticatedActorHash": opaque_identifier_hash(authenticated_actor),
        "streamProvider": observation.stream_provider.map(bounded_identifier).transpose()?,
        "elapsedMs": observation.elapsed_ms,
    }));
    let record = if let Some(action) = observation.action {
        record.with_action(action)
    } else {
        record
    };
    Ok(record)
}

pub fn append_service_failure_at(path: &Path, record: &ServiceFailureRecord) -> Result<(), String> {
    static PROCESS_APPEND_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = PROCESS_APPEND_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "service failure journal lock was poisoned".to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "service failure journal path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create service failure journal directory {}: {error}",
            parent.display()
        )
    })?;
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|error| {
        format!(
            "failed to open service failure journal {}: {error}",
            path.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| {
                format!(
                    "failed to restrict service failure journal {}: {error}",
                    path.display()
                )
            })?;
    }
    file.lock().map_err(|error| {
        format!(
            "failed to lock service failure journal {}: {error}",
            path.display()
        )
    })?;
    let mut encoded = serde_json::to_vec(record)
        .map_err(|error| format!("failed to encode service failure record: {error}"))?;
    encoded.push(b'\n');
    file.write_all(&encoded)
        .and_then(|_| file.sync_data())
        .map_err(|error| {
            format!(
                "failed to append service failure journal {}: {error}",
                path.display()
            )
        })?;
    Ok(())
}

pub fn default_failure_journal_path() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| {
            home.join(".agent-browser")
                .join("service")
                .join(FAILURE_JOURNAL_FILENAME)
        })
        .ok_or_else(|| "could not determine home directory for service failure journal".to_string())
}

pub fn opaque_identifier_hash(value: &str) -> String {
    let digest = Sha256::digest(value.trim().as_bytes());
    format!("sha256:{digest:x}")
}

fn bounded_token(value: String, fallback: &str) -> String {
    let trimmed = value.trim();
    let value = if trimmed.is_empty() {
        fallback
    } else {
        trimmed
    };
    value.chars().take(128).collect()
}

fn bounded_identifier(value: String) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 256 || trimmed.contains("://") || trimmed.contains('?')
    {
        return Err("failure observation identifier is empty, too long, or URL-shaped".to_string());
    }
    Ok(trimmed.to_string())
}

fn bounded_hash(value: String) -> Result<String, String> {
    let value = bounded_identifier(value)?;
    if !value.starts_with("sha256:") || value.len() != 71 {
        return Err("handoffIdHash must be a sha256 identifier".to_string());
    }
    Ok(value)
}

fn safe_reference(value: Option<String>) -> Option<String> {
    value.and_then(|value| bounded_identifier(value).ok())
}

fn redact_summary(value: &str) -> String {
    value
        .split_whitespace()
        .map(|token| {
            let lower = token.to_ascii_lowercase();
            if token.contains("://")
                || lower.starts_with("bearer")
                || lower.contains("token=")
                || lower.contains("password=")
                || lower.contains("secret=")
            {
                "[redacted]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_TEXT_BYTES)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_record(code: &str) -> ServiceFailureRecord {
        ServiceFailureRecord::new(
            ServiceFailureCategory::DashboardAction,
            "dashboard_http_gateway",
            "response",
            code,
            "failed https://private.test/path?token=secret password=also-secret",
        )
    }

    #[test]
    fn injected_dispatcher_durably_appends_redacted_record_to_temp_journal() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-failure-dispatcher-{}",
            uuid::Uuid::new_v4()
        ));
        let path = root.join("journal.jsonl");
        let sink_path = path.clone();
        let (written_tx, written_rx) = std::sync::mpsc::channel();
        let counters = Arc::new(FailureJournalDeliveryCounters::default());
        let dispatcher = FailureJournalDispatcher::start(
            4,
            Arc::new(move |record| {
                append_service_failure_at(&sink_path, record)?;
                let _ = written_tx.send(());
                Ok(())
            }),
            counters.clone(),
        )
        .unwrap();

        assert_eq!(
            dispatcher.enqueue(&test_record("gateway_timeout")),
            FailureJournalEnqueueResult::Accepted
        );
        written_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        let readback = read_service_failures_at(&path, 10).unwrap();
        assert_eq!(readback.records.len(), 1);
        assert_eq!(readback.records[0].code, "gateway_timeout");
        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("private.test"));
        assert!(!raw.contains("secret"));
        assert_eq!(counters.write_failures.load(Ordering::Relaxed), 0);
        drop(dispatcher);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn injected_dispatcher_backpressures_without_losing_records() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-failure-backpressure-{}",
            uuid::Uuid::new_v4()
        ));
        let path = root.join("journal.jsonl");
        let sink_path = path.clone();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (written_tx, written_rx) = std::sync::mpsc::channel();
        let release_rx = Arc::new(Mutex::new(release_rx));
        let first = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let counters = Arc::new(FailureJournalDeliveryCounters::default());
        let dispatcher = FailureJournalDispatcher::start(
            1,
            Arc::new(move |record| {
                if first.swap(false, Ordering::SeqCst) {
                    let _ = entered_tx.send(());
                    let _ = release_rx.lock().unwrap().recv();
                }
                append_service_failure_at(&sink_path, record)?;
                let _ = written_tx.send(());
                Ok(())
            }),
            counters.clone(),
        )
        .unwrap();

        assert_eq!(
            dispatcher.enqueue(&test_record("first")),
            FailureJournalEnqueueResult::Accepted
        );
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert_eq!(
            dispatcher.enqueue(&test_record("second")),
            FailureJournalEnqueueResult::Accepted
        );
        let third_dispatcher = dispatcher.clone();
        let (third_tx, third_rx) = std::sync::mpsc::channel();
        let third = thread::spawn(move || {
            let result = third_dispatcher.enqueue(&test_record("third"));
            let _ = third_tx.send(result);
        });
        for _ in 0..10_000 {
            if counters.queue_overloads.load(Ordering::Relaxed) == 1 {
                break;
            }
            thread::yield_now();
        }
        assert_eq!(counters.queue_overloads.load(Ordering::Relaxed), 1);
        assert!(matches!(
            third_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        assert_eq!(counters.delivery_failures.load(Ordering::Relaxed), 0);
        let _ = release_tx.send(());
        assert_eq!(
            third_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap(),
            FailureJournalEnqueueResult::Backpressured
        );
        third.join().unwrap();
        for _ in 0..3 {
            written_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap();
        }
        let readback = read_service_failures_at(&path, 10).unwrap();
        assert_eq!(
            readback
                .records
                .iter()
                .map(|record| record.code.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second", "third"]
        );
        assert_eq!(counters.write_failures.load(Ordering::Relaxed), 0);
        drop(dispatcher);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn injected_dispatcher_accounts_for_sink_write_failure() {
        let (attempted_tx, attempted_rx) = std::sync::mpsc::channel();
        let counters = Arc::new(FailureJournalDeliveryCounters::default());
        let dispatcher = FailureJournalDispatcher::start(
            1,
            Arc::new(move |_| {
                let _ = attempted_tx.send(());
                Err("injected sink failure with private path".to_string())
            }),
            counters.clone(),
        )
        .unwrap();

        assert_eq!(
            dispatcher.enqueue(&test_record("write-failure")),
            FailureJournalEnqueueResult::Accepted
        );
        attempted_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        for _ in 0..10_000 {
            if counters.delivery_failures.load(Ordering::Relaxed) == 1 {
                break;
            }
            thread::yield_now();
        }
        assert_eq!(counters.write_failures.load(Ordering::Relaxed), 1);
        assert_eq!(counters.delivery_failures.load(Ordering::Relaxed), 1);
        assert_eq!(counters.queue_overloads.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn appends_distinct_jsonl_occurrences_without_raw_urls() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-failure-journal-{}",
            uuid::Uuid::new_v4()
        ));
        let path = root.join("journal.jsonl");
        let record = ServiceFailureRecord::new(
            ServiceFailureCategory::HandoffLink,
            "dashboard",
            "resolve",
            "handoff_unusable",
            "failed https://example.test/remote-view/private?token=secret token=also-secret",
        )
        .with_references(ServiceFailureReferences {
            handoff_id_hash: Some(opaque_identifier_hash("private")),
            ..ServiceFailureReferences::default()
        });
        append_service_failure_at(&path, &record).unwrap();
        append_service_failure_at(&path, &record).unwrap();

        let journal = fs::read_to_string(&path).unwrap();
        assert_eq!(journal.lines().count(), 2);
        assert!(!journal.contains("example.test"));
        assert!(!journal.contains("also-secret"));
        assert!(journal.contains("handoff_unusable"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_oversized_details_without_losing_record() {
        let record = ServiceFailureRecord::new(
            ServiceFailureCategory::CdpStream,
            "stream",
            "watchdog",
            "frame_stalled",
            "no frame received",
        )
        .with_details(Value::String("x".repeat(MAX_DETAILS_BYTES + 1)));
        assert!(record.details.is_none());
    }

    #[test]
    fn client_observation_rejects_raw_handoff_urls_and_unobservable_categories() {
        let raw_url = serde_json::json!({
            "category": "handoff_link",
            "stage": "load",
            "code": "handoff_unusable",
            "summary": "viewer did not become ready",
            "observationId": "obs-1",
            "handoffIdHash": "https://example.test/private"
        });
        assert!(record_client_failure_observation(&raw_url.to_string(), "operator").is_err());

        let launch = serde_json::json!({
            "category": "browser_launch",
            "stage": "load",
            "code": "failed",
            "summary": "not observable here",
            "observationId": "obs-2"
        });
        assert!(record_client_failure_observation(&launch.to_string(), "operator").is_err());
    }

    #[test]
    fn client_observation_builds_a_bounded_actor_attributed_record() {
        let body = serde_json::json!({
            "category": "cdp_stream",
            "stage": "frame_watchdog",
            "code": "cdp_frame_never_received",
            "summary": "connected without frames",
            "action": "stream_frame",
            "observationId": "obs-3",
            "browserId": "browser-7",
            "streamProvider": "cdp_screencast",
            "elapsedMs": 15001
        });
        let record =
            build_client_failure_observation(&body.to_string(), "operator@example.test").unwrap();
        assert_eq!(record.category, ServiceFailureCategory::CdpStream);
        assert_eq!(record.references.browser_id.as_deref(), Some("browser-7"));
        assert_eq!(record.details.as_ref().unwrap()["observationId"], "obs-3");
        assert_ne!(
            record.details.as_ref().unwrap()["authenticatedActorHash"],
            "operator@example.test"
        );
    }

    #[test]
    fn readback_survives_malformed_lines_and_returns_latest_records() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-failure-journal-read-{}",
            uuid::Uuid::new_v4()
        ));
        let path = root.join("journal.jsonl");
        for code in ["one", "two", "three"] {
            append_service_failure_at(
                &path,
                &ServiceFailureRecord::new(
                    ServiceFailureCategory::DashboardAction,
                    "test",
                    "request",
                    code,
                    "failed",
                ),
            )
            .unwrap();
        }
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"not-json\n").unwrap();

        let readback = read_service_failures_at(&path, 2).unwrap();
        assert_eq!(readback.records.len(), 2);
        assert_eq!(readback.records[0].code, "two");
        assert_eq!(readback.records[1].code, "three");
        assert_eq!(readback.malformed_line_count, 1);
        let _ = fs::remove_dir_all(root);
    }
}
