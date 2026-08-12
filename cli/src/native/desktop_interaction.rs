//! Guarded, provider-neutral desktop interaction engine.
//!
//! This module owns the deterministic recipe, motion planner, event cleanup,
//! verification, and redacted receipt. Platform input and controller storage
//! remain injected seams. The source proof supplies only an in-memory fixture
//! adapter and never invokes an operating-system input facility.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::desktop_control_coordinator::{DesktopControlCoordinator, DesktopInteractionClaim};

pub(crate) const RECIPE_ID: &str = "p110-pointer-keyboard-v1";
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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

pub(crate) trait DesktopInteractionProvider {
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
        event: &InputEvent,
    ) -> Result<EventAcknowledgement, DesktopInteractionError>;
    fn observe_after(
        &mut self,
        binding: &DesktopBinding,
    ) -> Result<AfterObservation, DesktopInteractionError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IdempotencyRecord {
    InProgress,
    Complete(Box<InteractionReceipt>),
}

pub(crate) trait InteractionIdempotencyStore {
    fn lookup(&mut self, caller_id: &str, request_id: &str) -> Option<IdempotencyRecord>;
    fn begin(&mut self, caller_id: &str, request_id: &str, transaction_id: &str);
    fn complete(&mut self, caller_id: &str, request_id: &str, receipt: &InteractionReceipt);
    fn abort(&mut self, caller_id: &str, request_id: &str);
}

pub(crate) struct InteractionDependencies<'a> {
    pub provider: &'a mut dyn DesktopInteractionProvider,
    pub authority: &'a mut dyn ControllerAuthorityRepository,
    pub coordinator: &'a DesktopControlCoordinator,
    pub idempotency: &'a mut dyn InteractionIdempotencyStore,
    pub clock: &'a mut dyn InteractionClock,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InteractionReceipt {
    pub transaction_id: String,
    pub schema_version: &'static str,
    pub recipe_id: &'static str,
    pub recipe_version: &'static str,
    pub recipe_sha256: String,
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
    pub coordinate_mapping: &'static str,
    pub motion_profile: &'static str,
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
    pub retention: &'static str,
    pub persisted_pixels: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopInteractionError {
    code: &'static str,
    message: &'static str,
    receipt: Option<Box<InteractionReceipt>>,
}

