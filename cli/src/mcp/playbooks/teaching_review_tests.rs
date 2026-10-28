use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[test]
fn teaching_review_public_validation_and_reopen_preserve_inert_versions() {
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    use agent_browser_service_model::{
        BrowserSessionState, ManagedBrowserSession, ManagedBrowserTab,
    };
    let root =
        std::env::temp_dir().join(format!("teaching-review-public-{}", uuid::Uuid::new_v4()));
    let candidates = root.join("candidates");
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
    guard.set(
        "AGENT_BROWSER_SOCKET_DIR",
        root.join("sockets").to_str().unwrap(),
    );
    guard.set(
        "AGENT_BROWSER_SITEGUIDE_DIRECTORY",
        candidates.to_str().unwrap(),
    );
    std::fs::create_dir_all(&candidates).unwrap();
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    let mut state = BrowserSessionState::default();
    state.sessions.insert(
        "task".into(),
        ManagedBrowserSession {
            id: "task".into(),
            name: "teaching-review".into(),
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
    store.save_session_state(&state).unwrap();
    let original = include_str!("../../../../scripts/fixtures/teaching-report/draft.json");
    let parent_sha = format!("{:x}", Sha256::digest(original.as_bytes()));
    let mut guide: Value = serde_json::from_str(original).unwrap();
    guide["guideId"] = json!("daily-sales-draft-v1-r2");
    guide["teaching"]["routines"][1]["steps"][0]["expectedResult"] =
        json!("Caller-selected start date is visible.");
    guide["teaching"]["lineage"] = json!({"rootGuideId":"daily-sales-draft-v1","revision":2,"parentGuideId":"daily-sales-draft-v1","parentSha256":parent_sha,"operationId":"edit-1","operationKind":"edit","intentSha256":"a".repeat(64)});
    let call = |params: Value| {
        super::super::handle_jsonrpc_line(&json!({"jsonrpc":"2.0","id":475,"method":"tools/call","params":{"name":"browser_command","arguments":{"taskId":"task","action":"playbook","params":params}}}).to_string(),"unused-transport").unwrap()
    };
    let response = call(json!({"operation":"validate_teaching_candidate","guide":guide}));
    assert!(response["error"].is_null(), "{response}");
    let payload: Value =
        serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload["data"]["valid"], true);
    assert_eq!(payload["data"]["executionAllowed"], false);
    assert_eq!(
        payload["data"]["operatorReviewQualified"], false,
        "static data validation cannot grant operator approval"
    );
    std::fs::write(candidates.join("daily-sales-draft-v1.json"), original).unwrap();
    let bytes = guide.to_string();
    std::fs::write(candidates.join("daily-sales-draft-v1-r2.json"), &bytes).unwrap();
    let reopened = call(json!({"operation":"inspect_guide","guideId":"daily-sales-draft-v1-r2"}));
    assert!(reopened["error"].is_null(), "{reopened}");
    let result: Value =
        serde_json::from_str(reopened["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(result["data"]["teaching"]["lineage"]["revision"], 2);
    assert_eq!(
        result["data"]["teaching"]["routines"][1]["steps"][0]["expectedResult"],
        "Caller-selected start date is visible."
    );
    assert_eq!(
        std::fs::read_to_string(candidates.join("daily-sales-draft-v1.json")).unwrap(),
        original
    );
    assert_eq!(
        std::fs::read_to_string(candidates.join("daily-sales-draft-v1-r2.json")).unwrap(),
        bytes
    );
    let mut reviewed = guide.clone();
    let reviewed_parent_sha = format!("{:x}", Sha256::digest(bytes.as_bytes()));
    reviewed["guideId"] = json!("daily-sales-draft-v1-r3");
    reviewed["teaching"]["lineage"] = json!({"rootGuideId":"daily-sales-draft-v1","revision":3,"parentGuideId":"daily-sales-draft-v1-r2","parentSha256":reviewed_parent_sha,"operationId":"review-1","operationKind":"review","intentSha256":"b".repeat(64)});
    reviewed["teaching"]["review"] = json!({"decisionId":"operator-decision-1","decision":"approved","actorRef":"operator-1","sourceRef":"owner-receipt-1","provenance":"authenticated_operator","taskId":"task","rootGuideId":"daily-sales-draft-v1","reviewedGuideId":"daily-sales-draft-v1-r2","reviewedSha256":reviewed_parent_sha,"contentSha256":"c".repeat(64),"sourceUnavailableAcknowledged":false});
    let shape = call(json!({"operation":"validate_teaching_candidate","guide":reviewed}));
    assert!(shape["error"].is_null(), "{shape}");
    let shape_data: Value =
        serde_json::from_str(shape["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        shape_data["data"]["operatorReviewQualified"], false,
        "role labels/static metadata never establish actual approval"
    );
    std::fs::write(
        candidates.join("daily-sales-draft-v1-r3.json"),
        reviewed.to_string(),
    )
    .unwrap();
    let denied = call(json!({"operation":"verify_login","guideId":"daily-sales-draft-v1-r3"}));
    assert_eq!(
        denied["error"]["data"]["message"],
        "teaching_draft_not_executable"
    );
    guide["teaching"]["lineage"]["parentSha256"] = json!("bad");
    let invalid = call(json!({"operation":"validate_teaching_candidate","guide":guide}));
    assert_eq!(
        invalid["error"]["data"]["message"],
        "teaching_draft_contract_invalid"
    );
    state.sessions.get_mut("task").unwrap().current_tab_id = None;
    state.tabs.remove("tab");
    store.save_session_state(&state).unwrap();
    let targetless = call(json!({"operation":"inspect_guide","guideId":"daily-sales-draft-v1-r2"}));
    assert!(
        targetless["error"].is_null(),
        "retained procedure review needs no active browser target: {targetless}"
    );
    let detached_validation = call(
        json!({"operation":"validate_teaching_candidate","guide":serde_json::from_str::<Value>(&bytes).unwrap()}),
    );
    assert!(
        detached_validation["error"].is_null(),
        "{detached_validation}"
    );
    let still_fenced =
        call(json!({"operation":"verify_login","guideId":"daily-sales-draft-v1-r3"}));
    assert_eq!(
        still_fenced["error"]["data"]["message"],
        "browser_task_current_tab_missing"
    );
    assert_eq!(
        serde_json::to_value(store.load_session_state().unwrap()).unwrap(),
        serde_json::to_value(state).unwrap()
    );
    assert!(!root.join("sockets").exists());
    drop(store);
    fn remove(path: &std::path::Path) {
        let metadata = std::fs::symlink_metadata(path).unwrap();
        if metadata.is_dir() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
            }
            for entry in std::fs::read_dir(path).unwrap() {
                remove(&entry.unwrap().path());
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
    remove(&root);
    drop(guard);
}
