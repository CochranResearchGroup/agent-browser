use agent_browser_service_model::*;
use serde_json::Value;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Default)]
struct Evidence {
    pending: Option<BrowserLaunchCustodyRecord>,
    effects: Vec<String>,
    prior: Option<RemoteViewAssignmentRecord>,
    retirement_blocked: bool,
    allow_task_tab_retirement: bool,
    published_browser_requests: Vec<ManagedBrowserInstance>,
    retirements: Vec<BrowserIdleRetirementRecord>,
    missing_identities: std::collections::BTreeSet<String>,
    assignments: BTreeMap<String, RemoteViewAssignmentRecord>,
    original_ended: BTreeMap<String, Result<bool, String>>,
}
struct Store(Rc<RefCell<Evidence>>);
impl BrowserIdleRetirementCustodyStore for Store {
    fn idle_retirement_records(
        &mut self,
    ) -> Result<Vec<BrowserIdleRetirementRecord>, BrowserIdleRetirementStoreError> {
        Ok(self.0.borrow().retirements.clone())
    }
    fn claim_idle_retirement(
        &mut self,
        intent: &BrowserIdleRetirementIntent,
        state: &BrowserSessionState,
    ) -> Result<BrowserIdleRetirementAdmission, BrowserIdleRetirementStoreError> {
        intent.matches_baseline(state)?;
        for scope in &intent.scopes {
            if self
                .0
                .borrow()
                .assignments
                .get(&scope.browser.id)
                .is_some_and(|assignment| assignment != &intent.provider.assignment)
            {
                return Err(BrowserIdleRetirementStoreError::Conflict);
            }
        }
        self.0
            .borrow_mut()
            .retirements
            .push(BrowserIdleRetirementRecord::new(intent.clone())?);
        Ok(BrowserIdleRetirementAdmission::New)
    }
    fn admit_idle_retirement_preparation(
        &mut self,
        intent: &BrowserIdleRetirementIntent,
    ) -> Result<(BrowserIdleRetirementRecord, bool), BrowserIdleRetirementStoreError> {
        let mut evidence = self.0.borrow_mut();
        let current = evidence
            .retirements
            .iter_mut()
            .find(|record| record.intent == *intent)
            .unwrap();
        let admitted = !current.preparation_started();
        if admitted {
            current.begin_preparation()?;
        }
        Ok((current.clone(), admitted))
    }
    fn update_idle_retirement(
        &mut self,
        previous: &BrowserIdleRetirementRecord,
        next: &BrowserIdleRetirementRecord,
    ) -> Result<(), BrowserIdleRetirementStoreError> {
        previous.validate_successor(next)?;
        let mut evidence = self.0.borrow_mut();
        let current = evidence
            .retirements
            .iter_mut()
            .find(|r| r.intent.id == previous.intent.id)
            .unwrap();
        assert_eq!(current, previous);
        *current = next.clone();
        Ok(())
    }
    fn publish_idle_retirement_state(
        &mut self,
        _: &BrowserSessionState,
        _: &BrowserSessionState,
        _: &[String],
    ) -> Result<(), BrowserIdleRetirementStoreError> {
        Err(BrowserIdleRetirementStoreError::Unavailable)
    }
}
impl BrowserLaunchCustodyStore for Store {
    fn published_launch_process_identity(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<Option<RecordedProcessIdentity>, LaunchCustodyStoreError> {
        if self.0.borrow().missing_identities.contains(&browser.id) {
            return Ok(None);
        }
        Ok(Some(RecordedProcessIdentity {
            pid: browser.pid,
            start_token: "synthetic-original".into(),
            executable_path: Some("/synthetic/chrome".into()),
            browser_family: None,
        }))
    }

    fn unpublished_launch_records(
        &mut self,
    ) -> Result<Vec<BrowserLaunchCustodyRecord>, LaunchCustodyStoreError> {
        Ok(self.0.borrow().pending.iter().cloned().collect())
    }
    fn published_launch_assignment(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<RemoteViewAssignmentRecord, LaunchCustodyStoreError> {
        let mut evidence = self.0.borrow_mut();
        evidence.published_browser_requests.push(browser.clone());
        evidence
            .assignments
            .get(&browser.id)
            .cloned()
            .or_else(|| evidence.prior.clone())
            .ok_or(LaunchCustodyStoreError::Conflict)
    }
    fn admit_recovery_launch_intent(
        &mut self,
        intent: &BrowserLaunchIntent,
        expected: &BrowserSessionState,
        browser: &ManagedBrowserInstance,
    ) -> Result<LaunchCustodyAdmission, LaunchCustodyStoreError> {
        assert_eq!(expected.browsers.get(&browser.id), Some(browser));
        let mut e = self.0.borrow_mut();
        assert!(e.pending.is_none());
        e.effects.push("claim".into());
        e.pending = Some(BrowserLaunchCustodyRecord::pending(intent.clone()).unwrap());
        Ok(LaunchCustodyAdmission::New)
    }
    fn admit_launch_intent(
        &mut self,
        _: &BrowserLaunchIntent,
        _: &BrowserSessionState,
    ) -> Result<LaunchCustodyAdmission, LaunchCustodyStoreError> {
        panic!("recovery must use exact replacement admission")
    }
    fn observe_launch_intent(
        &mut self,
        _: &BrowserLaunchIntent,
        launch: &BrowserLaunch,
    ) -> Result<(), LaunchCustodyStoreError> {
        let mut e = self.0.borrow_mut();
        e.effects.push("record".into());
        e.pending.as_mut().unwrap().observe(launch).unwrap();
        Ok(())
    }
    fn publish_launch_intent(
        &mut self,
        _: &BrowserLaunchIntent,
        _: &BrowserSessionState,
        _: &BrowserSessionState,
    ) -> Result<(), LaunchCustodyStoreError> {
        panic!("host owns atomic publication")
    }
}
impl RemoteViewApplicationMutationStore for Store {
    fn read(
        &mut self,
        _: &RemoteViewApplicationEnvelope,
    ) -> Result<Option<RemoteViewApplicationMutationRecord>, RemoteViewApplicationMutationStoreError>
    {
        panic!("no viewer mutation during browser recovery")
    }
    fn claim(
        &mut self,
        _: &RemoteViewApplicationEnvelope,
    ) -> Result<RemoteViewApplicationMutationClaim, RemoteViewApplicationMutationStoreError> {
        panic!("no new assignment during browser recovery")
    }
    fn complete(
        &mut self,
        _: &RemoteViewApplicationEnvelope,
        _: &RemoteViewApplicationMutationOutcome,
    ) -> Result<(), RemoteViewApplicationMutationStoreError> {
        panic!("no new assignment during browser recovery")
    }
}
impl RemoteViewPoolRequestStore for Store {
    fn pool_assignment_available(
        &mut self,
        _: &RemoteViewAssignmentRecord,
    ) -> Result<bool, RemoteViewApplicationMutationStoreError> {
        panic!("retained desktop needs no placement")
    }
    fn confirmed_pool_acquisitions(
        &mut self,
        _: &str,
        _: &str,
    ) -> Result<Vec<RemoteViewAssignmentRecord>, RemoteViewApplicationMutationStoreError> {
        panic!("retained desktop needs no acquisition")
    }
    fn pending_pool_acquisitions(
        &mut self,
        _: &str,
        _: &str,
    ) -> Result<Vec<RemoteViewApplicationMutationRecord>, RemoteViewApplicationMutationStoreError>
    {
        panic!("retained desktop needs no acquisition")
    }
    fn pool_acquisition_request_key(
        &mut self,
        _: &str,
        _: &str,
        _: &[RemoteViewAssignmentRecord],
    ) -> Result<String, RemoteViewApplicationMutationStoreError> {
        panic!("retained desktop needs no acquisition")
    }
}
struct Transport {
    evidence: Rc<RefCell<Evidence>>,
    fixture: Value,
}
impl RemoteViewApplicationTransport for Transport {
    fn request(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<Value, RemoteViewApplicationTransportError> {
        assert!(
            self.evidence.borrow().pending.is_none(),
            "read-only recovery preflight must precede process custody"
        );
        Ok(match envelope.request {
            RemoteViewApplicationRequest::Inventory {} => self.fixture["inventory"].clone(),
            RemoteViewApplicationRequest::ObserveAssignment { .. } => {
                self.fixture["assignmentObservation"].clone()
            }
            RemoteViewApplicationRequest::LaunchEnvironment { .. } => {
                self.fixture["launchEnvironment"].clone()
            }
            RemoteViewApplicationRequest::ReadIdleRetirement { .. } => serde_json::json!({
                "schemaVersion":1,"target":self.fixture["assignmentObservation"]["target"],
                "readinessScope":"native_admission","retirement":{"revision":0,"fence":null,"completed":true}
            }),
            RemoteViewApplicationRequest::PrepareIdleRetirement {
                ref idempotency_key,
                ref expected_retirement,
                expected_revision,
                ..
            } => {
                use sha2::{Digest, Sha256};
                let target: RemoteViewApplicationTarget =
                    serde_json::from_value(self.fixture["assignmentObservation"]["target"].clone())
                        .unwrap();
                let key = format!(
                    "consumer-{:x}",
                    Sha256::digest(
                        format!("{}:{}", target.registration_id, idempotency_key).as_bytes()
                    )
                );
                let operation = format!(
                    "{:x}",
                    Sha256::digest(
                        serde_json::to_vec(&(key, &target, expected_retirement)).unwrap()
                    )
                );
                serde_json::json!({"schemaVersion":1,"target":target,"readinessScope":"native_admission","state":"prepared",
                    "fence":{"schema_version":1,"installation_id":"0123456789abcdef","config_hash":"a".repeat(64),
                        "slot":2,"generation":target.viewing_generation,"operation_id":operation,"revision":expected_revision + 1}})
            }
            RemoteViewApplicationRequest::ObserveIdle { .. } => {
                self.evidence
                    .borrow_mut()
                    .effects
                    .push("provider_idle_check".into());
                let mut observation = self.fixture["assignmentObservation"].clone();
                observation["viewerActive"] = false.into();
                observation["idle"] = true.into();
                let assignment = self.evidence.borrow().prior.clone().unwrap();
                observation["retirement"] = serde_json::json!({
                    "schemaVersion":1,"desktopId":assignment.desktop_id,
                    "generation":assignment.generation,"routes":[],"sessions":[]
                });
                observation
            }
            _ => panic!("unexpected provider effect"),
        })
    }
}
struct Runtime {
    evidence: Rc<RefCell<Evidence>>,
    absent: bool,
    fail_restoration: bool,
    next_target: u32,
}
impl RemoteViewBrowserProcessEffects for Runtime {
    fn prove_recovery_absence(
        &mut self,
        _: &ManagedBrowserInstance,
        _: &BrowserProfileCatalogEntry,
    ) -> Result<(), RemoteViewBrowserProcessError> {
        self.evidence.borrow_mut().effects.push("absence".into());
        if self.absent
            && self
                .evidence
                .borrow()
                .pending
                .as_ref()
                .is_none_or(|r| r.observed_pid.is_none())
        {
            Ok(())
        } else {
            Err(RemoteViewBrowserProcessError::Rejected)
        }
    }
    fn recover_launch(
        &mut self,
        browser: &ManagedBrowserInstance,
        _: &BrowserProfileCatalogEntry,
        intent: &BrowserLaunchIntent,
        environment: RemoteViewPrivateLaunchEnvironment,
        observation: &RemoteViewAssignmentObservation,
    ) -> Result<BrowserLaunch, RemoteViewBrowserProcessError> {
        assert!(self.evidence.borrow().pending.is_some());
        environment
            .into_environment(observation, &intent.assignment)
            .unwrap();
        self.evidence.borrow_mut().effects.push("launch".into());
        Ok(BrowserLaunch {
            process_identity: None,
            browser_id: browser.id.clone(),
            pid: 43,
            cdp_endpoint: "http://127.0.0.1:9322".into(),
            desktop: browser.desktop.clone(),
        })
    }
    fn launch(
        &mut self,
        _: &BrowserProfileCatalogEntry,
        _: &BrowserLaunchIntent,
        _: RemoteViewPrivateLaunchEnvironment,
        _: &RemoteViewAssignmentObservation,
    ) -> Result<BrowserLaunch, RemoteViewBrowserProcessError> {
        panic!("recovery cannot launch an unrelated logical browser")
    }
}
impl BrowserSessionEffects for Runtime {
    fn observe_idle_retirement_effect(
        &mut self,
        scope: &BrowserIdleRetirementScope,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<bool, String> {
        assert!(
            tab.is_none(),
            "historical qualification must not observe or close a target"
        );
        assert_eq!(
            scope.process_identity.as_ref().unwrap().pid,
            scope.browser.pid
        );
        self.evidence
            .borrow()
            .original_ended
            .get(&scope.browser.id)
            .cloned()
            .unwrap_or_else(|| Err("synthetic observation unavailable".into()))
    }

    fn permits_idle_browser_close(&mut self, _: &ManagedBrowserInstance) -> Result<bool, String> {
        self.evidence
            .borrow_mut()
            .effects
            .push("runtime_retirement_check".into());
        Ok(!self.evidence.borrow().retirement_blocked)
    }
    fn browser_is_live(&mut self, _: &ManagedBrowserInstance) -> Result<bool, String> {
        Ok(false)
    }
    fn launch_browser(
        &mut self,
        _: &BrowserProfileCatalogEntry,
        _: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String> {
        panic!("ordinary launch is not recovery")
    }
    fn close_browser(&mut self, _: &ManagedBrowserInstance) -> Result<(), String> {
        panic!("recovery cannot terminate a browser")
    }
    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        _: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        self.create_tab(browser)
    }
    fn create_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<BrowserTabAcquisition, String> {
        assert_eq!(browser.pid, 43);
        self.next_target += 1;
        Ok(BrowserTabAcquisition {
            tab_id: format!("physical-{}", self.next_target),
            target_id: format!("physical-{}", self.next_target),
            source: BrowserTabSource::ExplicitNew,
        })
    }
    fn navigate(
        &mut self,
        _: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        url: &str,
    ) -> Result<(), String> {
        self.evidence
            .borrow_mut()
            .effects
            .push(format!("restore:{}:{url}", tab.id));
        if self.fail_restoration {
            Err("restoration_failed".into())
        } else {
            Ok(())
        }
    }
    fn close_tab(
        &mut self,
        _: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
    ) -> Result<(), String> {
        if self.evidence.borrow().allow_task_tab_retirement {
            self.evidence
                .borrow_mut()
                .effects
                .push(format!("close:{}", tab.id));
            return Ok(());
        }
        panic!("recovery cannot close retained tabs")
    }
    fn focus_browser(
        &mut self,
        _: &ManagedBrowserInstance,
        _: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        panic!("presentation is qualified after publication")
    }
    fn allocate_disposable_profile(
        &mut self,
        _: &BrowserDisposableProfilePolicy,
        _: &str,
        _: &str,
    ) -> Result<BrowserProfileCatalogEntry, String> {
        panic!("profile identity is retained")
    }
    fn delete_disposable_profile(&mut self, _: &ManagedDisposableProfile) -> Result<(), String> {
        panic!("profile cannot be deleted")
    }
}

#[test]
fn recovery_restores_peer_tabs_and_failed_restoration_keeps_unpublished_custody() {
    for (absent, fail_restoration) in [(false, false), (true, true), (true, false)] {
        let f: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let assignment: RemoteViewAssignmentRecord =
            serde_json::from_value(f["assignment"].clone()).unwrap();
        let evidence = Rc::new(RefCell::new(Evidence {
            prior: Some(assignment.clone()),
            ..Default::default()
        }));
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "profile-a".into(),
            name: "Synthetic".into(),
            user_data_dir: "/synthetic/profile-a".into(),
            kind: BrowserProfileKind::Named,
        };
        let browser = ManagedBrowserInstance {
            id: "browser-a".into(),
            profile_id: profile.id.clone(),
            pid: 42,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: assignment.desktop_id,
                generation: assignment.generation,
                friendly_route_label: "synthetic".into(),
            }),
            active_session_ids: vec!["alice".into(), "bob".into()],
        };
        let tabs = ["alice", "bob"].map(|id| ManagedBrowserTab {
            id: format!("tab-{id}"),
            target_id: format!("old-{id}"),
            browser_id: browser.id.clone(),
            session_id: id.into(),
            created_at_ms: 1,
            last_activity_at_ms: 1,
        });
        let navigation = [1, 2].map(|visited_at_ms| BrowserNavigationRecord {
            profile_id: profile.id.clone(),
            session_id: "alice".into(),
            browser_id: browser.id.clone(),
            tab_id: "tab-alice".into(),
            target_id: "old-alice".into(),
            url: format!("https://example.test/{visited_at_ms}"),
            visited_at_ms,
            incident_ids: vec![],
        });
        let state = BrowserSessionState {
            browsers: BTreeMap::from([(browser.id.clone(), browser.clone())]),
            ..Default::default()
        };
        let mut effects = RemoteViewSessionEffects::new(
            Runtime {
                evidence: evidence.clone(),
                absent,
                fail_restoration,
                next_target: 0,
            },
            RemoteViewApplicationAdapter::new(
                "agent-browser".into(),
                Transport {
                    evidence: evidence.clone(),
                    fixture: f.clone(),
                },
            )
            .unwrap(),
            Store(evidence.clone()),
            vec![],
            || "33333333-3333-3333-3333-333333333333".into(),
        )
        .unwrap();
        effects.begin_operation(&state).unwrap();
        let outcome = effects.recover_browser(&browser, &profile, &tabs, &navigation);
        if !absent {
            assert_eq!(
                outcome.unwrap_err(),
                "browser_session_recovery_absence_unproven"
            );
            assert!(evidence.borrow().pending.is_none());
            assert_eq!(evidence.borrow().effects, ["absence"]);
        } else {
            assert!(!evidence.borrow().pending.as_ref().unwrap().published);
            assert_eq!(
                evidence.borrow().pending.as_ref().unwrap().observed_pid,
                Some(43)
            );
            if fail_restoration {
                assert_eq!(outcome.unwrap_err(), "restoration_failed");
                assert!(effects.pending_launch_intent().is_none());
            } else {
                let recovered = outcome.unwrap();
                assert_eq!(recovered.launch.browser_id, browser.id);
                assert_eq!(
                    recovered.target_ids,
                    BTreeMap::from([
                        ("tab-alice".into(), "physical-1".into()),
                        ("tab-bob".into(), "physical-2".into())
                    ])
                );
                assert!(effects.pending_launch_intent().is_some());
                assert_eq!(
                    evidence.borrow().effects,
                    [
                        "absence",
                        "claim",
                        "launch",
                        "record",
                        "restore:tab-alice:https://example.test/2",
                        "restore:tab-bob:about:blank"
                    ]
                );
            }
            // Interrupted bookkeeping does not poison the next ordinary task.
            effects.begin_operation(&state).unwrap();
            assert!(effects.pending_launch_intent().is_none());
            let mut restarted = RemoteViewSessionEffects::new(
                Runtime {
                    evidence: evidence.clone(),
                    absent,
                    fail_restoration,
                    next_target: 0,
                },
                RemoteViewApplicationAdapter::new(
                    "agent-browser".into(),
                    Transport {
                        evidence: evidence.clone(),
                        fixture: f,
                    },
                )
                .unwrap(),
                Store(evidence.clone()),
                vec![],
                || "44444444-4444-4444-4444-444444444444".into(),
            )
            .unwrap();
            restarted.begin_operation(&state).unwrap();
            assert_eq!(
                restarted
                    .recover_browser(&browser, &profile, &tabs, &navigation)
                    .unwrap_err(),
                "browser_session_recovery_absence_unproven"
            );
        }
    }
}

#[test]
fn pooled_idle_retirement_preserves_runtime_controller_and_operation_veto() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap();
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let evidence = Rc::new(RefCell::new(Evidence {
        prior: Some(assignment.clone()),
        retirement_blocked: true,
        ..Default::default()
    }));
    let browser = ManagedBrowserInstance {
        id: "browser-a".into(),
        profile_id: "profile-a".into(),
        pid: 42,
        cdp_endpoint: "http://127.0.0.1:9222".into(),
        desktop: Some(RemoteViewFixedDesktop {
            desktop_id: assignment.desktop_id.clone(),
            generation: assignment.generation,
            friendly_route_label: "synthetic".into(),
        }),
        active_session_ids: vec![],
    };
    let state = BrowserSessionState {
        browsers: BTreeMap::from([(browser.id.clone(), browser.clone())]),
        ..Default::default()
    };
    let mut effects = RemoteViewSessionEffects::new(
        Runtime {
            evidence: evidence.clone(),
            absent: false,
            fail_restoration: false,
            next_target: 0,
        },
        RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            Transport {
                evidence: evidence.clone(),
                fixture,
            },
        )
        .unwrap(),
        Store(evidence.clone()),
        vec![],
        || "33333333-3333-3333-3333-333333333333".into(),
    )
    .unwrap()
    .with_pool(RemoteViewSessionPool {
        name: "synthetic".into(),
        desired_desktops: 1,
    })
    .unwrap();
    effects.begin_operation(&state).unwrap();
    assert!(
        !effects.permits_idle_browser_close(&browser).unwrap(),
        "provider idle observation must not bypass a runtime controller/operation veto"
    );
    assert_eq!(evidence.borrow().effects, vec!["runtime_retirement_check"]);
    evidence.borrow_mut().retirement_blocked = false;
    assert!(effects.permits_idle_browser_close(&browser).unwrap());
    assert_eq!(
        evidence.borrow().effects,
        vec![
            "runtime_retirement_check",
            "runtime_retirement_check",
            "provider_idle_check"
        ]
    );
    let mut in_progress = browser.clone();
    in_progress.active_session_ids.push("remaining-peer".into());
    let guard = effects
        .acquire_idle_retirement_guard(&[in_progress])
        .unwrap();
    assert!(
        guard.is_some(),
        "session bookkeeping must not invalidate stable physical custody"
    );
    assert_eq!(
        evidence.borrow().published_browser_requests.last(),
        Some(&browser)
    );
    drop(guard);
    let reads_before = evidence.borrow().published_browser_requests.len();
    for mutation in 0..4 {
        let mut changed = browser.clone();
        match mutation {
            0 => changed.pid += 1,
            1 => changed.cdp_endpoint.push_str("/changed"),
            2 => changed.profile_id.push_str("-changed"),
            _ => changed.desktop.as_mut().unwrap().generation += 1,
        }
        match effects.acquire_idle_retirement_guard(&[changed]) {
            Err(error) => assert_eq!(error, "browser_session_idle_custody_identity_changed"),
            Ok(_) => panic!("changed physical custody must not acquire retirement authority"),
        }
    }
    assert_eq!(
        evidence.borrow().published_browser_requests.len(),
        reads_before,
        "changed physical custody must fail before assignment lookup"
    );
}

