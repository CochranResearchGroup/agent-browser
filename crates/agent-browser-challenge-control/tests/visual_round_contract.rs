use agent_browser_challenge_control::{
    decide_visual_round, visual_candidate_set_digest, visual_round_evidence_digest,
    visual_round_intent_digest, DeliveryState, VisualAfterStateOutcome, VisualProviderCapability,
    VisualRoundAfterState, VisualRoundDecision, VisualRoundEffectReceipt, VisualRoundError,
    VisualRoundEvent, VisualRoundEvidence, VisualRoundInterventionReason, VisualRoundPhase,
    VisualRoundPolicy, VisualRoundSelection, VisualRoundSnapshot, VisualRoundTerminalOutcome,
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
        max_steps_per_round: 1,
        max_pointer_events_per_round: 4,
        max_key_events_per_round: 0,
        max_total_selections: 3,
        max_total_steps: 2,
        max_total_pointer_events: 8,
        max_total_key_events: 0,
        deadline_at_ms: 10_000,
    }
}

fn evidence(round_index: u8, observed_at_ms: u64) -> VisualRoundEvidence {
    let (frame_digest, context_digest, geometry_digest) = if round_index == 1 {
        (digest('1'), digest('2'), digest('3'))
    } else {
        (digest('5'), digest('6'), digest('7'))
    };
    let candidate_ids = vec![
        format!("round:{round_index}:candidate:1"),
        format!("round:{round_index}:candidate:2"),
    ];
    let candidate_set_digest = visual_candidate_set_digest(&candidate_ids);
    let mut evidence = VisualRoundEvidence {
        task_id: "task:visual".to_string(),
        attempt_id: "attempt:1".to_string(),
        profile_digest: digest('b'),
        policy_digest: digest('a'),
        round_index,
        round_id: format!("round:{round_index}"),
        evidence_digest: String::new(),
        frame_digest,
        context_digest,
        geometry_digest,
        candidate_set_digest,
        candidate_ids,
        observed_at_ms,
        expires_at_ms: observed_at_ms + 500,
        provider_capability: capability(),
    };
    evidence.evidence_digest = visual_round_evidence_digest(&evidence);
    evidence
}

fn selection(evidence: &VisualRoundEvidence, selected: &[&str]) -> VisualRoundSelection {
    VisualRoundSelection {
        round_id: evidence.round_id.clone(),
        evidence_digest: evidence.evidence_digest.clone(),
        candidate_set_digest: evidence.candidate_set_digest.clone(),
        selected_candidate_ids: selected.iter().map(|value| (*value).to_string()).collect(),
        planned_steps: 1,
        planned_pointer_events: 4,
        planned_key_events: 0,
        provider_capability: capability(),
    }
}

fn acknowledged_effect(
    evidence: &VisualRoundEvidence,
    snapshot: &VisualRoundSnapshot,
) -> VisualRoundEffectReceipt {
    VisualRoundEffectReceipt {
        round_id: evidence.round_id.clone(),
        evidence_digest: evidence.evidence_digest.clone(),
        intent_digest: snapshot.active_intent_digest.clone().unwrap(),
        receipt_ref: format!("receipt:effect:{}", evidence.round_id),
        delivery: DeliveryState::Acknowledged,
        steps: 1,
        pointer_events: 4,
        key_events: 0,
    }
}

fn after_state(
    evidence: &VisualRoundEvidence,
    outcome: VisualAfterStateOutcome,
    observed_at_ms: u64,
) -> VisualRoundAfterState {
    VisualRoundAfterState {
        round_id: evidence.round_id.clone(),
        evidence_digest: evidence.evidence_digest.clone(),
        frame_digest: if evidence.round_index == 1 {
            digest('9')
        } else {
            digest('f')
        },
        context_digest: evidence.context_digest.clone(),
        geometry_digest: evidence.geometry_digest.clone(),
        candidate_set_digest: evidence.candidate_set_digest.clone(),
        observed_at_ms,
        expires_at_ms: observed_at_ms + 500,
        outcome,
        terminal_receipt_ref: (outcome != VisualAfterStateOutcome::NextRound)
            .then(|| "receipt:visual:two-round".to_string()),
    }
}

