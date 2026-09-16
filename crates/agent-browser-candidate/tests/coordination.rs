use agent_browser_candidate::{
    ArtifactPinReason, CoordinationAction, CoordinationLedger, CoordinationOutcome,
    CoordinationRequest,
};

fn request(
    ledger: &CoordinationLedger,
    request_id: &str,
    candidate_id: &str,
    artifact_id: &str,
    action: CoordinationAction,
) -> CoordinationRequest {
    CoordinationRequest {
        request_id: request_id.to_string(),
        candidate_id: candidate_id.to_string(),
        artifact_id: artifact_id.to_string(),
        expected_revision: ledger.revision,
        expected_fencing_generation: ledger.fencing_generation,
        action,
    }
}

#[test]
fn same_candidate_joins_and_exact_request_replays_without_mutation() {
    let mut ledger = CoordinationLedger::new("production");
    let started = ledger
        .apply(request(
            &ledger,
            "request-1",
            "candidate-a",
            "artifact-a",
            CoordinationAction::StartOrJoin,
        ))
        .expect("start");
    assert_eq!(started.outcome, CoordinationOutcome::Started);

    let join_request = request(
        &ledger,
        "request-2",
        "candidate-a",
        "artifact-a",
        CoordinationAction::StartOrJoin,
    );
    let joined = ledger.apply(join_request.clone()).expect("join");
    assert_eq!(joined.outcome, CoordinationOutcome::JoinedExisting);
    let revision_after_join = ledger.revision;

    let replay = ledger.apply(join_request).expect("replay");
    assert_eq!(replay, joined);
    assert_eq!(ledger.revision, revision_after_join);
    assert_eq!(ledger.receipts().len(), 2);
}

#[test]
fn competing_candidates_queue_fifo_and_keep_artifacts_pinned() {
    let mut ledger = CoordinationLedger::new("production");
    ledger
        .apply(request(
            &ledger,
            "request-a",
            "candidate-a",
            "artifact-a",
            CoordinationAction::StartOrJoin,
        ))
        .expect("start a");
    let queued_b = ledger
        .apply(request(
            &ledger,
            "request-b",
            "candidate-b",
            "artifact-b",
            CoordinationAction::Queue,
        ))
        .expect("queue b");
    let queued_c = ledger
        .apply(request(
            &ledger,
            "request-c",
            "candidate-c",
            "artifact-c",
            CoordinationAction::Queue,
        ))
        .expect("queue c");

    assert_eq!(queued_b.outcome, CoordinationOutcome::Queued);
    assert_eq!(queued_c.outcome, CoordinationOutcome::Queued);
    assert_eq!(ledger.queue()[0].candidate_id, "candidate-b");
    assert_eq!(ledger.queue()[1].candidate_id, "candidate-c");
    assert_eq!(
        ledger.artifact_pins().get("artifact-a"),
        Some(&vec![ArtifactPinReason::Active])
    );
    assert_eq!(
        ledger.artifact_pins().get("artifact-b"),
        Some(&vec![ArtifactPinReason::Queued])
    );

    let active_id = ledger
        .active()
        .expect("active operation")
        .operation_id
        .clone();
    ledger
        .apply(request(
            &ledger,
            "request-cancel-a",
            "candidate-a",
            "artifact-a",
            CoordinationAction::CancelActive {
                operation_id: active_id,
            },
        ))
        .expect("cancel a");
    let first_queued_id = ledger.queue()[0].operation_id.clone();
    let activated = ledger
        .apply(request(
            &ledger,
            "request-activate-b",
            "candidate-b",
            "artifact-b",
            CoordinationAction::ActivateQueued {
                operation_id: first_queued_id,
            },
        ))
        .expect("activate b");
    assert_eq!(activated.outcome, CoordinationOutcome::ActivatedQueued);
    assert_eq!(
        ledger.active().expect("active b").candidate_id,
        "candidate-b"
    );
    assert_eq!(ledger.queue()[0].candidate_id, "candidate-c");
}

#[test]
fn supersede_advances_fence_and_rejects_the_old_writer() {
    let mut ledger = CoordinationLedger::new("production");
    ledger
        .apply(request(
            &ledger,
            "request-a",
            "candidate-a",
            "artifact-a",
            CoordinationAction::StartOrJoin,
        ))
        .expect("start a");
    let old_revision = ledger.revision;
    let old_fence = ledger.fencing_generation;
    let active_id = ledger.active().expect("active a").operation_id.clone();

    let superseded = ledger
        .apply(request(
            &ledger,
            "request-b",
            "candidate-b",
            "artifact-b",
            CoordinationAction::Supersede {
                operation_id: active_id,
            },
        ))
        .expect("supersede");
    assert_eq!(superseded.outcome, CoordinationOutcome::Superseded);
    assert!(ledger.fencing_generation > old_fence);
    assert_eq!(
        ledger.active().expect("active b").candidate_id,
        "candidate-b"
    );

    let stale = ledger
        .apply(CoordinationRequest {
            request_id: "request-stale".to_string(),
            candidate_id: "candidate-a".to_string(),
            artifact_id: "artifact-a".to_string(),
            expected_revision: old_revision,
            expected_fencing_generation: old_fence,
            action: CoordinationAction::CompleteActive {
                operation_id: superseded.operation_id.expect("new operation id"),
            },
        })
        .expect_err("old writer must be fenced");
    assert_eq!(stale.code(), "stale_fencing_generation");
}

#[test]
fn discarded_queue_entry_releases_its_only_pin() {
    let mut ledger = CoordinationLedger::new("production");
    ledger
        .apply(request(
            &ledger,
            "request-a",
            "candidate-a",
            "artifact-a",
            CoordinationAction::StartOrJoin,
        ))
        .expect("start a");
    ledger
        .apply(request(
            &ledger,
            "request-b",
            "candidate-b",
            "artifact-b",
            CoordinationAction::Queue,
        ))
        .expect("queue b");
    let queued_id = ledger.queue()[0].operation_id.clone();
    let discarded = ledger
        .apply(request(
            &ledger,
            "request-discard-b",
            "candidate-b",
            "artifact-b",
            CoordinationAction::DiscardQueued {
                operation_id: queued_id,
            },
        ))
        .expect("discard b");

    assert_eq!(discarded.outcome, CoordinationOutcome::Discarded);
    assert!(!ledger.artifact_pins().contains_key("artifact-b"));
    assert!(ledger.queue().is_empty());
}

#[test]
fn exhausted_counters_fail_without_partial_mutation() {
    let mut ledger = CoordinationLedger::new("production");
    ledger.fencing_generation = u64::MAX;
    let before = ledger.clone();
    let error = ledger
        .apply(request(
            &ledger,
            "request-a",
            "candidate-a",
            "artifact-a",
            CoordinationAction::StartOrJoin,
        ))
        .expect_err("exhausted fence must fail");
    assert_eq!(error.code(), "fencing_generation_exhausted");
    assert_eq!(ledger, before);

    ledger.fencing_generation = 0;
    ledger.revision = u64::MAX;
    let before = ledger.clone();
    let error = ledger
        .apply(request(
            &ledger,
            "request-b",
            "candidate-b",
            "artifact-b",
            CoordinationAction::Queue,
        ))
        .expect_err("exhausted revision must fail");
    assert_eq!(error.code(), "revision_exhausted");
    assert_eq!(ledger, before);
}