#[test]
fn hosted_reconnect_explicit_generation_recovery_uses_fresh_target_without_acquisition_or_baseline_rewrite(
) {
    let mut f: Value = serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap();
    let old: RemoteViewAssignmentRecord = serde_json::from_value(f["assignment"].clone()).unwrap();
    let generation = old.generation + 1;
    f["assignment"]["generation"] = generation.into();
    f["inventory"]["assignments"] = serde_json::json!([f["assignment"]]);
    f["assignmentObservation"]["target"]["lifecycleGeneration"] = generation.into();
    f["launchEnvironment"]["target"]["lifecycleGeneration"] = generation.into();
    // Lifecycle generation changes while the independent viewer epoch can stay fixed.
    f["inventory"]["desktops"][0]["lifecycle"]["generation"] = generation.into();
    let target = RemoteViewFixedDesktop {
        desktop_id: old.desktop_id.clone(),
        generation,
        friendly_route_label: "explicit recovery".into(),
    };
    let profile = BrowserProfileCatalogEntry {
        manual_login_url: None,
        id: "profile-a".into(),
        name: "Synthetic".into(),
        user_data_dir: "/synthetic/profile-a".into(),
        kind: BrowserProfileKind::Named,
    };
    let browser = ManagedBrowserInstance {
        id: "browser-a".into(),
        profile_id: profile.id.clone(),
        pid: 42,
        cdp_endpoint: "http://127.0.0.1:9222".into(),
        desktop: Some(RemoteViewFixedDesktop {
            generation: old.generation,
            ..target.clone()
        }),
        active_session_ids: vec!["task".into()],
    };
    let tabs = [ManagedBrowserTab {
        id: "tab-task".into(),
        target_id: "old-target".into(),
        browser_id: browser.id.clone(),
        session_id: "task".into(),
        created_at_ms: 1,
        last_activity_at_ms: 1,
    }];
    let state = BrowserSessionState {
        browsers: BTreeMap::from([(browser.id.clone(), browser.clone())]),
        ..Default::default()
    };
    let evidence = Rc::new(RefCell::new(Evidence {
        prior: Some(old),
        ..Default::default()
    }));
    let mut effects = RemoteViewSessionEffects::new(
        Runtime {
            evidence: evidence.clone(),
            absent: true,
            fail_restoration: false,
            next_target: 0,
        },
        RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            Transport {
                evidence: evidence.clone(),
                fixture: f,
            },
        )
        .unwrap(),
        Store(evidence.clone()),
        vec![],
        || "55555555-5555-4555-8555-555555555555".into(),
    )
    .unwrap()
    .with_pool(RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    })
    .unwrap();
    effects.begin_operation(&state).unwrap();
    let recovered = effects
        .recover_browser_at(&browser, &profile, &tabs, &[], &target)
        .unwrap();
    assert_eq!(recovered.launch.desktop, Some(target));
    assert_eq!(recovered.launch.browser_id, browser.id);
    assert_eq!(state.browsers[&browser.id], browser);
    let e = evidence.borrow();
    assert!(
        e.published_browser_requests.is_empty(),
        "new recovery target must not use substituted old custody proof"
    );
    assert_eq!(
        e.pending.as_ref().unwrap().intent.assignment.generation,
        generation
    );
    assert_eq!(
        e.effects,
        [
            "absence",
            "claim",
            "launch",
            "record",
            "restore:tab-task:about:blank"
        ]
    );
}

