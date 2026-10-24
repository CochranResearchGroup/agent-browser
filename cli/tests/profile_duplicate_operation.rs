#[path = "../src/native/profile_duplicate_operation.rs"]
mod profile_duplicate_operation;
use profile_duplicate_operation::*;

fn intent() -> DuplicateIntent {
    DuplicateIntent { operation_id: "copy-1".into(), source_profile_id: "source".into(), destination_profile_id: "independent".into(), browser_build: "chrome-for-testing-123".into(), source_generation: 7 }
}

#[test]
fn running_source_offers_owner_choices_without_peer_policy_veto() {
    let operation = ProfileDuplicateOperation::prepare(intent(), vec!["own-task".into(), "peer-task".into()]).unwrap();
    assert_eq!(operation.phase, DuplicatePhase::AwaitingOwnerChoice);
    assert_eq!(operation.affected_task_ids, ["own-task", "peer-task"]);
    assert_eq!(operation.intent.destination_profile_id, "independent");
}
