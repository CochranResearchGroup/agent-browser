use std::collections::{BTreeMap, BTreeSet, VecDeque};

use agent_browser_service_model::{
    BrowserDisposableProfilePolicy, BrowserLaunch, BrowserNavigationRecord, BrowserProfileCatalog,
    BrowserProfileCatalogEntry, BrowserProfileKind, BrowserSessionEffects, BrowserSessionManager,
    BrowserSessionManagerConfig, BrowserSessionState, BrowserTabAcquisition, BrowserTabSource,
    OpenBrowserSession, SessionBrowserDisposition, SessionCloseDisposition, SessionEndReason,
    SessionRecordDisposition,
};

#[test]
fn exact_navigation_history_compacts_oldest_rows_into_restart_safe_daily_summaries() {
    fn navigation(url: &str, visited_at_ms: u64) -> BrowserNavigationRecord {
        BrowserNavigationRecord {
            profile_id: "profile-a".to_string(),
            session_id: "session-a".to_string(),
            browser_id: "browser-a".to_string(),
            tab_id: "tab-a".to_string(),
            target_id: "target-a".to_string(),
            url: url.to_string(),
            visited_at_ms,
            incident_ids: Vec::new(),
        }
    }

    let mut state = BrowserSessionState::default();
    state.navigation_history = vec![
        navigation("https://example.test/first", 1_758_758_400_000),
        navigation("https://example.test/second", 1_758_758_401_000),
        navigation("https://example.test/third", 1_758_844_800_000),
    ];
    state.navigation_history[0].incident_ids = vec!["incident-a".to_string()];
    let maximum = serde_json::to_vec(&state.navigation_history[1..])
        .unwrap()
        .len() as u64;

    let event = state.compact_navigation_history(maximum).unwrap().unwrap();
    assert_eq!(event.removed_navigation_count, 1);
    assert!(event.exact_bytes_after <= maximum);
    assert_eq!(state.navigation_history.len(), 2);
    assert_eq!(
        state.navigation_history[0].url,
        "https://example.test/second"
    );
    assert_eq!(state.navigation_daily_summaries.len(), 1);
    let summary = &state.navigation_daily_summaries[0];
    assert_eq!(summary.utc_day, "2025-09-25");
    assert_eq!(summary.first_url, "https://example.test/first");
    assert_eq!(summary.last_url, "https://example.test/first");
    assert_eq!(summary.navigation_count, 1);
    assert_eq!(summary.incident_ids, ["incident-a"]);

    let encoded = serde_json::to_vec(&state).unwrap();
    let mut restarted: BrowserSessionState = serde_json::from_slice(&encoded).unwrap();
    assert!(restarted
        .compact_navigation_history(maximum)
        .unwrap()
        .is_none());
    assert_eq!(restarted, state);
    restarted
        .navigation_history
        .push(navigation("https://example.test/fourth", 1_758_844_801_000));
    restarted
        .compact_navigation_history(maximum)
        .unwrap()
        .unwrap();
    assert_eq!(restarted.navigation_daily_summaries[0].navigation_count, 2);
    assert_eq!(
        restarted.navigation_daily_summaries[0].last_url,
        "https://example.test/second"
    );
    assert_eq!(restarted.history_compaction_events.len(), 2);
}

#[derive(Default)]
struct FixtureEffects {
    qualified_adoption: Option<agent_browser_service_model::BrowserRecovery>,
    idle_retirement_permitted: Option<bool>,
    idle_retirement_error_browser: Option<String>,
    retirement_guard_denied_browser: Option<String>,
    profile_cleanup_permitted: Option<bool>,
    profile_cleanup_denied_profile: Option<String>,
    disposable_delete_error: Option<String>,
    retirement_guard_active: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    launches: Vec<String>,
    manual_launch_urls: Vec<Option<String>>,
    executable: String,
    profile_executables: BTreeMap<String, String>,
    launch_executables: Vec<(String, String)>,
    probes: Vec<String>,
    browser_live: bool,
    closes: Vec<String>,
    close_error: Option<String>,
    initial_tab: Option<BrowserTabAcquisition>,
    initial_tabs: VecDeque<BrowserTabAcquisition>,
    initial_tab_requests: Vec<String>,
    initial_tab_attributed_targets: Vec<Vec<String>>,
    new_tab: Option<BrowserTabAcquisition>,
    new_tab_requests: Vec<String>,
    tab_closes: Vec<(String, String)>,
    tab_close_failure_target: Option<String>,
    disposable_allocations: Vec<String>,
    disposable_deletions: Vec<String>,
    disposable_sizes: BTreeMap<String, u64>,
    navigations: Vec<(String, String, String)>,
    focuses: Vec<(String, Option<String>)>,
}

