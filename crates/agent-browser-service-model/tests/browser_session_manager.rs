use std::collections::{BTreeMap, VecDeque};

use agent_browser_service_model::{
    BrowserDesktopAssignment, BrowserDisposableProfilePolicy, BrowserLaunch,
    BrowserOpenReservation, BrowserProfileCatalog, BrowserProfileCatalogEntry, BrowserProfileKind,
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
    initial_tabs: VecDeque<BrowserTabAcquisition>,
    initial_tab_requests: Vec<String>,
    initial_tab_attributed_targets: Vec<Vec<String>>,
    new_tab: Option<BrowserTabAcquisition>,
    new_tab_requests: Vec<String>,
    tab_closes: Vec<(String, String)>,
    disposable_allocations: Vec<String>,
    disposable_deletions: Vec<String>,
    navigations: Vec<(String, String, String)>,
    focuses: Vec<(String, Option<String>)>,
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
            remote_desktop_routes: Vec::new(),
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
            remote_desktop_routes: Vec::new(),
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

#[test]
fn reserved_open_uses_exact_session_browser_and_desktop_identity() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects::default();
    let desktop = BrowserDesktopAssignment {
        route_id: "route-a".to_string(),
        display_name: ":21".to_string(),
        live_browser_count: 0,
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: vec![agent_browser_service_model::BrowserDesktopRoute {
                id: desktop.route_id.clone(),
                display_name: desktop.display_name.clone(),
                healthy: true,
            }],
        },
    );

    let opened = manager
        .open_reserved(
            OpenBrowserSession::exact_profile("alice", "profile-a", 1_000),
            BrowserOpenReservation {
                session_id: "session:alice:profile-a:1".to_string(),
                browser_id: "browser:profile-a".to_string(),
                desktop: desktop.clone(),
            },
        )
        .unwrap();
    drop(manager);

    assert_eq!(opened.session_id, "session:alice:profile-a:1");
    assert_eq!(opened.browser_id, "browser:profile-a");
    assert_eq!(state.browsers[&opened.browser_id].desktop, Some(desktop));
    assert_eq!(effects.launches, ["profile-a"]);
}

#[test]
fn invalid_reserved_session_identity_is_rejected_before_launch() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects::default();
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: vec![agent_browser_service_model::BrowserDesktopRoute {
                id: "route-a".to_string(),
                display_name: ":21".to_string(),
                healthy: true,
            }],
        },
    );

    let error = manager
        .open_reserved(
            OpenBrowserSession::exact_profile("alice", "profile-a", 1_000),
            BrowserOpenReservation {
                session_id: "session:wrong".to_string(),
                browser_id: "browser:profile-a".to_string(),
                desktop: BrowserDesktopAssignment {
                    route_id: "route-a".to_string(),
                    display_name: ":21".to_string(),
                    live_browser_count: 0,
                },
            },
        )
        .unwrap_err();
    drop(manager);

    assert_eq!(error, "browser_session_reserved_session_identity_mismatch");
    assert!(effects.launches.is_empty());
    assert!(state.sessions.is_empty());
    assert!(state.browsers.is_empty());
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
        desktop: Option<&agent_browser_service_model::BrowserDesktopAssignment>,
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
            process_identity: None,
            desktop: desktop.cloned(),
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
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        self.initial_tab_requests.push(browser.id.clone());
        self.initial_tab_attributed_targets
            .push(attributed_target_ids.to_vec());
        self.initial_tabs
            .pop_front()
            .or_else(|| self.initial_tab.clone())
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

    fn navigate(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
        tab: &agent_browser_service_model::ManagedBrowserTab,
        url: &str,
    ) -> Result<(), String> {
        self.navigations
            .push((browser.id.clone(), tab.target_id.clone(), url.to_string()));
        Ok(())
    }

    fn focus_browser(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
        tab: Option<&agent_browser_service_model::ManagedBrowserTab>,
    ) -> Result<(), String> {
        self.focuses
            .push((browser.id.clone(), tab.map(|tab| tab.target_id.clone())));
        Ok(())
    }

    fn allocate_disposable_profile(
        &mut self,
        policy: &BrowserDisposableProfilePolicy,
        allocation_id: &str,
        _session_name: &str,
    ) -> Result<BrowserProfileCatalogEntry, String> {
        self.disposable_allocations.push(allocation_id.to_string());
        Ok(BrowserProfileCatalogEntry {
            id: allocation_id.to_string(),
            name: allocation_id.to_string(),
            user_data_dir: format!("{}/{}", policy.user_data_root, allocation_id),
            kind: BrowserProfileKind::Disposable,
        })
    }

    fn delete_disposable_profile(
        &mut self,
        allocation: &agent_browser_service_model::ManagedDisposableProfile,
    ) -> Result<(), String> {
        self.disposable_deletions
            .push(allocation.profile.id.clone());
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
            remote_desktop_routes: Vec::new(),
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
            remote_desktop_routes: Vec::new(),
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
fn dashboard_focus_selects_attributed_target_and_refreshes_its_session() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
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
            remote_desktop_routes: Vec::new(),
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

    let focused = manager
        .focus_browser(&alice.browser_id, Some("target-bootstrap"), 3_000)
        .unwrap();
    drop(manager);

    assert_eq!(focused.browser_id, alice.browser_id);
    assert_eq!(focused.tab_id.as_deref(), Some("tab-bootstrap"));
    assert_eq!(focused.target_id.as_deref(), Some("target-bootstrap"));
    assert_eq!(
        effects.focuses,
        [(
            "browser:profile-a".to_string(),
            Some("target-bootstrap".to_string())
        )]
    );
    assert_eq!(state.sessions[&alice.session_id].last_activity_at_ms, 3_000);
    assert_eq!(state.sessions[&alice.session_id].expires_at_ms, 303_000);
}

