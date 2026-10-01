//! Capacity demand through the actual host, pool coordinator and SQLite owner.
use super::*;
use std::cell::RefCell;

#[derive(Default)]
struct Provider {
    assignments: Vec<RemoteViewAssignmentRecord>,
    acquisitions: Vec<String>,
    operations: Vec<&'static str>,
    lose_reply: bool,
    view_requests: Vec<String>,
    lose_view_reply: bool,
    competing_view_publication: bool,
    unavailable_assignment: Option<String>,
    empty_desktop: Option<String>,
}
struct PoolTransport {
    provider: Rc<RefCell<Provider>>,
    root: PathBuf,
}
impl RemoteViewApplicationTransport for PoolTransport {
    fn request(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<Value, RemoteViewApplicationTransportError> {
        let mut provider = self.provider.borrow_mut();
        assert_eq!(envelope.application, "agent-browser");
        let fixture = wire();
        match &envelope.request {
            RemoteViewApplicationRequest::Inventory {} => {
                provider.operations.push("inventory");
                let mut inventory = fixture["inventory"].clone();
                inventory["assignments"] = serde_json::to_value(&provider.assignments).unwrap();
                inventory["pools"][0]["desktopMembers"] = serde_json::json!(provider
                    .assignments
                    .iter()
                    .map(|assignment| &assignment.desktop_id)
                    .collect::<Vec<_>>());
                Ok(inventory)
            }
            RemoteViewApplicationRequest::Acquire {
                pool_name,
                idempotency_key,
            } => {
                assert_eq!(pool_name, "main");
                let mut store = open_store(&self.root, false);
                let pending = store
                    .pending_pool_acquisitions("agent-browser", "main")
                    .unwrap();
                assert_eq!(pending.len(), 1);
                assert_eq!(pending[0].envelope, *envelope);
                provider.operations.push("acquire");
                provider.acquisitions.push(idempotency_key.clone());
                let mut assignment: RemoteViewAssignmentRecord =
                    serde_json::from_value(fixture["assignment"].clone()).unwrap();
                if !provider.assignments.is_empty() {
                    assignment.assignment_id =
                        format!("assignment-extra-{}", provider.assignments.len());
                    assignment.desktop_id = uuid::Uuid::new_v4().to_string();
                }
                provider.assignments.push(assignment.clone());
                if provider.lose_reply {
                    return Err(RemoteViewApplicationTransportError::OutcomeUnknown);
                }
                Ok(serde_json::to_value(assignment).unwrap())
            }
            RemoteViewApplicationRequest::ObserveAssignment {
                assignment_id,
                expected_generation,
            } => {
                provider.operations.push("observe_assignment");
                if provider.unavailable_assignment.as_ref() == Some(assignment_id) {
                    return Err(RemoteViewApplicationTransportError::Unavailable);
                }
                let assignment = provider
                    .assignments
                    .iter()
                    .find(|assignment| assignment.assignment_id == *assignment_id)
                    .unwrap();
                assert_eq!(assignment.generation, *expected_generation);
                let mut observation = fixture["assignmentObservation"].clone();
                observation["target"]["assignmentId"] = assignment.assignment_id.clone().into();
                observation["target"]["desktopId"] = assignment.desktop_id.clone().into();
                Ok(observation)
            }
            RemoteViewApplicationRequest::Windows { assignment_id, .. } => {
                provider.operations.push("windows");
                let assignment = provider
                    .assignments
                    .iter()
                    .find(|assignment| assignment.assignment_id == *assignment_id)
                    .unwrap();
                let mut windows = fixture["windows"].clone();
                windows["desktopId"] = assignment.desktop_id.clone().into();
                if provider.empty_desktop.as_ref() == Some(&assignment.desktop_id) {
                    windows["windows"] = serde_json::json!([]);
                }
                Ok(windows)
            }
            RemoteViewApplicationRequest::LaunchEnvironment { .. } => {
                provider.operations.push("launch_environment");
                assert_eq!(provider.assignments.len(), 1);
                Ok(fixture["launchEnvironment"].clone())
            }
            RemoteViewApplicationRequest::IssueView {
                idempotency_key,
                audience,
                capability,
                lifetime_seconds,
                ..
            } => {
                assert_eq!(audience, "remote_view");
                assert_eq!(*capability, RemoteViewApplicationViewCapability::Control);
                assert_eq!(*lifetime_seconds, 300);
                let store = open_store(&self.root, false);
                assert!(store
                    .load_session_state()
                    .unwrap()
                    .remote_view_tab_handoffs
                    .values()
                    .any(|record| record
                        .view
                        .as_ref()
                        .is_some_and(|view| view.idempotency_key == *idempotency_key
                            && view.issuance.is_none())));
                provider.operations.push("issue_view");
                provider.view_requests.push(idempotency_key.clone());
                if provider.competing_view_publication {
                    competing_write(&self.root);
                }
                if provider.lose_view_reply {
                    return Err(RemoteViewApplicationTransportError::OutcomeUnknown);
                }
                let mut issuance = fixture["viewIssuance"].clone();
                issuance["grant"]["request"]["idempotencyKey"] = idempotency_key.clone().into();
                Ok(issuance)
            }
            RemoteViewApplicationRequest::ResolveView { route_id, audience } => {
                provider.operations.push("resolve_view");
                assert_eq!(audience, "remote_view");
                assert_eq!(route_id, fixture["grant"]["routeId"].as_str().unwrap());
                let mut grant = fixture["grant"].clone();
                grant["request"]["idempotencyKey"] =
                    provider.view_requests.last().unwrap().clone().into();
                Ok(grant)
            }
            _ => panic!("unexpected provider mutation during capacity demand"),
        }
    }
}
fn adapter(
    fixture: &Fixture,
    provider: Rc<RefCell<Provider>>,
) -> RemoteViewApplicationAdapter<PoolTransport> {
    RemoteViewApplicationAdapter::new(
        "agent-browser".into(),
        PoolTransport {
            provider,
            root: fixture.root.clone(),
        },
    )
    .unwrap()
}
fn host(
    fixture: &Fixture,
    provider: Rc<RefCell<Provider>>,
    calls: Rc<Cell<u32>>,
) -> BrowserSessionHost<
    BrowserSessionSqliteStore,
    RemoteViewSessionEffects<Process, PoolTransport, BrowserSessionSqliteStore>,
> {
    let effects = RemoteViewSessionEffects::new(
        Process {
            mode: Mode::Success,
            root: fixture.root.clone(),
            calls,
        },
        adapter(fixture, provider),
        fixture.store(false),
        Vec::new(),
        || uuid::Uuid::new_v4().to_string(),
    )
    .unwrap()
    .with_pool(RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    })
    .unwrap();
    BrowserSessionHost::load(
        fixture.store(false),
        effects,
        &fixture.root.join("unused.json"),
        BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: Vec::new(),
            default_disposable_policy: Some(BrowserDisposableProfilePolicy {
                id: "default".into(),
                user_data_root: fixture
                    .root
                    .join("disposable")
                    .to_string_lossy()
                    .into_owned(),
                cleanup_delay_ms: 300_000,
                maximum_retained_profiles: 20,
                maximum_total_bytes: 1024 * 1024,
            }),
            exact_url_history_maximum_bytes: 1024,
        },
    )
    .unwrap()
}