#[test]
fn unresolved_initial_launch_readback_never_authorizes_implicit_second_launch() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap();
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let evidence = Rc::new(RefCell::new(Evidence {
        pending: Some(
            BrowserLaunchCustodyRecord::pending(BrowserLaunchIntent {
                intent_id: "66666666-6666-4666-8666-666666666666".into(),
                profile_id: "profile-a".into(),
                assignment,
            })
            .unwrap(),
        ),
        ..Default::default()
    }));
    let mut effects = RemoteViewSessionEffects::new(
        Runtime {
            evidence: evidence.clone(),
            absent: false,
            fail_restoration: false,
            next_target: 0,
        },
        RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            Transport {
                evidence: evidence.clone(),
                fixture,
            },
        )
        .unwrap(),
        Store(evidence.clone()),
        vec![],
        || "unused-new-launch".into(),
    )
    .unwrap();
    effects
        .begin_operation(&BrowserSessionState::default())
        .unwrap();
    let profile = BrowserProfileCatalogEntry {
        manual_login_url: None,
        id: "profile-a".into(),
        name: "Independent destination".into(),
        user_data_dir: "/synthetic/destination".into(),
        kind: BrowserProfileKind::Named,
    };
    assert_eq!(
        effects
            .reconcile_profile_browser(&profile, None, &[], &[])
            .unwrap_err(),
        "remote_view_session_launch_readback_required"
    );
    assert!(evidence.borrow().effects.is_empty());
    assert_eq!(
        evidence.borrow().pending.as_ref().unwrap().intent.intent_id,
        "66666666-6666-4666-8666-666666666666"
    );
    let peer = BrowserProfileCatalogEntry {
        id: "unrelated-profile".into(),
        ..profile
    };
    assert!(effects
        .reconcile_profile_browser(&peer, None, &[], &[])
        .unwrap()
        .is_none());
}

