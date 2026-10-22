use agent_browser_service_model::*;
use std::collections::BTreeMap;

fn desktop(id: &str, generation: u64) -> RemoteViewFixedDesktop {
    RemoteViewFixedDesktop {
        desktop_id: id.into(),
        generation,
        friendly_route_label: id.into(),
    }
}
fn binding(id: &str, generation: u64) -> BrowserDesktopBinding {
    BrowserDesktopBinding {
        desktop_id: id.into(),
        generation,
    }
}
#[derive(Default)]
struct Effects {
    candidates: Vec<RemoteViewDesktopCandidate>,
    launched: Vec<Option<RemoteViewFixedDesktop>>,
    mutations: usize,
    live: Option<bool>,
    reconciled: Option<BrowserRecovery>,
}
impl BrowserSessionEffects for Effects {
    fn remote_view_desktop_candidates(
        &mut self,
    ) -> Result<Option<Vec<RemoteViewDesktopCandidate>>, String> {
        Ok(Some(self.candidates.clone()))
    }
    fn browser_is_live(&mut self, _: &ManagedBrowserInstance) -> Result<bool, String> {
        Ok(self.live.unwrap_or(true))
    }
    fn reconcile_profile_browser(
        &mut self,
        _: &BrowserProfileCatalogEntry,
        _: Option<&ManagedBrowserInstance>,
        _: &[ManagedBrowserTab],
        _: &[BrowserNavigationRecord],
    ) -> Result<Option<BrowserRecovery>, String> {
        Ok(self.reconciled.clone())
    }
    fn launch_browser(
        &mut self,
        _: &BrowserProfileCatalogEntry,
        desktop: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String> {
        self.launched.push(desktop.cloned());
        Ok(BrowserLaunch {
            process_identity: None,
            browser_id: "browser".into(),
            pid: 123,
            cdp_endpoint: "http://fixture".into(),
            desktop: desktop.cloned(),
        })
    }
    fn close_browser(&mut self, _: &ManagedBrowserInstance) -> Result<(), String> {
        self.mutations += 1;
        Ok(())
    }
    fn acquire_initial_tab(
        &mut self,
        _: &ManagedBrowserInstance,
        _: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        panic!("unexpected tab effect")
    }
    fn create_tab(&mut self, _: &ManagedBrowserInstance) -> Result<BrowserTabAcquisition, String> {
        panic!("unexpected tab effect")
    }
    fn close_tab(
        &mut self,
        _: &ManagedBrowserInstance,
        _: &ManagedBrowserTab,
    ) -> Result<(), String> {
        self.mutations += 1;
        Ok(())
    }
    fn navigate(
        &mut self,
        _: &ManagedBrowserInstance,
        _: &ManagedBrowserTab,
        _: &str,
    ) -> Result<(), String> {
        panic!("unexpected navigation")
    }
    fn focus_browser(
        &mut self,
        _: &ManagedBrowserInstance,
        _: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        panic!("unexpected focus")
    }
    fn allocate_disposable_profile(
        &mut self,
        _: &BrowserDisposableProfilePolicy,
        _: &str,
        _: &str,
    ) -> Result<BrowserProfileCatalogEntry, String> {
        panic!("unexpected allocation")
    }
    fn delete_disposable_profile(&mut self, _: &ManagedDisposableProfile) -> Result<(), String> {
        panic!("unexpected profile delete")
    }
}
fn catalog() -> BrowserProfileCatalog {
    let p = BrowserProfileCatalogEntry {
        id: "profile".into(),
        name: "Profile".into(),
        user_data_dir: "/fixture/profile".into(),
        kind: BrowserProfileKind::Named,
    };
    BrowserProfileCatalog {
        schema_version: "agent-browser.browser-profile-catalog.v1".into(),
        profiles: BTreeMap::from([(p.id.clone(), p)]),
        disposable_policies: BTreeMap::new(),
    }
}
fn effects(ids: &[(&str, u64, bool)]) -> Effects {
    Effects {
        candidates: ids
            .iter()
            .map(|(id, generation, ready)| RemoteViewDesktopCandidate {
                desktop: desktop(id, *generation),
                ready: *ready,
            })
            .collect(),
        ..Default::default()
    }
}
fn open(
    state: &mut BrowserSessionState,
    effects: &mut Effects,
    placement: Option<BrowserPlacement>,
    time: u64,
) -> Result<OpenBrowserSessionResult, String> {
    let catalog = catalog();
    let mut request = OpenBrowserSession::exact_profile("task", "profile", time);
    if let Some(placement) = placement {
        request = request.with_placement(placement);
    }
    BrowserSessionManager::new(
        state,
        &catalog,
        effects,
        BrowserSessionManagerConfig {
            session_idle_timeout_ms: 100,
            remote_view_desktops: vec![],
        },
    )
    .open(request)
}
#[test]
fn required_placement_selects_exact_generation_instead_of_provider_order() {
    let mut state = BrowserSessionState::default();
    let mut fx = effects(&[("other", 1, true), ("requested", 2, true)]);
    open(
        &mut state,
        &mut fx,
        Some(BrowserPlacement::Required {
            desktop: binding("requested", 2),
        }),
        1,
    )
    .unwrap();
    assert_eq!(fx.launched, vec![Some(desktop("requested", 2))]);
}
#[test]
fn stale_or_unready_required_placement_has_no_launch_or_state_effect() {
    for candidates in [
        vec![("requested", 1, true)],
        vec![("requested", 2, false)],
        vec![("other", 2, true)],
    ] {
        let mut state = BrowserSessionState::default();
        let before = state.clone();
        let mut fx = effects(&candidates);
        assert!(open(
            &mut state,
            &mut fx,
            Some(BrowserPlacement::Required {
                desktop: binding("requested", 2)
            }),
            1
        )
        .unwrap_err()
        .starts_with("browser_placement_unavailable:"));
        assert_eq!(state, before);
        assert!(fx.launched.is_empty());
        assert_eq!(fx.mutations, 0);
    }
}
#[test]
fn preferred_placement_uses_ordered_explicit_alternatives_only() {
    let placement = BrowserPlacement::Preferred {
        desktop: binding("preferred", 1),
        alternatives: vec![binding("second", 2), binding("third", 1)],
    };
    let mut state = BrowserSessionState::default();
    let mut fx = effects(&[
        ("unrelated", 1, true),
        ("third", 1, true),
        ("second", 2, true),
    ]);
    open(&mut state, &mut fx, Some(placement.clone()), 1).unwrap();
    assert_eq!(fx.launched, vec![Some(desktop("second", 2))]);
    let mut state = BrowserSessionState::default();
    let mut fx = effects(&[("unrelated", 1, true)]);
    assert!(open(&mut state, &mut fx, Some(placement), 1).is_err());
    assert!(fx.launched.is_empty());
}
#[test]
fn placement_conflict_preserves_even_expired_tasks_before_retirement() {
    let mut state = BrowserSessionState::default();
    let mut fx = effects(&[("original", 1, true), ("requested", 1, true)]);
    open(&mut state, &mut fx, None, 1).unwrap();
    let before = state.clone();
    assert!(open(
        &mut state,
        &mut fx,
        Some(BrowserPlacement::Required {
            desktop: binding("requested", 1)
        }),
        200
    )
    .unwrap_err()
    .starts_with("browser_placement_conflict:"));
    assert_eq!(state, before);
    assert_eq!(fx.launched.len(), 1);
    assert_eq!(fx.mutations, 0);
}
#[test]
fn hosted_wire_shape_round_trips_and_rejects_unrecognized_fields() {
    let placement = BrowserPlacement::Preferred {
        desktop: binding("a", 2),
        alternatives: vec![binding("b", 3)],
    };
    let wire = serde_json::json!({"mode":"preferred","desktop":{"desktopId":"a","generation":2},"alternatives":[{"desktopId":"b","generation":3}]});
    assert_eq!(serde_json::to_value(&placement).unwrap(), wire);
    assert_eq!(
        serde_json::from_value::<BrowserPlacement>(wire).unwrap(),
        placement
    );
    assert!(serde_json::from_value::<BrowserPlacement>(serde_json::json!({"mode":"required","desktop":{"desktopId":"a","generation":2},"fallback":true})).is_err());
}

#[test]
fn reconciliation_cannot_silently_rebind_a_constrained_task() {
    let mut state = BrowserSessionState::default();
    let mut fx = effects(&[("original", 1, true), ("other", 1, true)]);
    open(&mut state, &mut fx, None, 1).unwrap();
    let before = state.clone();
    fx.live = Some(false);
    fx.reconciled = Some(BrowserRecovery {
        launch: BrowserLaunch {
            process_identity: None,
            browser_id: "browser".into(),
            pid: 456,
            cdp_endpoint: "http://replacement".into(),
            desktop: Some(desktop("other", 1)),
        },
        target_ids: BTreeMap::new(),
    });
    let error = open(
        &mut state,
        &mut fx,
        Some(BrowserPlacement::Required {
            desktop: binding("original", 1),
        }),
        2,
    )
    .unwrap_err();
    assert!(error.starts_with("browser_placement_conflict:"));
    assert_eq!(state, before);
    assert_eq!(fx.mutations, 0);
    assert_eq!(fx.launched.len(), 1);
}

fn retain_hosted_constraint(
    state: &mut BrowserSessionState,
    session_id: &str,
    placement: BrowserPlacement,
) {
    state.host_bindings.insert(
        session_id.into(),
        BrowserHostTaskBinding {
            request: BrowserHostBinding {
                version: 1,
                host_id: "fixture".into(),
                participant_id: "participant".into(),
                session_id: "app-session".into(),
                placement,
            },
            desktop: desktop("original", 1),
        },
    );
}

#[test]
fn ordinary_continuation_and_recovery_keep_persisted_required_placement() {
    let mut state = BrowserSessionState::default();
    let mut fx = effects(&[("original", 1, true), ("other", 1, true)]);
    let opened = open(&mut state, &mut fx, None, 1).unwrap();
    retain_hosted_constraint(
        &mut state,
        &opened.session_id,
        BrowserPlacement::Required {
            desktop: binding("original", 1),
        },
    );
    let before = state.clone();
    fx.live = Some(false);
    fx.reconciled = Some(BrowserRecovery {
        launch: BrowserLaunch {
            process_identity: None,
            browser_id: "browser".into(),
            pid: 456,
            cdp_endpoint: "http://replacement".into(),
            desktop: Some(desktop("other", 1)),
        },
        target_ids: BTreeMap::new(),
    });
    assert!(open(&mut state, &mut fx, None, 2)
        .unwrap_err()
        .starts_with("browser_placement_conflict:"));
    assert_eq!(state, before);
    let catalog = catalog();
    assert!(BrowserSessionManager::new(
        &mut state,
        &catalog,
        &mut fx,
        BrowserSessionManagerConfig::default()
    )
    .recover_retained_browser(&opened.browser_id)
    .unwrap_err()
    .starts_with("browser_placement_conflict:"));
    assert_eq!(state, before);
    assert_eq!(fx.launched.len(), 1);
    assert_eq!(fx.mutations, 0);
}

#[test]
fn conflicting_shared_host_constraints_fail_before_effects() {
    let mut state = BrowserSessionState::default();
    let mut fx = effects(&[("original", 1, true), ("other", 1, true)]);
    let opened = open(&mut state, &mut fx, None, 1).unwrap();
    retain_hosted_constraint(
        &mut state,
        &opened.session_id,
        BrowserPlacement::Required {
            desktop: binding("original", 1),
        },
    );
    let mut peer = state.sessions[&opened.session_id].clone();
    peer.id = "peer".into();
    peer.name = "peer".into();
    state.sessions.insert(peer.id.clone(), peer);
    retain_hosted_constraint(
        &mut state,
        "peer",
        BrowserPlacement::Required {
            desktop: binding("other", 1),
        },
    );
    let before = state.clone();
    assert!(open(&mut state, &mut fx, None, 2)
        .unwrap_err()
        .starts_with("browser_placement_conflict:"));
    assert_eq!(state, before);
    assert_eq!(fx.launched.len(), 1);
    assert_eq!(fx.mutations, 0);
}

#[test]
fn authorized_preferred_recovery_updates_actual_desktop_without_erasing_intent() {
    let mut state = BrowserSessionState::default();
    let mut fx = effects(&[("original", 1, true), ("other", 1, true)]);
    let opened = open(&mut state, &mut fx, None, 1).unwrap();
    let placement = BrowserPlacement::Preferred {
        desktop: binding("original", 1),
        alternatives: vec![binding("other", 1)],
    };
    retain_hosted_constraint(&mut state, &opened.session_id, placement.clone());
    fx.live = Some(false);
    fx.reconciled = Some(BrowserRecovery {
        launch: BrowserLaunch {
            process_identity: None,
            browser_id: "browser".into(),
            pid: 456,
            cdp_endpoint: "http://replacement".into(),
            desktop: Some(desktop("other", 1)),
        },
        target_ids: BTreeMap::new(),
    });
    let result = open(&mut state, &mut fx, None, 2).unwrap();
    assert_eq!(result.session_id, opened.session_id);
    assert_eq!(
        state.host_bindings[&opened.session_id].request.placement,
        placement
    );
    assert_eq!(
        state.host_bindings[&opened.session_id].desktop,
        desktop("other", 1)
    );
    assert_eq!(
        state.browsers[&opened.browser_id].desktop,
        Some(desktop("other", 1))
    );
}
