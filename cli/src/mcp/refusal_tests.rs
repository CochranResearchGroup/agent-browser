use super::*;

#[test]
fn public_refusal_keeps_recovery_and_occurrence_id() {
    let root = std::env::temp_dir().join(format!("public-refusal-{}", uuid::Uuid::new_v4()));
    let guard = crate::test_utils::EnvGuard::new(&["HOME", "AGENT_BROWSER_HOME"]);
    guard.set("HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_HOME", root.to_str().unwrap());
    let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"browser_fill","arguments":{"taskId":"session:absent",
            "requestId":"original","selector":"#field","value":"PRIVATE-MUST-NOT-LOG"}}});
    let response = handle_jsonrpc_line(&request.to_string(), "unused").unwrap();
    let data = &response["error"]["data"];
    assert!(
        data["failure"]["executableNextAction"].is_object(),
        "{response}"
    );
    assert!(data["failure"]["occurrenceId"].is_string(), "{response}");
    assert!(data["failure"]["effectState"].is_string());
    let occurrence = data["failure"]["occurrenceId"].as_str().unwrap();
    let lookup = json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"browser_command","arguments":{"taskId":"session:absent",
            "action":"failure_log","params":{"occurrenceId":occurrence}}}});
    let readback = handle_jsonrpc_line(&lookup.to_string(), "unused").unwrap();
    let payload: Value = serde_json::from_str(readback["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload["data"]["records"][0]["occurrenceId"], occurrence, "{readback}");
    assert!(!readback.to_string().contains("PRIVATE-MUST-NOT-LOG"));
    drop(guard);
    let _ = std::fs::remove_dir_all(root);
}
