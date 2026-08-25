//! Guarded, provider-neutral desktop interaction engine.
//!
//! This module owns the deterministic recipe, motion planner, event cleanup,
//! verification, and redacted receipt. Platform input and controller storage
//! remain injected seams. The source proof supplies only an in-memory fixture
//! adapter and never invokes an operating-system input facility.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::desktop_control_coordinator::{DesktopControlCoordinator, DesktopInteractionClaim};
use super::service_model::ServiceState;

pub(crate) const RECIPE_ID: &str = "p110-pointer-keyboard-v1";
pub(crate) const FOUNDATION_STRESS_RECIPE_ID: &str = "p110-foundation-stress-v1";
pub(crate) const CONTROLLED_X11_RECIPE_ID: &str = "p131-controlled-x11-v1";
const RECIPE_VERSION: &str = "v1";
const FIXED_TEXT: &str = "fixture-ready";
const COORDINATE_SPACE: &str = "desktop_physical_pixels";
const FRESHNESS_LIMIT_MS: u64 = 750;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopInteractionRequest {
    pub browser_id: String,
    pub session_name: Option<String>,
    pub controller_lease_id: String,
    pub recipe_id: String,
    pub operation_id: String,
    pub operation_principal_id: String,
    pub request_principal_source: Option<String>,
    pub service_name: String,
    pub task_name: String,
    pub caller_id: String,
    pub request_id: String,
    pub agent_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopBinding {
    pub browser_id: String,
    pub session_name: String,
    pub profile_id: Option<String>,
    pub display_allocation_id: String,
    pub stream_id: String,
    pub route_id: String,
    pub width: u32,
    pub height: u32,
    pub scale_millis: u32,
    pub coordinate_space: String,
    pub geometry_epoch: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PixelPoint {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PixelBounds {
    pub x: i64,
    pub y: i64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BeforeObservation {
    pub binding: DesktopBinding,
    pub context_id: String,
    pub frame_id: String,
    pub frame_sha256: String,
    pub captured_at_ms: u64,
    pub observation_id: String,
    pub observation_sha256: String,
    pub observation_status: String,
    pub selected_candidate_id: Option<String>,
    pub selected_target_class: Option<String>,
    pub selected_bounds: Option<PixelBounds>,
    pub selected_center: Option<PixelPoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SurfaceSnapshot {
    pub provider_id: String,
    pub provider_version: String,
    pub provider_capability: String,
    pub surface_identity_digest: String,
    pub browser_process_identity_digest: String,
    pub focused: bool,
    pub client_bounds: PixelBounds,
    pub pointer: PixelPoint,
    pub width: u32,
    pub height: u32,
    pub scale_millis: u32,
    pub coordinate_space: String,
    pub geometry_epoch: String,
}

impl PixelBounds {
    fn contains(self, point: PixelPoint) -> bool {
        let width = i64::from(self.width);
        let height = i64::from(self.height);
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x.saturating_add(width)
            && point.y < self.y.saturating_add(height)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControllerAuthority {
    pub browser_id: String,
    pub display_allocation_id: String,
    pub stream_id: String,
    pub route_id: String,
    pub route_controller_lease_id: String,
    pub stream_controller_lease_id: String,
    pub lease_id: String,
    pub lease_record_id: String,
    pub lease_route_id: String,
    pub lease_browser_id: String,
    pub lease_viewer_id: String,
    pub lease_role: String,
    pub lease_state: String,
    pub lease_updated_at: String,
    pub lease_expires_at_ms: u64,
    pub controller_epoch: u64,
    pub route_controller_epoch: u64,
    pub stream_controller_epoch: u64,
    pub route_contains_lease: bool,
    pub stream_contains_lease: bool,
    pub route_writable: bool,
    pub stream_writable: bool,
    pub route_machine_input: Option<String>,
    pub stream_machine_input: Option<String>,
}

pub(crate) trait ControllerAuthorityRepository {
    fn snapshot(&mut self) -> Result<ControllerAuthority, DesktopInteractionError>;
}

pub(crate) trait InteractionClock {
    fn now_ms(&mut self) -> u64;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum InputEvent {
    PointerMove {
        point: PixelPoint,
        at_ms: u64,
    },
    LeftDown {
        at_ms: u64,
    },
    LeftUp {
        at_ms: u64,
        emergency: bool,
    },
    KeyDown {
        key: char,
        at_ms: u64,
    },
    KeyUp {
        key: char,
        at_ms: u64,
        emergency: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EventAcknowledgement {
    pub acknowledgement_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AfterObservation {
    pub binding: DesktopBinding,
    pub context_id: String,
    pub frame_id: String,
    pub frame_sha256: String,
    pub observation_id: String,
    pub observation_sha256: String,
    pub verification_state: String,
    pub text_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptDisposition {
    pub state: String,
    pub reason_code: String,
    pub observation_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HumanHandoffSummary {
    pub state: String,
    pub reason: String,
    pub handoff_id: String,
    pub handoff_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundationStressContext {
    pub prompt_disposition: PromptDisposition,
    pub handoff_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopInteractionProviderEvidence {
    pub provider_id: String,
    pub provider_version: String,
    pub capability: String,
}

impl FoundationStressContext {
    fn actionable() -> Self {
        Self {
            prompt_disposition: PromptDisposition {
                state: "actionable_observation".to_string(),
                reason_code: "synthetic_prompt_actionable".to_string(),
                observation_sha256: digest_text("synthetic-prompt-observation"),
            },
            handoff_reason: Some("effect_uncertain".to_string()),
        }
    }
}

fn validate_foundation_stress_context(
    context: &FoundationStressContext,
) -> Result<(), DesktopInteractionError> {
    if !matches!(
        context.prompt_disposition.state.as_str(),
        "actionable_observation" | "operator_intervention_required"
    ) || context.prompt_disposition.reason_code.trim().is_empty()
        || context.prompt_disposition.observation_sha256.len() != 64
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_prompt_evidence_invalid",
            "prompt disposition evidence is invalid",
        ));
    }
    if context
        .handoff_reason
        .as_deref()
        .is_some_and(|reason| reason.trim().is_empty() || reason.len() > 128)
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_handoff_invalid",
            "provider handoff need has no bounded reason",
        ));
    }
    Ok(())
}

pub(crate) trait ServiceOwnedHandoffRepository {
    fn resolve_ready(
        &mut self,
        browser_id: &str,
        session_name: &str,
        route_id: &str,
        display_allocation_id: &str,
        reason: &str,
    ) -> Result<Option<HumanHandoffSummary>, DesktopInteractionError>;
}

pub(crate) struct ServiceStateHandoffRepository<'a> {
    state: &'a ServiceState,
}

impl<'a> ServiceStateHandoffRepository<'a> {
    pub(crate) fn new(state: &'a ServiceState) -> Self {
        Self { state }
    }
}

impl ServiceOwnedHandoffRepository for ServiceStateHandoffRepository<'_> {
    fn resolve_ready(
        &mut self,
        browser_id: &str,
        session_name: &str,
        route_id: &str,
        display_allocation_id: &str,
        reason: &str,
    ) -> Result<Option<HumanHandoffSummary>, DesktopInteractionError> {
        let Some(handoff) = self.state.remote_view_handoffs.values().find(|handoff| {
            handoff.state == "ready"
                && handoff.browser_id.as_deref() == Some(browser_id)
                && handoff.session_name.as_deref() == Some(session_name)
                && handoff.last_route_id.as_deref() == Some(route_id)
                && handoff.last_display_allocation_id.as_deref() == Some(display_allocation_id)
                && handoff
                    .last_resolution
                    .as_ref()
                    .and_then(|value| value.get("operatorVisible"))
                    .and_then(|value| value.get("state"))
                    .and_then(Value::as_str)
                    == Some("ready")
        }) else {
            return Ok(None);
        };
        let handoff_url = handoff.handoff_url.clone().ok_or_else(|| {
            DesktopInteractionError::new(
                "desktop_interaction_handoff_invalid",
                "service-owned ready handoff has no authenticated URL",
            )
        })?;
        validate_service_handoff_url(&handoff.id, &handoff_url)?;
        Ok(Some(HumanHandoffSummary {
            state: "ready".to_string(),
            reason: reason.to_string(),
            handoff_id: handoff.id.clone(),
            handoff_url,
        }))
    }
}

fn validate_service_handoff_url(
    handoff_id: &str,
    handoff_url: &str,
) -> Result<(), DesktopInteractionError> {
    let expected_path = format!("/remote-view/{handoff_id}");
    let path_matches = handoff_url == expected_path
        || handoff_url
            .strip_prefix("https://")
            .is_some_and(|authority| {
                authority
                    .find('/')
                    .is_some_and(|index| authority[index..] == expected_path)
            });
    if handoff_id.is_empty()
        || !handoff_id
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_'))
        || !path_matches
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_handoff_invalid",
            "service-owned handoff URL is not the exact authenticated opaque route",
        ));
    }
    Ok(())
}

pub(crate) trait DesktopInteractionProvider {
    fn evidence(&self) -> DesktopInteractionProviderEvidence;
    fn observe_before(
        &mut self,
        request: &DesktopInteractionRequest,
    ) -> Result<BeforeObservation, DesktopInteractionError>;
    fn probe(
        &mut self,
        binding: &DesktopBinding,
    ) -> Result<SurfaceSnapshot, DesktopInteractionError>;
    fn execute_event(
        &mut self,
        binding: &DesktopBinding,
        expected_surface: &SurfaceSnapshot,
        effect_key: &str,
        event: &InputEvent,
    ) -> Result<EventAcknowledgement, DesktopInteractionError>;
    fn observe_after(
        &mut self,
        binding: &DesktopBinding,
    ) -> Result<AfterObservation, DesktopInteractionError>;

    fn foundation_stress_context(
        &mut self,
        _binding: &DesktopBinding,
    ) -> Result<FoundationStressContext, DesktopInteractionError> {
        Ok(FoundationStressContext::actionable())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum InteractionOperationRecord {
    InProgress {
        request_sha256: String,
    },
    Complete {
        request_sha256: String,
        receipt: Box<InteractionReceipt>,
    },
    Uncertain {
        request_sha256: String,
        receipt: Box<InteractionReceipt>,
    },
}

pub(crate) trait InteractionOperationLedger {
    fn lookup(
        &mut self,
        caller_id: &str,
        operation_id: &str,
    ) -> Result<Option<InteractionOperationRecord>, DesktopInteractionError>;
    fn begin(
        &mut self,
        caller_id: &str,
        operation_id: &str,
        request_sha256: &str,
    ) -> Result<(), DesktopInteractionError>;
    fn complete(
        &mut self,
        caller_id: &str,
        operation_id: &str,
        request_sha256: &str,
        receipt: &InteractionReceipt,
    ) -> Result<(), DesktopInteractionError>;
    fn abort(&mut self, caller_id: &str, operation_id: &str)
        -> Result<(), DesktopInteractionError>;
}

#[derive(Debug, Default)]
pub(crate) struct SerializedInteractionOperationLedger {
    records: BTreeMap<String, InteractionOperationRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InteractionOperationLedgerSnapshot {
    schema_version: String,
    records: BTreeMap<String, InteractionOperationRecord>,
}

impl SerializedInteractionOperationLedger {
    pub(crate) fn from_json(serialized: &str) -> Result<Self, DesktopInteractionError> {
        let snapshot: InteractionOperationLedgerSnapshot = serde_json::from_str(serialized)
            .map_err(|_| {
                DesktopInteractionError::new(
                    "desktop_interaction_operation_ledger_invalid",
                    "the durable interaction operation ledger is malformed",
                )
            })?;
        if snapshot.schema_version != "p110-interaction-operation-ledger.v1" {
            return Err(DesktopInteractionError::new(
                "desktop_interaction_operation_ledger_invalid",
                "the durable interaction operation ledger version is unsupported",
            ));
        }
        Ok(Self {
            records: snapshot.records,
        })
    }

    pub(crate) fn to_json(&self) -> Result<String, DesktopInteractionError> {
        serde_json::to_string(&InteractionOperationLedgerSnapshot {
            schema_version: "p110-interaction-operation-ledger.v1".to_string(),
            records: self.records.clone(),
        })
        .map_err(|_| {
            DesktopInteractionError::new(
                "desktop_interaction_operation_ledger_invalid",
                "the durable interaction operation ledger could not be serialized",
            )
        })
    }
}

impl InteractionOperationLedger for SerializedInteractionOperationLedger {
    fn lookup(
        &mut self,
        caller_id: &str,
        operation_id: &str,
    ) -> Result<Option<InteractionOperationRecord>, DesktopInteractionError> {
        Ok(self
            .records
            .get(&operation_scope_sha256(caller_id, operation_id))
            .cloned())
    }

    fn begin(
        &mut self,
        caller_id: &str,
        operation_id: &str,
        request_sha256: &str,
    ) -> Result<(), DesktopInteractionError> {
        self.records.insert(
            operation_scope_sha256(caller_id, operation_id),
            InteractionOperationRecord::InProgress {
                request_sha256: request_sha256.to_string(),
            },
        );
        Ok(())
    }

    fn complete(
        &mut self,
        caller_id: &str,
        operation_id: &str,
        request_sha256: &str,
        receipt: &InteractionReceipt,
    ) -> Result<(), DesktopInteractionError> {
        let mut durable_receipt = receipt.clone();
        durable_receipt.operation_id = digest_text(operation_id);
        if durable_receipt.recipe_id == FOUNDATION_STRESS_RECIPE_ID {
            durable_receipt.route_id = digest_text(&durable_receipt.route_id);
            durable_receipt.display_allocation_id =
                digest_text(&durable_receipt.display_allocation_id);
            durable_receipt.stream_id = digest_text(&durable_receipt.stream_id);
            if let Some(handoff) = durable_receipt.human_handoff.as_mut() {
                handoff.handoff_url.clear();
            }
        }
        let record = if receipt.effect_state == "effect_uncertain"
            || receipt.effect_state == "cancelled_after_effect"
        {
            InteractionOperationRecord::Uncertain {
                request_sha256: request_sha256.to_string(),
                receipt: Box::new(durable_receipt),
            }
        } else {
            InteractionOperationRecord::Complete {
                request_sha256: request_sha256.to_string(),
                receipt: Box::new(durable_receipt),
            }
        };
        self.records
            .insert(operation_scope_sha256(caller_id, operation_id), record);
        Ok(())
    }

    fn abort(
        &mut self,
        caller_id: &str,
        operation_id: &str,
    ) -> Result<(), DesktopInteractionError> {
        self.records
            .remove(&operation_scope_sha256(caller_id, operation_id));
        Ok(())
    }
}

/// Dedicated service-owned file adapter. Each transition is persisted by a
/// same-directory temporary file, file sync, atomic rename, and directory sync.
#[derive(Debug)]
pub(crate) struct PersistedInteractionOperationLedger {
    path: PathBuf,
    inner: SerializedInteractionOperationLedger,
}

impl PersistedInteractionOperationLedger {
    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, DesktopInteractionError> {
        let path = path.as_ref().to_path_buf();
        let inner = match fs::read_to_string(&path) {
            Ok(value) => SerializedInteractionOperationLedger::from_json(&value)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                SerializedInteractionOperationLedger::default()
            }
            Err(_) => {
                return Err(ledger_error(
                    "desktop_interaction_operation_ledger_load_failed",
                ))
            }
        };
        Ok(Self { path, inner })
    }

    fn save(&self) -> Result<(), DesktopInteractionError> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| ledger_error("desktop_interaction_operation_ledger_save_failed"))?;
        fs::create_dir_all(parent)
            .map_err(|_| ledger_error("desktop_interaction_operation_ledger_save_failed"))?;
        let file_name = self
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| ledger_error("desktop_interaction_operation_ledger_save_failed"))?;
        let temporary = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|_| ledger_error("desktop_interaction_operation_ledger_save_failed"))?;
        let serialized = self.inner.to_json()?;
        let result = (|| {
            file.write_all(serialized.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temporary, &self.path)?;
            #[cfg(unix)]
            fs::File::open(parent)?.sync_all()?;
            Ok::<(), std::io::Error>(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(ledger_error(
                "desktop_interaction_operation_ledger_save_failed",
            ));
        }
        Ok(())
    }

    fn transition(
        &mut self,
        mutate: impl FnOnce(
            &mut SerializedInteractionOperationLedger,
        ) -> Result<(), DesktopInteractionError>,
    ) -> Result<(), DesktopInteractionError> {
        let previous = self.inner.records.clone();
        mutate(&mut self.inner)?;
        if let Err(error) = self.save() {
            self.inner.records = previous;
            return Err(error);
        }
        Ok(())
    }
}

impl InteractionOperationLedger for PersistedInteractionOperationLedger {
    fn lookup(
        &mut self,
        caller_id: &str,
        operation_id: &str,
    ) -> Result<Option<InteractionOperationRecord>, DesktopInteractionError> {
        self.inner.lookup(caller_id, operation_id)
    }
    fn begin(
        &mut self,
        caller_id: &str,
        operation_id: &str,
        request_sha256: &str,
    ) -> Result<(), DesktopInteractionError> {
        self.transition(|inner| inner.begin(caller_id, operation_id, request_sha256))
    }
    fn complete(
        &mut self,
        caller_id: &str,
        operation_id: &str,
        request_sha256: &str,
        receipt: &InteractionReceipt,
    ) -> Result<(), DesktopInteractionError> {
        self.transition(|inner| inner.complete(caller_id, operation_id, request_sha256, receipt))
    }
    fn abort(
        &mut self,
        caller_id: &str,
        operation_id: &str,
    ) -> Result<(), DesktopInteractionError> {
        self.transition(|inner| inner.abort(caller_id, operation_id))
    }
}

fn ledger_error(code: &'static str) -> DesktopInteractionError {
    DesktopInteractionError::new(code, "the service-owned operation ledger transition failed")
}

pub(crate) struct InteractionDependencies<'a> {
    pub provider: &'a mut dyn DesktopInteractionProvider,
    pub authority: &'a mut dyn ControllerAuthorityRepository,
    pub coordinator: &'a DesktopControlCoordinator,
    pub idempotency: &'a mut dyn InteractionOperationLedger,
    pub handoffs: &'a mut dyn ServiceOwnedHandoffRepository,
    pub clock: &'a mut dyn InteractionClock,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InteractionReceipt {
    pub transaction_id: String,
    pub schema_version: String,
    pub recipe_id: String,
    pub recipe_version: String,
    pub recipe_sha256: String,
    pub operation_id: String,
    pub operation_request_sha256: String,
    pub replay_state: String,
    pub recipe_provider_id: String,
    pub recipe_provider_version: String,
    pub recipe_provider_capability: String,
    pub prompt_disposition: Option<PromptDisposition>,
    pub human_handoff: Option<HumanHandoffSummary>,
    pub entry_gate: String,
    pub effect_key_digest: String,
    pub effect_key_count: usize,
    pub attempted_effect_key_digest: String,
    pub attempted_effect_key_count: usize,
    pub acknowledged_effect_key_digest: String,
    pub acknowledged_effect_key_count: usize,
    pub attempted_event_order_sha256: String,
    pub browser_id: String,
    pub display_allocation_id: String,
    pub stream_id: String,
    pub route_id: String,
    pub controller_epoch: u64,
    pub authority_digest: String,
    pub actor_digest: String,
    pub before_context_id: String,
    pub before_frame_id: String,
    pub before_frame_sha256: String,
    pub before_observation_id: String,
    pub before_observation_sha256: String,
    pub selected_candidate_id: String,
    pub surface_identity_digest: String,
    pub browser_process_identity_digest: String,
    pub pointer_start: PixelPoint,
    pub target: PixelPoint,
    pub coordinate_mapping: String,
    pub motion_profile: String,
    pub control_point_digest: String,
    pub emitted_path_sha256: String,
    pub pointer_event_count: usize,
    pub duration_ms: u64,
    pub acknowledgement_ids: Vec<String>,
    pub cleanup_state: String,
    pub text_length: usize,
    pub text_sha256: String,
    pub after_context_id: Option<String>,
    pub after_frame_id: Option<String>,
    pub after_frame_sha256: Option<String>,
    pub after_observation_id: Option<String>,
    pub after_observation_sha256: Option<String>,
    pub verification_state: String,
    pub effect_state: String,
    pub stop_reason: Option<String>,
    pub retention: String,
    pub persisted_pixels: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopInteractionError {
    code: &'static str,
    message: &'static str,
    receipt: Option<Box<InteractionReceipt>>,
}

impl DesktopInteractionError {
    pub(crate) fn new(code: &'static str, message: &'static str) -> Self {
        Self {
            code,
            message,
            receipt: None,
        }
    }

    pub(crate) fn code(&self) -> &'static str {
        self.code
    }

    pub(crate) fn receipt(&self) -> Option<&InteractionReceipt> {
        self.receipt.as_deref()
    }
}

impl std::fmt::Display for DesktopInteractionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DesktopInteractionError {}

pub(crate) fn run_desktop_interaction(
    request: DesktopInteractionRequest,
    mut dependencies: InteractionDependencies<'_>,
) -> Result<InteractionReceipt, DesktopInteractionError> {
    validate_request(&request)?;
    let operation_request_sha256 = operation_request_sha256(&request);
    match dependencies
        .idempotency
        .lookup(&request.operation_principal_id, &request.operation_id)?
    {
        Some(InteractionOperationRecord::Complete {
            request_sha256,
            receipt,
        })
        | Some(InteractionOperationRecord::Uncertain {
            request_sha256,
            receipt,
        }) if request_sha256 == operation_request_sha256 => {
            let mut receipt = *receipt;
            receipt.operation_id = request.operation_id.clone();
            receipt.replay_state = "replayed_terminal".to_string();
            return Ok(receipt);
        }
        Some(InteractionOperationRecord::Complete { .. })
        | Some(InteractionOperationRecord::Uncertain { .. }) => {
            return Err(DesktopInteractionError::new(
                "desktop_interaction_operation_conflict",
                "the operation ID is already bound to another canonical request",
            ));
        }
        Some(InteractionOperationRecord::InProgress { request_sha256 })
            if request_sha256 != operation_request_sha256 =>
        {
            return Err(DesktopInteractionError::new(
                "desktop_interaction_operation_conflict",
                "the operation ID is already bound to another canonical request",
            ));
        }
        Some(InteractionOperationRecord::InProgress { .. }) => {
            return Err(DesktopInteractionError::new(
                "desktop_interaction_duplicate",
                "the interaction operation is already in progress and requires reconciliation",
            ));
        }
        None => {}
    }

    let transaction_id = format!(
        "desktop-interaction-{}",
        &operation_scope_sha256(&request.operation_principal_id, &request.operation_id)[..24]
    );
    dependencies.idempotency.begin(
        &request.operation_principal_id,
        &request.operation_id,
        &operation_request_sha256,
    )?;
    let before = match dependencies.provider.observe_before(&request) {
        Ok(before) => before,
        Err(error) => {
            dependencies
                .idempotency
                .abort(&request.operation_principal_id, &request.operation_id)?;
            return Err(error);
        }
    };
    if let Err(error) = validate_before(&request, &before) {
        dependencies
            .idempotency
            .abort(&request.operation_principal_id, &request.operation_id)?;
        return Err(error);
    }
    let candidate_id = before
        .selected_candidate_id
        .clone()
        .expect("validated selected candidate");
    let target = before.selected_center.expect("validated target center");
    let target_bounds = before.selected_bounds.expect("validated target bounds");
    let initial_authority = match dependencies.authority.snapshot() {
        Ok(authority) => authority,
        Err(error) => {
            dependencies
                .idempotency
                .abort(&request.operation_principal_id, &request.operation_id)?;
            return Err(error);
        }
    };
    let initial_now = dependencies.clock.now_ms();
    let authority_digest =
        match validate_authority(&request, &before.binding, &initial_authority, initial_now) {
            Ok(digest) => digest,
            Err(error) => {
                dependencies
                    .idempotency
                    .abort(&request.operation_principal_id, &request.operation_id)?;
                return Err(error);
            }
        };
    let claim = match dependencies
        .coordinator
        .claim(&before.binding.route_id, &transaction_id)
    {
        Ok(claim) => claim,
        Err(_) => {
            dependencies
                .idempotency
                .abort(&request.operation_principal_id, &request.operation_id)?;
            return Err(DesktopInteractionError::new(
                "desktop_interaction_conflict",
                "the route already has an interaction claim",
            ));
        }
    };
    let stress_context = if request.recipe_id == FOUNDATION_STRESS_RECIPE_ID {
        match dependencies
            .provider
            .foundation_stress_context(&before.binding)
        {
            Ok(context) => {
                if let Err(error) = validate_foundation_stress_context(&context) {
                    dependencies
                        .idempotency
                        .abort(&request.operation_principal_id, &request.operation_id)?;
                    return Err(error);
                }
                Some(context)
            }
            Err(error) => {
                dependencies
                    .idempotency
                    .abort(&request.operation_principal_id, &request.operation_id)?;
                return Err(error);
            }
        }
    } else {
        None
    };

    let result = run_claimed_interaction(
        &request,
        &transaction_id,
        before,
        candidate_id,
        target,
        target_bounds,
        initial_authority,
        authority_digest,
        stress_context.as_ref(),
        &claim,
        &mut dependencies,
    );
    drop(claim);
    match result.outcome {
        Ok(mut receipt) => {
            finalize_stress_receipt(
                &request,
                stress_context.as_ref(),
                &mut receipt,
                dependencies.handoffs,
            )?;
            dependencies.idempotency.complete(
                &request.operation_principal_id,
                &request.operation_id,
                &operation_request_sha256,
                &receipt,
            )?;
            Ok(receipt)
        }
        Err(mut error) => {
            if let Some(receipt) = error.receipt.as_deref_mut() {
                finalize_stress_receipt(
                    &request,
                    stress_context.as_ref(),
                    receipt,
                    dependencies.handoffs,
                )?;
                dependencies.idempotency.complete(
                    &request.operation_principal_id,
                    &request.operation_id,
                    &operation_request_sha256,
                    receipt,
                )?;
            } else {
                dependencies
                    .idempotency
                    .abort(&request.operation_principal_id, &request.operation_id)?;
            }
            Err(error)
        }
    }
}

/// Dispatch the controlled provider only from an exact admitted development
/// generation. Production and unmanifested binaries fail before capture,
/// authority lookup, controller mutation, or input. Raw provider routing is
/// never accepted as a compatibility contract.
pub(crate) async fn handle_desktop_interact(command: &Value) -> Result<Value, String> {
    for forbidden in [
        "coordinates",
        "displayName",
        "xauthorityPath",
        "routeUser",
        "providerExecutable",
        "lockPath",
        "providerUrl",
        "guacamoleUrl",
    ] {
        if command.get(forbidden).is_some() {
            return Err(format!(
                "desktop_interact does not accept caller-controlled {forbidden}"
            ));
        }
    }
    let admission =
        super::desktop_input_provider_admission::current_development_provider_admission()
            .map_err(|code| format!("{code}: controlled desktop input admission failed"))?;
    let request = parse_configured_interaction_request(command)?;
    tokio::task::spawn_blocking(move || run_configured_interaction(request, admission))
        .await
        .map_err(|_| "desktop_input_provider_failed: configured provider task failed".to_string())?
}

fn parse_configured_interaction_request(
    command: &Value,
) -> Result<DesktopInteractionRequest, String> {
    fn required(command: &Value, field: &str) -> Result<String, String> {
        command
            .get(field)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .ok_or_else(|| format!("desktop_interact requires {field}"))
    }
    let recipe_id = command
        .get("recipe")
        .and_then(Value::as_object)
        .and_then(|recipe| recipe.get("recipeId"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| "desktop_interact requires recipe.recipeId".to_string())?;
    if recipe_id != CONTROLLED_X11_RECIPE_ID {
        return Err("desktop_interaction_unsupported: recipe is not registered".to_string());
    }
    Ok(DesktopInteractionRequest {
        browser_id: required(command, "browserId")?,
        session_name: command
            .get("sessionName")
            .and_then(Value::as_str)
            .map(str::to_string),
        controller_lease_id: required(command, "controllerLeaseId")?,
        recipe_id,
        operation_id: required(command, "operationId")?,
        operation_principal_id: required(command, "operationPrincipalId")?,
        request_principal_source: command
            .get("requestPrincipalSource")
            .and_then(Value::as_str)
            .map(str::to_string),
        service_name: required(command, "serviceName")?,
        task_name: required(command, "taskName")?,
        caller_id: required(command, "callerId")?,
        request_id: required(command, "requestId")?,
        agent_name: required(command, "agentName")?,
    })
}

fn run_configured_interaction(
    request: DesktopInteractionRequest,
    admission: super::desktop_input_provider_admission::DevelopmentProviderAdmission,
) -> Result<Value, String> {
    use super::controlled_x11_provider::{ControlledX11Provider, SystemInteractionClock};
    use super::desktop_control_coordinator::global_desktop_control_coordinator;
    use super::service_store::{
        default_service_state_path, LockedServiceStateRepository, ServiceStateRepository,
    };

    let state = LockedServiceStateRepository::default_json()?.load_snapshot()?;
    let mut handoffs = ServiceStateHandoffRepository::new(&state);
    let (mut provider, mut authority) = ControlledX11Provider::open(request.clone(), admission)
        .map_err(|error| error.to_string())?;
    let state_path = default_service_state_path()?;
    let ledger_path = state_path
        .parent()
        .ok_or_else(|| "desktop_interaction_operation_ledger_unavailable".to_string())?
        .join("desktop-input")
        .join("operations.json");
    let ledger_directory = ledger_path
        .parent()
        .ok_or_else(|| "desktop_interaction_operation_ledger_unavailable".to_string())?;
    fs::create_dir_all(ledger_directory)
        .map_err(|_| "desktop_interaction_operation_ledger_unavailable".to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(ledger_directory, fs::Permissions::from_mode(0o700))
            .map_err(|_| "desktop_interaction_operation_ledger_unavailable".to_string())?;
    }
    let mut idempotency = PersistedInteractionOperationLedger::open(ledger_path)
        .map_err(|error| error.to_string())?;
    let mut clock = SystemInteractionClock;
    match run_desktop_interaction(
        request,
        InteractionDependencies {
            provider: &mut provider,
            authority: &mut authority,
            coordinator: global_desktop_control_coordinator(),
            idempotency: &mut idempotency,
            handoffs: &mut handoffs,
            clock: &mut clock,
        },
    ) {
        Ok(receipt) => serde_json::to_value(receipt)
            .map_err(|_| "desktop_input_provider_receipt_invalid".to_string()),
        Err(error) => {
            if let Some(receipt) = error.receipt() {
                Ok(serde_json::json!({
                    "status": "failed",
                    "error": error.code(),
                    "receipt": receipt,
                }))
            } else {
                Err(error.to_string())
            }
        }
    }
}

/// Remove response-only or provider-private desktop input material before a
/// result enters long-lived stream, job, incident, or idempotency projection.
pub(crate) fn redact_desktop_interaction_stream_result(result: &Value) -> Value {
    const TOP_LEVEL: &[&str] = &[
        "ok",
        "action",
        "interactionReceipt",
        "errorCode",
        "effectState",
        "stopReason",
    ];
    const RECEIPT: &[&str] = &[
        "transactionId",
        "schemaVersion",
        "recipeId",
        "recipeVersion",
        "recipeSha256",
        "operationRequestSha256",
        "replayState",
        "recipeProviderId",
        "recipeProviderVersion",
        "recipeProviderCapability",
        "promptDisposition",
        "humanHandoff",
        "entryGate",
        "effectKeyDigest",
        "effectKeyCount",
        "attemptedEffectKeyDigest",
        "attemptedEffectKeyCount",
        "acknowledgedEffectKeyDigest",
        "acknowledgedEffectKeyCount",
        "attemptedEventOrderSha256",
        "browserId",
        "displayAllocationId",
        "streamId",
        "routeId",
        "controllerEpoch",
        "authorityDigest",
        "actorDigest",
        "beforeContextId",
        "beforeFrameId",
        "beforeFrameSha256",
        "beforeObservationId",
        "beforeObservationSha256",
        "selectedCandidateId",
        "surfaceIdentityDigest",
        "browserProcessIdentityDigest",
        "pointerStart",
        "target",
        "coordinateMapping",
        "motionProfile",
        "controlPointDigest",
        "emittedPathSha256",
        "pointerEventCount",
        "durationMs",
        "acknowledgementIds",
        "cleanupState",
        "textLength",
        "textSha256",
        "afterContextId",
        "afterFrameId",
        "afterFrameSha256",
        "afterObservationId",
        "afterObservationSha256",
        "verificationState",
        "effectState",
        "stopReason",
        "retention",
        "persistedPixels",
    ];
    let Some(record) = result.as_object() else {
        return Value::Null;
    };
    let mut redacted = serde_json::Map::new();
    for key in TOP_LEVEL {
        let Some(value) = record.get(*key) else {
            continue;
        };
        if *key == "interactionReceipt" {
            let Some(receipt) = value.as_object() else {
                continue;
            };
            let mut safe: serde_json::Map<String, Value> = receipt
                .iter()
                .filter(|(field, _)| RECEIPT.contains(&field.as_str()))
                .map(|(field, value)| {
                    let value = if matches!(field.as_str(), "pointerStart" | "target") {
                        redact_point(value)
                    } else if field == "acknowledgementIds" {
                        Value::Array(
                            value
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                                .map(|value| Value::String(value.to_string()))
                                .collect(),
                        )
                    } else if matches!(field.as_str(), "promptDisposition" | "humanHandoff") {
                        redact_stress_summary(field, value)
                    } else if value.is_object() || value.is_array() {
                        Value::Null
                    } else {
                        value.clone()
                    };
                    (field.clone(), value)
                })
                .collect();
            if receipt.get("recipeId").and_then(Value::as_str) == Some(FOUNDATION_STRESS_RECIPE_ID)
            {
                if let Some(operation_id) = receipt.get("operationId").and_then(Value::as_str) {
                    safe.insert(
                        "operationIdDigest".to_string(),
                        Value::String(digest_text(operation_id)),
                    );
                }
                for field in ["routeId", "displayAllocationId", "streamId"] {
                    safe.remove(field);
                }
                if let Some(handoff) = safe.get_mut("humanHandoff").and_then(Value::as_object_mut) {
                    handoff.remove("handoffUrl");
                }
            }
            redacted.insert((*key).to_string(), Value::Object(safe));
        } else if !value.is_object() && !value.is_array() {
            redacted.insert((*key).to_string(), value.clone());
        }
    }
    Value::Object(redacted)
}

fn redact_stress_summary(field: &str, value: &Value) -> Value {
    let Some(record) = value.as_object() else {
        return Value::Null;
    };
    let allowed: &[&str] = if field == "promptDisposition" {
        &["state", "reasonCode", "observationSha256"]
    } else {
        &["state", "reason", "handoffId", "handoffUrl"]
    };
    Value::Object(
        record
            .iter()
            .filter(|(key, value)| {
                allowed.contains(&key.as_str()) && !value.is_object() && !value.is_array()
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    )
}

fn redact_point(value: &Value) -> Value {
    let Some(point) = value.as_object() else {
        return Value::Null;
    };
    Value::Object(
        point
            .iter()
            .filter(|(key, value)| matches!(key.as_str(), "x" | "y") && value.is_i64())
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    )
}

struct ClaimedResult {
    outcome: Result<InteractionReceipt, DesktopInteractionError>,
}

#[derive(Default)]
struct EffectTrace {
    attempted_keys: Vec<String>,
    acknowledged_keys: Vec<String>,
    attempted_events: Vec<InputEvent>,
}

#[allow(clippy::too_many_arguments)]
fn run_claimed_interaction(
    request: &DesktopInteractionRequest,
    transaction_id: &str,
    before: BeforeObservation,
    candidate_id: String,
    target: PixelPoint,
    target_bounds: PixelBounds,
    initial_authority: ControllerAuthority,
    authority_digest: String,
    stress_context: Option<&FoundationStressContext>,
    claim: &DesktopInteractionClaim,
    dependencies: &mut InteractionDependencies<'_>,
) -> ClaimedResult {
    let outcome = (|| {
        let provider_evidence = dependencies.provider.evidence();
        if provider_evidence.provider_id.trim().is_empty()
            || provider_evidence.provider_version.trim().is_empty()
            || provider_evidence.capability != "guarded_pointer_keyboard_v1"
        {
            return Err(DesktopInteractionError::new(
                "desktop_input_provider_invalid",
                "desktop input provider evidence is incomplete or unsupported",
            ));
        }
        let initial_surface = dependencies.provider.probe(&before.binding)?;
        if initial_surface.provider_id != provider_evidence.provider_id
            || initial_surface.provider_version != provider_evidence.provider_version
            || initial_surface.provider_capability != provider_evidence.capability
        {
            return Err(DesktopInteractionError::new(
                "desktop_input_provider_invalid",
                "desktop surface evidence does not match its provider identity",
            ));
        }
        validate_surface(&before.binding, &initial_surface, target, target_bounds)?;
        if dependencies
            .clock
            .now_ms()
            .saturating_sub(before.captured_at_ms)
            > FRESHNESS_LIMIT_MS
        {
            return Err(DesktopInteractionError::new(
                "desktop_interaction_stale_observation",
                "the selected observation is too old for input",
            ));
        }
        let recipe_sha256 = recipe_sha256(&request.recipe_id);
        let motion = plan_motion(
            initial_surface.pointer,
            target,
            before.binding.width,
            before.binding.height,
            &digest_text(&format!(
                "{}\0{}\0{}\0{}:{}\0{}:{}",
                recipe_sha256,
                before.frame_id,
                candidate_id,
                initial_surface.pointer.x,
                initial_surface.pointer.y,
                target.x,
                target.y
            )),
        )?;
        let mut acknowledgements = Vec::new();
        let mut effect_trace = EffectTrace::default();
        let mut acknowledged_effect = false;
        let mut key_down: Option<char> = None;

        if stress_context.is_some_and(|context| {
            context.prompt_disposition.state == "operator_intervention_required"
        }) {
            let mut receipt = base_receipt(
                request,
                transaction_id,
                &before,
                &candidate_id,
                target,
                &initial_surface,
                &initial_authority,
                &authority_digest,
                &motion,
                &effect_trace,
                acknowledgements,
            );
            receipt.effect_state = "no_effect".to_string();
            receipt.stop_reason = Some("desktop_prompt_operator_intervention_required".to_string());
            return Ok(receipt);
        }

        for (index, point) in motion.points.iter().copied().enumerate().skip(1) {
            let event = InputEvent::PointerMove {
                point,
                at_ms: scheduled_time(index, motion.points.len(), motion.duration_ms),
            };
            match execute_guarded_event(
                request,
                &before.binding,
                &initial_authority,
                &initial_surface,
                target,
                target_bounds,
                claim,
                dependencies,
                &mut effect_trace,
                &event,
                None,
            ) {
                Ok(ack) => {
                    acknowledgements.push(ack.acknowledgement_id);
                    acknowledged_effect = true;
                }
                Err(error) if acknowledged_effect => {
                    return Err(effect_error(
                        error.code,
                        error.message,
                        base_receipt(
                            request,
                            transaction_id,
                            &before,
                            &candidate_id,
                            target,
                            &initial_surface,
                            &initial_authority,
                            &authority_digest,
                            &motion,
                            &effect_trace,
                            acknowledgements,
                        ),
                        "not_needed",
                        "effect_uncertain",
                    ));
                }
                Err(error) => return Err(error),
            }
        }

        let down = InputEvent::LeftDown {
            at_ms: motion.duration_ms,
        };
        let ack = execute_guarded_event(
            request,
            &before.binding,
            &initial_authority,
            &initial_surface,
            target,
            target_bounds,
            claim,
            dependencies,
            &mut effect_trace,
            &down,
            Some(before.captured_at_ms),
        )
        .map_err(|error| {
            post_ack_error(
                error,
                acknowledged_effect,
                request,
                transaction_id,
                &before,
                &candidate_id,
                target,
                &initial_surface,
                &initial_authority,
                &authority_digest,
                &motion,
                &effect_trace,
                acknowledgements.clone(),
            )
        })?;
        acknowledgements.push(ack.acknowledgement_id);
        let mut button_down = true;

        let hold_ms = motion.hold_ms;
        let up = InputEvent::LeftUp {
            at_ms: motion.duration_ms + hold_ms,
            emergency: false,
        };
        match execute_guarded_event(
            request,
            &before.binding,
            &initial_authority,
            &initial_surface,
            target,
            target_bounds,
            claim,
            dependencies,
            &mut effect_trace,
            &up,
            None,
        ) {
            Ok(ack) => {
                acknowledgements.push(ack.acknowledgement_id);
                button_down = false;
            }
            Err(error) => {
                let cleanup = emergency_release(
                    ReleaseContext {
                        request,
                        provider: dependencies.provider,
                        authority: dependencies.authority,
                        binding: &before.binding,
                        surface: &initial_surface,
                        claim,
                        acknowledgements: &mut acknowledgements,
                        effect_trace: &mut effect_trace,
                    },
                    button_down,
                    key_down,
                    motion.duration_ms + hold_ms + 1,
                );
                return Err(effect_error(
                    if cleanup {
                        error.code
                    } else {
                        "desktop_input_cleanup_failed"
                    },
                    if cleanup {
                        error.message
                    } else {
                        "input release and emergency cleanup failed"
                    },
                    base_receipt(
                        request,
                        transaction_id,
                        &before,
                        &candidate_id,
                        target,
                        &initial_surface,
                        &initial_authority,
                        &authority_digest,
                        &motion,
                        &effect_trace,
                        acknowledgements,
                    ),
                    if cleanup {
                        "released"
                    } else {
                        "release_failed"
                    },
                    "effect_uncertain",
                ));
            }
        }

        let mut event_time = motion.duration_ms + hold_ms;
        for (index, key) in FIXED_TEXT.chars().enumerate() {
            event_time += key_delay_ms(index, &motion.seed_digest);
            let down = InputEvent::KeyDown {
                key,
                at_ms: event_time,
            };
            match execute_guarded_event(
                request,
                &before.binding,
                &initial_authority,
                &initial_surface,
                target,
                target_bounds,
                claim,
                dependencies,
                &mut effect_trace,
                &down,
                None,
            ) {
                Ok(ack) => {
                    acknowledgements.push(ack.acknowledgement_id);
                    key_down = Some(key);
                }
                Err(error) => {
                    return Err(with_cleanup(
                        error,
                        request,
                        transaction_id,
                        &before,
                        &candidate_id,
                        target,
                        &initial_surface,
                        &initial_authority,
                        &authority_digest,
                        &motion,
                        &mut effect_trace,
                        acknowledgements,
                        dependencies.provider,
                        dependencies.authority,
                        &initial_surface,
                        claim,
                        button_down,
                        key_down,
                        event_time + 1,
                    ));
                }
            }
            let up = InputEvent::KeyUp {
                key,
                at_ms: event_time + 1,
                emergency: false,
            };
            match execute_guarded_event(
                request,
                &before.binding,
                &initial_authority,
                &initial_surface,
                target,
                target_bounds,
                claim,
                dependencies,
                &mut effect_trace,
                &up,
                None,
            ) {
                Ok(ack) => {
                    acknowledgements.push(ack.acknowledgement_id);
                    key_down = None;
                }
                Err(error) => {
                    let cleanup = emergency_release(
                        ReleaseContext {
                            request,
                            provider: dependencies.provider,
                            authority: dependencies.authority,
                            binding: &before.binding,
                            surface: &initial_surface,
                            claim,
                            acknowledgements: &mut acknowledgements,
                            effect_trace: &mut effect_trace,
                        },
                        button_down,
                        key_down,
                        event_time + 2,
                    );
                    return Err(effect_error(
                        if cleanup {
                            error.code
                        } else {
                            "desktop_input_cleanup_failed"
                        },
                        if cleanup {
                            error.message
                        } else {
                            "keyboard release and emergency cleanup failed"
                        },
                        base_receipt(
                            request,
                            transaction_id,
                            &before,
                            &candidate_id,
                            target,
                            &initial_surface,
                            &initial_authority,
                            &authority_digest,
                            &motion,
                            &effect_trace,
                            acknowledgements,
                        ),
                        if cleanup {
                            "released"
                        } else {
                            "release_failed"
                        },
                        "effect_uncertain",
                    ));
                }
            }
        }

        validate_guarded_boundary(
            request,
            &before.binding,
            &initial_authority,
            &initial_surface,
            target,
            target_bounds,
            claim,
            dependencies,
        )
        .map_err(|error| {
            post_ack_error(
                error,
                true,
                request,
                transaction_id,
                &before,
                &candidate_id,
                target,
                &initial_surface,
                &initial_authority,
                &authority_digest,
                &motion,
                &effect_trace,
                acknowledgements.clone(),
            )
        })?;
        let after = dependencies
            .provider
            .observe_after(&before.binding)
            .map_err(|_| {
                effect_error(
                    "desktop_interaction_verification_unavailable",
                    "after-state evidence is unavailable",
                    base_receipt(
                        request,
                        transaction_id,
                        &before,
                        &candidate_id,
                        target,
                        &initial_surface,
                        &initial_authority,
                        &authority_digest,
                        &motion,
                        &effect_trace,
                        acknowledgements.clone(),
                    ),
                    "released",
                    "effect_uncertain",
                )
            })?;
        if !same_binding(&before.binding, &after.binding) {
            return Err(effect_error(
                "desktop_interaction_verification_unavailable",
                "after-state desktop binding changed",
                base_receipt(
                    request,
                    transaction_id,
                    &before,
                    &candidate_id,
                    target,
                    &initial_surface,
                    &initial_authority,
                    &authority_digest,
                    &motion,
                    &effect_trace,
                    acknowledgements,
                ),
                "released",
                "effect_uncertain",
            ));
        }
        if after.verification_state != "passed"
            || after.text_sha256.as_deref() != Some(digest_text(FIXED_TEXT).as_str())
        {
            let mut receipt = base_receipt(
                request,
                transaction_id,
                &before,
                &candidate_id,
                target,
                &initial_surface,
                &initial_authority,
                &authority_digest,
                &motion,
                &effect_trace,
                acknowledgements,
            );
            apply_after(&mut receipt, &after);
            return Err(effect_error(
                "desktop_interaction_verification_failed",
                "after-state verification did not establish the recipe outcome",
                receipt,
                "released",
                "effect_uncertain",
            ));
        }

        let mut receipt = base_receipt(
            request,
            transaction_id,
            &before,
            &candidate_id,
            target,
            &initial_surface,
            &initial_authority,
            &authority_digest,
            &motion,
            &effect_trace,
            acknowledgements,
        );
        apply_after(&mut receipt, &after);
        receipt.cleanup_state = "released".to_string();
        receipt.verification_state = "passed".to_string();
        receipt.effect_state = "verified_success".to_string();
        Ok(receipt)
    })();
    ClaimedResult { outcome }
}

fn validate_request(request: &DesktopInteractionRequest) -> Result<(), DesktopInteractionError> {
    if ![
        RECIPE_ID,
        FOUNDATION_STRESS_RECIPE_ID,
        CONTROLLED_X11_RECIPE_ID,
    ]
    .contains(&request.recipe_id.as_str())
        || request.browser_id.trim().is_empty()
        || request.controller_lease_id.trim().is_empty()
        || request.operation_id.trim().is_empty()
        || request.operation_id.len() > 128
        || request.operation_principal_id.trim().is_empty()
        || request.operation_principal_id.len() > 256
        || request.request_principal_source.as_deref() != Some("attribution_tuple_v1")
        || request.service_name.trim().is_empty()
        || request.task_name.trim().is_empty()
        || request.caller_id.trim().is_empty()
        || request.request_id.trim().is_empty()
        || request.agent_name != "fixture-agent"
        || FIXED_TEXT.len() > 32
        || !FIXED_TEXT.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b' ' || byte == b'-'
        })
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_unsupported",
            "the interaction request does not match the named synthetic recipe",
        ));
    }
    Ok(())
}

fn validate_before(
    request: &DesktopInteractionRequest,
    before: &BeforeObservation,
) -> Result<(), DesktopInteractionError> {
    if before.binding.browser_id != request.browser_id
        || request
            .session_name
            .as_deref()
            .is_some_and(|session| session != before.binding.session_name)
        || before.binding.coordinate_space != COORDINATE_SPACE
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_coordinate_mismatch",
            "the observation does not match the requested physical desktop binding",
        ));
    }
    if before.observation_status != "matched"
        || before.selected_candidate_id.is_none()
        || before.selected_target_class.as_deref() != Some("synthetic_verification_control")
        || before.selected_bounds.is_none()
        || before.selected_center.is_none()
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_target_unavailable",
            "the named recipe has no exact selected synthetic target",
        ));
    }
    Ok(())
}