#[test]
fn pool_demand_empty_open_acquires_once_and_status_reuse_restart_do_not_allocate() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    assert!(provider.borrow().operations.is_empty());
    let status = consumer.handle_command(&serde_json::json!({"action":"browser_session_status"}));
    assert_eq!(status["success"], true);
    assert!(provider.borrow().operations.is_empty());
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert_eq!(calls.get(), 1);
    let requests = provider.borrow().operations.len();
    let reused = consumer
        .open(OpenBrowserSession::exact_profile("Bob", "profile-a", 101))
        .unwrap();
    assert_eq!(reused.browser_id, opened.browser_id);
    assert_eq!(provider.borrow().operations.len(), requests);
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    restarted
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 102))
        .unwrap();
    assert_eq!(provider.borrow().operations.len(), requests);
    assert_eq!(calls.get(), 1);
    restarted
        .open(OpenBrowserSession::exact_profile("Other", "profile-b", 103))
        .unwrap();
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert_eq!(calls.get(), 2);
    let mut store = fixture.store(false);
    assert!(store
        .pending_pool_acquisitions("agent-browser", "main")
        .unwrap()
        .is_empty());
    assert!(store.unpublished_launch_records().unwrap().is_empty());
}

#[test]
fn pool_demand_lost_acquire_reply_retains_custody_even_when_provider_assignment_is_visible() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        lose_reply: true,
        ..Provider::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    assert_eq!(
        consumer
            .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
            .unwrap_err(),
        "remote_view_pool_acquisition_readback_required"
    );
    let requests = provider.borrow().operations.len();
    assert_eq!(provider.borrow().assignments.len(), 1);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    assert_eq!(
        restarted
            .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 101))
            .unwrap_err(),
        "remote_view_pool_acquisition_readback_required"
    );
    assert_eq!(provider.borrow().operations.len(), requests);
    assert_eq!(calls.get(), 0);
    let mut store = fixture.store(false);
    assert_eq!(
        store
            .pending_pool_acquisitions("agent-browser", "main")
            .unwrap()
            .len(),
        1
    );
    assert!(store.unpublished_launch_records().unwrap().is_empty());
}

