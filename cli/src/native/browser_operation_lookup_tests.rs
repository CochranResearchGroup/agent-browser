use super::*;
use serde_json::json;

#[test]
fn operation_lookup_reads_original_mode_without_effects_or_private_paths() {
    let fixture = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let opened = host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let path = fixture
        .root
        .join("runtime.sqlite3")
        .with_extension("mode.sqlite3");
    let mut operations =
        crate::native::browser_mode_operation::OperationStore::open(&path).unwrap();
    operations
        .request(crate::native::browser_mode_operation::Request {
            operation_id: "retained-mode".into(),
            browser_id: opened.browser_id.clone(),
            profile_id: "profile-a".into(),
            original_process: "private-process-evidence".into(),
            executable: "/private/profile/chrome".into(),
            task_ids: vec![opened.session_id.clone()],
            requested_mode: crate::native::browser_mode_operation::Mode::CdpFree,
        })
        .unwrap();
    drop(host);
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let before = host.state().clone();
    let effect_counts = (calls.get(), requests.get());
    let reply = host.handle_command(&json!({"action":"browser_session_mode_convert","sessionId":opened.session_id,"sessionName":"Alice","choice":"inspect"}));
    assert_eq!(reply["success"], true, "{reply}");
    assert_eq!(
        reply["data"]["operations"][0]["operationId"],
        "retained-mode"
    );
    assert_eq!(reply["data"]["operations"][0]["phase"], "prepared");
    assert_eq!(
        reply["data"]["operations"][0]["actions"][0]["action"],
        "relaunch"
    );
    assert_eq!(host.state(), &before);
    assert_eq!((calls.get(), requests.get()), effect_counts);
    assert!(!reply.to_string().contains("private"));
    let busy = host.handle_command(&json!({"action":"browser_session_mode_convert","sessionId":opened.session_id,"sessionName":"Alice","operationId":"new-id-after-reload","mode":"cdp_free","choice":"prepare"}));
    assert_eq!(busy["success"], true, "{busy}");
    assert_eq!(busy["data"]["state"], "busy");
    assert_eq!(
        busy["data"]["activeOperation"]["operationId"],
        "retained-mode"
    );
    assert_eq!(busy["data"]["activeOperation"]["taskId"], opened.session_id);
    assert_eq!(
        busy["data"]["activeOperation"]["actions"][0]["action"],
        "relaunch"
    );
    assert_eq!(host.state(), &before);
    assert_eq!((calls.get(), requests.get()), effect_counts);
    assert!(
        operations.read("new-id-after-reload").unwrap().is_none(),
        "Busy requests never create new owner intent"
    );

    let refused = host.handle_command(&json!({"action":"browser_session_mode_convert","sessionId":opened.session_id,"sessionName":"Mallory","choice":"inspect"}));
    assert_eq!(refused["success"], false, "{refused}");
}

#[test]
fn operation_lookup_recovers_multiple_offline_copy_intents_without_effects() {
    let fixture = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let inspect = json!({"action":"browser_session_profile_duplicate","sourceProfileId":"profile-a","choice":"inspect"});
    let empty = host.handle_command(&inspect);
    assert_eq!(empty["success"], true, "{empty}");
    assert_eq!(empty["data"]["operations"], json!([]));
    assert!(
        !fixture.root.join("profile-duplication").exists(),
        "Inspection cannot initialize a copy journal"
    );
    for id in ["copy-one", "copy-two"] {
        let prepared = host.handle_command(&json!({"action":"browser_session_profile_duplicate","operationId":id,"sourceProfileId":"profile-a","destinationName":id,"launch":false,"choice":"prepare"}));
        assert_eq!(prepared["success"], true, "{prepared}");
        assert_eq!(prepared["data"]["phase"], "source_preparation_required");
    }
    drop(host);
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let before = host.state().clone();
    let readback = host.handle_command(&inspect);
    assert_eq!(readback["success"], true, "{readback}");
    assert_eq!(readback["data"]["operations"].as_array().unwrap().len(), 2);
    assert_eq!(readback["data"]["operations"][0]["operationId"], "copy-one");
    assert_eq!(readback["data"]["operations"][1]["operationId"], "copy-two");
    assert_eq!(
        readback["data"]["operations"][1]["actions"][0]["action"],
        "open_source"
    );
    assert!(!readback
        .to_string()
        .contains(fixture.root.to_str().unwrap()));
    assert_eq!(host.state(), &before);
    assert_eq!(
        (calls.get(), requests.get()),
        (0, 0),
        "Preparing and reading offline intent never launches or contacts a desktop"
    );
    let other = host.handle_command(&json!({"action":"browser_session_profile_duplicate","sourceProfileId":"profile-b","operationId":"copy-one","choice":"inspect"}));
    assert_eq!(other["success"], false, "{other}");
    let selected = host.handle_command(&json!({"action":"browser_session_profile_duplicate","sourceProfileId":"profile-a","operationId":"copy-two","choice":"inspect"}));
    assert_eq!(selected["success"], true, "{selected}");
    assert_eq!(selected["data"]["operations"].as_array().unwrap().len(), 1);
    assert_eq!(selected["data"]["operations"][0]["operationId"], "copy-two");
}
