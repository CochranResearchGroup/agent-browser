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
    attachments: usize,
    wrong_process: bool,
    app_desktop: Option<String>,
}
impl BrowserSessionEffects for Effects {
    fn attach_app_browser(
        &mut self,
        request: &BrowserAttachmentRequest,
        _: &BrowserProfileCatalogEntry,
        desktop: &RemoteViewFixedDesktop,
    ) -> Result<BrowserAttachmentObservation, String> {
        self.attachments += 1;
        if self.app_desktop.as_ref().is_some_and(|id|id!=&desktop.desktop_id) {return Err("browser_attachment_desktop_process_mismatch".into());}
        Ok(BrowserAttachmentObservation {
            launch: BrowserLaunch {
                process_identity: Some(request.process.clone()),
                browser_id: "app-browser".into(),
                pid: if self.wrong_process {
                    999
                } else {
                    request.process.pid
                },
                cdp_endpoint: request.cdp_endpoint.clone(),
                desktop: Some(desktop.clone()),
            },
            target_id: request.target_id.clone(),
            assignment: None,
        })
    }

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

fn app_request() -> BrowserAttachmentRequest {
    BrowserAttachmentRequest {
        version: 1,
        process: RecordedProcessIdentity {
            pid: 4100,
            start_token: "linux:boot:100".into(),
            executable_path: Some("/fixture/chromium".into()),
            browser_family: Some("chromium".into()),
        },
        profile_path: "/fixture/profile".into(),
        cdp_endpoint: "ws://127.0.0.1:9222/devtools/browser/exact".into(),
        target_id: "original-page".into(),
        shared_with: Vec::new(),
    }
}
fn app_binding(participant: &str) -> BrowserHostBinding {
    BrowserHostBinding {
        version: 1,
        host_id: "app".into(),
        participant_id: participant.into(),
        session_id: "app-session".into(),
        placement: BrowserPlacement::Required {
            desktop: binding("one", 1),
        },
    }
}
fn attach(
    state: &mut BrowserSessionState,
    effects: &mut Effects,
    name: &str,
    binding: &BrowserHostBinding,
    request: &BrowserAttachmentRequest,
) -> Result<OpenBrowserSessionResult, String> {
    attach_app_browser_task(
        state,
        effects,
        &catalog().profiles["profile"],
        name,
        binding,
        request,
        10,
        100,
    )
}
#[test]
fn app_attachment_borrows_original_page_and_close_detaches_without_process_or_tab_effects() {
    let mut state = BrowserSessionState::default();
    let mut effects = effects(&[("one", 1, true)]);
    let owner = app_binding("owner");
    let request = app_request();
    let task = attach(&mut state, &mut effects, "task", &owner, &request).unwrap();
    assert_eq!(
        state.tabs.values().next().unwrap().target_id,
        "original-page"
    );
    assert_eq!(effects.mutations, 0);
    assert!(effects.launched.is_empty());
    let bytes = serde_json::to_vec(&state).unwrap();
    let mut restored: BrowserSessionState = serde_json::from_slice(&bytes).unwrap();
    let cfg = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 100,
        remote_view_desktops: vec![],
    };
    let cat = catalog();
    let mut manager = BrowserSessionManager::new(&mut restored, &cat, &mut effects, cfg);
    assert!(manager
        .close_current_tab(&task.session_id, 11)
        .unwrap_err()
        .starts_with("browser_attachment_borrowed_tab_close_forbidden"));
    manager
        .close_session(&task.session_id, SessionEndReason::ExplicitClose, 12)
        .unwrap();
    assert_eq!(effects.mutations, 0);
    assert!(restored.sessions.is_empty());
    assert!(restored.app_browser_attachments[&task.browser_id]
        .borrowed_tab_ids
        .contains("original-page"));
}
#[test]
fn app_attachment_sharing_requires_owner_permission_and_distinct_task_identities() {
    let mut state = BrowserSessionState::default();
    let mut effects = effects(&[("one", 1, true)]);
    let owner = app_binding("owner");
    let peer = app_binding("peer");
    let mut request = app_request();
    let first = attach(&mut state, &mut effects, "first", &owner, &request).unwrap();
    let before = state.clone();
    assert!(attach(&mut state, &mut effects, "second", &peer, &request)
        .unwrap_err()
        .starts_with("browser_attachment_sharing_required"));
    assert_eq!(state, before);
    request.shared_with.push(BrowserHostIdentity::from(&peer));
    attach(&mut state, &mut effects, "first", &owner, &request).unwrap();
    let second = attach(&mut state, &mut effects, "second", &peer, &request).unwrap();
    assert_ne!(first.session_id, second.session_id);
    assert_eq!(first.browser_id, second.browser_id);
    assert_ne!(
        state.sessions[&first.session_id].current_tab_id,
        state.sessions[&second.session_id].current_tab_id
    );
    let cat = catalog();
    BrowserSessionManager::new(
        &mut state,
        &cat,
        &mut effects,
        BrowserSessionManagerConfig::default(),
    )
    .close_session(&second.session_id, SessionEndReason::ExplicitClose, 13)
    .unwrap();
    assert!(state.sessions.contains_key(&first.session_id));
    assert_eq!(effects.mutations, 0);
}
#[test]
fn app_attachment_conflicts_and_unsupported_endpoints_have_zero_browser_effects() {
    let mut state = BrowserSessionState::default();
    let mut effects = effects(&[("one", 1, true)]);
    let owner = app_binding("owner");
    let request = app_request();
    attach(&mut state, &mut effects, "first", &owner, &request).unwrap();
    let before = state.clone();
    let calls = effects.attachments;
    let mut wrong = request.clone();
    wrong.process.start_token = "linux:boot:other".into();
    assert!(attach(&mut state, &mut effects, "wrong", &owner, &wrong)
        .unwrap_err()
        .starts_with("browser_attachment_profile_conflict"));
    let mut placement = owner.clone();
    placement.placement = BrowserPlacement::Required {
        desktop: binding("two", 1),
    };
    assert!(
        attach(&mut state, &mut effects, "wrong", &placement, &request)
            .unwrap_err()
            .starts_with("browser_attachment_placement_conflict")
    );
    for endpoint in [
        "http://127.0.0.1:9222",
        "ws://127.0.0.1:9222@other/devtools/browser/x",
        "ws://remote:9222/devtools/browser/x",
        "ws://127.0.0.1:9222/devtools/browser/x?secret=1",
    ] {
        let mut wrong = request.clone();
        wrong.cdp_endpoint = endpoint.into();
        assert!(wrong.validate().is_err());
    }
    assert_eq!(effects.attachments, calls);
    assert_eq!(state, before);
    assert_eq!(effects.mutations, 0);
}
#[test]
fn app_attachment_rejects_mismatched_observation_without_publishing_task() {
    let mut state = BrowserSessionState::default();
    let mut effects = effects(&[("one", 1, true)]);
    effects.wrong_process = true;
    assert!(attach(
        &mut state,
        &mut effects,
        "task",
        &app_binding("owner"),
        &app_request()
    )
    .unwrap_err()
    .starts_with("browser_attachment_observation_mismatch"));
    assert!(state.sessions.is_empty());
    assert!(state.browsers.is_empty());
    assert_eq!(effects.mutations, 0);
}
#[test]
fn app_attachment_cannot_be_implicitly_shared_or_recovered_by_ordinary_open() {
    let mut state = BrowserSessionState::default();
    let mut effects = effects(&[("one", 1, true)]);
    attach(
        &mut state,
        &mut effects,
        "task",
        &app_binding("owner"),
        &app_request(),
    )
    .unwrap();
    let cat = catalog();
    let cfg = BrowserSessionManagerConfig {
        session_idle_timeout_ms: 100,
        remote_view_desktops: vec![],
    };
    assert!(
        BrowserSessionManager::new(&mut state, &cat, &mut effects, cfg.clone())
            .open(OpenBrowserSession::exact_profile("unshared", "profile", 12))
            .unwrap_err()
            .starts_with("browser_attachment_sharing_required")
    );
    effects.live = Some(false);
    assert!(
        BrowserSessionManager::new(&mut state, &cat, &mut effects, cfg)
            .open(OpenBrowserSession::exact_profile("task", "profile", 13))
            .unwrap_err()
            .starts_with("browser_attachment_lost")
    );
    assert!(effects.launched.is_empty());
    assert_eq!(effects.mutations, 0);
}

#[test]
fn app_attachment_preferred_fallback_observes_existing_location_without_moving() {
    let mut state=BrowserSessionState::default();let mut effects=effects(&[("one",1,true),("two",2,true)]);effects.app_desktop=Some("two".into());
    let mut owner=app_binding("owner");owner.placement=BrowserPlacement::Preferred{desktop:binding("one",1),alternatives:vec![binding("two",2)]};
    let opened=attach(&mut state,&mut effects,"task",&owner,&app_request()).unwrap();
    assert_eq!(state.browsers[&opened.browser_id].desktop.as_ref().unwrap().desktop_id,"two");
    assert!(effects.launched.is_empty());assert_eq!(effects.mutations,0);
    // A required desktop cannot silently substitute even though a peer is ready.
    let mut state=BrowserSessionState::default();assert!(attach(&mut state,&mut effects,"required",&app_binding("owner"),&app_request()).is_err());assert!(state.sessions.is_empty());
}
