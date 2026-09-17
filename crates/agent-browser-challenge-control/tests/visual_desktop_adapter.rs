use agent_browser_challenge_control::{
    admit_visual_desktop_candidate_intent, decide_visual_round, visual_candidate_set_digest,
    visual_round_evidence_digest, VisualDesktopAdmissionError, VisualDesktopControllerBinding,
    VisualProviderCapability, VisualRoundDecision, VisualRoundError, VisualRoundEvent,
    VisualRoundEvidence, VisualRoundIntent, VisualRoundPhase, VisualRoundPolicy,
    VisualRoundSelection, VisualRoundSnapshot,
};
use agent_browser_desktop_services::{
    desktop_candidate_observation_digest, desktop_candidate_set_digest, ControllerAuthority,
    DesktopBinding, DesktopCandidateAdmissionError, DesktopCandidateGeometry,
    DesktopCandidateObservation, PixelBounds, PixelPoint, COORDINATE_SPACE,
};

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn capability() -> VisualProviderCapability {
    VisualProviderCapability {
        capability_id: "synthetic-visual-reasoning".to_string(),
        capability_version: "v1".to_string(),
        capability_digest: digest('c'),
    }
}

fn policy() -> VisualRoundPolicy {
    VisualRoundPolicy {
        policy_digest: digest('a'),
        profile_digest: digest('b'),
        provider_capability: capability(),
        max_rounds: 2,
        max_selections_per_round: 2,
        max_steps_per_round: 2,
        max_pointer_events_per_round: 8,
        max_key_events_per_round: 0,
        max_total_selections: 3,
        max_total_steps: 3,
        max_total_pointer_events: 12,
        max_total_key_events: 0,
        deadline_at_ms: 10_000,
    }
}

fn evidence() -> VisualRoundEvidence {
    let candidate_ids = vec!["candidate:1".to_string(), "candidate:2".to_string()];
    let mut evidence = VisualRoundEvidence {
        task_id: "task:visual".to_string(),
        attempt_id: "attempt:1".to_string(),
        profile_digest: digest('b'),
        policy_digest: digest('a'),
        round_index: 1,
        round_id: "round:1".to_string(),
        evidence_digest: String::new(),
        frame_digest: digest('1'),
        context_digest: digest('2'),
        geometry_digest: digest('3'),
        candidate_set_digest: visual_candidate_set_digest(&candidate_ids),
        candidate_ids,
        observed_at_ms: 1_000,
        expires_at_ms: 2_000,
        provider_capability: capability(),
    };
    evidence.evidence_digest = visual_round_evidence_digest(&evidence);
    evidence
}

fn authorized_with_selection(
    selected_candidate_ids: &[&str],
    planned_steps: u8,
    planned_pointer_events: u8,
) -> (VisualRoundSnapshot, VisualRoundIntent) {
    authorized_for_policy(
        &policy(),
        selected_candidate_ids,
        planned_steps,
        planned_pointer_events,
    )
}

fn authorized_for_policy(
    policy: &VisualRoundPolicy,
    selected_candidate_ids: &[&str],
    planned_steps: u8,
    planned_pointer_events: u8,
) -> (VisualRoundSnapshot, VisualRoundIntent) {
    let evidence = evidence();
    let initial = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
    let observed = decide_visual_round(
        policy,
        &initial,
        VisualRoundEvent::Observe {
            evidence: evidence.clone(),
            now_ms: 1_000,
        },
    )
    .unwrap();
    let decision = decide_visual_round(
        policy,
        observed.snapshot(),
        VisualRoundEvent::Select {
            selection: VisualRoundSelection {
                round_id: evidence.round_id.clone(),
                evidence_digest: evidence.evidence_digest.clone(),
                candidate_set_digest: evidence.candidate_set_digest.clone(),
                selected_candidate_ids: selected_candidate_ids
                    .iter()
                    .map(|candidate_id| (*candidate_id).to_string())
                    .collect(),
                planned_steps,
                planned_pointer_events,
                planned_key_events: 0,
                provider_capability: capability(),
            },
            now_ms: 1_100,
        },
    )
    .unwrap();
    match decision {
        VisualRoundDecision::PermitIntent { snapshot, intent } => (snapshot, *intent),
        other => panic!("expected visual intent, got {other:?}"),
    }
}