#[test]
fn borrowed_tab_lifetime_does_not_require_native_browser_launch_custody() {
    for peer_uses_ttl in [false, true] {
        for blocked in [false, true] {
            let evidence = Rc::new(RefCell::new(Evidence {
                retirement_blocked: blocked,
                allow_task_tab_retirement: true,
                ..Default::default()
            }));
            let browser = ManagedBrowserInstance {
                id: "borrowed-browser".into(),
                profile_id: "borrowed-profile".into(),
                pid: 43,
                cdp_endpoint: "http://127.0.0.1:9222".into(),
                desktop: None,
                active_session_ids: vec!["task".into(), "peer-task".into()],
            };
            let tab = ManagedBrowserTab {
                id: "own-tab".into(),
                target_id: "own-target".into(),
                browser_id: browser.id.clone(),
                session_id: "task".into(),
                created_at_ms: 100,
                last_activity_at_ms: 100,
            };
            let session = ManagedBrowserSession {
                id: "task".into(),
                name: "task".into(),
                profile_id: browser.profile_id.clone(),
                browser_id: browser.id.clone(),
                created_at_ms: 100,
                last_activity_at_ms: 100,
                expires_at_ms: 300_100,
                current_tab_id: Some(tab.id.clone()),
            };
            let mut state = BrowserSessionState::default();
            state.browsers.insert(browser.id.clone(), browser.clone());
            state.tabs.insert(tab.id.clone(), tab.clone());
            let peer_tab = ManagedBrowserTab {
                id: "peer-tab".into(),
                target_id: "peer-target".into(),
                session_id: "peer-task".into(),
                ..tab.clone()
            };
            let peer_session = ManagedBrowserSession {
                id: "peer-task".into(),
                name: "peer-task".into(),
                expires_at_ms: if peer_uses_ttl { 300_100 } else { 129 },
                current_tab_id: Some(peer_tab.id.clone()),
                ..session.clone()
            };
            state.sessions.insert(session.id.clone(), session);
            state.sessions.insert(peer_session.id.clone(), peer_session);
            state.tabs.insert(peer_tab.id.clone(), peer_tab.clone());
            if peer_uses_ttl {
                state.tab_lifetimes.insert(
                    peer_tab.id.clone(),
                    BrowserTabLifetime {
                        inactivity_ms: Some(30),
                        pressure_inactivity_ms: None,
                    },
                );
            }
            state.shared_browser_ids.insert(browser.id.clone());
            state.tab_lifetimes.insert(
                tab.id.clone(),
                BrowserTabLifetime {
                    inactivity_ms: Some(30),
                    pressure_inactivity_ms: None,
                },
            );
            let mut effects = RemoteViewSessionEffects::new(
                Runtime {
                    evidence: evidence.clone(),
                    absent: false,
                    fail_restoration: false,
                    next_target: 0,
                },
                RemoteViewApplicationAdapter::new(
                    "agent-browser".into(),
                    Transport {
                        evidence: evidence.clone(),
                        fixture: serde_json::json!({}),
                    },
                )
                .unwrap(),
                Store(evidence.clone()),
                vec![],
                || "55555555-5555-4555-8555-555555555555".into(),
            )
            .unwrap()
            .with_pool(RemoteViewSessionPool {
                name: "native".into(),
                desired_desktops: 1,
            })
            .unwrap();
            effects.begin_operation(&state).unwrap();
            let catalog = BrowserProfileCatalog {
                schema_version: "agent-browser.browser-profile-catalog.v1".into(),
                profiles: BTreeMap::new(),
                disposable_policies: BTreeMap::new(),
            };
            let result = BrowserSessionManager::new(
                &mut state,
                &catalog,
                &mut effects,
                BrowserSessionManagerConfig {
                    session_idle_timeout_ms: 300_000,
                    remote_view_desktops: vec![],
                },
            )
            .reap(130)
            .unwrap();
            assert_eq!(state.tabs.contains_key(&tab.id), blocked, "task TTL must preserve authority but not require custody of the borrowed browser: {result:?}");
            assert_eq!(state.tabs.contains_key(&peer_tab.id), blocked);
            assert!(state.browsers.contains_key(&browser.id));
            assert!(result.deferred_browser_errors.is_empty());
            assert!(
                evidence.borrow().published_browser_requests.is_empty(),
                "borrowed task expiry cannot claim the host browser's native launch"
            );
            assert_eq!(
                evidence
                    .borrow()
                    .effects
                    .iter()
                    .filter(|e| e.starts_with("close:"))
                    .count(),
                2 * usize::from(!blocked)
            );
            if !blocked {
                assert_eq!(state.tab_history.len(), 2);
                assert!(result.closed_browser_ids.is_empty());
            }
        }
    }
}

