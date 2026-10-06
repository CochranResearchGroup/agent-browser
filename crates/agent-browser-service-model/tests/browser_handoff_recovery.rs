use agent_browser_service_model::*;
use serde_json::Value;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Default)]
struct Evidence {
    pending: Option<BrowserLaunchCustodyRecord>,
    effects: Vec<String>,
    prior: Option<RemoteViewAssignmentRecord>,
}
struct Store(Rc<RefCell<Evidence>>);
impl BrowserLaunchCustodyStore for Store {
    fn unpublished_launch_records(
        &mut self,
    ) -> Result<Vec<BrowserLaunchCustodyRecord>, LaunchCustodyStoreError> {
        Ok(self.0.borrow().pending.iter().cloned().collect())
    }
    fn published_launch_assignment(
        &mut self,
        _: &ManagedBrowserInstance,
    ) -> Result<RemoteViewAssignmentRecord, LaunchCustodyStoreError> {
        Ok(self.0.borrow().prior.clone().unwrap())
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
            RemoteViewApplicationRequest::ObserveAssignment { .. } => {
                self.fixture["assignmentObservation"].clone()
            }
            RemoteViewApplicationRequest::LaunchEnvironment { .. } => {
                self.fixture["launchEnvironment"].clone()
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
        if self.absent {
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
        _: &ManagedBrowserTab,
    ) -> Result<(), String> {
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
            assert_eq!(
                effects.begin_operation(&state).unwrap_err(),
                "remote_view_session_launch_readback_required"
            );
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
            assert!(RemoteViewSessionEffects::new(
                Runtime {
                    evidence: evidence.clone(),
                    absent,
                    fail_restoration,
                    next_target: 0
                },
                RemoteViewApplicationAdapter::new(
                    "agent-browser".into(),
                    Transport {
                        evidence: evidence.clone(),
                        fixture: f
                    }
                )
                .unwrap(),
                Store(evidence.clone()),
                vec![],
                || "44444444-4444-4444-4444-444444444444".into()
            )
            .is_err());
        }
    }
}