#[test]
fn second_session_gets_one_initial_tab_when_bootstrap_is_already_attributed() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        initial_tabs: VecDeque::from([
            BrowserTabAcquisition {
                tab_id: "tab-alice".to_string(),
                target_id: "target-alice".to_string(),
                source: BrowserTabSource::Bootstrap,
            },
            BrowserTabAcquisition {
                tab_id: "tab-bob".to_string(),
                target_id: "target-bob".to_string(),
                source: BrowserTabSource::SessionInitial,
            },
        ]),
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: Vec::new(),
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
    let bob_tab = manager.tab_for_navigation(&bob.session_id, 4_000).unwrap();
    drop(manager);

    assert_eq!(bob_tab.source, BrowserTabSource::SessionInitial);
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(
        effects.initial_tab_attributed_targets[0],
        Vec::<String>::new()
    );
    assert_eq!(
        effects.initial_tab_attributed_targets[1],
        ["target-alice".to_string()]
    );
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
            remote_desktop_routes: Vec::new(),
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
            remote_desktop_routes: Vec::new(),
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
            remote_desktop_routes: Vec::new(),
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
            remote_desktop_routes: Vec::new(),
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
        initial_tabs: VecDeque::from([
            BrowserTabAcquisition {
                tab_id: "tab-alice".to_string(),
                target_id: "target-alice".to_string(),
                source: BrowserTabSource::Bootstrap,
            },
            BrowserTabAcquisition {
                tab_id: "tab-bob".to_string(),
                target_id: "target-bob".to_string(),
                source: BrowserTabSource::SessionInitial,
            },
        ]),
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: Vec::new(),
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
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs["tab-bob"].session_id, bob.session_id);
    assert_eq!(state.tab_history.len(), 1);
    assert_eq!(state.tab_history[0].session_id, alice.session_id);
    assert_eq!(state.tab_history[0].profile_id, "profile-a");
    assert_eq!(state.tab_history[0].target_id, "target-alice");
    assert_eq!(state.tab_history[0].closed_at_ms, 4_000);
    assert!(state.sessions.contains_key(&bob.session_id));
    assert!(state.browsers.contains_key(&alice.browser_id));
}

#[test]
fn navigation_history_remains_queryable_after_session_close() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
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
            remote_desktop_routes: Vec::new(),
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
    manager
        .navigate(&alice.session_id, "https://example.test/path", 3_000)
        .unwrap();
    manager
        .close_session(&alice.session_id, SessionEndReason::ExplicitClose, 4_000)
        .unwrap();
    drop(manager);

    assert_eq!(state.navigation_history.len(), 1);
    let navigation = &state.navigation_history[0];
    assert_eq!(navigation.profile_id, "profile-a");
    assert_eq!(navigation.session_id, alice.session_id);
    assert_eq!(navigation.tab_id, "tab-alice");
    assert_eq!(navigation.url, "https://example.test/path");
    assert_eq!(navigation.visited_at_ms, 3_000);
    assert_eq!(
        effects.navigations,
        [(
            alice.browser_id,
            "target-alice".to_string(),
            "https://example.test/path".to_string()
        )]
    );
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
            remote_desktop_routes: Vec::new(),
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
        disposable_policies: BTreeMap::new(),
    }
}