fn authorized() -> (VisualRoundSnapshot, VisualRoundIntent) {
    authorized_with_selection(&["candidate:1"], 1, 4)
}

fn observation() -> DesktopCandidateObservation {
    let evidence = evidence();
    let candidates = vec![
        DesktopCandidateGeometry {
            candidate_id: "candidate:1".to_string(),
            bounds: PixelBounds {
                x: 100,
                y: 200,
                width: 80,
                height: 40,
            },
            center: PixelPoint { x: 140, y: 220 },
        },
        DesktopCandidateGeometry {
            candidate_id: "candidate:2".to_string(),
            bounds: PixelBounds {
                x: 300,
                y: 200,
                width: 80,
                height: 40,
            },
            center: PixelPoint { x: 340, y: 220 },
        },
    ];
    let mut observation = DesktopCandidateObservation {
        binding: DesktopBinding {
            browser_id: "browser:1".to_string(),
            session_name: "session:1".to_string(),
            profile_id: Some("profile:1".to_string()),
            display_allocation_id: "display:1".to_string(),
            stream_id: "stream:1".to_string(),
            route_id: "route:1".to_string(),
            width: 1280,
            height: 720,
            scale_millis: 1000,
            coordinate_space: COORDINATE_SPACE.to_string(),
            geometry_epoch: "geometry:1".to_string(),
        },
        evidence_digest: evidence.evidence_digest,
        frame_digest: evidence.frame_digest,
        context_digest: evidence.context_digest,
        geometry_digest: evidence.geometry_digest,
        candidate_set_digest: desktop_candidate_set_digest(&candidates),
        candidates,
        captured_at_ms: evidence.observed_at_ms,
        expires_at_ms: evidence.expires_at_ms,
        observation_digest: String::new(),
    };
    observation.observation_digest = desktop_candidate_observation_digest(&observation);
    observation
}

fn authority() -> ControllerAuthority {
    ControllerAuthority {
        browser_id: "browser:1".to_string(),
        display_allocation_id: "display:1".to_string(),
        stream_id: "stream:1".to_string(),
        route_id: "route:1".to_string(),
        route_controller_lease_id: "lease:1".to_string(),
        stream_controller_lease_id: "lease:1".to_string(),
        lease_id: "lease:1".to_string(),
        lease_record_id: "lease:1".to_string(),
        lease_route_id: "route:1".to_string(),
        lease_browser_id: "browser:1".to_string(),
        lease_viewer_id: "challenge-agent".to_string(),
        lease_role: "controller".to_string(),
        lease_state: "controlling".to_string(),
        lease_updated_at: "2026-09-17T15:00:00Z".to_string(),
        lease_expires_at_ms: 2_000,
        controller_epoch: 7,
        route_controller_epoch: 7,
        stream_controller_epoch: 7,
        route_contains_lease: true,
        stream_contains_lease: true,
        route_writable: true,
        stream_writable: true,
        route_machine_input: Some("synthetic_input_v1".to_string()),
        stream_machine_input: Some("synthetic_input_v1".to_string()),
    }
}

fn controller() -> VisualDesktopControllerBinding {
    VisualDesktopControllerBinding {
        controller_lease_id: "lease:1".to_string(),
        controller_viewer_id: "challenge-agent".to_string(),
    }
}

fn refresh_observation(observation: &mut DesktopCandidateObservation) {
    observation.candidate_set_digest = desktop_candidate_set_digest(&observation.candidates);
    observation.observation_digest = desktop_candidate_observation_digest(observation);
}