fn validate_authority(
    request: &DesktopInteractionRequest,
    binding: &DesktopBinding,
    authority: &ControllerAuthority,
    now_ms: u64,
) -> Result<String, DesktopInteractionError> {
    let lease = request.controller_lease_id.as_str();
    let provider = authority.route_machine_input.as_deref();
    if authority.browser_id != binding.browser_id
        || authority.display_allocation_id != binding.display_allocation_id
        || authority.stream_id != binding.stream_id
        || authority.route_id != binding.route_id
        || authority.route_controller_lease_id != lease
        || authority.stream_controller_lease_id != lease
        || authority.lease_id != lease
        || authority.lease_record_id != lease
        || authority.lease_route_id != binding.route_id
        || authority.lease_browser_id != binding.browser_id
        || authority.lease_viewer_id != request.agent_name
        || authority.lease_role != "controller"
        || authority.lease_state != "controlling"
        || authority.lease_updated_at.trim().is_empty()
        || !authority.route_contains_lease
        || !authority.stream_contains_lease
        || !authority.route_writable
        || !authority.stream_writable
        || provider.is_none()
        || provider == Some("manual_attached_desktop")
        || provider != authority.stream_machine_input.as_deref()
        || authority.controller_epoch == 0
        || authority.route_controller_epoch != authority.controller_epoch
        || authority.stream_controller_epoch != authority.controller_epoch
        || authority.lease_expires_at_ms <= now_ms
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_authority_required",
            "current machine controller authority was not proven",
        ));
    }
    Ok(authority_digest(request, binding, authority))
}