#[test]
fn observed_popup_custody_preserves_peers_and_rejects_overflow_without_partial_claims() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        initial_tabs: VecDeque::from(["alice-root", "bob-root"].map(|target| {
            BrowserTabAcquisition {
                tab_id: target.into(),
                target_id: target.into(),
                source: BrowserTabSource::SessionInitial,
            }
        })),
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 300_000,
        remote_view_desktops: Vec::new(),
    };
    let mut manager =
        BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone());
    let alice = manager
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1000,
        ))
        .unwrap();
    manager.tab_for_navigation(&alice.session_id, 1000).unwrap();
    let bob = manager
        .open(OpenBrowserSession::exact_profile("bob", "profile-a", 1000))
        .unwrap();
    manager.tab_for_navigation(&bob.session_id, 1000).unwrap();
    let observations = [
        ("destination", "popup"),
        ("popup", "alice-root"),
        ("peer-child", "bob-root"),
        ("bob-root", "alice-root"),
        ("unowned", "unknown"),
    ]
    .map(|(target, opener)| (target.into(), opener.into()));
    let admitted = manager
        .observe_child_tabs(&alice.session_id, &observations, 1100)
        .unwrap();
    assert_eq!(admitted.len(), 2);
    assert!(manager
        .observe_child_tabs(&alice.session_id, &observations, 1200)
        .unwrap()
        .is_empty());
    drop(manager);
    assert_eq!(
        state.sessions[&alice.session_id].current_tab_id.as_deref(),
        Some("alice-root")
    );
    assert_eq!(state.tabs["bob-root"].session_id, bob.session_id);
    assert!(!state.tabs.contains_key("peer-child"));
    assert!(!state.tabs.contains_key("unowned"));
    let before = state.clone();
    let oversized = (0..65)
        .map(|index| (format!("overflow-{index}"), "alice-root".into()))
        .collect::<Vec<_>>();
    let mut manager = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config);
    assert_eq!(
        manager
            .observe_child_tabs(&alice.session_id, &oversized, 1300)
            .unwrap_err(),
        "browser_session_popup_limit_exceeded"
    );
    drop(manager);
    assert_eq!(
        state, before,
        "Rejected observations publish no partial custody"
    );
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
            remote_view_desktops: Vec::new(),
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
            remote_view_desktops: Vec::new(),
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
    fn permits_idle_browser_close(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> Result<bool, String> {
        if self.idle_retirement_error_browser.as_deref() == Some(browser.id.as_str()) {
            return Err("synthetic_owner_observation_unavailable".into());
        }
        Ok(self.idle_retirement_permitted.unwrap_or(true))
    }
    fn acquire_idle_retirement_guard(
        &mut self,
        browsers: &[agent_browser_service_model::ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn agent_browser_service_model::BrowserIdleRetirementGuard>>, String>
    {
        for browser in browsers {
            if self.retirement_guard_denied_browser.as_deref() == Some(browser.id.as_str())
                || !self.permits_idle_browser_close(browser)?
            {
                return Ok(None);
            }
        }
        struct Guard(std::sync::Arc<std::sync::atomic::AtomicBool>);
        impl agent_browser_service_model::BrowserIdleRetirementGuard for Guard {}
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }
        if let Some(active) = &self.retirement_guard_active {
            active.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(Some(Box::new(Guard(active.clone()))))
        } else {
            Ok(Some(Box::new(())))
        }
    }
    fn acquire_profile_cleanup_guard(
        &mut self,
        allocation: &agent_browser_service_model::ManagedDisposableProfile,
    ) -> Result<Option<Box<dyn agent_browser_service_model::BrowserIdleRetirementGuard>>, String>
    {
        if !self.profile_cleanup_permitted.unwrap_or(true)
            || self.profile_cleanup_denied_profile.as_deref()
                == Some(allocation.profile.id.as_str())
        {
            return Ok(None);
        }
        self.acquire_idle_retirement_guard(&[])
    }
    fn select_profile_browser_executable(
        &mut self,
        profile_id: String,
        path: String,
    ) -> Result<(), String> {
        self.profile_executables.insert(profile_id, path);
        Ok(())
    }
    fn select_browser_executable(&mut self, path: String) -> Result<(), String> {
        self.executable = path;
        Ok(())
    }
    fn recover_browser(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        tabs: &[agent_browser_service_model::ManagedBrowserTab],
        _navigation: &[BrowserNavigationRecord],
    ) -> Result<agent_browser_service_model::BrowserRecovery, String> {
        self.launches.push(profile.id.clone());
        self.launch_executables.push((
            profile.id.clone(),
            self.profile_executables
                .get(&profile.id)
                .unwrap_or(&self.executable)
                .clone(),
        ));
        self.browser_live = true;
        Ok(agent_browser_service_model::BrowserRecovery {
            launch: BrowserLaunch {
                process_identity: None,
                browser_id: browser.id.clone(),
                pid: 5252,
                cdp_endpoint: "http://127.0.0.1:9522".into(),
                desktop: browser.desktop.clone(),
            },
            target_ids: tabs
                .iter()
                .map(|tab| (tab.id.clone(), format!("recovered-{}", tab.target_id)))
                .collect(),
        })
    }
    fn reconcile_profile_browser(
        &mut self,
        _profile: &BrowserProfileCatalogEntry,
        _browser: Option<&agent_browser_service_model::ManagedBrowserInstance>,
        _tabs: &[agent_browser_service_model::ManagedBrowserTab],
        _navigation: &[BrowserNavigationRecord],
    ) -> Result<Option<agent_browser_service_model::BrowserRecovery>, String> {
        Ok(self.qualified_adoption.take())
    }
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
        desktop: Option<&agent_browser_service_model::RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String> {
        self.launches.push(profile.id.clone());
        self.launch_executables.push((
            profile.id.clone(),
            self.profile_executables
                .get(&profile.id)
                .unwrap_or(&self.executable)
                .clone(),
        ));
        self.manual_launch_urls
            .push(profile.manual_login_url.clone());
        let sequence = self.launches.len();
        Ok(BrowserLaunch {
            process_identity: None,
            browser_id: if sequence == 1 {
                format!("browser:{}", profile.id)
            } else {
                format!("browser:{}:{sequence}", profile.id)
            },
            pid: 4242,
            cdp_endpoint: "http://127.0.0.1:9422".to_string(),
            desktop: desktop.cloned(),
        })
    }

    fn close_browser(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> Result<(), String> {
        if let Some(error) = &self.close_error {
            return Err(error.clone());
        }
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
        if self.tab_close_failure_target.as_ref() == Some(&tab.target_id) {
            return Err("synthetic_tab_close_unknown".into());
        }
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
            manual_login_url: None,
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
        if let Some(active) = &self.retirement_guard_active {
            assert!(
                active.load(std::sync::atomic::Ordering::SeqCst),
                "profile deletion must retain retirement admission exclusion"
            );
        }
        if let Some(error) = &self.disposable_delete_error {
            return Err(error.clone());
        }
        self.disposable_deletions
            .push(allocation.profile.id.clone());
        Ok(())
    }

    fn disposable_profile_size_bytes(
        &mut self,
        allocation: &agent_browser_service_model::ManagedDisposableProfile,
    ) -> Result<u64, String> {
        Ok(self
            .disposable_sizes
            .get(&allocation.profile.id)
            .copied()
            .unwrap_or(0))
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
            remote_view_desktops: Vec::new(),
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
            remote_view_desktops: Vec::new(),
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
            remote_view_desktops: Vec::new(),
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
            remote_view_desktops: Vec::new(),
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
            remote_view_desktops: Vec::new(),
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
fn explicit_close_uses_the_named_tab_after_current_selection_changes() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "tab-original".into(),
            target_id: "target-original".into(),
            source: BrowserTabSource::Bootstrap,
        }),
        new_tab: Some(BrowserTabAcquisition {
            tab_id: "tab-peer".into(),
            target_id: "target-peer".into(),
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
            remote_view_desktops: vec![],
        },
    );
    let opened = manager
        .open(OpenBrowserSession::exact_profile("alice", "profile-a", 100))
        .unwrap();
    manager.tab_for_navigation(&opened.session_id, 110).unwrap();
    manager.new_tab(&opened.session_id, 120).unwrap();
    let denied = manager.close_exact_tab_with(
        &opened.session_id,
        "tab-original",
        130,
        |_effects, _browser, _tab| Err("synthetic close permission revoked".into()),
    );
    assert_eq!(denied.unwrap_err(), "synthetic close permission revoked");
    let closed = manager
        .close_exact_tab_with(
            &opened.session_id,
            "tab-original",
            140,
            |effects, browser, tab| effects.close_tab(browser, tab),
        )
        .unwrap();
    assert_eq!(closed.closed_tab_id, "tab-original");
    assert_eq!(closed.current_tab_id.as_deref(), Some("tab-peer"));
    drop(manager);
    assert_eq!(
        effects.tab_closes,
        [(opened.browser_id.clone(), "target-original".into())]
    );
    assert!(effects.closes.is_empty());
    assert!(state.tabs.contains_key("tab-peer"));
    assert!(!state.tabs.contains_key("tab-original"));
}

#[test]
fn exact_final_tab_close_preserves_borrowed_browser_custody() {
    for borrowed in [false, true] {
        let catalog = catalog_with_named_profile();
        let mut state = BrowserSessionState::default();
        let mut effects = FixtureEffects {
            initial_tab: Some(BrowserTabAcquisition {
                tab_id: "tab-only".into(),
                target_id: "target-only".into(),
                source: BrowserTabSource::Bootstrap,
            }),
            ..FixtureEffects::default()
        };
        let config = BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: vec![],
        };
        let mut manager =
            BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone());
        let opened = manager
            .open(OpenBrowserSession::exact_profile("alice", "profile-a", 100))
            .unwrap();
        manager.tab_for_navigation(&opened.session_id, 110).unwrap();
        drop(manager);
        // The existing adoption owner publishes this custody marker. Closing
        // an attributed tab never transfers ownership of its borrowed browser.
        if borrowed {
            state.shared_browser_ids.insert(opened.browser_id.clone());
        }
        let mut manager = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config);
        let closed = manager
            .close_exact_tab_with(
                &opened.session_id,
                "tab-only",
                120,
                |effects, browser, tab| effects.close_tab(browser, tab),
            )
            .unwrap();
        assert_eq!(closed.closed_tab_id, "tab-only");
        drop(manager);
        assert_eq!(
            effects.tab_closes,
            [(opened.browser_id.clone(), "target-only".into())]
        );
        assert!(state.tabs.is_empty());
        assert_eq!(state.browsers.contains_key(&opened.browser_id), borrowed);
        assert_eq!(
            effects.closes,
            if borrowed {
                vec![]
            } else {
                vec![opened.browser_id]
            }
        );
    }
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
            remote_view_desktops: Vec::new(),
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
            remote_view_desktops: Vec::new(),
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
            remote_view_desktops: Vec::new(),
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
fn closing_final_session_closes_browser_and_clears_active_state() {
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
            remote_view_desktops: Vec::new(),
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

    let closed = manager
        .close_session(&alice.session_id, SessionEndReason::ExplicitClose, 3_000)
        .unwrap();
    drop(manager);

    assert_eq!(closed.disposition, SessionCloseDisposition::BrowserClosed);
    assert_eq!(effects.closes, [alice.browser_id.clone()]);
    assert!(effects.tab_closes.is_empty());
    assert!(state.sessions.is_empty());
    assert!(state.browsers.is_empty());
    assert!(state.tabs.is_empty());
    assert_eq!(state.session_history.len(), 1);
    assert_eq!(state.session_history[0].id, alice.session_id);
    assert_eq!(state.tab_history.len(), 1);
    assert_eq!(state.tab_history[0].browser_id, alice.browser_id);
    assert_eq!(state.tab_history[0].target_id, "target-alice");
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
            remote_view_desktops: Vec::new(),
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
            remote_view_desktops: Vec::new(),
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
            remote_view_desktops: Vec::new(),
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

#[test]
fn retained_handoff_survives_browser_idle_close_and_reconnect_until_explicit_close() {
    use agent_browser_service_model::{
        resolve_remote_view_tab_handoff, retain_remote_view_tab_handoff,
    };
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "operator-tab".into(),
            target_id: "operator-target".into(),
            source: BrowserTabSource::SessionInitial,
        }),
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 1_000,
        remote_view_desktops: Vec::new(),
    };
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .tab_for_navigation(&opened.session_id, 1_000)
        .unwrap();
    retain_remote_view_tab_handoff(
        &mut state,
        "operator-handoff",
        &opened.session_id,
        "operator-tab",
    )
    .unwrap();
    // The same persisted aggregate and manager seam are used after host restart.
    state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    let retained = state.clone();
    let reaped = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .with_protected_sessions(BTreeSet::new())
        .reap(20_000)
        .unwrap();
    assert!(
        reaped.expired_session_ids.is_empty(),
        "retained operator session was reaped"
    );
    assert_eq!(
        resolve_remote_view_tab_handoff(&state, "operator-handoff")
            .unwrap()
            .tab
            .id,
        "operator-tab"
    );
    effects.browser_live = false;
    let resumed = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            30_000,
        ))
        .unwrap();
    assert_eq!(resumed.session_id, opened.session_id);
    assert_eq!(resumed.browser_id, opened.browser_id);
    assert_eq!(effects.launches.len(), 2);
    assert_eq!(effects.closes, [opened.browser_id.clone()]);
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .close_session(&opened.session_id, SessionEndReason::ExplicitClose, 31_000)
        .unwrap();
    assert!(state.browsers.is_empty());
    assert!(state.sessions.is_empty());
    assert!(resolve_remote_view_tab_handoff(&state, "operator-handoff").is_err());
    assert_eq!(
        effects.closes,
        [opened.browser_id.clone(), opened.browser_id]
    );
    // A dangling historical handoff cannot pin a live browser indefinitely.
    state = retained;
    state
        .remote_view_tab_handoffs
        .get_mut("operator-handoff")
        .unwrap()
        .tab_id = "missing-tab".into();
    let reaped = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .reap(20_000)
        .unwrap();
    assert_eq!(reaped.expired_session_ids, [opened.session_id]);
    assert!(state.sessions.is_empty());
}

