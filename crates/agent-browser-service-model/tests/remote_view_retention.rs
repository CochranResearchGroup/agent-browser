use std::collections::BTreeMap;

use agent_browser_service_model::{
    detach_remote_view_presentation, project_remote_view_operator_handoff,
    release_remote_view_presentation, retain_remote_view_presentation, BrowserSessionState,
    ManagedBrowserInstance, ManagedBrowserSession, ManagedBrowserTab, RemoteViewAssignmentRecord,
    RemoteViewAssignmentState, RemoteViewDesktopPresentationBinding,
    RemoteViewDesktopViewingRetirement, RemoteViewFixedDesktop, RemoteViewJoinedReleaseOutcome,
    RemoteViewPresentationRetentionState,
};

fn fixture_state() -> BrowserSessionState {
    BrowserSessionState {
        browsers: BTreeMap::from([(
            "browser-a".into(),
            ManagedBrowserInstance {
                id: "browser-a".into(),
                profile_id: "profile-a".into(),
                pid: 42,
                cdp_endpoint: "http://127.0.0.1:9222".into(),
                desktop: Some(RemoteViewFixedDesktop {
                    desktop_id: "11111111-1111-1111-1111-111111111111".into(),
                    friendly_route_label: "desktop-a".into(),
                    generation: 7,
                }),
                active_session_ids: vec!["session-a".into()],
            },
        )]),
        sessions: BTreeMap::from([(
            "session-a".into(),
            ManagedBrowserSession {
                id: "session-a".into(),
                name: "work".into(),
                profile_id: "profile-a".into(),
                browser_id: "browser-a".into(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
                expires_at_ms: 10,
                current_tab_id: Some("tab-a".into()),
            },
        )]),
        tabs: BTreeMap::from([(
            "tab-a".into(),
            ManagedBrowserTab {
                id: "tab-a".into(),
                target_id: "target-a".into(),
                browser_id: "browser-a".into(),
                session_id: "session-a".into(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
            },
        )]),
        ..BrowserSessionState::default()
    }
}

fn binding() -> RemoteViewDesktopPresentationBinding {
    RemoteViewDesktopPresentationBinding {
        registration_id: "registration-a".into(),
        pool_id: "pool-a".into(),
        desktop_id: "11111111-1111-1111-1111-111111111111".into(),
        generation: 7,
        assignment_id: "assignment-a".into(),
        placement_id: "placement-a".into(),
        route_id: "22222222-2222-2222-2222-222222222222".into(),
        viewer_session_ids: vec!["viewer-desktop".into(), "viewer-mobile".into()],
    }
}

fn add_peer_browser(state: &mut BrowserSessionState) {
    state.browsers.insert(
        "browser-b".into(),
        ManagedBrowserInstance {
            id: "browser-b".into(),
            profile_id: "profile-b".into(),
            pid: 43,
            cdp_endpoint: "http://127.0.0.1:9223".into(),
            desktop: state.browsers["browser-a"].desktop.clone(),
            active_session_ids: vec!["session-b".into()],
        },
    );
    state.sessions.insert(
        "session-b".into(),
        ManagedBrowserSession {
            id: "session-b".into(),
            name: "personal".into(),
            profile_id: "profile-b".into(),
            browser_id: "browser-b".into(),
            created_at_ms: 1,
            last_activity_at_ms: 2,
            expires_at_ms: 10,
            current_tab_id: Some("tab-b".into()),
        },
    );
    state.tabs.insert(
        "tab-b".into(),
        ManagedBrowserTab {
            id: "tab-b".into(),
            target_id: "target-b".into(),
            browser_id: "browser-b".into(),
            session_id: "session-b".into(),
            created_at_ms: 1,
            last_activity_at_ms: 2,
        },
    );
}

fn release() -> RemoteViewJoinedReleaseOutcome {
    RemoteViewJoinedReleaseOutcome {
        assignment: RemoteViewAssignmentRecord {
            assignment_id: "assignment-a".into(),
            registration_id: "registration-a".into(),
            pool_id: "pool-a".into(),
            desktop_id: "11111111-1111-1111-1111-111111111111".into(),
            generation: 7,
            state: RemoteViewAssignmentState::Released,
        },
        retirement: RemoteViewDesktopViewingRetirement {
            schema_version: 1,
            desktop_id: "11111111-1111-1111-1111-111111111111".into(),
            generation: 7,
            routes: vec!["22222222-2222-2222-2222-222222222222".into()],
            sessions: vec!["viewer-mobile".into(), "viewer-desktop".into()],
        },
    }
}

#[test]
fn exact_public_binding_is_retained_idempotently_without_provider_urls() {
    let mut state = fixture_state();
    let first =
        retain_remote_view_presentation(&mut state, "browser-a", "session-a", "tab-a", &binding())
            .unwrap();
    let mut reordered = binding();
    reordered.viewer_session_ids.reverse();
    let replay =
        retain_remote_view_presentation(&mut state, "browser-a", "session-a", "tab-a", &reordered)
            .unwrap();

    assert_eq!(first, replay);
    assert_eq!(first.target_id, "target-a");
    assert_eq!(first.state, RemoteViewPresentationRetentionState::Active);
    let encoded = serde_json::to_value(&state).unwrap();
    let retained = &encoded["remoteViewPresentations"]["browser-a"];
    assert_eq!(retained["routeId"], binding().route_id);
    assert_eq!(retained["placementId"], "");
    assert!(retained.get("providerUrl").is_none());
    assert!(retained.get("display").is_none());
}

#[test]
fn release_requires_the_exact_assignment_generation_route_and_viewer_set() {
    let mut state = fixture_state();
    retain_remote_view_presentation(&mut state, "browser-a", "session-a", "tab-a", &binding())
        .unwrap();
    let release = release();

    let released = release_remote_view_presentation(&mut state, "browser-a", &release).unwrap();
    assert_eq!(
        released.state,
        RemoteViewPresentationRetentionState::Released
    );
    assert_eq!(
        release_remote_view_presentation(&mut state, "browser-a", &release).unwrap(),
        released
    );

    let mut stale = fixture_state();
    retain_remote_view_presentation(&mut stale, "browser-a", "session-a", "tab-a", &binding())
        .unwrap();
    let mut wrong_generation = release;
    wrong_generation.retirement.generation = 8;
    assert!(release_remote_view_presentation(&mut stale, "browser-a", &wrong_generation).is_err());
    assert_eq!(
        stale.remote_view_presentations["browser-a"].state,
        RemoteViewPresentationRetentionState::Active
    );
}

#[test]
fn exact_desktop_assignment_can_be_shared_and_releases_only_after_final_reference() {
    let mut state = fixture_state();
    add_peer_browser(&mut state);
    retain_remote_view_presentation(&mut state, "browser-a", "session-a", "tab-a", &binding())
        .unwrap();
    let mut peer_binding = binding();
    peer_binding.placement_id = "placement-b".into();
    let peer = retain_remote_view_presentation(
        &mut state,
        "browser-b",
        "session-b",
        "tab-b",
        &peer_binding,
    )
    .unwrap();

    assert_eq!(peer.browser_id, "browser-b");
    assert_eq!(peer.target_id, "target-b");
    assert_eq!(peer.assignment_id, "assignment-a");
    assert_eq!(
        release_remote_view_presentation(&mut state, "browser-b", &release()),
        Err("remote_view_retention_release_still_referenced:browser-b:1".into())
    );

    assert_eq!(
        detach_remote_view_presentation(&mut state, "browser-a").unwrap(),
        1
    );
    assert_eq!(
        state.remote_view_presentations["browser-a"].state,
        RemoteViewPresentationRetentionState::Detached
    );
    let released = release_remote_view_presentation(&mut state, "browser-b", &release()).unwrap();
    assert_eq!(
        released.state,
        RemoteViewPresentationRetentionState::Released
    );
}

#[test]
fn shared_desktop_identity_must_match_exactly() {
    let mut state = fixture_state();
    add_peer_browser(&mut state);
    retain_remote_view_presentation(&mut state, "browser-a", "session-a", "tab-a", &binding())
        .unwrap();
    let mut mismatched = binding();
    mismatched.assignment_id = "assignment-b".into();
    mismatched.placement_id = "placement-b".into();

    assert_eq!(
        retain_remote_view_presentation(&mut state, "browser-b", "session-b", "tab-b", &mismatched,),
        Err("remote_view_retention_identity_already_bound".into())
    );
    detach_remote_view_presentation(&mut state, "browser-a").unwrap();
    assert_eq!(
        retain_remote_view_presentation(&mut state, "browser-b", "session-b", "tab-b", &mismatched,),
        Err("remote_view_retention_identity_already_bound".into())
    );
}

#[test]
fn operator_handoff_projection_returns_only_agent_browser_durable_identity() {
    let mut state = fixture_state();
    let retained =
        retain_remote_view_presentation(&mut state, "browser-a", "session-a", "tab-a", &binding())
            .unwrap();

    let link = project_remote_view_operator_handoff(&retained, "handoff-123").unwrap();
    assert_eq!(link.handoff_id, "handoff-123");
    assert_eq!(link.handoff_url, "/remote-view/handoff-123");
    assert_eq!(
        serde_json::to_value(&link).unwrap(),
        serde_json::json!({
            "handoffId": "handoff-123",
            "handoffUrl": "/remote-view/handoff-123"
        })
    );
    assert!(project_remote_view_operator_handoff(&retained, "route/id").is_err());

    let mut released = retained;
    released.state = RemoteViewPresentationRetentionState::Released;
    assert!(project_remote_view_operator_handoff(&released, "handoff-123").is_err());
}
