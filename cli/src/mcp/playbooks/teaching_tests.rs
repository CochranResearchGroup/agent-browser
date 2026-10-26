use serde_json::{json, Value};

#[test]
fn teaching_draft_public_inspection_preserves_useful_evidence_without_execution() {
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    use agent_browser_service_model::{
        BrowserSessionState, ManagedBrowserSession, ManagedBrowserTab,
    };
    let root = std::env::temp_dir().join(format!("teaching-draft-public-{}", uuid::Uuid::new_v4()));
    let candidates = root.join("candidate-guides");
    let sockets = root.join("sockets");
    let guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_SOCKET_DIR",
        "AGENT_BROWSER_SITEGUIDE_DIRECTORY",
    ]);
    guard.set("HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    guard.set("AGENT_BROWSER_HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_SOCKET_DIR", sockets.to_str().unwrap());
    guard.set(
        "AGENT_BROWSER_SITEGUIDE_DIRECTORY",
        candidates.to_str().unwrap(),
    );
    std::fs::create_dir_all(&candidates).unwrap();
    let document = include_str!("../../../../scripts/fixtures/teaching-report/draft.json");
    std::fs::write(candidates.join("daily-sales-draft-v1.json"), document).unwrap();
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    let mut state = BrowserSessionState::default();
    state.sessions.insert(
        "task".into(),
        ManagedBrowserSession {
            id: "task".into(),
            name: "teaching".into(),
            profile_id: "profile".into(),
            browser_id: "browser".into(),
            created_at_ms: 1,
            last_activity_at_ms: 1,
            expires_at_ms: 0,
            current_tab_id: Some("tab".into()),
        },
    );
    state.tabs.insert(
        "tab".into(),
        ManagedBrowserTab {
            id: "tab".into(),
            target_id: "target".into(),
            browser_id: "browser".into(),
            session_id: "task".into(),
            created_at_ms: 1,
            last_activity_at_ms: 1,
        },
    );
    let mut peer = state.sessions["task"].clone();
    peer.id = "peer".into();
    state.sessions.insert("peer".into(), peer);
    store.save_session_state(&state).unwrap();
    let request = |operation: &str| json!({"jsonrpc":"2.0","id":474,"method":"tools/call","params":{"name":"browser_command","arguments":{"taskId":"task","action":"playbook","params":{"operation":operation,"guideId":"daily-sales-draft-v1"}}}});
    let response = super::super::handle_jsonrpc_line(
        &request("inspect_guide").to_string(),
        "unused-transport",
    )
    .unwrap();
    assert!(response["error"].is_null(), "{response}");
    let payload: Value =
        serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload["success"], true, "{payload}");
    let draft = &payload["data"]["teaching"];
    assert_eq!(draft["state"], "draft");
    assert_eq!(draft["parameters"][0]["example"], "2026-05-01");
    assert_eq!(draft["parameters"][1]["example"], "2026-05-31");
    assert_eq!(draft["parameters"][2]["example"], true);
    assert_eq!(draft["routines"].as_array().unwrap().len(), 4);
    assert_eq!(draft["playbook"]["routineIds"][3], "export-and-verify");
    assert_eq!(draft["claims"][2]["attribution"], "observed");
    assert_eq!(draft["claims"][3]["attribution"], "inferred");
    assert_eq!(draft["claims"][4]["attribution"], "unknown");
    assert_eq!(draft["unresolved"][0]["id"], "refunds-policy");
    assert_eq!(
        serde_json::to_value(store.load_session_state().unwrap()).unwrap(),
        serde_json::to_value(&state).unwrap()
    );
    assert!(
        !sockets.exists(),
        "inspection creates no daemon or browser socket"
    );
    assert_eq!(
        std::fs::read_to_string(candidates.join("daily-sales-draft-v1.json")).unwrap(),
        document
    );
    let attempted =
        super::super::handle_jsonrpc_line(&request("verify_login").to_string(), "unused-transport")
            .unwrap();
    assert_eq!(
        attempted["error"]["data"]["message"], "teaching_draft_not_executable",
        "{attempted}"
    );
    assert!(
        !sockets.exists(),
        "draft execution refusal creates no transport"
    );
    let mut ungrounded: Value = serde_json::from_str(document).unwrap();
    ungrounded["teaching"]["claims"][0]["evidenceRefs"] = json!(["invented-frame"]);
    std::fs::write(
        candidates.join("daily-sales-draft-v1.json"),
        ungrounded.to_string(),
    )
    .unwrap();
    let rejected = super::super::handle_jsonrpc_line(
        &request("inspect_guide").to_string(),
        "unused-transport",
    )
    .unwrap();
    assert_eq!(
        rejected["error"]["data"]["message"],
        "teaching_draft_contract_invalid"
    );
    std::fs::write(candidates.join("daily-sales-draft-v1.json"), document).unwrap();
    std::fs::write(
        candidates.join("bill-session-v1.json"),
        include_str!("../../../../config/siteguides/bill-session-v1.json"),
    )
    .unwrap();
    let mut legacy = request("inspect_guide");
    legacy["params"]["arguments"]["params"]["guideId"] = json!("bill-session-v1");
    let response =
        super::super::handle_jsonrpc_line(&legacy.to_string(), "unused-transport").unwrap();
    assert!(response["error"].is_null(), "{response}");
    let result: Value =
        serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        result["data"]["controls"]["password"],
        "input[type=password]"
    );
    assert!(result["data"]["teaching"].is_null());
    assert_eq!(
        serde_json::to_value(store.load_session_state().unwrap()).unwrap(),
        serde_json::to_value(&state).unwrap()
    );
    assert!(!sockets.exists());
    drop(store);
    fn remove_fixture(path: &std::path::Path) {
        let metadata = std::fs::symlink_metadata(path).unwrap();
        if metadata.is_dir() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
            }
            for entry in std::fs::read_dir(path).unwrap() {
                remove_fixture(&entry.unwrap().path());
            }
            std::fs::remove_dir(path).unwrap();
        } else {
            #[cfg(windows)]
            {
                let mut permissions = metadata.permissions();
                permissions.set_readonly(false);
                std::fs::set_permissions(path, permissions).unwrap();
            }
            std::fs::remove_file(path).unwrap();
        }
    }
    remove_fixture(&root);
    drop(guard);
}