fn catalog_with_named_profile() -> BrowserProfileCatalog {
    let profile = BrowserProfileCatalogEntry {
        manual_login_url: None,
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
fn uncertain_browser_observation_does_not_starve_another_tabs_lifetime() {
    use agent_browser_service_model::{retain_remote_view_tab_handoff, BrowserTabLifetime};
    let mut catalog = catalog_with_named_profile();
    let mut peer = catalog.profiles["profile-a"].clone();
    peer.id = "profile-b".into();
    peer.user_data_dir = "/managed/profiles/profile-b".into();
    catalog.profiles.insert(peer.id.clone(), peer);
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 1_000,
        remote_view_desktops: Vec::new(),
    };
    let mut created = Vec::new();
    for (profile, task, tab) in [
        ("profile-a", "uncertain", "tab-a"),
        ("profile-b", "expired", "tab-b"),
    ] {
        effects.initial_tab = Some(BrowserTabAcquisition {
            tab_id: tab.into(),
            target_id: format!("target-{tab}"),
            source: BrowserTabSource::SessionInitial,
        });
        let mut manager =
            BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone());
        let opened = manager
            .open(OpenBrowserSession::exact_profile(task, profile, 100))
            .unwrap();
        manager.tab_for_navigation(&opened.session_id, 100).unwrap();
        drop(manager);
        retain_remote_view_tab_handoff(&mut state, task, &opened.session_id, tab).unwrap();
        created.push(opened);
    }
    effects.idle_retirement_error_browser = Some(created[0].browser_id.clone());
    state
        .shared_browser_ids
        .insert(created[1].browser_id.clone());
    state.tab_lifetimes.insert(
        "tab-b".into(),
        BrowserTabLifetime {
            inactivity_ms: Some(30),
            pressure_inactivity_ms: None,
        },
    );

    let observed =
        BrowserSessionManager::new(&mut state, &catalog, &mut effects, config).reap(2_000);

    assert!(
        !state.tabs.contains_key("tab-b"),
        "unrelated expiry was starved: {observed:?}"
    );
    assert_eq!(
        observed.unwrap().deferred_browser_errors,
        BTreeMap::from([(
            created[0].browser_id.clone(),
            "synthetic_owner_observation_unavailable".into()
        )])
    );
    assert!(
        state.tabs.contains_key("tab-a"),
        "uncertain owner must remain protected"
    );
    assert_eq!(
        effects.tab_closes,
        [(created[1].browser_id.clone(), "target-tab-b".into())]
    );
    assert!(
        effects.closes.is_empty(),
        "shared or uncertain browser must remain open"
    );
}

#[test]
fn distinct_profile_browsers_use_least_crowded_configured_desktops() {
    let mut catalog = catalog_with_named_profile();
    catalog.profiles.insert(
        "profile-b".to_string(),
        BrowserProfileCatalogEntry {
            manual_login_url: None,
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
            remote_view_desktops: vec![
                agent_browser_service_model::RemoteViewDesktopCandidate {
                    desktop: agent_browser_service_model::RemoteViewFixedDesktop {
                        desktop_id: "11111111-1111-1111-1111-111111111111".to_string(),
                        friendly_route_label: "desktop-a".to_string(),
                        generation: 1,
                    },
                    ready: true,
                },
                agent_browser_service_model::RemoteViewDesktopCandidate {
                    desktop: agent_browser_service_model::RemoteViewFixedDesktop {
                        desktop_id: "22222222-2222-2222-2222-222222222222".to_string(),
                        friendly_route_label: "desktop-b".to_string(),
                        generation: 1,
                    },
                    ready: true,
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
            .map(|desktop| (
                desktop.desktop_id.as_str(),
                desktop.friendly_route_label.as_str()
            )),
        Some(("11111111-1111-1111-1111-111111111111", "desktop-a"))
    );
    assert_eq!(
        state.browsers[&bob.browser_id]
            .desktop
            .as_ref()
            .map(|desktop| (
                desktop.desktop_id.as_str(),
                desktop.friendly_route_label.as_str()
            )),
        Some(("22222222-2222-2222-2222-222222222222", "desktop-b"))
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
            remote_view_desktops: Vec::new(),
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
            remote_view_desktops: Vec::new(),
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
        manual_login_url: None,
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
            remote_view_desktops: Vec::new(),
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
            remote_view_desktops: Vec::new(),
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
fn disposable_count_quota_evicts_oldest_unprotected_session_before_admission() {
    let mut catalog = catalog_with_disposable_policy();
    catalog
        .disposable_policies
        .get_mut("default")
        .unwrap()
        .maximum_retained_profiles = 2;
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig::default(),
    );
    let alice = manager
        .open(OpenBrowserSession::disposable("alice", "default", 1_000))
        .unwrap();
    let bob = manager
        .open(OpenBrowserSession::disposable("bob", "default", 2_000))
        .unwrap();
    let carol = manager
        .open(OpenBrowserSession::disposable("carol", "default", 3_000))
        .unwrap();
    drop(manager);

    assert!(!state.sessions.contains_key(&alice.session_id));
    assert!(state.sessions.contains_key(&bob.session_id));
    assert!(state.sessions.contains_key(&carol.session_id));
    assert_eq!(effects.disposable_deletions, [alice.profile_id.clone()]);
    assert_eq!(
        state.session_history.last().unwrap().reason,
        SessionEndReason::QuotaEvicted
    );
}

#[test]
fn disposable_quota_fails_closed_when_only_candidate_is_protected() {
    let mut catalog = catalog_with_disposable_policy();
    catalog
        .disposable_policies
        .get_mut("default")
        .unwrap()
        .maximum_retained_profiles = 1;
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let first = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig::default(),
    )
    .open(OpenBrowserSession::disposable("alice", "default", 1_000))
    .unwrap();
    let error = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig::default(),
    )
    .with_protected_sessions(BTreeSet::from([first.session_id.clone()]))
    .open(OpenBrowserSession::disposable("bob", "default", 2_000))
    .unwrap_err();

    assert_eq!(error, "browser_disposable_profile_count_quota_protected");
    assert!(state.sessions.contains_key(&first.session_id));
    assert!(effects.disposable_deletions.is_empty());
    assert_eq!(effects.disposable_allocations.len(), 1);
}

#[test]
fn disposable_byte_quota_evicts_oldest_profile_first() {
    let mut catalog = catalog_with_disposable_policy();
    catalog
        .disposable_policies
        .get_mut("default")
        .unwrap()
        .maximum_total_bytes = 10;
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let alice = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 60_000,
            ..BrowserSessionManagerConfig::default()
        },
    )
    .open(OpenBrowserSession::disposable("alice", "default", 1_000))
    .unwrap();
    let bob = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 60_000,
            ..BrowserSessionManagerConfig::default()
        },
    )
    .open(OpenBrowserSession::disposable("bob", "default", 2_000))
    .unwrap();
    effects.disposable_sizes.insert(alice.profile_id.clone(), 6);
    effects.disposable_sizes.insert(bob.profile_id.clone(), 6);

    let reaped = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 60_000,
            ..BrowserSessionManagerConfig::default()
        },
    )
    .reap(3_000)
    .unwrap();

    assert_eq!(reaped.deleted_disposable_profile_ids, [alice.profile_id]);
    assert!(state.sessions.contains_key(&bob.session_id));
}