fn authorized_snapshot(
    policy: &VisualRoundPolicy,
    evidence: &VisualRoundEvidence,
) -> VisualRoundSnapshot {
    let initial = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
    let observed = decide_visual_round(
        policy,
        &initial,
        VisualRoundEvent::Observe {
            evidence: evidence.clone(),
            now_ms: evidence.observed_at_ms,
        },
    )
    .unwrap();
    decide_visual_round(
        policy,
        observed.snapshot(),
        VisualRoundEvent::Select {
            selection: selection(evidence, &["round:1:candidate:1"]),
            now_ms: evidence.observed_at_ms + 100,
        },
    )
    .unwrap()
    .snapshot()
    .clone()
}

fn await_after_state(
    policy: &VisualRoundPolicy,
    evidence: &VisualRoundEvidence,
) -> VisualRoundSnapshot {
    let selected = authorized_snapshot(policy, evidence);
    decide_visual_round(
        policy,
        &selected,
        VisualRoundEvent::EffectFinished(acknowledged_effect(evidence, &selected)),
    )
    .unwrap()
    .snapshot()
    .clone()
}

#[test]
fn restored_intent_phase_requires_the_complete_active_envelope() {
    let policy = policy();
    let mut malformed = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
    malformed.phase = VisualRoundPhase::IntentAuthorized;

    assert_eq!(
        decide_visual_round(
            &policy,
            &malformed,
            VisualRoundEvent::Observe {
                evidence: evidence(1, 1_000),
                now_ms: 1_000,
            },
        ),
        Err(VisualRoundError::InvalidSnapshot)
    );
}

#[test]
fn restored_terminal_pass_or_denial_requires_a_receipt() {
    let policy = policy();
    for outcome in [
        VisualRoundTerminalOutcome::Passed,
        VisualRoundTerminalOutcome::Denied,
    ] {
        let mut malformed = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
        malformed.phase = VisualRoundPhase::Terminal;
        malformed.terminal_outcome = Some(outcome);

        assert_eq!(
            decide_visual_round(
                &policy,
                &malformed,
                VisualRoundEvent::Replay {
                    original_receipt_ref: "receipt:missing".to_string(),
                },
            ),
            Err(VisualRoundError::InvalidSnapshot)
        );
    }
}

#[test]
fn restored_active_intent_digest_must_match_the_bound_evidence_and_selection() {
    let policy = policy();
    let evidence = evidence(1, 1_000);
    let mut malformed = authorized_snapshot(&policy, &evidence);
    malformed.active_intent_digest = Some(digest('d'));

    assert_eq!(
        decide_visual_round(
            &policy,
            &malformed,
            VisualRoundEvent::EffectFinished(acknowledged_effect(&evidence, &malformed)),
        ),
        Err(VisualRoundError::InvalidSnapshot)
    );
}

#[test]
fn restored_awaiting_after_state_requires_an_effect_receipt() {
    let policy = policy();
    let evidence = evidence(1, 1_000);
    let mut malformed = authorized_snapshot(&policy, &evidence);
    malformed.phase = VisualRoundPhase::AwaitingAfterState;

    assert_eq!(
        decide_visual_round(
            &policy,
            &malformed,
            VisualRoundEvent::Classify {
                after_state: after_state(&evidence, VisualAfterStateOutcome::Passed, 1_200),
                now_ms: 1_200,
            },
        ),
        Err(VisualRoundError::InvalidSnapshot)
    );
}