// Public retirement admission with synthetic provider and durable-custody boundaries.
fn retirement_peer_fixture() -> (
    RemoteViewSessionEffects<Runtime, Transport, Store>,
    BrowserSessionState,
    ManagedBrowserInstance,
    Rc<RefCell<Evidence>>,
) {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap();
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let evidence = Rc::new(RefCell::new(Evidence {
        prior: Some(assignment.clone()),
        ..Default::default()
    }));
    let browser = ManagedBrowserInstance {
        id: "current".into(),
        profile_id: "current-profile".into(),
        pid: 42,
        cdp_endpoint: "ws://127.0.0.1:9222/devtools/browser/current".into(),
        desktop: Some(RemoteViewFixedDesktop {
            desktop_id: assignment.desktop_id.clone(),
            generation: assignment.generation,
            friendly_route_label: "synthetic".into(),
        }),
        active_session_ids: vec![],
    };
    let state = BrowserSessionState {
        browsers: BTreeMap::from([(browser.id.clone(), browser.clone())]),
        ..Default::default()
    };
    let effects = RemoteViewSessionEffects::new(
        Runtime {
            evidence: evidence.clone(),
            absent: false,
            fail_restoration: false,
            next_target: 0,
        },
        RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            Transport {
                evidence: evidence.clone(),
                fixture,
            },
        )
        .unwrap(),
        Store(evidence.clone()),
        vec![],
        || "33333333-3333-3333-3333-333333333333".into(),
    )
    .unwrap()
    .with_pool(RemoteViewSessionPool {
        name: "synthetic".into(),
        desired_desktops: 1,
    })
    .unwrap();
    (effects, state, browser, evidence)
}