#[test]
fn exact_intent_rejects_disposable_profile_definition() {
    let profile = BrowserProfileCatalogEntry {
        manual_login_url: None,
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
            remote_view_desktops: Vec::new(),
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
                maximum_retained_profiles: 20,
                maximum_total_bytes: 10 * 1024 * 1024 * 1024,
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
                remote_view_desktops: Vec::new(),
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
            remote_view_desktops: Vec::new(),
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

#[test]
fn exhausted_new_session_fields_do_not_launch_and_sequence_exhaustion_allows_healthy_reuse() {
    let catalog = catalog_with_named_profile();
    for (sequence, activity, error) in [
        (u64::MAX, 1_000, "browser_session_sequence_exhausted"),
        (0, u64::MAX, "browser_session_expiry_exhausted"),
    ] {
        let mut state = BrowserSessionState {
            next_session_sequence: sequence,
            ..BrowserSessionState::default()
        };
        let before = state.clone();
        let mut effects = FixtureEffects::default();
        let result = BrowserSessionManager::new(
            &mut state,
            &catalog,
            &mut effects,
            BrowserSessionManagerConfig {
                session_idle_timeout_ms: 300_000,
                remote_view_desktops: vec![],
            },
        )
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            activity,
        ));
        assert_eq!(result.unwrap_err(), error);
        assert!(effects.launches.is_empty());
        assert!(effects.closes.is_empty());
        assert_eq!(state, before);
    }
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 300_000,
        remote_view_desktops: vec![],
    };
    let first = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    state.next_session_sequence = u64::MAX;
    let reused = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            2_000,
        ))
        .unwrap();
    assert_eq!(reused.session_id, first.session_id);
    assert_eq!(reused.session_disposition, SessionRecordDisposition::Reused);
    assert_eq!(effects.launches.len(), 1);
}

#[test]
fn retained_handoff_recovers_dead_browser_without_retiring_logical_sessions_or_peer_tabs() {
    use agent_browser_service_model::{
        resolve_remote_view_tab_handoff, retain_remote_view_tab_handoff,
    };
    let catalog = catalog_with_named_profile();
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 300_000,
        remote_view_desktops: vec![],
    };
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "retained-tab".into(),
            target_id: "dead-target".into(),
            source: BrowserTabSource::SessionInitial,
        }),
        new_tab: Some(BrowserTabAcquisition {
            tab_id: "peer-tab".into(),
            target_id: "dead-peer-target".into(),
            source: BrowserTabSource::ExplicitNew,
        }),
        ..Default::default()
    };
    let alice = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .tab_for_navigation(&alice.session_id, 1_000)
        .unwrap();
    let bob = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile("bob", "profile-a", 1_000))
        .unwrap();
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .new_tab(&bob.session_id, 1_000)
        .unwrap();
    retain_remote_view_tab_handoff(
        &mut state,
        "same-handoff",
        &alice.session_id,
        "retained-tab",
    )
    .unwrap();
    state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    effects.browser_live = false;
    let resumed = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            2_000,
        ))
        .unwrap();
    assert_eq!(resumed.session_id, alice.session_id);
    assert_eq!(resumed.browser_id, alice.browser_id);
    let resolved = resolve_remote_view_tab_handoff(&state, "same-handoff").unwrap();
    assert_eq!(resolved.tab.id, "retained-tab");
    assert_eq!(resolved.tab.target_id, "recovered-dead-target");
    assert_eq!(resolved.browser.pid, 5252);
    assert_eq!(
        state.tabs["peer-tab"].target_id,
        "recovered-dead-peer-target"
    );
    assert_eq!(state.sessions[&bob.session_id].browser_id, alice.browser_id);
    assert!(state.session_history.is_empty());
    assert!(state.tab_history.is_empty());
    assert!(effects.closes.is_empty());
}

#[test]
fn retained_handoff_does_not_disable_browser_idle_reaper() {
    use agent_browser_service_model::{
        resolve_remote_view_tab_handoff, retain_remote_view_tab_handoff,
    };
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "operator-tab".into(),
            target_id: "operator-target".into(),
            source: BrowserTabSource::SessionInitial,
        }),
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 1_000,
        remote_view_desktops: Vec::new(),
    };
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .tab_for_navigation(&opened.session_id, 1_000)
        .unwrap();
    retain_remote_view_tab_handoff(
        &mut state,
        "operator-handoff",
        &opened.session_id,
        "operator-tab",
    )
    .unwrap();
    // The same persisted aggregate and manager seam are used after host restart.
    state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    let reaped = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .with_protected_sessions(BTreeSet::new())
        .reap(20_000)
        .unwrap();
    assert!(
        reaped.expired_session_ids.is_empty(),
        "retained operator session was reaped"
    );
    assert_eq!(
        resolve_remote_view_tab_handoff(&state, "operator-handoff")
            .unwrap()
            .tab
            .id,
        "operator-tab"
    );
    assert_eq!(
        effects.closes,
        [opened.browser_id.clone()],
        "a retained handoff must not pin its idle browser process"
    );
    assert_eq!(reaped.closed_browser_ids, [opened.browser_id.clone()]);
    effects.browser_live = false;
    let reaped_again =
        BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
            .reap(21_000)
            .unwrap();
    assert!(
        reaped_again.closed_browser_ids.is_empty(),
        "idle closure must be idempotent after restart"
    );
    state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    let resumed = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            30_000,
        ))
        .unwrap();
    assert_eq!(resumed.session_id, opened.session_id);
    assert_eq!(resumed.browser_id, opened.browser_id);
    assert_eq!(effects.launches.len(), 2);
    assert_eq!(
        resolve_remote_view_tab_handoff(&state, "operator-handoff")
            .unwrap()
            .tab
            .id,
        "operator-tab"
    );
}

#[test]
fn idle_browser_close_preserves_active_peers_and_external_operation_custody() {
    use agent_browser_service_model::{
        resolve_remote_view_tab_handoff, retain_remote_view_tab_handoff,
    };
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "operator-tab".into(),
            target_id: "operator-target".into(),
            source: BrowserTabSource::SessionInitial,
        }),
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 1_000,
        remote_view_desktops: Vec::new(),
    };
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .tab_for_navigation(&opened.session_id, 1_000)
        .unwrap();
    retain_remote_view_tab_handoff(
        &mut state,
        "operator-handoff",
        &opened.session_id,
        "operator-tab",
    )
    .unwrap();
    // The same persisted aggregate and manager seam are used after host restart.
    state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    let peer = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "bob",
            "profile-a",
            19_500,
        ))
        .unwrap();
    let result = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .reap(20_000)
        .unwrap();
    assert!(result.closed_browser_ids.is_empty());
    assert!(effects.closes.is_empty());
    let result = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .with_protected_sessions(BTreeSet::from([peer.session_id.clone()]))
        .reap(30_000)
        .unwrap();
    assert!(result.closed_browser_ids.is_empty());
    assert!(effects.closes.is_empty());
    let result = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .reap(31_000)
        .unwrap();
    assert_eq!(result.closed_browser_ids, [opened.browser_id.clone()]);
    assert_eq!(effects.closes, [opened.browser_id]);
    assert!(resolve_remote_view_tab_handoff(&state, "operator-handoff").is_ok());
}

