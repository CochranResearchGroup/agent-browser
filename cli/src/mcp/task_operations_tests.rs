use super::*;

#[test]
fn public_original_task_operation_readback_does_not_start_transport() {
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    use agent_browser_service_model::BrowserSessionState;
    let directory =
        std::env::temp_dir().join(format!("task-operation-public-{}", uuid::Uuid::new_v4()));
    let guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_SOCKET_DIR",
    ]);
    guard.set("HOME", directory.to_str().unwrap());
    guard.set("AGENT_BROWSER_HOME", directory.to_str().unwrap());
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    guard.set(
        "AGENT_BROWSER_SOCKET_DIR",
        directory.join("never-started").to_str().unwrap(),
    );
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    let mut value = serde_json::to_value(BrowserSessionState::default()).unwrap();
    value["taskOperations"] = json!({"session:alice":{"lost-fill":{
        "requestId":"lost-fill","taskId":"session:alice","profileId":"profile","browserId":"browser",
        "tabId":"tab","targetId":"target","processPid":42,"controlEpoch":3,"action":"fill",
        "fingerprint":"private-command-mac","state":"pending","createdAtMs":10,
        "completedAtMs":null,"evidenceReferences":{}
    }}});
    value["sessions"] = json!({"session:alice":{"id":"session:alice","name":"alice",
        "profileId":"profile","browserId":"browser","createdAtMs":1,"lastActivityAtMs":1,
        "expiresAtMs":999999,"currentTabId":null}});
    let state: BrowserSessionState = serde_json::from_value(value).unwrap();
    store.save_session_state(&state).unwrap();
    let request = json!({"jsonrpc":"2.0","id":468,"method":"tools/call",
        "params":{"name":"browser_command","arguments":{"taskId":"session:alice",
            "action":"task_operation","params":{"requestId":"lost-fill"}}}});
    let response = handle_jsonrpc_line(&request.to_string(), "unused").unwrap();
    assert!(response.get("error").is_none(), "{response}");
    let payload: Value =
        serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload["data"]["operation"]["requestId"], "lost-fill");
    assert_eq!(payload["data"]["operation"]["state"], "pending");
    assert_eq!(payload["data"]["operation"]["retryAllowed"], false);
    assert!(!response.to_string().contains("private-command-mac"));
    assert!(!directory.join("never-started").exists());
    assert_eq!(store.load_session_state().unwrap(), state);
    let key_path = BrowserSessionSqliteStore::default_sqlite_path()
        .unwrap()
        .parent()
        .unwrap()
        .join("task-operation-mac.key");
    assert!(!key_path.exists());
    let same_original = json!({"jsonrpc":"2.0","id":470,"method":"tools/call",
        "params":{"name":"browser_fill","arguments":{"taskId":"session:alice",
        "requestId":"lost-fill","selector":"#draft","value":"never replay"}}});
    let missing_key = handle_jsonrpc_line(&same_original.to_string(), "unused").unwrap();
    assert!(missing_key
        .to_string()
        .contains("browser_task_operation_key_unavailable"));
    assert!(
        !key_path.exists(),
        "Original history must never cause key regeneration"
    );
    assert_eq!(store.load_session_state().unwrap(), state);
    let invalid = json!({"jsonrpc":"2.0","id":469,"method":"tools/call",
        "params":{"name":"browser_fill","arguments":{"taskId":"session:alice",
        "requestId":"invalid identifier", "selector":"#draft", "value":"must not be persisted"}}});
    let invalid_response = handle_jsonrpc_line(&invalid.to_string(), "unused").unwrap();
    assert!(invalid_response.get("error").is_some());
    assert_eq!(store.load_session_state().unwrap(), state);
    assert!(!directory.join("never-started").exists());
    let unscoped = json!({"jsonrpc":"2.0","id":471,"method":"tools/call",
        "params":{"name":"browser_fill","arguments":{"requestId":"existing legacy identifier",
        "value":"not sent because selector is missing"}}});
    let legacy_validation = handle_jsonrpc_line(&unscoped.to_string(), "unused").unwrap();
    assert!(
        legacy_validation.to_string().contains("taskId"),
        "{legacy_validation}"
    );
    assert!(!legacy_validation.to_string().contains("requestId must"));
    let recovered = handle_jsonrpc_line(&request.to_string(), "unused").unwrap();
    assert!(
        recovered.get("error").is_none(),
        "Failed ingress must not poison subsequent readback"
    );
    drop(store);
    fn unseal_owned_directories(path: &std::path::Path) {
        if !std::fs::symlink_metadata(path).unwrap().is_dir() {
            return;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                unseal_owned_directories(&entry.path());
            }
        }
    }
    unseal_owned_directories(&directory);
    std::fs::remove_dir_all(directory).unwrap();
}
