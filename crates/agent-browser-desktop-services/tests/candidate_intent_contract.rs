use agent_browser_desktop_services::{
    admit_desktop_candidate_intent, desktop_candidate_intent_digest,
    desktop_candidate_observation_digest, desktop_candidate_set_digest, ControllerAuthority,
    DesktopBinding, DesktopCandidateAdmissionError, DesktopCandidateGeometry,
    DesktopCandidateIntent, DesktopCandidateObservation, PixelBounds, PixelPoint, COORDINATE_SPACE,
};

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn binding() -> DesktopBinding {
    DesktopBinding {
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
    }
}

fn candidates() -> Vec<DesktopCandidateGeometry> {
    vec![
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
    ]
}

fn observation() -> DesktopCandidateObservation {
    let candidates = candidates();
    let mut observation = DesktopCandidateObservation {
        binding: binding(),
        evidence_digest: digest('a'),
        frame_digest: digest('b'),
        context_digest: digest('c'),
        geometry_digest: digest('d'),
        candidate_set_digest: desktop_candidate_set_digest(&candidates),
        candidates,
        captured_at_ms: 1_000,
        expires_at_ms: 2_000,
        observation_digest: String::new(),
    };
    observation.observation_digest = desktop_candidate_observation_digest(&observation);
    observation
}