#[test]
fn failed_idle_browser_close_does_not_publish_closed_state() {
    use agent_browser_service_model::retain_remote_view_tab_handoff;
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "operator-tab".into(),
            target_id: "operator-target".into(),
            source: BrowserTabSource::SessionInitial,
        }),
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 1_000,
        remote_view_desktops: Vec::new(),
    };
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .tab_for_navigation(&opened.session_id, 1_000)
        .unwrap();
    retain_remote_view_tab_handoff(
        &mut state,
        "operator-handoff",
        &opened.session_id,
        "operator-tab",
    )
    .unwrap();
    // The same persisted aggregate and manager seam are used after host restart.
    state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    let before = state.clone();
    effects.close_error = Some("exact owner close failed".into());
    let error = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .reap(20_000)
        .unwrap_err();
    assert_eq!(error, "exact owner close failed");
    assert_eq!(state, before);
    effects.close_error = None;
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .reap(20_000)
        .unwrap();
    assert_eq!(effects.closes, [opened.browser_id]);
}

#[test]
fn a_new_peer_recovers_the_retained_browser_instead_of_retiring_existing_sessions() {
    use agent_browser_service_model::{
        resolve_remote_view_tab_handoff, retain_remote_view_tab_handoff,
    };
    let catalog = catalog_with_named_profile();
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 300_000,
        remote_view_desktops: vec![],
    };
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "retained-tab".into(),
            target_id: "dead-target".into(),
            source: BrowserTabSource::SessionInitial,
        }),
        new_tab: Some(BrowserTabAcquisition {
            tab_id: "peer-tab".into(),
            target_id: "dead-peer-target".into(),
            source: BrowserTabSource::ExplicitNew,
        }),
        ..Default::default()
    };
    let alice = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .tab_for_navigation(&alice.session_id, 1_000)
        .unwrap();
    let bob = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile("bob", "profile-a", 1_000))
        .unwrap();
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .new_tab(&bob.session_id, 1_000)
        .unwrap();
    retain_remote_view_tab_handoff(
        &mut state,
        "same-handoff",
        &alice.session_id,
        "retained-tab",
    )
    .unwrap();
    state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    effects.browser_live = false;
    let resumed = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .open(OpenBrowserSession::exact_profile(
            "charlie",
            "profile-a",
            2_000,
        ))
        .unwrap();
    assert_ne!(resumed.session_id, alice.session_id);
    assert!(state.sessions.contains_key(&alice.session_id));
    assert_eq!(resumed.browser_id, alice.browser_id);
    let resolved = resolve_remote_view_tab_handoff(&state, "same-handoff").unwrap();
    assert_eq!(resolved.tab.id, "retained-tab");
    assert_eq!(resolved.tab.target_id, "recovered-dead-target");
    assert_eq!(resolved.browser.pid, 5252);
    assert_eq!(
        state.tabs["peer-tab"].target_id,
        "recovered-dead-peer-target"
    );
    assert_eq!(state.sessions[&bob.session_id].browser_id, alice.browser_id);
    assert!(state.session_history.is_empty());
    assert!(state.tab_history.is_empty());
    assert!(effects.closes.is_empty());
}

#[test]
fn profile_executables_survive_restart_and_recovery_after_another_profile_selection() {
    use agent_browser_service_model::retain_remote_view_tab_handoff;
    let mut catalog = catalog_with_named_profile();
    let mut second = catalog.profiles["profile-a"].clone();
    second.id = "profile-b".into();
    second.user_data_dir = "/managed/profiles/profile-b".into();
    catalog.profiles.insert(second.id.clone(), second);
    let mut state = BrowserSessionState::default();
    state.profile_executables = BTreeMap::from([
        ("profile-a".into(), "/reviewed/stock/chrome".into()),
        ("profile-b".into(), "/reviewed/stealth/chrome".into()),
    ]);
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 300_000,
        remote_view_desktops: vec![],
    };
    let mut effects = FixtureEffects {
        browser_live: true,
        ..Default::default()
    };
    let mut opened = Vec::new();
    for (name, profile) in [("alice", "profile-a"), ("bob", "profile-b")] {
        effects.initial_tab = Some(BrowserTabAcquisition {
            tab_id: format!("tab-{name}"),
            target_id: format!("target-{name}"),
            source: BrowserTabSource::SessionInitial,
        });
        let session =
            BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
                .open(OpenBrowserSession::exact_profile(name, profile, 100))
                .unwrap();
        let tab = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
            .tab_for_navigation(&session.session_id, 100)
            .unwrap();
        retain_remote_view_tab_handoff(
            &mut state,
            &format!("handoff-{name}"),
            &session.session_id,
            &tab.tab_id,
        )
        .unwrap();
        opened.push(session);
    }
    assert_eq!(
        effects.launch_executables,
        vec![
            ("profile-a".into(), "/reviewed/stock/chrome".into()),
            ("profile-b".into(), "/reviewed/stealth/chrome".into())
        ]
    );
    state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    let peer = state.browsers[&opened[1].browser_id].clone();
    let mut restarted = FixtureEffects {
        executable: "/wrong/global/chrome".into(),
        ..Default::default()
    };
    BrowserSessionManager::new(&mut state, &catalog, &mut restarted, config.clone())
        .recover_retained_browser(&opened[0].browser_id)
        .unwrap();
    assert_eq!(state.browsers[&opened[1].browser_id], peer);
    restarted.browser_live = false;
    BrowserSessionManager::new(&mut state, &catalog, &mut restarted, config)
        .recover_retained_browser(&opened[1].browser_id)
        .unwrap();
    assert_eq!(
        restarted.launch_executables,
        vec![
            ("profile-a".into(), "/reviewed/stock/chrome".into()),
            ("profile-b".into(), "/reviewed/stealth/chrome".into())
        ]
    );
}

#[test]
fn pressure_policy_preserves_active_peer_after_link_deletion() {
    let mut catalog = catalog_with_disposable_policy();
    catalog
        .disposable_policies
        .get_mut("default")
        .unwrap()
        .maximum_retained_profiles = 1;
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "pressure-alice-tab".into(),
            target_id: "pressure-alice-target".into(),
            source: BrowserTabSource::SessionInitial,
        }),
        ..Default::default()
    };
    let config = BrowserSessionManagerConfig::default();
    let alice = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::disposable("alice", "default", 100))
        .unwrap();
    let alice_tab = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .tab_for_navigation(&alice.session_id, 100)
        .unwrap();
    agent_browser_service_model::retain_remote_view_tab_handoff(
        &mut state,
        "alice-link",
        &alice.session_id,
        &alice_tab.tab_id,
    )
    .unwrap();
    state.tab_lifetimes.insert(
        alice_tab.tab_id,
        agent_browser_service_model::BrowserTabLifetime {
            inactivity_ms: None,
            pressure_inactivity_ms: Some(30),
        },
    );
    effects.new_tab = Some(BrowserTabAcquisition {
        tab_id: "pressure-peer-tab".into(),
        target_id: "pressure-peer-target".into(),
        source: BrowserTabSource::ExplicitNew,
    });
    let peer = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .new_tab(&alice.session_id, 125)
        .unwrap();
    state.remote_view_tab_handoffs.remove("alice-link");
    state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    let result = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .open(OpenBrowserSession::disposable("bob", "default", 130));
    assert_eq!(
        result.unwrap_err(),
        "browser_disposable_profile_count_quota_protected"
    );
    assert!(state.sessions.contains_key(&alice.session_id));
    assert!(state.tabs.contains_key(&peer.tab_id));
    assert!(effects.closes.is_empty());
    assert!(effects.disposable_deletions.is_empty());
}

