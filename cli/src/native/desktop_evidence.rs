//! Evidence selection and bounded desktop evidence episode authority.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PageEvidenceSurface {
    Dom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BrowserExternalSurface {
    PasskeyChooser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DiagnosticFailure {
    CdpTimeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HumanContinuationSurface {
    Biometric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SupportedCdpSurface {
    JavaScriptDialog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceRequest {
    Page(PageEvidenceSurface),
    BrowserExternal {
        surface: BrowserExternalSurface,
        stage_before_trigger: bool,
    },
    DiagnosticFailure(DiagnosticFailure),
    HumanOnly(HumanContinuationSurface),
    SupportedCdp(SupportedCdpSurface),
}

impl EvidenceRequest {
    pub(crate) fn page(surface: PageEvidenceSurface) -> Self {
        Self::Page(surface)
    }

    pub(crate) fn browser_external(
        surface: BrowserExternalSurface,
        stage_before_trigger: bool,
    ) -> Self {
        Self::BrowserExternal {
            surface,
            stage_before_trigger,
        }
    }

    pub(crate) fn diagnostic_failure(failure: DiagnosticFailure) -> Self {
        Self::DiagnosticFailure(failure)
    }

    pub(crate) fn human_only(surface: HumanContinuationSurface) -> Self {
        Self::HumanOnly(surface)
    }

    pub(crate) fn supported_cdp(surface: SupportedCdpSurface) -> Self {
        Self::SupportedCdp(surface)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceOutcome {
    Cdp,
    DesktopEvidenceEpisode,
    DiagnosticFailure,
    HumanContinuation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceDecisionReason {
    PageEvidenceAvailableThroughCdp,
    BrowserExternalSurfaceRequiresDesktop,
    DiagnosticFailureDoesNotAuthorizeDesktop,
    SensitiveSurfaceRequiresHumanContinuation,
    SupportedCdpMechanism,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceDecision {
    pub(crate) outcome: EvidenceOutcome,
    pub(crate) presentation_slot_required: bool,
    pub(crate) paired_page_absence_required: bool,
    pub(crate) stage_before_trigger: bool,
    pub(crate) reason: EvidenceDecisionReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ViewerPosture {
    None,
    Passive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ControllerPosture {
    Uncontrolled,
    Automated,
    Human,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureRegion {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaptureReadyEvidence {
    browser_id: String,
    process_generation: String,
    route_id: String,
    display_allocation_id: String,
    presentation_slot_id: String,
    scene_generation: String,
    geometry_epoch: String,
    frame_width: u32,
    frame_height: u32,
    scale_factor_millis: u32,
    capture_region: CaptureRegion,
    coordinate_space: String,
    active_window_owned: bool,
    topmost_window_owned: bool,
    authorized_geometry: bool,
    capture_region_unoccluded: bool,
    frame_mapping_current: bool,
    viewer_posture: ViewerPosture,
    controller_posture: ControllerPosture,
    proof_age_ms: u64,
    maximum_age_ms: u64,
}

impl CaptureReadyEvidence {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        browser_id: impl Into<String>,
        process_generation: impl Into<String>,
        route_id: impl Into<String>,
        display_allocation_id: impl Into<String>,
        presentation_slot_id: impl Into<String>,
        scene_generation: impl Into<String>,
        geometry_epoch: impl Into<String>,
        frame_width: u32,
        frame_height: u32,
        scale_factor_millis: u32,
        active_window_owned: bool,
        topmost_window_owned: bool,
        authorized_geometry: bool,
        capture_region_unoccluded: bool,
        viewer_posture: ViewerPosture,
        controller_posture: ControllerPosture,
        proof_age_ms: u64,
        maximum_age_ms: u64,
    ) -> Self {
        Self {
            browser_id: browser_id.into(),
            process_generation: process_generation.into(),
            route_id: route_id.into(),
            display_allocation_id: display_allocation_id.into(),
            presentation_slot_id: presentation_slot_id.into(),
            scene_generation: scene_generation.into(),
            geometry_epoch: geometry_epoch.into(),
            frame_width,
            frame_height,
            scale_factor_millis,
            capture_region: CaptureRegion {
                x: 0,
                y: 0,
                width: frame_width,
                height: frame_height,
            },
            coordinate_space: "desktop_physical_pixels".to_string(),
            active_window_owned,
            topmost_window_owned,
            authorized_geometry,
            capture_region_unoccluded,
            frame_mapping_current: true,
            viewer_posture,
            controller_posture,
            proof_age_ms,
            maximum_age_ms,
        }
    }

    #[cfg(test)]
    fn complete(scene_generation: impl Into<String>, geometry_epoch: impl Into<String>) -> Self {
        Self::new(
            "browser-1",
            "process-2",
            "route-4",
            "display-allocation-4",
            "slot-4",
            scene_generation,
            geometry_epoch,
            1280,
            720,
            1000,
            true,
            true,
            true,
            true,
            ViewerPosture::Passive,
            ControllerPosture::Uncontrolled,
            50,
            500,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureReadyProof {
    pub(crate) browser_id: String,
    pub(crate) process_generation: String,
    pub(crate) route_id: String,
    pub(crate) display_allocation_id: String,
    pub(crate) presentation_slot_id: String,
    pub(crate) scene_generation: String,
    pub(crate) geometry_epoch: String,
    pub(crate) frame_width: u32,
    pub(crate) frame_height: u32,
    pub(crate) scale_factor_millis: u32,
    pub(crate) capture_region: CaptureRegion,
    pub(crate) coordinate_space: String,
    pub(crate) viewer_posture: ViewerPosture,
    pub(crate) controller_posture: ControllerPosture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CaptureReadinessFailure {
    MissingIdentity,
    WindowNotActive,
    WindowNotTopmost,
    GeometryNotAuthorized,
    CaptureRegionOccluded,
    FrameMappingStale,
    ProofStale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SceneAdmissionRequest {
    pub(crate) viewer_posture: ViewerPosture,
    pub(crate) controller_posture: ControllerPosture,
    pub(crate) requires_staging: bool,
    pub(crate) capture_ready: bool,
    pub(crate) explicit_takeover: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SceneAdmission {
    CaptureAllowed,
    StagingAllowed,
    WaitForViewer,
    WaitForHumanController,
    NotCaptureReady,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RestorationAuthority {
    scene_generation: String,
    route_generation: String,
    controller_generation: String,
}

impl RestorationAuthority {
    pub(crate) fn new(
        scene_generation: impl Into<String>,
        route_generation: impl Into<String>,
        controller_generation: impl Into<String>,
    ) -> Self {
        Self {
            scene_generation: scene_generation.into(),
            route_generation: route_generation.into(),
            controller_generation: controller_generation.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RestorationDecision {
    Restore,
    CancelledAuthorityDrift,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopEvidenceEpisodeReceipt {
    pub(crate) episode_id: String,
    pub(crate) evidence_decision: EvidenceDecision,
    pub(crate) admission_receipt_id: String,
    pub(crate) before_scene_generation: String,
    pub(crate) stage_receipt_id: Option<String>,
    pub(crate) page_absence_receipt_id: String,
    pub(crate) trigger_receipt_id: String,
    pub(crate) capture_proof: CaptureReadyProof,
    pub(crate) capture_receipt_id: String,
    pub(crate) input_receipt_id: Option<String>,
    pub(crate) verification_receipt_id: String,
    pub(crate) after_capture_proof: CaptureReadyProof,
    pub(crate) after_scene_generation: String,
    pub(crate) restoration_decision: RestorationDecision,
    pub(crate) slot_release_receipt_id: String,
    pub(crate) cleanup_receipt_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopEvidenceReleaseFailureReceipt {
    pub(crate) episode_id: String,
    pub(crate) admission_receipt_id: String,
    pub(crate) failure: DesktopEpisodeAdmissionFailure,
    pub(crate) cleanup_receipt_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopEpisodeRequest {
    pub(crate) episode_id: String,
    pub(crate) browser_id: String,
    pub(crate) evidence: EvidenceRequest,
    pub(crate) input: EpisodeInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EpisodeInput {
    None,
    Authorized { authority_receipt_id: String },
    ConfiguredProduction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CdpEvidenceReceipt {
    pub(crate) receipt_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HumanContinuationReceipt {
    pub(crate) handoff_receipt_id: String,
    pub(crate) reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub(crate) enum DesktopEpisodeOutcome {
    Cdp {
        evidence: CdpEvidenceReceipt,
    },
    Desktop {
        receipt: Box<DesktopEvidenceEpisodeReceipt>,
    },
    HumanContinuation {
        receipt: HumanContinuationReceipt,
    },
    DiagnosticFailure {
        reason: EvidenceDecisionReason,
    },
    AdmissionUnavailable {
        failure: DesktopEpisodeAdmissionFailure,
    },
    AdapterUnavailable {
        failure: DesktopEpisodeAdapterFailure,
    },
    ReleaseFailed {
        receipt: DesktopEvidenceReleaseFailureReceipt,
    },
    Aborted {
        receipt: DesktopEvidenceTerminalReceipt,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopEpisodeAdmissionFailure {
    pub(crate) code: String,
    pub(crate) detail: String,
}

impl DesktopEpisodeAdmissionFailure {
    pub(crate) fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopEpisodeAdapterFailure {
    pub(crate) phase: &'static str,
    pub(crate) code: String,
    pub(crate) detail: String,
}

impl DesktopEpisodeAdapterFailure {
    pub(crate) fn new(
        phase: &'static str,
        code: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            phase,
            code: code.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DesktopEpisodeFailure {
    CaptureReadiness(CaptureReadinessFailure),
    CaptureBindingDrift,
    Adapter(DesktopEpisodeAdapterFailure),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopEvidenceTerminalReceipt {
    pub(crate) episode_id: String,
    pub(crate) evidence_decision: EvidenceDecision,
    pub(crate) admission_receipt_id: String,
    pub(crate) before_scene_generation: String,
    pub(crate) stage_receipt_id: Option<String>,
    pub(crate) failure: DesktopEpisodeFailure,
    pub(crate) restoration_decision: RestorationDecision,
    pub(crate) slot_release_receipt_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) release_failure: Option<DesktopEpisodeAdmissionFailure>,
    pub(crate) cleanup_receipt_id: String,
}

pub(crate) trait CdpEvidenceAdapter {
    fn collect_page(&mut self, browser_id: &str) -> CdpEvidenceReceipt;
    fn confirm_browser_external_absent(&mut self, browser_id: &str) -> String;
}

pub(crate) trait PresentationSlotAdapter {
    fn reserve(&mut self, browser_id: &str) -> Result<String, DesktopEpisodeAdmissionFailure>;
    fn release(&mut self, browser_id: &str) -> Result<String, DesktopEpisodeAdmissionFailure>;
}

pub(crate) trait SceneStagingAdapter {
    fn snapshot(&mut self, browser_id: &str) -> (String, RestorationAuthority);
    fn stage(&mut self, browser_id: &str) -> String;
    fn current_authority(&mut self, browser_id: &str) -> RestorationAuthority;
    fn restore(&mut self, browser_id: &str) -> String;
}

pub(crate) trait WindowSemanticAdapter {
    fn scene_admission(
        &mut self,
        browser_id: &str,
        requires_staging: bool,
    ) -> Result<SceneAdmissionRequest, DesktopEpisodeAdapterFailure>;
    fn capture_ready(
        &mut self,
        browser_id: &str,
    ) -> Result<CaptureReadyEvidence, DesktopEpisodeAdapterFailure>;
}

pub(crate) trait ExternalUiTriggerAdapter {
    fn trigger(&mut self, browser_id: &str) -> String;
}

pub(crate) trait DesktopFrameAdapter {
    fn capture(
        &mut self,
        browser_id: &str,
        proof: &CaptureReadyProof,
    ) -> Result<String, DesktopEpisodeAdapterFailure>;
}

pub(crate) trait DesktopInputAdapter {
    fn apply(&mut self, browser_id: &str, authority_receipt_id: &str) -> String;
}

pub(crate) trait EpisodeVerificationAdapter {
    fn verify(&mut self, browser_id: &str) -> (String, String);
}

pub(crate) trait HumanHandoffAdapter {
    fn prepare(&mut self, browser_id: &str, reason: &'static str) -> String;
}

pub(crate) trait EpisodeCleanupAdapter {
    fn complete(&mut self, episode_id: &str) -> String;
}

pub(crate) struct DesktopEpisodeAdapters<'a> {
    pub(crate) cdp: &'a mut dyn CdpEvidenceAdapter,
    pub(crate) slots: &'a mut dyn PresentationSlotAdapter,
    pub(crate) staging: &'a mut dyn SceneStagingAdapter,
    pub(crate) windows: &'a mut dyn WindowSemanticAdapter,
    pub(crate) trigger: &'a mut dyn ExternalUiTriggerAdapter,
    pub(crate) frames: &'a mut dyn DesktopFrameAdapter,
    pub(crate) input: &'a mut dyn DesktopInputAdapter,
    pub(crate) verification: &'a mut dyn EpisodeVerificationAdapter,
    pub(crate) handoff: &'a mut dyn HumanHandoffAdapter,
    pub(crate) cleanup: &'a mut dyn EpisodeCleanupAdapter,
}

pub(crate) struct DesktopEvidenceCoordinator;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopEvidencePolicyProjection {
    pub(crate) schema_version: &'static str,
    pub(crate) page_evidence_transport: &'static str,
    pub(crate) desktop_evidence_surfaces: Vec<&'static str>,
    pub(crate) human_continuation_surfaces: Vec<&'static str>,
    pub(crate) capture_ready_proof_required: bool,
    pub(crate) paired_page_absence_required: bool,
    pub(crate) generic_cdp_failure_authorizes_desktop: bool,
    pub(crate) configured_production_input: &'static str,
}

impl DesktopEvidenceCoordinator {
    pub(crate) fn policy_projection() -> DesktopEvidencePolicyProjection {
        DesktopEvidencePolicyProjection {
            schema_version: "agent-browser.desktop-evidence-policy.v1",
            page_evidence_transport: "cdp",
            desktop_evidence_surfaces: vec![
                "browser_chrome",
                "extension_ui",
                "password_manager_prompt",
                "passkey_chooser",
                "native_dialog",
                "os_window",
                "stacking_or_occlusion",
            ],
            human_continuation_surfaces: vec![
                "biometric",
                "secure_desktop",
                "pin",
                "master_password",
                "consent",
            ],
            capture_ready_proof_required: true,
            paired_page_absence_required: true,
            generic_cdp_failure_authorizes_desktop: false,
            configured_production_input: "unavailable_pending_plan_0110",
        }
    }

    pub(crate) fn run(
        request: DesktopEpisodeRequest,
        adapters: &mut DesktopEpisodeAdapters<'_>,
    ) -> DesktopEpisodeOutcome {
        let decision = Self::decide(request.evidence);
        match decision.outcome {
            EvidenceOutcome::Cdp => {
                return DesktopEpisodeOutcome::Cdp {
                    evidence: adapters.cdp.collect_page(&request.browser_id),
                };
            }
            EvidenceOutcome::HumanContinuation => {
                return Self::human_continuation(
                    &request.browser_id,
                    "sensitive_surface_requires_human_continuation",
                    adapters,
                );
            }
            EvidenceOutcome::DiagnosticFailure => {
                return DesktopEpisodeOutcome::DiagnosticFailure {
                    reason: decision.reason,
                };
            }
            EvidenceOutcome::DesktopEvidenceEpisode => {}
        }
        if request.input == EpisodeInput::ConfiguredProduction {
            return Self::human_continuation(
                &request.browser_id,
                "configured_production_input_unavailable",
                adapters,
            );
        }
        let scene_admission = match adapters
            .windows
            .scene_admission(&request.browser_id, decision.stage_before_trigger)
        {
            Ok(admission) => admission,
            Err(failure) => return DesktopEpisodeOutcome::AdapterUnavailable { failure },
        };
        match Self::admit_scene(scene_admission) {
            SceneAdmission::WaitForHumanController => {
                return Self::human_continuation(
                    &request.browser_id,
                    "human_controller_has_precedence",
                    adapters,
                );
            }
            SceneAdmission::WaitForViewer => {
                return Self::human_continuation(
                    &request.browser_id,
                    "passive_viewer_blocks_scene_rearrangement",
                    adapters,
                );
            }
            SceneAdmission::CaptureAllowed
            | SceneAdmission::StagingAllowed
            | SceneAdmission::NotCaptureReady => {}
        }

        let admission_receipt_id = match adapters.slots.reserve(&request.browser_id) {
            Ok(receipt_id) => receipt_id,
            Err(failure) => return DesktopEpisodeOutcome::AdmissionUnavailable { failure },
        };
        let (before_scene_generation, restoration_authority) =
            adapters.staging.snapshot(&request.browser_id);
        let stage_receipt_id = decision
            .stage_before_trigger
            .then(|| adapters.staging.stage(&request.browser_id));
        let page_absence_receipt_id = adapters
            .cdp
            .confirm_browser_external_absent(&request.browser_id);
        let trigger_receipt_id = adapters.trigger.trigger(&request.browser_id);

        let capture_proof = match adapters.windows.capture_ready(&request.browser_id) {
            Err(failure) => {
                return Self::abort_after_reservation(
                    &request,
                    decision,
                    admission_receipt_id,
                    before_scene_generation,
                    stage_receipt_id,
                    DesktopEpisodeFailure::Adapter(failure),
                    &restoration_authority,
                    true,
                    adapters,
                );
            }
            Ok(evidence) => match Self::prove_capture_ready(evidence) {
                Ok(proof) => proof,
                Err(failure) => {
                    return Self::abort_after_reservation(
                        &request,
                        decision,
                        admission_receipt_id,
                        before_scene_generation,
                        stage_receipt_id,
                        DesktopEpisodeFailure::CaptureReadiness(failure),
                        &restoration_authority,
                        true,
                        adapters,
                    );
                }
            },
        };
        let capture_receipt_id = match adapters.frames.capture(&request.browser_id, &capture_proof)
        {
            Ok(receipt_id) => receipt_id,
            Err(failure) => {
                return Self::abort_after_reservation(
                    &request,
                    decision,
                    admission_receipt_id,
                    before_scene_generation,
                    stage_receipt_id,
                    DesktopEpisodeFailure::Adapter(failure),
                    &restoration_authority,
                    true,
                    adapters,
                );
            }
        };
        let input_receipt_id = match &request.input {
            EpisodeInput::None => None,
            EpisodeInput::Authorized {
                authority_receipt_id,
            } => Some(
                adapters
                    .input
                    .apply(&request.browser_id, authority_receipt_id),
            ),
            EpisodeInput::ConfiguredProduction => unreachable!(),
        };
        let (verification_receipt_id, after_scene_generation) =
            adapters.verification.verify(&request.browser_id);
        let after_capture_proof = match adapters.windows.capture_ready(&request.browser_id) {
            Err(failure) => {
                return Self::abort_after_reservation(
                    &request,
                    decision,
                    admission_receipt_id,
                    before_scene_generation,
                    stage_receipt_id,
                    DesktopEpisodeFailure::Adapter(failure),
                    &restoration_authority,
                    false,
                    adapters,
                );
            }
            Ok(evidence) => match Self::prove_capture_ready(evidence) {
                Ok(proof) if Self::same_capture_binding(&capture_proof, &proof) => proof,
                Ok(_) => {
                    return Self::abort_after_reservation(
                        &request,
                        decision,
                        admission_receipt_id,
                        before_scene_generation,
                        stage_receipt_id,
                        DesktopEpisodeFailure::CaptureBindingDrift,
                        &restoration_authority,
                        false,
                        adapters,
                    );
                }
                Err(failure) => {
                    return Self::abort_after_reservation(
                        &request,
                        decision,
                        admission_receipt_id,
                        before_scene_generation,
                        stage_receipt_id,
                        DesktopEpisodeFailure::CaptureReadiness(failure),
                        &restoration_authority,
                        false,
                        adapters,
                    );
                }
            },
        };
        let current_authority = adapters.staging.current_authority(&request.browser_id);
        let restoration_decision =
            Self::authorize_restoration(&restoration_authority, &current_authority);
        if restoration_decision == RestorationDecision::Restore {
            adapters.staging.restore(&request.browser_id);
        }
        let slot_release_receipt_id = match adapters.slots.release(&request.browser_id) {
            Ok(receipt_id) => receipt_id,
            Err(failure) => {
                return DesktopEpisodeOutcome::ReleaseFailed {
                    receipt: DesktopEvidenceReleaseFailureReceipt {
                        episode_id: request.episode_id.clone(),
                        admission_receipt_id,
                        failure,
                        cleanup_receipt_id: adapters.cleanup.complete(&request.episode_id),
                    },
                };
            }
        };
        let cleanup_receipt_id = adapters.cleanup.complete(&request.episode_id);

        DesktopEpisodeOutcome::Desktop {
            receipt: Box::new(DesktopEvidenceEpisodeReceipt {
                episode_id: request.episode_id,
                evidence_decision: decision,
                admission_receipt_id,
                before_scene_generation,
                stage_receipt_id,
                page_absence_receipt_id,
                trigger_receipt_id,
                capture_proof,
                capture_receipt_id,
                input_receipt_id,
                verification_receipt_id,
                after_capture_proof,
                after_scene_generation,
                restoration_decision,
                slot_release_receipt_id,
                cleanup_receipt_id,
            }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn abort_after_reservation(
        request: &DesktopEpisodeRequest,
        decision: EvidenceDecision,
        admission_receipt_id: String,
        before_scene_generation: String,
        stage_receipt_id: Option<String>,
        failure: DesktopEpisodeFailure,
        restoration_authority: &RestorationAuthority,
        restoration_permitted: bool,
        adapters: &mut DesktopEpisodeAdapters<'_>,
    ) -> DesktopEpisodeOutcome {
        let restoration_decision = if restoration_permitted {
            let current = adapters.staging.current_authority(&request.browser_id);
            Self::authorize_restoration(restoration_authority, &current)
        } else {
            RestorationDecision::CancelledAuthorityDrift
        };
        if restoration_decision == RestorationDecision::Restore {
            adapters.staging.restore(&request.browser_id);
        }
        let release = adapters.slots.release(&request.browser_id);
        DesktopEpisodeOutcome::Aborted {
            receipt: DesktopEvidenceTerminalReceipt {
                episode_id: request.episode_id.clone(),
                evidence_decision: decision,
                admission_receipt_id,
                before_scene_generation,
                stage_receipt_id,
                failure,
                restoration_decision,
                slot_release_receipt_id: release.as_ref().ok().cloned(),
                release_failure: release.err(),
                cleanup_receipt_id: adapters.cleanup.complete(&request.episode_id),
            },
        }
    }

    fn human_continuation(
        browser_id: &str,
        reason: &'static str,
        adapters: &mut DesktopEpisodeAdapters<'_>,
    ) -> DesktopEpisodeOutcome {
        DesktopEpisodeOutcome::HumanContinuation {
            receipt: HumanContinuationReceipt {
                handoff_receipt_id: adapters.handoff.prepare(browser_id, reason),
                reason,
            },
        }
    }

    fn same_capture_binding(before: &CaptureReadyProof, after: &CaptureReadyProof) -> bool {
        before.browser_id == after.browser_id
            && before.process_generation == after.process_generation
            && before.route_id == after.route_id
            && before.display_allocation_id == after.display_allocation_id
            && before.presentation_slot_id == after.presentation_slot_id
            && before.scene_generation == after.scene_generation
            && before.geometry_epoch == after.geometry_epoch
            && before.frame_width == after.frame_width
            && before.frame_height == after.frame_height
            && before.scale_factor_millis == after.scale_factor_millis
            && before.capture_region == after.capture_region
            && before.coordinate_space == after.coordinate_space
    }

    pub(crate) fn decide(request: EvidenceRequest) -> EvidenceDecision {
        match request {
            EvidenceRequest::Page(_) => EvidenceDecision {
                outcome: EvidenceOutcome::Cdp,
                presentation_slot_required: false,
                paired_page_absence_required: false,
                stage_before_trigger: false,
                reason: EvidenceDecisionReason::PageEvidenceAvailableThroughCdp,
            },
            EvidenceRequest::BrowserExternal {
                surface: _,
                stage_before_trigger,
            } => EvidenceDecision {
                outcome: EvidenceOutcome::DesktopEvidenceEpisode,
                presentation_slot_required: true,
                paired_page_absence_required: true,
                stage_before_trigger,
                reason: EvidenceDecisionReason::BrowserExternalSurfaceRequiresDesktop,
            },
            EvidenceRequest::DiagnosticFailure(_) => EvidenceDecision {
                outcome: EvidenceOutcome::DiagnosticFailure,
                presentation_slot_required: false,
                paired_page_absence_required: false,
                stage_before_trigger: false,
                reason: EvidenceDecisionReason::DiagnosticFailureDoesNotAuthorizeDesktop,
            },
            EvidenceRequest::HumanOnly(_) => EvidenceDecision {
                outcome: EvidenceOutcome::HumanContinuation,
                presentation_slot_required: false,
                paired_page_absence_required: false,
                stage_before_trigger: false,
                reason: EvidenceDecisionReason::SensitiveSurfaceRequiresHumanContinuation,
            },
            EvidenceRequest::SupportedCdp(_) => EvidenceDecision {
                outcome: EvidenceOutcome::Cdp,
                presentation_slot_required: false,
                paired_page_absence_required: false,
                stage_before_trigger: false,
                reason: EvidenceDecisionReason::SupportedCdpMechanism,
            },
        }
    }

    pub(crate) fn prove_capture_ready(
        evidence: CaptureReadyEvidence,
    ) -> Result<CaptureReadyProof, CaptureReadinessFailure> {
        if [
            evidence.browser_id.as_str(),
            evidence.process_generation.as_str(),
            evidence.route_id.as_str(),
            evidence.display_allocation_id.as_str(),
            evidence.presentation_slot_id.as_str(),
            evidence.scene_generation.as_str(),
            evidence.geometry_epoch.as_str(),
        ]
        .contains(&"")
        {
            return Err(CaptureReadinessFailure::MissingIdentity);
        }
        if !evidence.active_window_owned {
            return Err(CaptureReadinessFailure::WindowNotActive);
        }
        if !evidence.topmost_window_owned {
            return Err(CaptureReadinessFailure::WindowNotTopmost);
        }
        if !evidence.authorized_geometry {
            return Err(CaptureReadinessFailure::GeometryNotAuthorized);
        }
        if !evidence.capture_region_unoccluded {
            return Err(CaptureReadinessFailure::CaptureRegionOccluded);
        }
        if !evidence.frame_mapping_current {
            return Err(CaptureReadinessFailure::FrameMappingStale);
        }
        if evidence.frame_width == 0
            || evidence.frame_height == 0
            || evidence.scale_factor_millis == 0
            || evidence.capture_region.x != 0
            || evidence.capture_region.y != 0
            || evidence.capture_region.width != evidence.frame_width
            || evidence.capture_region.height != evidence.frame_height
            || evidence.coordinate_space != "desktop_physical_pixels"
        {
            return Err(CaptureReadinessFailure::FrameMappingStale);
        }
        if evidence.proof_age_ms > evidence.maximum_age_ms {
            return Err(CaptureReadinessFailure::ProofStale);
        }
        Ok(CaptureReadyProof {
            browser_id: evidence.browser_id,
            process_generation: evidence.process_generation,
            route_id: evidence.route_id,
            display_allocation_id: evidence.display_allocation_id,
            presentation_slot_id: evidence.presentation_slot_id,
            scene_generation: evidence.scene_generation,
            geometry_epoch: evidence.geometry_epoch,
            frame_width: evidence.frame_width,
            frame_height: evidence.frame_height,
            scale_factor_millis: evidence.scale_factor_millis,
            capture_region: evidence.capture_region,
            coordinate_space: evidence.coordinate_space,
            viewer_posture: evidence.viewer_posture,
            controller_posture: evidence.controller_posture,
        })
    }

    pub(crate) fn admit_scene(request: SceneAdmissionRequest) -> SceneAdmission {
        if request.controller_posture == ControllerPosture::Human && !request.explicit_takeover {
            return SceneAdmission::WaitForHumanController;
        }
        if request.viewer_posture == ViewerPosture::Passive && request.requires_staging {
            return SceneAdmission::WaitForViewer;
        }
        if request.capture_ready {
            return SceneAdmission::CaptureAllowed;
        }
        if request.requires_staging {
            return SceneAdmission::StagingAllowed;
        }
        SceneAdmission::NotCaptureReady
    }

    pub(crate) fn authorize_restoration(
        recorded: &RestorationAuthority,
        current: &RestorationAuthority,
    ) -> RestorationDecision {
        if recorded == current {
            RestorationDecision::Restore
        } else {
            RestorationDecision::CancelledAuthorityDrift
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    type Log = Rc<RefCell<Vec<&'static str>>>;

    struct FakeCdp(Log);
    impl CdpEvidenceAdapter for FakeCdp {
        fn collect_page(&mut self, _browser_id: &str) -> CdpEvidenceReceipt {
            self.0.borrow_mut().push("cdp");
            CdpEvidenceReceipt {
                receipt_id: "cdp-1".to_string(),
            }
        }
        fn confirm_browser_external_absent(&mut self, _browser_id: &str) -> String {
            self.0.borrow_mut().push("page_absence");
            "absence-1".to_string()
        }
    }

    struct FakeSlots {
        log: Log,
        admission_failure: Option<DesktopEpisodeAdmissionFailure>,
        release_failure: Option<DesktopEpisodeAdmissionFailure>,
    }
    impl PresentationSlotAdapter for FakeSlots {
        fn reserve(&mut self, _browser_id: &str) -> Result<String, DesktopEpisodeAdmissionFailure> {
            self.log.borrow_mut().push("reserve");
            if let Some(failure) = self.admission_failure.clone() {
                return Err(failure);
            }
            Ok("admission-1".to_string())
        }
        fn release(&mut self, _browser_id: &str) -> Result<String, DesktopEpisodeAdmissionFailure> {
            self.log.borrow_mut().push("release");
            if let Some(failure) = self.release_failure.clone() {
                return Err(failure);
            }
            Ok("release-1".to_string())
        }
    }

    struct FakeStaging {
        log: Log,
        current: RestorationAuthority,
    }
    impl SceneStagingAdapter for FakeStaging {
        fn snapshot(&mut self, _browser_id: &str) -> (String, RestorationAuthority) {
            self.log.borrow_mut().push("snapshot");
            (
                "scene-before".to_string(),
                RestorationAuthority::new("scene-staged", "route-1", "control-1"),
            )
        }
        fn stage(&mut self, _browser_id: &str) -> String {
            self.log.borrow_mut().push("stage");
            "stage-1".to_string()
        }
        fn current_authority(&mut self, _browser_id: &str) -> RestorationAuthority {
            self.log.borrow_mut().push("current_authority");
            self.current.clone()
        }
        fn restore(&mut self, _browser_id: &str) -> String {
            self.log.borrow_mut().push("restore");
            "restore-1".to_string()
        }
    }

    struct FakeWindows {
        log: Log,
        evidence: VecDeque<CaptureReadyEvidence>,
        admission: SceneAdmissionRequest,
        scene_failure: Option<DesktopEpisodeAdapterFailure>,
        capture_failure: Option<DesktopEpisodeAdapterFailure>,
    }
    impl WindowSemanticAdapter for FakeWindows {
        fn scene_admission(
            &mut self,
            _browser_id: &str,
            _requires_staging: bool,
        ) -> Result<SceneAdmissionRequest, DesktopEpisodeAdapterFailure> {
            self.log.borrow_mut().push("scene_admission");
            if let Some(failure) = self.scene_failure.clone() {
                return Err(failure);
            }
            Ok(self.admission)
        }
        fn capture_ready(
            &mut self,
            _browser_id: &str,
        ) -> Result<CaptureReadyEvidence, DesktopEpisodeAdapterFailure> {
            self.log.borrow_mut().push("capture_ready");
            if let Some(failure) = self.capture_failure.clone() {
                return Err(failure);
            }
            Ok(self.evidence.pop_front().unwrap())
        }
    }

    macro_rules! receipt_adapter {
        ($name:ident, $trait_name:ident, $method:ident, $event:literal, $receipt:literal) => {
            struct $name(Log);
            impl $trait_name for $name {
                fn $method(&mut self, _browser_id: &str) -> String {
                    self.0.borrow_mut().push($event);
                    $receipt.to_string()
                }
            }
        };
    }
    receipt_adapter!(
        FakeTrigger,
        ExternalUiTriggerAdapter,
        trigger,
        "trigger",
        "trigger-1"
    );

    struct FakeFrames {
        log: Log,
        failure: Option<DesktopEpisodeAdapterFailure>,
    }
    impl DesktopFrameAdapter for FakeFrames {
        fn capture(
            &mut self,
            _browser_id: &str,
            _proof: &CaptureReadyProof,
        ) -> Result<String, DesktopEpisodeAdapterFailure> {
            self.log.borrow_mut().push("capture");
            if let Some(failure) = self.failure.clone() {
                return Err(failure);
            }
            Ok("capture-1".to_string())
        }
    }
    struct FakeInput(Log);
    impl DesktopInputAdapter for FakeInput {
        fn apply(&mut self, _browser_id: &str, _authority_receipt_id: &str) -> String {
            self.0.borrow_mut().push("input");
            "input-1".to_string()
        }
    }
    struct FakeVerification(Log);
    impl EpisodeVerificationAdapter for FakeVerification {
        fn verify(&mut self, _browser_id: &str) -> (String, String) {
            self.0.borrow_mut().push("verify");
            ("verify-1".to_string(), "scene-after".to_string())
        }
    }
    struct FakeHandoff(Log);
    impl HumanHandoffAdapter for FakeHandoff {
        fn prepare(&mut self, _browser_id: &str, _reason: &'static str) -> String {
            self.0.borrow_mut().push("handoff");
            "handoff-1".to_string()
        }
    }
    struct FakeCleanup(Log);
    impl EpisodeCleanupAdapter for FakeCleanup {
        fn complete(&mut self, _episode_id: &str) -> String {
            self.0.borrow_mut().push("cleanup");
            "cleanup-1".to_string()
        }
    }

    struct Harness {
        log: Log,
        cdp: FakeCdp,
        slots: FakeSlots,
        staging: FakeStaging,
        windows: FakeWindows,
        trigger: FakeTrigger,
        frames: FakeFrames,
        input: FakeInput,
        verification: FakeVerification,
        handoff: FakeHandoff,
        cleanup: FakeCleanup,
    }

    impl Harness {
        fn new() -> Self {
            let log = Rc::new(RefCell::new(Vec::new()));
            Self {
                cdp: FakeCdp(log.clone()),
                slots: FakeSlots {
                    log: log.clone(),
                    admission_failure: None,
                    release_failure: None,
                },
                staging: FakeStaging {
                    log: log.clone(),
                    current: RestorationAuthority::new("scene-staged", "route-1", "control-1"),
                },
                windows: FakeWindows {
                    log: log.clone(),
                    admission: SceneAdmissionRequest {
                        viewer_posture: ViewerPosture::None,
                        controller_posture: ControllerPosture::Uncontrolled,
                        requires_staging: true,
                        capture_ready: false,
                        explicit_takeover: false,
                    },
                    scene_failure: None,
                    capture_failure: None,
                    evidence: VecDeque::from([
                        CaptureReadyEvidence::complete("scene-staged", "geometry-1"),
                        CaptureReadyEvidence::complete("scene-staged", "geometry-1"),
                    ]),
                },
                trigger: FakeTrigger(log.clone()),
                frames: FakeFrames {
                    log: log.clone(),
                    failure: None,
                },
                input: FakeInput(log.clone()),
                verification: FakeVerification(log.clone()),
                handoff: FakeHandoff(log.clone()),
                cleanup: FakeCleanup(log.clone()),
                log,
            }
        }

        fn run(&mut self, evidence: EvidenceRequest, input: EpisodeInput) -> DesktopEpisodeOutcome {
            let mut adapters = DesktopEpisodeAdapters {
                cdp: &mut self.cdp,
                slots: &mut self.slots,
                staging: &mut self.staging,
                windows: &mut self.windows,
                trigger: &mut self.trigger,
                frames: &mut self.frames,
                input: &mut self.input,
                verification: &mut self.verification,
                handoff: &mut self.handoff,
                cleanup: &mut self.cleanup,
            };
            DesktopEvidenceCoordinator::run(
                DesktopEpisodeRequest {
                    episode_id: "episode-1".to_string(),
                    browser_id: "browser-1".to_string(),
                    evidence,
                    input,
                },
                &mut adapters,
            )
        }
    }

    #[test]
    fn page_dom_uses_cdp_without_presentation_capacity() {
        let decision =
            DesktopEvidenceCoordinator::decide(EvidenceRequest::page(PageEvidenceSurface::Dom));

        assert_eq!(decision.outcome, EvidenceOutcome::Cdp);
        assert!(!decision.presentation_slot_required);
        assert_eq!(
            decision.reason,
            EvidenceDecisionReason::PageEvidenceAvailableThroughCdp
        );
    }

    #[test]
    fn policy_projection_is_redacted_and_preserves_the_independent_input_gate() {
        let value = serde_json::to_value(DesktopEvidenceCoordinator::policy_projection()).unwrap();
        assert_eq!(value["pageEvidenceTransport"], "cdp");
        assert_eq!(value["captureReadyProofRequired"], true);
        assert_eq!(value["genericCdpFailureAuthorizesDesktop"], false);
        assert_eq!(
            value["configuredProductionInput"],
            "unavailable_pending_plan_0110"
        );
        assert!(value.get("routeId").is_none());
        assert!(value.get("displayName").is_none());
    }

    #[test]
    fn page_evidence_executes_only_cdp_and_never_reserves_a_slot() {
        let mut harness = Harness::new();
        let outcome = harness.run(
            EvidenceRequest::page(PageEvidenceSurface::Dom),
            EpisodeInput::None,
        );

        assert!(matches!(outcome, DesktopEpisodeOutcome::Cdp { .. }));
        assert_eq!(*harness.log.borrow(), vec!["cdp"]);
    }

    #[test]
    fn browser_external_episode_stages_before_trigger_and_binds_every_phase() {
        let mut harness = Harness::new();
        let outcome = harness.run(
            EvidenceRequest::browser_external(BrowserExternalSurface::PasskeyChooser, true),
            EpisodeInput::None,
        );

        let DesktopEpisodeOutcome::Desktop { receipt } = outcome else {
            panic!("expected a desktop episode receipt");
        };
        assert_eq!(receipt.stage_receipt_id.as_deref(), Some("stage-1"));
        assert_eq!(receipt.page_absence_receipt_id, "absence-1");
        assert_eq!(receipt.capture_receipt_id, "capture-1");
        assert_eq!(receipt.verification_receipt_id, "verify-1");
        assert_eq!(receipt.slot_release_receipt_id, "release-1");
        assert_eq!(receipt.cleanup_receipt_id, "cleanup-1");
        assert_eq!(
            *harness.log.borrow(),
            vec![
                "scene_admission",
                "reserve",
                "snapshot",
                "stage",
                "page_absence",
                "trigger",
                "capture_ready",
                "capture",
                "verify",
                "capture_ready",
                "current_authority",
                "restore",
                "release",
                "cleanup",
            ]
        );
    }

    #[test]
    fn release_failure_is_terminal_and_cleanup_still_runs() {
        let mut harness = Harness::new();
        harness.slots.release_failure = Some(DesktopEpisodeAdmissionFailure::new(
            "presentation_release_failed",
            "durable capacity mutation was not committed",
        ));

        let outcome = harness.run(
            EvidenceRequest::browser_external(BrowserExternalSurface::PasskeyChooser, true),
            EpisodeInput::None,
        );

        let DesktopEpisodeOutcome::ReleaseFailed { receipt } = outcome else {
            panic!("expected a terminal release failure receipt");
        };
        assert_eq!(receipt.episode_id, "episode-1");
        assert_eq!(receipt.failure.code, "presentation_release_failed");
        assert_eq!(receipt.cleanup_receipt_id, "cleanup-1");
        assert!(harness.log.borrow().ends_with(&["release", "cleanup"]));
    }

    #[test]
    fn configured_production_input_is_unavailable_and_returns_human_handoff() {
        let mut harness = Harness::new();
        let outcome = harness.run(
            EvidenceRequest::browser_external(BrowserExternalSurface::PasskeyChooser, true),
            EpisodeInput::ConfiguredProduction,
        );

        assert!(matches!(
            outcome,
            DesktopEpisodeOutcome::HumanContinuation {
                receipt: HumanContinuationReceipt {
                    reason: "configured_production_input_unavailable",
                    ..
                }
            }
        ));
        assert_eq!(*harness.log.borrow(), vec!["handoff"]);
    }

    #[test]
    fn human_controller_precedence_blocks_reservation_staging_and_trigger() {
        let mut harness = Harness::new();
        harness.windows.admission.controller_posture = ControllerPosture::Human;

        let outcome = harness.run(
            EvidenceRequest::browser_external(BrowserExternalSurface::PasskeyChooser, true),
            EpisodeInput::None,
        );

        assert!(matches!(
            outcome,
            DesktopEpisodeOutcome::HumanContinuation {
                receipt: HumanContinuationReceipt {
                    reason: "human_controller_has_precedence",
                    ..
                }
            }
        ));
        assert_eq!(*harness.log.borrow(), vec!["scene_admission", "handoff"]);
    }

    #[test]
    fn unavailable_capacity_stops_before_scene_snapshot_or_external_trigger() {
        let mut harness = Harness::new();
        harness.slots.admission_failure = Some(DesktopEpisodeAdmissionFailure::new(
            "presentation_capacity_reserved",
            "human and recovery capacity is protected",
        ));

        let outcome = harness.run(
            EvidenceRequest::browser_external(BrowserExternalSurface::PasskeyChooser, true),
            EpisodeInput::None,
        );

        assert_eq!(
            outcome,
            DesktopEpisodeOutcome::AdmissionUnavailable {
                failure: DesktopEpisodeAdmissionFailure::new(
                    "presentation_capacity_reserved",
                    "human and recovery capacity is protected",
                ),
            }
        );
        assert_eq!(*harness.log.borrow(), vec!["scene_admission", "reserve"]);
    }

    #[test]
    fn unavailable_scene_probe_stops_before_capacity_reservation() {
        let mut harness = Harness::new();
        harness.windows.scene_failure = Some(DesktopEpisodeAdapterFailure::new(
            "scene_admission",
            "desktop_scene_probe_unavailable",
            "the configured window provider returned no current observation",
        ));

        let outcome = harness.run(
            EvidenceRequest::browser_external(BrowserExternalSurface::PasskeyChooser, true),
            EpisodeInput::None,
        );

        assert!(matches!(
            outcome,
            DesktopEpisodeOutcome::AdapterUnavailable {
                failure: DesktopEpisodeAdapterFailure {
                    code,
                    ..
                }
            } if code == "desktop_scene_probe_unavailable"
        ));
        assert_eq!(*harness.log.borrow(), vec!["scene_admission"]);
    }

    #[test]
    fn unavailable_capture_probe_releases_capacity_and_cleans_up() {
        let mut harness = Harness::new();
        harness.windows.capture_failure = Some(DesktopEpisodeAdapterFailure::new(
            "capture_ready",
            "desktop_capture_ready_probe_unavailable",
            "the configured window provider returned no current capture proof",
        ));

        let outcome = harness.run(
            EvidenceRequest::browser_external(BrowserExternalSurface::PasskeyChooser, true),
            EpisodeInput::None,
        );

        let DesktopEpisodeOutcome::Aborted { receipt } = outcome else {
            panic!("expected a terminal adapter failure receipt");
        };
        assert!(matches!(
            receipt.failure,
            DesktopEpisodeFailure::Adapter(DesktopEpisodeAdapterFailure { code, .. })
                if code == "desktop_capture_ready_probe_unavailable"
        ));
        assert!(harness.log.borrow().ends_with(&["release", "cleanup"]));
    }

    #[test]
    fn unavailable_frame_capture_releases_capacity_and_cleans_up() {
        let mut harness = Harness::new();
        harness.frames.failure = Some(DesktopEpisodeAdapterFailure::new(
            "desktop_frame_capture",
            "desktop_frame_binding_drift",
            "the configured frame no longer matches the capture-ready proof",
        ));

        let outcome = harness.run(
            EvidenceRequest::browser_external(BrowserExternalSurface::PasskeyChooser, true),
            EpisodeInput::None,
        );

        let DesktopEpisodeOutcome::Aborted { receipt } = outcome else {
            panic!("expected a terminal frame failure receipt");
        };
        assert!(matches!(
            receipt.failure,
            DesktopEpisodeFailure::Adapter(DesktopEpisodeAdapterFailure { code, .. })
                if code == "desktop_frame_binding_drift"
        ));
        assert!(harness.log.borrow().ends_with(&["release", "cleanup"]));
    }

    #[test]
    fn capture_binding_drift_stops_and_still_releases_and_cleans_up() {
        let mut harness = Harness::new();
        harness.windows.evidence[1].process_generation = "process-3".to_string();

        let outcome = harness.run(
            EvidenceRequest::browser_external(BrowserExternalSurface::PasskeyChooser, true),
            EpisodeInput::None,
        );

        let DesktopEpisodeOutcome::Aborted { receipt } = outcome else {
            panic!("expected a terminal receipt");
        };
        assert_eq!(receipt.failure, DesktopEpisodeFailure::CaptureBindingDrift);
        assert_eq!(
            receipt.restoration_decision,
            RestorationDecision::CancelledAuthorityDrift
        );
        assert!(harness.log.borrow().ends_with(&["release", "cleanup"]));
        assert!(!harness.log.borrow().contains(&"restore"));
    }

    #[test]
    fn passkey_chooser_uses_paired_desktop_evidence_with_pretrigger_staging() {
        let decision = DesktopEvidenceCoordinator::decide(EvidenceRequest::browser_external(
            BrowserExternalSurface::PasskeyChooser,
            true,
        ));

        assert_eq!(decision.outcome, EvidenceOutcome::DesktopEvidenceEpisode);
        assert!(decision.presentation_slot_required);
        assert!(decision.paired_page_absence_required);
        assert!(decision.stage_before_trigger);
        assert_eq!(
            decision.reason,
            EvidenceDecisionReason::BrowserExternalSurfaceRequiresDesktop
        );
    }

    #[test]
    fn cdp_timeout_does_not_authorize_desktop_fallback() {
        let decision = DesktopEvidenceCoordinator::decide(EvidenceRequest::diagnostic_failure(
            DiagnosticFailure::CdpTimeout,
        ));

        assert_eq!(decision.outcome, EvidenceOutcome::DiagnosticFailure);
        assert!(!decision.presentation_slot_required);
        assert!(!decision.paired_page_absence_required);
        assert_eq!(
            decision.reason,
            EvidenceDecisionReason::DiagnosticFailureDoesNotAuthorizeDesktop
        );
    }

    #[test]
    fn biometric_surface_requires_typed_human_continuation() {
        let decision = DesktopEvidenceCoordinator::decide(EvidenceRequest::human_only(
            HumanContinuationSurface::Biometric,
        ));

        assert_eq!(decision.outcome, EvidenceOutcome::HumanContinuation);
        assert!(!decision.presentation_slot_required);
        assert_eq!(
            decision.reason,
            EvidenceDecisionReason::SensitiveSurfaceRequiresHumanContinuation
        );
    }

    #[test]
    fn supported_javascript_dialog_remains_a_cdp_operation() {
        let decision = DesktopEvidenceCoordinator::decide(EvidenceRequest::supported_cdp(
            SupportedCdpSurface::JavaScriptDialog,
        ));

        assert_eq!(decision.outcome, EvidenceOutcome::Cdp);
        assert!(!decision.presentation_slot_required);
        assert_eq!(
            decision.reason,
            EvidenceDecisionReason::SupportedCdpMechanism
        );
    }

    #[test]
    fn capture_ready_proof_requires_all_scene_and_authority_bindings() {
        let evidence = CaptureReadyEvidence::complete("scene-7", "geometry-3");

        let proof = DesktopEvidenceCoordinator::prove_capture_ready(evidence)
            .expect("complete evidence should be capture ready");

        assert_eq!(proof.scene_generation, "scene-7");
        assert_eq!(proof.geometry_epoch, "geometry-3");
        assert_eq!(proof.viewer_posture, ViewerPosture::Passive);
        assert_eq!(proof.controller_posture, ControllerPosture::Uncontrolled);
    }

    #[test]
    fn human_controller_and_passive_viewer_bound_automated_staging() {
        assert_eq!(
            DesktopEvidenceCoordinator::admit_scene(SceneAdmissionRequest {
                viewer_posture: ViewerPosture::Passive,
                controller_posture: ControllerPosture::Uncontrolled,
                requires_staging: false,
                capture_ready: true,
                explicit_takeover: false,
            }),
            SceneAdmission::CaptureAllowed
        );
        assert_eq!(
            DesktopEvidenceCoordinator::admit_scene(SceneAdmissionRequest {
                viewer_posture: ViewerPosture::Passive,
                controller_posture: ControllerPosture::Uncontrolled,
                requires_staging: true,
                capture_ready: false,
                explicit_takeover: false,
            }),
            SceneAdmission::WaitForViewer
        );
        assert_eq!(
            DesktopEvidenceCoordinator::admit_scene(SceneAdmissionRequest {
                viewer_posture: ViewerPosture::None,
                controller_posture: ControllerPosture::Human,
                requires_staging: true,
                capture_ready: false,
                explicit_takeover: false,
            }),
            SceneAdmission::WaitForHumanController
        );
    }

    #[test]
    fn restoration_drift_cannot_overwrite_newer_human_or_route_intent() {
        let recorded = RestorationAuthority::new("scene-2", "route-generation-4", "control-3");

        assert_eq!(
            DesktopEvidenceCoordinator::authorize_restoration(
                &recorded,
                &RestorationAuthority::new("scene-2", "route-generation-5", "control-3"),
            ),
            RestorationDecision::CancelledAuthorityDrift
        );
        assert_eq!(
            DesktopEvidenceCoordinator::authorize_restoration(
                &recorded,
                &RestorationAuthority::new("scene-2", "route-generation-4", "control-3"),
            ),
            RestorationDecision::Restore
        );
    }

    #[test]
    fn episode_receipt_binds_before_capture_after_restore_release_and_cleanup() {
        let proof = DesktopEvidenceCoordinator::prove_capture_ready(
            CaptureReadyEvidence::complete("scene-staged", "geometry-9"),
        )
        .unwrap();
        let receipt = DesktopEvidenceEpisodeReceipt {
            episode_id: "episode-9".to_string(),
            evidence_decision: DesktopEvidenceCoordinator::decide(
                EvidenceRequest::browser_external(BrowserExternalSurface::PasskeyChooser, true),
            ),
            admission_receipt_id: "admission-8".to_string(),
            before_scene_generation: "scene-before".to_string(),
            stage_receipt_id: Some("stage-8".to_string()),
            page_absence_receipt_id: "absence-8".to_string(),
            trigger_receipt_id: "trigger-8".to_string(),
            capture_proof: proof,
            capture_receipt_id: "capture-8".to_string(),
            input_receipt_id: None,
            verification_receipt_id: "verify-8".to_string(),
            after_capture_proof: DesktopEvidenceCoordinator::prove_capture_ready(
                CaptureReadyEvidence::complete("scene-staged", "geometry-9"),
            )
            .unwrap(),
            after_scene_generation: "scene-after".to_string(),
            restoration_decision: RestorationDecision::Restore,
            slot_release_receipt_id: "release-3".to_string(),
            cleanup_receipt_id: "cleanup-3".to_string(),
        };

        let value = serde_json::to_value(receipt).unwrap();
        assert_eq!(value["episodeId"], "episode-9");
        assert_eq!(value["captureProof"]["sceneGeneration"], "scene-staged");
        assert_eq!(value["restorationDecision"], "restore");
        assert_eq!(value["cleanupReceiptId"], "cleanup-3");
    }
}
