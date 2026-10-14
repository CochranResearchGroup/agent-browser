use agent_browser_service_model::*;
use serde_json::{json, Value};

fn fixture() -> (
    BrowserSessionState,
    RemoteViewIdleRetirementIntent,
    RemoteViewIdleRetirementReceipt,
) {
    let f: Value = serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap();
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(f["assignment"].clone()).unwrap();
    let mut state = BrowserSessionState::default();
    state.browsers.insert(
        "browser-a".into(),
        ManagedBrowserInstance {
            id: "browser-a".into(),
            profile_id: "profile-a".into(),
            pid: 42,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: assignment.desktop_id.clone(),
                generation: assignment.generation,
                friendly_route_label: "fixture".into(),
            }),
            active_session_ids: vec!["session-a".into()],
        },
    );
    for id in ["tab-a", "tab-b"] {
        state.tabs.insert(
            id.into(),
            ManagedBrowserTab {
                id: id.into(),
                target_id: format!("target-{id}"),
                browser_id: "browser-a".into(),
                session_id: "session-a".into(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
            },
        );
    }
    let provider = serde_json::from_value(json!({"schemaVersion":1,"assignment":assignment,
        "target":f["assignmentObservation"]["target"],"idempotencyKey":"close-one","expectedRevision":0,
        "expectedRetirement":{"schemaVersion":1,"desktopId":assignment.desktop_id,"generation":assignment.generation,"routes":[],"sessions":[]}})).unwrap();
    let prepared = serde_json::from_value(json!({"schemaVersion":1,"target":f["assignmentObservation"]["target"],
        "readinessScope":"native_admission","state":"prepared","fence":{"schema_version":1,
            "installation_id":"0123456789abcdef","config_hash":"a".repeat(64),"slot":2,"generation":19,
            "operation_id":"5deb7e63650d881f18b5d68f85ec45bcd7523255499618982a3c2995400637cc","revision":1}})).unwrap();
    (state, provider, prepared)
}

#[test]
fn idle_retirement_custody_requires_ordered_settlement_and_never_readmits_an_unknown_effect() {
    let (state, provider, prepared) = fixture();
    let intent = BrowserIdleRetirementIntent::new(
        "11111111-1111-4111-8111-111111111111",
        provider,
        &state,
        &["browser-a".into()],
    )
    .unwrap();
    let mut record = BrowserIdleRetirementRecord::new(intent).unwrap();
    let effect = BrowserIdleRetirementEffect::CloseTab {
        browser_id: "browser-a".into(),
        tab_id: "tab-a".into(),
        target_id: "target-tab-a".into(),
    };
    assert!(record.begin_effect(effect.clone()).is_err());
    record.begin_preparation().unwrap();
    record.record_prepared(prepared.clone()).unwrap();
    assert_eq!(
        record.begin_effect(effect.clone()).unwrap(),
        BrowserIdleRetirementEffectAdmission::New
    );
    let serialized = serde_json::to_vec(&record).unwrap();
    let mut restarted: BrowserIdleRetirementRecord = serde_json::from_slice(&serialized).unwrap();
    restarted.validate().unwrap();
    assert_eq!(
        restarted.begin_effect(effect.clone()).unwrap(),
        BrowserIdleRetirementEffectAdmission::PendingReadback
    );
    assert!(restarted.record_publication(&state).is_err());
    restarted.settle_effect(&effect).unwrap();
    assert_eq!(
        restarted.begin_effect(effect.clone()).unwrap(),
        BrowserIdleRetirementEffectAdmission::AlreadySettled
    );
    assert!(
        restarted.record_publication(&state).is_err(),
        "a still-live target cannot be published closed"
    );
    let mut completed_json = serde_json::to_value(&prepared).unwrap();
    completed_json["state"] = json!("completed");
    let completed: RemoteViewIdleRetirementReceipt =
        serde_json::from_value(completed_json).unwrap();
    assert!(restarted.record_completed(completed.clone()).is_err());
    let mut closed = state.clone();
    closed.tabs.remove("tab-a");
    restarted.record_publication(&closed).unwrap();
    restarted.record_completed(completed).unwrap();
    assert!(restarted.is_terminal());
    assert!(closed.tabs.contains_key("tab-b"));
    assert!(closed.browsers.contains_key("browser-a"));
    let mut changed = effect;
    if let BrowserIdleRetirementEffect::CloseTab { target_id, .. } = &mut changed {
        *target_id = "foreign-target".into();
    }
    assert!(restarted.begin_effect(changed).is_err());
}

#[test]
fn idle_retirement_publication_preserves_closed_retained_browser_identity() {
    let (state, provider, prepared) = fixture();
    let intent = BrowserIdleRetirementIntent::new(
        "11111111-1111-4111-8111-111111111111",
        provider,
        &state,
        &["browser-a".into()],
    )
    .unwrap();
    let mut record = BrowserIdleRetirementRecord::new(intent).unwrap();
    record.begin_preparation().unwrap();
    record.record_prepared(prepared).unwrap();
    let effect = BrowserIdleRetirementEffect::CloseBrowser {
        browser_id: "browser-a".into(),
    };
    record.begin_effect(effect.clone()).unwrap();
    record.settle_effect(&effect).unwrap();
    assert!(record.clone().record_publication(&state).is_err());
    let mut retained = state.clone();
    retained
        .idle_closed_browsers
        .insert("browser-a".into(), 100);
    record.clone().record_publication(&retained).unwrap();
    retained
        .browsers
        .get_mut("browser-a")
        .unwrap()
        .active_session_ids
        .clear();
    record.clone().record_publication(&retained).unwrap();
    retained.browsers.get_mut("browser-a").unwrap().pid = 43;
    assert!(
        record.record_publication(&retained).is_err(),
        "a reused logical ID must not publish a replacement as the retired browser"
    );
}

#[test]
fn idle_retirement_publication_preserves_unattempted_peers() {
    let (state, provider, prepared) = fixture();
    let intent = BrowserIdleRetirementIntent::new(
        "11111111-1111-4111-8111-111111111111",
        provider,
        &state,
        &["browser-a".into()],
    )
    .unwrap();
    let mut record = BrowserIdleRetirementRecord::new(intent).unwrap();
    record.begin_preparation().unwrap();
    record.record_prepared(prepared).unwrap();
    let mut closed = state.clone();
    closed.tabs.remove("tab-a");
    assert!(
        record.clone().record_publication(&closed).is_err(),
        "a prepared fence does not prove an unattempted tab ended"
    );
    let effect = BrowserIdleRetirementEffect::CloseTab {
        browser_id: "browser-a".into(),
        tab_id: "tab-a".into(),
        target_id: "target-tab-a".into(),
    };
    record.begin_effect(effect.clone()).unwrap();
    record.settle_effect(&effect).unwrap();
    record.clone().record_publication(&closed).unwrap();
    closed.tabs.remove("tab-b");
    assert!(
        record.clone().record_publication(&closed).is_err(),
        "settling one target must preserve its unattempted peer"
    );
    let mut rebound = state.clone();
    rebound.tabs.remove("tab-a");
    rebound.browsers.get_mut("browser-a").unwrap().pid = 43;
    assert!(
        record.record_publication(&rebound).is_err(),
        "a target close does not authorize replacing its browser identity"
    );
}