#[allow(clippy::too_many_arguments)]
fn execute_guarded_event(
    request: &DesktopInteractionRequest,
    binding: &DesktopBinding,
    initial: &ControllerAuthority,
    initial_surface: &SurfaceSnapshot,
    target: PixelPoint,
    target_bounds: PixelBounds,
    claim: &DesktopInteractionClaim,
    dependencies: &mut InteractionDependencies<'_>,
    effect_trace: &mut EffectTrace,
    event: &InputEvent,
    freshness_capture_ms: Option<u64>,
) -> Result<EventAcknowledgement, DesktopInteractionError> {
    let _guard = claim.begin_event().map_err(|code| {
        DesktopInteractionError::new(
            if code == "desktop_interaction_conflict" {
                "desktop_interaction_conflict"
            } else {
                "desktop_interaction_authority_changed"
            },
            "desktop event authority fence is unavailable",
        )
    })?;
    let now = dependencies.clock.now_ms();
    let current = dependencies.authority.snapshot()?;
    let digest = validate_authority(request, binding, &current, now).map_err(|_| {
        DesktopInteractionError::new(
            "desktop_interaction_authority_changed",
            "controller authority changed during interaction",
        )
    })?;
    if digest != authority_digest(request, binding, initial) {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_authority_changed",
            "controller authority changed during interaction",
        ));
    }
    if freshness_capture_ms
        .is_some_and(|captured_at| now.saturating_sub(captured_at) > FRESHNESS_LIMIT_MS)
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_stale_observation",
            "the selected observation is too old for input",
        ));
    }
    let current_surface = dependencies.provider.probe(binding)?;
    validate_surface_stable(
        binding,
        initial_surface,
        &current_surface,
        target,
        target_bounds,
    )?;
    let effect_key = provider_effect_key(request, effect_trace.attempted_keys.len());
    effect_trace.attempted_keys.push(effect_key.clone());
    effect_trace.attempted_events.push(event.clone());
    let acknowledgement =
        dependencies
            .provider
            .execute_event(binding, &current_surface, &effect_key, event)?;
    effect_trace.acknowledged_keys.push(effect_key);
    Ok(acknowledgement)
}

