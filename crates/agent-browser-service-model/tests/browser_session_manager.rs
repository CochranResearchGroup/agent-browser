use std::collections::BTreeMap;

use agent_browser_service_model::{
    BrowserLaunch, BrowserProfileCatalog, BrowserProfileCatalogEntry, BrowserProfileKind,
    BrowserSessionEffects, BrowserSessionManager, BrowserSessionManagerConfig, BrowserSessionState,
    BrowserTabAcquisition, BrowserTabSource, OpenBrowserSession, SessionBrowserDisposition,
    SessionCloseDisposition, SessionEndReason, SessionRecordDisposition,
};

#[derive(Default)]
struct FixtureEffects {
    launches: Vec<String>,
    probes: Vec<String>,
    browser_live: bool,
    closes: Vec<String>,
    initial_tab: Option<BrowserTabAcquisition>,
    initial_tab_requests: Vec<String>,
    new_tab: Option<BrowserTabAcquisition>,
    new_tab_requests: Vec<String>,
    tab_closes: Vec<(String, String)>,
}

#[test]
fn repeated_command_reuses_and_refreshes_named_session() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
        },
    );

    let first = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    let repeated = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            2_000,
        ))
        .unwrap();
    drop(manager);

    assert_eq!(
        repeated.session_disposition,
        SessionRecordDisposition::Reused
    );
    assert_eq!(repeated.session_id, first.session_id);
    assert_eq!(effects.launches, ["profile-a"]);
    assert_eq!(state.sessions.len(), 1);
    assert_eq!(state.sessions[&first.session_id].last_activity_at_ms, 2_000);
    assert_eq!(state.sessions[&first.session_id].expires_at_ms, 302_000);
}

#[test]
fn open_after_expiry_ends_old_session_before_new_epoch() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 1_000,
        },
    );
    let first = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();

    let replacement = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            2_000,
        ))
        .unwrap();
    drop(manager);

    assert_ne!(replacement.session_id, first.session_id);
    assert!(!state.sessions.contains_key(&first.session_id));
    assert!(state.sessions.contains_key(&replacement.session_id));
    assert_eq!(state.sessions.len(), 1);
    assert_eq!(state.session_history.len(), 1);
    assert_eq!(
        state.session_history[0].reason,
        SessionEndReason::HeartbeatExpired
    );
}

impl BrowserSessionEffects for FixtureEffects {
    fn browser_is_live(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> Result<bool, String> {
        self.probes.push(browser.id.clone());
        Ok(self.browser_live)
    }

    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
    ) -> Result<BrowserLaunch, String> {
        self.launches.push(profile.id.clone());
        let sequence = self.launches.len();
        Ok(BrowserLaunch {
            browser_id: if sequence == 1 {
                format!("browser:{}", profile.id)
            } else {
                format!("browser:{}:{sequence}", profile.id)
            },
            pid: 4242,
            cdp_endpoint: "http://127.0.0.1:9422".to_string(),
        })
    }

    fn close_browser(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> Result<(), String> {
        self.closes.push(browser.id.clone());
        Ok(())
    }

    fn acquire_initial_tab(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> Result<BrowserTabAcquisition, String> {
        self.initial_tab_requests.push(browser.id.clone());
        self.initial_tab
            .clone()
            .ok_or_else(|| "fixture_initial_tab_missing".to_string())
    }

    fn create_tab(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> Result<BrowserTabAcquisition, String> {
        self.new_tab_requests.push(browser.id.clone());
        self.new_tab
            .clone()
            .ok_or_else(|| "fixture_new_tab_missing".to_string())
    }

    fn close_tab(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
        tab: &agent_browser_service_model::ManagedBrowserTab,
    ) -> Result<(), String> {
        self.tab_closes
            .push((browser.id.clone(), tab.target_id.clone()));
        Ok(())
    }
}

#[test]
fn first_navigation_adopts_bootstrap_tab_without_creating_another() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "tab-bootstrap".to_string(),
            target_id: "target-bootstrap".to_string(),
            source: BrowserTabSource::Bootstrap,
        }),
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
        },
    );
    let alice = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();

    let tab = manager
        .tab_for_navigation(&alice.session_id, 2_000)
        .unwrap();
    drop(manager);

    assert_eq!(tab.source, BrowserTabSource::Bootstrap);
    assert_eq!(tab.tab_id, "tab-bootstrap");
    assert_eq!(effects.initial_tab_requests, [alice.browser_id]);
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(
        state.sessions[&alice.session_id].current_tab_id.as_deref(),
        Some("tab-bootstrap")
    );
    assert_eq!(state.sessions[&alice.session_id].last_activity_at_ms, 2_000);
}

