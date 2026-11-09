use super::*;

#[test]
fn native_cli_saved_login_requires_journaled_stdio_before_task_or_owner_access() {
    let root = std::env::temp_dir().join(format!("native-cli610-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let guard = crate::test_utils::EnvGuard::new(&["HOME", "AGENT_BROWSER_HOME", "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME"]);
    guard.set("HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    for (action, params) in [
        ("native_capture", json!({"requestId":"original-capture","loginControls":{"profile":"english-chrome-v1","expectedAccount":"private-account-canary@example.test"}})),
        ("native_input", json!({"requestId":"original-input","loginTarget":{"observationRequestId":"original-capture","targetRef":"private-target-canary","episodeId":"original-episode"}})),
    ] {
        let result = mcp_command_response(&["mcp".into(), "browser-native".into(), "missing-task".into(), action.into(), params.to_string()]);
        assert_eq!(result.unwrap_err(), "browser_native_login_requires_mcp_stdio");
    }
    drop(guard);
    std::fs::remove_dir_all(root).unwrap();
}
