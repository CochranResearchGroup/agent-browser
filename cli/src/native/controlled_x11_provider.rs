//! Development-only configured X11/XTEST implementation of the interaction seam.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::DateTime;
use sha2::{Digest, Sha256};

use super::desktop_capture::{
    capture_configured_desktop_frame, resolve_desktop_capture_binding_for_session,
    DesktopCaptureRequest, DesktopCaptureResult, HARD_MAX_BYTES,
};
use super::desktop_input_provider::{
    ControlledX11Event, DesktopInputProviderError, ProviderEffectExecutor, XTestSink,
};
use super::desktop_input_provider_admission::ProviderAdmission;
use super::desktop_interaction::{
    AfterObservation, BeforeObservation, ControllerAuthority, ControllerAuthorityRepository,
    DesktopBinding, DesktopInteractionError, DesktopInteractionProvider,
    DesktopInteractionProviderEvidence, DesktopInteractionRequest, EventAcknowledgement,
    InputEvent, InteractionClock, PixelBounds, PixelPoint, SurfaceSnapshot,
};
use super::service_model::{controller_authority_fence_matches, ServiceState};
use super::service_store::{
    default_service_state_path, LockedServiceStateRepository, ServiceStateRepository,
};

const TARGET_RGB: [u8; 3] = [32, 122, 214];
const SUCCESS_RGB: [u8; 3] = [46, 160, 67];
const FIXED_TEXT: &str = "fixture-ready";
const EFFECT_FENCE_DEADLINE: Duration = Duration::from_secs(5);

pub(crate) struct ControlledX11Provider {
    admission: ProviderAdmission,
    request: DesktopInteractionRequest,
    display_name: String,
    executor: ProviderEffectExecutor<XTestSink>,
    initial_capture: Option<DesktopCaptureResult>,
    controller_epoch: u64,
    process_identity_digest: String,
}

pub(crate) struct ConfiguredControllerAuthorityRepository {
    request: DesktopInteractionRequest,
    route_id: String,
    stream_id: String,
    display_allocation_id: String,
    machine_input: String,
}

pub(crate) struct SystemInteractionClock;

impl ControlledX11Provider {
    pub(crate) fn open(
        request: DesktopInteractionRequest,
        admission: ProviderAdmission,
    ) -> Result<(Self, ConfiguredControllerAuthorityRepository), DesktopInteractionError> {
        let repository = LockedServiceStateRepository::default_json()
            .map_err(|_| provider_error("desktop_input_provider_state_unavailable"))?;
        let state = repository
            .load_snapshot()
            .map_err(|_| provider_error("desktop_input_provider_state_unavailable"))?;
        let capture_binding = resolve_desktop_capture_binding_for_session(
            &state,
            &request.browser_id,
            request.session_name.as_deref(),
        )
        .map_err(|error| provider_error(error.code()))?;
        let browser = state
            .browsers
            .get(&request.browser_id)
            .ok_or_else(|| provider_error("desktop_input_provider_identity_unavailable"))?;
        let stream = browser
            .view_streams
            .iter()
            .find(|stream| {
                stream.id == capture_binding.route_id
                    || stream.route_id.as_deref() == Some(capture_binding.route_id.as_str())
            })
            .or_else(|| {
                browser.view_streams.iter().find(|stream| {
                    stream.route_id.as_deref() == Some(capture_binding.route_id.as_str())
                })
            })
            .ok_or_else(|| provider_error("desktop_input_provider_identity_unavailable"))?;
        let route = state
            .remote_view_routes
            .get(&capture_binding.route_id)
            .ok_or_else(|| provider_error("desktop_input_provider_identity_unavailable"))?;
        if route.controller_lease_id.as_deref() != Some(request.controller_lease_id.as_str())
            || stream.controller_lease_id.as_deref() != Some(request.controller_lease_id.as_str())
            || route.controller_epoch == 0
            || route.controller_epoch != stream.controller_epoch
        {
            return Err(provider_error("desktop_interaction_authority_required"));
        }
        let process_identity_digest = digest_serializable(&(
            state.browser_process_identities.get(&request.browser_id),
            browser.pid,
            &admission.generation_sha256,
        ))?;
        let state_path = default_service_state_path()
            .map_err(|_| provider_error("desktop_input_provider_state_unavailable"))?;
        let runtime_state_root = state_path
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| provider_error("desktop_input_provider_state_unavailable"))?
            .to_path_buf();
        let initial_capture = capture_configured_desktop_frame(DesktopCaptureRequest {
            browser_id: request.browser_id.clone(),
            session_name: request.session_name.clone(),
            max_bytes: HARD_MAX_BYTES,
        })
        .map_err(|error| provider_error(error.code()))?;
        let sink = XTestSink::new(&capture_binding.display_name)
            .map_err(|error| provider_error(error.code()))?;
        let executor = ProviderEffectExecutor::new(
            &runtime_state_root,
            &admission.runtime_environment,
            &capture_binding.route_id,
            &capture_binding.display_allocation_id,
            &admission.generation_sha256,
            sink,
        )
        .map_err(|error| provider_error(error.code()))?;
        let authority = ConfiguredControllerAuthorityRepository {
            request: request.clone(),
            route_id: capture_binding.route_id.clone(),
            stream_id: stream.id.clone(),
            display_allocation_id: capture_binding.display_allocation_id.clone(),
            machine_input: admission.provider_id.clone(),
        };
        Ok((
            Self {
                admission,
                request,
                display_name: capture_binding.display_name,
                executor,
                initial_capture: Some(initial_capture),
                controller_epoch: route.controller_epoch,
                process_identity_digest,
            },
            authority,
        ))
    }

    fn capture(&self) -> Result<DesktopCaptureResult, DesktopInteractionError> {
        capture_configured_desktop_frame(DesktopCaptureRequest {
            browser_id: self.request.browser_id.clone(),
            session_name: self.request.session_name.clone(),
            max_bytes: HARD_MAX_BYTES,
        })
        .map_err(|error| provider_error(error.code()))
    }

    fn binding(capture: &DesktopCaptureResult) -> DesktopBinding {
        DesktopBinding {
            browser_id: capture.context.browser_id.clone(),
            session_name: capture.context.session_name.clone(),
            profile_id: capture.context.profile_id.clone(),
            display_allocation_id: capture.context.display_allocation_id.clone(),
            stream_id: capture.context.stream_id.clone(),
            route_id: capture.context.route_id.clone(),
            width: capture.context.width,
            height: capture.context.height,
            scale_millis: (capture.context.scale_factor * 1000.0).round() as u32,
            coordinate_space: capture.context.coordinate_space.to_string(),
            geometry_epoch: capture.context.geometry_epoch.clone(),
        }
    }
}