fn intent(observation: &DesktopCandidateObservation) -> DesktopCandidateIntent {
    let mut intent = DesktopCandidateIntent {
        source_intent_digest: digest('e'),
        observation_digest: observation.observation_digest.clone(),
        evidence_digest: observation.evidence_digest.clone(),
        frame_digest: observation.frame_digest.clone(),
        context_digest: observation.context_digest.clone(),
        geometry_digest: observation.geometry_digest.clone(),
        candidate_set_digest: observation.candidate_set_digest.clone(),
        selected_candidate_ids: vec!["candidate:1".to_string()],
        controller_lease_id: "lease:1".to_string(),
        controller_viewer_id: "challenge-agent".to_string(),
        provider_capability_digest: digest('f'),
        planned_steps: 1,
        planned_pointer_events: 4,
        planned_key_events: 0,
        expires_at_ms: observation.expires_at_ms,
        intent_digest: String::new(),
    };
    intent.intent_digest = desktop_candidate_intent_digest(&intent);
    intent
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

fn refresh_observation(observation: &mut DesktopCandidateObservation) {
    observation.candidate_set_digest = desktop_candidate_set_digest(&observation.candidates);
    observation.observation_digest = desktop_candidate_observation_digest(observation);
}

fn refresh_intent(intent: &mut DesktopCandidateIntent) {
    intent.intent_digest = desktop_candidate_intent_digest(intent);
}

fn rejection(
    intent: &DesktopCandidateIntent,
    observation: &DesktopCandidateObservation,
    authority: &ControllerAuthority,
    now_ms: u64,
) -> DesktopCandidateAdmissionError {
    admit_desktop_candidate_intent(intent, observation, authority, now_ms)
        .expect_err("mutated contract must fail closed")
}

#[test]
fn valid_candidate_intent_returns_one_exact_effect_free_permit() {
    let observation = observation();
    let intent = intent(&observation);
    let permit = admit_desktop_candidate_intent(&intent, &observation, &authority(), 1_100)
        .expect("valid candidate intent should be admitted");

    assert_eq!(permit.source_intent_digest, intent.source_intent_digest);
    assert_eq!(permit.selected_candidates, vec![candidates()[0].clone()]);
    assert_eq!(permit.binding, binding());
    assert_eq!(permit.planned_steps, 1);
    assert_eq!(permit.planned_pointer_events, 4);
    assert_eq!(permit.planned_key_events, 0);
    assert!(!permit.emitted_effects());

    let replay = admit_desktop_candidate_intent(&intent, &observation, &authority(), 1_100)
        .expect("exact replay should remain deterministic");
    assert_eq!(permit, replay);
}

#[test]
fn candidate_set_digest_binds_order_bounds_and_center() {
    let original = candidates();
    let original_digest = desktop_candidate_set_digest(&original);

    let mut reordered = original.clone();
    reordered.reverse();
    assert_ne!(desktop_candidate_set_digest(&reordered), original_digest);

    let mut moved_bounds = original.clone();
    moved_bounds[0].bounds.x += 1;
    assert_ne!(desktop_candidate_set_digest(&moved_bounds), original_digest);

    let mut moved_center = original;
    moved_center[0].center.x += 1;
    assert_ne!(desktop_candidate_set_digest(&moved_center), original_digest);
}

#[test]
fn invalid_candidate_selection_fails_closed() {
    let observation = observation();
    for selected in [
        Vec::<String>::new(),
        vec!["candidate:1".to_string(), "candidate:1".to_string()],
        vec!["candidate:missing".to_string()],
        vec!["candidate:2".to_string(), "candidate:1".to_string()],
    ] {
        let mut intent = intent(&observation);
        intent.selected_candidate_ids = selected;
        intent.planned_steps = u8::try_from(intent.selected_candidate_ids.len()).unwrap_or(0);
        refresh_intent(&mut intent);
        assert!(matches!(
            rejection(&intent, &observation, &authority(), 1_100),
            DesktopCandidateAdmissionError::InvalidIntent
                | DesktopCandidateAdmissionError::CandidateMismatch
        ));
    }
}

#[test]
fn observation_digest_and_intent_evidence_mismatches_fail_closed() {
    let observation = observation();

    let mut invalid_observation = observation.clone();
    invalid_observation.frame_digest = digest('9');
    assert_eq!(
        rejection(
            &intent(&invalid_observation),
            &invalid_observation,
            &authority(),
            1_100
        ),
        DesktopCandidateAdmissionError::InvalidObservation
    );

    for field in 0..6 {
        let mut mismatched_intent = intent(&observation);
        match field {
            0 => mismatched_intent.observation_digest = digest('0'),
            1 => mismatched_intent.evidence_digest = digest('1'),
            2 => mismatched_intent.frame_digest = digest('2'),
            3 => mismatched_intent.context_digest = digest('3'),
            4 => mismatched_intent.geometry_digest = digest('4'),
            _ => mismatched_intent.candidate_set_digest = digest('5'),
        }
        refresh_intent(&mut mismatched_intent);
        assert_eq!(
            rejection(&mismatched_intent, &observation, &authority(), 1_100),
            DesktopCandidateAdmissionError::CandidateMismatch
        );
    }
}

#[test]
fn invalid_candidate_geometry_fails_closed() {
    let cases = [
        PixelBounds {
            x: 100,
            y: 200,
            width: 0,
            height: 40,
        },
        PixelBounds {
            x: -1,
            y: 200,
            width: 80,
            height: 40,
        },
        PixelBounds {
            x: 1_250,
            y: 200,
            width: 80,
            height: 40,
        },
        PixelBounds {
            x: i64::MAX,
            y: 200,
            width: 80,
            height: 40,
        },
    ];
    for bounds in cases {
        let mut invalid = observation();
        invalid.candidates[0].bounds = bounds;
        refresh_observation(&mut invalid);
        assert_eq!(
            rejection(&intent(&invalid), &invalid, &authority(), 1_100),
            DesktopCandidateAdmissionError::InvalidObservation
        );
    }

    let mut outside_center = observation();
    outside_center.candidates[0].center = PixelPoint { x: 180, y: 220 };
    refresh_observation(&mut outside_center);
    assert_eq!(
        rejection(
            &intent(&outside_center),
            &outside_center,
            &authority(),
            1_100
        ),
        DesktopCandidateAdmissionError::InvalidObservation
    );
}

#[test]
fn stale_or_inconsistent_time_binding_fails_closed() {
    let observation = observation();
    assert_eq!(
        rejection(&intent(&observation), &observation, &authority(), 999),
        DesktopCandidateAdmissionError::Stale
    );
    assert_eq!(
        rejection(&intent(&observation), &observation, &authority(), 2_000),
        DesktopCandidateAdmissionError::Stale
    );

    let mut intent = intent(&observation);
    intent.expires_at_ms += 1;
    refresh_intent(&mut intent);
    assert_eq!(
        rejection(&intent, &observation, &authority(), 1_100),
        DesktopCandidateAdmissionError::Stale
    );
}

#[test]
fn binding_and_authority_drift_fail_closed() {
    for field in 0..6 {
        let mut drifted = observation();
        let original_intent = intent(&drifted);
        match field {
            0 => drifted.binding.browser_id.push_str(":drift"),
            1 => drifted.binding.display_allocation_id.push_str(":drift"),
            2 => drifted.binding.stream_id.push_str(":drift"),
            3 => drifted.binding.route_id.push_str(":drift"),
            4 => drifted.binding.coordinate_space = "css_pixels".to_string(),
            _ => drifted.binding.geometry_epoch.push_str(":drift"),
        }
        refresh_observation(&mut drifted);
        assert!(matches!(
            rejection(&original_intent, &drifted, &authority(), 1_100),
            DesktopCandidateAdmissionError::InvalidObservation
                | DesktopCandidateAdmissionError::CandidateMismatch
                | DesktopCandidateAdmissionError::AuthorityMismatch
        ));
    }

    for field in 0..14 {
        let observation = observation();
        let intent = intent(&observation);
        let mut drifted = authority();
        match field {
            0 => drifted.lease_id.push_str(":drift"),
            1 => drifted.lease_viewer_id.push_str(":drift"),
            2 => drifted.lease_role = "observer".to_string(),
            3 => drifted.lease_state = "released".to_string(),
            4 => drifted.route_contains_lease = false,
            5 => drifted.stream_contains_lease = false,
            6 => drifted.route_writable = false,
            7 => drifted.stream_writable = false,
            8 => drifted.route_machine_input = Some("manual_attached_desktop".to_string()),
            9 => drifted.stream_machine_input = Some("different_provider".to_string()),
            10 => drifted.controller_epoch = 0,
            11 => drifted.stream_controller_epoch += 1,
            12 => drifted.lease_updated_at.clear(),
            _ => {
                drifted.route_machine_input = Some(String::new());
                drifted.stream_machine_input = Some(String::new());
            }
        }
        assert_eq!(
            rejection(&intent, &observation, &drifted, 1_100),
            DesktopCandidateAdmissionError::AuthorityMismatch
        );
    }
}

#[test]
fn noncanonical_digests_and_empty_optional_identity_fail_closed() {
    let observation = observation();
    let mut uppercase_digest = intent(&observation);
    uppercase_digest.source_intent_digest = "A".repeat(64);
    refresh_intent(&mut uppercase_digest);
    assert_eq!(
        rejection(&uppercase_digest, &observation, &authority(), 1_100),
        DesktopCandidateAdmissionError::InvalidIntent
    );

    let mut empty_profile = observation.clone();
    empty_profile.binding.profile_id = Some(" ".to_string());
    refresh_observation(&mut empty_profile);
    assert_eq!(
        rejection(&intent(&empty_profile), &empty_profile, &authority(), 1_100),
        DesktopCandidateAdmissionError::InvalidObservation
    );
}

#[test]
fn zero_oversized_or_inconsistent_effect_budgets_fail_closed() {
    let observation = observation();
    for mutation in 0..5 {
        let mut intent = intent(&observation);
        match mutation {
            0 => intent.planned_steps = 0,
            1 => {
                intent.planned_pointer_events = 0;
                intent.planned_key_events = 0;
            }
            2 => intent.planned_steps = 2,
            3 => intent.planned_pointer_events = u8::MAX,
            _ => intent.planned_key_events = u8::MAX,
        }
        refresh_intent(&mut intent);
        assert_eq!(
            rejection(&intent, &observation, &authority(), 1_100),
            DesktopCandidateAdmissionError::InvalidIntent
        );
    }
}