#[test]
fn restored_selection_must_remain_inside_its_bound_candidate_set() {
    let policy = policy();
    let evidence = evidence(1, 1_000);
    let mut malformed = authorized_snapshot(&policy, &evidence);
    let selection = malformed.active_selection.as_mut().unwrap();
    selection.selected_candidate_ids = vec!["round:1:candidate:injected".to_string()];
    malformed.active_intent_digest = Some(visual_round_intent_digest(&evidence, selection));

    assert_eq!(
        decide_visual_round(
            &policy,
            &malformed,
            VisualRoundEvent::EffectFinished(acknowledged_effect(&evidence, &malformed)),
        ),
        Err(VisualRoundError::InvalidSnapshot)
    );
}

#[test]
fn two_round_pass_stays_inside_one_attempt_and_accumulates_budgets() {
    let policy = policy();
    let mut snapshot = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();

    let first = evidence(1, 1_000);
    let decision = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::Observe {
            evidence: first.clone(),
            now_ms: 1_000,
        },
    )
    .unwrap();
    assert!(matches!(
        decision,
        VisualRoundDecision::AwaitSelection { .. }
    ));
    snapshot = decision.snapshot().clone();

    let decision = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::Select {
            selection: selection(&first, &["round:1:candidate:1"]),
            now_ms: 1_100,
        },
    )
    .unwrap();
    let VisualRoundDecision::PermitIntent { intent, .. } = &decision else {
        panic!("first selection did not authorize one registered intent");
    };
    assert_eq!(intent.round_index, 1);
    assert!(!decision.emitted_effects());
    snapshot = decision.snapshot().clone();

    let decision = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::EffectFinished(acknowledged_effect(&first, &snapshot)),
    )
    .unwrap();
    assert!(matches!(
        decision,
        VisualRoundDecision::AwaitAfterState { .. }
    ));
    snapshot = decision.snapshot().clone();

    let second = evidence(2, 1_300);
    let mut next_state = after_state(&first, VisualAfterStateOutcome::NextRound, 1_200);
    next_state.frame_digest = second.frame_digest.clone();
    next_state.context_digest = second.context_digest.clone();
    next_state.geometry_digest = second.geometry_digest.clone();
    next_state.candidate_set_digest = second.candidate_set_digest.clone();
    let decision = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::Classify {
            after_state: next_state,
            now_ms: 1_200,
        },
    )
    .unwrap();
    assert!(matches!(decision, VisualRoundDecision::Continue { .. }));
    snapshot = decision.snapshot().clone();

    snapshot = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::Observe {
            evidence: second.clone(),
            now_ms: 1_300,
        },
    )
    .unwrap()
    .snapshot()
    .clone();
    snapshot = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::Select {
            selection: selection(&second, &["round:2:candidate:1", "round:2:candidate:2"]),
            now_ms: 1_400,
        },
    )
    .unwrap()
    .snapshot()
    .clone();
    snapshot = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::EffectFinished(acknowledged_effect(&second, &snapshot)),
    )
    .unwrap()
    .snapshot()
    .clone();
    snapshot = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::Classify {
            after_state: after_state(&second, VisualAfterStateOutcome::Passed, 1_500),
            now_ms: 1_500,
        },
    )
    .unwrap()
    .snapshot()
    .clone();

    assert_eq!(snapshot.task_id, "task:visual");
    assert_eq!(snapshot.attempt_id, "attempt:1");
    assert_eq!(snapshot.completed_rounds, 2);
    assert_eq!(snapshot.total_selections, 3);
    assert_eq!(snapshot.total_steps, 2);
    assert_eq!(snapshot.total_pointer_events, 8);
    assert_eq!(snapshot.total_key_events, 0);
    assert_eq!(
        snapshot.terminal_outcome,
        Some(VisualRoundTerminalOutcome::Passed)
    );
    assert_eq!(
        snapshot.terminal_receipt_ref.as_deref(),
        Some("receipt:visual:two-round")
    );
}

