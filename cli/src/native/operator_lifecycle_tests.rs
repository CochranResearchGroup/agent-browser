//! Explicit lifecycle requests at the public owner seam with real owned child processes.
use super::*;
use serde_json::json;

#[cfg(target_os = "linux")]
#[test]
fn human_handoff_allows_attributed_owned_browser_close_and_preserves_profile_and_peers() {
    qualify_operator_close(false);
}

#[test]
fn explicit_process_close_does_not_wait_for_manual_window_close() {
    qualify_operator_close(true);
}

fn qualify_operator_close(window_stays_live: bool) {
    use crate::test_utils::EnvGuard;
    let fixture = Fixture::new();
    let guard = EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
    ]);
    guard.set("HOME", fixture.root.to_str().unwrap());
    guard.set(
        "AGENT_BROWSER_HOME",
        fixture.root.join(".agent-browser").to_str().unwrap(),
    );
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    std::fs::create_dir_all(fixture.root.join(".agent-browser/service")).unwrap();
    let _processes = CopyProcessFixture::new(&fixture.root);
    std::fs::write(fixture.root.join("operator-os-close-readback"), "").unwrap();
    if window_stays_live {
        std::fs::write(fixture.root.join("manual-window-close-stays-live"), "").unwrap();
    }
    let mut host = fixture.host(Mode::Success, Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let original = host
        .open(OpenBrowserSession::exact_profile(
            "Signin",
            "profile-a",
            100,
        ))
        .unwrap();
    let peer = host
        .open(OpenBrowserSession::exact_profile(
            "Independent",
            "profile-b",
            101,
        ))
        .unwrap();
    let original_pid = host.state().browsers[&original.browser_id].pid;
    let peer_pid = host.state().browsers[&peer.browser_id].pid;
    let profile = fixture.root.join("profile-a");
    std::fs::create_dir_all(&profile).unwrap();
    std::fs::write(
        profile.join("persistent-marker"),
        b"authenticated-profile-fixture",
    )
    .unwrap();
    crate::native::browser_host_control::begin_handoff(
        &original.browser_id,
        "manual-signin",
        "exact-target",
    )
    .unwrap();
    let ordinary = host.handle_command(
        &json!({"action":"browser_session_close", "sessionId":original.session_id, "scope":"task"}),
    );
    assert_eq!(ordinary["success"], false);
    assert!(ordinary["error"]
        .as_str()
        .unwrap()
        .starts_with("browser_control_human"));
    let missing = host.handle_command(&json!({"action":"browser_session_close", "sessionId":original.session_id, "scope":"browser"}));
    assert_eq!(missing["success"], false);
    assert_eq!(missing["error"], "browser_lifecycle_attribution_required");
    let closed = host.handle_command(&json!({"id":"operator-close", "action":"browser_session_close", "sessionId":original.session_id, "scope":"browser", "actor":"operator", "reason":"Close this signed-in profile for an explicitly requested relaunch"}));
    assert_eq!(closed["success"], true, "{closed}");
    assert_eq!(closed["data"]["residue"]["originalProcess"], "absent");
    assert!(matches!(
        crate::process_identity::observe_process(original_pid),
        crate::process_identity::ProcessObservation::Missing
    ));
    assert!(matches!(
        crate::process_identity::observe_process(peer_pid),
        crate::process_identity::ProcessObservation::Observed(_)
    ));
    assert!(!host.state().sessions.contains_key(&original.session_id));
    assert!(host.state().sessions.contains_key(&peer.session_id));
    assert_eq!(
        std::fs::read(profile.join("persistent-marker")).unwrap(),
        b"authenticated-profile-fixture"
    );
    // Closing a process is not a claim that page state was observed or an old
    // interactive invitation was returned. Its audit/uncertainty stays intact.
    let old = crate::native::browser_host_control::metadata(&original.browser_id).unwrap();
    assert_eq!(old["epoch"], 1);
    assert_eq!(old["owner"], "human");
    let reopened = host
        .open(OpenBrowserSession::exact_profile(
            "AfterSignin",
            "profile-a",
            102,
        ))
        .unwrap();
    assert_eq!(
        host.state().sessions[&reopened.session_id].profile_id,
        "profile-a"
    );
    assert_eq!(
        std::fs::read(profile.join("persistent-marker")).unwrap(),
        b"authenticated-profile-fixture"
    );
}