impl DesktopInteractionProvider for ControlledX11Provider {
    fn evidence(&self) -> DesktopInteractionProviderEvidence {
        DesktopInteractionProviderEvidence {
            provider_id: self.admission.provider_id.clone(),
            provider_version: self.admission.generation_id.clone(),
            capability: self.admission.capability.clone(),
        }
    }

    fn observe_before(
        &mut self,
        _request: &DesktopInteractionRequest,
    ) -> Result<BeforeObservation, DesktopInteractionError> {
        let capture = self
            .initial_capture
            .take()
            .ok_or_else(|| provider_error("desktop_input_provider_observation_reused"))?;
        let bounds = locate_unique_color(&capture.image_bytes, TARGET_RGB)?;
        let center = PixelPoint {
            x: bounds.x + i64::from(bounds.width / 2),
            y: bounds.y + i64::from(bounds.height / 2),
        };
        let observation_sha256 = digest_text(&format!(
            "{}\0{}\0{}\0{}\0{}",
            capture.frame_receipt.content_sha256, bounds.x, bounds.y, bounds.width, bounds.height
        ));
        Ok(BeforeObservation {
            binding: Self::binding(&capture),
            context_id: capture.context.context_id,
            frame_id: capture.frame_receipt.frame_id,
            frame_sha256: capture.frame_receipt.content_sha256,
            captured_at_ms: now_ms(),
            observation_id: format!("desktop-observation-{}", &observation_sha256[..24]),
            observation_sha256,
            observation_status: "matched".to_string(),
            selected_candidate_id: Some("controlled-target".to_string()),
            selected_target_class: Some("synthetic_verification_control".to_string()),
            selected_bounds: Some(bounds),
            selected_center: Some(center),
        })
    }