#[test]
fn selection_outside_the_bound_candidate_set_requires_intervention_before_intent() {
    let policy = policy();
    let evidence = evidence(1, 1_000);
    let initial = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
    let observed = decide_visual_round(
        &policy,
        &initial,
        VisualRoundEvent::Observe {
            evidence: evidence.clone(),
            now_ms: 1_000,
        },
    )
    .unwrap();
    let decision = decide_visual_round(
        &policy,
        observed.snapshot(),
        VisualRoundEvent::Select {
            selection: selection(&evidence, &["round:1:candidate:unknown"]),
            now_ms: 1_100,
        },
    )
    .unwrap();

    assert!(matches!(decision, VisualRoundDecision::Terminal { .. }));
    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::CandidateMismatch)
    );
    assert_eq!(decision.snapshot().total_selections, 0);
    assert!(!decision.emitted_effects());
}

#[test]
fn duplicate_candidate_selection_is_ambiguous_and_stops_before_intent() {
    let policy = policy();
    let evidence = evidence(1, 1_000);
    let initial = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
    let observed = decide_visual_round(
        &policy,
        &initial,
        VisualRoundEvent::Observe {
            evidence: evidence.clone(),
            now_ms: 1_000,
        },
    )
    .unwrap();
    let decision = decide_visual_round(
        &policy,
        observed.snapshot(),
        VisualRoundEvent::Select {
            selection: selection(&evidence, &["round:1:candidate:1", "round:1:candidate:1"]),
            now_ms: 1_100,
        },
    )
    .unwrap();

    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::CandidateMismatch)
    );
    assert_eq!(decision.snapshot().total_selections, 0);
    assert!(!matches!(
        decision,
        VisualRoundDecision::PermitIntent { .. }
    ));
}

#[test]
fn expired_round_evidence_requires_intervention_before_intent() {
    let policy = policy();
    let evidence = evidence(1, 1_000);
    let initial = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
    let observed = decide_visual_round(
        &policy,
        &initial,
        VisualRoundEvent::Observe {
            evidence: evidence.clone(),
            now_ms: 1_000,
        },
    )
    .unwrap();
    let decision = decide_visual_round(
        &policy,
        observed.snapshot(),
        VisualRoundEvent::Select {
            selection: selection(&evidence, &["round:1:candidate:1"]),
            now_ms: evidence.expires_at_ms,
        },
    )
    .unwrap();

    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::StaleEvidence)
    );
    assert!(!matches!(
        decision,
        VisualRoundDecision::PermitIntent { .. }
    ));
}

#[test]
fn next_round_frame_context_geometry_and_candidates_must_match_the_fresh_after_state() {
    let policy = policy();
    let first = evidence(1, 1_000);
    let second = evidence(2, 1_300);
    let snapshot = await_after_state(&policy, &first);
    let mut next_state = after_state(&first, VisualAfterStateOutcome::NextRound, 1_200);
    next_state.frame_digest = second.frame_digest.clone();
    next_state.context_digest = second.context_digest.clone();
    next_state.geometry_digest = second.geometry_digest.clone();
    next_state.candidate_set_digest = second.candidate_set_digest.clone();
    let continued = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::Classify {
            after_state: next_state,
            now_ms: 1_200,
        },
    )
    .unwrap();
    assert!(matches!(continued, VisualRoundDecision::Continue { .. }));

    for axis in 0..4 {
        let mut changed = second.clone();
        match axis {
            0 => changed.frame_digest = digest('a'),
            1 => changed.context_digest = digest('a'),
            2 => changed.geometry_digest = digest('a'),
            3 => {
                changed
                    .candidate_ids
                    .push("round:2:candidate:3".to_string());
                changed.candidate_set_digest = visual_candidate_set_digest(&changed.candidate_ids);
            }
            _ => unreachable!(),
        }
        changed.evidence_digest = visual_round_evidence_digest(&changed);
        let decision = decide_visual_round(
            &policy,
            continued.snapshot(),
            VisualRoundEvent::Observe {
                evidence: changed,
                now_ms: 1_300,
            },
        )
        .unwrap();

        assert_eq!(
            decision.snapshot().intervention,
            Some(VisualRoundInterventionReason::EvidenceChanged),
            "next-round evidence axis {axis}"
        );
        assert_eq!(decision.snapshot().completed_rounds, 1);
    }
}

