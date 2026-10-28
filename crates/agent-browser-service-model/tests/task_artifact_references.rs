use agent_browser_service_model::{BrowserSessionState, BrowserTaskArtifactReference};
use serde_json::json;

fn state() -> BrowserSessionState {
    let mut state = BrowserSessionState::default();
    state.sessions.insert(
        "task".into(),
        serde_json::from_value(json!({
            "id":"task","name":"lane","profileId":"profile","browserId":"browser",
            "createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":100,"currentTabId":"tab"
        }))
        .unwrap(),
    );
    state.browsers.insert(
        "browser".into(),
        serde_json::from_value(json!({
            "id":"browser","profileId":"profile","pid":42,"cdpEndpoint":"ws://synthetic",
            "activeSessionIds":["task"]
        }))
        .unwrap(),
    );
    state.tabs.insert(
        "tab".into(),
        serde_json::from_value(json!({
            "id":"tab","targetId":"target","browserId":"browser","sessionId":"task",
            "createdAtMs":1,"lastActivityAtMs":1
        }))
        .unwrap(),
    );
    state
}

#[test]
fn completed_capture_survives_normal_activity_and_peer_changes() {
    let before = state();
    let mut after = before.clone();
    // Normal owner activity is progress, not replacement of the addressed task.
    after.sessions.get_mut("task").unwrap().last_activity_at_ms = 20;
    after.sessions.get_mut("task").unwrap().expires_at_ms = 120;
    after.tabs.get_mut("tab").unwrap().last_activity_at_ms = 20;
    let mut peer = after.sessions["task"].clone();
    peer.id = "peer".into();
    peer.name = "peer-lane".into();
    after.sessions.insert("peer".into(), peer.clone());
    after
        .browsers
        .get_mut("browser")
        .unwrap()
        .active_session_ids
        .push("peer".into());
    let reference = BrowserTaskArtifactReference {
        artifact_id: "capture-original".into(),
        kind: "screenshot".into(),
        handle: "reports".into(),
        observed_at_ms: 20,
    };
    after
        .retain_completed_task_artifact(&before, "task", reference.clone())
        .unwrap();
    assert_eq!(after.task_artifact_references["task"], vec![reference]);
    assert_eq!(after.sessions["peer"], peer);
    assert_eq!(after.sessions["task"].last_activity_at_ms, 20);
}

#[test]
fn completed_capture_does_not_publish_under_a_replaced_target_or_process() {
    let before = state();
    for replacement in ["target", "process"] {
        let mut after = before.clone();
        if replacement == "target" {
            after.tabs.get_mut("tab").unwrap().target_id = "other-target".into();
        } else {
            after.browsers.get_mut("browser").unwrap().pid = 99;
        }
        let unchanged = after.clone();
        let result = after.retain_completed_task_artifact(
            &before,
            "task",
            BrowserTaskArtifactReference {
                artifact_id: "capture-original".into(),
                kind: "screenshot".into(),
                handle: "reports".into(),
                observed_at_ms: 20,
            },
        );
        assert!(
            result.is_err(),
            "replacement {replacement} must retain completion uncertainty"
        );
        assert_eq!(
            after, unchanged,
            "refusal cannot mutate a peer or create a reference"
        );
    }
}
