use std::process::Command;

#[test]
fn invalid_requested_scope_rejects_before_doctor_probes() {
    let output = Command::new(env!("CARGO_BIN_EXE_agent-browser"))
        .args([
            "--json",
            "doctor",
            "remote-view",
            "--route-id",
            "../unsafe?route",
        ])
        .env(
            "HOME",
            std::env::temp_dir().join("agent-browser-doctor-scope-no-home"),
        )
        .env(
            "AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT",
            "/path-that-must-not-be-probed",
        )
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["success"], false);
    assert_eq!(response["type"], "usage_error");
    assert_eq!(response["code"], "invalid_remote_view_doctor_scope");
    assert!(response["error"]
        .as_str()
        .is_some_and(|value| value.contains("unsupported characters")));
}