#[test]
fn candidate_set_mutation_invalidates_the_evidence_envelope() {
    let policy = policy();
    let mut evidence = evidence(1, 1_000);
    evidence
        .candidate_ids
        .push("round:1:candidate:injected".to_string());
    let initial = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();

    let decision = decide_visual_round(
        &policy,
        &initial,
        VisualRoundEvent::Observe {
            evidence,
            now_ms: 1_000,
        },
    )
    .unwrap();

    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::InvalidEvidence)
    );
}

#[test]
fn policy_digest_mismatch_fails_closed_before_selection() {
    let policy = policy();
    let mut mismatched = evidence(1, 1_000);
    mismatched.policy_digest = digest('d');
    mismatched.evidence_digest = visual_round_evidence_digest(&mismatched);
    let initial = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();

    let decision = decide_visual_round(
        &policy,
        &initial,
        VisualRoundEvent::Observe {
            evidence: mismatched,
            now_ms: 1_000,
        },
    )
    .unwrap();

    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::InvalidEvidence)
    );
    assert!(!matches!(
        decision,
        VisualRoundDecision::AwaitSelection { .. }
    ));
}

#[test]
fn per_round_budget_exhaustion_stops_before_intent() {
    for axis in 0..4 {
        let mut policy = policy();
        let evidence = evidence(1, 1_000);
        let mut selected = selection(&evidence, &["round:1:candidate:1"]);
        match axis {
            0 => {
                policy.max_selections_per_round = 1;
                selected.selected_candidate_ids = evidence.candidate_ids.clone();
            }
            1 => selected.planned_steps = policy.max_steps_per_round + 1,
            2 => {
                selected.planned_pointer_events = policy.max_pointer_events_per_round + 1;
            }
            3 => selected.planned_key_events = policy.max_key_events_per_round + 1,
            _ => unreachable!(),
        }
        let initial = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
        let observed = decide_visual_round(
            &policy,
            &initial,
            VisualRoundEvent::Observe {
                evidence: evidence.clone(),
                now_ms: 1_000,
            },
        )
        .unwrap();
        let decision = decide_visual_round(
            &policy,
            observed.snapshot(),
            VisualRoundEvent::Select {
                selection: selected,
                now_ms: 1_100,
            },
        )
        .unwrap();

        assert_eq!(
            decision.snapshot().intervention,
            Some(VisualRoundInterventionReason::RoundBudgetExceeded),
            "per-round axis {axis}"
        );
        assert_eq!(decision.snapshot().total_selections, 0);
    }
}