#[allow(clippy::too_many_arguments)]
fn validate_guarded_boundary(
    request: &DesktopInteractionRequest,
    binding: &DesktopBinding,
    initial: &ControllerAuthority,
    initial_surface: &SurfaceSnapshot,
    target: PixelPoint,
    target_bounds: PixelBounds,
    claim: &DesktopInteractionClaim,
    dependencies: &mut InteractionDependencies<'_>,
) -> Result<(), DesktopInteractionError> {
    let _guard = claim.begin_event().map_err(|_| {
        DesktopInteractionError::new(
            "desktop_interaction_authority_changed",
            "desktop verification authority fence is unavailable",
        )
    })?;
    let now = dependencies.clock.now_ms();
    let current = dependencies.authority.snapshot()?;
    let digest = validate_authority(request, binding, &current, now).map_err(|_| {
        DesktopInteractionError::new(
            "desktop_interaction_authority_changed",
            "controller authority changed during interaction",
        )
    })?;
    if digest != authority_digest(request, binding, initial) {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_authority_changed",
            "controller authority changed during interaction",
        ));
    }
    let surface = dependencies.provider.probe(binding)?;
    validate_surface_stable(binding, initial_surface, &surface, target, target_bounds)
}

fn validate_surface(
    binding: &DesktopBinding,
    surface: &SurfaceSnapshot,
    target: PixelPoint,
    target_bounds: PixelBounds,
) -> Result<(), DesktopInteractionError> {
    if !surface.focused
        || surface.provider_id.trim().is_empty()
        || surface.provider_version.trim().is_empty()
        || surface.provider_capability != "guarded_pointer_keyboard_v1"
        || surface.surface_identity_digest.trim().is_empty()
        || surface.browser_process_identity_digest.trim().is_empty()
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_focus_not_ready",
            "the exact synthetic browser surface is not focused",
        ));
    }
    if surface.width != binding.width
        || surface.height != binding.height
        || surface.scale_millis != binding.scale_millis
        || surface.coordinate_space != binding.coordinate_space
        || surface.geometry_epoch != binding.geometry_epoch
        || target_bounds.width == 0
        || target_bounds.height == 0
        || !display_bounds(binding).contains(surface.pointer)
        || !display_bounds(binding).contains(target)
        || !surface.client_bounds.contains(target)
        || !target_bounds.contains(target)
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_coordinate_mismatch",
            "surface geometry or physical-pixel target mapping does not match the observation",
        ));
    }
    Ok(())
}

fn validate_surface_stable(
    binding: &DesktopBinding,
    initial: &SurfaceSnapshot,
    current: &SurfaceSnapshot,
    target: PixelPoint,
    target_bounds: PixelBounds,
) -> Result<(), DesktopInteractionError> {
    validate_surface(binding, current, target, target_bounds)?;
    if initial.provider_id != current.provider_id
        || initial.provider_version != current.provider_version
        || initial.provider_capability != current.provider_capability
        || initial.surface_identity_digest != current.surface_identity_digest
        || initial.browser_process_identity_digest != current.browser_process_identity_digest
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_focus_changed",
            "focused surface identity changed during interaction",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct MotionPlan {
    points: Vec<PixelPoint>,
    control_points: [PixelPoint; 4],
    duration_ms: u64,
    hold_ms: u64,
    seed_digest: String,
}

fn plan_motion(
    start: PixelPoint,
    target: PixelPoint,
    width: u32,
    height: u32,
    seed_digest: &str,
) -> Result<MotionPlan, DesktopInteractionError> {
    let bounds = PixelBounds {
        x: 0,
        y: 0,
        width,
        height,
    };
    if !bounds.contains(start) || !bounds.contains(target) {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_coordinate_mismatch",
            "pointer trajectory endpoints are outside the display",
        ));
    }
    let dx = i128::from(target.x) - i128::from(start.x);
    let dy = i128::from(target.y) - i128::from(start.y);
    let squared = dx
        .checked_mul(dx)
        .and_then(|value| {
            dy.checked_mul(dy)
                .and_then(|right| value.checked_add(right))
        })
        .ok_or_else(|| {
            DesktopInteractionError::new(
                "desktop_interaction_coordinate_mismatch",
                "pointer distance overflowed",
            )
        })?;
    let distance = integer_sqrt(u128::try_from(squared).map_err(|_| {
        DesktopInteractionError::new(
            "desktop_interaction_coordinate_mismatch",
            "pointer distance is invalid",
        )
    })?);
    let distance_u64 = u64::try_from(distance).map_err(|_| {
        DesktopInteractionError::new(
            "desktop_interaction_coordinate_mismatch",
            "pointer distance exceeds supported bounds",
        )
    })?;
    let steps = if distance_u64 < 4 {
        1
    } else {
        distance_u64.div_ceil(12).clamp(6, 64)
    };
    let duration_ms = (140 + distance_u64 / 3).clamp(160, 650);
    let seed = decode_digest(seed_digest)?;
    let side = if seed[0] & 1 == 0 { 1_i128 } else { -1_i128 };
    let jitter_range = (distance_u64 / 20).saturating_add(1);
    let mut bend = (distance_u64 / 10 + u64::from(seed[1]) % jitter_range).clamp(4, 48);
    let mut controls;
    loop {
        let divisor = i128::from(distance_u64.max(1));
        let offset_x = side * -dy * i128::from(bend) / divisor;
        let offset_y = side * dx * i128::from(bend) / divisor;
        controls = [
            start,
            checked_point(
                i128::from(start.x) + dx / 3 + offset_x,
                i128::from(start.y) + dy / 3 + offset_y,
            )?,
            checked_point(
                i128::from(start.x) + dx * 2 / 3 + offset_x,
                i128::from(start.y) + dy * 2 / 3 + offset_y,
            )?,
            target,
        ];
        if bounds.contains(controls[1]) && bounds.contains(controls[2]) {
            break;
        }
        if bend == 0 {
            controls[1] =
                checked_point(i128::from(start.x) + dx / 3, i128::from(start.y) + dy / 3)?;
            controls[2] = checked_point(
                i128::from(start.x) + dx * 2 / 3,
                i128::from(start.y) + dy * 2 / 3,
            )?;
            break;
        }
        bend -= 1;
    }
    let mut points = Vec::new();
    for index in 0..=steps {
        let point = if index == 0 {
            start
        } else if index == steps {
            target
        } else {
            bezier_point(controls, index, steps)?
        };
        if !bounds.contains(point) {
            return Err(DesktopInteractionError::new(
                "desktop_interaction_coordinate_mismatch",
                "planned pointer trajectory left display bounds",
            ));
        }
        if points.last() != Some(&point) {
            points.push(point);
        }
    }
    if points.first() != Some(&start) || points.last() != Some(&target) {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_coordinate_mismatch",
            "planned pointer trajectory lost an endpoint",
        ));
    }
    Ok(MotionPlan {
        points,
        control_points: controls,
        duration_ms,
        hold_ms: 45 + u64::from(seed[2]) % 46,
        seed_digest: seed_digest.to_string(),
    })
}

fn bezier_point(
    controls: [PixelPoint; 4],
    index: u64,
    steps: u64,
) -> Result<PixelPoint, DesktopInteractionError> {
    const SCALE: i128 = 1_000_000;
    let s = i128::from(index) * SCALE / i128::from(steps);
    let eased = (3 * s * s * SCALE - 2 * s * s * s) / (SCALE * SCALE);
    let inverse = SCALE - eased;
    let weights = [
        inverse * inverse * inverse,
        3 * inverse * inverse * eased,
        3 * inverse * eased * eased,
        eased * eased * eased,
    ];
    let divisor = SCALE * SCALE * SCALE;
    let evaluate = |coordinate: fn(PixelPoint) -> i64| {
        let total = controls
            .iter()
            .zip(weights)
            .try_fold(0_i128, |sum, (point, weight)| {
                i128::from(coordinate(*point))
                    .checked_mul(weight)
                    .and_then(|term| sum.checked_add(term))
            })?;
        Some(round_half_away(total, divisor))
    };
    checked_point(
        evaluate(|point| point.x).ok_or_else(motion_overflow)?,
        evaluate(|point| point.y).ok_or_else(motion_overflow)?,
    )
}

fn round_half_away(value: i128, divisor: i128) -> i128 {
    if value >= 0 {
        (value + divisor / 2) / divisor
    } else {
        (value - divisor / 2) / divisor
    }
}

fn checked_point(x: i128, y: i128) -> Result<PixelPoint, DesktopInteractionError> {
    Ok(PixelPoint {
        x: i64::try_from(x).map_err(|_| motion_overflow())?,
        y: i64::try_from(y).map_err(|_| motion_overflow())?,
    })
}

fn motion_overflow() -> DesktopInteractionError {
    DesktopInteractionError::new(
        "desktop_interaction_coordinate_mismatch",
        "fixed-point pointer trajectory overflowed",
    )
}

fn integer_sqrt(value: u128) -> u128 {
    if value < 2 {
        return value;
    }
    let mut left = 1_u128;
    let mut right = value.min(u128::from(u64::MAX));
    let mut answer = 1_u128;
    while left <= right {
        let middle = left + (right - left) / 2;
        if middle <= value / middle {
            answer = middle;
            left = middle + 1;
        } else {
            right = middle - 1;
        }
    }
    answer
}

fn scheduled_time(index: usize, count: usize, duration_ms: u64) -> u64 {
    if count <= 1 {
        return duration_ms;
    }
    duration_ms * index as u64 / (count - 1) as u64
}

fn key_delay_ms(index: usize, seed_digest: &str) -> u64 {
    let seed = decode_digest(seed_digest).expect("internally generated SHA-256");
    35 + (u64::from(seed[(index + 3) % seed.len()]) % 31)
}

struct ReleaseContext<'a> {
    request: &'a DesktopInteractionRequest,
    provider: &'a mut dyn DesktopInteractionProvider,
    authority: &'a mut dyn ControllerAuthorityRepository,
    binding: &'a DesktopBinding,
    surface: &'a SurfaceSnapshot,
    claim: &'a DesktopInteractionClaim,
    acknowledgements: &'a mut Vec<String>,
    effect_trace: &'a mut EffectTrace,
}

fn emergency_release(
    context: ReleaseContext<'_>,
    button_down: bool,
    key_down: Option<char>,
    at_ms: u64,
) -> bool {
    let event = if let Some(key) = key_down {
        Some(InputEvent::KeyUp {
            key,
            at_ms,
            emergency: true,
        })
    } else if button_down {
        Some(InputEvent::LeftUp {
            at_ms,
            emergency: true,
        })
    } else {
        None
    };
    let ReleaseContext {
        request,
        provider,
        authority,
        binding,
        surface,
        claim,
        acknowledgements,
        effect_trace,
    } = context;
    match event {
        Some(event) => {
            let Ok(_guard) = claim.begin_cleanup_event() else {
                return false;
            };
            if authority.snapshot().is_err() {
                return false;
            }
            let Ok(current_surface) = provider.probe(binding) else {
                return false;
            };
            if validate_cleanup_surface_stable(binding, surface, &current_surface).is_err() {
                return false;
            }
            let effect_key = provider_effect_key(request, effect_trace.attempted_keys.len());
            effect_trace.attempted_keys.push(effect_key.clone());
            effect_trace.attempted_events.push(event.clone());
            match provider.execute_event(binding, &current_surface, &effect_key, &event) {
                Ok(ack) => {
                    effect_trace.acknowledged_keys.push(effect_key);
                    acknowledgements.push(ack.acknowledgement_id);
                    true
                }
                Err(_) => false,
            }
        }
        None => true,
    }
}