#[test]
fn repeated_navigation_reuses_current_tab_without_another_acquisition() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "tab-bootstrap".to_string(),
            target_id: "target-bootstrap".to_string(),
            source: BrowserTabSource::Bootstrap,
        }),
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
        },
    );
    let alice = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();

    manager
        .tab_for_navigation(&alice.session_id, 2_000)
        .unwrap();
    let repeated = manager
        .tab_for_navigation(&alice.session_id, 3_000)
        .unwrap();
    drop(manager);

    assert_eq!(repeated.source, BrowserTabSource::Current);
    assert_eq!(repeated.tab_id, "tab-bootstrap");
    assert_eq!(effects.initial_tab_requests, [alice.browser_id]);
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs["tab-bootstrap"].last_activity_at_ms, 3_000);
    assert_eq!(state.sessions[&alice.session_id].last_activity_at_ms, 3_000);
}

#[test]
fn explicit_new_tab_is_the_only_ordinary_tab_growth_path() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "tab-bootstrap".to_string(),
            target_id: "target-bootstrap".to_string(),
            source: BrowserTabSource::Bootstrap,
        }),
        new_tab: Some(BrowserTabAcquisition {
            tab_id: "tab-new".to_string(),
            target_id: "target-new".to_string(),
            source: BrowserTabSource::ExplicitNew,
        }),
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
        },
    );
    let alice = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    manager
        .tab_for_navigation(&alice.session_id, 2_000)
        .unwrap();

    let created = manager.new_tab(&alice.session_id, 3_000).unwrap();
    drop(manager);

    assert_eq!(created.source, BrowserTabSource::ExplicitNew);
    assert_eq!(created.tab_id, "tab-new");
    assert_eq!(effects.new_tab_requests, [alice.browser_id]);
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(
        state.sessions[&alice.session_id].current_tab_id.as_deref(),
        Some("tab-new")
    );
}

#[test]
fn closing_current_tab_selects_most_recent_remaining_tab() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "tab-bootstrap".to_string(),
            target_id: "target-bootstrap".to_string(),
            source: BrowserTabSource::Bootstrap,
        }),
        new_tab: Some(BrowserTabAcquisition {
            tab_id: "tab-new".to_string(),
            target_id: "target-new".to_string(),
            source: BrowserTabSource::ExplicitNew,
        }),
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
        },
    );
    let alice = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    manager
        .tab_for_navigation(&alice.session_id, 2_000)
        .unwrap();
    manager.new_tab(&alice.session_id, 3_000).unwrap();

    let closed = manager.close_current_tab(&alice.session_id, 4_000).unwrap();
    drop(manager);

    assert_eq!(closed.closed_tab_id, "tab-new");
    assert_eq!(closed.current_tab_id.as_deref(), Some("tab-bootstrap"));
    assert_eq!(
        effects.tab_closes,
        [(alice.browser_id, "target-new".to_string())]
    );
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(
        state.sessions[&alice.session_id].current_tab_id.as_deref(),
        Some("tab-bootstrap")
    );
}

#[test]
fn unresponsive_browser_ends_old_session_before_replacement_launch() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects::default();
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
        },
    );
    let first = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();

    let replacement = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            2_000,
        ))
        .unwrap();
    drop(manager);

    assert_eq!(replacement.disposition, SessionBrowserDisposition::Launched);
    assert_eq!(
        replacement.session_disposition,
        SessionRecordDisposition::Created
    );
    assert_ne!(replacement.session_id, first.session_id);
    assert_ne!(replacement.browser_id, first.browser_id);
    assert_eq!(effects.launches, ["profile-a", "profile-a"]);
    assert_eq!(effects.closes, [first.browser_id]);
    assert_eq!(state.sessions.len(), 1);
    assert!(state.sessions.contains_key(&replacement.session_id));
    assert_eq!(state.session_history.len(), 1);
    assert_eq!(
        state.session_history[0].reason,
        SessionEndReason::BrowserUnresponsive
    );
}

#[test]
fn closing_one_shared_session_preserves_browser_for_other_session() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
        },
    );
    let alice = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    let bob = manager
        .open(OpenBrowserSession::exact_profile("bob", "profile-a", 2_000))
        .unwrap();

    let closed = manager
        .close_session(&alice.session_id, SessionEndReason::ExplicitClose, 3_000)
        .unwrap();
    drop(manager);

    assert_eq!(
        closed.disposition,
        SessionCloseDisposition::BrowserPreserved
    );
    assert!(!state.sessions.contains_key(&alice.session_id));
    assert!(state.sessions.contains_key(&bob.session_id));
    assert_eq!(
        state.browsers[&bob.browser_id].active_session_ids,
        [bob.session_id]
    );
    assert_eq!(state.session_history.len(), 1);
    assert_eq!(state.session_history[0].id, alice.session_id);
    assert!(effects.closes.is_empty());
}