fn rejection(
    snapshot: &VisualRoundSnapshot,
    intent: &VisualRoundIntent,
    observation: &DesktopCandidateObservation,
    authority: &ControllerAuthority,
    controller: &VisualDesktopControllerBinding,
) -> VisualDesktopAdmissionError {
    admit_visual_desktop_candidate_intent(
        &policy(),
        snapshot,
        intent,
        observation,
        authority,
        controller,
        1_100,
    )
    .expect_err("mutated visual desktop admission must fail closed")
}

#[test]
fn authorized_visual_intent_returns_one_exact_effect_free_desktop_permit() {
    let (snapshot, intent) = authorized();
    let permit = admit_visual_desktop_candidate_intent(
        &policy(),
        &snapshot,
        &intent,
        &observation(),
        &authority(),
        &controller(),
        1_100,
    )
    .expect("authorized visual intent should map to a desktop permit");

    assert_eq!(permit.source_intent_digest, intent.intent_digest);
    assert_eq!(
        permit.selected_candidates,
        vec![observation().candidates[0].clone()]
    );
    assert_eq!(permit.provider_capability_digest, digest('c'));
    assert_eq!(permit.planned_steps, 1);
    assert_eq!(permit.planned_pointer_events, 4);
    assert_eq!(permit.planned_key_events, 0);
    assert!(!permit.emitted_effects());

    let replay = admit_visual_desktop_candidate_intent(
        &policy(),
        &snapshot,
        &intent,
        &observation(),
        &authority(),
        &controller(),
        1_100,
    )
    .expect("exact replay should remain deterministic");
    assert_eq!(permit, replay);
}

#[test]
fn visual_intent_must_be_the_exact_current_snapshot_output() {
    let (snapshot, intent) = authorized();

    let mut wrong_phase = snapshot.clone();
    wrong_phase.phase = VisualRoundPhase::AwaitingAfterState;
    assert!(matches!(
        rejection(
            &wrong_phase,
            &intent,
            &observation(),
            &authority(),
            &controller()
        ),
        VisualDesktopAdmissionError::VisualIntent(_)
    ));

    for field in 0..16 {
        let mut changed = intent.clone();
        match field {
            0 => changed.intent_digest = digest('d'),
            1 => changed.task_id.push_str(":drift"),
            2 => changed.attempt_id.push_str(":drift"),
            3 => changed.round_index += 1,
            4 => changed.round_id.push_str(":drift"),
            5 => changed.evidence_digest = digest('d'),
            6 => changed.candidate_set_digest = digest('d'),
            7 => changed.selected_candidate_ids = vec!["candidate:2".to_string()],
            8 => changed
                .selected_candidate_ids
                .push("candidate:1".to_string()),
            9 => changed.selected_candidate_ids = vec!["candidate:missing".to_string()],
            10 => changed.planned_steps += 1,
            11 => changed.planned_pointer_events += 1,
            12 => changed.planned_key_events += 1,
            13 => changed.provider_capability.capability_id.push_str(":drift"),
            14 => changed
                .provider_capability
                .capability_version
                .push_str(":drift"),
            _ => changed.provider_capability.capability_digest = digest('d'),
        }
        assert_eq!(
            rejection(
                &snapshot,
                &changed,
                &observation(),
                &authority(),
                &controller()
            ),
            VisualDesktopAdmissionError::VisualIntent(VisualRoundError::InvalidTransition)
        );
    }
}

#[test]
fn active_snapshot_mutation_fails_before_desktop_mapping() {
    let (snapshot, intent) = authorized();
    for field in 0..3 {
        let mut changed = snapshot.clone();
        match field {
            0 => changed.active_intent_digest = Some(digest('d')),
            1 => changed.active_evidence.as_mut().unwrap().frame_digest = digest('d'),
            _ => changed
                .active_selection
                .as_mut()
                .unwrap()
                .selected_candidate_ids
                .push("candidate:2".to_string()),
        }
        assert!(matches!(
            rejection(
                &changed,
                &intent,
                &observation(),
                &authority(),
                &controller()
            ),
            VisualDesktopAdmissionError::VisualIntent(_)
        ));
    }
}

