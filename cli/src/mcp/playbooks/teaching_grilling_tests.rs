use serde_json::{json, Value};

#[test]
fn teaching_grilling_public_validation_preserves_questions_and_answer_provenance() {
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    use agent_browser_service_model::{
        BrowserSessionState, ManagedBrowserSession, ManagedBrowserTab,
    };
    let root =
        std::env::temp_dir().join(format!("teaching-grilling-public-{}", uuid::Uuid::new_v4()));
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
    let mut guide: Value = serde_json::from_str(include_str!(
        "../../../../scripts/fixtures/teaching-report/draft.json"
    ))
    .unwrap();
    guide["teaching"]["clarification"] = json!({"questions":[{"id":"refunds-policy","question":"Should future runs include refunds?","interpretation":"Keep the choice explicit until policy is settled.","category":"missing_intent","evidenceRefs":["dates","export"],"stepRefs":["set-report-period:2"],"dependsOn":[]}],"answers":[]});
    let call_for = |guide: &Value, task_id: &str| {
        super::super::handle_jsonrpc_line(&json!({"jsonrpc":"2.0","id":476,"method":"tools/call","params":{"name":"browser_command","arguments":{"taskId":task_id,"action":"playbook","params":{"operation":"validate_teaching_candidate","guide":guide}}}}).to_string(),"unused-transport").unwrap()
    };
    let call = |guide: &Value| call_for(guide, "task");
    let response = call(&guide);
    assert!(response["error"].is_null(), "{response}");
    for (field, value) in [
        ("evidenceRefs", json!(["invented"])),
        ("stepRefs", json!(["set-report-period:900"])),
        ("dependsOn", json!(["refunds-policy"])),
    ] {
        let mut invalid = guide.clone();
        invalid["teaching"]["clarification"]["questions"][0][field] = value;
        assert_eq!(
            call(&invalid)["error"]["data"]["message"],
            "teaching_draft_contract_invalid"
        );
    }
    guide["teaching"]["clarification"]["answers"] = json!([{"questionId":"refunds-policy","questionSha256":"a".repeat(64),"answerRef":"operator-answer-1","answer":"Caller chooses per report.","actorRef":"operator-1","sourceRef":"operator-event-1","provenance":"authenticated_operator","taskId":"task","rootGuideId":"daily-sales-draft-v1","answeredGuideId":"daily-sales-draft-v1","answeredSha256":"b".repeat(64),"contentSha256":"c".repeat(64)}]);
    let answered = call(&guide);
    assert!(answered["error"].is_null(), "{answered}");
    std::fs::write(
        candidates.join("daily-sales-draft-v1.json"),
        guide.to_string(),
    )
    .unwrap();
    let reopened = super::super::handle_jsonrpc_line(&json!({"jsonrpc":"2.0","id":476,"method":"tools/call","params":{"name":"browser_command","arguments":{"taskId":"task","action":"playbook","params":{"operation":"inspect_guide","guideId":"daily-sales-draft-v1"}}}}).to_string(),"unused-transport").unwrap();
    let data: Value =
        serde_json::from_str(reopened["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        data["data"]["teaching"]["clarification"],
        guide["teaching"]["clarification"]
    );
    let mut forged = guide.clone();
    forged["teaching"]["clarification"]["answers"][0]["provenance"] = json!("model_recommendation");
    assert_eq!(
        call(&forged)["error"]["data"]["message"],
        "teaching_draft_contract_invalid"
    );
    // Provider contract for 477: later metadata-only versions retain original
    // review, but validation remains an inert claim rather than execution proof.
    let mut repeated = guide.clone();
    repeated["guideId"] = json!("daily-sales-draft-v1-r4");
    repeated["teaching"]["lineage"] = json!({"rootGuideId":"daily-sales-draft-v1","revision":4,"parentGuideId":"daily-sales-draft-v1-r3","parentSha256":"d".repeat(64),"operationId":"repeat-1","operationKind":"repeat","intentSha256":"e".repeat(64)});
    repeated["teaching"]["review"] = json!({"decisionId":"review-1","decision":"approved","actorRef":"operator-1","sourceRef":"review-event-1","provenance":"authenticated_operator","taskId":"task","rootGuideId":"daily-sales-draft-v1","reviewedGuideId":"daily-sales-draft-v1-r2","reviewedSha256":"f".repeat(64),"contentSha256":"c".repeat(64),"sourceUnavailableAcknowledged":false});
    repeated["teaching"]["validation"] = json!({"schemaVersion":"agent-browser.teaching-fresh-repeat.v1","validationId":"validation-1","provenance":"receiving_application_verifier","taskId":"fresh-task","sourceTaskId":"task","rootGuideId":"daily-sales-draft-v1","executedGuideId":"daily-sales-draft-v1-r3","executedSha256":"d".repeat(64),"contentSha256":"c".repeat(64),"reviewDecisionId":"review-1","playbookId":"daily-sales-report-export","requestedStartDate":"2026-06-01","requestedEndDate":"2026-06-30","reportId":"daily-sales","artifactId":"csv-1","artifactSha256":"a".repeat(64),"artifactMediaType":"text/csv","originalOperationId":"report-submission-1","verification":"report_identity_requested_dates_and_content"});
    let repeat = call(&repeated);
    assert!(repeat["error"].is_null(), "{repeat}");
    let inert: Value =
        serde_json::from_str(repeat["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(inert["data"]["executionAllowed"], false);
    assert_eq!(inert["data"]["operatorReviewQualified"], false);
    for (field, value) in [
        ("taskId", json!("task")),
        ("requestedEndDate", json!("2026-02-31")),
        ("artifactSha256", json!("invented")),
        ("reviewDecisionId", json!("peer-review")),
    ] {
        let mut invalid = repeated.clone();
        invalid["teaching"]["validation"][field] = value;
        assert_eq!(
            call(&invalid)["error"]["data"]["message"],
            "teaching_draft_contract_invalid"
        );
    }
    let mut malformed_review = repeated.clone();
    malformed_review["teaching"]["review"]["reviewedSha256"] = json!("invented");
    assert_eq!(
        call(&malformed_review)["error"]["data"]["message"],
        "teaching_draft_contract_invalid"
    );
    repeated["teaching"]["lineage"]["operationKind"] = json!("promote");
    assert!(call(&repeated)["error"].is_null());
    let long_task = format!("teaching-workflow-{}-profile", "a".repeat(160));
    let mut retained_task = state.sessions.get("task").unwrap().clone();
    retained_task.id = long_task.clone();
    retained_task.current_tab_id = None;
    state.sessions.insert(long_task.clone(), retained_task);
    store.save_session_state(&state).unwrap();
    let mut long_guide = repeated.clone();
    long_guide["teaching"]["sourceTaskId"] = json!(long_task);
    long_guide["teaching"]["review"]["taskId"] = json!(long_task);
    long_guide["teaching"]["clarification"]["answers"][0]["taskId"] = json!(long_task);
    long_guide["teaching"]["validation"]["sourceTaskId"] = json!(long_task);
    long_guide["teaching"]["validation"]["taskId"] = json!(format!("fresh-{long_task}"));
    assert!(call_for(&long_guide, &long_task)["error"].is_null());
    for field in ["sourceTaskId", "review", "validation"] {
        let mut oversized = long_guide.clone();
        if field == "sourceTaskId" {
            oversized["teaching"][field] = json!("a".repeat(4001));
        } else {
            oversized["teaching"][field]["taskId"] = json!("a".repeat(4001));
        }
        assert_eq!(
            call_for(&oversized, &long_task)["error"]["data"]["message"],
            "teaching_draft_contract_invalid"
        );
    }
    std::fs::write(
        candidates.join("daily-sales-draft-v1-r4.json"),
        repeated.to_string(),
    )
    .unwrap();
    let denied = super::super::handle_jsonrpc_line(&json!({"jsonrpc":"2.0","id":476,"method":"tools/call","params":{"name":"browser_command","arguments":{"taskId":"task","action":"playbook","params":{"operation":"verify_login","guideId":"daily-sales-draft-v1-r4"}}}}).to_string(),"unused-transport").unwrap();
    assert_eq!(
        denied["error"]["data"]["message"], "teaching_draft_not_executable",
        "stored repeat/promote labels do not grant stock MCP execution"
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