#[test]
fn cumulative_budget_is_not_renewed_by_a_new_round() {
    for axis in 0..4 {
        let mut policy = policy();
        match axis {
            0 => policy.max_total_selections = 1,
            1 => policy.max_total_steps = 1,
            2 => policy.max_total_pointer_events = 4,
            3 => {
                policy.max_key_events_per_round = 1;
                policy.max_total_key_events = 0;
            }
            _ => unreachable!(),
        }
        let first = evidence(1, 1_000);
        let second = evidence(2, 1_300);
        let mut snapshot = await_after_state(&policy, &first);
        let mut next_state = after_state(&first, VisualAfterStateOutcome::NextRound, 1_200);
        next_state.frame_digest = second.frame_digest.clone();
        next_state.context_digest = second.context_digest.clone();
        next_state.geometry_digest = second.geometry_digest.clone();
        next_state.candidate_set_digest = second.candidate_set_digest.clone();
        snapshot = decide_visual_round(
            &policy,
            &snapshot,
            VisualRoundEvent::Classify {
                after_state: next_state,
                now_ms: 1_200,
            },
        )
        .unwrap()
        .snapshot()
        .clone();
        snapshot = decide_visual_round(
            &policy,
            &snapshot,
            VisualRoundEvent::Observe {
                evidence: second.clone(),
                now_ms: 1_300,
            },
        )
        .unwrap()
        .snapshot()
        .clone();
        let mut selected = selection(&second, &["round:2:candidate:1"]);
        if axis == 3 {
            selected.planned_key_events = 1;
        }
        let decision = decide_visual_round(
            &policy,
            &snapshot,
            VisualRoundEvent::Select {
                selection: selected,
                now_ms: 1_400,
            },
        )
        .unwrap();

        assert_eq!(
            decision.snapshot().intervention,
            Some(VisualRoundInterventionReason::CumulativeBudgetExceeded),
            "cumulative axis {axis}"
        );
        assert_eq!(decision.snapshot().completed_rounds, 1);
    }
}

#[test]
fn selection_budget_boundaries_do_not_saturate_cumulative_totals() {
    let mut policy = policy();
    policy.max_selections_per_round = 250;
    policy.max_total_selections = u8::MAX;

    let mut evidence = evidence(2, 1_000);
    evidence.candidate_ids = (0..10)
        .map(|index| format!("round:2:candidate:{index}"))
        .collect();
    evidence.candidate_set_digest = visual_candidate_set_digest(&evidence.candidate_ids);
    evidence.evidence_digest = visual_round_evidence_digest(&evidence);

    let mut near_limit = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
    near_limit.phase = VisualRoundPhase::AwaitingSelection;
    near_limit.next_round_index = 2;
    near_limit.completed_rounds = 1;
    near_limit.total_selections = 250;
    near_limit.completed_round_ids = vec!["round:1".to_string()];
    near_limit.active_evidence = Some(evidence.clone());
    let selected = VisualRoundSelection {
        round_id: evidence.round_id.clone(),
        evidence_digest: evidence.evidence_digest.clone(),
        candidate_set_digest: evidence.candidate_set_digest.clone(),
        selected_candidate_ids: evidence.candidate_ids.clone(),
        planned_steps: 1,
        planned_pointer_events: 4,
        planned_key_events: 0,
        provider_capability: capability(),
    };

    let decision = decide_visual_round(
        &policy,
        &near_limit,
        VisualRoundEvent::Select {
            selection: selected,
            now_ms: 1_100,
        },
    )
    .unwrap();

    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::CumulativeBudgetExceeded)
    );
    assert!(!matches!(
        decision,
        VisualRoundDecision::PermitIntent { .. }
    ));
}

#[test]
fn selection_budget_boundaries_report_typed_intervention_for_unrepresentable_count() {
    let mut policy = policy();
    policy.max_selections_per_round = u8::MAX;
    policy.max_total_selections = u8::MAX;

    let mut evidence = evidence(1, 1_000);
    evidence.candidate_ids = (0..=u8::MAX)
        .map(|index| format!("round:1:candidate:{index}"))
        .collect();
    evidence.candidate_set_digest = visual_candidate_set_digest(&evidence.candidate_ids);
    evidence.evidence_digest = visual_round_evidence_digest(&evidence);

    let initial = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
    let observed = decide_visual_round(
        &policy,
        &initial,
        VisualRoundEvent::Observe {
            evidence: evidence.clone(),
            now_ms: 1_000,
        },
    )
    .unwrap();
    let selected = VisualRoundSelection {
        round_id: evidence.round_id.clone(),
        evidence_digest: evidence.evidence_digest.clone(),
        candidate_set_digest: evidence.candidate_set_digest.clone(),
        selected_candidate_ids: evidence.candidate_ids.clone(),
        planned_steps: 1,
        planned_pointer_events: 4,
        planned_key_events: 0,
        provider_capability: capability(),
    };

    let decision = decide_visual_round(
        &policy,
        observed.snapshot(),
        VisualRoundEvent::Select {
            selection: selected,
            now_ms: 1_100,
        },
    )
    .unwrap();

    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::RoundBudgetExceeded)
    );
    assert!(!matches!(
        decision,
        VisualRoundDecision::PermitIntent { .. }
    ));
}

