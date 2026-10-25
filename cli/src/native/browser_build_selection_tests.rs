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