#[test]
fn closing_shared_session_closes_only_its_attributed_tabs() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "tab-alice".to_string(),
            target_id: "target-alice".to_string(),
            source: BrowserTabSource::Bootstrap,
        }),
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
        },
    );
    let alice = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    let bob = manager
        .open(OpenBrowserSession::exact_profile("bob", "profile-a", 2_000))
        .unwrap();
    manager
        .tab_for_navigation(&alice.session_id, 3_000)
        .unwrap();

    let closed = manager
        .close_session(&alice.session_id, SessionEndReason::ExplicitClose, 4_000)
        .unwrap();
    drop(manager);

    assert_eq!(
        closed.disposition,
        SessionCloseDisposition::BrowserPreserved
    );
    assert_eq!(
        effects.tab_closes,
        [(alice.browser_id.clone(), "target-alice".to_string())]
    );
    assert!(state.tabs.is_empty());
    assert!(state.sessions.contains_key(&bob.session_id));
    assert!(state.browsers.contains_key(&alice.browser_id));
}

#[test]
fn reaper_expires_idle_session_and_closes_sessionless_browser() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects::default();
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
        },
    );
    let alice = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();

    let reaped = manager.reap(301_000).unwrap();
    drop(manager);

    assert_eq!(reaped.expired_session_ids, [alice.session_id.clone()]);
    assert_eq!(reaped.closed_browser_ids, [alice.browser_id.clone()]);
    assert!(state.sessions.is_empty());
    assert!(state.browsers.is_empty());
    assert_eq!(state.session_history.len(), 1);
    assert_eq!(
        state.session_history[0].reason,
        SessionEndReason::HeartbeatExpired
    );
    assert_eq!(effects.closes, [alice.browser_id]);
}

fn catalog_with_named_profile() -> BrowserProfileCatalog {
    let profile = BrowserProfileCatalogEntry {
        id: "profile-a".to_string(),
        name: "Profile A".to_string(),
        user_data_dir: "/managed/profiles/profile-a".to_string(),
        kind: BrowserProfileKind::Named,
    };
    BrowserProfileCatalog {
        schema_version: "agent-browser.browser-profile-catalog.v1".to_string(),
        profiles: BTreeMap::from([(profile.id.clone(), profile)]),
    }
}

#[test]
fn first_named_session_launches_one_browser_for_exact_profile() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects::default();
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
        },
    );

    let opened = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();

    assert_eq!(opened.disposition, SessionBrowserDisposition::Launched);
    assert_eq!(opened.session_name, "alice");
    assert_eq!(opened.profile_id, "profile-a");
    assert_eq!(opened.browser_id, "browser:profile-a");
    assert_eq!(effects.launches, ["profile-a"]);
    assert_eq!(state.browsers.len(), 1);
    assert_eq!(state.sessions.len(), 1);
    assert_eq!(
        state.sessions[&opened.session_id].last_activity_at_ms,
        1_000
    );
}

#[test]
fn second_named_session_reuses_live_browser_for_exact_profile() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
        },
    );

    let alice = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    let bob = manager
        .open(OpenBrowserSession::exact_profile("bob", "profile-a", 2_000))
        .unwrap();
    drop(manager);

    assert_eq!(bob.disposition, SessionBrowserDisposition::Reused);
    assert_eq!(bob.browser_id, alice.browser_id);
    assert_eq!(effects.launches, ["profile-a"]);
    assert_eq!(effects.probes, [alice.browser_id]);
    assert_eq!(state.browsers.len(), 1);
    assert_eq!(state.sessions.len(), 2);
    assert_eq!(
        state.browsers[&bob.browser_id].active_session_ids,
        [alice.session_id, bob.session_id]
    );
}

#[test]
fn same_session_name_can_open_a_different_exact_profile() {
    let mut catalog = catalog_with_named_profile();
    let profile_b = BrowserProfileCatalogEntry {
        id: "profile-b".to_string(),
        name: "Profile B".to_string(),
        user_data_dir: "/managed/profiles/profile-b".to_string(),
        kind: BrowserProfileKind::Named,
    };
    catalog.profiles.insert(profile_b.id.clone(), profile_b);
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
        },
    );

    let profile_a = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    let profile_b = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-b",
            2_000,
        ))
        .unwrap();
    drop(manager);

    assert_ne!(profile_a.session_id, profile_b.session_id);
    assert_ne!(profile_a.browser_id, profile_b.browser_id);
    assert_eq!(effects.launches, ["profile-a", "profile-b"]);
    assert_eq!(state.sessions.len(), 2);
    assert_eq!(state.browsers.len(), 2);
}