#[test]
fn desktop_observation_must_match_visual_evidence_exactly() {
    let (snapshot, intent) = authorized();
    for field in 0..10 {
        let mut changed = observation();
        match field {
            0 => changed.candidates.reverse(),
            1 => changed.candidates.pop().map(|_| ()).unwrap(),
            2 => changed.candidates[1].candidate_id = "candidate:1".to_string(),
            3 => changed.candidates[1].candidate_id = "candidate:missing".to_string(),
            4 => changed.evidence_digest = digest('4'),
            5 => changed.frame_digest = digest('5'),
            6 => changed.context_digest = digest('6'),
            7 => changed.geometry_digest = digest('7'),
            8 => changed.captured_at_ms += 1,
            _ => changed.expires_at_ms += 1,
        }
        refresh_observation(&mut changed);
        assert_eq!(
            rejection(&snapshot, &intent, &changed, &authority(), &controller()),
            VisualDesktopAdmissionError::ObservationMismatch
        );
    }
}

#[test]
fn p212_selection_and_authority_rejections_remain_typed() {
    let (reordered_snapshot, reordered_intent) =
        authorized_with_selection(&["candidate:2", "candidate:1"], 2, 8);
    assert_eq!(
        rejection(
            &reordered_snapshot,
            &reordered_intent,
            &observation(),
            &authority(),
            &controller()
        ),
        VisualDesktopAdmissionError::Desktop(DesktopCandidateAdmissionError::CandidateMismatch)
    );

    let (snapshot, intent) = authorized();
    let mut invalid_geometry = observation();
    invalid_geometry.binding.geometry_epoch.clear();
    refresh_observation(&mut invalid_geometry);
    assert_eq!(
        rejection(
            &snapshot,
            &intent,
            &invalid_geometry,
            &authority(),
            &controller()
        ),
        VisualDesktopAdmissionError::Desktop(DesktopCandidateAdmissionError::InvalidObservation)
    );

    let mut wrong_controller = controller();
    wrong_controller.controller_viewer_id.push_str(":drift");
    assert_eq!(
        rejection(
            &snapshot,
            &intent,
            &observation(),
            &authority(),
            &wrong_controller
        ),
        VisualDesktopAdmissionError::Desktop(DesktopCandidateAdmissionError::AuthorityMismatch)
    );

    for field in 0..4 {
        let mut changed = authority();
        match field {
            0 => changed.browser_id.push_str(":drift"),
            1 => changed.route_id.push_str(":drift"),
            2 => changed.stream_contains_lease = false,
            _ => changed.stream_controller_epoch += 1,
        }
        assert_eq!(
            rejection(&snapshot, &intent, &observation(), &changed, &controller()),
            VisualDesktopAdmissionError::Desktop(DesktopCandidateAdmissionError::AuthorityMismatch)
        );
    }
}

#[test]
fn permit_expiry_is_bounded_by_visual_policy_deadline() {
    let mut policy = policy();
    policy.deadline_at_ms = 1_500;
    let (snapshot, intent) = authorized_for_policy(&policy, &["candidate:1"], 1, 4);
    let mut observation = observation();
    observation.expires_at_ms = policy.deadline_at_ms;
    refresh_observation(&mut observation);

    let permit = admit_visual_desktop_candidate_intent(
        &policy,
        &snapshot,
        &intent,
        &observation,
        &authority(),
        &controller(),
        1_100,
    )
    .expect("desktop permit should use the earlier visual policy deadline");
    assert_eq!(permit.expires_at_ms, 1_500);

    assert_eq!(
        admit_visual_desktop_candidate_intent(
            &policy,
            &snapshot,
            &intent,
            &observation,
            &authority(),
            &controller(),
            1_500,
        ),
        Err(VisualDesktopAdmissionError::StaleVisualIntent)
    );
}