#[test]
fn pool_demand_policy_bounds_growth_and_completed_absence_requires_release_proof() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut adapter = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    assert_eq!(
        prepare_remote_view_session_pool(
            &mut adapter,
            &mut store,
            &RemoteViewSessionPool {
                name: "main".into(),
                desired_desktops: 3
            }
        )
        .err()
        .unwrap(),
        "remote_view_pool_demand_exceeds_policy"
    );
    assert!(provider.borrow().acquisitions.is_empty());
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 2,
    };
    let prepared = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(prepared.desktops.len(), 2);
    let keys = provider.borrow().acquisitions.clone();
    assert_eq!(keys.len(), 2);
    assert_ne!(keys[0], keys[1]);
    drop(store);
    let mut store = fixture.store(false);
    prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(provider.borrow().acquisitions, keys);
    provider.borrow_mut().assignments.clear();
    assert_eq!(
        prepare_remote_view_session_pool(&mut adapter, &mut store, &pool)
            .err()
            .unwrap(),
        "remote_view_pool_acquisition_readback_required"
    );
    assert_eq!(provider.borrow().acquisitions, keys);
}

#[test]
fn pool_request_heads_share_unsubmitted_and_pending_identity_across_connections() {
    let fixture = Fixture::new();
    let mut first = fixture.store(false);
    let mut peer = fixture.store(false);
    let key = first
        .pool_acquisition_request_key("agent-browser", "main", &[])
        .unwrap();
    assert_eq!(
        peer.pool_acquisition_request_key("agent-browser", "main", &[])
            .unwrap(),
        key
    );
    let envelope = RemoteViewApplicationEnvelope {
        application: "agent-browser".into(),
        request: RemoteViewApplicationRequest::Acquire {
            pool_name: "main".into(),
            idempotency_key: key.clone(),
        },
    };
    assert_eq!(
        first.claim(&envelope).unwrap(),
        RemoteViewApplicationMutationClaim::New
    );
    assert_eq!(
        peer.pool_acquisition_request_key("agent-browser", "main", &[])
            .unwrap(),
        key
    );
    assert!(matches!(
        peer.claim(&envelope).unwrap(),
        RemoteViewApplicationMutationClaim::Existing(_)
    ));
    assert_eq!(
        peer.pending_pool_acquisitions("agent-browser", "main")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn pool_request_advances_after_exact_completed_retirement_and_preserves_history() {
    use RemoteViewApplicationObligationInventory as Inventory;
    let fixture = Fixture::new();
    let mut store = fixture.store(false);
    let key = store
        .pool_acquisition_request_key("agent-browser", "main", &[])
        .unwrap();
    let envelope = RemoteViewApplicationEnvelope {
        application: "agent-browser".into(),
        request: RemoteViewApplicationRequest::Acquire {
            pool_name: "main".into(),
            idempotency_key: key.clone(),
        },
    };
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(wire()["assignment"].clone()).unwrap();
    store.claim(&envelope).unwrap();
    store
        .complete(
            &envelope,
            &RemoteViewApplicationMutationOutcome::Acquire(assignment.clone()),
        )
        .unwrap();
    assert!(store
        .pool_acquisition_request_key("agent-browser", "main", &[])
        .is_err());
    let target = RemoteViewApplicationReleaseTarget {
        assignment: assignment.clone(),
        route_ids: Vec::new(),
        viewer_session_ids: Vec::new(),
    };
    let snapshot = RemoteViewApplicationCleanupSnapshot {
        schema_version: 1,
        assignment_id: assignment.assignment_id.clone(),
        desktop_id: assignment.desktop_id.clone(),
        lifecycle_generation: assignment.generation,
        pending_recovery: Inventory::Complete(Vec::new()),
        foreground_leases: Inventory::Complete(Vec::new()),
        cleanup_tasks: Inventory::Complete(Vec::new()),
    };
    let release_id = uuid::Uuid::new_v4().to_string();
    store
        .admit_assignment_release(&target, &snapshot, &release_id)
        .unwrap();
    assert!(store
        .pool_acquisition_request_key("agent-browser", "main", &[])
        .is_err());
    let mut released = assignment.clone();
    released.state = RemoteViewAssignmentState::Released;
    let outcome = RemoteViewJoinedReleaseOutcome {
        assignment: released,
        retirement: RemoteViewDesktopViewingRetirement {
            schema_version: 1,
            desktop_id: assignment.desktop_id,
            generation: assignment.generation,
            routes: Vec::new(),
            sessions: Vec::new(),
        },
    };
    store
        .complete_assignment_release(&target, &release_id, &outcome)
        .unwrap();
    drop(store);
    let mut restarted = fixture.store(false);
    let next = restarted
        .pool_acquisition_request_key("agent-browser", "main", &[])
        .unwrap();
    assert_ne!(next, key);
    assert!(
        matches!(restarted.claim(&envelope).unwrap(), RemoteViewApplicationMutationClaim::Existing(record) if record.outcome.is_some())
    );
}

#[test]
fn pool_demand_prefers_window_free_desktops_and_retains_healthy_occupied_peers() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut adapter = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 2,
    };
    let prepared = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    let first = prepared.assignments[0].clone();
    let second = prepared.assignments[1].clone();
    provider.borrow_mut().empty_desktop = Some(second.desktop_id.clone());
    let prepared = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(prepared.desktops[0].desktop.desktop_id, second.desktop_id);
    assert_eq!(prepared.desktops.len(), 2);
    provider.borrow_mut().empty_desktop = None;
    provider.borrow_mut().unavailable_assignment = Some(first.assignment_id);
    let prepared = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(prepared.desktops.len(), 1);
    assert_eq!(prepared.desktops[0].desktop.desktop_id, second.desktop_id);
    assert_eq!(provider.borrow().acquisitions.len(), 2);
}