#[test]
fn distinct_profile_browsers_use_least_crowded_configured_desktops() {
    let mut catalog = catalog_with_named_profile();
    catalog.profiles.insert(
        "profile-b".to_string(),
        BrowserProfileCatalogEntry {
            id: "profile-b".to_string(),
            name: "Profile B".to_string(),
            user_data_dir: "/managed/profiles/profile-b".to_string(),
            kind: BrowserProfileKind::Named,
        },
    );
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects::default();
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: vec![
                agent_browser_service_model::BrowserDesktopRoute {
                    id: "guac-a".to_string(),
                    display_name: ":10".to_string(),
                    healthy: true,
                },
                agent_browser_service_model::BrowserDesktopRoute {
                    id: "guac-b".to_string(),
                    display_name: ":11".to_string(),
                    healthy: true,
                },
            ],
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
        .open(OpenBrowserSession::exact_profile("bob", "profile-b", 2_000))
        .unwrap();
    drop(manager);

    assert_eq!(
        state.browsers[&alice.browser_id]
            .desktop
            .as_ref()
            .map(|desktop| (desktop.route_id.as_str(), desktop.display_name.as_str())),
        Some(("guac-a", ":10"))
    );
    assert_eq!(
        state.browsers[&bob.browser_id]
            .desktop
            .as_ref()
            .map(|desktop| (desktop.route_id.as_str(), desktop.display_name.as_str())),
        Some(("guac-b", ":11"))
    );
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
            remote_desktop_routes: Vec::new(),
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
            remote_desktop_routes: Vec::new(),
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
            remote_desktop_routes: Vec::new(),
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

#[test]
fn disposable_profiles_are_session_scoped_reused_and_reaped() {
    let catalog = catalog_with_disposable_policy();
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
            remote_desktop_routes: Vec::new(),
        },
    );

    let alice = manager
        .open(OpenBrowserSession::disposable("alice", "default", 1_000))
        .unwrap();
    let alice_again = manager
        .open(OpenBrowserSession::disposable("alice", "default", 2_000))
        .unwrap();
    let bob = manager
        .open(OpenBrowserSession::disposable("bob", "default", 3_000))
        .unwrap();

    assert_eq!(alice_again.session_id, alice.session_id);
    assert_eq!(alice_again.browser_id, alice.browser_id);
    assert_ne!(bob.profile_id, alice.profile_id);
    assert_ne!(bob.browser_id, alice.browser_id);
    manager
        .close_session(&alice.session_id, SessionEndReason::ExplicitClose, 4_000)
        .unwrap();
    let reaped = manager.reap(4_000).unwrap();
    drop(manager);

    assert_eq!(effects.disposable_allocations.len(), 2);
    assert_eq!(effects.disposable_deletions, [alice.profile_id.clone()]);
    assert_eq!(reaped.deleted_disposable_profile_ids, [alice.profile_id]);
    assert!(state.sessions.contains_key(&bob.session_id));
    assert!(state.disposable_profiles.contains_key(&bob.profile_id));
}

#[test]
fn exact_intent_rejects_disposable_profile_definition() {
    let profile = BrowserProfileCatalogEntry {
        id: "legacy-one-time".to_string(),
        name: "Legacy One Time".to_string(),
        user_data_dir: "/managed/legacy-one-time".to_string(),
        kind: BrowserProfileKind::Disposable,
    };
    let catalog = BrowserProfileCatalog {
        schema_version: "agent-browser.browser-profile-catalog.v1".to_string(),
        profiles: BTreeMap::from([(profile.id.clone(), profile)]),
        disposable_policies: BTreeMap::new(),
    };
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects::default();
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: Vec::new(),
        },
    );

    let error = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "legacy-one-time",
            1_000,
        ))
        .unwrap_err();

    assert_eq!(
        error,
        "browser_profile_requires_disposable_intent:legacy-one-time"
    );
    assert!(state.sessions.is_empty());
    assert!(effects.launches.is_empty());
}

fn catalog_with_disposable_policy() -> BrowserProfileCatalog {
    BrowserProfileCatalog {
        schema_version: "agent-browser.browser-profile-catalog.v1".to_string(),
        profiles: BTreeMap::new(),
        disposable_policies: BTreeMap::from([(
            "default".to_string(),
            BrowserDisposableProfilePolicy {
                id: "default".to_string(),
                user_data_root: "/managed/disposable".to_string(),
                cleanup_delay_ms: 0,
            },
        )]),
    }
}

#[test]
fn serialized_state_reuses_healthy_session_after_service_restart() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let first = {
        let mut effects = FixtureEffects::default();
        let mut manager = BrowserSessionManager::new(
            &mut state,
            &catalog,
            &mut effects,
            BrowserSessionManagerConfig {
                session_idle_timeout_ms: 300_000,
                remote_desktop_routes: Vec::new(),
            },
        );
        manager
            .open(OpenBrowserSession::exact_profile(
                "alice",
                "profile-a",
                1_000,
            ))
            .unwrap()
    };
    let encoded = serde_json::to_string(&state).unwrap();
    let mut restarted_state: BrowserSessionState = serde_json::from_str(&encoded).unwrap();
    let mut restarted_effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let mut restarted_manager = BrowserSessionManager::new(
        &mut restarted_state,
        &catalog,
        &mut restarted_effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: Vec::new(),
        },
    );

    let resumed = restarted_manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            2_000,
        ))
        .unwrap();

    assert_eq!(resumed.session_id, first.session_id);
    assert_eq!(resumed.browser_id, first.browser_id);
    assert_eq!(
        resumed.session_disposition,
        SessionRecordDisposition::Reused
    );
    assert!(restarted_effects.launches.is_empty());
}
