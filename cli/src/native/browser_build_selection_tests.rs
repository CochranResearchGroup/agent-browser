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
    use sha2::{Digest,Sha256};
    let fixture=Fixture::new();
    let store=fixture.store(false);
    let mut state=store.load_session_state().unwrap();
    state.profile_executables.insert("profile-a".into(),"/owner/original-build".into());
    store.save_session_state(&state).unwrap();
    let calls=Rc::new(Cell::new(0));let requests=Rc::new(Cell::new(0));
    let mut host=fixture.host(Mode::Success,calls.clone(),requests.clone());
    let sha=format!("{:x}",Sha256::digest(std::fs::read("/usr/bin/python3").unwrap()));
    let command=json!({"action":"browser_session_profile_duplicate","sourceProfileId":"profile-a","operationId":"test-selected-copy","choice":"prepare","executablePath":"/usr/bin/python3","expectedSha256":sha,"mode":"cdp_free"});
    let reviewed=host.handle_command(&command);
    assert_eq!(reviewed["success"],true,"{reviewed}");
    assert_eq!(reviewed["data"]["selectedBuild"]["sha256"],sha,"Duplicate-and-test must retain the selected destination build: {reviewed}");
    assert_eq!(reviewed["data"]["requestedMode"],"cdp_free");
    assert_eq!(store.load_session_state().unwrap().profile_executables["profile-a"],"/owner/original-build");
    assert_eq!((calls.get(),requests.get()),(0,0),"Review does not open the source or launch the selected destination build");
    let mut changed=command.clone();changed["executablePath"]=json!("/usr/bin/sleep");
    changed["expectedSha256"]=json!(format!("{:x}",Sha256::digest(std::fs::read("/usr/bin/sleep").unwrap())));
    assert_eq!(host.handle_command(&changed)["success"],false,"Changed build intent cannot silently retarget an existing copy operation");
}