#[test]
fn ordinary_first_open_admits_exact_profile_and_reuses_tab_with_navigation_history() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let command = serde_json::json!({"id": "first-open", "action": "navigate", "runtimeProfile": "profile-a",
        "url": "https://synthetic.example/first", "headers": {"X-Test":"fixture"}, "waitUntil":"domcontentloaded", "activityAtMs": 100});
    let opened = consumer
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(opened["success"], true);
    assert_eq!(opened["id"], "first-open");
    assert_eq!(opened["data"]["headers"], command["headers"]);
    assert_eq!(opened["data"]["waitUntil"], "domcontentloaded");
    assert_eq!(opened["browserSession"]["profileId"], "profile-a");
    let handoff_id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let persisted = fixture.store(false).load_session_state().unwrap();
    assert_eq!(persisted.remote_view_tab_handoffs.len(), 1);
    let retained =
        agent_browser_service_model::resolve_remote_view_tab_handoff(&persisted, handoff_id)
            .unwrap();
    assert_eq!(retained.tab.id, opened["browserSession"]["tabId"]);
    assert!(opened.get("handoffUrl").is_none());
    assert_eq!(consumer.state().navigation_history.len(), 1);
    assert_eq!(
        consumer.state().navigation_history[0].url,
        "https://synthetic.example/first"
    );
    let mut second = command.clone();
    second["url"] = "https://synthetic.example/second".into();
    second["activityAtMs"] = 101.into();
    second.as_object_mut().unwrap().remove("runtimeProfile");
    second["profile"] = fixture
        .root
        .join("profile-a")
        .to_string_lossy()
        .into_owned()
        .into();
    let reopened = consumer
        .execute_managed_command("Alice", &second)
        .unwrap()
        .unwrap();
    assert_eq!(reopened["browserSession"], opened["browserSession"]);
    assert_eq!(calls.get(), 1);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert_eq!(consumer.state().navigation_history.len(), 2);
    second["url"] = "https://synthetic.example/failure".into();
    let failed = consumer
        .execute_managed_command("Alice", &second)
        .unwrap()
        .unwrap();
    assert_eq!(failed["success"], false);
    assert_eq!(consumer.state().navigation_history.len(), 2);
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    let result = restarted
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(result["browserSession"], opened["browserSession"]);
    assert_eq!(calls.get(), 1);
}

