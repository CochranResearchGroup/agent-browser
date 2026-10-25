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
            executable_sha256: None,
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
    let ids = readback["data"]["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|operation| operation["operationId"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        ids,
        std::collections::BTreeSet::from(["copy-one", "copy-two"])
    );
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

#[test]
fn operation_lookup_keeps_current_mode_discoverable_after_long_scoped_history() {
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
    let mut store = crate::native::browser_mode_operation::OperationStore::open(&path).unwrap();
    let current = store
        .request(crate::native::browser_mode_operation::Request {
            operation_id: "zz-current-mode".into(),
            browser_id: opened.browser_id.clone(),
            profile_id: "profile-a".into(),
            original_process: "private-process-evidence".into(),
            executable: "/private/profile/chrome".into(),
            executable_sha256: None,
            task_ids: vec![opened.session_id.clone()],
            requested_mode: crate::native::browser_mode_operation::Mode::CdpFree,
        })
        .unwrap();
    // A journal fixture supplies retained history; assertions stay at the owner command seam.
    let mut journal = rusqlite::Connection::open(&path).unwrap();
    let transaction = journal.transaction().unwrap();
    for index in 0..300 {
        for wrong_scope in [false, true] {
            let mut historical = current.clone();
            historical.request.operation_id = format!("history-{wrong_scope}-{index:04}");
            historical.phase = crate::native::browser_mode_operation::Phase::Cancelled;
            if wrong_scope {
                historical.request.task_ids = vec!["unrelated-task".into()];
            }
            transaction.execute("INSERT INTO browser_mode_operations(id,browser_id,terminal,state) VALUES(?1,?2,1,?3)", rusqlite::params![historical.request.operation_id, historical.request.browser_id, serde_json::to_string(&historical).unwrap()]).unwrap();
        }
    }
    transaction.commit().unwrap();
    drop(host);
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let before = host.state().clone();
    let effects_before = (calls.get(), requests.get());
    let response = host.handle_command(&json!({"action":"browser_session_mode_convert","sessionId":opened.session_id,"sessionName":"Alice","choice":"inspect"}));
    assert_eq!(
        response["success"], true,
        "Long history must not hide the operation: {response}"
    );
    let operations = response["data"]["operations"].as_array().unwrap();
    assert_eq!(operations[0]["operationId"], "zz-current-mode");
    assert_eq!(
        operations.len(),
        256,
        "Unrelated tasks cannot consume the scoped history budget"
    );
    assert_eq!(response["data"]["historyTruncated"], true);
    assert!(operations
        .iter()
        .all(|operation| !operation["operationId"].as_str().unwrap().contains("true")));
    assert!(operations
        .iter()
        .all(|operation| operation["taskId"] == opened.session_id));
    assert!(!response.to_string().contains("private"));
    assert_eq!(host.state(), &before);
    assert_eq!((calls.get(), requests.get()), effects_before);
    let busy = host.handle_command(&json!({"action":"browser_session_mode_convert","sessionId":opened.session_id,"sessionName":"Alice","operationId":"new-after-reload","mode":"cdp_free","choice":"prepare"}));
    assert_eq!(
        busy["data"]["activeOperation"]["operationId"], "zz-current-mode",
        "{busy}"
    );
    assert_eq!(host.state(), &before);
    assert_eq!((calls.get(), requests.get()), effects_before);
}