#[test]
fn idle_closed_history_without_identity_does_not_block_current_retirement() {
    let (mut effects, mut state, browser, evidence) = retirement_peer_fixture();
    let mut old = browser.clone();
    old.id = "retired".into();
    old.pid = 41;
    state.browsers.insert(old.id.clone(), old.clone());
    state.idle_closed_browsers.insert(old.id.clone(), 1);
    evidence
        .borrow_mut()
        .missing_identities
        .insert(old.id.clone());
    effects.begin_operation(&state).unwrap();
    let guard = effects
        .acquire_idle_retirement_guard(std::slice::from_ref(&browser))
        .unwrap();
    assert!(
        guard.is_some(),
        "already retired history must not veto current cleanup"
    );
    let e = evidence.borrow();
    assert_eq!(e.retirements.len(), 1);
    assert_eq!(
        e.retirements[0]
            .intent
            .scopes
            .iter()
            .map(|s| s.browser.id.as_str())
            .collect::<Vec<_>>(),
        vec!["current"]
    );
    assert!(!e.published_browser_requests.iter().any(|b| b.id == old.id));
}

#[test]
fn ended_peer_from_previous_assignment_does_not_block_current_retirement() {
    let (mut effects, mut state, browser, evidence) = retirement_peer_fixture();
    let mut old = browser.clone();
    old.id = "old-assignment".into();
    old.pid = 41;
    old.profile_id = "old-profile".into();
    state.browsers.insert(old.id.clone(), old.clone());
    let mut previous = evidence.borrow().prior.clone().unwrap();
    previous.assignment_id = "historical-assignment".into();
    evidence
        .borrow_mut()
        .assignments
        .insert(old.id.clone(), previous);
    evidence
        .borrow_mut()
        .original_ended
        .insert(old.id.clone(), Ok(true));
    effects.begin_operation(&state).unwrap();
    let guard = effects
        .acquire_idle_retirement_guard(std::slice::from_ref(&browser))
        .expect("positively ended history must not create a mixed-assignment claim");
    assert!(guard.is_some());
    let e = evidence.borrow();
    assert_eq!(e.retirements.len(), 1);
    assert_eq!(
        e.retirements[0]
            .intent
            .scopes
            .iter()
            .map(|s| s.browser.id.as_str())
            .collect::<Vec<_>>(),
        vec!["current"]
    );
}