#[test]
fn idle_retirement_guard_preserves_expired_matching_session_before_replacement() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..Default::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 1_000,
        remote_view_desktops: vec![],
    };
    let first = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "alice",
            "profile-a",
            1_000,
        ))
        .unwrap();
    effects.idle_retirement_permitted = Some(false);
    let replacement =
        BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone()).open(
            OpenBrowserSession::exact_profile("alice", "profile-a", 2_000),
        );
    assert!(
        effects.closes.is_empty(),
        "active authority must prevent expired-epoch replacement close"
    );
    assert!(replacement.is_err());
    assert!(state.sessions.contains_key(&first.session_id));
    assert_eq!(effects.launches.len(), 1);
    // Explicit user close remains authorized even while idle retirement is denied.
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .close_session(&first.session_id, SessionEndReason::ExplicitClose, 2_001)
        .unwrap();
    assert_eq!(effects.closes, [first.browser_id]);
}

#[test]
fn idle_retirement_guard_covers_quota_profile_cleanup_and_releases() {
    let mut catalog = catalog_with_disposable_policy();
    catalog
        .disposable_policies
        .get_mut("default")
        .unwrap()
        .maximum_retained_profiles = 1;
    let active = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        retirement_guard_active: Some(active.clone()),
        ..Default::default()
    };
    let first = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig::default(),
    )
    .open(OpenBrowserSession::disposable("alice", "default", 1_000))
    .unwrap();
    BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig::default(),
    )
    .open(OpenBrowserSession::disposable("bob", "default", 2_000))
    .unwrap();
    assert_eq!(effects.disposable_deletions, [first.profile_id]);
    assert!(!active.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn quota_qualifies_all_candidates_before_first_retirement_effect() {
    let mut catalog = catalog_with_disposable_policy();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..Default::default()
    };
    let active = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    effects.retirement_guard_active = Some(active.clone());
    let config = BrowserSessionManagerConfig::default();
    let first = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::disposable("alice", "default", 1_000))
        .unwrap();
    let second = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::disposable("bob", "default", 2_000))
        .unwrap();
    catalog
        .disposable_policies
        .get_mut("default")
        .unwrap()
        .maximum_retained_profiles = 1;
    effects.retirement_guard_denied_browser = Some(second.browser_id.clone());
    let result = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .enforce_disposable_quota("default", &Default::default(), 1, 0, 3_000);
    assert_eq!(
        result.unwrap_err(),
        "browser_disposable_profile_activity_changed"
    );
    assert!(
        effects.closes.is_empty(),
        "later candidate protection must precede every close"
    );
    assert!(effects.disposable_deletions.is_empty());
    assert!(state.sessions.contains_key(&first.session_id));
    assert!(state.sessions.contains_key(&second.session_id));
    assert_eq!(state.disposable_profiles.len(), 2);
    assert!(!active.load(std::sync::atomic::Ordering::SeqCst));
    effects.retirement_guard_denied_browser = None;
    let admitted = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .enforce_disposable_quota("default", &Default::default(), 1, 0, 3_001)
        .unwrap();
    assert_eq!(admitted.quota_evicted_session_ids.len(), 2);
    assert_eq!(effects.closes.len(), 2);
    assert_eq!(effects.disposable_deletions.len(), 2);
    assert!(state.sessions.is_empty());
    assert!(state.disposable_profiles.is_empty());
    assert!(!active.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn scheduled_profile_cleanup_retains_activity_exclusion_through_deletion() {
    let catalog = catalog_with_disposable_policy();
    let mut state = BrowserSessionState::default();
    let active = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut effects = FixtureEffects {
        browser_live: true,
        retirement_guard_active: Some(active.clone()),
        ..Default::default()
    };
    let config = BrowserSessionManagerConfig::default();
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::disposable("alice", "default", 1_000))
        .unwrap();
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .close_session(&opened.session_id, SessionEndReason::ExplicitClose, 2_000)
        .unwrap();
    assert!(state.browsers.is_empty());
    assert!(state.sessions.is_empty());
    let due = state
        .disposable_profiles
        .get(&opened.profile_id)
        .unwrap()
        .cleanup_eligible_at_ms
        .unwrap();
    effects.profile_cleanup_permitted = Some(false);
    let deferred = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .reap(due)
        .unwrap();
    assert!(deferred.deleted_disposable_profile_ids.is_empty());
    assert!(effects.disposable_deletions.is_empty());
    assert!(state.disposable_profiles.contains_key(&opened.profile_id));
    effects.profile_cleanup_permitted = Some(true);
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .reap(due)
        .unwrap();
    assert_eq!(effects.disposable_deletions, [opened.profile_id]);
    assert!(!active.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn quota_preserves_orphan_profile_when_cleanup_authority_denies() {
    let mut catalog = catalog_with_disposable_policy();
    catalog
        .disposable_policies
        .get_mut("default")
        .unwrap()
        .maximum_retained_profiles = 1;
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..Default::default()
    };
    let config = BrowserSessionManagerConfig::default();
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::disposable("alice", "default", 1_000))
        .unwrap();
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .close_session(&opened.session_id, SessionEndReason::ExplicitClose, 2_000)
        .unwrap();
    assert!(state.browsers.is_empty());
    assert!(state.sessions.is_empty());
    let before = state.clone();
    effects.closes.clear();
    effects.profile_cleanup_permitted = Some(false);
    let result = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .enforce_disposable_quota("default", &BTreeSet::new(), 1, 0, 2_001);
    assert!(
        effects.disposable_deletions.is_empty(),
        "quota deleted a profile protected by cleanup authority"
    );
    assert!(effects.closes.is_empty());
    assert_eq!(state, before);
    assert!(
        result.is_err(),
        "pressure admission must defer when its required profile is protected"
    );
}

#[test]
fn quota_checks_later_orphan_before_closing_first_browser_and_releases_on_failure() {
    let mut catalog = catalog_with_disposable_policy();
    let mut state = BrowserSessionState::default();
    let active = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut effects = FixtureEffects {
        browser_live: true,
        retirement_guard_active: Some(active.clone()),
        ..Default::default()
    };
    let config = BrowserSessionManagerConfig::default();
    let first = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::disposable("alice", "default", 1_000))
        .unwrap();
    let orphan = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::disposable("bob", "default", 2_000))
        .unwrap();
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .close_session(&orphan.session_id, SessionEndReason::ExplicitClose, 2_001)
        .unwrap();
    effects.closes.clear();
    catalog
        .disposable_policies
        .get_mut("default")
        .unwrap()
        .maximum_retained_profiles = 1;
    effects.profile_cleanup_denied_profile = Some(orphan.profile_id.clone());
    let before = state.clone();
    let error = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .enforce_disposable_quota("default", &BTreeSet::new(), 1, 0, 3_000)
        .unwrap_err();
    assert_eq!(error, "browser_disposable_profile_activity_changed");
    assert!(effects.closes.is_empty());
    assert!(effects.disposable_deletions.is_empty());
    assert_eq!(state, before);
    assert!(!active.load(std::sync::atomic::Ordering::SeqCst));
    effects.profile_cleanup_denied_profile = None;
    effects.disposable_delete_error = Some("synthetic_delete_failure".into());
    let error = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .enforce_disposable_quota("default", &BTreeSet::new(), 1, 0, 3_001)
        .unwrap_err();
    assert_eq!(error, "synthetic_delete_failure");
    assert!(!active.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(effects.closes, [first.browser_id]);
    assert!(state.disposable_profiles.contains_key(&first.profile_id));
    assert!(state.disposable_profiles.contains_key(&orphan.profile_id));
    effects.disposable_delete_error = None;
    let result = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .enforce_disposable_quota("default", &BTreeSet::new(), 1, 0, 3_002)
        .unwrap();
    assert_eq!(result.deleted_disposable_profile_ids.len(), 2);
    assert!(state.disposable_profiles.is_empty());
    assert!(!active.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn retained_owner_operation_prevents_competing_profile_open() {
    let catalog = catalog_with_named_profile();
    let mut retained = serde_json::to_value(BrowserSessionState::default()).unwrap();
    retained["profileOperationIds"] = serde_json::json!({"profile-a":"mode-one"});
    let mut state: BrowserSessionState = serde_json::from_value(retained).unwrap();
    let mut effects = FixtureEffects::default();
    let mut manager = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: vec![],
        },
    );
    assert_eq!(
        manager
            .open(OpenBrowserSession::exact_profile(
                "competing",
                "profile-a",
                1000
            ))
            .unwrap_err(),
        "browser_profile_operation_pending:mode-one:inspect or resume the owner operation"
    );
}