#[test]
fn operation_lookup_keeps_current_copies_discoverable_after_long_scoped_history() {
    let fixture = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let mut destinations = std::collections::BTreeMap::new();
    for id in ["zz-current-copy", "zy-pending-copy"] {
        let prepared = host.handle_command(&json!({"action":"browser_session_profile_duplicate","sourceProfileId":"profile-a","operationId":id,"destinationName":id,"launch":false,"choice":"prepare"}));
        assert_eq!(prepared["success"], true, "{prepared}");
        destinations.insert(id.to_owned(), prepared["data"]["destination"].clone());
    }
    let opened = host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let mut state = host.state().clone();
    let tab = ManagedBrowserTab {
        id: "retained-copy-tab".into(),
        target_id: "synthetic-target".into(),
        browser_id: opened.browser_id.clone(),
        session_id: opened.session_id.clone(),
        created_at_ms: 100,
        last_activity_at_ms: 100,
    };
    state.tabs.insert(tab.id.clone(), tab.clone());
    state
        .sessions
        .get_mut(&opened.session_id)
        .unwrap()
        .current_tab_id = Some(tab.id.clone());
    fixture.store(false).save_session_state(&state).unwrap();
    let browser = state.browsers[&opened.browser_id].clone();
    let session = state.sessions[&opened.session_id].clone();
    let path = fixture.root.join("profile-duplication/operations.sqlite3");
    let mut journal = rusqlite::Connection::open(&path).unwrap();
    journal.execute_batch("CREATE TABLE profile_duplicate_operations(id TEXT PRIMARY KEY,destination TEXT NOT NULL,terminal INTEGER NOT NULL DEFAULT 0,state TEXT NOT NULL)").unwrap();
    let transaction = journal.transaction().unwrap();
    // Seed valid retained journal records; only public owner replies are asserted below.
    let seed = |id: &str, wrong_task: bool, terminal: bool| {
        let destination = destinations
            .get(id)
            .cloned()
            .unwrap_or_else(|| json!({"id":format!("destination-{id}"),"name":id}));
        if terminal {
            let intent = json!({"operation_id":id,"source_profile_id":"profile-a","destination_profile_id":destination["id"],"destination_name":destination["name"],"launch":false,"cancelled":false});
            transaction
                .execute(
                    "INSERT INTO profile_duplicate_host_intents(id,intent) VALUES(?1,?2)",
                    rusqlite::params![id, intent.to_string()],
                )
                .unwrap();
        }
        let mut session = session.clone();
        let mut tab = tab.clone();
        let mut browser = browser.clone();
        if wrong_task {
            session.id = "unrelated-task".into();
            tab.session_id = session.id.clone();
            browser.active_session_ids = vec![session.id.clone()];
        }
        let binding = json!({"borrowed_anchor":null,"original":{"pid":browser.pid,"startToken":"private-process-evidence","executablePath":"/private/chrome","browserFamily":null},"browser":browser,"session":session,"tab":tab,"executable":"/private/chrome","digest":"synthetic-build","close_dispatched":false,"clean_close":false,"reviewed_tasks":[session.id]});
        transaction
            .execute(
                "INSERT INTO profile_duplicate_owner_bindings(id,binding) VALUES(?1,?2)",
                rusqlite::params![id, binding.to_string()],
            )
            .unwrap();
        let operation = json!({"request":{"operationId":id,"source":{"id":"profile-a","name":"Source","userDataDir":fixture.root.join("profile-a"),"browserBuild":"fixture-build","generation":1},"destination":{"id":destination["id"],"name":destination["name"],"userDataDir":fixture.root.join(id)},"launch":false},"phase":if terminal {"cancelled"} else {"prepared"},"affectedTaskIds":[session.id],"duplicate":null,"launch":null,"destinationTask":null,"failure":null});
        transaction.execute("INSERT INTO profile_duplicate_operations(id,destination,terminal,state) VALUES(?1,?2,?3,?4)",rusqlite::params![id,destination["id"].as_str().unwrap(),terminal,operation.to_string()]).unwrap();
    };
    seed("zz-current-copy", false, false);
    seed("zy-pending-copy", false, false);
    for index in 0..300 {
        for wrong_task in [false, true] {
            seed(
                &format!("history-{wrong_task}-{index:04}"),
                wrong_task,
                true,
            );
        }
    }
    transaction.commit().unwrap();
    let persistence = fixture.store(false);
    let mut persisted = persistence.load_session_state().unwrap();
    persisted
        .profile_operation_ids
        .insert("profile-a".into(), "zz-current-copy".into());
    persistence.save_session_state(&persisted).unwrap();
    drop(host);
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let before = host.state().clone();
    let effects_before = (calls.get(), requests.get());
    let inspect = json!({"action":"browser_session_profile_duplicate","sourceProfileId":"profile-a","sessionId":opened.session_id,"sessionName":"Alice","choice":"inspect"});
    let response = host.handle_command(&inspect);
    assert_eq!(
        response["success"], true,
        "Long copy history must not hide pending work: {response}"
    );
    let operations = response["data"]["operations"].as_array().unwrap();
    assert_eq!(
        operations.len(),
        256,
        "Wrong-task history cannot consume the scoped budget"
    );
    assert_eq!(operations[0]["operationId"], "zz-current-copy");
    assert_eq!(operations[1]["operationId"], "zy-pending-copy");
    assert_eq!(response["data"]["historyTruncated"], true);
    assert!(operations
        .iter()
        .all(|operation| !operation["operationId"].as_str().unwrap().contains("true")));
    assert!(!response.to_string().contains("private"));
    assert_eq!(host.state(), &before);
    assert_eq!((calls.get(), requests.get()), effects_before);
    let mut exact = inspect.clone();
    exact["operationId"] = json!("history-false-0000");
    let older = host.handle_command(&exact);
    assert_eq!(older["success"], true, "{older}");
    assert_eq!(older["data"]["operations"].as_array().unwrap().len(), 1);
    assert_eq!(
        older["data"]["operations"][0]["operationId"],
        "history-false-0000"
    );
    assert_eq!(older["data"]["historyTruncated"], false);
    let busy = host.handle_command(&json!({"action":"browser_session_profile_duplicate","sourceProfileId":"profile-a","sessionId":opened.session_id,"sessionName":"Alice","operationId":"new-copy-after-reload","choice":"prepare"}));
    assert_eq!(
        busy["data"]["activeOperation"]["operationId"], "zz-current-copy",
        "{busy}"
    );
    assert_eq!(host.state(), &before);
    assert_eq!((calls.get(), requests.get()), effects_before);
}

