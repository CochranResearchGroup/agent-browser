#![cfg(target_os = "linux")]
use super::*;
use serde_json::json;

#[test]
fn browser_build_inspection_reports_evidence_without_profile_or_browser_effects() {
    let fixture = Fixture::new();
    let profile = fixture.root.join("profile-a");
    std::fs::create_dir(&profile).unwrap();
    std::fs::write(profile.join("Last Version"), "153.0.8010.52\n").unwrap();
    std::fs::write(
        profile.join("Local State"),
        r#"{"os_crypt":{"encrypted_key":"PRIVATE-CANARY"}}"#,
    )
    .unwrap();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let before = host.state().clone();
    let response = host.handle_command(&json!({
        "action":"browser_session_build_select", "profileId":"profile-a", "choice":"inspect",
        "executablePath":"/usr/bin/python3"
    }));
    assert_eq!(
        response["success"], true,
        "Build discovery must be supported: {response}"
    );
    assert_eq!(
        response["data"]["schemaVersion"],
        "agent-browser.build-selection.v1"
    );
    assert_eq!(response["data"]["profileDataVersion"], "153.0.8010.52");
    assert_eq!(response["data"]["credentialStorage"], "unverified");
    assert_eq!(response["data"]["profileDataReadable"], "unverified");
    assert!(!response["data"]["availableBuilds"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        host.state(),
        &before,
        "Discovery never changes profile settings"
    );
    assert_eq!(
        (calls.get(), requests.get()),
        (0, 0),
        "Discovery does not launch, focus, close or navigate"
    );
    assert!(!response.to_string().contains("PRIVATE-CANARY"));
    assert!(!response
        .to_string()
        .contains(&fixture.root.to_string_lossy().to_string()));
}

#[test]
fn browser_build_offline_selection_requires_confirmation_and_replays_without_overwrite() {
    let fixture = Fixture::new();
    std::fs::create_dir(fixture.root.join("profile-a")).unwrap();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    use sha2::{Digest, Sha256};
    let digest = format!(
        "{:x}",
        Sha256::digest(std::fs::read("/usr/bin/python3").unwrap())
    );
    let mut command = json!({"action":"browser_session_build_select","profileId":"profile-a",
        "choice":"select_for_next_launch","operationId":"selection-a","executablePath":"/usr/bin/python3",
        "mode":"cdp_free","expectedSha256":digest});
    let refused = host.handle_command(&command);
    assert_eq!(
        refused["success"], false,
        "Consequential selection requires informed confirmation"
    );
    command["ownerConfirmed"] = json!(true);
    let selected = host.handle_command(&command);
    assert_eq!(selected["success"], true, "{selected}");
    assert_eq!(selected["data"]["phase"], "selected_for_next_launch");
    assert!(selected["data"]["observedMode"].is_null());
    let peer = fixture.store(false);
    let mut newer = peer.load_session_state().unwrap();
    newer
        .profile_executables
        .insert("profile-a".into(), "/owner/later-choice".into());
    peer.save_session_state(&newer).unwrap();
    let mut restarted = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let replay = restarted.handle_command(&command);
    assert_eq!(replay["success"], true, "{replay}");
    assert_eq!(
        peer.load_session_state().unwrap().profile_executables["profile-a"],
        "/owner/later-choice"
    );
    assert_eq!(
        (calls.get(), requests.get()),
        (0, 0),
        "Offline selection cannot launch or operate a browser"
    );
    assert!(!selected
        .to_string()
        .contains(&fixture.root.to_string_lossy().to_string()));
}

#[test]
fn duplicate_build_review_retains_source_pin_and_rejects_changed_replay() {
    use sha2::{Digest, Sha256};
    let fixture = Fixture::new();
    let store = fixture.store(false);
    let mut state = store.load_session_state().unwrap();
    state
        .profile_executables
        .insert("profile-a".into(), "/owner/original-build".into());
    store.save_session_state(&state).unwrap();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let sha = format!(
        "{:x}",
        Sha256::digest(std::fs::read("/usr/bin/python3").unwrap())
    );
    let command = json!({"action":"browser_session_profile_duplicate","sourceProfileId":"profile-a","operationId":"test-selected-copy","choice":"prepare","executablePath":"/usr/bin/python3","expectedSha256":sha,"mode":"cdp_free"});
    let reviewed = host.handle_command(&command);
    assert_eq!(reviewed["success"], true, "{reviewed}");
    assert_eq!(
        reviewed["data"]["selectedBuild"]["sha256"], sha,
        "Duplicate-and-test must retain the selected destination build: {reviewed}"
    );
    assert_eq!(reviewed["data"]["requestedMode"], "cdp_free");
    assert_eq!(
        store.load_session_state().unwrap().profile_executables["profile-a"],
        "/owner/original-build"
    );
    assert_eq!(
        (calls.get(), requests.get()),
        (0, 0),
        "Review does not open the source or launch the selected destination build"
    );
    let mut changed = command.clone();
    changed["executablePath"] = json!("/usr/bin/sleep");
    changed["expectedSha256"] = json!(format!(
        "{:x}",
        Sha256::digest(std::fs::read("/usr/bin/sleep").unwrap())
    ));
    assert_eq!(
        host.handle_command(&changed)["success"],
        false,
        "Changed build intent cannot silently retarget an existing copy operation"
    );
}

#[test]
fn future_build_selection_mode_survives_unspecified_open_and_explicit_mode_still_wins() {
    use sha2::{Digest, Sha256};
    for explicit in [None, Some(false)] {
        let fixture = Fixture::new();
        std::fs::write(fixture.root.join("automatic-pool"), "enabled").unwrap();
        std::fs::create_dir(fixture.root.join("profile-a")).unwrap();
        let mut host = fixture.host(Mode::Success, Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let sha = format!(
            "{:x}",
            Sha256::digest(std::fs::read("/usr/bin/python3").unwrap())
        );
        let selected=host.handle_command(&json!({"action":"browser_session_build_select","profileId":"profile-a","choice":"select_for_next_launch","operationId":"future-mode","executablePath":"/usr/bin/python3","expectedSha256":sha,"mode":"cdp_free","ownerConfirmed":true}));
        assert_eq!(selected["success"], true, "{selected}");
        let mut command = json!({"action":"browser_session_open","profileId":"profile-a","sessionName":"Future preference","url":"about:blank"});
        if let Some(value) = explicit {
            command["manualLogin"] = json!(value);
        }
        let opened = host.handle_command(&command);
        assert_eq!(opened["success"], true, "{opened}");
        assert_eq!(
            fixture
                .store(false)
                .load_profile_catalog()
                .unwrap()
                .catalog
                .profiles["profile-a"]
                .manual_login_url
                .is_some(),
            explicit.is_none(),
            "An omitted mode must preserve No-CDP intent; an explicit mode may change it"
        );
    }
}

#[test]
fn build_selection_existing_copy_requires_published_lineage_and_exact_directory() {
    let fixture = Fixture::new();
    std::fs::create_dir(fixture.root.join("profile-a")).unwrap();
    let store = fixture.store(false);
    assert!(
        store.published_copy_profile("profile-a").unwrap().is_none(),
        "An ordinary catalog row is not copy publication proof"
    );
    let connection = rusqlite::Connection::open(fixture.root.join("runtime.sqlite3")).unwrap();
    connection.execute_batch("CREATE TABLE browser_profile_duplication_lineage(profile_id TEXT PRIMARY KEY,source_profile_id TEXT NOT NULL,operation_id TEXT NOT NULL,browser_build TEXT NOT NULL);INSERT INTO browser_profile_duplication_lineage VALUES('profile-a','source-original','copy-operation','source-build');").unwrap();
    let copied = store.published_copy_profile("profile-a").unwrap();
    assert!(
        copied.is_some(),
        "A published named copy must be available to the existing-profile open adapter"
    );
    std::fs::remove_dir(fixture.root.join("profile-a")).unwrap();
    assert!(
        store.published_copy_profile("profile-a").is_err(),
        "A missing copied directory cannot become a newly created empty profile"
    );
}

#[test]
fn build_selection_receipt_cannot_open_changed_build_or_different_profile() {
    use sha2::{Digest, Sha256};
    for changed in ["pin", "bytes", "profile"] {
        let fixture = Fixture::new();
        for profile in ["profile-a", "profile-b"] {
            std::fs::create_dir(fixture.root.join(profile)).unwrap();
        }
        let executable = fixture.root.join("reviewed-browser");
        std::fs::copy("/usr/bin/python3", &executable).unwrap();
        let sha = format!("{:x}", Sha256::digest(std::fs::read(&executable).unwrap()));
        let calls = Rc::new(Cell::new(0));
        let requests = Rc::new(Cell::new(0));
        let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
        let selected=host.handle_command(&json!({"action":"browser_session_build_select","profileId":"profile-a","choice":"select_for_next_launch","operationId":"bound-open","executablePath":executable,"expectedSha256":sha,"mode":"cdp","ownerConfirmed":true}));
        assert_eq!(selected["success"], true, "{selected}");
        if changed == "pin" {
            let store = fixture.store(false);
            let mut state = store.load_session_state().unwrap();
            state
                .profile_executables
                .insert("profile-a".into(), "/owner/later-choice".into());
            store.save_session_state(&state).unwrap();
        }
        if changed == "bytes" {
            std::fs::write(&executable, b"changed after confirmation").unwrap();
        }
        let opened=host.handle_command(&json!({"action":"browser_session_open","profileId":if changed=="profile" {"profile-b"} else {"profile-a"},"sessionName":"Bound build open","url":"about:blank","buildSelectionOperationId":"bound-open"}));
        assert_eq!(
            opened["success"], false,
            "A stale build selection must be refused before launch ({changed}): {opened}"
        );
        assert!(
            opened["error"]
                .as_str()
                .unwrap()
                .starts_with("browser_build_"),
            "{opened}"
        );
        assert_eq!(
            (calls.get(), requests.get()),
            (0, 0),
            "A stale selection cannot launch or operate any browser"
        );
    }
}

#[test]
fn build_selection_opaque_discovery_choice_executes_without_exposing_profile_paths() {
    let fixture = Fixture::new();
    std::fs::create_dir(fixture.root.join("profile-a")).unwrap();
    let store = fixture.store(false);
    let mut state = store.load_session_state().unwrap();
    state
        .profile_executables
        .insert("profile-a".into(), "/usr/bin/python3".into());
    store.save_session_state(&state).unwrap();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let inspected=host.handle_command(&json!({"action":"browser_session_build_select","profileId":"profile-a","choice":"inspect","executablePath":"/usr/bin/python3"}));
    assert_eq!(inspected["success"], true, "{inspected}");
    let mut selected = inspected["data"]["availableBuilds"][0]["selection"].clone();
    selected["action"] = json!("browser_session_build_select");
    selected["profileId"] = json!("profile-a");
    selected["operationId"] = json!("opaque-choice");
    selected["choice"] = json!("select_for_next_launch");
    selected["mode"] = json!("cdp");
    selected["ownerConfirmed"] = json!(true);
    let selected = host.handle_command(&selected);
    assert_eq!(
        selected["success"], true,
        "A discovered opaque choice must be executable: {selected}"
    );
    assert_eq!((calls.get(), requests.get()), (0, 0));
    assert!(!inspected
        .to_string()
        .contains(&fixture.root.to_string_lossy().to_string()));
}
