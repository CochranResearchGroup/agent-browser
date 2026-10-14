use agent_browser_service_model::*;

fn intent() -> BrowserLaunchIntent {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap();
    BrowserLaunchIntent {
        intent_id: "33333333-3333-3333-3333-333333333333".into(),
        profile_id: "profile-a".into(),
        assignment: serde_json::from_value(fixture["assignment"].clone()).unwrap(),
    }
}
fn launch(intent: &BrowserLaunchIntent) -> BrowserLaunch {
    BrowserLaunch {
        process_identity: None,
        browser_id: "browser-a".into(),
        pid: 42,
        cdp_endpoint: "http://127.0.0.1:9222".into(),
        desktop: Some(RemoteViewFixedDesktop {
            desktop_id: intent.assignment.desktop_id.clone(),
            generation: intent.assignment.generation,
            friendly_route_label: "synthetic".into(),
        }),
    }
}

#[test]
fn interrupted_launch_retains_intent_until_exact_browser_publication() {
    let intent = intent();
    let mut record = BrowserLaunchCustodyRecord::pending(intent.clone()).unwrap();
    let bytes = serde_json::to_vec(&record).unwrap();
    record = serde_json::from_slice(&bytes).unwrap();
    assert!(!record.published);
    assert_eq!(
        record
            .confirm_publication(&BrowserSessionState::default())
            .unwrap_err(),
        BrowserLaunchCustodyError::PublicationMissing
    );
    let launch = launch(&intent);
    record.observe(&launch).unwrap();
    record.observe(&launch).unwrap();
    let mut changed = launch.clone();
    changed.pid += 1;
    assert_eq!(
        record.observe(&changed).unwrap_err(),
        BrowserLaunchCustodyError::IdentityConflict
    );
    let mut state = BrowserSessionState::default();
    state.browsers.insert(
        launch.browser_id.clone(),
        ManagedBrowserInstance {
            id: launch.browser_id,
            profile_id: intent.profile_id,
            pid: launch.pid,
            cdp_endpoint: launch.cdp_endpoint,
            desktop: launch.desktop,
            active_session_ids: vec![],
        },
    );
    record.confirm_publication(&state).unwrap();
    assert!(record.published);
    record.validate().unwrap();
    let encoded = serde_json::to_value(&record).unwrap();
    assert!(encoded.get("environment").is_none());
    assert!(encoded.get("cdpEndpoint").is_none());
}

#[test]
fn launch_custody_rejects_partial_observation_and_inexact_publication() {
    let intent = intent();
    let mut record = BrowserLaunchCustodyRecord::pending(intent.clone()).unwrap();
    record.observed_pid = Some(42);
    assert_eq!(
        record.validate().unwrap_err(),
        BrowserLaunchCustodyError::InvalidRecord
    );
    record.observed_pid = None;
    let mut launch = launch(&intent);
    launch.desktop.as_mut().unwrap().generation += 1;
    assert_eq!(
        record.observe(&launch).unwrap_err(),
        BrowserLaunchCustodyError::IdentityConflict
    );
    assert_eq!(record.observed_pid, None);
    launch.desktop.as_mut().unwrap().generation -= 1;
    record.observe(&launch).unwrap();
    let mut state = BrowserSessionState::default();
    state.browsers.insert(
        launch.browser_id.clone(),
        ManagedBrowserInstance {
            id: launch.browser_id,
            profile_id: "other-profile".into(),
            pid: launch.pid,
            cdp_endpoint: launch.cdp_endpoint,
            desktop: launch.desktop,
            active_session_ids: vec![],
        },
    );
    assert_eq!(
        record.confirm_publication(&state).unwrap_err(),
        BrowserLaunchCustodyError::IdentityConflict
    );
    assert!(!record.published);
}

#[test]
fn launch_custody_preserves_original_kernel_identity_after_restart() {
    let intent = intent();
    let mut launched = launch(&intent);
    let original = RecordedProcessIdentity {
        pid: launched.pid,
        start_token: "linux:boot:original".into(),
        executable_path: Some("/synthetic/chrome".into()),
        browser_family: None,
    };
    launched.process_identity = Some(original.clone());
    let mut record = BrowserLaunchCustodyRecord::pending(intent).unwrap();
    record.observe(&launched).unwrap();
    let mut restarted: BrowserLaunchCustodyRecord =
        serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap();
    assert_eq!(restarted.observed_process_identity, Some(original));
    let original_record = restarted.clone();
    let mut reused = launched.clone();
    reused.process_identity.as_mut().unwrap().start_token = "linux:boot:replacement".into();
    assert_eq!(
        restarted.observe(&reused).unwrap_err(),
        BrowserLaunchCustodyError::IdentityConflict
    );
    assert_eq!(restarted, original_record);
    let mut missing = launched;
    missing.process_identity = None;
    assert_eq!(
        restarted.observe(&missing).unwrap_err(),
        BrowserLaunchCustodyError::IdentityConflict
    );
}

#[test]
fn launch_custody_does_not_backfill_a_legacy_process_identity() {
    let intent = intent();
    let mut launched = launch(&intent);
    let mut original = BrowserLaunchCustodyRecord::pending(intent).unwrap();
    original.observe(&launched).unwrap();
    let bytes = serde_json::to_vec(&original).unwrap();
    assert!(!String::from_utf8(bytes.clone())
        .unwrap()
        .contains("observedProcessIdentity"));
    let mut legacy: BrowserLaunchCustodyRecord = serde_json::from_slice(&bytes).unwrap();
    launched.process_identity = Some(RecordedProcessIdentity {
        pid: launched.pid,
        start_token: "linux:boot:current-process".into(),
        executable_path: Some("/synthetic/chrome".into()),
        browser_family: None,
    });
    assert_eq!(
        legacy.observe(&launched).unwrap_err(),
        BrowserLaunchCustodyError::IdentityConflict
    );
    assert_eq!(legacy, original);
}
