use super::*;
use crate::native::{
    browser_session_store::BrowserSessionSqliteStore, browser_task_operations as journal,
};
use agent_browser_service_model::BrowserSessionState;
struct LoginFixture {
    root: std::path::PathBuf,
    child: std::process::Child,
    _guard: crate::test_utils::EnvGuard<'static>,
}
impl Drop for LoginFixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn fixture() -> LoginFixture {
    let root = std::env::temp_dir().join(format!("native-login-journal-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let chrome = root.join("chrome");
    std::fs::copy("/bin/sleep", &chrome).unwrap();
    let child = std::process::Command::new(&chrome)
        .arg("120")
        .spawn()
        .unwrap();
    let crate::process_identity::ProcessObservation::Observed(process) =
        crate::process_identity::observe_process(child.id())
    else {
        panic!("fixture process")
    };
    let guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
    ]);
    guard.set("HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    let mut state = serde_json::to_value(BrowserSessionState::default()).unwrap();
    state["sessions"] = json!({"task":{"id":"task","name":"login","profileId":"profile","browserId":"browser","createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":0,"currentTabId":"tab"}});
    state["browsers"] = json!({"browser":{"id":"browser","profileId":"profile","pid":child.id(),"cdpEndpoint":"","desktop":null,"activeSessionIds":["task"]}});
    state["tabs"] = json!({"tab":{"id":"tab","targetId":"target","browserId":"browser","sessionId":"task","createdAtMs":1,"lastActivityAtMs":1}});
    state["sharedBrowserIds"] = json!(["browser"]);
    state["adoptedBrowserProcessIdentities"] = json!({"browser":{"profileId":"profile","cdpEndpoint":"","process":{"pid":process.pid,"startToken":process.start_token,"executablePath":process.executable_path,"browserFamily":process.browser_family}}});
    BrowserSessionSqliteStore::default_sqlite()
        .unwrap()
        .save_session_state(&serde_json::from_value(state).unwrap())
        .unwrap();
    LoginFixture {
        root,
        child,
        _guard: guard,
    }
}
fn clock() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn observed(kind: &str) -> Value {
    let reference = format!("login:{}", "a".repeat(64));
    let mut controls = vec![
        json!({"kind":kind,"targetRef":reference,"label":match kind{"username"=>"Username","credential_entry"=>"Saved account",_=>"Sign in"},"bounds":{"x":100,"y":150,"width":300,"height":40},"configuredAccountMatch":true,"fieldValueState":"empty"}),
    ];
    if kind == "submit" {
        controls.push(json!({"kind":"username","label":"Username","targetRef":format!("login:{}","c".repeat(64)),"bounds":{"x":100,"y":50,"width":300,"height":40},"configuredAccountMatch":true,"fieldValueState":"text_present"}));
        controls.push(json!({"kind":"password","label":"Password","targetRef":format!("login:{}","d".repeat(64)),"bounds":{"x":100,"y":100,"width":300,"height":40},"configuredAccountMatch":false,"fieldValueState":"masked"}));
    }
    json!({"binding":{"desktopId":"desktop","generation":7,"windowId":11,"window":{"x":0,"y":0,"width":800,"height":600},"displayWidth":800,"displayHeight":600,"region":{"x":0,"y":0,"width":800,"height":600},"nativeControlEpoch":4,"observedAtMs":clock()},"controls":{"schemaVersion":"agent-browser.native-login-controls.v1","profile":"english-chrome-v1","status":"observed","controls":controls,"frameSha256":"b".repeat(64)}})
}
fn capture(id: &str, kind: &str) {
    let params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_capture","requestId":id,"params":{"loginControls":{"profile":"english-chrome-v1"}}}});
    task_operations::with_context(Some(&params), || {
        let response = task_operations::transport(
            json!({"action":"native_capture","loginControls":{"profile":"english-chrome-v1"}}),
            |_| {
                Ok(crate::connection::Response {
                    success: true,
                    data: Some(json!({"nativeLoginObservation":observed(kind)})),
                    ..Default::default()
                })
            },
        )
        .map_err(|error| JsonRpcError::invalid_params(&error))?;
        assert!(response.success);
        Ok(Value::Null)
    })
    .unwrap();
}
fn input(
    id: &str,
    observation: &str,
    episode: &str,
    lose: bool,
    calls: &mut usize,
) -> Result<(), String> {
    let target = json!({"observationRequestId":observation,"episodeId":episode,"targetRef":format!("login:{}","a".repeat(64))});
    let params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_input","requestId":id,"params":{"loginTarget":target}}});
    task_operations::with_context(Some(&params), || {
        let r = task_operations::transport(
            json!({"action":"native_input","loginTarget":target}),
            |command| {
                journal::native_login_attempt_observation(&command)?;
                *calls += 1;
                if lose {
                    return Err("fixture lost acknowledgement".into());
                }
                Ok(crate::connection::Response {
                    success: true,
                    data: Some(json!({"source":"remote_view_native","delivery":"dispatched"})),
                    ..Default::default()
                })
            },
        )
        .map_err(|error| JsonRpcError::invalid_params(&error))?;
        assert!(
            r.success
                || r.data
                    .as_ref()
                    .is_some_and(|d| d["dispatchAttempted"] == false)
        );
        Ok(Value::Null)
    })
    .map(|_| ())
    .map_err(native_request_error)
}
#[test]
fn native_login_perception_public_journal_retains_three_steps_and_refuses_replay_or_new_episode() {
    let _fixture = fixture();
    let mut calls = 0;
    for (n, kind) in ["username", "credential_entry", "submit"]
        .iter()
        .enumerate()
    {
        let cap = format!("capture-{n}");
        let id = format!("input-{n}");
        capture(&cap, kind);
        input(&id, &cap, "original-episode", false, &mut calls).unwrap();
        input(&id, &cap, "original-episode", false, &mut calls).unwrap();
    }
    assert_eq!(calls, 3);
    let original = task_operations::lookup("task", "input-2").unwrap();
    assert_eq!(original["operation"]["state"], "owner_acknowledged");
    assert_eq!(
        original["operation"]["evidenceReferences"]["nativeLoginStep"],
        "2"
    );
    capture("fresh-capture", "username");
    assert!(input(
        "new-episode",
        "fresh-capture",
        "renewed-episode",
        false,
        &mut calls
    )
    .unwrap_err()
    .contains("episode_changed"));
    assert!(input(
        "extra-input",
        "fresh-capture",
        "original-episode",
        false,
        &mut calls
    )
    .unwrap_err()
    .contains("budget_exhausted"));
    assert_eq!(calls, 3);
}
#[test]
fn native_login_perception_public_journal_unknown_input_survives_reopen_and_blocks_progression() {
    let _fixture = fixture();
    let mut calls = 0;
    capture("cap-0", "username");
    input("input-0", "cap-0", "episode", false, &mut calls).unwrap();
    capture("cap-1", "credential_entry");
    assert!(input("input-1", "cap-1", "episode", true, &mut calls).is_err());
    let reopened = BrowserSessionSqliteStore::default_sqlite().unwrap();
    let original =
        reopened.load_session_state().unwrap().task_operations["task"]["input-1"].clone();
    assert_ne!(
        original.state,
        agent_browser_service_model::BrowserTaskOperationState::OwnerAcknowledged
    );
    capture("fresh-after-unknown", "submit");
    assert!(input(
        "input-2",
        "fresh-after-unknown",
        "episode",
        false,
        &mut calls
    )
    .unwrap_err()
    .contains("unsettled"));
    assert_eq!(calls, 2);
}
fn age_acknowledged_prefix() -> (u64, u64) {
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    let mut state = store.load_session_state().unwrap();
    let start = clock() - 7_200_000;
    let rows = state.task_operations.get_mut("task").unwrap();
    rows.get_mut("input-0").unwrap().created_at_ms = start;
    rows.get_mut("input-1").unwrap().created_at_ms = start + 1000;
    store.save_session_state(&state).unwrap();
    (start, start + 1000)
}
#[test]
fn native_login_perception_public_journal_old_ack_prefix_uses_fresh_submit_without_clock_rewrite() {
    let _fixture = fixture();
    let mut calls = 0;
    for (n, kind) in ["username", "credential_entry"].iter().enumerate() {
        let cap = format!("cap-{n}");
        capture(&cap, kind);
        input(&format!("input-{n}"), &cap, "episode", false, &mut calls).unwrap();
    }
    let original = age_acknowledged_prefix();
    capture("fresh-submit", "submit");
    input("input-2", "fresh-submit", "episode", false, &mut calls).unwrap();
    assert_eq!(calls, 3);
    let state = BrowserSessionSqliteStore::default_sqlite()
        .unwrap()
        .load_session_state()
        .unwrap();
    assert_eq!(
        (
            state.task_operations["task"]["input-0"].created_at_ms,
            state.task_operations["task"]["input-1"].created_at_ms
        ),
        original
    );
}
#[test]
fn native_login_perception_public_journal_old_ack_prefix_still_refuses_rollback_context_and_stale_frame(
) {
    for scenario in ["rollback", "context", "stale"] {
        let _fixture = fixture();
        let mut calls = 0;
        for (n, kind) in ["username", "credential_entry"].iter().enumerate() {
            let cap = format!("cap-{n}");
            capture(&cap, kind);
            input(&format!("input-{n}"), &cap, "episode", false, &mut calls).unwrap();
        }
        age_acknowledged_prefix();
        let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
        let mut state = store.load_session_state().unwrap();
        if scenario == "rollback" {
            state
                .task_operations
                .get_mut("task")
                .unwrap()
                .get_mut("input-0")
                .unwrap()
                .created_at_ms = clock() + 60_000;
        }
        if scenario == "context" {
            state.tabs.get_mut("tab").unwrap().target_id = "foreign-target".into();
        }
        store.save_session_state(&state).unwrap();
        drop(store);
        capture("new-submit", "submit");
        if scenario == "stale" {
            let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
            let mut state = store.load_session_state().unwrap();
            let row = state
                .task_operations
                .get_mut("task")
                .unwrap()
                .get_mut("new-submit")
                .unwrap();
            let mut observation: Value =
                serde_json::from_str(&row.evidence_references["nativeLoginObservation"]).unwrap();
            observation["binding"]["observedAtMs"] = json!(clock() - 10_001);
            row.evidence_references.insert(
                "nativeLoginObservation".into(),
                serde_json::to_string(&observation).unwrap(),
            );
            store.save_session_state(&state).unwrap();
        }
        assert!(
            input("input-2", "new-submit", "episode", false, &mut calls).is_err(),
            "{scenario}"
        );
        assert_eq!(calls, 2);
    }
}

#[test]
fn native_login_perception_public_journal_legacy_capture_is_not_acknowledged_or_replayed() {
    let _fixture = fixture();
    let params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_capture","requestId":"legacy-capture","params":{"loginControls":{"profile":"english-chrome-v1"}}}});
    let mut calls = 0;
    task_operations::with_context(Some(&params),||{let failure=task_operations::transport(json!({"action":"native_capture","loginControls":{"profile":"english-chrome-v1"}}),|_|{calls+=1;Ok(crate::connection::Response{success:true,data:Some(json!({"source":"remote_view_native","capture":{"image":{"data":"private-legacy-image-canary"},"ocr":{"text":"private-password-canary"}}})),..Default::default()})}).err().expect("legacy producer must refuse completion");assert!(!failure.contains("canary"));assert!(failure.contains("browser_native_login_producer_contract_unavailable"));Ok(Value::Null)}).unwrap();
    assert_eq!(calls, 1);
    let original = task_operations::lookup("task", "legacy-capture").unwrap();
    assert_eq!(original["operation"]["state"], "pending");
    assert!(!original.to_string().contains("canary"));
    task_operations::with_context(Some(&params), || {
        let replay = task_operations::transport(
            json!({"action":"native_capture","loginControls":{"profile":"english-chrome-v1"}}),
            |_| panic!("legacy capture must not replay"),
        )
        .unwrap();
        assert!(!replay.success);
        Ok(Value::Null)
    })
    .unwrap();
}