impl DesktopInteractionError {
    fn new(code: &'static str, message: &'static str) -> Self {
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
    match dependencies
        .idempotency
        .lookup(&request.caller_id, &request.request_id)
    {
        Some(IdempotencyRecord::Complete(receipt)) => return Ok(*receipt),
        Some(IdempotencyRecord::InProgress) => {
            return Err(DesktopInteractionError::new(
                "desktop_interaction_duplicate",
                "the interaction request is already in progress",
            ));
        }
        None => {}
    }

    let transaction_id = format!(
        "desktop-interaction-{}",
        &digest_text(&format!("{}\0{}", request.caller_id, request.request_id))[..24]
    );
    let before = dependencies.provider.observe_before(&request)?;
    validate_before(&request, &before)?;
    let candidate_id = before
        .selected_candidate_id
        .clone()
        .expect("validated selected candidate");
    let target = before.selected_center.expect("validated target center");
    let target_bounds = before.selected_bounds.expect("validated target bounds");
    let initial_authority = dependencies.authority.snapshot()?;
    let initial_now = dependencies.clock.now_ms();
    let authority_digest =
        validate_authority(&request, &before.binding, &initial_authority, initial_now)?;
    let claim = dependencies
        .coordinator
        .claim(&before.binding.route_id, &transaction_id)
        .map_err(|_| {
            DesktopInteractionError::new(
                "desktop_interaction_conflict",
                "the route already has an interaction claim",
            )
        })?;
    dependencies
        .idempotency
        .begin(&request.caller_id, &request.request_id, &transaction_id);

    let result = run_claimed_interaction(
        &request,
        &transaction_id,
        before,
        candidate_id,
        target,
        target_bounds,
        initial_authority,
        authority_digest,
        &claim,
        &mut dependencies,
    );
    drop(claim);
    match result.outcome {
        Ok(receipt) => {
            dependencies
                .idempotency
                .complete(&request.caller_id, &request.request_id, &receipt);
            Ok(receipt)
        }
        Err(error) => {
            if let Some(receipt) = error.receipt.as_deref() {
                dependencies
                    .idempotency
                    .complete(&request.caller_id, &request.request_id, receipt);
            } else {
                dependencies
                    .idempotency
                    .abort(&request.caller_id, &request.request_id);
            }
            Err(error)
        }
    }
}

/// Public production dispatch resolves no input provider in PoC 3. This must
/// fail before capture, authority lookup, controller mutation, or input.
pub(crate) async fn handle_desktop_interact(_command: &Value) -> Result<Value, String> {
    Err(
        "desktop_input_provider_unavailable: no production desktop input provider is configured"
            .to_string(),
    )
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
            let safe = receipt
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
                    } else if value.is_object() || value.is_array() {
                        Value::Null
                    } else {
                        value.clone()
                    };
                    (field.clone(), value)
                })
                .collect();
            redacted.insert((*key).to_string(), Value::Object(safe));
        } else if !value.is_object() && !value.is_array() {
            redacted.insert((*key).to_string(), value.clone());
        }
    }
    Value::Object(redacted)
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
    claim: &DesktopInteractionClaim,
    dependencies: &mut InteractionDependencies<'_>,
) -> ClaimedResult {
    let outcome = (|| {
        let initial_surface = dependencies.provider.probe(&before.binding)?;
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
        let recipe_sha256 = recipe_sha256();
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
        let mut acknowledged_effect = false;
        let mut key_down: Option<char> = None;

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
                        provider: dependencies.provider,
                        authority: dependencies.authority,
                        binding: &before.binding,
                        surface: &initial_surface,
                        claim,
                        acknowledgements: &mut acknowledgements,
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
                            provider: dependencies.provider,
                            authority: dependencies.authority,
                            binding: &before.binding,
                            surface: &initial_surface,
                            claim,
                            acknowledgements: &mut acknowledgements,
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
    if request.recipe_id != RECIPE_ID
        || request.browser_id.trim().is_empty()
        || request.controller_lease_id.trim().is_empty()
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
    dependencies
        .provider
        .execute_event(binding, &current_surface, event)
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
        || surface.provider_id != "synthetic-fixture-v1"
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
    provider: &'a mut dyn DesktopInteractionProvider,
    authority: &'a mut dyn ControllerAuthorityRepository,
    binding: &'a DesktopBinding,
    surface: &'a SurfaceSnapshot,
    claim: &'a DesktopInteractionClaim,
    acknowledgements: &'a mut Vec<String>,
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
        provider,
        authority,
        binding,
        surface,
        claim,
        acknowledgements,
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
            match provider.execute_event(binding, &current_surface, &event) {
                Ok(ack) => {
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
        || current.provider_id != "synthetic-fixture-v1"
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
            provider,
            authority: authority_repository,
            binding: &before.binding,
            surface: cleanup_surface,
            claim,
            acknowledgements: &mut acknowledgements,
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
    acknowledgement_ids: Vec<String>,
) -> InteractionReceipt {
    InteractionReceipt {
        transaction_id: transaction_id.to_string(),
        schema_version: "v1",
        recipe_id: RECIPE_ID,
        recipe_version: RECIPE_VERSION,
        recipe_sha256: recipe_sha256(),
        browser_id: before.binding.browser_id.clone(),
        display_allocation_id: before.binding.display_allocation_id.clone(),
        stream_id: before.binding.stream_id.clone(),
        route_id: before.binding.route_id.clone(),
        controller_epoch: authority.controller_epoch,
        authority_digest: authority_digest.to_string(),
        actor_digest: digest_text(&format!("{}\0{}", request.caller_id, request.agent_name)),
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
        coordinate_mapping: "identity_physical_pixels_v1",
        motion_profile: "fixed_cubic_bezier_v1",
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
        retention: "ephemeral",
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
        request.caller_id,
        request.request_id,
        authority.route_machine_input.as_deref().unwrap_or("")
    ))
}

fn recipe_sha256() -> String {
    digest_text(&format!(
        "{RECIPE_ID}\0{RECIPE_VERSION}\0p110-control-v1\0{FIXED_TEXT}\0fixed_cubic_bezier_v1"
    ))
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
                clock: &mut clock,
            },
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(fixture.events.len(), emitted);
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
        idempotency.begin("caller-1", "request-1", "transaction-existing");
        let mut clock = FixedClock::new(1_000);
        let error = run_desktop_interaction(
            request(),
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &mut coordinator,
                idempotency: &mut idempotency,
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
    fn every_event_reprobes_and_sink_rejects_boundary_drift() {
        let seed = digest_text(&format!(
            "{}\0{}\0{}\0{}:{}\0{}:{}",
            recipe_sha256(),
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
                    idempotency.lookup("caller-1", "request-1"),
                    Some(IdempotencyRecord::Complete(_))
                ));
            } else {
                assert!(error.receipt().is_none());
                assert_eq!(idempotency.lookup("caller-1", "request-1"), None);
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
            caller_id: "caller-1".to_string(),
            request_id: "request-1".to_string(),
            agent_name: "fixture-agent".to_string(),
        }
    }

    type SyntheticCoordinator = DesktopControlCoordinator;

    #[derive(Default)]
    struct MemoryIdempotency(BTreeMap<(String, String), IdempotencyRecord>);

    impl InteractionIdempotencyStore for MemoryIdempotency {
        fn lookup(&mut self, caller_id: &str, request_id: &str) -> Option<IdempotencyRecord> {
            self.0
                .get(&(caller_id.to_string(), request_id.to_string()))
                .cloned()
        }

        fn begin(&mut self, caller_id: &str, request_id: &str, _transaction_id: &str) {
            self.0.insert(
                (caller_id.to_string(), request_id.to_string()),
                IdempotencyRecord::InProgress,
            );
        }

        fn complete(&mut self, caller_id: &str, request_id: &str, receipt: &InteractionReceipt) {
            self.0.insert(
                (caller_id.to_string(), request_id.to_string()),
                IdempotencyRecord::Complete(Box::new(receipt.clone())),
            );
        }

        fn abort(&mut self, caller_id: &str, request_id: &str) {
            self.0
                .remove(&(caller_id.to_string(), request_id.to_string()));
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
        activated: bool,
        typed: String,
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
                activated: false,
                typed: String::new(),
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
                provider_id: "synthetic-fixture-v1".to_string(),
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
            event: &InputEvent,
        ) -> Result<EventAcknowledgement, DesktopInteractionError> {
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
            Ok(EventAcknowledgement {
                acknowledgement_id: format!("ack-{}", self.events.len()),
            })
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
    }

    impl DesktopInteractionProvider for AdversarialFixture {
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
                    return self.inner.execute_event(binding, expected_surface, event);
                }
            };
            if !self.failure_emitted && self.event_failure == Some(kind) {
                self.failure_emitted = true;
                return Err(DesktopInteractionError::new(
                    "desktop_input_failed",
                    "synthetic input event failed",
                ));
            }
            self.inner.execute_event(binding, expected_surface, event)
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
    }
}
