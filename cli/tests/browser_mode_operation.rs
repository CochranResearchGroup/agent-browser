#[path = "../src/native/browser_mode_operation.rs"]
mod browser_mode_operation;
use browser_mode_operation::*;

fn request() -> Request {
    Request {
        operation_id: "convert-1".into(),
        browser_id: "browser-1".into(),
        profile_id: "profile-1".into(),
        original_process: "boot:pid:start".into(),
        executable: "/synthetic/chrome".into(),
        task_ids: vec!["task-1".into(), "peer-task".into()],
        requested_mode: Mode::CdpFree,
    }
}
#[test]
fn close_prompt_is_durable_and_offers_owner_remediation_without_replacing_identity() {
    let directory = std::env::temp_dir().join(format!("mode-operation-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("operations.sqlite3");
    let mut store = OperationStore::open(&path).unwrap();
    let initial = store.request(request()).unwrap();
    assert_eq!(
        initial.actions(),
        vec![Action::GracefulClose, Action::Cancel]
    );
    store
        .observe("convert-1", "boot:pid:start", Observation::ClosePrompt)
        .unwrap();
    drop(store);
    let mut reopened = OperationStore::open(&path).unwrap();
    let retained = reopened.request(request()).unwrap();
    assert_eq!(retained.phase, Phase::ClosePrompt);
    assert_eq!(retained.request.task_ids, vec!["task-1", "peer-task"]);
    assert_eq!(
        retained.actions(),
        vec![
            Action::ReturnToBrowser,
            Action::RetryGracefulClose,
            Action::ConfirmForceClose,
            Action::Cancel
        ]
    );
    drop(reopened);
    std::fs::remove_dir_all(directory).unwrap();
}
