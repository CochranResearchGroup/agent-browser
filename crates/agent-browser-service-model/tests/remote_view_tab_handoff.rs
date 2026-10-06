use agent_browser_service_model::{
    resolve_remote_view_tab_handoff, retain_remote_view_tab_handoff, BrowserSessionState,
    ManagedBrowserInstance, ManagedBrowserSession, ManagedBrowserTab,
};

fn state() -> BrowserSessionState {
    let mut state = BrowserSessionState::default();
    state.browsers.insert(
        "browser-a".into(),
        ManagedBrowserInstance {
            id: "browser-a".into(),
            profile_id: "profile-a".into(),
            pid: 10,
            cdp_endpoint: "ws://localhost/a".into(),
            desktop: None,
            active_session_ids: vec!["session-a".into()],
        },
    );
    state.sessions.insert(
        "session-a".into(),
        ManagedBrowserSession {
            id: "session-a".into(),
            name: "Alice".into(),
            profile_id: "profile-a".into(),
            browser_id: "browser-a".into(),
            created_at_ms: 1,
            last_activity_at_ms: 1,
            expires_at_ms: 100,
            current_tab_id: Some("tab-a".into()),
        },
    );
    for id in ["tab-a", "tab-b"] {
        state.tabs.insert(
            id.into(),
            ManagedBrowserTab {
                id: id.into(),
                target_id: format!("target:{id}"),
                browser_id: "browser-a".into(),
                session_id: "session-a".into(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
            },
        );
    }
    state
}

#[test]
fn tab_handoff_survives_restart_current_tab_changes_and_browser_relocation() {
    let mut state = state();
    let first =
        retain_remote_view_tab_handoff(&mut state, "opaque-a", "session-a", "tab-a").unwrap();
    assert_eq!(
        first,
        retain_remote_view_tab_handoff(&mut state, "unused-id", "session-a", "tab-a").unwrap()
    );
    retain_remote_view_tab_handoff(&mut state, "opaque-b", "session-a", "tab-b").unwrap();
    state.sessions.get_mut("session-a").unwrap().current_tab_id = Some("tab-b".into());
    let mut restarted: BrowserSessionState =
        serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    let mut browser = restarted.browsers.remove("browser-a").unwrap();
    browser.id = "browser-recovered".into();
    browser.pid = 20;
    restarted.sessions.get_mut("session-a").unwrap().browser_id = browser.id.clone();
    for tab in restarted.tabs.values_mut() {
        tab.browser_id = browser.id.clone();
    }
    restarted.browsers.insert(browser.id.clone(), browser);
    let resolved = resolve_remote_view_tab_handoff(&restarted, "opaque-a").unwrap();
    assert_eq!(resolved.tab.id, "tab-a");
    assert_eq!(resolved.browser.id, "browser-recovered");
    assert_eq!(
        resolve_remote_view_tab_handoff(&restarted, "opaque-b")
            .unwrap()
            .tab
            .id,
        "tab-b"
    );
}

#[test]
fn conflicting_or_closed_targets_never_rebind_or_mutate_handoffs() {
    let mut state = state();
    retain_remote_view_tab_handoff(&mut state, "opaque-a", "session-a", "tab-a").unwrap();
    let before = state.clone();
    assert!(retain_remote_view_tab_handoff(&mut state, "opaque-a", "session-a", "tab-b").is_err());
    assert!(retain_remote_view_tab_handoff(&mut state, "../bad", "session-a", "tab-b").is_err());
    assert_eq!(before, state);
    state.tabs.get_mut("tab-a").unwrap().session_id = "foreign".into();
    assert!(resolve_remote_view_tab_handoff(&state, "opaque-a").is_err());
    state.tabs.remove("tab-a");
    assert!(resolve_remote_view_tab_handoff(&state, "opaque-a").is_err());
    assert_eq!(
        state.remote_view_tab_handoffs,
        before.remote_view_tab_handoffs
    );
}

#[test]
fn old_session_documents_default_to_no_tab_handoffs() {
    let mut document = serde_json::to_value(state()).unwrap();
    document
        .as_object_mut()
        .unwrap()
        .remove("remoteViewTabHandoffs");
    let restored: BrowserSessionState = serde_json::from_value(document).unwrap();
    assert!(restored.remote_view_tab_handoffs.is_empty());
}