#[test]
fn partial_and_uncertain_effects_never_continue_to_another_round() {
    let policy = policy();
    for (delivery, reason) in [
        (
            DeliveryState::Partial,
            VisualRoundInterventionReason::PartialEffect,
        ),
        (
            DeliveryState::Uncertain,
            VisualRoundInterventionReason::UncertainEffect,
        ),
    ] {
        let evidence = evidence(1, 1_000);
        let snapshot = authorized_snapshot(&policy, &evidence);
        let mut receipt = acknowledged_effect(&evidence, &snapshot);
        receipt.delivery = delivery;

        let decision = decide_visual_round(
            &policy,
            &snapshot,
            VisualRoundEvent::EffectFinished(receipt),
        )
        .unwrap();

        assert!(matches!(decision, VisualRoundDecision::Terminal { .. }));
        assert_eq!(decision.snapshot().intervention, Some(reason));
        assert_eq!(decision.snapshot().completed_rounds, 0);
        assert_eq!(decision.snapshot().total_selections, 1);
        assert!(!decision.emitted_effects());
    }
}

#[test]
fn exact_terminal_replay_returns_no_new_intent_or_effect() {
    let policy = policy();
    let evidence = evidence(1, 1_000);
    let snapshot = await_after_state(&policy, &evidence);
    let terminal = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::Classify {
            after_state: after_state(&evidence, VisualAfterStateOutcome::Passed, 1_200),
            now_ms: 1_200,
        },
    )
    .unwrap();
    let terminal_snapshot = terminal.snapshot().clone();

    let replay = decide_visual_round(
        &policy,
        &terminal_snapshot,
        VisualRoundEvent::Replay {
            original_receipt_ref: "receipt:visual:two-round".to_string(),
        },
    )
    .unwrap();
    assert!(matches!(
        replay,
        VisualRoundDecision::Replay {
            emitted_new_effects: false,
            ..
        }
    ));
    assert_eq!(replay.snapshot(), &terminal_snapshot);
    assert!(!replay.emitted_effects());

    assert_eq!(
        decide_visual_round(
            &policy,
            &terminal_snapshot,
            VisualRoundEvent::Replay {
                original_receipt_ref: "receipt:wrong".to_string(),
            },
        ),
        Err(VisualRoundError::InvalidTransition)
    );
}

#[test]
fn provider_capability_mismatch_stops_before_intent() {
    let policy = policy();
    let evidence = evidence(1, 1_000);
    let initial = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
    let observed = decide_visual_round(
        &policy,
        &initial,
        VisualRoundEvent::Observe {
            evidence: evidence.clone(),
            now_ms: 1_000,
        },
    )
    .unwrap();
    let mut mismatched = selection(&evidence, &["round:1:candidate:1"]);
    mismatched.provider_capability.capability_digest = digest('d');

    let decision = decide_visual_round(
        &policy,
        observed.snapshot(),
        VisualRoundEvent::Select {
            selection: mismatched,
            now_ms: 1_100,
        },
    )
    .unwrap();

    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::CapabilityMismatch)
    );
    assert_eq!(decision.snapshot().total_selections, 0);
}

#[test]
fn skipped_round_index_fails_closed() {
    let policy = policy();
    let initial = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
    let decision = decide_visual_round(
        &policy,
        &initial,
        VisualRoundEvent::Observe {
            evidence: evidence(2, 1_000),
            now_ms: 1_000,
        },
    )
    .unwrap();

    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::RoundOrder)
    );
}