#[test]
fn owner_operation_retains_tasks_during_automatic_reaping() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 100,
        remote_view_desktops: vec![],
    };
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "owner",
            "profile-a",
            1000,
        ))
        .unwrap();
    let mut retained = serde_json::to_value(state).unwrap();
    retained["profileOperationIds"] = serde_json::json!({"profile-a":"mode-one"});
    let mut state: BrowserSessionState = serde_json::from_value(retained).unwrap();
    let result = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .reap(10_000)
        .unwrap();
    assert!(result.closed_browser_ids.is_empty());
    assert!(result.expired_session_ids.is_empty());
    assert!(state.sessions.contains_key(&opened.session_id));
}

#[test]
fn owner_mode_replacement_preserves_logical_task_and_handoff() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        initial_tab: Some(BrowserTabAcquisition {
            tab_id: "original-tab".into(),
            target_id: "original-target".into(),
            source: BrowserTabSource::SessionInitial,
        }),
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 300_000,
        remote_view_desktops: vec![],
    };
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "owner",
            "profile-a",
            1000,
        ))
        .unwrap();
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .navigate(&opened.session_id, "https://synthetic.test/", 1001)
        .unwrap();
    let tab = state.sessions[&opened.session_id]
        .current_tab_id
        .clone()
        .unwrap();
    agent_browser_service_model::retain_remote_view_tab_handoff(
        &mut state,
        "durable-original",
        &opened.session_id,
        &tab,
    )
    .unwrap();
    let mut retained = serde_json::to_value(state).unwrap();
    retained["profileOperationIds"] = serde_json::json!({"profile-a":"mode-one"});
    let mut state: BrowserSessionState = serde_json::from_value(retained).unwrap();
    let previous = state.clone();
    let replacement = agent_browser_service_model::BrowserRecovery {
        launch: BrowserLaunch {
            browser_id: opened.browser_id.clone(),
            pid: 999,
            cdp_endpoint: String::new(),
            desktop: None,
            process_identity: Some(agent_browser_service_model::RecordedProcessIdentity {
                pid: 999,
                start_token: "boot:new".into(),
                executable_path: Some("/fixture/chrome".into()),
                browser_family: Some("chrome".into()),
            }),
        },
        target_ids: BTreeMap::from([(tab.clone(), "native-window-new".into())]),
    };
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .publish_owner_replacement(&opened.browser_id, "mode-one", replacement)
        .unwrap();
    assert_eq!(state.sessions, previous.sessions);
    assert_eq!(
        state.remote_view_tab_handoffs,
        previous.remote_view_tab_handoffs
    );
    assert_eq!(state.tabs[&tab].target_id, "native-window-new");
    assert_eq!(state.browsers[&opened.browser_id].pid, 999);
    assert!(state.browsers[&opened.browser_id].cdp_endpoint.is_empty());
}

#[test]
fn owner_operation_prevents_a_competing_legacy_close() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 300_000,
        remote_view_desktops: vec![],
    };
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "owner",
            "profile-a",
            1000,
        ))
        .unwrap();
    let mut retained = serde_json::to_value(state).unwrap();
    retained["profileOperationIds"] = serde_json::json!({"profile-a":"copy-one"});
    let mut state: BrowserSessionState = serde_json::from_value(retained).unwrap();
    let result = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .close_session(&opened.session_id, SessionEndReason::ExplicitClose, 1001);
    assert_eq!(result.unwrap_err(),"browser_profile_operation_pending:copy-one:inspect or cancel the owner operation before closing");
}

#[test]
fn persisted_profile_pin_preserves_unpinned_default_in_both_launch_orders_and_restart() {
    for profiles in [["profile-a", "profile-b"], ["profile-b", "profile-a"]] {
        let mut catalog = catalog_with_named_profile();
        let mut second = catalog.profiles["profile-a"].clone();
        second.id = "profile-b".into();
        second.user_data_dir = "/managed/profiles/profile-b".into();
        catalog.profiles.insert(second.id.clone(), second);
        let mut state = BrowserSessionState::default();
        state
            .profile_executables
            .insert("profile-a".into(), "/reviewed/pinned/chrome".into());
        for _restart in 0..2 {
            state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
            let mut effects = FixtureEffects {
                browser_live: true,
                executable: "/reviewed/default/chrome".into(),
                ..Default::default()
            };
            let config = BrowserSessionManagerConfig::default();
            for profile in profiles {
                let opened =
                    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
                        .open(OpenBrowserSession::exact_profile(profile, profile, 100))
                        .unwrap();
                BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
                    .close_session(&opened.session_id, SessionEndReason::ExplicitClose, 101)
                    .unwrap();
            }
            let expected = profiles.map(|id| {
                (
                    id.to_owned(),
                    if id == "profile-a" {
                        "/reviewed/pinned/chrome"
                    } else {
                        "/reviewed/default/chrome"
                    }
                    .to_owned(),
                )
            });
            assert_eq!(
                effects.launch_executables, expected,
                "unrelated profile must retain configured default"
            );
        }
    }
}

#[test]
fn borrowed_close_failure_protects_the_failed_tasks_remaining_expired_tabs() {
    use agent_browser_service_model::{
        BrowserTabLifetime, ManagedBrowserInstance, ManagedBrowserSession, ManagedBrowserTab,
    };
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let browser = ManagedBrowserInstance {
        id: "borrowed".into(),
        profile_id: "profile-a".into(),
        pid: 42,
        cdp_endpoint: "http://127.0.0.1:9222".into(),
        desktop: None,
        active_session_ids: vec!["good".into(), "failed".into(), "later".into()],
    };
    state.browsers.insert(browser.id.clone(), browser.clone());
    state.shared_browser_ids.insert(browser.id.clone());
    for (id, tabs) in [
        ("good", vec!["a-good"]),
        ("failed", vec!["b-failed-0", "b-failed-1"]),
        ("later", vec!["c-good"]),
    ] {
        state.sessions.insert(
            id.into(),
            ManagedBrowserSession {
                id: id.into(),
                name: id.into(),
                profile_id: browser.profile_id.clone(),
                browser_id: browser.id.clone(),
                created_at_ms: 100,
                last_activity_at_ms: 100,
                expires_at_ms: 1100,
                current_tab_id: Some(tabs[0].into()),
            },
        );
        for tab in tabs {
            state.tabs.insert(
                tab.into(),
                ManagedBrowserTab {
                    id: tab.into(),
                    target_id: tab.into(),
                    browser_id: browser.id.clone(),
                    session_id: id.into(),
                    created_at_ms: 100,
                    last_activity_at_ms: 100,
                },
            );
            state.tab_lifetimes.insert(
                tab.into(),
                BrowserTabLifetime {
                    inactivity_ms: Some(30),
                    pressure_inactivity_ms: None,
                },
            );
        }
    }
    let before = state.clone();
    let mut effects = FixtureEffects {
        tab_close_failure_target: Some("b-failed-0".into()),
        ..Default::default()
    };
    let result = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig::default(),
    )
    .reap(130)
    .unwrap();
    assert!(!state.sessions.contains_key("good"));
    assert_eq!(state.sessions["failed"], before.sessions["failed"]);
    assert_eq!(state.tabs["b-failed-0"], before.tabs["b-failed-0"]);
    assert_eq!(
        state.tabs.get("b-failed-1"),
        before.tabs.get("b-failed-1"),
        "newly protected task projection must survive the precomputed expiry list"
    );
    assert!(
        !state.sessions.contains_key("later"),
        "a failed task cannot starve a distinct later eligible task on the same borrowed browser"
    );
    assert_eq!(state.tab_history.len(), 2);
    assert_eq!(
        effects.tab_closes,
        vec![
            ("borrowed".into(), "a-good".into()),
            ("borrowed".into(), "b-failed-0".into()),
            ("borrowed".into(), "c-good".into())
        ]
    );
    assert_eq!(
        result
            .deferred_browser_errors
            .get("borrowed")
            .map(String::as_str),
        Some("synthetic_tab_close_unknown")
    );
    assert!(effects.closes.is_empty());
}