#[test]
fn live_or_uncertain_previous_assignment_remains_protected() {
    for observation in [Ok(false), Err("observation unavailable".into())] {
        let (mut effects, mut state, browser, evidence) = retirement_peer_fixture();
        let mut peer = browser.clone();
        peer.id = "previous-assignment".into();
        peer.pid = 41;
        state.browsers.insert(peer.id.clone(), peer.clone());
        let mut previous = evidence.borrow().prior.clone().unwrap();
        previous.assignment_id = "historical-assignment".into();
        evidence
            .borrow_mut()
            .assignments
            .insert(peer.id.clone(), previous);
        evidence
            .borrow_mut()
            .original_ended
            .insert(peer.id.clone(), observation);
        effects.begin_operation(&state).unwrap();
        assert!(effects
            .acquire_idle_retirement_guard(std::slice::from_ref(&browser))
            .is_err());
        assert!(
            evidence.borrow().retirements.is_empty(),
            "no native preparation or claim before positive original exit"
        );
    }
}

#[test]
fn current_assignment_peers_remain_in_frozen_retirement_scope() {
    let (mut effects, mut state, browser, evidence) = retirement_peer_fixture();
    let mut peer = browser.clone();
    peer.id = "peer".into();
    peer.pid = 41;
    state.browsers.insert(peer.id.clone(), peer.clone());
    effects.begin_operation(&state).unwrap();
    assert!(effects
        .acquire_idle_retirement_guard(std::slice::from_ref(&browser))
        .unwrap()
        .is_some());
    let e = evidence.borrow();
    assert_eq!(
        e.retirements[0]
            .intent
            .scopes
            .iter()
            .map(|s| s.browser.id.as_str())
            .collect::<Vec<_>>(),
        vec!["current", "peer"]
    );
}

#[test]
fn missing_identity_on_nonretired_peer_defers_with_a_reason() {
    let (mut effects, mut state, browser, evidence) = retirement_peer_fixture();
    let mut peer = browser.clone();
    peer.id = "legacy-live-or-unknown".into();
    peer.pid = 41;
    state.browsers.insert(peer.id.clone(), peer.clone());
    evidence.borrow_mut().missing_identities.insert(peer.id);
    effects.begin_operation(&state).unwrap();
    let error = match effects.acquire_idle_retirement_guard(std::slice::from_ref(&browser)) {
        Err(reason) => reason,
        Ok(_) => panic!("missing original identity must remain an explained deferral"),
    };
    assert_eq!(error, "browser_session_idle_original_identity_missing");
    assert!(evidence.borrow().retirements.is_empty());
}