#[test]
fn repeated_or_reordered_rounds_fail_closed_after_continuation() {
    let policy = policy();
    let first = evidence(1, 1_000);
    let second = evidence(2, 1_300);
    let snapshot = await_after_state(&policy, &first);
    let mut next_state = after_state(&first, VisualAfterStateOutcome::NextRound, 1_200);
    next_state.frame_digest = second.frame_digest.clone();
    next_state.context_digest = second.context_digest.clone();
    next_state.geometry_digest = second.geometry_digest.clone();
    next_state.candidate_set_digest = second.candidate_set_digest.clone();
    let continued = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::Classify {
            after_state: next_state,
            now_ms: 1_200,
        },
    )
    .unwrap();

    let mut repeated_identity = second.clone();
    repeated_identity.round_id = first.round_id.clone();
    repeated_identity.evidence_digest = visual_round_evidence_digest(&repeated_identity);
    let mut reordered_index = second;
    reordered_index.round_index = 1;
    reordered_index.evidence_digest = visual_round_evidence_digest(&reordered_index);

    for invalid in [repeated_identity, reordered_index] {
        let decision = decide_visual_round(
            &policy,
            continued.snapshot(),
            VisualRoundEvent::Observe {
                evidence: invalid,
                now_ms: 1_300,
            },
        )
        .unwrap();

        assert_eq!(
            decision.snapshot().intervention,
            Some(VisualRoundInterventionReason::RoundOrder)
        );
        assert_eq!(decision.snapshot().completed_rounds, 1);
    }
}

#[test]
fn unexpected_extra_round_exhausts_the_round_budget() {
    let mut policy = policy();
    policy.max_rounds = 1;
    let evidence = evidence(1, 1_000);
    let snapshot = await_after_state(&policy, &evidence);
    let decision = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::Classify {
            after_state: after_state(&evidence, VisualAfterStateOutcome::NextRound, 1_200),
            now_ms: 1_200,
        },
    )
    .unwrap();

    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::UnexpectedRound)
    );
    assert_eq!(decision.snapshot().completed_rounds, 1);
}

#[test]
fn effect_counts_must_match_the_pre_effect_intent() {
    let policy = policy();
    let evidence = evidence(1, 1_000);
    let snapshot = authorized_snapshot(&policy, &evidence);
    let mut receipt = acknowledged_effect(&evidence, &snapshot);
    receipt.pointer_events = 3;

    let decision = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::EffectFinished(receipt),
    )
    .unwrap();

    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::EffectMismatch)
    );
    assert_eq!(decision.snapshot().total_pointer_events, 0);
}

#[test]
fn effect_receipt_must_bind_the_exact_authorized_intent() {
    let policy = policy();
    let evidence = evidence(1, 1_000);
    let snapshot = authorized_snapshot(&policy, &evidence);
    let mut receipt = acknowledged_effect(&evidence, &snapshot);
    receipt.intent_digest = digest('d');

    let decision = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::EffectFinished(receipt),
    )
    .unwrap();

    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::EffectMismatch)
    );
    assert_eq!(decision.snapshot().total_selections, 0);
}

#[test]
fn ambiguous_after_state_requires_human_intervention() {
    let policy = policy();
    let evidence = evidence(1, 1_000);
    let snapshot = await_after_state(&policy, &evidence);
    let decision = decide_visual_round(
        &policy,
        &snapshot,
        VisualRoundEvent::Classify {
            after_state: after_state(&evidence, VisualAfterStateOutcome::Ambiguous, 1_200),
            now_ms: 1_200,
        },
    )
    .unwrap();

    assert_eq!(
        decision.snapshot().terminal_outcome,
        Some(VisualRoundTerminalOutcome::InterventionRequired)
    );
    assert_eq!(
        decision.snapshot().intervention,
        Some(VisualRoundInterventionReason::Ambiguous)
    );
}