#[test]
fn public_existing_profile_adoption_retains_original_kernel_identity_across_restart() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let identity = agent_browser_service_model::RecordedProcessIdentity {
        pid: 7123,
        start_token: "boot:original".into(),
        executable_path: Some("/fixture/chrome".into()),
        browser_family: Some("chrome".into()),
    };
    let mut effects = FixtureEffects {
        qualified_adoption: Some(agent_browser_service_model::BrowserRecovery {
            launch: BrowserLaunch {
                browser_id: "borrowed-browser".into(),
                pid: identity.pid,
                cdp_endpoint: "ws://127.0.0.1:9999/devtools/browser/original".into(),
                desktop: None,
                process_identity: Some(identity.clone()),
            },
            target_ids: BTreeMap::new(),
        }),
        ..FixtureEffects::default()
    };
    let opened = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: vec![],
        },
    )
    .open(OpenBrowserSession::exact_profile(
        "original",
        "profile-a",
        1000,
    ))
    .unwrap();
    assert!(effects.launches.is_empty());
    assert!(state.shared_browser_ids.contains(&opened.browser_id));
    let restored: serde_json::Value =
        serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    assert_eq!(
        restored["adoptedBrowserProcessIdentities"][&opened.browser_id]["process"],
        serde_json::to_value(identity).unwrap(),
        "Qualified public existing-profile adoption must retain its original immutable identity"
    );
}

#[test]
fn authentication_retention_survives_restart_and_releases_only_its_own_claim() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 100,
        remote_view_desktops: vec![],
    };
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "Auth wait",
            "profile-a",
            1000,
        ))
        .unwrap();
    let browser = state.browsers[&opened.browser_id].clone();
    let request = agent_browser_service_model::BrowserSessionRetentionRequest {
        operation: "acquire".into(),
        claim_id: "auth:request-one".into(),
        owner_id: "auth-receipts".into(),
        session_id: opened.session_id.clone(),
        browser_id: browser.id.clone(),
        browser_pid: browser.pid,
        ttl_ms: Some(1000),
        generation: None,
    };
    let receipt = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .retain_session(request.clone(), 1050)
        .unwrap();
    state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    let result = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .reap(1500)
        .unwrap();
    assert!(result.closed_browser_ids.is_empty());
    assert!(state.leave_open_session_ids.is_empty());
    let mut released = request.clone();
    released.operation = "release".into();
    released.generation = Some(receipt.generation.clone());
    let mut wrong_owner = released.clone();
    wrong_owner.owner_id = "another-service".into();
    assert!(
        BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
            .retain_session(wrong_owner, 1600)
            .is_err()
    );
    BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .retain_session(released, 1600)
        .unwrap();
    assert!(
        BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
            .retain_session(request.clone(), 1601)
            .is_err()
    );
    let result = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .reap(1602)
        .unwrap();
    assert_eq!(result.closed_browser_ids, vec![browser.id]);
    let mut terminal = request;
    terminal.operation = "release".into();
    terminal.generation = Some(receipt.generation);
    let done = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig::default(),
    )
    .retain_session(terminal, 1700)
    .unwrap();
    assert!(done.released);
}

#[test]
fn authentication_retention_expires_after_owner_crash_and_cannot_rebind_a_process() {
    let catalog = catalog_with_named_profile();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 100,
        remote_view_desktops: vec![],
    };
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(OpenBrowserSession::exact_profile(
            "Auth wait",
            "profile-a",
            1000,
        ))
        .unwrap();
    let browser = state.browsers[&opened.browser_id].clone();
    let request = agent_browser_service_model::BrowserSessionRetentionRequest {
        operation: "acquire".into(),
        claim_id: "auth:request-one".into(),
        owner_id: "auth-receipts".into(),
        session_id: opened.session_id.clone(),
        browser_id: browser.id.clone(),
        browser_pid: browser.pid,
        ttl_ms: Some(200),
        generation: None,
    };
    let receipt = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .retain_session(request.clone(), 1050)
        .unwrap();
    let mut bad = request.clone();
    bad.browser_pid += 1;
    assert!(
        BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
            .retain_session(bad, 1100)
            .is_err()
    );
    let mut renewal = request.clone();
    renewal.operation = "renew".into();
    renewal.generation = Some(receipt.generation);
    state.browsers.get_mut(&browser.id).unwrap().pid += 1;
    assert!(
        BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
            .retain_session(renewal, 1100)
            .is_err()
    );
    state.browsers.get_mut(&browser.id).unwrap().pid = browser.pid;
    state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    let result = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config)
        .reap(1250)
        .unwrap();
    assert_eq!(result.closed_browser_ids, vec![browser.id]);
}

#[test]
fn authentication_retention_blocks_new_profile_quota_eviction() {
    let mut catalog = catalog_with_disposable_policy();
    catalog
        .disposable_policies
        .get_mut("default")
        .unwrap()
        .maximum_retained_profiles = 1;
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let first = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig::default(),
    )
    .open(OpenBrowserSession::disposable("auth", "default", 1000))
    .unwrap();
    let browser = state.browsers[&first.browser_id].clone();
    let request = agent_browser_service_model::BrowserSessionRetentionRequest {
        operation: "acquire".into(),
        claim_id: "auth:quota-wait".into(),
        owner_id: "auth-receipts".into(),
        session_id: first.session_id.clone(),
        browser_id: browser.id,
        browser_pid: browser.pid,
        ttl_ms: Some(5000),
        generation: None,
    };
    BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig::default(),
    )
    .retain_session(request, 1001)
    .unwrap();
    let next = BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut effects,
        BrowserSessionManagerConfig::default(),
    )
    .open(OpenBrowserSession::disposable("peer", "default", 2000));
    assert_eq!(
        next.unwrap_err(),
        "browser_disposable_profile_count_quota_protected"
    );
    assert!(state.sessions.contains_key(&first.session_id));
    assert!(effects.disposable_deletions.is_empty());
    assert_eq!(effects.disposable_allocations.len(), 1);
}

#[test]
fn disposable_manual_browser_keeps_its_mode_and_data_lifetime_across_reopen() {
    let catalog = catalog_with_disposable_policy();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 300_000,
        remote_view_desktops: vec![],
    };
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(
            OpenBrowserSession::disposable("operator", "default", 1_000)
                .with_manual_login_url("about:blank"),
        )
        .unwrap();
    assert_eq!(
        effects.manual_launch_urls,
        [Some("about:blank".to_string())]
    );
    assert_eq!(
        state.disposable_profiles[&opened.profile_id].profile.kind,
        BrowserProfileKind::Disposable
    );
    let mut restored: BrowserSessionState =
        serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    let reopened = BrowserSessionManager::new(&mut restored, &catalog, &mut effects, config)
        .open(
            OpenBrowserSession::disposable("operator", "default", 2_000)
                .with_manual_login_url("about:blank"),
        )
        .unwrap();
    assert_eq!(reopened.browser_id, opened.browser_id);
    assert_eq!(effects.disposable_allocations.len(), 1);
    assert_eq!(effects.manual_launch_urls.len(), 1);
}

#[test]
fn disposable_launch_pins_the_selected_executable_before_launch_and_after_reload() {
    let catalog = catalog_with_disposable_policy();
    let mut state = BrowserSessionState::default();
    let mut effects = FixtureEffects {
        browser_live: true,
        ..FixtureEffects::default()
    };
    let config = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 300_000,
        remote_view_desktops: vec![],
    };
    let opened = BrowserSessionManager::new(&mut state, &catalog, &mut effects, config.clone())
        .open(
            OpenBrowserSession::disposable("build-choice", "default", 1_000)
                .with_disposable_executable("/opt/chromium/chromium"),
        )
        .unwrap();
    assert_eq!(
        effects.launch_executables,
        [(opened.profile_id.clone(), "/opt/chromium/chromium".into())]
    );
    let mut restored: BrowserSessionState =
        serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    assert_eq!(
        restored.profile_executables[&opened.profile_id],
        "/opt/chromium/chromium"
    );
    let error = BrowserSessionManager::new(&mut restored, &catalog, &mut effects, config)
        .open(
            OpenBrowserSession::disposable("build-choice", "default", 2_000)
                .with_disposable_executable("/opt/chrome/chrome"),
        )
        .unwrap_err();
    assert!(error.contains("build_conflict"), "{error}");
    assert_eq!(effects.launch_executables.len(), 1);
}
