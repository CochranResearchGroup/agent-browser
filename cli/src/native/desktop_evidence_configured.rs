use super::desktop_capture::{
    capture_configured_desktop_frame, desktop_geometry_epoch, resolve_desktop_capture_binding,
    DesktopCaptureBinding, DesktopCaptureRequest, DesktopCaptureResult, DEFAULT_MAX_BYTES,
};
use super::desktop_evidence::{
    BrowserExternalSurface, CaptureReadyEvidence, CaptureReadyProof, CdpEvidenceAdapter,
    CdpEvidenceReceipt, ControllerPosture, DesktopEpisodeAdapterFailure,
    DesktopEpisodeAdmissionFailure, DesktopFrameAdapter, DesktopInputAdapter,
    EpisodeCleanupAdapter, EpisodeVerificationAdapter, ExternalUiTriggerAdapter,
    HumanHandoffAdapter, PresentationSlotAdapter, RestorationAuthority, SceneAdmissionRequest,
    SceneStagingAdapter, ViewerPosture, WindowSemanticAdapter,
};
use super::desktop_evidence_cdp::{CdpEpisodeProvider, ConfiguredCdpProvider, ConfiguredCdpTarget};
use super::presentation_capacity::{
    CapacityDecision, PresentationRequest, PresentationSlotState, PressureAdmission,
};
use super::service_model::ServiceState;
use super::service_store::ServiceStateRepository;
use super::x11_scene::{
    observe_browser_scene, restore_browser_scene, snapshot_browser_scene, stage_browser_scene,
    X11SceneEvidence, X11SceneSnapshot,
};
use std::time::Instant;

const CAPTURE_READY_MAXIMUM_AGE_MS: u64 = 500;
const FRAME_SCALE_FACTOR_MILLIS: u32 = 1000;

trait SceneProbe {
    fn observe(&self, pid: u32, display_name: &str) -> Result<X11SceneEvidence, String>;
}

struct ConfiguredSceneProbe;