#[cfg(target_os = "linux")]
#[test]
fn mode_binding_uses_actual_borrowed_process_posture_instead_of_catalog_intent() {
    use std::process::{Child, Command};
    struct ProcessGuard(Child);
    impl Drop for ProcessGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let fixture = Fixture::new();
    // This is a provider-free process identity oracle, not a real Chrome claim.
    // Its complete argv has an explicit debugging flag and its copied ELF name
    // exercises the same OS observer used by owner binding.
    let executable = fixture.root.join("chrome");
    std::fs::copy("/usr/bin/python3", &executable).unwrap();
    let process = ProcessGuard(
        Command::new(&executable)
            .args([
                "-c",
                "import time; time.sleep(60)",
                "--remote-debugging-port=0",
            ])
            .spawn()
            .unwrap(),
    );
    let crate::process_identity::ProcessObservation::Observed(observed) =
        crate::process_identity::observe_process(process.0.id())
    else {
        panic!("exact fixture process unavailable")
    };
    let original = RecordedProcessIdentity {
        pid: process.0.id(),
        start_token: observed.start_token.unwrap(),
        executable_path: observed.executable_path,
        browser_family: observed.browser_family,
    };
    assert_eq!(original.browser_family.as_deref(), Some("chrome"));
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let opened = host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    host.navigate(&opened.session_id, "https://synthetic.example/", 101)
        .unwrap();
    let mut state = host.state().clone();
    let tab_id = state.sessions[&opened.session_id]
        .current_tab_id
        .clone()
        .unwrap();
    let target_id = state.tabs[&tab_id].target_id.clone();
    state.shared_browser_ids.insert(opened.browser_id.clone());
    let browser = state.browsers.get_mut(&opened.browser_id).unwrap();
    browser.pid = original.pid;
    browser.cdp_endpoint = "ws://127.0.0.1:9222/devtools/browser/synthetic".into();
    state.app_browser_attachments.insert(
        browser.id.clone(),
        BrowserAppAttachment {
            request: BrowserAttachmentRequest {
                version: 1,
                process: original,
                profile_path: fixture
                    .root
                    .join("profile-a")
                    .to_string_lossy()
                    .into_owned(),
                cdp_endpoint: browser.cdp_endpoint.clone(),
                target_id: target_id.clone(),
                shared_with: vec![],
            },
            owner: BrowserHostIdentity {
                host_id: "app".into(),
                participant_id: "app-owner".into(),
                session_id: "app-session".into(),
            },
            borrowed_tab_ids: std::collections::BTreeSet::from([target_id]),
            profile_id: "profile-a".into(),
            desktop: browser.desktop.clone().unwrap(),
            assignment: None,
        },
    );
    drop(host);
    let store = fixture.store(false);
    store.save_session_state(&state).unwrap();
    let mut catalog = store.load_profile_catalog().unwrap().catalog;
    // Opposite launch intent must not rewrite observed original process mode.
    catalog
        .profiles
        .get_mut("profile-a")
        .unwrap()
        .manual_login_url = Some("about:blank".into());
    store.save_profile_catalog(&catalog).unwrap();
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let effect_counts = (calls.get(), requests.get());
    let prepared = host.mode_convert(&json!({"sessionId":opened.session_id,"operationId":"observed-mode","mode":"cdp_free","choice":"prepare"})).unwrap();
    assert_eq!(prepared["phase"], "prepared");
    assert_eq!(
        (calls.get(), requests.get()),
        effect_counts,
        "binding is effect-free"
    );
    let connection = rusqlite::Connection::open(fixture.root.join("runtime.mode.sqlite3")).unwrap();
    let binding: String = connection
        .query_row(
            "SELECT binding FROM browser_mode_owner_bindings WHERE id='observed-mode'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let binding: Value = serde_json::from_str(&binding).unwrap();
    assert_eq!(
        binding["original_mode"], "Cdp",
        "recovery must bind verified original posture, not catalog intent"
    );
    assert!(
        host.state()
            .app_browser_attachments
            .contains_key(&opened.browser_id),
        "inspection retains app ownership metadata"
    );
    let paused = host.mode_convert(&json!({"sessionId":opened.session_id,"operationId":"observed-mode","mode":"cdp_free","choice":"pause_and_relaunch","ownerConfirmed":true})).unwrap();
    assert_eq!(
        paused["phase"], "paused",
        "borrowed metadata must not veto the owner's explicit operation"
    );
    let cancelled = host.mode_convert(&json!({"sessionId":opened.session_id,"operationId":"observed-mode","mode":"cdp_free","choice":"cancel","ownerConfirmed":true})).unwrap();
    assert_eq!(cancelled["phase"], "cancelled");
    assert!(!host.state().profile_operation_ids.contains_key("profile-a"));
    assert!(host
        .state()
        .app_browser_attachments
        .contains_key(&opened.browser_id));
    assert_eq!(
        (calls.get(), requests.get()),
        effect_counts,
        "pause/cancel does not take lifecycle custody from the app"
    );
    use sha2::{Digest, Sha256};
    let selected_path = fixture.root.join("chromium");
    std::fs::copy("/usr/bin/python3", &selected_path).unwrap();
    let selected_sha = format!(
        "{:x}",
        Sha256::digest(std::fs::read(&selected_path).unwrap())
    );
    let mut selection = json!({"sessionId":opened.session_id,"operationId":"selected-build","mode":"cdp_free","choice":"prepare","executablePath":selected_path,"expectedSha256":selected_sha});
    let selected = host.mode_convert(&selection).unwrap();
    assert_eq!(selected["selectedBuild"]["sha256"],selected_sha,"Build review must bind the exact selected bytes independently of original mode: {selected}");
    selection["choice"] = json!("relaunch");
    let refused = host.mode_convert(&selection).unwrap_err();
    assert!(
        refused.starts_with("browser_build_owner_confirmation_required"),
        "{refused}"
    );
    assert_eq!(
        (calls.get(), requests.get()),
        effect_counts,
        "A build switch cannot close the original without informed confirmation"
    );
    std::fs::write(&selected_path, b"#!/bin/sh\nexit 0\n").unwrap();
    selection["ownerConfirmed"] = json!(true);
    let unavailable = host.mode_convert(&selection).unwrap();
    assert_eq!(
        unavailable["phase"], "prepared",
        "An unsupported launcher must be refused before closing the original: {unavailable}"
    );
    assert_eq!(unavailable["failure"], "browser_build_launcher_unverified");
    assert_eq!((calls.get(), requests.get()), effect_counts);
    let readback=host.handle_command(&json!({"action":"browser_session_mode_convert","sessionId":opened.session_id,"sessionName":"Alice","choice":"inspect","operationId":"selected-build"}));
    assert_eq!(
        readback["data"]["operations"][0]["selectedBuild"]["sha256"], selected_sha,
        "Reload retains the reviewed build: {readback}"
    );
    std::fs::remove_file(&selected_path).unwrap();
    selection["choice"] = json!("cancel");
    assert_eq!(host.mode_convert(&selection).unwrap()["phase"], "cancelled");
}