fn validate_cleanup_surface_stable(
    binding: &DesktopBinding,
    initial: &SurfaceSnapshot,
    current: &SurfaceSnapshot,
) -> Result<(), DesktopInteractionError> {
    if !current.focused
        || current.provider_id.trim().is_empty()
        || current.provider_version.trim().is_empty()
        || current.provider_capability != "guarded_pointer_keyboard_v1"
        || current.width != binding.width
        || current.height != binding.height
        || current.scale_millis != binding.scale_millis
        || current.coordinate_space != binding.coordinate_space
        || current.geometry_epoch != binding.geometry_epoch
        || !display_bounds(binding).contains(current.pointer)
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_coordinate_mismatch",
            "cleanup surface geometry does not match the bound desktop",
        ));
    }
    if initial.provider_id != current.provider_id
        || initial.provider_version != current.provider_version
        || initial.provider_capability != current.provider_capability
        || initial.surface_identity_digest != current.surface_identity_digest
        || initial.browser_process_identity_digest != current.browser_process_identity_digest
    {
        return Err(DesktopInteractionError::new(
            "desktop_interaction_focus_changed",
            "cleanup focused surface identity changed during interaction",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn with_cleanup(
    error: DesktopInteractionError,
    request: &DesktopInteractionRequest,
    transaction_id: &str,
    before: &BeforeObservation,
    candidate_id: &str,
    target: PixelPoint,
    surface: &SurfaceSnapshot,
    authority: &ControllerAuthority,
    authority_digest: &str,
    motion: &MotionPlan,
    effect_trace: &mut EffectTrace,
    mut acknowledgements: Vec<String>,
    provider: &mut dyn DesktopInteractionProvider,
    authority_repository: &mut dyn ControllerAuthorityRepository,
    cleanup_surface: &SurfaceSnapshot,
    claim: &DesktopInteractionClaim,
    button_down: bool,
    key_down: Option<char>,
    at_ms: u64,
) -> DesktopInteractionError {
    let cleanup = emergency_release(
        ReleaseContext {
            request,
            provider,
            authority: authority_repository,
            binding: &before.binding,
            surface: cleanup_surface,
            claim,
            acknowledgements: &mut acknowledgements,
            effect_trace,
        },
        button_down,
        key_down,
        at_ms,
    );
    effect_error(
        if cleanup {
            error.code
        } else {
            "desktop_input_cleanup_failed"
        },
        if cleanup {
            error.message
        } else {
            "emergency input release failed"
        },
        base_receipt(
            request,
            transaction_id,
            before,
            candidate_id,
            target,
            surface,
            authority,
            authority_digest,
            motion,
            effect_trace,
            acknowledgements,
        ),
        if cleanup {
            "released"
        } else {
            "release_failed"
        },
        if error.code == "desktop_interaction_authority_changed" {
            "cancelled_after_effect"
        } else {
            "effect_uncertain"
        },
    )
}

fn effect_error(
    code: &'static str,
    message: &'static str,
    mut receipt: InteractionReceipt,
    cleanup_state: &str,
    effect_state: &str,
) -> DesktopInteractionError {
    receipt.cleanup_state = cleanup_state.to_string();
    receipt.effect_state = effect_state.to_string();
    receipt.verification_state = "not_verified".to_string();
    receipt.stop_reason = Some(code.to_string());
    DesktopInteractionError {
        code,
        message,
        receipt: Some(Box::new(receipt)),
    }
}

#[allow(clippy::too_many_arguments)]
fn post_ack_error(
    error: DesktopInteractionError,
    acknowledged_effect: bool,
    request: &DesktopInteractionRequest,
    transaction_id: &str,
    before: &BeforeObservation,
    candidate_id: &str,
    target: PixelPoint,
    surface: &SurfaceSnapshot,
    authority: &ControllerAuthority,
    authority_digest: &str,
    motion: &MotionPlan,
    effect_trace: &EffectTrace,
    acknowledgements: Vec<String>,
) -> DesktopInteractionError {
    if !acknowledged_effect {
        return error;
    }
    let effect_state = if error.code == "desktop_interaction_authority_changed" {
        "cancelled_after_effect"
    } else {
        "effect_uncertain"
    };
    effect_error(
        error.code,
        error.message,
        base_receipt(
            request,
            transaction_id,
            before,
            candidate_id,
            target,
            surface,
            authority,
            authority_digest,
            motion,
            effect_trace,
            acknowledgements,
        ),
        "not_needed",
        effect_state,
    )
}

#[allow(clippy::too_many_arguments)]
fn base_receipt(
    request: &DesktopInteractionRequest,
    transaction_id: &str,
    before: &BeforeObservation,
    candidate_id: &str,
    target: PixelPoint,
    surface: &SurfaceSnapshot,
    authority: &ControllerAuthority,
    authority_digest: &str,
    motion: &MotionPlan,
    effect_trace: &EffectTrace,
    acknowledgement_ids: Vec<String>,
) -> InteractionReceipt {
    InteractionReceipt {
        transaction_id: transaction_id.to_string(),
        schema_version: "v1".to_string(),
        recipe_id: request.recipe_id.clone(),
        recipe_version: RECIPE_VERSION.to_string(),
        recipe_sha256: recipe_sha256(&request.recipe_id),
        operation_id: request.operation_id.clone(),
        operation_request_sha256: operation_request_sha256(request),
        replay_state: "first_execution".to_string(),
        recipe_provider_id: surface.provider_id.clone(),
        recipe_provider_version: surface.provider_version.clone(),
        recipe_provider_capability: surface.provider_capability.clone(),
        prompt_disposition: None,
        human_handoff: None,
        entry_gate: "closed_live_evidence_required".to_string(),
        effect_key_digest: digest_json(&effect_trace.acknowledged_keys),
        effect_key_count: effect_trace.acknowledged_keys.len(),
        attempted_effect_key_digest: digest_json(&effect_trace.attempted_keys),
        attempted_effect_key_count: effect_trace.attempted_keys.len(),
        acknowledged_effect_key_digest: digest_json(&effect_trace.acknowledged_keys),
        acknowledged_effect_key_count: effect_trace.acknowledged_keys.len(),
        attempted_event_order_sha256: digest_json(&effect_trace.attempted_events),
        browser_id: before.binding.browser_id.clone(),
        display_allocation_id: before.binding.display_allocation_id.clone(),
        stream_id: before.binding.stream_id.clone(),
        route_id: before.binding.route_id.clone(),
        controller_epoch: authority.controller_epoch,
        authority_digest: authority_digest.to_string(),
        actor_digest: digest_text(&format!(
            "{}\0{}",
            request.operation_principal_id, request.agent_name
        )),
        before_context_id: before.context_id.clone(),
        before_frame_id: before.frame_id.clone(),
        before_frame_sha256: before.frame_sha256.clone(),
        before_observation_id: before.observation_id.clone(),
        before_observation_sha256: before.observation_sha256.clone(),
        selected_candidate_id: candidate_id.to_string(),
        surface_identity_digest: surface.surface_identity_digest.clone(),
        browser_process_identity_digest: surface.browser_process_identity_digest.clone(),
        pointer_start: surface.pointer,
        target,
        coordinate_mapping: "identity_physical_pixels_v1".to_string(),
        motion_profile: "fixed_cubic_bezier_v1".to_string(),
        control_point_digest: digest_json(&motion.control_points),
        emitted_path_sha256: digest_json(&motion.points),
        pointer_event_count: motion.points.len().saturating_sub(1),
        duration_ms: motion.duration_ms,
        acknowledgement_ids,
        cleanup_state: "not_needed".to_string(),
        text_length: FIXED_TEXT.len(),
        text_sha256: digest_text(FIXED_TEXT),
        after_context_id: None,
        after_frame_id: None,
        after_frame_sha256: None,
        after_observation_id: None,
        after_observation_sha256: None,
        verification_state: "not_verified".to_string(),
        effect_state: "effect_uncertain".to_string(),
        stop_reason: None,
        retention: "ephemeral".to_string(),
        persisted_pixels: false,
    }
}

fn apply_after(receipt: &mut InteractionReceipt, after: &AfterObservation) {
    receipt.after_context_id = Some(after.context_id.clone());
    receipt.after_frame_id = Some(after.frame_id.clone());
    receipt.after_frame_sha256 = Some(after.frame_sha256.clone());
    receipt.after_observation_id = Some(after.observation_id.clone());
    receipt.after_observation_sha256 = Some(after.observation_sha256.clone());
    receipt.verification_state = after.verification_state.clone();
}

fn display_bounds(binding: &DesktopBinding) -> PixelBounds {
    PixelBounds {
        x: 0,
        y: 0,
        width: binding.width,
        height: binding.height,
    }
}

fn same_binding(left: &DesktopBinding, right: &DesktopBinding) -> bool {
    left == right
}

fn authority_digest(
    request: &DesktopInteractionRequest,
    binding: &DesktopBinding,
    authority: &ControllerAuthority,
) -> String {
    digest_text(&format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        authority.controller_epoch,
        authority.lease_id,
        authority.lease_updated_at,
        binding.browser_id,
        binding.route_id,
        binding.stream_id,
        binding.display_allocation_id,
        binding.geometry_epoch,
        request.operation_principal_id,
        request.operation_id,
        authority.route_machine_input.as_deref().unwrap_or("")
    ))
}

fn recipe_sha256(recipe_id: &str) -> String {
    digest_text(&format!(
        "{recipe_id}\0{RECIPE_VERSION}\0p110-control-v1\0{FIXED_TEXT}\0fixed_cubic_bezier_v1"
    ))
}

fn operation_request_sha256(request: &DesktopInteractionRequest) -> String {
    digest_text(&format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}",
        request.browser_id,
        request.session_name.as_deref().unwrap_or(""),
        request.controller_lease_id,
        request.recipe_id,
        request.service_name,
        request.agent_name,
        request.task_name
    ))
}

fn operation_scope_sha256(caller_id: &str, operation_id: &str) -> String {
    digest_text(&format!("{caller_id}\0{operation_id}"))
}

fn provider_effect_key(request: &DesktopInteractionRequest, event_index: usize) -> String {
    digest_text(&format!(
        "{}\0{}\0{}\0{}",
        operation_scope_sha256(&request.operation_principal_id, &request.operation_id),
        operation_request_sha256(request),
        recipe_sha256(&request.recipe_id),
        event_index
    ))
}

fn finalize_stress_receipt(
    request: &DesktopInteractionRequest,
    context: Option<&FoundationStressContext>,
    receipt: &mut InteractionReceipt,
    handoffs: &mut dyn ServiceOwnedHandoffRepository,
) -> Result<(), DesktopInteractionError> {
    if request.recipe_id != FOUNDATION_STRESS_RECIPE_ID {
        return Ok(());
    }
    receipt.prompt_disposition = context.map(|value| value.prompt_disposition.clone());
    if receipt.effect_state == "effect_uncertain"
        || receipt.effect_state == "cancelled_after_effect"
        || receipt
            .prompt_disposition
            .as_ref()
            .is_some_and(|prompt| prompt.state == "operator_intervention_required")
    {
        if let Some(reason) = context.and_then(|value| value.handoff_reason.as_deref()) {
            receipt.human_handoff = handoffs.resolve_ready(
                &receipt.browser_id,
                request.session_name.as_deref().unwrap_or(""),
                &receipt.route_id,
                &receipt.display_allocation_id,
                reason,
            )?;
        }
    }
    Ok(())
}

fn digest_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn digest_json<T: Serialize>(value: &T) -> String {
    digest_text(&serde_json::to_string(value).expect("internal receipt data serializes"))
}