impl SceneProbe for ConfiguredSceneProbe {
    fn observe(&self, pid: u32, display_name: &str) -> Result<X11SceneEvidence, String> {
        observe_browser_scene(pid, display_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SceneAuthority {
    binding: DesktopCaptureBinding,
    pid: u32,
    process_generation: String,
    display_isolation: String,
    viewer_posture: ViewerPosture,
    controller_posture: ControllerPosture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReservedSceneAuthority {
    scene: SceneAuthority,
    presentation_slot_id: String,
    slot_state: PresentationSlotState,
    scene_generation_number: u64,
    scene_generation: String,
}

/// Configured read-only scene and capture-readiness provider. It resolves only
/// service-owned browser, route, display, process-generation, slot, viewer,
/// and controller identities, then asks the native X11 probe for semantic
/// evidence without changing the desktop.
pub(crate) struct ConfiguredWindowSemanticAdapter<R> {
    repository: R,
    request_id: String,
    probe: Box<dyn SceneProbe>,
}

#[derive(Clone)]
enum NativeSceneSnapshot {
    X11(X11SceneSnapshot),
    #[cfg(test)]
    Fixture(String),
}

trait SceneStager {
    fn snapshot(&self, pid: u32, display_name: &str) -> Result<NativeSceneSnapshot, String>;
    fn stage(&self, snapshot: &NativeSceneSnapshot) -> Result<(), String>;
    fn restore(&self, snapshot: &NativeSceneSnapshot) -> Result<(), String>;
}

struct ConfiguredSceneStager;

impl SceneStager for ConfiguredSceneStager {
    fn snapshot(&self, pid: u32, display_name: &str) -> Result<NativeSceneSnapshot, String> {
        snapshot_browser_scene(pid, display_name).map(NativeSceneSnapshot::X11)
    }

    fn stage(&self, snapshot: &NativeSceneSnapshot) -> Result<(), String> {
        match snapshot {
            NativeSceneSnapshot::X11(snapshot) => stage_browser_scene(snapshot),
            #[cfg(test)]
            NativeSceneSnapshot::Fixture(_) => {
                Err("fixture scene snapshot reached the configured X11 provider".to_string())
            }
        }
    }

    fn restore(&self, snapshot: &NativeSceneSnapshot) -> Result<(), String> {
        match snapshot {
            NativeSceneSnapshot::X11(snapshot) => restore_browser_scene(snapshot),
            #[cfg(test)]
            NativeSceneSnapshot::Fixture(_) => {
                Err("fixture scene snapshot reached the configured X11 provider".to_string())
            }
        }
    }
}

/// Exact authority and ephemeral native snapshot for a reversible configured
/// scene-staging transaction. Provider handles never enter durable state.
pub(crate) struct ConfiguredSceneStagingAdapter<R> {
    repository: R,
    request_id: String,
    snapshot: Option<RestorationAuthority>,
    native_snapshot: Option<NativeSceneSnapshot>,
    prior_slot_state: Option<PresentationSlotState>,
    staged: bool,
    stager: Box<dyn SceneStager>,
}

impl<R> ConfiguredSceneStagingAdapter<R>
where
    R: ServiceStateRepository,
{
    pub(crate) fn new(repository: R, request_id: impl Into<String>) -> Self {
        Self {
            repository,
            request_id: request_id.into(),
            snapshot: None,
            native_snapshot: None,
            prior_slot_state: None,
            staged: false,
            stager: Box::new(ConfiguredSceneStager),
        }
    }

    #[cfg(test)]
    fn with_stager(
        repository: R,
        request_id: impl Into<String>,
        stager: impl SceneStager + 'static,
    ) -> Self {
        Self {
            repository,
            request_id: request_id.into(),
            snapshot: None,
            native_snapshot: None,
            prior_slot_state: None,
            staged: false,
            stager: Box::new(stager),
        }
    }

    fn resolved(
        &self,
        browser_id: &str,
    ) -> Result<(ServiceState, ReservedSceneAuthority), DesktopEpisodeAdapterFailure> {
        let state = self.repository.load_snapshot().map_err(|error| {
            DesktopEpisodeAdapterFailure::new(
                "scene_restoration",
                "desktop_scene_state_unavailable",
                error,
            )
        })?;
        let scene = resolve_scene_authority(&state, browser_id).map_err(|error| {
            DesktopEpisodeAdapterFailure::new(
                "scene_restoration",
                "desktop_scene_binding_unavailable",
                error,
            )
        })?;
        let reserved =
            resolve_reserved_scene_authority(&state, scene, &self.request_id).map_err(|error| {
                DesktopEpisodeAdapterFailure::new(
                    "scene_restoration",
                    "desktop_scene_reservation_unavailable",
                    error,
                )
            })?;
        Ok((state, reserved))
    }

    fn current(
        &self,
        browser_id: &str,
        staged_generation: bool,
    ) -> Result<(String, RestorationAuthority), DesktopEpisodeAdapterFailure> {
        let (state, reserved) = self.resolved(browser_id)?;
        let route = state
            .remote_view_routes
            .get(&reserved.scene.binding.route_id)
            .ok_or_else(|| {
                DesktopEpisodeAdapterFailure::new(
                    "scene_restoration",
                    "desktop_scene_route_unavailable",
                    "the exact scene route disappeared",
                )
            })?;
        let display = state
            .display_allocations
            .get(&reserved.scene.binding.display_allocation_id)
            .ok_or_else(|| {
                DesktopEpisodeAdapterFailure::new(
                    "scene_restoration",
                    "desktop_scene_display_unavailable",
                    "the exact scene display allocation disappeared",
                )
            })?;
        let route_generation = format!(
            "route:{}:{}:{}:display:{}:{}:{}:{}",
            route.id,
            route.controller_epoch,
            route.connection_id.as_deref().unwrap_or("none"),
            display.id,
            display.state,
            display.display_isolation,
            display.display_name.as_deref().unwrap_or("none")
        );
        let controller_generation = format!(
            "controller:{}:{}",
            route.controller_epoch,
            route.controller_lease_id.as_deref().unwrap_or("none")
        );
        let authority_scene_generation = if staged_generation {
            format!(
                "scene:{}:{}",
                reserved.presentation_slot_id,
                reserved.scene_generation_number.saturating_add(1)
            )
        } else {
            reserved.scene_generation.clone()
        };
        Ok((
            reserved.scene_generation.clone(),
            RestorationAuthority::new(
                authority_scene_generation,
                route_generation,
                controller_generation,
            ),
        ))
    }

    fn transition_slot(
        &self,
        browser_id: &str,
        next: PresentationSlotState,
    ) -> Result<(), DesktopEpisodeAdapterFailure> {
        let request_id = self.request_id.clone();
        let browser_id = browser_id.to_string();
        self.repository
            .mutate(|state| {
                let scene = resolve_scene_authority(state, &browser_id)?;
                let reserved = resolve_reserved_scene_authority(state, scene, &request_id)?;
                let slot_id = reserved.presentation_slot_id;
                let mut capacity = state
                    .presentation_capacity
                    .take()
                    .ok_or_else(|| "presentation capacity is missing".to_string())?;
                let result = capacity.transition_slot(&slot_id, &request_id, next, Some(state));
                state.presentation_capacity = Some(capacity);
                result.map(|_| ())
            })
            .map_err(|error| {
                DesktopEpisodeAdapterFailure::new(
                    "scene_staging",
                    "desktop_scene_transition_failed",
                    error,
                )
            })
    }

    fn quarantine(&self, browser_id: &str, reason: &str) {
        let request_id = self.request_id.clone();
        let browser_id = browser_id.to_string();
        let obligation = format!("scene-restoration:{}:{reason}", self.request_id);
        let _ = self.repository.mutate(|state| {
            let scene = resolve_scene_authority(state, &browser_id)?;
            let reserved = resolve_reserved_scene_authority(state, scene, &request_id)?;
            state
                .presentation_capacity
                .as_mut()
                .ok_or_else(|| "presentation capacity is missing".to_string())?
                .quarantine_slot(&reserved.presentation_slot_id, obligation)
        });
    }
}

impl<R> SceneStagingAdapter for ConfiguredSceneStagingAdapter<R>
where
    R: ServiceStateRepository,
{
    fn snapshot(
        &mut self,
        browser_id: &str,
        requires_staging: bool,
    ) -> Result<(String, RestorationAuthority), DesktopEpisodeAdapterFailure> {
        let (scene_generation, authority) = self.current(browser_id, requires_staging)?;
        self.snapshot = Some(authority.clone());
        Ok((scene_generation, authority))
    }

    fn stage(&mut self, browser_id: &str) -> Result<String, DesktopEpisodeAdapterFailure> {
        let expected_authority = self.snapshot.as_ref().ok_or_else(|| {
            DesktopEpisodeAdapterFailure::new(
                "scene_staging",
                "desktop_scene_snapshot_missing",
                "scene staging was requested without a recorded authority snapshot",
            )
        })?;
        let (_, current_expected_authority) = self.current(browser_id, true)?;
        if expected_authority != &current_expected_authority {
            return Err(DesktopEpisodeAdapterFailure::new(
                "scene_staging",
                "desktop_scene_authority_drift",
                "service scene, route, or controller authority changed before native staging",
            ));
        }
        let (_, reserved) = self.resolved(browser_id)?;
        if !matches!(
            reserved.slot_state,
            PresentationSlotState::Reserved | PresentationSlotState::Active
        ) {
            return Err(DesktopEpisodeAdapterFailure::new(
                "scene_staging",
                "desktop_scene_slot_not_stageable",
                "the exact presentation slot is not reserved or active",
            ));
        }
        if reserved.scene.display_isolation != "private_virtual_display" {
            return Err(DesktopEpisodeAdapterFailure::new(
                "scene_staging",
                "desktop_scene_staging_requires_private_display",
                "visible scene staging is allowed only on an isolated private virtual display",
            ));
        }
        let native_snapshot = self
            .stager
            .snapshot(reserved.scene.pid, &reserved.scene.binding.display_name)
            .map_err(|error| {
                DesktopEpisodeAdapterFailure::new(
                    "scene_staging",
                    "desktop_scene_native_snapshot_failed",
                    error,
                )
            })?;
        let (_, authority_after_native_snapshot) = self.current(browser_id, true)?;
        if expected_authority != &authority_after_native_snapshot {
            return Err(DesktopEpisodeAdapterFailure::new(
                "scene_staging",
                "desktop_scene_authority_drift",
                "service scene, route, or controller authority changed during native snapshot",
            ));
        }
        if let Err(error) = self.stager.stage(&native_snapshot) {
            if let Err(restore_error) = self.stager.restore(&native_snapshot) {
                self.quarantine(browser_id, "stage_rollback_unverified");
                return Err(DesktopEpisodeAdapterFailure::new(
                    "scene_staging",
                    "desktop_scene_stage_and_rollback_failed",
                    format!("stage={error}; rollback={restore_error}"),
                ));
            }
            return Err(DesktopEpisodeAdapterFailure::new(
                "scene_staging",
                "desktop_scene_native_stage_failed",
                error,
            ));
        }
        if let Err(transition_error) =
            self.transition_slot(browser_id, PresentationSlotState::Staging)
        {
            if let Err(restore_error) = self.stager.restore(&native_snapshot) {
                self.quarantine(browser_id, "stage_commit_rollback_unverified");
                return Err(DesktopEpisodeAdapterFailure::new(
                    "scene_staging",
                    "desktop_scene_stage_commit_and_rollback_failed",
                    format!(
                        "commit={}; rollback={restore_error}",
                        transition_error.detail
                    ),
                ));
            }
            return Err(transition_error);
        }
        self.prior_slot_state = Some(reserved.slot_state);
        self.native_snapshot = Some(native_snapshot);
        self.staged = true;
        Ok(format!("scene-stage:{}", self.request_id))
    }

    fn current_authority(
        &mut self,
        browser_id: &str,
    ) -> Result<RestorationAuthority, DesktopEpisodeAdapterFailure> {
        self.current(browser_id, false)
            .map(|(_, authority)| authority)
    }

    fn restore(&mut self, browser_id: &str) -> Result<String, DesktopEpisodeAdapterFailure> {
        if self.snapshot.is_none() {
            return Err(DesktopEpisodeAdapterFailure::new(
                "scene_restoration",
                "desktop_scene_snapshot_missing",
                "scene restoration was requested without a recorded authority snapshot",
            ));
        }
        if !self.staged {
            return Ok(format!("scene-restoration-noop:{}", self.request_id));
        }
        let native_snapshot = self.native_snapshot.clone().ok_or_else(|| {
            DesktopEpisodeAdapterFailure::new(
                "scene_restoration",
                "desktop_scene_native_snapshot_missing",
                "a staged scene has no ephemeral native restoration snapshot",
            )
        })?;
        let prior_slot_state = self.prior_slot_state.ok_or_else(|| {
            DesktopEpisodeAdapterFailure::new(
                "scene_restoration",
                "desktop_scene_prior_slot_state_missing",
                "a staged scene has no recorded prior presentation-slot state",
            )
        })?;
        self.transition_slot(browser_id, PresentationSlotState::Restoring)
            .inspect_err(|_failure| {
                self.quarantine(browser_id, "restoration_transition_failed");
            })?;
        if let Err(error) = self.stager.restore(&native_snapshot) {
            self.quarantine(browser_id, "native_restoration_unverified");
            return Err(DesktopEpisodeAdapterFailure::new(
                "scene_restoration",
                "desktop_scene_native_restoration_failed",
                error,
            ));
        }
        if let Err(failure) = self.transition_slot(browser_id, prior_slot_state) {
            self.quarantine(browser_id, "restoration_commit_failed");
            return Err(failure);
        }
        self.staged = false;
        self.native_snapshot = None;
        self.prior_slot_state = None;
        Ok(format!("scene-restoration:{}", self.request_id))
    }
}

pub(crate) struct ConfiguredEpisodeVerificationAdapter<R> {
    repository: R,
    request_id: String,
}

impl<R> ConfiguredEpisodeVerificationAdapter<R>
where
    R: ServiceStateRepository,
{
    pub(crate) fn new(repository: R, request_id: impl Into<String>) -> Self {
        Self {
            repository,
            request_id: request_id.into(),
        }
    }
}

impl<R> EpisodeVerificationAdapter for ConfiguredEpisodeVerificationAdapter<R>
where
    R: ServiceStateRepository,
{
    fn verify(
        &mut self,
        browser_id: &str,
    ) -> Result<(String, String), DesktopEpisodeAdapterFailure> {
        let state = self.repository.load_snapshot().map_err(|error| {
            DesktopEpisodeAdapterFailure::new(
                "episode_verification",
                "desktop_scene_state_unavailable",
                error,
            )
        })?;
        let scene = resolve_scene_authority(&state, browser_id).map_err(|error| {
            DesktopEpisodeAdapterFailure::new(
                "episode_verification",
                "desktop_scene_binding_unavailable",
                error,
            )
        })?;
        let reserved =
            resolve_reserved_scene_authority(&state, scene, &self.request_id).map_err(|error| {
                DesktopEpisodeAdapterFailure::new(
                    "episode_verification",
                    "desktop_scene_reservation_unavailable",
                    error,
                )
            })?;
        Ok((
            format!(
                "episode-verification:{}:{}",
                self.request_id, reserved.scene_generation
            ),
            reserved.scene_generation,
        ))
    }
}

pub(crate) struct ConfiguredUnusedCdpAdapter;

impl CdpEvidenceAdapter for ConfiguredUnusedCdpAdapter {
    fn collect_page(
        &mut self,
        browser_id: &str,
    ) -> Result<CdpEvidenceReceipt, DesktopEpisodeAdapterFailure> {
        Ok(CdpEvidenceReceipt {
            receipt_id: format!("cdp-delegation:{browser_id}"),
        })
    }

    fn confirm_browser_external_absent(
        &mut self,
        browser_id: &str,
        _surface: BrowserExternalSurface,
    ) -> Result<String, DesktopEpisodeAdapterFailure> {
        Err(DesktopEpisodeAdapterFailure::new(
            "paired_page_evidence",
            "desktop_paired_cdp_provider_unavailable",
            format!("paired CDP evidence is unavailable for {browser_id}"),
        ))
    }
}

pub(crate) struct ConfiguredPairedCdpAdapter {
    target: ConfiguredCdpTarget,
    provider: Box<dyn CdpEpisodeProvider>,
}

impl ConfiguredPairedCdpAdapter {
    pub(crate) fn new(target: ConfiguredCdpTarget, provider: ConfiguredCdpProvider) -> Self {
        Self {
            target,
            provider: Box::new(provider),
        }
    }

    #[cfg(test)]
    fn with_provider(
        target: ConfiguredCdpTarget,
        provider: impl CdpEpisodeProvider + 'static,
    ) -> Self {
        Self {
            target,
            provider: Box::new(provider),
        }
    }
}

impl CdpEvidenceAdapter for ConfiguredPairedCdpAdapter {
    fn collect_page(
        &mut self,
        _browser_id: &str,
    ) -> Result<CdpEvidenceReceipt, DesktopEpisodeAdapterFailure> {
        Err(DesktopEpisodeAdapterFailure::new(
            "page_evidence",
            "desktop_cdp_page_collection_not_configured",
            "this configured adapter is scoped to paired browser-external evidence",
        ))
    }

    fn confirm_browser_external_absent(
        &mut self,
        browser_id: &str,
        surface: BrowserExternalSurface,
    ) -> Result<String, DesktopEpisodeAdapterFailure> {
        if self.target.browser_id != browser_id {
            return Err(DesktopEpisodeAdapterFailure::new(
                "paired_page_evidence",
                "desktop_cdp_browser_binding_drift",
                "paired CDP evidence no longer identifies the requested service browser",
            ));
        }
        self.provider
            .confirm_browser_external_absent(&self.target, surface)
            .map_err(|failure| {
                DesktopEpisodeAdapterFailure::new(
                    "paired_page_evidence",
                    failure.code,
                    failure.detail,
                )
            })
    }
}

pub(crate) struct ConfiguredUnusedTriggerAdapter;

impl ExternalUiTriggerAdapter for ConfiguredUnusedTriggerAdapter {
    fn trigger(&mut self, browser_id: &str) -> Result<String, DesktopEpisodeAdapterFailure> {
        Err(DesktopEpisodeAdapterFailure::new(
            "browser_external_trigger",
            "desktop_external_trigger_provider_unavailable",
            format!("browser-external trigger is unavailable for {browser_id}"),
        ))
    }
}

pub(crate) struct ConfiguredCdpTriggerAdapter {
    target: ConfiguredCdpTarget,
    selector: String,
    effect_key: String,
    provider: Box<dyn CdpEpisodeProvider>,
}

impl ConfiguredCdpTriggerAdapter {
    pub(crate) fn new(
        target: ConfiguredCdpTarget,
        selector: impl Into<String>,
        effect_key: impl Into<String>,
        provider: ConfiguredCdpProvider,
    ) -> Self {
        Self {
            target,
            selector: selector.into(),
            effect_key: effect_key.into(),
            provider: Box::new(provider),
        }
    }

    #[cfg(test)]
    fn with_provider(
        target: ConfiguredCdpTarget,
        selector: impl Into<String>,
        effect_key: impl Into<String>,
        provider: impl CdpEpisodeProvider + 'static,
    ) -> Self {
        Self {
            target,
            selector: selector.into(),
            effect_key: effect_key.into(),
            provider: Box::new(provider),
        }
    }
}

impl ExternalUiTriggerAdapter for ConfiguredCdpTriggerAdapter {
    fn trigger(&mut self, browser_id: &str) -> Result<String, DesktopEpisodeAdapterFailure> {
        if self.target.browser_id != browser_id {
            return Err(DesktopEpisodeAdapterFailure::new(
                "browser_external_trigger",
                "desktop_trigger_browser_binding_drift",
                "the page trigger no longer identifies the requested service browser",
            ));
        }
        self.provider
            .trigger(&self.target, &self.selector, &self.effect_key)
            .map_err(|failure| {
                DesktopEpisodeAdapterFailure::new(
                    "browser_external_trigger",
                    failure.code,
                    failure.detail,
                )
            })
    }
}

pub(crate) struct ConfiguredBlockedInputAdapter;

impl DesktopInputAdapter for ConfiguredBlockedInputAdapter {
    fn apply(&mut self, browser_id: &str, _authority_receipt_id: &str) -> String {
        format!("desktop-input-blocked:{browser_id}")
    }
}

pub(crate) struct ConfiguredExistingHandoffAdapter {
    handoff_receipt_id: String,
}

impl ConfiguredExistingHandoffAdapter {
    pub(crate) fn from_state(state: &ServiceState, browser_id: &str) -> Self {
        let handoff_receipt_id = state
            .remote_view_handoffs
            .values()
            .filter(|handoff| handoff.browser_id.as_deref() == Some(browser_id))
            .filter(|handoff| matches!(handoff.state.as_str(), "ready" | "resolving" | "active"))
            .map(|handoff| handoff.id.as_str())
            .min()
            .map(|handoff_id| format!("durable-handoff:{handoff_id}"))
            .unwrap_or_else(|| "durable-handoff-unavailable".to_string());
        Self { handoff_receipt_id }
    }
}

impl HumanHandoffAdapter for ConfiguredExistingHandoffAdapter {
    fn prepare(&mut self, _browser_id: &str, _reason: &'static str) -> String {
        self.handoff_receipt_id.clone()
    }
}

pub(crate) struct ConfiguredEpisodeCleanupAdapter;

impl EpisodeCleanupAdapter for ConfiguredEpisodeCleanupAdapter {
    fn complete(&mut self, episode_id: &str) -> String {
        format!("episode-cleanup:{episode_id}")
    }
}

impl<R> ConfiguredWindowSemanticAdapter<R>
where
    R: ServiceStateRepository,
{
    pub(crate) fn new(repository: R, request_id: impl Into<String>) -> Self {
        Self {
            repository,
            request_id: request_id.into(),
            probe: Box::new(ConfiguredSceneProbe),
        }
    }

    #[cfg(test)]
    fn with_probe(
        repository: R,
        request_id: impl Into<String>,
        probe: impl SceneProbe + 'static,
    ) -> Self {
        Self {
            repository,
            request_id: request_id.into(),
            probe: Box::new(probe),
        }
    }

    fn adapter_failure(
        phase: &'static str,
        code: impl Into<String>,
        detail: impl Into<String>,
    ) -> DesktopEpisodeAdapterFailure {
        DesktopEpisodeAdapterFailure::new(phase, code, detail)
    }

    fn load_scene_authority(
        &self,
        browser_id: &str,
        phase: &'static str,
    ) -> Result<(ServiceState, SceneAuthority), DesktopEpisodeAdapterFailure> {
        let state = self.repository.load_snapshot().map_err(|error| {
            Self::adapter_failure(phase, "desktop_scene_state_unavailable", error)
        })?;
        let authority = resolve_scene_authority(&state, browser_id).map_err(|error| {
            Self::adapter_failure(phase, "desktop_scene_binding_unavailable", error)
        })?;
        Ok((state, authority))
    }

    fn load_reserved_scene_authority(
        &self,
        browser_id: &str,
        phase: &'static str,
    ) -> Result<ReservedSceneAuthority, DesktopEpisodeAdapterFailure> {
        let (state, scene) = self.load_scene_authority(browser_id, phase)?;
        resolve_reserved_scene_authority(&state, scene, &self.request_id).map_err(|error| {
            Self::adapter_failure(phase, "desktop_scene_reservation_unavailable", error)
        })
    }

    fn mark_capture_ready_if_staging(
        &self,
        browser_id: &str,
    ) -> Result<(), DesktopEpisodeAdapterFailure> {
        let request_id = self.request_id.clone();
        let browser_id = browser_id.to_string();
        self.repository
            .mutate(|state| {
                let scene = resolve_scene_authority(state, &browser_id)?;
                let reserved = resolve_reserved_scene_authority(state, scene, &request_id)?;
                if reserved.slot_state != PresentationSlotState::Staging {
                    return Ok(());
                }
                let slot_id = reserved.presentation_slot_id;
                let mut capacity = state
                    .presentation_capacity
                    .take()
                    .ok_or_else(|| "presentation capacity is missing".to_string())?;
                let result = capacity.transition_slot(
                    &slot_id,
                    &request_id,
                    PresentationSlotState::CaptureReady,
                    None,
                );
                state.presentation_capacity = Some(capacity);
                result.map(|_| ())
            })
            .map_err(|error| {
                Self::adapter_failure(
                    "capture_ready",
                    "desktop_scene_capture_ready_transition_failed",
                    error,
                )
            })
    }
}

impl<R> WindowSemanticAdapter for ConfiguredWindowSemanticAdapter<R>
where
    R: ServiceStateRepository,
{
    fn scene_admission(
        &mut self,
        browser_id: &str,
        requires_staging: bool,
    ) -> Result<SceneAdmissionRequest, DesktopEpisodeAdapterFailure> {
        let (_, authority) = self.load_scene_authority(browser_id, "scene_admission")?;
        let evidence = self
            .probe
            .observe(authority.pid, &authority.binding.display_name)
            .map_err(|error| {
                Self::adapter_failure("scene_admission", "desktop_scene_probe_unavailable", error)
            })?;
        Ok(SceneAdmissionRequest {
            viewer_posture: authority.viewer_posture,
            controller_posture: authority.controller_posture,
            requires_staging,
            capture_ready: scene_is_capture_ready(evidence),
            explicit_takeover: false,
        })
    }

    fn capture_ready(
        &mut self,
        browser_id: &str,
    ) -> Result<CaptureReadyEvidence, DesktopEpisodeAdapterFailure> {
        let before = self.load_reserved_scene_authority(browser_id, "capture_ready")?;
        let started = Instant::now();
        let evidence = self
            .probe
            .observe(before.scene.pid, &before.scene.binding.display_name)
            .map_err(|error| {
                Self::adapter_failure("capture_ready", "desktop_scene_probe_unavailable", error)
            })?;
        let after = self.load_reserved_scene_authority(browser_id, "capture_ready")?;
        if before != after {
            return Err(Self::adapter_failure(
                "capture_ready",
                "desktop_scene_binding_drift",
                "service-owned process, route, display, slot, scene, viewer, or controller authority changed during the scene probe",
            ));
        }
        if scene_is_capture_ready(evidence) {
            self.mark_capture_ready_if_staging(browser_id)?;
        }
        let proof_age_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let geometry_epoch = desktop_geometry_epoch(
            &before.scene.binding,
            evidence.frame_width,
            evidence.frame_height,
            FRAME_SCALE_FACTOR_MILLIS,
        );
        Ok(CaptureReadyEvidence::new(
            before.scene.binding.browser_id,
            before.scene.process_generation,
            before.scene.binding.route_id,
            before.scene.binding.display_allocation_id,
            before.presentation_slot_id,
            before.scene_generation,
            geometry_epoch,
            evidence.frame_width,
            evidence.frame_height,
            FRAME_SCALE_FACTOR_MILLIS,
            evidence.active_window_owned,
            evidence.topmost_window_owned,
            evidence.authorized_geometry,
            evidence.capture_region_unoccluded,
            before.scene.viewer_posture,
            before.scene.controller_posture,
            proof_age_ms,
            CAPTURE_READY_MAXIMUM_AGE_MS,
        ))
    }
}

#[derive(Default)]
pub(crate) struct ConfiguredDesktopFrameAdapter {
    capture: Option<DesktopCaptureResult>,
}

impl ConfiguredDesktopFrameAdapter {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn take_capture(&mut self) -> Option<DesktopCaptureResult> {
        self.capture.take()
    }
}

impl DesktopFrameAdapter for ConfiguredDesktopFrameAdapter {
    fn capture(
        &mut self,
        browser_id: &str,
        proof: &CaptureReadyProof,
    ) -> Result<String, DesktopEpisodeAdapterFailure> {
        let capture = capture_configured_desktop_frame(DesktopCaptureRequest {
            browser_id: browser_id.to_string(),
            session_name: None,
            max_bytes: DEFAULT_MAX_BYTES,
        })
        .map_err(|error| {
            DesktopEpisodeAdapterFailure::new(
                "desktop_frame_capture",
                error.code(),
                error.to_string(),
            )
        })?;
        let receipt_id = validate_configured_capture(proof, &capture)?;
        self.capture = Some(capture);
        Ok(receipt_id)
    }
}

fn validate_configured_capture(
    proof: &CaptureReadyProof,
    capture: &DesktopCaptureResult,
) -> Result<String, DesktopEpisodeAdapterFailure> {
    let context = &capture.context;
    let receipt = &capture.frame_receipt;
    let scale_factor_millis = (context.scale_factor * 1000.0).round() as u32;
    if context.browser_id != proof.browser_id
        || context.route_id != proof.route_id
        || context.display_allocation_id != proof.display_allocation_id
        || context.geometry_epoch != proof.geometry_epoch
        || receipt.geometry_epoch != proof.geometry_epoch
        || context.width != proof.frame_width
        || context.height != proof.frame_height
        || receipt.width != proof.frame_width
        || receipt.height != proof.frame_height
        || scale_factor_millis != proof.scale_factor_millis
        || proof.capture_region.x != 0
        || proof.capture_region.y != 0
        || proof.capture_region.width != context.width
        || proof.capture_region.height != context.height
        || proof.coordinate_space != context.coordinate_space
    {
        return Err(DesktopEpisodeAdapterFailure::new(
            "desktop_frame_capture",
            "desktop_frame_binding_drift",
            "captured frame does not match the exact capture-ready browser, route, display, geometry, crop, scale, or coordinate binding",
        ));
    }
    Ok(receipt.frame_id.clone())
}

fn resolve_scene_authority(
    state: &ServiceState,
    browser_id: &str,
) -> Result<SceneAuthority, String> {
    let binding =
        resolve_desktop_capture_binding(state, browser_id).map_err(|error| error.to_string())?;
    let browser = state
        .browsers
        .get(browser_id)
        .ok_or_else(|| "service browser is missing".to_string())?;
    let pid = browser
        .pid
        .ok_or_else(|| "service browser PID is missing".to_string())?;
    let process_identity = state
        .browser_process_identities
        .get(browser_id)
        .ok_or_else(|| "service browser process generation is missing".to_string())?;
    if process_identity.process_identity.pid != pid {
        return Err("service browser PID and process-generation identity disagree".to_string());
    }
    let route = state
        .remote_view_routes
        .get(&binding.route_id)
        .ok_or_else(|| "service browser route is missing".to_string())?;
    let display = state
        .display_allocations
        .get(&binding.display_allocation_id)
        .ok_or_else(|| "service browser display allocation is missing".to_string())?;
    let controller_posture = if route.controller_lease_id.is_some() {
        ControllerPosture::Human
    } else {
        ControllerPosture::Uncontrolled
    };
    let passive_viewer_present = route
        .viewer_lease_ids
        .iter()
        .filter_map(|lease_id| state.viewer_leases.get(lease_id))
        .any(|lease| {
            lease.viewer_role != "controller"
                && matches!(lease.state.as_str(), "requested" | "active" | "ready")
        });
    let viewer_posture = if passive_viewer_present {
        ViewerPosture::Passive
    } else {
        ViewerPosture::None
    };
    Ok(SceneAuthority {
        binding,
        pid,
        process_generation: format!(
            "process:{pid}:{}",
            process_identity.process_identity.start_token
        ),
        display_isolation: display.display_isolation.clone(),
        viewer_posture,
        controller_posture,
    })
}

fn resolve_reserved_scene_authority(
    state: &ServiceState,
    scene: SceneAuthority,
    request_id: &str,
) -> Result<ReservedSceneAuthority, String> {
    let capacity = state
        .presentation_capacity
        .as_ref()
        .ok_or_else(|| "presentation capacity is missing".to_string())?;
    let slot = capacity
        .slots
        .iter()
        .find(|slot| slot.lease_request_id.as_deref() == Some(request_id))
        .ok_or_else(|| "episode presentation-slot lease is missing".to_string())?;
    if slot.browser_id.as_deref() != Some(scene.binding.browser_id.as_str())
        || slot.route_id.as_deref() != Some(scene.binding.route_id.as_str())
        || slot.display_allocation_id.as_deref()
            != Some(scene.binding.display_allocation_id.as_str())
        || !matches!(
            slot.state,
            PresentationSlotState::Reserved
                | PresentationSlotState::Staging
                | PresentationSlotState::CaptureReady
                | PresentationSlotState::Active
                | PresentationSlotState::Restoring
        )
    {
        return Err("episode slot does not match the exact browser presentation".to_string());
    }
    Ok(ReservedSceneAuthority {
        scene,
        presentation_slot_id: slot.id.clone(),
        slot_state: slot.state,
        scene_generation_number: slot.scene_generation,
        scene_generation: format!("scene:{}:{}", slot.id, slot.scene_generation),
    })
}

fn scene_is_capture_ready(evidence: X11SceneEvidence) -> bool {
    evidence.active_window_owned
        && evidence.topmost_window_owned
        && evidence.authorized_geometry
        && evidence.capture_region_unoccluded
        && evidence.frame_width > 0
        && evidence.frame_height > 0
}

/// Durable presentation-capacity adapter for one configured desktop evidence
/// episode. The adapter never invents a slot when Service State cannot commit
/// the admission or release mutation.
pub(crate) struct ConfiguredPresentationSlotAdapter<R> {
    repository: R,
    request_id: String,
    pressure: PressureAdmission,
    reservation: Option<(String, String)>,
}

impl<R> ConfiguredPresentationSlotAdapter<R>
where
    R: ServiceStateRepository,
{
    pub(crate) fn new(
        repository: R,
        request_id: impl Into<String>,
        admitted_maximum: usize,
    ) -> Self {
        Self {
            repository,
            request_id: request_id.into(),
            pressure: PressureAdmission::admit(admitted_maximum),
            reservation: None,
        }
    }

    fn admission_failure(decision: CapacityDecision) -> DesktopEpisodeAdmissionFailure {
        match decision {
            CapacityDecision::Queued {
                queue_position,
                limiting_resource,
                next_safe_action,
                ..
            } => DesktopEpisodeAdmissionFailure::new(
                "presentation_capacity_queued",
                format!(
                    "queuePosition={queue_position}; limitingResource={limiting_resource:?}; nextSafeAction={next_safe_action:?}"
                ),
            ),
            CapacityDecision::Rejected {
                limiting_resource,
                next_safe_action,
                ..
            } => DesktopEpisodeAdmissionFailure::new(
                "presentation_capacity_rejected",
                format!(
                    "limitingResource={limiting_resource:?}; nextSafeAction={next_safe_action:?}"
                ),
            ),
            CapacityDecision::Granted { .. } => DesktopEpisodeAdmissionFailure::new(
                "presentation_capacity_invalid_decision",
                "granted admission was handled as unavailable",
            ),
        }
    }
}

impl<R> PresentationSlotAdapter for ConfiguredPresentationSlotAdapter<R>
where
    R: ServiceStateRepository,
{
    fn reserve(&mut self, browser_id: &str) -> Result<String, DesktopEpisodeAdmissionFailure> {
        if self.reservation.is_some() {
            return Err(DesktopEpisodeAdmissionFailure::new(
                "presentation_capacity_duplicate_reservation",
                "the configured episode already owns a presentation slot",
            ));
        }
        let request_id = self.request_id.clone();
        let browser_id = browser_id.to_string();
        let pressure = self.pressure;
        let mut unavailable = None;
        let mutation = self.repository.mutate(|state| {
            let binding = resolve_desktop_capture_binding(state, &browser_id)
                .map_err(|error| error.to_string())?;
            let Some(mut capacity) = state.presentation_capacity.take() else {
                return Err("presentation_capacity_unavailable".to_string());
            };
            let decision = capacity.request_bound_observation(
                PresentationRequest::observation(request_id.clone()).for_browser(&browser_id),
                pressure,
                state,
                &binding.route_id,
                &binding.display_allocation_id,
            );
            state.presentation_capacity = Some(capacity);
            match decision {
                CapacityDecision::Granted { slot_id, .. } => Ok(slot_id),
                decision => {
                    unavailable = Some(decision);
                    Err("presentation_capacity_not_admitted".to_string())
                }
            }
        });

        let slot_id = match mutation {
            Ok(slot_id) => slot_id,
            Err(_) if unavailable.is_some() => {
                return Err(Self::admission_failure(unavailable.expect("checked above")));
            }
            Err(error) => {
                return Err(DesktopEpisodeAdmissionFailure::new(
                    "presentation_capacity_persistence_failed",
                    error,
                ));
            }
        };
        self.reservation = Some((browser_id, slot_id.clone()));
        Ok(format!("presentation-admission:{request_id}:{slot_id}"))
    }

    fn release(&mut self, browser_id: &str) -> Result<String, DesktopEpisodeAdmissionFailure> {
        let Some((reserved_browser_id, slot_id)) = self.reservation.clone() else {
            return Err(DesktopEpisodeAdmissionFailure::new(
                "presentation_release_without_reservation",
                "the configured episode does not own a presentation slot",
            ));
        };
        if reserved_browser_id != browser_id {
            return Err(DesktopEpisodeAdmissionFailure::new(
                "presentation_release_browser_mismatch",
                "the release browser does not match the admitted browser",
            ));
        }
        let pressure = self.pressure;
        let request_id = self.request_id.clone();
        let mutation = self.repository.mutate(|state| {
            let Some(mut capacity) = state.presentation_capacity.take() else {
                return Err("presentation_capacity_unavailable".to_string());
            };
            if !capacity.slots.iter().any(|slot| slot.id == slot_id) {
                state.presentation_capacity = Some(capacity);
                return Err("presentation_reserved_slot_missing".to_string());
            }
            capacity.release_bound_observation(&slot_id, &request_id, pressure, state)?;
            state.presentation_capacity = Some(capacity);
            Ok(())
        });
        if let Err(error) = mutation {
            return Err(DesktopEpisodeAdmissionFailure::new(
                "presentation_release_persistence_failed",
                error,
            ));
        }
        self.reservation = None;
        Ok(format!(
            "presentation-release:{}:{slot_id}",
            self.request_id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::desktop_capture::{DesktopContext, FrameReceipt};
    use crate::native::desktop_evidence::{
        DesktopEvidenceCoordinator, SceneAdmission, WindowSemanticAdapter,
    };
    use crate::native::presentation_capacity::{
        PresentationCapacityAuthority, PresentationCapacityConfig, PresentationSlot,
        PresentationSlotState,
    };
    use crate::native::service_model::{
        BrowserHealth, BrowserHost, BrowserProcess, DisplayAllocation, RemoteViewRoute,
        ServiceBrowserProcessIdentity, ServiceState, ViewStream, ViewStreamProvider, ViewerLease,
    };
    use crate::process_identity::RecordedProcessIdentity;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct FakeCdpEpisodeProvider {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl FakeCdpEpisodeProvider {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl CdpEpisodeProvider for FakeCdpEpisodeProvider {
        fn confirm_browser_external_absent(
            &self,
            target: &ConfiguredCdpTarget,
            surface: BrowserExternalSurface,
        ) -> Result<String, super::super::desktop_evidence_cdp::ConfiguredCdpFailure> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("absence:{}:{surface:?}", target.browser_id));
            Ok("paired-cdp-absence:fixture".to_string())
        }

        fn trigger(
            &self,
            target: &ConfiguredCdpTarget,
            selector: &str,
            effect_key: &str,
        ) -> Result<String, super::super::desktop_evidence_cdp::ConfiguredCdpFailure> {
            self.calls.lock().unwrap().push(format!(
                "trigger:{}:{selector}:{effect_key}",
                target.browser_id
            ));
            Ok("cdp-page-trigger:fixture".to_string())
        }
    }

    #[derive(Clone)]
    struct MemoryRepository(Arc<Mutex<ServiceState>>);

    impl MemoryRepository {
        fn new(state: ServiceState) -> Self {
            Self(Arc::new(Mutex::new(state)))
        }
    }

    impl ServiceStateRepository for MemoryRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn mutate<T>(
            &self,
            mutator: impl FnOnce(&mut ServiceState) -> Result<T, String>,
        ) -> Result<T, String> {
            let mut state = self.0.lock().unwrap();
            let mut candidate = state.clone();
            let result = mutator(&mut candidate)?;
            *state = candidate;
            Ok(result)
        }
    }

    fn state(slot_count: usize) -> ServiceState {
        let mut state = configured_scene_state(false, false);
        let slots = (0..slot_count)
            .map(|index| {
                if index == 0 {
                    PresentationSlot::warm_idle("slot-0").with_binding("route-1", "display-1")
                } else {
                    PresentationSlot::warm_idle(format!("slot-{index}")).with_binding(
                        format!("route-extra-{index}"),
                        format!("display-extra-{index}"),
                    )
                }
            })
            .collect();
        state.presentation_capacity = Some(
            PresentationCapacityAuthority::new(
                PresentationCapacityConfig {
                    warm_minimum: slot_count,
                    hard_maximum: slot_count,
                    human_priority_reserve: usize::from(slot_count > 0),
                    recovery_reserve: usize::from(slot_count > 1),
                    max_queue_depth: 8,
                },
                slots,
            )
            .unwrap(),
        );
        state
    }

    #[derive(Clone, Copy)]
    struct FakeSceneProbe {
        evidence: X11SceneEvidence,
    }

    impl FakeSceneProbe {
        fn ready() -> Self {
            Self {
                evidence: X11SceneEvidence {
                    active_window_owned: true,
                    topmost_window_owned: true,
                    authorized_geometry: true,
                    capture_region_unoccluded: true,
                    frame_width: 1280,
                    frame_height: 720,
                },
            }
        }
    }

    impl SceneProbe for FakeSceneProbe {
        fn observe(&self, _pid: u32, _display_name: &str) -> Result<X11SceneEvidence, String> {
            Ok(self.evidence)
        }
    }

    #[derive(Clone)]
    struct FakeSceneStager {
        calls: Arc<Mutex<Vec<&'static str>>>,
        stage_failure: Option<String>,
        restore_failure: Option<String>,
    }

    impl FakeSceneStager {
        fn ready() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                stage_failure: None,
                restore_failure: None,
            }
        }
    }

    impl SceneStager for FakeSceneStager {
        fn snapshot(&self, _pid: u32, _display_name: &str) -> Result<NativeSceneSnapshot, String> {
            self.calls.lock().unwrap().push("snapshot");
            Ok(NativeSceneSnapshot::Fixture("fixture-scene".to_string()))
        }

        fn stage(&self, snapshot: &NativeSceneSnapshot) -> Result<(), String> {
            self.calls.lock().unwrap().push("stage");
            assert!(
                matches!(snapshot, NativeSceneSnapshot::Fixture(value) if value == "fixture-scene")
            );
            self.stage_failure.clone().map_or(Ok(()), Err)
        }

        fn restore(&self, snapshot: &NativeSceneSnapshot) -> Result<(), String> {
            self.calls.lock().unwrap().push("restore");
            assert!(
                matches!(snapshot, NativeSceneSnapshot::Fixture(value) if value == "fixture-scene")
            );
            self.restore_failure.clone().map_or(Ok(()), Err)
        }
    }

    fn configured_scene_state(passive_viewer: bool, human_controller: bool) -> ServiceState {
        let stream = ViewStream {
            id: "stream-1".to_string(),
            provider: ViewStreamProvider::RdpGateway,
            route_id: Some("route-1".to_string()),
            display_allocation_id: Some("display-1".to_string()),
            readiness: Some(json!({
                "state": "ready",
                "displayContent": { "state": "browser_window_visible" }
            })),
            ..ViewStream::default()
        };
        let browser = BrowserProcess {
            id: "browser-1".to_string(),
            profile_id: Some("profile-1".to_string()),
            host: BrowserHost::RemoteHeaded,
            health: BrowserHealth::Ready,
            display_isolation: Some("private_virtual_display".to_string()),
            display_name: Some(":101".to_string()),
            display_allocation_id: Some("display-1".to_string()),
            pid: Some(4242),
            view_streams: vec![stream],
            active_session_ids: vec!["session-1".to_string()],
            ..BrowserProcess::default()
        };
        let display = DisplayAllocation {
            id: "display-1".to_string(),
            display_name: Some(":101".to_string()),
            display_isolation: "private_virtual_display".to_string(),
            owner_browser_id: Some("browser-1".to_string()),
            owner_session_id: Some("session-1".to_string()),
            profile_id: Some("profile-1".to_string()),
            host: Some(BrowserHost::RemoteHeaded),
            state: "ready".to_string(),
            route_ids: vec!["route-1".to_string()],
            readiness: Some(json!({ "state": "ready" })),
            ..DisplayAllocation::default()
        };
        let viewer_lease_ids = passive_viewer
            .then(|| vec!["viewer-1".to_string()])
            .unwrap_or_default();
        let route = RemoteViewRoute {
            id: "route-1".to_string(),
            provider: ViewStreamProvider::RdpGateway,
            display_allocation_id: Some("display-1".to_string()),
            browser_id: Some("browser-1".to_string()),
            session_id: Some("session-1".to_string()),
            state: "ready".to_string(),
            viewer_lease_ids: viewer_lease_ids.clone(),
            controller_lease_id: human_controller.then(|| "controller-1".to_string()),
            readiness: Some(json!({ "state": "ready" })),
            ..RemoteViewRoute::default()
        };
        let viewer_leases = passive_viewer
            .then(|| {
                BTreeMap::from([(
                    "viewer-1".to_string(),
                    ViewerLease {
                        id: "viewer-1".to_string(),
                        route_id: Some("route-1".to_string()),
                        browser_id: Some("browser-1".to_string()),
                        viewer_role: "observer".to_string(),
                        state: "active".to_string(),
                        ..ViewerLease::default()
                    },
                )])
            })
            .unwrap_or_default();
        let mut slot = PresentationSlot::warm_idle("slot-1").with_binding("route-1", "display-1");
        slot.state = PresentationSlotState::Reserved;
        slot.lease_request_id = Some("episode-1".to_string());
        slot.browser_id = Some("browser-1".to_string());
        slot.scene_generation = 7;
        ServiceState {
            browsers: BTreeMap::from([("browser-1".to_string(), browser)]),
            browser_process_identities: BTreeMap::from([(
                "browser-1".to_string(),
                ServiceBrowserProcessIdentity {
                    process_identity: RecordedProcessIdentity {
                        pid: 4242,
                        start_token: "start-9".to_string(),
                        executable_path: Some("/opt/chrome".to_string()),
                        browser_family: Some("chrome".to_string()),
                    },
                    user_data_dir: None,
                    runtime_profile: None,
                },
            )]),
            display_allocations: BTreeMap::from([("display-1".to_string(), display)]),
            remote_view_routes: BTreeMap::from([("route-1".to_string(), route)]),
            viewer_leases,
            presentation_capacity: Some(
                PresentationCapacityAuthority::new(
                    PresentationCapacityConfig {
                        warm_minimum: 1,
                        hard_maximum: 1,
                        human_priority_reserve: 0,
                        recovery_reserve: 0,
                        max_queue_depth: 8,
                    },
                    vec![slot],
                )
                .unwrap(),
            ),
            ..ServiceState::default()
        }
    }

    #[test]
    fn configured_adapter_commits_exact_reservation_and_release() {
        let repository = MemoryRepository::new(state(4));
        let mut adapter = ConfiguredPresentationSlotAdapter::new(repository, "episode-1", 4);

        let admission = adapter.reserve("browser-1").unwrap();
        assert_eq!(admission, "presentation-admission:episode-1:slot-0");
        let snapshot = adapter.repository.load_snapshot().unwrap();
        let slot = &snapshot.presentation_capacity.unwrap().slots[0];
        assert_eq!(slot.state, PresentationSlotState::Reserved);
        assert_eq!(slot.browser_id.as_deref(), Some("browser-1"));

        let release = adapter.release("browser-1").unwrap();
        assert_eq!(release, "presentation-release:episode-1:slot-0");
        let snapshot = adapter.repository.load_snapshot().unwrap();
        let slot = &snapshot.presentation_capacity.unwrap().slots[0];
        assert_eq!(slot.state, PresentationSlotState::WarmIdle);
        assert_eq!(slot.browser_id, None);
    }

    #[test]
    fn configured_paired_cdp_adapters_preserve_exact_browser_binding() {
        let provider = FakeCdpEpisodeProvider::new();
        let calls = provider.calls.clone();
        let target = ConfiguredCdpTarget::fixture("browser-1");
        let mut cdp = ConfiguredPairedCdpAdapter::with_provider(target.clone(), provider.clone());
        let mut trigger = ConfiguredCdpTriggerAdapter::with_provider(
            target,
            "#show-passkeys",
            "effect-1",
            provider,
        );

        assert_eq!(
            trigger.trigger("browser-1").unwrap(),
            "cdp-page-trigger:fixture"
        );
        assert_eq!(
            cdp.confirm_browser_external_absent(
                "browser-1",
                BrowserExternalSurface::PasskeyChooser
            )
            .unwrap(),
            "paired-cdp-absence:fixture"
        );
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                "trigger:browser-1:#show-passkeys:effect-1",
                "absence:browser-1:PasskeyChooser"
            ]
        );

        assert_eq!(
            trigger.trigger("browser-2").unwrap_err().code,
            "desktop_trigger_browser_binding_drift"
        );
        assert_eq!(
            cdp.confirm_browser_external_absent(
                "browser-2",
                BrowserExternalSurface::PasskeyChooser
            )
            .unwrap_err()
            .code,
            "desktop_cdp_browser_binding_drift"
        );
    }

    #[test]
    fn configured_adapter_does_not_persist_unresumable_queue_entries() {
        let repository = MemoryRepository::new(state(2));
        let mut adapter = ConfiguredPresentationSlotAdapter::new(repository, "episode-2", 2);

        let failure = adapter.reserve("browser-1").unwrap_err();
        assert_eq!(
            failure.code, "presentation_capacity_queued",
            "{}",
            failure.detail
        );
        let snapshot = adapter.repository.load_snapshot().unwrap();
        let capacity = snapshot.presentation_capacity.unwrap();
        assert!(capacity.queued_requests.is_empty());
        assert!(capacity
            .slots
            .iter()
            .all(|slot| slot.state == PresentationSlotState::WarmIdle));
    }

    #[test]
    fn configured_adapter_leases_and_releases_active_presentation_without_parking_browser() {
        let mut state = configured_scene_state(true, false);
        let slot = &mut state.presentation_capacity.as_mut().unwrap().slots[0];
        slot.state = PresentationSlotState::Active;
        slot.lease_request_id = None;
        slot.lease_priority = None;
        let repository = MemoryRepository::new(state);
        let mut adapter =
            ConfiguredPresentationSlotAdapter::new(repository.clone(), "episode-active", 1);

        let admission = adapter.reserve("browser-1").unwrap();
        assert_eq!(admission, "presentation-admission:episode-active:slot-1");
        let snapshot = repository.load_snapshot().unwrap();
        let slot = &snapshot.presentation_capacity.unwrap().slots[0];
        assert_eq!(slot.state, PresentationSlotState::Active);
        assert_eq!(slot.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(slot.lease_request_id.as_deref(), Some("episode-active"));

        let release = adapter.release("browser-1").unwrap();
        assert_eq!(release, "presentation-release:episode-active:slot-1");
        let snapshot = repository.load_snapshot().unwrap();
        let slot = &snapshot.presentation_capacity.unwrap().slots[0];
        assert_eq!(slot.state, PresentationSlotState::Active);
        assert_eq!(slot.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(slot.lease_request_id, None);
    }

    #[test]
    fn passive_viewer_allows_only_an_already_ready_unstaged_scene() {
        let repository = MemoryRepository::new(configured_scene_state(true, false));
        let mut adapter = ConfiguredWindowSemanticAdapter::with_probe(
            repository,
            "episode-1",
            FakeSceneProbe::ready(),
        );

        let unstaged = adapter.scene_admission("browser-1", false).unwrap();
        assert_eq!(
            DesktopEvidenceCoordinator::admit_scene(unstaged),
            SceneAdmission::CaptureAllowed
        );
        let staged = adapter.scene_admission("browser-1", true).unwrap();
        assert_eq!(
            DesktopEvidenceCoordinator::admit_scene(staged),
            SceneAdmission::WaitForViewer
        );
    }

    #[test]
    fn active_human_controller_blocks_before_capacity_or_scene_mutation() {
        let repository = MemoryRepository::new(configured_scene_state(false, true));
        let mut adapter = ConfiguredWindowSemanticAdapter::with_probe(
            repository,
            "episode-1",
            FakeSceneProbe::ready(),
        );

        let admission = adapter.scene_admission("browser-1", false).unwrap();
        assert_eq!(
            DesktopEvidenceCoordinator::admit_scene(admission),
            SceneAdmission::WaitForHumanController
        );
    }

    #[test]
    fn configured_capture_ready_proof_binds_process_route_display_slot_and_geometry() {
        let repository = MemoryRepository::new(configured_scene_state(true, false));
        let mut adapter = ConfiguredWindowSemanticAdapter::with_probe(
            repository,
            "episode-1",
            FakeSceneProbe::ready(),
        );

        let evidence = adapter.capture_ready("browser-1").unwrap();
        let proof = DesktopEvidenceCoordinator::prove_capture_ready(evidence).unwrap();

        assert_eq!(proof.browser_id, "browser-1");
        assert_eq!(proof.process_generation, "process:4242:start-9");
        assert_eq!(proof.route_id, "route-1");
        assert_eq!(proof.display_allocation_id, "display-1");
        assert_eq!(proof.presentation_slot_id, "slot-1");
        assert_eq!(proof.scene_generation, "scene:slot-1:7");
        assert_eq!(proof.frame_width, 1280);
        assert_eq!(proof.frame_height, 720);
        assert_eq!(proof.scale_factor_millis, 1000);
        assert_eq!(proof.viewer_posture, ViewerPosture::Passive);
        assert_eq!(proof.controller_posture, ControllerPosture::Uncontrolled);
    }

    #[test]
    fn configured_capture_ready_rejects_a_slot_bound_to_another_route() {
        let mut state = configured_scene_state(false, false);
        state.presentation_capacity.as_mut().unwrap().slots[0].route_id =
            Some("route-other".to_string());
        let repository = MemoryRepository::new(state);
        let mut adapter = ConfiguredWindowSemanticAdapter::with_probe(
            repository,
            "episode-1",
            FakeSceneProbe::ready(),
        );

        let failure = adapter.capture_ready("browser-1").unwrap_err();
        assert_eq!(failure.phase, "capture_ready");
        assert_eq!(failure.code, "desktop_scene_reservation_unavailable");
    }

    #[test]
    fn configured_frame_validation_rejects_geometry_drift() {
        let repository = MemoryRepository::new(configured_scene_state(false, false));
        let mut adapter = ConfiguredWindowSemanticAdapter::with_probe(
            repository,
            "episode-1",
            FakeSceneProbe::ready(),
        );
        let proof = DesktopEvidenceCoordinator::prove_capture_ready(
            adapter.capture_ready("browser-1").unwrap(),
        )
        .unwrap();
        let capture = DesktopCaptureResult {
            context: DesktopContext {
                context_id: "context-1".to_string(),
                schema_version: "v1",
                browser_id: "browser-1".to_string(),
                session_name: "session-1".to_string(),
                profile_id: Some("profile-1".to_string()),
                display_allocation_id: "display-1".to_string(),
                stream_id: "stream-1".to_string(),
                route_id: "route-1".to_string(),
                capture_provider: "x11_root",
                view_stream_provider: ViewStreamProvider::RdpGateway,
                display_isolation: "private_virtual_display".to_string(),
                coordinate_space: "desktop_physical_pixels",
                width: 1279,
                height: 720,
                scale_factor: 1.0,
                geometry_epoch: proof.geometry_epoch.clone(),
                resolved_at: "2026-08-24T00:00:00Z".to_string(),
                readiness: json!({ "state": "ready" }),
            },
            frame_receipt: FrameReceipt {
                frame_id: "frame-1".to_string(),
                schema_version: "v1",
                context_id: "context-1".to_string(),
                capture_provider: "x11_root",
                provider_version: "fixture-v1".to_string(),
                sequence: 1,
                captured_at: "2026-08-24T00:00:00Z".to_string(),
                width: 1279,
                height: 720,
                scale_factor: 1.0,
                geometry_epoch: proof.geometry_epoch.clone(),
                mime_type: "image/png",
                byte_length: 4,
                content_sha256: "fixture".to_string(),
                freshness: "fresh_capture",
                retention: "ephemeral",
                persisted: false,
            },
            image_bytes: vec![1, 2, 3, 4],
        };

        let failure = validate_configured_capture(&proof, &capture).unwrap_err();
        assert_eq!(failure.phase, "desktop_frame_capture");
        assert_eq!(failure.code, "desktop_frame_binding_drift");
    }

    #[test]
    fn configured_unstaged_scene_restoration_is_an_authority_checked_noop() {
        let repository = MemoryRepository::new(configured_scene_state(false, false));
        let mut adapter = ConfiguredSceneStagingAdapter::new(repository.clone(), "episode-1");

        let (before_generation, recorded) = adapter.snapshot("browser-1", false).unwrap();
        let current = adapter.current_authority("browser-1").unwrap();

        assert_eq!(before_generation, "scene:slot-1:7");
        assert_eq!(recorded, current);
        assert_eq!(
            adapter.restore("browser-1").unwrap(),
            "scene-restoration-noop:episode-1"
        );
        let snapshot = repository.load_snapshot().unwrap();
        assert_eq!(
            snapshot.presentation_capacity.unwrap().slots[0].state,
            PresentationSlotState::Reserved
        );
    }

    #[test]
    fn configured_staging_snapshots_stages_and_restores_the_exact_slot() {
        let repository = MemoryRepository::new(configured_scene_state(false, false));
        let stager = FakeSceneStager::ready();
        let calls = stager.calls.clone();
        let mut adapter =
            ConfiguredSceneStagingAdapter::with_stager(repository.clone(), "episode-1", stager);
        let (before_generation, recorded) = adapter.snapshot("browser-1", true).unwrap();

        let stage_receipt = adapter.stage("browser-1").unwrap();
        let current = adapter.current_authority("browser-1").unwrap();
        assert_eq!(before_generation, "scene:slot-1:7");
        assert_eq!(recorded, current);
        assert_eq!(stage_receipt, "scene-stage:episode-1");
        let staged = repository.load_snapshot().unwrap();
        let slot = &staged.presentation_capacity.unwrap().slots[0];
        assert_eq!(slot.state, PresentationSlotState::Staging);
        assert_eq!(slot.scene_generation, 8);
        assert!(slot.restoration_pending);

        assert_eq!(
            adapter.restore("browser-1").unwrap(),
            "scene-restoration:episode-1"
        );
        let restored = repository.load_snapshot().unwrap();
        let slot = &restored.presentation_capacity.unwrap().slots[0];
        assert_eq!(slot.state, PresentationSlotState::Reserved);
        assert_eq!(slot.scene_generation, 8);
        assert!(!slot.restoration_pending);
        assert_eq!(slot.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &["snapshot", "stage", "restore"]
        );
    }

    #[test]
    fn configured_staging_refuses_to_rearrange_a_shared_display() {
        let mut state = configured_scene_state(false, false);
        state
            .display_allocations
            .get_mut("display-1")
            .unwrap()
            .display_isolation = "shared_display".to_string();
        state
            .browsers
            .get_mut("browser-1")
            .unwrap()
            .display_isolation = Some("shared_display".to_string());
        let repository = MemoryRepository::new(state);
        let stager = FakeSceneStager::ready();
        let calls = stager.calls.clone();
        let mut adapter =
            ConfiguredSceneStagingAdapter::with_stager(repository, "episode-1", stager);
        adapter.snapshot("browser-1", true).unwrap();

        let failure = adapter.stage("browser-1").unwrap_err();

        assert_eq!(
            failure.code,
            "desktop_scene_staging_requires_private_display"
        );
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn unverified_native_restoration_quarantines_the_exact_slot() {
        let repository = MemoryRepository::new(configured_scene_state(false, false));
        let mut stager = FakeSceneStager::ready();
        stager.restore_failure = Some("restore did not converge".to_string());
        let mut adapter =
            ConfiguredSceneStagingAdapter::with_stager(repository.clone(), "episode-1", stager);
        adapter.snapshot("browser-1", true).unwrap();
        adapter.stage("browser-1").unwrap();

        let failure = adapter.restore("browser-1").unwrap_err();

        assert_eq!(failure.code, "desktop_scene_native_restoration_failed");
        let state = repository.load_snapshot().unwrap();
        let slot = &state.presentation_capacity.unwrap().slots[0];
        assert_eq!(slot.state, PresentationSlotState::Quarantined);
        assert_eq!(
            slot.cleanup_obligation_ids,
            vec!["scene-restoration:episode-1:native_restoration_unverified"]
        );
    }

    #[test]
    fn configured_verification_rechecks_the_reserved_scene_generation() {
        let repository = MemoryRepository::new(configured_scene_state(false, false));
        let mut adapter = ConfiguredEpisodeVerificationAdapter::new(repository, "episode-1");

        let (receipt_id, scene_generation) = adapter.verify("browser-1").unwrap();

        assert_eq!(receipt_id, "episode-verification:episode-1:scene:slot-1:7");
        assert_eq!(scene_generation, "scene:slot-1:7");
    }
}