#[test]
fn ordinary_profile_selector_conflict_precedes_allocation_and_default_is_disposable() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let command = serde_json::json!({"action":"navigate", "url":"https://synthetic.example/", "activityAtMs":100,
        "runtimeProfile":"profile-a", "profileId":"profile-b"});
    assert_eq!(
        consumer
            .execute_managed_command("Alice", &command)
            .unwrap_err(),
        "browser_session_profile_selector_conflict"
    );
    assert!(provider.borrow().operations.is_empty());
    assert_eq!(calls.get(), 0);
    for (invalid, expected) in [
        (
            serde_json::json!({"action":"navigate", "url":"https://synthetic.example/", "profileId":4}),
            "browser_session_profile_selector_invalid",
        ),
        (
            serde_json::json!({"action":"navigate", "url":"https://synthetic.example/", "profileId":"unknown"}),
            "browser_session_profile_selector_unknown",
        ),
        (
            serde_json::json!({"action":"navigate", "profileId":"profile-a"}),
            "browser_session_field_missing:url",
        ),
    ] {
        assert_eq!(
            consumer
                .execute_managed_command("Alice", &invalid)
                .unwrap_err(),
            expected
        );
        assert!(provider.borrow().operations.is_empty());
    }
    let command = serde_json::json!({"id":"default-open", "action":"navigate", "url":"https://synthetic.example/", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(opened["success"], true);
    assert_eq!(consumer.state().disposable_profiles.len(), 1);
    assert_eq!(calls.get(), 1);
}

#[test]
fn handoff_view_issuance_and_resolution_survive_host_restart_without_launch_or_navigation() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let issuance = consumer.resolve_tab_handoff_view(id, 2000).unwrap();
    assert_eq!(issuance.presentation_state, "grant_issued");
    let stored = fixture.store(false).load_session_state().unwrap();
    assert_eq!(
        stored.remote_view_tab_handoffs[id]
            .view
            .as_ref()
            .unwrap()
            .issuance
            .as_ref(),
        Some(&issuance)
    );
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    let response = restarted.handle_command(&serde_json::json!({
        "id":"resolve", "action":"service_remote_view_handoff_resolve", "handoffId":id, "activityAtMs":2001,
    }));
    assert_eq!(response["success"], true);
    assert_eq!(response["data"]["status"], "converging");
    assert_eq!(response["data"]["operatorVisible"]["state"], "pending");
    assert_eq!(
        response["data"]["targetId"],
        opened["browserSession"]["targetId"]
    );
    assert!(response["data"].get("providerExternalUrl").is_none());
    assert_eq!(provider.borrow().view_requests.len(), 1);
    assert_eq!(calls.get(), 1);
    assert_eq!(restarted.state().navigation_history.len(), 1);
    let reused = restarted
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "activityAtMs":2001,
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(reused["browserSession"]["handoffId"], id);
    let original_assignment = provider.borrow().assignments[0].assignment_id.clone();
    provider.borrow_mut().assignments[0].assignment_id = "replacement-assignment".into();
    assert!(restarted.resolve_tab_handoff_view(id, 2001).is_err());
    assert_eq!(provider.borrow().view_requests.len(), 1);
    provider.borrow_mut().assignments[0].assignment_id = original_assignment;
    let session_id = opened["browserSession"]["sessionId"].as_str().unwrap();
    restarted.close_current_tab(session_id, 2002).unwrap();
    let operations = provider.borrow().operations.len();
    assert!(restarted.resolve_tab_handoff_view(id, 2003).is_err());
    assert_eq!(provider.borrow().operations.len(), operations);
}

#[test]
fn handoff_lost_view_reply_keeps_one_request_across_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    provider.borrow_mut().lose_view_reply = true;
    assert!(consumer.resolve_tab_handoff_view(id, 2000).is_err());
    let before = fixture
        .store(false)
        .load_session_state()
        .unwrap()
        .remote_view_tab_handoffs[id]
        .view
        .clone();
    assert!(before.as_ref().unwrap().issuance.is_none());
    drop(consumer);
    provider.borrow_mut().lose_view_reply = false;
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    assert!(restarted.resolve_tab_handoff_view(id, 2001).is_err());
    assert_eq!(restarted.state().remote_view_tab_handoffs[id].view, before);
    assert_eq!(provider.borrow().view_requests.len(), 1);
    assert_eq!(calls.get(), 1);
}

#[test]
fn completed_view_with_failed_publication_reuses_ledger_outcome_after_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    provider.borrow_mut().competing_view_publication = true;
    assert!(consumer.resolve_tab_handoff_view(id, 2000).is_err());
    assert!(fixture
        .store(false)
        .load_session_state()
        .unwrap()
        .remote_view_tab_handoffs[id]
        .view
        .as_ref()
        .unwrap()
        .issuance
        .is_none());
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    let recovered = restarted.resolve_tab_handoff_view(id, 2001).unwrap();
    assert_eq!(recovered.presentation_state, "grant_issued");
    assert_eq!(provider.borrow().view_requests.len(), 1);
    assert_eq!(calls.get(), 1);
}