    fn probe(
        &mut self,
        binding: &DesktopBinding,
    ) -> Result<SurfaceSnapshot, DesktopInteractionError> {
        let state = LockedServiceStateRepository::default_json()
            .and_then(|repository| repository.load_snapshot())
            .map_err(|_| provider_error("desktop_input_provider_state_unavailable"))?;
        let current = resolve_desktop_capture_binding_for_session(
            &state,
            &binding.browser_id,
            Some(&binding.session_name),
        )
        .map_err(|error| provider_error(error.code()))?;
        if current.route_id != binding.route_id
            || current.display_allocation_id != binding.display_allocation_id
            || current.display_name != self.display_name
        {
            return Err(provider_error("desktop_interaction_focus_changed"));
        }
        let scene = XTestSink::new(&self.display_name)
            .and_then(|sink| sink.probe())
            .map_err(|error| provider_error(error.code()))?;
        Ok(SurfaceSnapshot {
            provider_id: self.admission.provider_id.clone(),
            provider_version: self.admission.generation_id.clone(),
            provider_capability: self.admission.capability.clone(),
            surface_identity_digest: digest_text(&format!(
                "{}\0{}\0{}\0{}",
                binding.browser_id,
                binding.route_id,
                binding.display_allocation_id,
                binding.geometry_epoch
            )),
            browser_process_identity_digest: self.process_identity_digest.clone(),
            focused: scene.controlled_fixture_focused,
            client_bounds: PixelBounds {
                x: 0,
                y: 0,
                width: binding.width,
                height: binding.height,
            },
            pointer: PixelPoint {
                x: i64::from(scene.pointer_x),
                y: i64::from(scene.pointer_y),
            },
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
        _expected_surface: &SurfaceSnapshot,
        effect_key: &str,
        event: &InputEvent,
    ) -> Result<EventAcknowledgement, DesktopInteractionError> {
        let controlled = match event {
            InputEvent::PointerMove { point, .. } => ControlledX11Event::PointerMove {
                x: u32::try_from(point.x)
                    .map_err(|_| provider_error("desktop_input_event_out_of_bounds"))?,
                y: u32::try_from(point.y)
                    .map_err(|_| provider_error("desktop_input_event_out_of_bounds"))?,
            },
            InputEvent::LeftDown { .. } => ControlledX11Event::LeftDown,
            InputEvent::LeftUp { .. } => ControlledX11Event::LeftUp,
            InputEvent::KeyDown { key, .. } => ControlledX11Event::KeyDown { key: *key },
            InputEvent::KeyUp { key, .. } => ControlledX11Event::KeyUp { key: *key },
        };
        let route_id = binding.route_id.clone();
        let stream_id = binding.stream_id.clone();
        let lease_id = self.request.controller_lease_id.clone();
        let epoch = self.controller_epoch;
        let receipt = self
            .executor
            .execute_guarded(effect_key, &controlled, EFFECT_FENCE_DEADLINE, move || {
                let state = LockedServiceStateRepository::default_json()
                    .and_then(|repository| repository.load_snapshot())
                    .map_err(|_| {
                        DesktopInputProviderError::from_code(
                            "desktop_input_provider_state_unavailable",
                        )
                    })?;
                if !controller_authority_fence_matches(
                    &state, &route_id, &stream_id, &lease_id, epoch,
                ) {
                    return Err(DesktopInputProviderError::from_code(
                        "desktop_interaction_authority_changed",
                    ));
                }
                Ok(())
            })
            .map_err(|error| provider_error(error.code()))?;
        Ok(EventAcknowledgement {
            acknowledgement_id: receipt.acknowledgement_id,
        })
    }

    fn observe_after(
        &mut self,
        _binding: &DesktopBinding,
    ) -> Result<AfterObservation, DesktopInteractionError> {
        let capture = self.capture()?;
        let passed = locate_unique_color(&capture.image_bytes, SUCCESS_RGB).is_ok();
        let observation_sha256 = digest_text(&format!(
            "{}\0{}",
            capture.frame_receipt.content_sha256, passed
        ));
        Ok(AfterObservation {
            binding: Self::binding(&capture),
            context_id: capture.context.context_id,
            frame_id: capture.frame_receipt.frame_id,
            frame_sha256: capture.frame_receipt.content_sha256,
            observation_id: format!("desktop-observation-{}", &observation_sha256[..24]),
            observation_sha256,
            verification_state: if passed { "passed" } else { "failed" }.to_string(),
            text_sha256: passed.then(|| digest_text(FIXED_TEXT)),
        })
    }
}

impl ConfiguredControllerAuthorityRepository {
    fn resolve(
        &self,
        state: &ServiceState,
    ) -> Result<ControllerAuthority, DesktopInteractionError> {
        let browser = state
            .browsers
            .get(&self.request.browser_id)
            .ok_or_else(|| provider_error("desktop_interaction_authority_required"))?;
        let stream = browser
            .view_streams
            .iter()
            .find(|stream| stream.id == self.stream_id)
            .ok_or_else(|| provider_error("desktop_interaction_authority_required"))?;
        let route = state
            .remote_view_routes
            .get(&self.route_id)
            .ok_or_else(|| provider_error("desktop_interaction_authority_required"))?;
        let lease = state
            .viewer_leases
            .get(&self.request.controller_lease_id)
            .ok_or_else(|| provider_error("desktop_interaction_authority_required"))?;
        let expires = lease
            .expires_at
            .as_deref()
            .and_then(parse_timestamp_ms)
            .unwrap_or(u64::MAX);
        Ok(ControllerAuthority {
            browser_id: self.request.browser_id.clone(),
            display_allocation_id: self.display_allocation_id.clone(),
            stream_id: self.stream_id.clone(),
            route_id: self.route_id.clone(),
            route_controller_lease_id: route.controller_lease_id.clone().unwrap_or_default(),
            stream_controller_lease_id: stream.controller_lease_id.clone().unwrap_or_default(),
            lease_id: lease.id.clone(),
            lease_record_id: lease.id.clone(),
            lease_route_id: lease.route_id.clone().unwrap_or_default(),
            lease_browser_id: lease.browser_id.clone().unwrap_or_default(),
            lease_viewer_id: lease.viewer_id.clone().unwrap_or_default(),
            lease_role: lease.viewer_role.clone(),
            lease_state: lease.state.clone(),
            lease_updated_at: lease.updated_at.clone().unwrap_or_default(),
            lease_expires_at_ms: expires,
            controller_epoch: route.controller_epoch,
            route_controller_epoch: route.controller_epoch,
            stream_controller_epoch: stream.controller_epoch,
            route_contains_lease: route.viewer_lease_ids.contains(&lease.id),
            stream_contains_lease: stream.viewer_lease_ids.contains(&lease.id),
            route_writable: !route.read_only,
            stream_writable: !stream.read_only,
            route_machine_input: Some(self.machine_input.clone()),
            stream_machine_input: Some(self.machine_input.clone()),
        })
    }
}

impl ControllerAuthorityRepository for ConfiguredControllerAuthorityRepository {
    fn snapshot(&mut self) -> Result<ControllerAuthority, DesktopInteractionError> {
        let state = LockedServiceStateRepository::default_json()
            .and_then(|repository| repository.load_snapshot())
            .map_err(|_| provider_error("desktop_input_provider_state_unavailable"))?;
        self.resolve(&state)
    }
}

impl InteractionClock for SystemInteractionClock {
    fn now_ms(&mut self) -> u64 {
        now_ms()
    }
}

fn locate_unique_color(bytes: &[u8], rgb: [u8; 3]) -> Result<PixelBounds, DesktopInteractionError> {
    let image = image::load_from_memory(bytes)
        .map_err(|_| provider_error("desktop_input_provider_frame_invalid"))?
        .to_rgb8();
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut count = 0u64;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel.0 == rgb {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            count += 1;
        }
    }
    if count < 64 || min_x == u32::MAX {
        return Err(provider_error("desktop_interaction_target_unavailable"));
    }
    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    if count != u64::from(width) * u64::from(height) {
        return Err(provider_error("desktop_interaction_target_ambiguous"));
    }
    Ok(PixelBounds {
        x: i64::from(min_x),
        y: i64::from(min_y),
        width,
        height,
    })
}

fn parse_timestamp_ms(value: &str) -> Option<u64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .and_then(|value| u64::try_from(value.timestamp_millis()).ok())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn digest_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn digest_serializable(value: &impl serde::Serialize) -> Result<String, DesktopInteractionError> {
    serde_json::to_vec(value)
        .map(|value| format!("{:x}", Sha256::digest(value)))
        .map_err(|_| provider_error("desktop_input_provider_identity_unavailable"))
}

fn provider_error(code: &'static str) -> DesktopInteractionError {
    DesktopInteractionError::new(code, "the controlled desktop input provider failed closed")
}

#[cfg(test)]
mod tests {
    use super::{locate_unique_color, SUCCESS_RGB, TARGET_RGB};
    use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    fn png(rectangles: &[([u8; 3], u32, u32, u32, u32)]) -> Vec<u8> {
        let mut image = RgbImage::from_pixel(160, 120, Rgb([250, 250, 250]));
        for (color, x, y, width, height) in rectangles {
            for py in *y..(*y + *height) {
                for px in *x..(*x + *width) {
                    image.put_pixel(px, py, Rgb(*color));
                }
            }
        }
        let mut output = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(image)
            .write_to(&mut output, ImageFormat::Png)
            .unwrap();
        output.into_inner()
    }

    #[test]
    fn controlled_target_locator_requires_one_solid_exact_region() {
        let image = png(&[(TARGET_RGB, 20, 30, 24, 12), (SUCCESS_RGB, 80, 60, 30, 10)]);
        let bounds = locate_unique_color(&image, TARGET_RGB).unwrap();
        assert_eq!(
            (bounds.x, bounds.y, bounds.width, bounds.height),
            (20, 30, 24, 12)
        );
    }

    #[test]
    fn controlled_target_locator_rejects_ambiguous_regions() {
        let image = png(&[(TARGET_RGB, 20, 30, 24, 12), (TARGET_RGB, 80, 30, 24, 12)]);
        let error = locate_unique_color(&image, TARGET_RGB).unwrap_err();
        assert_eq!(error.code(), "desktop_interaction_target_ambiguous");
    }
}
