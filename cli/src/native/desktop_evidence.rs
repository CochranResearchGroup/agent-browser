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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaptureReadyEvidence {
    browser_id: String,
    process_generation: String,
    route_id: String,
    display_allocation_id: String,
    presentation_slot_id: String,
    scene_generation: String,
    geometry_epoch: String,
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
    #[cfg(test)]
    fn complete(scene_generation: impl Into<String>, geometry_epoch: impl Into<String>) -> Self {
        Self {
            browser_id: "browser-1".to_string(),
            process_generation: "process-2".to_string(),
            route_id: "route-4".to_string(),
            display_allocation_id: "display-allocation-4".to_string(),
            presentation_slot_id: "slot-4".to_string(),
            scene_generation: scene_generation.into(),
            geometry_epoch: geometry_epoch.into(),
            active_window_owned: true,
            topmost_window_owned: true,
            authorized_geometry: true,
            capture_region_unoccluded: true,
            frame_mapping_current: true,
            viewer_posture: ViewerPosture::Passive,
            controller_posture: ControllerPosture::Uncontrolled,
            proof_age_ms: 50,
            maximum_age_ms: 500,
        }
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
    pub(crate) capture_proof: CaptureReadyProof,
    pub(crate) after_scene_generation: String,
    pub(crate) restoration_decision: RestorationDecision,
    pub(crate) slot_release_receipt_id: String,
    pub(crate) cleanup_receipt_id: String,
}

pub(crate) struct DesktopEvidenceCoordinator;

impl DesktopEvidenceCoordinator {
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
            capture_proof: proof,
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