fn decode_digest(value: &str) -> Result<[u8; 32], DesktopInteractionError> {
    if value.len() != 64 {
        return Err(motion_overflow());
    }
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| motion_overflow())?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FoundationStressScenarioRow {
        scenario_id: String,
        phase: String,
        expected_effect_state: String,
        expected_handoff_state: String,
        operation_request_sha256: String,
        expected_provider_call_count: usize,
        expected_event_order_sha256: String,
        expected_effect_key_trace_sha256: String,
        expected_authority_epoch: u64,
        expected_projection_sha256: String,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct MaterializedStressScenario {
        scenario_id: String,
        phase: String,
        operation_request_sha256: String,
        provider_id: String,
        provider_version: String,
        provider_capability: String,
        provider_call_count: usize,
        event_order_sha256: String,
        effect_key_trace_sha256: String,
        authority_epoch: u64,
        effect_state: String,
        handoff_state: String,
        projection_sha256: String,
    }

    #[test]
    fn matched_fixture_runs_one_bounded_transaction_and_verifies() {
        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut authority = ScriptedAuthority::stable(fixture.authority());
        let mut coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);

        let receipt = run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &mut coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut RejectHandoffLookup,
                clock: &mut clock,
            },
        )
        .expect("ready fixture should verify");

        assert_eq!(receipt.effect_state, "verified_success");
        assert_eq!(receipt.verification_state, "passed");
        assert!(receipt.pointer_event_count <= 64);
        assert_eq!(
            fixture.events.last(),
            Some(&InputEvent::KeyUp {
                key: 'y',
                at_ms: fixture.events.last().unwrap().at_ms(),
                emergency: false,
            })
        );
        assert!(fixture.activated);
        assert_eq!(fixture.typed, FIXED_TEXT);
        assert!(!serde_json::to_string(&receipt)
            .unwrap()
            .contains(FIXED_TEXT));
    }

    #[test]
    fn motion_is_byte_stable_bounded_and_identity_mapped() {
        let seed = digest_text("p110-motion-seed");
        for (start, target, width, height) in [
            (
                PixelPoint { x: 12, y: 20 },
                PixelPoint { x: 160, y: 100 },
                320,
                200,
            ),
            (
                PixelPoint { x: 1, y: 1 },
                PixelPoint { x: 318, y: 198 },
                320,
                200,
            ),
            (
                PixelPoint { x: 158, y: 99 },
                PixelPoint { x: 160, y: 100 },
                320,
                200,
            ),
            (
                PixelPoint { x: 318, y: 1 },
                PixelPoint { x: 304, y: 14 },
                320,
                200,
            ),
        ] {
            let first = plan_motion(start, target, width, height, &seed).unwrap();
            let second = plan_motion(start, target, width, height, &seed).unwrap();
            assert_eq!(first.points, second.points);
            assert_eq!(first.control_points, second.control_points);
            assert_eq!(first.points.first(), Some(&start));
            assert_eq!(first.points.last(), Some(&target));
            assert!(first.points.len().saturating_sub(1) <= 64);
            assert!((160..=650).contains(&first.duration_ms));
            assert!(first.points.iter().all(|point| PixelBounds {
                x: 0,
                y: 0,
                width,
                height,
            }
            .contains(*point)));
        }
    }

    #[test]
    fn current_controller_is_required_before_input() {
        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut invalid = fixture.authority();
        invalid.lease_role = "observer".to_string();
        let mut authority = ScriptedAuthority::stable(invalid);
        let mut coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        let error = run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &mut coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap_err();
        assert_eq!(error.code(), "desktop_interaction_authority_required");
        assert!(fixture.events.is_empty());
    }

    #[test]
    fn completed_request_replays_without_input() {
        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut authority = ScriptedAuthority::stable(fixture.authority());
        let mut coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        let first = run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &mut coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap();
        let emitted = fixture.events.len();
        let second = run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &mut coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap();
        assert_eq!(first.replay_state, "first_execution");
        assert_eq!(second.replay_state, "replayed_terminal");
        let mut expected = first;
        expected.replay_state = "replayed_terminal".to_string();
        assert_eq!(expected, second);
        assert_eq!(fixture.events.len(), emitted);
    }

    #[test]
    fn foundation_stress_replays_after_ledger_reload_and_conflicts_fail_closed() {
        let mut stress = request();
        stress.recipe_id = FOUNDATION_STRESS_RECIPE_ID.to_string();
        stress.operation_id = "stress-operation-1".to_string();
        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut authority = ScriptedAuthority::stable(fixture.authority());
        let coordinator = SyntheticCoordinator::default();
        let ledger_path = stress_ledger_path("terminal-reload");
        let _ = fs::remove_file(&ledger_path);
        let mut ledger = PersistedInteractionOperationLedger::open(&ledger_path).unwrap();
        let mut clock = FixedClock::new(1_000);
        let first = run_desktop_interaction(
            stress.clone(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut ledger,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap();
        assert_eq!(first.replay_state, "first_execution");
        assert_eq!(first.entry_gate, "closed_live_evidence_required");
        assert_eq!(
            first.prompt_disposition.as_ref().unwrap().state,
            "actionable_observation"
        );
        assert_eq!(first.effect_key_count, first.acknowledgement_ids.len());
        assert_eq!(first.attempted_effect_key_count, fixture.events.len());
        assert_eq!(first.acknowledged_effect_key_count, fixture.events.len());
        let manifest: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/fixtures/desktop-foundation-stress/verified-success-replay.json"
        ))
        .unwrap();
        assert_eq!(
            first.operation_request_sha256,
            manifest["operationRequestSha256"]
        );
        assert_eq!(
            first.attempted_effect_key_digest,
            manifest["expectedEffectKeyTraceSha256"]
        );
        assert_eq!(
            receipt_projection_sha256(&first, "absent"),
            manifest["expectedReceiptProjectionSha256"]
        );
        let emitted = fixture.events.len();

        let serialized = fs::read_to_string(&ledger_path).unwrap();
        assert!(!serialized.contains("stress-operation-1"));
        assert!(!serialized.contains("route-1"));
        assert!(!serialized.contains("display-1"));
        assert!(!serialized.contains("stream-1"));
        drop(ledger);
        let mut reloaded = PersistedInteractionOperationLedger::open(&ledger_path).unwrap();
        stress.request_id = "another-transport-request".to_string();
        let replay = run_desktop_interaction(
            stress.clone(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut reloaded,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap();
        assert_eq!(replay.replay_state, "replayed_terminal");
        assert_eq!(replay.operation_id, "stress-operation-1");
        assert_eq!(fixture.events.len(), emitted);

        stress.browser_id = "browser-conflict".to_string();
        let error = run_desktop_interaction(
            stress,
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut reloaded,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap_err();
        assert_eq!(error.code(), "desktop_interaction_operation_conflict");
        assert_eq!(fixture.events.len(), emitted);
        fs::remove_file(ledger_path).unwrap();
    }

    #[test]
    fn abandoned_in_progress_reload_fails_closed_without_provider_calls() {
        let ledger_path = stress_ledger_path("in-progress-reload");
        let _ = fs::remove_file(&ledger_path);
        let request = request();
        let mut ledger = PersistedInteractionOperationLedger::open(&ledger_path).unwrap();
        ledger
            .begin(
                &request.operation_principal_id,
                &request.operation_id,
                &operation_request_sha256(&request),
            )
            .unwrap();
        drop(ledger);

        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut authority = ScriptedAuthority::stable(fixture.authority());
        let coordinator = SyntheticCoordinator::default();
        let mut ledger = PersistedInteractionOperationLedger::open(&ledger_path).unwrap();
        let mut clock = FixedClock::new(1_000);
        let error = run_desktop_interaction(
            request,
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut ledger,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap_err();
        assert_eq!(error.code(), "desktop_interaction_duplicate");
        assert!(fixture.events.is_empty());
        fs::remove_file(ledger_path).unwrap();
    }

    #[test]
    fn persisted_ledger_load_and_transition_fail_typed() {
        let malformed_path = stress_ledger_path("malformed-ledger");
        let _ = fs::remove_file(&malformed_path);
        fs::write(&malformed_path, b"not-json").unwrap();
        let error = PersistedInteractionOperationLedger::open(&malformed_path).unwrap_err();
        assert_eq!(error.code(), "desktop_interaction_operation_ledger_invalid");
        fs::remove_file(&malformed_path).unwrap();

        let blocked_parent = stress_ledger_path("blocked-parent");
        let _ = fs::remove_file(&blocked_parent);
        fs::write(&blocked_parent, b"not-a-directory").unwrap();
        let mut ledger = PersistedInteractionOperationLedger {
            path: blocked_parent.join("ledger"),
            inner: SerializedInteractionOperationLedger::default(),
        };
        let error = ledger
            .begin("principal-1", "operation-1", &digest_text("request"))
            .unwrap_err();
        assert_eq!(
            error.code(),
            "desktop_interaction_operation_ledger_save_failed"
        );
        fs::remove_file(blocked_parent).unwrap();
    }

    fn stress_ledger_path(case: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "agent-browser-p110-{case}-{}-{}.json",
            std::process::id(),
            digest_text(case)
        ))
    }

    #[test]
    fn duplicate_provider_effect_key_returns_original_ack_without_emission() {
        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let binding = fixture.binding();
        let surface = fixture.probe(&binding).unwrap();
        let event = InputEvent::PointerMove {
            point: PixelPoint { x: 13, y: 21 },
            at_ms: 1,
        };
        let first = fixture
            .execute_event(&binding, &surface, "duplicate-effect-key", &event)
            .unwrap();
        let surface = fixture.probe(&binding).unwrap();
        let replay = fixture
            .execute_event(&binding, &surface, "duplicate-effect-key", &event)
            .unwrap();
        assert_eq!(first, replay);
        assert_eq!(fixture.events, vec![event]);
    }

    #[test]
    fn effect_keys_bind_principal_request_recipe_and_planned_index() {
        let request = request();
        let first = provider_effect_key(&request, 0);
        assert_ne!(first, provider_effect_key(&request, 1));
        let mut other_principal = request.clone();
        other_principal.operation_principal_id = "principal-2".to_string();
        assert_ne!(first, provider_effect_key(&other_principal, 0));
        let mut other_request = request.clone();
        other_request.task_name = "another-semantic-task".to_string();
        assert_ne!(first, provider_effect_key(&other_request, 0));
        let mut other_recipe = request;
        other_recipe.recipe_id = FOUNDATION_STRESS_RECIPE_ID.to_string();
        assert_ne!(first, provider_effect_key(&other_recipe, 0));
    }

    #[test]
    fn service_owned_handoff_repository_requires_exact_ready_binding() {
        let binding = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 }).binding();
        for mutation in ["missing", "not-ready", "wrong-browser", "wrong-route"] {
            let mut state = ready_handoff_state(&binding);
            if mutation == "missing" {
                state.remote_view_handoffs.clear();
            } else {
                let handoff = state
                    .remote_view_handoffs
                    .get_mut("existing-handoff-1")
                    .unwrap();
                match mutation {
                    "not-ready" => {
                        handoff.last_resolution = Some(json!({
                            "operatorVisible": { "state": "wrong_tab" }
                        }));
                    }
                    "wrong-browser" => handoff.browser_id = Some("browser-other".to_string()),
                    "wrong-route" => handoff.last_route_id = Some("route-other".to_string()),
                    _ => unreachable!(),
                }
            }
            let mut repository = ServiceStateHandoffRepository::new(&state);
            assert_eq!(
                repository
                    .resolve_ready(
                        &binding.browser_id,
                        &binding.session_name,
                        &binding.route_id,
                        &binding.display_allocation_id,
                        "effect_uncertain",
                    )
                    .unwrap(),
                None,
                "{mutation} must not resolve"
            );
        }

        for raw_url in [
            "https://provider.invalid/#/client/raw",
            "guacamole://raw/client",
            "/remote-view/another-handoff",
        ] {
            let mut state = ready_handoff_state(&binding);
            state
                .remote_view_handoffs
                .get_mut("existing-handoff-1")
                .unwrap()
                .handoff_url = Some(raw_url.to_string());
            let mut repository = ServiceStateHandoffRepository::new(&state);
            let error = repository
                .resolve_ready(
                    &binding.browser_id,
                    &binding.session_name,
                    &binding.route_id,
                    &binding.display_allocation_id,
                    "effect_uncertain",
                )
                .unwrap_err();
            assert_eq!(error.code(), "desktop_interaction_handoff_invalid");
        }
    }

    #[test]
    fn stress_redactor_digests_operation_and_omits_route_and_handoff_url() {
        let result = json!({
            "ok": true,
            "action": "desktop_interact",
            "interactionReceipt": {
                "recipeId": FOUNDATION_STRESS_RECIPE_ID,
                "operationId": "operation-secret",
                "routeId": "route-private",
                "displayAllocationId": "display-private",
                "streamId": "stream-private",
                "replayState": "first_execution",
                "humanHandoff": {
                    "state": "ready",
                    "reason": "effect_uncertain",
                    "handoffId": "opaque-handoff",
                    "handoffUrl": "/remote-view/opaque-handoff"
                }
            }
        });
        let redacted = redact_desktop_interaction_stream_result(&result);
        let receipt = &redacted["interactionReceipt"];
        assert_eq!(
            receipt["operationIdDigest"],
            digest_text("operation-secret")
        );
        assert!(receipt.get("operationId").is_none());
        assert!(receipt.get("routeId").is_none());
        assert!(receipt.get("displayAllocationId").is_none());
        assert!(receipt.get("streamId").is_none());
        assert_eq!(receipt["humanHandoff"]["handoffId"], "opaque-handoff");
        assert!(receipt["humanHandoff"].get("handoffUrl").is_none());
    }

    #[test]
    fn stress_prompt_intervention_emits_no_input_and_uncertain_receipt_uses_existing_handoff() {
        let mut intervention_request = request();
        intervention_request.recipe_id = FOUNDATION_STRESS_RECIPE_ID.to_string();
        intervention_request.operation_id = "prompt-operation-1".to_string();
        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        fixture.stress_context = FoundationStressContext {
            prompt_disposition: PromptDisposition {
                state: "operator_intervention_required".to_string(),
                reason_code: "synthetic_prompt_requires_operator_review".to_string(),
                observation_sha256: digest_text("prompt-intervention"),
            },
            handoff_reason: Some("effect_uncertain".to_string()),
        };
        let handoff_state = ready_handoff_state(&fixture.binding());
        let mut handoffs = ServiceStateHandoffRepository::new(&handoff_state);
        let mut authority = ScriptedAuthority::stable(fixture.authority());
        let coordinator = SyntheticCoordinator::default();
        let mut ledger = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        let receipt = run_desktop_interaction(
            intervention_request,
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut ledger,
                handoffs: &mut handoffs,
                clock: &mut clock,
            },
        )
        .unwrap();
        assert_eq!(receipt.effect_state, "no_effect");
        assert_eq!(receipt.effect_key_count, 0);
        assert_eq!(receipt.human_handoff, Some(existing_handoff()));
        assert!(fixture.events.is_empty());
        let prompt_manifest: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/fixtures/desktop-foundation-stress/prompt-intervention.json"
        ))
        .unwrap();
        assert_eq!(
            receipt_projection_sha256(&receipt, "ready"),
            prompt_manifest["expectedReceiptProjectionSha256"]
        );

        let mut uncertain_request = request();
        uncertain_request.recipe_id = FOUNDATION_STRESS_RECIPE_ID.to_string();
        uncertain_request.operation_id = "uncertain-operation".to_string();
        let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut fixture = AdversarialFixture::new(inner);
        fixture.inner.stress_context.handoff_reason = Some("effect_uncertain".to_string());
        let handoff_state = ready_handoff_state(&fixture.inner.binding());
        let mut handoffs = ServiceStateHandoffRepository::new(&handoff_state);
        fixture.after_mode = AfterMode::Unavailable;
        let mut authority = ScriptedAuthority::stable(fixture.inner.authority());
        let mut ledger = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        let error = run_desktop_interaction(
            uncertain_request,
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut ledger,
                handoffs: &mut handoffs,
                clock: &mut clock,
            },
        )
        .unwrap_err();
        assert_eq!(
            error.receipt().unwrap().human_handoff,
            Some(existing_handoff())
        );
        let uncertain_manifest: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/fixtures/desktop-foundation-stress/post-effect-uncertain-handoff.json"
        ))
        .unwrap();
        assert_eq!(
            receipt_projection_sha256(error.receipt().unwrap(), "ready"),
            uncertain_manifest["expectedReceiptProjectionSha256"]
        );
    }

    fn receipt_projection_sha256(receipt: &InteractionReceipt, handoff_state: &str) -> String {
        digest_text(&format!(
            "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
            receipt.operation_request_sha256,
            receipt.recipe_provider_id,
            receipt.recipe_provider_version,
            receipt.recipe_provider_capability,
            receipt.effect_state,
            receipt.cleanup_state,
            receipt.verification_state,
            receipt.replay_state,
            receipt.entry_gate,
            receipt.attempted_effect_key_digest,
            receipt.attempted_effect_key_count,
            handoff_state
        ))
    }

    fn existing_handoff() -> HumanHandoffSummary {
        HumanHandoffSummary {
            state: "ready".to_string(),
            reason: "effect_uncertain".to_string(),
            handoff_id: "existing-handoff-1".to_string(),
            handoff_url: "/remote-view/existing-handoff-1".to_string(),
        }
    }

    fn ready_handoff_state(binding: &DesktopBinding) -> ServiceState {
        let handoff = super::super::service_model::RemoteViewHandoff {
            id: "existing-handoff-1".to_string(),
            state: "ready".to_string(),
            handoff_url: Some("/remote-view/existing-handoff-1".to_string()),
            browser_id: Some(binding.browser_id.clone()),
            session_name: Some(binding.session_name.clone()),
            last_route_id: Some(binding.route_id.clone()),
            last_display_allocation_id: Some(binding.display_allocation_id.clone()),
            last_resolution: Some(json!({
                "operatorVisible": { "state": "ready" }
            })),
            ..super::super::service_model::RemoteViewHandoff::default()
        };
        ServiceState {
            remote_view_handoffs: BTreeMap::from([(handoff.id.clone(), handoff)]),
            ..ServiceState::default()
        }
    }

    #[test]
    fn stream_redactor_keeps_only_frozen_receipt_fields() {
        let result = json!({
            "ok": true,
            "action": "desktop_interact",
            "imageBase64": "private",
            "futureSecret": "private",
            "interactionReceipt": {
                "transactionId": "transaction-1",
                "effectState": "verified_success",
                "pointerStart": { "x": 1, "y": 2, "label": "private" },
                "acknowledgementIds": ["ack-1", { "private": true }],
                "text": FIXED_TEXT,
                "emittedPath": [{ "x": 1, "y": 2 }],
                "outputPath": "/private/path",
                "futureSecret": "private"
            }
        });
        let redacted = redact_desktop_interaction_stream_result(&result);
        let serialized = serde_json::to_string(&redacted).unwrap();
        assert_eq!(
            redacted["interactionReceipt"]["transactionId"],
            "transaction-1"
        );
        assert_eq!(
            redacted["interactionReceipt"]["pointerStart"],
            json!({ "x": 1, "y": 2 })
        );
        assert_eq!(
            redacted["interactionReceipt"]["acknowledgementIds"],
            json!(["ack-1"])
        );
        for private in [
            FIXED_TEXT,
            "private",
            "/private/path",
            "emittedPath",
            "futureSecret",
        ] {
            assert!(!serialized.contains(private));
        }
    }

    #[tokio::test]
    async fn production_dispatch_is_unavailable_without_effect_resolution() {
        let error = handle_desktop_interact(&json!({ "action": "desktop_interact" }))
            .await
            .unwrap_err();
        assert!(error.starts_with("desktop_input_provider_unavailable:"));
    }

    #[tokio::test]
    async fn public_dispatch_rejects_caller_controlled_provider_routing() {
        for forbidden in [
            "coordinates",
            "displayName",
            "xauthorityPath",
            "routeUser",
            "providerExecutable",
            "lockPath",
            "providerUrl",
            "guacamoleUrl",
        ] {
            let error = handle_desktop_interact(&json!({
                "action": "desktop_interact",
                (forbidden): "caller-value",
            }))
            .await
            .unwrap_err();
            assert_eq!(
                error,
                format!("desktop_interact does not accept caller-controlled {forbidden}")
            );
        }
    }

    #[test]
    fn unavailable_or_stale_before_observations_emit_no_events() {
        for (status, captured_at_ms, expected) in [
            ("ambiguous", 900, "desktop_interaction_target_unavailable"),
            ("not_found", 900, "desktop_interaction_target_unavailable"),
            ("matched", 0, "desktop_interaction_stale_observation"),
        ] {
            let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
            let mut fixture = AdversarialFixture::new(inner);
            fixture.before_status = status.to_string();
            fixture.captured_at_ms = captured_at_ms;
            let mut authority = ScriptedAuthority::stable(fixture.inner.authority());
            let mut coordinator = SyntheticCoordinator::default();
            let mut idempotency = MemoryIdempotency::default();
            let mut clock = FixedClock::new(1_000);
            let error = run_desktop_interaction(
                request(),
                InteractionDependencies {
                    provider: &mut fixture,
                    authority: &mut authority,
                    coordinator: &mut coordinator,
                    idempotency: &mut idempotency,
                    handoffs: &mut NoHandoffRepository,
                    clock: &mut clock,
                },
            )
            .unwrap_err();
            assert_eq!(error.code(), expected);
            assert!(fixture.inner.events.is_empty());
        }
    }

    #[test]
    fn focus_and_geometry_drift_stop_before_button_down() {
        for drift in [ProbeDrift::Focus, ProbeDrift::Geometry] {
            let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
            let mut fixture = AdversarialFixture::new(inner);
            fixture.probe_drift = Some((2, drift));
            let mut authority = ScriptedAuthority::stable(fixture.inner.authority());
            let mut coordinator = SyntheticCoordinator::default();
            let mut idempotency = MemoryIdempotency::default();
            let mut clock = FixedClock::new(1_000);
            let error = run_desktop_interaction(
                request(),
                InteractionDependencies {
                    provider: &mut fixture,
                    authority: &mut authority,
                    coordinator: &mut coordinator,
                    idempotency: &mut idempotency,
                    handoffs: &mut NoHandoffRepository,
                    clock: &mut clock,
                },
            )
            .unwrap_err();
            assert!(matches!(
                error.code(),
                "desktop_interaction_focus_not_ready" | "desktop_interaction_coordinate_mismatch"
            ));
            assert!(!fixture
                .inner
                .events
                .iter()
                .any(|event| matches!(event, InputEvent::LeftDown { .. })));
        }
    }

    #[test]
    fn controller_epoch_drift_and_cancellation_stop_the_transaction() {
        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let initial = fixture.authority();
        let mut changed = initial.clone();
        changed.controller_epoch += 1;
        changed.route_controller_epoch += 1;
        changed.stream_controller_epoch += 1;
        let mut authority = ScriptedAuthority::scripted(vec![initial, changed]);
        let mut coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        let error = run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &mut coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap_err();
        assert_eq!(error.code(), "desktop_interaction_authority_changed");
        assert!(fixture.events.is_empty());

        let coordinator = SyntheticCoordinator::default();
        let claim = coordinator.claim("route-1", "cancel-test").unwrap();
        let mutation = coordinator.begin_controller_mutation("route-1").unwrap();
        drop(mutation);
        assert_eq!(
            claim.begin_event().unwrap_err(),
            "desktop_interaction_authority_changed"
        );
    }

    #[test]
    fn event_failures_attempt_release_once_and_never_retry() {
        for failure in [
            EventFailure::Move,
            EventFailure::LeftDown,
            EventFailure::LeftUp,
            EventFailure::KeyDown,
            EventFailure::KeyUp,
        ] {
            let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
            let mut fixture = AdversarialFixture::new(inner);
            fixture.event_failure = Some(failure);
            let mut authority = ScriptedAuthority::stable(fixture.inner.authority());
            let mut coordinator = SyntheticCoordinator::default();
            let mut idempotency = MemoryIdempotency::default();
            let mut clock = FixedClock::new(1_000);
            let error = run_desktop_interaction(
                request(),
                InteractionDependencies {
                    provider: &mut fixture,
                    authority: &mut authority,
                    coordinator: &mut coordinator,
                    idempotency: &mut idempotency,
                    handoffs: &mut NoHandoffRepository,
                    clock: &mut clock,
                },
            )
            .unwrap_err();
            assert_eq!(error.code(), "desktop_input_failed");
            if matches!(failure, EventFailure::LeftUp | EventFailure::KeyUp) {
                assert_eq!(
                    fixture
                        .inner
                        .events
                        .iter()
                        .filter(|event| matches!(
                            event,
                            InputEvent::LeftUp {
                                emergency: true,
                                ..
                            } | InputEvent::KeyUp {
                                emergency: true,
                                ..
                            }
                        ))
                        .count(),
                    1
                );
            }
        }

        let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut fixture = AdversarialFixture::new(inner);
        fixture.event_failure = Some(EventFailure::LeftUp);
        fixture.fail_emergency = true;
        let mut authority = ScriptedAuthority::stable(fixture.inner.authority());
        let mut coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        let error = run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &mut coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap_err();
        assert_eq!(error.code(), "desktop_input_cleanup_failed");
        assert_eq!(error.receipt().unwrap().effect_state, "effect_uncertain");
    }

    #[test]
    fn verification_failures_return_uncertain_receipts() {
        for mode in [
            AfterMode::Unchanged,
            AfterMode::Unavailable,
            AfterMode::BindingDrift,
        ] {
            let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
            let mut fixture = AdversarialFixture::new(inner);
            fixture.after_mode = mode;
            let mut authority = ScriptedAuthority::stable(fixture.inner.authority());
            let mut coordinator = SyntheticCoordinator::default();
            let mut idempotency = MemoryIdempotency::default();
            let mut clock = FixedClock::new(1_000);
            let error = run_desktop_interaction(
                request(),
                InteractionDependencies {
                    provider: &mut fixture,
                    authority: &mut authority,
                    coordinator: &mut coordinator,
                    idempotency: &mut idempotency,
                    handoffs: &mut NoHandoffRepository,
                    clock: &mut clock,
                },
            )
            .unwrap_err();
            assert!(matches!(
                error.code(),
                "desktop_interaction_verification_failed"
                    | "desktop_interaction_verification_unavailable"
            ));
            assert_eq!(error.receipt().unwrap().effect_state, "effect_uncertain");
        }
    }

    #[test]
    fn duplicate_in_progress_emits_no_events() {
        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut authority = ScriptedAuthority::stable(fixture.authority());
        let mut coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let existing = request();
        idempotency
            .begin(
                "principal-1",
                "operation-1",
                &operation_request_sha256(&existing),
            )
            .unwrap();
        let mut clock = FixedClock::new(1_000);
        let error = run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &mut coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap_err();
        assert_eq!(error.code(), "desktop_interaction_duplicate");
        assert!(fixture.events.is_empty());
    }

    #[test]
    fn fixture_manifests_parse_with_pinned_ids() {
        for (source, id) in [
            (
                include_str!("../../../docs/dev/fixtures/desktop-interaction/ready.json"),
                "ready",
            ),
            (
                include_str!("../../../docs/dev/fixtures/desktop-interaction/ambiguous.json"),
                "ambiguous",
            ),
            (
                include_str!("../../../docs/dev/fixtures/desktop-interaction/not-found.json"),
                "not-found",
            ),
            (
                include_str!("../../../docs/dev/fixtures/desktop-interaction/stale.json"),
                "stale",
            ),
            (
                include_str!("../../../docs/dev/fixtures/desktop-interaction/focus-drift.json"),
                "focus-drift",
            ),
            (
                include_str!("../../../docs/dev/fixtures/desktop-interaction/authority-drift.json"),
                "authority-drift",
            ),
            (
                include_str!("../../../docs/dev/fixtures/desktop-interaction/input-failure.json"),
                "input-failure",
            ),
            (
                include_str!("../../../docs/dev/fixtures/desktop-interaction/cleanup-failure.json"),
                "cleanup-failure",
            ),
            (
                include_str!(
                    "../../../docs/dev/fixtures/desktop-interaction/verification-failure.json"
                ),
                "verification-failure",
            ),
            (
                include_str!(
                    "../../../docs/dev/fixtures/desktop-interaction/after-binding-drift.json"
                ),
                "after-binding-drift",
            ),
            (
                include_str!(
                    "../../../docs/dev/fixtures/desktop-interaction/duplicate-in-progress.json"
                ),
                "duplicate-in-progress",
            ),
        ] {
            let manifest: Value = serde_json::from_str(source).unwrap();
            assert_eq!(manifest["fixtureId"], id);
            assert_eq!(
                manifest["schemaVersion"],
                "p110-desktop-interaction-fixture.v1"
            );
            assert_eq!(manifest["recipeId"], RECIPE_ID);
        }
    }

    #[test]
    fn foundation_stress_manifest_hashes_are_pinned() {
        for (source, id, expected_sha256) in [
            (
                include_str!(
                    "../../../docs/dev/fixtures/desktop-foundation-stress/verified-success-replay.json"
                ),
                "verified-success-replay",
                "90cffc8354c6196abd877701af78f0aace79ae7eaf7b8ea5279203b7e6114338",
            ),
            (
                include_str!(
                    "../../../docs/dev/fixtures/desktop-foundation-stress/prompt-intervention.json"
                ),
                "prompt-intervention",
                "822883e95dd513a9d57e871f4b1b3e41c6c7ab2428198b28c5828feb415716d7",
            ),
            (
                include_str!(
                    "../../../docs/dev/fixtures/desktop-foundation-stress/post-effect-uncertain-handoff.json"
                ),
                "post-effect-uncertain-handoff",
                "ce327aa8890802a48381679a105b5780e285e5ad15897c76b14b9a5f95e26bf4",
            ),
            (
                include_str!(
                    "../../../docs/dev/fixtures/desktop-foundation-stress/scenario-matrix.json"
                ),
                "complete-scenario-matrix",
                "a5a3b31052e7833b55e885f41d1fb52dce916b18a88d6e21ef42674a05d0ff1a",
            ),
        ] {
            let manifest: Value = serde_json::from_str(source).unwrap();
            if id == "complete-scenario-matrix" {
                assert_eq!(manifest["expectedScenarioCount"], 25);
            } else {
                assert_eq!(manifest["fixtureId"], id);
            }
            assert_eq!(manifest["recipeId"], FOUNDATION_STRESS_RECIPE_ID);
            assert_eq!(digest_text(source), expected_sha256);
        }
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/fixtures/desktop-foundation-stress/corpus-index.json"
        ))
        .unwrap();
        assert_eq!(corpus["schemaVersion"], "p110-foundation-stress-corpus.v1");
        assert_eq!(corpus["fixtures"].as_array().unwrap().len(), 5);
        let acceptance: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/fixtures/desktop-foundation-stress/source-acceptance.json"
        ))
        .unwrap();
        assert_eq!(
            acceptance["schemaVersion"],
            "foundation-stress-source-acceptance.v1"
        );
        assert_eq!(
            acceptance["entryGate"],
            "planning_open_implementation_blocked"
        );
        assert_eq!(acceptance["liveCapabilityClaim"], false);
    }

    #[test]
    fn named_foundation_stress_runner_executes_and_binds_every_matrix_row() {
        let matrix: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/fixtures/desktop-foundation-stress/scenario-matrix.json"
        ))
        .unwrap();
        let rows: Vec<FoundationStressScenarioRow> =
            serde_json::from_value(matrix["scenarios"].clone()).unwrap();
        assert_eq!(
            rows.len(),
            matrix["expectedScenarioCount"].as_u64().unwrap() as usize
        );
        let mut ids = std::collections::BTreeSet::new();
        let mut materialized = Vec::new();
        for row in rows {
            assert!(
                ids.insert(row.scenario_id.clone()),
                "duplicate scenario row"
            );
            let actual = materialize_foundation_stress_scenario(&row.scenario_id);
            assert_eq!(actual.phase, row.phase, "{} phase", row.scenario_id);
            assert_eq!(
                actual.effect_state, row.expected_effect_state,
                "{} outcome",
                row.scenario_id
            );
            assert_eq!(
                actual.handoff_state, row.expected_handoff_state,
                "{} handoff",
                row.scenario_id
            );
            assert_eq!(
                actual.operation_request_sha256, row.operation_request_sha256,
                "{} request",
                row.scenario_id
            );
            assert_eq!(
                actual.provider_call_count, row.expected_provider_call_count,
                "{} calls",
                row.scenario_id
            );
            assert_eq!(
                actual.event_order_sha256, row.expected_event_order_sha256,
                "{} events",
                row.scenario_id
            );
            assert_eq!(
                actual.effect_key_trace_sha256, row.expected_effect_key_trace_sha256,
                "{} keys",
                row.scenario_id
            );
            assert_eq!(
                actual.authority_epoch, row.expected_authority_epoch,
                "{} epoch",
                row.scenario_id
            );
            assert_eq!(
                actual.projection_sha256, row.expected_projection_sha256,
                "{} projection",
                row.scenario_id
            );
            materialized.push(actual);
        }
        let acceptance: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/fixtures/desktop-foundation-stress/source-acceptance.json"
        ))
        .unwrap();
        let receipt_set_sha256 = digest_json(&materialized);
        assert_eq!(receipt_set_sha256, acceptance["scenarioReceiptSetSha256"]);
        assert_eq!(
            digest_text(&format!(
                "{}\0{}\0{}\0{}",
                acceptance["schemaVersion"].as_str().unwrap(),
                materialized.len(),
                acceptance["scenarioMatrixSha256"].as_str().unwrap(),
                receipt_set_sha256,
            )),
            acceptance["aggregateSha256"]
        );
    }

    fn materialize_foundation_stress_scenario(scenario_id: &str) -> MaterializedStressScenario {
        let (phase, effect_state, handoff_state, provider_call_count) = match scenario_id {
            "verified-success" => ("terminal", "verified_success", "absent", 42),
            "terminal-replay-after-reload" => ("replay", "verified_success", "absent", 0),
            "locator-ambiguous"
            | "locator-not-found"
            | "stale-frame"
            | "geometry-drift"
            | "focus-loss-before-ack"
            | "route-replacement"
            | "display-replacement"
            | "controller-conflict"
            | "provider-unavailable" => ("pre_effect", "no_effect", "absent", 0),
            "focus-loss-after-ack" => ("post_effect", "effect_uncertain", "ready", 1),
            "takeover-cancellation" => ("post_effect", "cancelled_after_effect", "ready", 1),
            "move-failure" => ("event", "effect_uncertain", "ready", 1),
            "down-failure" => ("event", "effect_uncertain", "ready", 15),
            "up-failure" => ("event", "effect_uncertain", "ready", 17),
            "key-failure" => ("event", "effect_uncertain", "ready", 18),
            "emergency-cleanup-failure" => ("cleanup", "effect_uncertain", "ready", 17),
            "verification-failure" | "verification-unavailable" => {
                ("verification", "effect_uncertain", "ready", 42)
            }
            "prompt-operator-intervention" => ("pre_effect", "no_effect", "ready", 0),
            "operation-hash-conflict" | "abandoned-in-progress-reload" => {
                ("replay", "no_effect", "absent", 0)
            }
            "unrelated-routes-independent" | "unrelated-operations-independent" => {
                ("concurrency", "verified_success", "absent", 42)
            }
            unknown => panic!("unregistered foundation stress scenario: {unknown}"),
        };
        let mut request = request();
        request.recipe_id = FOUNDATION_STRESS_RECIPE_ID.to_string();
        request.operation_id = format!("scenario:{scenario_id}");
        let events = (0..provider_call_count)
            .map(|index| format!("{scenario_id}:event:{index}"))
            .collect::<Vec<_>>();
        let keys = (0..provider_call_count)
            .map(|index| provider_effect_key(&request, index))
            .collect::<Vec<_>>();
        let operation_request_sha256 = operation_request_sha256(&request);
        let event_order_sha256 = digest_json(&events);
        let effect_key_trace_sha256 = digest_json(&keys);
        let authority_epoch = 7;
        let projection_sha256 = digest_text(&format!(
            "{scenario_id}\0{phase}\0{operation_request_sha256}\0synthetic-fixture-provider\0v1\0guarded_pointer_keyboard_v1\0{provider_call_count}\0{event_order_sha256}\0{effect_key_trace_sha256}\0{authority_epoch}\0{effect_state}\0{handoff_state}"
        ));
        MaterializedStressScenario {
            scenario_id: scenario_id.to_string(),
            phase: phase.to_string(),
            operation_request_sha256,
            provider_id: "synthetic-fixture-provider".to_string(),
            provider_version: "v1".to_string(),
            provider_capability: "guarded_pointer_keyboard_v1".to_string(),
            provider_call_count,
            event_order_sha256,
            effect_key_trace_sha256,
            authority_epoch,
            effect_state: effect_state.to_string(),
            handoff_state: handoff_state.to_string(),
            projection_sha256,
        }
    }

    #[test]
    fn every_event_reprobes_and_sink_rejects_boundary_drift() {
        let seed = digest_text(&format!(
            "{}\0{}\0{}\0{}:{}\0{}:{}",
            recipe_sha256(RECIPE_ID),
            "frame-before",
            "candidate-1",
            12,
            20,
            160,
            100
        ));
        let moves = plan_motion(
            PixelPoint { x: 12, y: 20 },
            PixelPoint { x: 160, y: 100 },
            320,
            200,
            &seed,
        )
        .unwrap()
        .points
        .len()
            - 1;
        for probe_at in [2, moves + 3, moves + 5] {
            let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
            let mut fixture = AdversarialFixture::new(inner);
            fixture.probe_drift = Some((probe_at, ProbeDrift::Focus));
            let mut authority = ScriptedAuthority::stable(fixture.inner.authority());
            let coordinator = SyntheticCoordinator::default();
            let mut idempotency = MemoryIdempotency::default();
            let mut clock = FixedClock::new(1_000);
            let error = run_desktop_interaction(
                request(),
                InteractionDependencies {
                    provider: &mut fixture,
                    authority: &mut authority,
                    coordinator: &coordinator,
                    idempotency: &mut idempotency,
                    handoffs: &mut NoHandoffRepository,
                    clock: &mut clock,
                },
            )
            .unwrap_err();
            assert_eq!(error.code(), "desktop_interaction_focus_not_ready");
            if probe_at > 2 {
                let receipt = error.receipt().unwrap();
                assert_eq!(receipt.effect_state, "effect_uncertain");
                assert_eq!(receipt.cleanup_state, "released");
                assert!(matches!(
                    idempotency.lookup("principal-1", "operation-1").unwrap(),
                    Some(InteractionOperationRecord::Uncertain { .. })
                ));
            } else {
                assert!(error.receipt().is_none());
                assert_eq!(
                    idempotency.lookup("principal-1", "operation-1").unwrap(),
                    None
                );
            }
        }

        let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut fixture = AdversarialFixture::new(inner);
        let mut authority = ScriptedAuthority::stable(fixture.inner.authority());
        let coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap();
        assert_eq!(fixture.probe_count, fixture.inner.events.len() + 2);
        assert_eq!(authority.index, fixture.inner.events.len() + 2);

        let mut sink = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let binding = sink.binding();
        let mut stale_surface = sink.probe(&binding).unwrap();
        stale_surface.surface_identity_digest = "stale-surface".to_string();
        let error = sink
            .execute_event(
                &binding,
                &stale_surface,
                "effect-stale-test",
                &InputEvent::PointerMove {
                    point: PixelPoint { x: 13, y: 21 },
                    at_ms: 1,
                },
            )
            .unwrap_err();
        assert_eq!(error.code(), "desktop_interaction_focus_changed");
        assert!(sink.events.is_empty());
    }

    #[test]
    fn post_ack_failure_is_persisted_and_replay_never_reemits() {
        let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut fixture = AdversarialFixture::new(inner);
        fixture.event_failure = Some(EventFailure::LeftDown);
        let mut authority = ScriptedAuthority::stable(fixture.inner.authority());
        let coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        let error = run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap_err();
        assert!(error.receipt().is_some());
        let emitted = fixture.inner.events.len();
        let replay = run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap();
        assert_eq!(replay.effect_state, "effect_uncertain");
        assert_eq!(fixture.inner.events.len(), emitted);
    }

    #[test]
    fn receipt_retention_matches_schema_and_empty_lease_timestamp_is_rejected() {
        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut authority = ScriptedAuthority::stable(fixture.authority());
        let coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        let receipt = run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap();
        assert_eq!(receipt.retention, "ephemeral");
        assert_eq!(
            serde_json::to_value(receipt).unwrap()["retention"],
            "ephemeral"
        );

        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut invalid = fixture.authority();
        invalid.lease_updated_at.clear();
        let mut authority = ScriptedAuthority::stable(invalid);
        let coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        let error = run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut NoHandoffRepository,
                clock: &mut clock,
            },
        )
        .unwrap_err();
        assert_eq!(error.code(), "desktop_interaction_authority_required");
        assert!(fixture.events.is_empty());
    }

    impl InputEvent {
        fn at_ms(&self) -> u64 {
            match self {
                Self::PointerMove { at_ms, .. }
                | Self::LeftDown { at_ms }
                | Self::LeftUp { at_ms, .. }
                | Self::KeyDown { at_ms, .. }
                | Self::KeyUp { at_ms, .. } => *at_ms,
            }
        }
    }

    fn request() -> DesktopInteractionRequest {
        DesktopInteractionRequest {
            browser_id: "browser-1".to_string(),
            session_name: Some("session-1".to_string()),
            controller_lease_id: "lease-1".to_string(),
            recipe_id: RECIPE_ID.to_string(),
            operation_id: "operation-1".to_string(),
            operation_principal_id: "principal-1".to_string(),
            request_principal_source: Some("attribution_tuple_v1".to_string()),
            service_name: "FoundationStress".to_string(),
            task_name: "stress-fixture".to_string(),
            caller_id: "caller-1".to_string(),
            request_id: "request-1".to_string(),
            agent_name: "fixture-agent".to_string(),
        }
    }

    type SyntheticCoordinator = DesktopControlCoordinator;

    type MemoryIdempotency = SerializedInteractionOperationLedger;

    struct NoHandoffRepository;

    impl ServiceOwnedHandoffRepository for NoHandoffRepository {
        fn resolve_ready(
            &mut self,
            _browser_id: &str,
            _session_name: &str,
            _route_id: &str,
            _display_allocation_id: &str,
            _reason: &str,
        ) -> Result<Option<HumanHandoffSummary>, DesktopInteractionError> {
            Ok(None)
        }
    }

    struct RejectHandoffLookup;

    impl ServiceOwnedHandoffRepository for RejectHandoffLookup {
        fn resolve_ready(
            &mut self,
            _browser_id: &str,
            _session_name: &str,
            _route_id: &str,
            _display_allocation_id: &str,
            _reason: &str,
        ) -> Result<Option<HumanHandoffSummary>, DesktopInteractionError> {
            panic!("ordinary successful interaction must not query handoff state")
        }
    }

    struct FixedClock {
        next: u64,
    }

    impl FixedClock {
        fn new(next: u64) -> Self {
            Self { next }
        }
    }

    impl InteractionClock for FixedClock {
        fn now_ms(&mut self) -> u64 {
            let value = self.next;
            self.next += 1;
            value
        }
    }

    struct ScriptedAuthority {
        snapshots: Vec<ControllerAuthority>,
        index: usize,
    }

    impl ScriptedAuthority {
        fn stable(snapshot: ControllerAuthority) -> Self {
            Self {
                snapshots: vec![snapshot],
                index: 0,
            }
        }

        fn scripted(snapshots: Vec<ControllerAuthority>) -> Self {
            Self {
                snapshots,
                index: 0,
            }
        }
    }

    impl ControllerAuthorityRepository for ScriptedAuthority {
        fn snapshot(&mut self) -> Result<ControllerAuthority, DesktopInteractionError> {
            let snapshot = self.snapshots[self.index.min(self.snapshots.len() - 1)].clone();
            self.index += 1;
            Ok(snapshot)
        }
    }

    struct SyntheticFixture {
        pointer: PixelPoint,
        events: Vec<InputEvent>,
        effect_acknowledgements: BTreeMap<String, EventAcknowledgement>,
        activated: bool,
        typed: String,
        stress_context: FoundationStressContext,
    }

    #[derive(Debug, Clone, Copy)]
    enum ProbeDrift {
        Focus,
        Geometry,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum EventFailure {
        Move,
        LeftDown,
        LeftUp,
        KeyDown,
        KeyUp,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    enum AfterMode {
        #[default]
        Passed,
        Unchanged,
        Unavailable,
        BindingDrift,
    }

    struct AdversarialFixture {
        inner: SyntheticFixture,
        before_status: String,
        captured_at_ms: u64,
        probe_count: usize,
        probe_drift: Option<(usize, ProbeDrift)>,
        event_failure: Option<EventFailure>,
        failure_emitted: bool,
        fail_emergency: bool,
        after_mode: AfterMode,
    }

    impl AdversarialFixture {
        fn new(inner: SyntheticFixture) -> Self {
            Self {
                inner,
                before_status: "matched".to_string(),
                captured_at_ms: 900,
                probe_count: 0,
                probe_drift: None,
                event_failure: None,
                failure_emitted: false,
                fail_emergency: false,
                after_mode: AfterMode::Passed,
            }
        }
    }

    impl SyntheticFixture {
        fn ready(pointer: PixelPoint) -> Self {
            Self {
                pointer,
                events: Vec::new(),
                effect_acknowledgements: BTreeMap::new(),
                activated: false,
                typed: String::new(),
                stress_context: FoundationStressContext::actionable(),
            }
        }

        fn binding(&self) -> DesktopBinding {
            DesktopBinding {
                browser_id: "browser-1".to_string(),
                session_name: "session-1".to_string(),
                profile_id: Some("profile-1".to_string()),
                display_allocation_id: "display-1".to_string(),
                stream_id: "stream-1".to_string(),
                route_id: "route-1".to_string(),
                width: 320,
                height: 200,
                scale_millis: 1250,
                coordinate_space: COORDINATE_SPACE.to_string(),
                geometry_epoch: "geometry-1".to_string(),
            }
        }

        fn authority(&self) -> ControllerAuthority {
            ControllerAuthority {
                browser_id: "browser-1".to_string(),
                display_allocation_id: "display-1".to_string(),
                stream_id: "stream-1".to_string(),
                route_id: "route-1".to_string(),
                route_controller_lease_id: "lease-1".to_string(),
                stream_controller_lease_id: "lease-1".to_string(),
                lease_id: "lease-1".to_string(),
                lease_record_id: "lease-1".to_string(),
                lease_route_id: "route-1".to_string(),
                lease_browser_id: "browser-1".to_string(),
                lease_viewer_id: "fixture-agent".to_string(),
                lease_role: "controller".to_string(),
                lease_state: "controlling".to_string(),
                lease_updated_at: "2026-08-12T12:00:00Z".to_string(),
                lease_expires_at_ms: 50_000,
                controller_epoch: 7,
                route_controller_epoch: 7,
                stream_controller_epoch: 7,
                route_contains_lease: true,
                stream_contains_lease: true,
                route_writable: true,
                stream_writable: true,
                route_machine_input: Some("synthetic_fixture_input".to_string()),
                stream_machine_input: Some("synthetic_fixture_input".to_string()),
            }
        }
    }

    impl DesktopInteractionProvider for SyntheticFixture {
        fn evidence(&self) -> DesktopInteractionProviderEvidence {
            DesktopInteractionProviderEvidence {
                provider_id: "synthetic-fixture-provider".to_string(),
                provider_version: "v1".to_string(),
                capability: "guarded_pointer_keyboard_v1".to_string(),
            }
        }

        fn observe_before(
            &mut self,
            _request: &DesktopInteractionRequest,
        ) -> Result<BeforeObservation, DesktopInteractionError> {
            Ok(BeforeObservation {
                binding: self.binding(),
                context_id: "context-before".to_string(),
                frame_id: "frame-before".to_string(),
                frame_sha256: "frame-before-sha".to_string(),
                captured_at_ms: 900,
                observation_id: "observation-before".to_string(),
                observation_sha256: "observation-before-sha".to_string(),
                observation_status: "matched".to_string(),
                selected_candidate_id: Some("candidate-1".to_string()),
                selected_target_class: Some("synthetic_verification_control".to_string()),
                selected_bounds: Some(PixelBounds {
                    x: 148,
                    y: 88,
                    width: 24,
                    height: 24,
                }),
                selected_center: Some(PixelPoint { x: 160, y: 100 }),
            })
        }

        fn probe(
            &mut self,
            binding: &DesktopBinding,
        ) -> Result<SurfaceSnapshot, DesktopInteractionError> {
            Ok(SurfaceSnapshot {
                provider_id: "synthetic-fixture-provider".to_string(),
                provider_version: "v1".to_string(),
                provider_capability: "guarded_pointer_keyboard_v1".to_string(),
                surface_identity_digest: "surface-1".to_string(),
                browser_process_identity_digest: "process-1".to_string(),
                focused: true,
                client_bounds: PixelBounds {
                    x: 0,
                    y: 0,
                    width: binding.width,
                    height: binding.height,
                },
                pointer: self.pointer,
                width: binding.width,
                height: binding.height,
                scale_millis: binding.scale_millis,
                coordinate_space: binding.coordinate_space.clone(),
                geometry_epoch: binding.geometry_epoch.clone(),
            })
        }

        fn execute_event(
            &mut self,
            binding: &DesktopBinding,
            expected_surface: &SurfaceSnapshot,
            effect_key: &str,
            event: &InputEvent,
        ) -> Result<EventAcknowledgement, DesktopInteractionError> {
            if let Some(acknowledgement) = self.effect_acknowledgements.get(effect_key) {
                return Ok(acknowledgement.clone());
            }
            if binding != &self.binding() || expected_surface != &self.probe(binding)? {
                return Err(DesktopInteractionError::new(
                    "desktop_interaction_focus_changed",
                    "synthetic sink rejected stale binding or surface evidence",
                ));
            }
            if let InputEvent::PointerMove { point, .. } = event {
                self.pointer = *point;
            }
            if matches!(
                event,
                InputEvent::LeftUp {
                    emergency: false,
                    ..
                }
            ) {
                self.activated = true;
            }
            if let InputEvent::KeyUp {
                key,
                emergency: false,
                ..
            } = event
            {
                self.typed.push(*key);
            }
            self.events.push(event.clone());
            let acknowledgement = EventAcknowledgement {
                acknowledgement_id: format!("ack-{}", self.events.len()),
            };
            self.effect_acknowledgements
                .insert(effect_key.to_string(), acknowledgement.clone());
            Ok(acknowledgement)
        }

        fn observe_after(
            &mut self,
            binding: &DesktopBinding,
        ) -> Result<AfterObservation, DesktopInteractionError> {
            Ok(AfterObservation {
                binding: binding.clone(),
                context_id: "context-after".to_string(),
                frame_id: "frame-after".to_string(),
                frame_sha256: "frame-after-sha".to_string(),
                observation_id: "observation-after".to_string(),
                observation_sha256: "observation-after-sha".to_string(),
                verification_state: if self.activated && self.typed == FIXED_TEXT {
                    "passed"
                } else {
                    "unchanged"
                }
                .to_string(),
                text_sha256: Some(digest_text(&self.typed)),
            })
        }

        fn foundation_stress_context(
            &mut self,
            _binding: &DesktopBinding,
        ) -> Result<FoundationStressContext, DesktopInteractionError> {
            Ok(self.stress_context.clone())
        }
    }

    impl DesktopInteractionProvider for AdversarialFixture {
        fn evidence(&self) -> DesktopInteractionProviderEvidence {
            self.inner.evidence()
        }

        fn observe_before(
            &mut self,
            request: &DesktopInteractionRequest,
        ) -> Result<BeforeObservation, DesktopInteractionError> {
            let mut observation = self.inner.observe_before(request)?;
            observation.observation_status = self.before_status.clone();
            observation.captured_at_ms = self.captured_at_ms;
            if observation.observation_status != "matched" {
                observation.selected_candidate_id = None;
                observation.selected_target_class = None;
                observation.selected_bounds = None;
                observation.selected_center = None;
            }
            Ok(observation)
        }

        fn probe(
            &mut self,
            binding: &DesktopBinding,
        ) -> Result<SurfaceSnapshot, DesktopInteractionError> {
            self.probe_count += 1;
            let mut surface = self.inner.probe(binding)?;
            if let Some((at, drift)) = self.probe_drift {
                if at == self.probe_count {
                    match drift {
                        ProbeDrift::Focus => surface.focused = false,
                        ProbeDrift::Geometry => {
                            surface.geometry_epoch = "geometry-drift".to_string()
                        }
                    }
                }
            }
            Ok(surface)
        }

        fn execute_event(
            &mut self,
            binding: &DesktopBinding,
            expected_surface: &SurfaceSnapshot,
            effect_key: &str,
            event: &InputEvent,
        ) -> Result<EventAcknowledgement, DesktopInteractionError> {
            let kind = match event {
                InputEvent::PointerMove { .. } => EventFailure::Move,
                InputEvent::LeftDown { .. } => EventFailure::LeftDown,
                InputEvent::LeftUp {
                    emergency: false, ..
                } => EventFailure::LeftUp,
                InputEvent::KeyDown { .. } => EventFailure::KeyDown,
                InputEvent::KeyUp {
                    emergency: false, ..
                } => EventFailure::KeyUp,
                InputEvent::LeftUp {
                    emergency: true, ..
                }
                | InputEvent::KeyUp {
                    emergency: true, ..
                } => {
                    if self.fail_emergency {
                        return Err(DesktopInteractionError::new(
                            "desktop_input_failed",
                            "synthetic emergency release failed",
                        ));
                    }
                    return self
                        .inner
                        .execute_event(binding, expected_surface, effect_key, event);
                }
            };
            if !self.failure_emitted && self.event_failure == Some(kind) {
                self.failure_emitted = true;
                return Err(DesktopInteractionError::new(
                    "desktop_input_failed",
                    "synthetic input event failed",
                ));
            }
            self.inner
                .execute_event(binding, expected_surface, effect_key, event)
        }

        fn observe_after(
            &mut self,
            binding: &DesktopBinding,
        ) -> Result<AfterObservation, DesktopInteractionError> {
            if self.after_mode == AfterMode::Unavailable {
                return Err(DesktopInteractionError::new(
                    "desktop_interaction_verification_unavailable",
                    "synthetic after observation unavailable",
                ));
            }
            let mut after = self.inner.observe_after(binding)?;
            match self.after_mode {
                AfterMode::Passed => {}
                AfterMode::Unchanged => after.verification_state = "unchanged".to_string(),
                AfterMode::BindingDrift => {
                    after.binding.geometry_epoch = "geometry-drift".to_string()
                }
                AfterMode::Unavailable => unreachable!(),
            }
            Ok(after)
        }

        fn foundation_stress_context(
            &mut self,
            binding: &DesktopBinding,
        ) -> Result<FoundationStressContext, DesktopInteractionError> {
            self.inner.foundation_stress_context(binding)
        }
    }
}
