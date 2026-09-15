//! CLI adapters for the provider-neutral desktop interaction services.
//!
//! Command dispatch, Service State handoff lookup, durable ledger persistence,
//! and stream redaction remain here. The transaction kernel is owned by the
//! `agent-browser-desktop-services` crate.

#[cfg(test)]
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::service_model::ServiceState;

pub(crate) use agent_browser_desktop_services::*;

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
        let previous = self.inner.clone();
        mutate(&mut self.inner)?;
        if let Err(error) = self.save() {
            self.inner = previous;
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

/// Dispatch the controlled provider only from an exact admitted immutable
/// generation. Development and production use separate manifest schemas and
/// state roots. Unmanifested binaries fail before capture, authority lookup,
/// controller mutation, or input. Raw provider routing is never accepted as a
/// compatibility contract.
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
    let admission = super::desktop_input_provider_admission::current_provider_admission()
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
    if ![
        CONTROLLED_X11_RECIPE_ID,
        TURNSTILE_RECIPE_ID,
        HCAPTCHA_RECIPE_ID,
    ]
    .contains(&recipe_id.as_str())
    {
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
    admission: super::desktop_input_provider_admission::ProviderAdmission,
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
    fn turnstile_recipe_moves_hovers_and_clicks_without_keyboard_input() {
        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut authority = ScriptedAuthority::stable(fixture.authority());
        let coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        let mut turnstile_request = request();
        turnstile_request.recipe_id = TURNSTILE_RECIPE_ID.to_string();

        let receipt = run_desktop_interaction(
            turnstile_request,
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut RejectHandoffLookup,
                clock: &mut clock,
            },
        )
        .expect("Turnstile fixture should verify after one click");

        assert_eq!(receipt.effect_state, "verified_success");
        assert_eq!(receipt.text_length, 0);
        assert!(fixture.activated);
        assert!(fixture.typed.is_empty());
        assert_eq!(
            fixture
                .events
                .iter()
                .filter(|event| matches!(event, InputEvent::LeftDown { .. }))
                .count(),
            1
        );
        assert!(!fixture
            .events
            .iter()
            .any(|event| matches!(event, InputEvent::KeyDown { .. } | InputEvent::KeyUp { .. })));
    }

    #[test]
    fn hcaptcha_recipe_emits_exactly_one_click_and_no_keyboard_input() {
        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut authority = ScriptedAuthority::stable(fixture.authority());
        let coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        let mut hcaptcha_request = request();
        hcaptcha_request.recipe_id = HCAPTCHA_RECIPE_ID.to_string();

        let receipt = run_desktop_interaction(
            hcaptcha_request,
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut RejectHandoffLookup,
                clock: &mut clock,
            },
        )
        .expect("hCaptcha fixture should verify after one click");

        assert_eq!(receipt.effect_state, "verified_success");
        assert_eq!(receipt.text_length, 0);
        assert_eq!(
            fixture
                .events
                .iter()
                .filter(|event| matches!(event, InputEvent::LeftDown { .. }))
                .count(),
            1
        );
        assert_eq!(
            fixture
                .events
                .iter()
                .filter(|event| matches!(event, InputEvent::LeftUp { .. }))
                .count(),
            1
        );
        assert!(!fixture
            .events
            .iter()
            .any(|event| matches!(event, InputEvent::KeyDown { .. } | InputEvent::KeyUp { .. })));
    }

    #[test]
    fn long_motion_refreshes_continuous_target_before_single_click() {
        let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut fixture = AdversarialFixture::new(inner);
        fixture.refresh_captured_at_ms = Some(2_500);
        let mut authority = ScriptedAuthority::stable(fixture.inner.authority());
        let coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = SteppingClock {
            next: 1_000,
            step: 100,
        };
        let mut hcaptcha_request = request();
        hcaptcha_request.recipe_id = HCAPTCHA_RECIPE_ID.to_string();

        let receipt = run_desktop_interaction(
            hcaptcha_request,
            InteractionDependencies {
                provider: &mut fixture,
                authority: &mut authority,
                coordinator: &coordinator,
                idempotency: &mut idempotency,
                handoffs: &mut RejectHandoffLookup,
                clock: &mut clock,
            },
        )
        .expect("a fresh continuous target should permit one click after long motion");

        assert_eq!(fixture.observation_count, 2);
        assert_eq!(receipt.effect_state, "verified_success");
        assert_eq!(
            fixture
                .inner
                .events
                .iter()
                .filter(|event| matches!(event, InputEvent::LeftDown { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn stale_effect_phase_refresh_stops_before_button_down_as_uncertain_motion() {
        let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut fixture = AdversarialFixture::new(inner);
        fixture.refresh_captured_at_ms = Some(0);
        let mut authority = ScriptedAuthority::stable(fixture.inner.authority());
        let coordinator = SyntheticCoordinator::default();
        let mut idempotency = MemoryIdempotency::default();
        let mut clock = FixedClock::new(1_000);
        let mut hcaptcha_request = request();
        hcaptcha_request.recipe_id = HCAPTCHA_RECIPE_ID.to_string();

        let error = run_desktop_interaction(
            hcaptcha_request,
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

        assert_eq!(fixture.observation_count, 2);
        assert_eq!(error.code(), "desktop_interaction_stale_observation");
        assert_eq!(error.receipt().unwrap().effect_state, "effect_uncertain");
        assert!(fixture
            .inner
            .events
            .iter()
            .all(|event| !matches!(event, InputEvent::LeftDown { .. })));
    }

    #[test]
    fn moved_target_at_effect_phase_stops_before_button_down() {
        let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut fixture = AdversarialFixture::new(inner);
        fixture.refresh_target_offset = Some(1);
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

        assert_eq!(error.code(), "desktop_interaction_target_changed");
        assert_eq!(error.receipt().unwrap().effect_state, "effect_uncertain");
        assert!(fixture
            .inner
            .events
            .iter()
            .all(|event| !matches!(event, InputEvent::LeftDown { .. })));
    }

    #[test]
    fn changed_effect_phase_geometry_stops_before_button_down() {
        let inner = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let mut fixture = AdversarialFixture::new(inner);
        fixture.refresh_geometry_epoch = Some("geometry-2".to_string());
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

        assert_eq!(error.code(), "desktop_interaction_target_changed");
        assert!(fixture
            .inner
            .events
            .iter()
            .all(|event| !matches!(event, InputEvent::LeftDown { .. })));
    }

    #[test]
    fn controller_change_at_effect_phase_stops_before_button_down() {
        let mut fixture = SyntheticFixture::ready(PixelPoint { x: 12, y: 20 });
        let initial = fixture.authority();
        let mut changed = initial.clone();
        changed.controller_epoch += 1;
        changed.route_controller_epoch += 1;
        changed.stream_controller_epoch += 1;
        let mut snapshots = vec![initial; 15];
        snapshots.push(changed);
        let mut authority = ScriptedAuthority::scripted(snapshots);
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

        assert_eq!(error.code(), "desktop_interaction_authority_changed");
        assert_eq!(
            error.receipt().unwrap().effect_state,
            "cancelled_after_effect"
        );
        assert!(fixture
            .events
            .iter()
            .all(|event| !matches!(event, InputEvent::LeftDown { .. })));
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
    async fn unmanifested_dispatch_fails_without_effect_resolution() {
        let error = handle_desktop_interact(&json!({ "action": "desktop_interact" }))
            .await
            .unwrap_err();
        assert!(error.starts_with("desktop_input_provider_generation_unavailable:"));
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
            AfterMode::ChallengeOpen,
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
            if mode == AfterMode::ChallengeOpen {
                assert_eq!(
                    error.receipt().unwrap().verification_state,
                    "challenge_open"
                );
            }
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

    struct SteppingClock {
        next: u64,
        step: u64,
    }

    impl InteractionClock for SteppingClock {
        fn now_ms(&mut self) -> u64 {
            let value = self.next;
            self.next += self.step;
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
        turnstile: bool,
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
        ChallengeOpen,
        Unavailable,
        BindingDrift,
    }

    struct AdversarialFixture {
        inner: SyntheticFixture,
        before_status: String,
        captured_at_ms: u64,
        refresh_captured_at_ms: Option<u64>,
        refresh_target_offset: Option<i64>,
        refresh_geometry_epoch: Option<String>,
        observation_count: usize,
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
                refresh_captured_at_ms: None,
                refresh_target_offset: None,
                refresh_geometry_epoch: None,
                observation_count: 0,
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
                turnstile: false,
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
            request: &DesktopInteractionRequest,
        ) -> Result<BeforeObservation, DesktopInteractionError> {
            self.turnstile = is_captcha_recipe(&request.recipe_id);
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
                selected_target_class: Some(
                    match request.recipe_id.as_str() {
                        TURNSTILE_RECIPE_ID => {
                            super::super::desktop_locator::TURNSTILE_TARGET_CLASS
                        }
                        HCAPTCHA_RECIPE_ID => super::super::desktop_locator::HCAPTCHA_TARGET_CLASS,
                        _ => "synthetic_verification_control",
                    }
                    .to_string(),
                ),
                selected_bounds: Some(PixelBounds {
                    x: 148,
                    y: 88,
                    width: 24,
                    height: 24,
                }),
                selected_center: Some(PixelPoint { x: 160, y: 100 }),
            })
        }

        fn refresh_before_effect(
            &mut self,
            request: &DesktopInteractionRequest,
        ) -> Result<BeforeObservation, DesktopInteractionError> {
            self.observe_before(request)
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
                verification_state: if self.activated
                    && (self.turnstile || self.typed == FIXED_TEXT)
                {
                    "passed"
                } else {
                    "unchanged"
                }
                .to_string(),
                text_sha256: (!self.turnstile).then(|| digest_text(&self.typed)),
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
            self.observation_count += 1;
            observation.observation_status = self.before_status.clone();
            observation.captured_at_ms = if self.observation_count > 1 {
                self.refresh_captured_at_ms.unwrap_or(self.captured_at_ms)
            } else {
                self.captured_at_ms
            };
            if self.observation_count > 1 {
                if let Some(offset) = self.refresh_target_offset {
                    observation.selected_bounds.as_mut().unwrap().x += offset;
                    observation.selected_center.as_mut().unwrap().x += offset;
                }
                if let Some(epoch) = &self.refresh_geometry_epoch {
                    observation.binding.geometry_epoch = epoch.clone();
                }
            }
            if observation.observation_status != "matched" {
                observation.selected_candidate_id = None;
                observation.selected_target_class = None;
                observation.selected_bounds = None;
                observation.selected_center = None;
            }
            Ok(observation)
        }

        fn refresh_before_effect(
            &mut self,
            request: &DesktopInteractionRequest,
        ) -> Result<BeforeObservation, DesktopInteractionError> {
            self.observe_before(request)
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
                AfterMode::ChallengeOpen => after.verification_state = "challenge_open".to_string(),
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
