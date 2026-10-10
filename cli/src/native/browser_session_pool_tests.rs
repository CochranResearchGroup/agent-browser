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
    delayed_scoped_view: bool,
    view_issued_at: Option<u64>,
    terminal_view_key: Option<String>,
    all_views_terminal: bool,
    unavailable_assignment: Option<String>,
    empty_desktop: Option<String>,
    occupied_desktops: std::collections::BTreeSet<String>,
    fail_new_windows: bool,
    pool_maximum: Option<u32>,
    window_pid: Option<u32>,
    inactive_window: bool,
    viewer_active_desktops: std::collections::BTreeSet<String>,
    defer_idle_release: bool,
    lose_idle_release_reply: bool,
    idle_release_requests: Vec<String>,
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
            RemoteViewApplicationRequest::ObserveIdle { assignment_id, .. } => {
                provider.operations.push("observe_idle");
                let assignment = provider
                    .assignments
                    .iter()
                    .find(|a| &a.assignment_id == assignment_id)
                    .unwrap();
                let mut observed = fixture["assignmentObservation"].clone();
                observed["target"]["assignmentId"] = assignment.assignment_id.clone().into();
                observed["target"]["desktopId"] = assignment.desktop_id.clone().into();
                let viewer = provider
                    .viewer_active_desktops
                    .contains(&assignment.desktop_id);
                observed["retirement"] = serde_json::json!({"schemaVersion":1,"desktopId":assignment.desktop_id,
                    "generation":assignment.generation,"routes":[],"sessions":[]});
                observed["idle"] = serde_json::json!(
                    !viewer
                        && !provider.occupied_desktops.contains(&assignment.desktop_id)
                        && provider.window_pid.is_none()
                );
                observed["viewerActive"] = viewer.into();
                Ok(observed)
            }
            RemoteViewApplicationRequest::ReleaseIdle {
                assignment_id,
                expected_retirement,
                idempotency_key,
                ..
            } => {
                provider.operations.push("release_idle");
                provider.idle_release_requests.push(idempotency_key.clone());
                let defer = provider.defer_idle_release;
                let assignment = provider
                    .assignments
                    .iter_mut()
                    .find(|a| &a.assignment_id == assignment_id)
                    .unwrap();
                if defer {
                    return Ok(
                        serde_json::json!({"schemaVersion":1,"operation":"release_idle","state":"retained",
                        "assignmentId":assignment_id,"generation":assignment.generation}),
                    );
                }
                assignment.state = RemoteViewAssignmentState::Released;
                let outcome =
                    serde_json::json!({"assignment":assignment,"retirement":expected_retirement});
                if provider.lose_idle_release_reply {
                    return Err(RemoteViewApplicationTransportError::OutcomeUnknown);
                }
                Ok(outcome)
            }
            RemoteViewApplicationRequest::Inventory {} => {
                provider.operations.push("inventory");
                let mut inventory = fixture["inventory"].clone();
                if let Some(maximum) = provider.pool_maximum {
                    inventory["policy"]["pools"]["main"]["maximum_reserved"] = maximum.into();
                }
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
                if let Some(index) = provider
                    .acquisitions
                    .iter()
                    .position(|key| key == idempotency_key)
                {
                    if provider.lose_reply {
                        return Err(RemoteViewApplicationTransportError::OutcomeUnknown);
                    }
                    return Ok(serde_json::to_value(&provider.assignments[index]).unwrap());
                }
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
                if provider.fail_new_windows
                    && provider
                        .assignments
                        .first()
                        .is_some_and(|first| first.assignment_id != *assignment_id)
                {
                    return Err(RemoteViewApplicationTransportError::Unavailable);
                }
                let assignment = provider
                    .assignments
                    .iter()
                    .find(|assignment| assignment.assignment_id == *assignment_id)
                    .unwrap();
                let mut windows = fixture["windows"].clone();
                windows["desktopId"] = assignment.desktop_id.clone().into();
                if let Some(pid) = provider.window_pid {
                    windows["windows"][0]["pid"] = serde_json::json!(pid);
                }
                if provider.inactive_window {
                    windows["windows"][0]["active"] = serde_json::json!(false);
                }
                if provider.empty_desktop.as_ref() == Some(&assignment.desktop_id)
                    || (provider.window_pid.is_none()
                        && !provider.occupied_desktops.contains(&assignment.desktop_id))
                {
                    windows["windows"] = serde_json::json!([]);
                }
                Ok(windows)
            }
            RemoteViewApplicationRequest::LaunchEnvironment { assignment_id, .. } => {
                provider.operations.push("launch_environment");
                let assignment = provider
                    .assignments
                    .iter()
                    .find(|assignment| assignment.assignment_id == *assignment_id)
                    .unwrap();
                let mut environment = fixture["launchEnvironment"].clone();
                environment["target"]["assignmentId"] = assignment.assignment_id.clone().into();
                environment["target"]["desktopId"] = assignment.desktop_id.clone().into();
                Ok(environment)
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
                if provider.all_views_terminal
                    || provider.terminal_view_key.as_ref() == Some(idempotency_key)
                {
                    return Err(RemoteViewApplicationTransportError::ViewGrantTerminal);
                }
                if provider.lose_view_reply {
                    return Err(RemoteViewApplicationTransportError::OutcomeUnknown);
                }
                let mut issuance = fixture["viewIssuance"].clone();
                issuance["grant"]["request"]["idempotencyKey"] = if provider.delayed_scoped_view {
                    format!("provider-scoped-{idempotency_key}")
                } else {
                    idempotency_key.clone()
                }
                .into();
                if provider.delayed_scoped_view {
                    issuance["grant"]["issuedAt"] = 2001.into();
                    issuance["grant"]["expiresAt"] = 302001.into();
                }
                if let Some(now) = provider.view_issued_at {
                    issuance["grant"]["issuedAt"] = now.into();
                    issuance["grant"]["expiresAt"] = (now + 300_000).into();
                }
                Ok(issuance)
            }
            RemoteViewApplicationRequest::ResolveView { route_id, audience } => {
                provider.operations.push("resolve_view");
                assert_eq!(audience, "remote_view");
                assert_eq!(route_id, fixture["grant"]["routeId"].as_str().unwrap());
                let mut grant = fixture["grant"].clone();
                let key = provider.view_requests.last().unwrap();
                grant["request"]["idempotencyKey"] = if provider.delayed_scoped_view {
                    format!("provider-scoped-{key}")
                } else {
                    key.clone()
                }
                .into();
                if provider.delayed_scoped_view {
                    grant["issuedAt"] = 2001.into();
                    grant["expiresAt"] = 302001.into();
                }
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
    host_mode(fixture, provider, calls, Mode::Success)
}
fn host_mode(
    fixture: &Fixture,
    provider: Rc<RefCell<Provider>>,
    calls: Rc<Cell<u32>>,
    mode: Mode,
) -> BrowserSessionHost<
    BrowserSessionSqliteStore,
    RemoteViewSessionEffects<Process, PoolTransport, BrowserSessionSqliteStore>,
> {
    host_mode_clock(fixture, provider, calls, mode, || 2001)
}
fn host_mode_clock(
    fixture: &Fixture,
    provider: Rc<RefCell<Provider>>,
    calls: Rc<Cell<u32>>,
    mode: Mode,
    clock: fn() -> u64,
) -> BrowserSessionHost<
    BrowserSessionSqliteStore,
    RemoteViewSessionEffects<Process, PoolTransport, BrowserSessionSqliteStore>,
> {
    let effects = RemoteViewSessionEffects::new(
        Process {
            mode,
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
    .unwrap()
    .with_view_clock(clock);
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
    let first_desktop = provider.borrow().assignments[0].desktop_id.clone();
    provider
        .borrow_mut()
        .occupied_desktops
        .insert(first_desktop);
    let other = restarted
        .open(OpenBrowserSession::exact_profile("Other", "profile-b", 103))
        .unwrap();
    assert_ne!(
        restarted.state().browsers[&opened.browser_id].desktop,
        restarted.state().browsers[&other.browser_id].desktop
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
    assert_eq!(calls.get(), 2);
    let mut store = fixture.store(false);
    assert!(store
        .pending_pool_acquisitions("agent-browser", "main")
        .unwrap()
        .is_empty());
    assert!(store.unpublished_launch_records().unwrap().is_empty());
}

#[test]
fn pool_demand_lost_acquire_reply_recovers_same_assignment_after_restart() {
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
    assert_eq!(provider.borrow().assignments.len(), 1);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    drop(consumer);
    provider.borrow_mut().lose_reply = false;
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    restarted
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 101))
        .unwrap();
    assert_eq!(provider.borrow().assignments.len(), 1);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert_eq!(calls.get(), 1);
    assert!(fixture
        .store(false)
        .pending_pool_acquisitions("agent-browser", "main")
        .unwrap()
        .is_empty());
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
fn pool_demand_acquires_a_free_desktop_for_an_independent_browser() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut adapter = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    let first = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    let occupied = first.desktops[0].desktop.desktop_id.clone();
    provider
        .borrow_mut()
        .occupied_desktops
        .insert(occupied.clone());
    let second = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(
        provider.borrow().acquisitions.len(),
        2,
        "independent browser demand must grow beyond the warm minimum"
    );
    assert!(second
        .desktops
        .iter()
        .any(|candidate| candidate.desktop.desktop_id != occupied));
    provider.borrow_mut().occupied_desktops.extend(
        second
            .assignments
            .iter()
            .map(|assignment| assignment.desktop_id.clone()),
    );
    let overflow = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(
        overflow.desktops.len(),
        2,
        "at the growth limit occupied healthy desktops remain eligible for wrap-around"
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
}

#[test]
fn pool_demand_unknown_new_desktop_stops_before_another_acquisition() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        pool_maximum: Some(3),
        ..Provider::default()
    }));
    let mut adapter = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    let first = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    provider
        .borrow_mut()
        .occupied_desktops
        .insert(first.assignments[0].desktop_id.clone());
    provider.borrow_mut().fail_new_windows = true;
    assert_eq!(
        prepare_remote_view_session_pool(&mut adapter, &mut store, &pool)
            .err()
            .unwrap(),
        "remote_view_runtime_assignment_unavailable"
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
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
fn pool_demand_excludes_occupied_desktops_and_retains_their_assignments() {
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
    provider
        .borrow_mut()
        .occupied_desktops
        .insert(first.desktop_id.clone());
    provider.borrow_mut().empty_desktop = Some(second.desktop_id.clone());
    let prepared = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(prepared.desktops[0].desktop.desktop_id, second.desktop_id);
    assert_eq!(prepared.desktops.len(), 1);
    assert_eq!(prepared.assignments.len(), 2);
    provider.borrow_mut().empty_desktop = None;
    provider.borrow_mut().unavailable_assignment = Some(first.assignment_id);
    let prepared = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(prepared.desktops.len(), 1);
    assert_eq!(prepared.desktops[0].desktop.desktop_id, second.desktop_id);
    assert_eq!(provider.borrow().acquisitions.len(), 2);
}

#[test]
fn ordinary_read_recovers_idle_retained_browser_without_opening_handoff() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    let identity = opened["browserSession"].clone();
    let original_target = opened["targetId"].clone();
    let browser_id = identity["browserId"].as_str().unwrap();
    let reaped = consumer.reap(300_101).unwrap();
    assert_eq!(reaped.closed_browser_ids, vec![browser_id]);
    assert!(consumer
        .state()
        .idle_closed_browsers
        .contains_key(browser_id));
    drop(consumer);

    let mut restarted = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let resumed = restarted
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":300_102
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(resumed["success"], true);
    let mut recovered_identity = resumed["browserSession"].clone();
    recovered_identity["targetId"] = identity["targetId"].clone();
    assert_eq!(recovered_identity, identity);
    assert_eq!(resumed["browserSession"]["targetId"], resumed["targetId"]);
    assert_ne!(resumed["targetId"], original_target);
    assert_eq!(calls.get(), 2);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert!(provider.borrow().view_requests.is_empty());
    assert!(restarted.state().idle_closed_browsers.is_empty());
    assert!(fixture
        .store(false)
        .unpublished_launch_records()
        .unwrap()
        .is_empty());
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        *restarted.state()
    );
    restarted
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":300_103
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(calls.get(), 2);
}

#[test]
fn ordinary_idle_recovery_preserves_unrelated_unfinished_launch() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    let identity = opened["browserSession"].clone();
    let original_target = opened["targetId"].clone();
    let browser_id = identity["browserId"].as_str().unwrap();
    let reaped = consumer.reap(300_101).unwrap();
    assert_eq!(reaped.closed_browser_ids, vec![browser_id]);
    assert!(consumer
        .state()
        .idle_closed_browsers
        .contains_key(browser_id));
    drop(consumer);

    let mut store = fixture.store(false);
    let assignment = provider.borrow().assignments[0].clone();
    let unrelated = BrowserLaunchIntent {
        intent_id: uuid::Uuid::new_v4().to_string(),
        profile_id: "profile-b".into(),
        assignment,
    };
    let baseline = store.load_session_state().unwrap();
    store.admit_launch_intent(&unrelated, &baseline).unwrap();
    let retained_claim = store.unpublished_launch_records().unwrap();
    crate::native::browser_session_remote_view::reconcile_pending_recoveries(
        &mut Process {
            mode: Mode::IdleRecovery,
            root: fixture.root.clone(),
            calls: calls.clone(),
        },
        &mut store,
    )
    .unwrap();
    assert_eq!(store.unpublished_launch_records().unwrap(), retained_claim);

    let mut restarted = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let resumed = restarted
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":300_102
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(resumed["success"], true);
    let mut recovered_identity = resumed["browserSession"].clone();
    recovered_identity["targetId"] = identity["targetId"].clone();
    assert_eq!(recovered_identity, identity);
    assert_eq!(resumed["browserSession"]["targetId"], resumed["targetId"]);
    assert_ne!(resumed["targetId"], original_target);
    let handoff_id = identity["handoffId"].as_str().unwrap();
    provider.borrow_mut().window_pid = Some(restarted.state().browsers[browser_id].pid);
    let handoff = restarted.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":handoff_id,
        "nativeDesktop":true, "activityAtMs":300_103,
    }));
    assert_eq!(handoff["success"], true, "{handoff}");
    assert_eq!(handoff["data"]["operatorVisible"]["state"], "ready");
    assert_eq!(handoff["data"]["targetId"], resumed["targetId"]);
    assert_eq!(calls.get(), 2);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert!(provider.borrow().view_requests.is_empty());
    assert!(restarted.state().idle_closed_browsers.is_empty());
    assert_eq!(
        fixture.store(false).unpublished_launch_records().unwrap(),
        retained_claim
    );
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        *restarted.state()
    );
    restarted
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":300_103
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(calls.get(), 2);
}

#[test]
fn ordinary_idle_recovery_refuses_unproven_absence_before_launch_or_activity_refresh() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    consumer.reap(300_101).unwrap();
    let before = consumer.state().clone();
    std::fs::write(fixture.root.join("recovery-absence-unknown"), "unknown").unwrap();
    assert_eq!(
        consumer
            .execute_managed_command(
                "Alice",
                &serde_json::json!({
                    "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":300_102
                })
            )
            .unwrap_err(),
        "browser_session_recovery_absence_unproven"
    );
    assert_eq!(consumer.state(), &before);
    assert_eq!(fixture.store(false).load_session_state().unwrap(), before);
    assert_eq!(calls.get(), 1);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert!(fixture
        .store(false)
        .unpublished_launch_records()
        .unwrap()
        .is_empty());
}

#[test]
fn ordinary_first_open_admits_exact_profile_and_reuses_tab_with_navigation_history() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let request = serde_json::json!({"action": "navigate", "runtimeProfile": "profile-a", "sessionName":"Alice",
        "params":{"url": "https://synthetic.example/first", "headers": {"X-Test":"fixture"}, "waitUntil":"domcontentloaded", "activityAtMs": 100}});
    let command =
        crate::native::stream::service_request_adapter_fixture(&request.to_string()).unwrap();
    let opened = consumer
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(opened["success"], true);
    assert_eq!(opened["id"], command["id"]);
    assert_eq!(opened["data"]["headers"], command["headers"]);
    assert_eq!(opened["data"]["waitUntil"], "domcontentloaded");
    assert_eq!(opened["browserSession"]["profileId"], "profile-a");
    let handoff_id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let persisted = fixture.store(false).load_session_state().unwrap();
    assert_eq!(persisted.remote_view_tab_handoffs.len(), 1);
    assert_eq!(
        opened["browserSession"]["handoffUrl"],
        format!("/remote-view/{handoff_id}")
    );
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
    let mut second = crate::mcp::service_request_adapter_fixture_for_session(
        &serde_json::json!({
            "action":"navigate", "sessionName":"Alice",
            "profile":fixture.root.join("profile-a").to_string_lossy().into_owned(),
            "params":{"url":"https://synthetic.example/second", "activityAtMs":101},
        }),
        "service-default",
    )
    .unwrap();
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
fn named_session_navigation_registers_explicit_profile_before_open() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider, calls.clone());
    let path = fixture.root.join("cli-fresh-profile");
    std::fs::create_dir_all(&path).unwrap();
    let response = consumer.handle_command(&serde_json::json!({
        "action":"browser_session_navigate", "sessionName":"Alice", "profileId":"cli-fresh",
        "runtimeProfile":"cli-fresh", "profile":path, "url":"https://synthetic.example/", "activityAtMs":100
    }));
    assert_eq!(response["success"], true, "{response}");
    assert_eq!(response["data"]["profileId"], "cli-fresh");
    assert_eq!(calls.get(), 1);
}

#[test]
fn ordinary_open_registers_explicit_profile_and_rejects_identity_rebinding() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let profile_path = fixture.root.join("fresh-profile");
    std::fs::create_dir_all(&profile_path).unwrap();
    let command = serde_json::json!({"action":"navigate", "url":"https://synthetic.example/",
        "runtimeProfile":"fresh", "profile":profile_path, "activityAtMs":100});
    let mut conflict = command.clone();
    conflict["profileId"] = serde_json::json!("different");
    assert_eq!(
        consumer
            .execute_managed_command("Alice", &conflict)
            .unwrap_err(),
        "browser_session_profile_selector_conflict"
    );
    assert!(!fixture
        .store(false)
        .load_profile_catalog()
        .unwrap()
        .catalog
        .profiles
        .contains_key("fresh"));
    assert_eq!(calls.get(), 0);
    let opened = consumer
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(opened["success"], true);
    assert_eq!(calls.get(), 1);
    let persisted = fixture.store(false).load_profile_catalog().unwrap();
    assert_eq!(
        persisted.catalog.profiles["fresh"].user_data_dir,
        profile_path.canonicalize().unwrap().to_string_lossy()
    );
    let other_path = fixture.root.join("other-profile");
    std::fs::create_dir_all(&other_path).unwrap();
    let mut rebound = command.clone();
    rebound["profile"] = serde_json::json!(other_path);
    assert_eq!(
        consumer
            .execute_managed_command("Alice", &rebound)
            .unwrap_err(),
        "browser_session_profile_selector_conflict"
    );
    let mut alias = command;
    alias["runtimeProfile"] = serde_json::json!("alias");
    assert_eq!(
        consumer
            .execute_managed_command("Alice", &alias)
            .unwrap_err(),
        "browser_session_profile_selector_conflict"
    );
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
fn native_handoff_resolution_survives_restart_without_viewer_grants_or_navigation_replay() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    provider.borrow_mut().all_views_terminal = true;
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_string();
    let browser_id = opened["browserSession"]["browserId"].as_str().unwrap();
    provider.borrow_mut().window_pid = Some(consumer.state().browsers[browser_id].pid);
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    let command = serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "nativeDesktop":true, "activityAtMs":2001,
    });
    for _ in 0..2 {
        let response = restarted.handle_command(&command);
        assert_eq!(response["success"], true, "{response}");
        assert_eq!(response["data"]["operatorVisible"]["state"], "ready");
        assert_eq!(
            response["data"]["targetId"],
            opened["browserSession"]["targetId"]
        );
    }
    assert!(provider.borrow().view_requests.is_empty());
    assert_eq!(calls.get(), 1);
    assert_eq!(restarted.state().navigation_history.len(), 1);
    let session_id = opened["browserSession"]["sessionId"].as_str().unwrap();
    restarted
        .close_session(session_id, SessionEndReason::ExplicitClose, 2002)
        .unwrap();
    let response = restarted.handle_command(&command);
    assert_eq!(response["success"], false);
    assert_eq!(calls.get(), 1);
}

#[test]
fn handoff_view_issuance_and_resolution_survive_host_restart_without_launch_or_navigation() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    provider.borrow_mut().delayed_scoped_view = true;
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
    let before_view = fixture.store(false).load_session_state().unwrap();
    let target =
        agent_browser_service_model::resolve_remote_view_tab_handoff(&before_view, id).unwrap();
    let view = before_view.remote_view_tab_handoffs[id]
        .view
        .as_ref()
        .unwrap();
    let read_only = agent_browser_service_model::resolve_published_remote_view_tab_view(
        &mut adapter(&fixture, provider.clone()),
        &mut fixture.store(false),
        &target,
        view,
        2001,
    )
    .unwrap();
    assert_eq!(read_only, issuance);
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        before_view
    );
    assert_eq!(provider.borrow().view_requests.len(), 1);
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
    restarted.resolve_tab_handoff_view(id, 2001).unwrap();
    assert_eq!(
        restarted.state().remote_view_tab_handoffs[id]
            .view
            .as_ref()
            .unwrap()
            .idempotency_key,
        before.as_ref().unwrap().idempotency_key
    );
    assert_eq!(provider.borrow().view_requests.len(), 2);
    assert_eq!(
        provider.borrow().view_requests[0],
        provider.borrow().view_requests[1]
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn published_view_expiry_renews_after_restart_without_relaunch_or_navigation() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let issuance = consumer.resolve_tab_handoff_view(id, 2000).unwrap();
    let before = consumer.state().remote_view_tab_handoffs[id].clone();
    drop(consumer);
    provider.borrow_mut().view_issued_at = Some(issuance.grant.expires_at);
    let mut restarted = host_mode_clock(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::Success,
        || 301000,
    );
    let replacement = restarted
        .resolve_tab_handoff_view(id, issuance.grant.expires_at)
        .unwrap();
    let after = &restarted.state().remote_view_tab_handoffs[id];
    assert_eq!(after.id, before.id);
    assert_eq!(after.session_id, before.session_id);
    assert_eq!(after.tab_id, before.tab_id);
    assert_ne!(
        after.view.as_ref().unwrap().idempotency_key,
        before.view.as_ref().unwrap().idempotency_key
    );
    assert!(replacement.grant.expires_at > issuance.grant.expires_at);
    assert_eq!(provider.borrow().view_requests.len(), 2);
    assert_eq!(calls.get(), 1);
    assert_eq!(restarted.state().navigation_history.len(), 1);
}

#[test]
fn published_view_expiry_rejects_invalid_or_revoked_issuance() {
    for invalid in ["revoked", "application", "target"] {
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider::default()));
        let calls = Rc::new(Cell::new(0));
        let mut consumer = host(&fixture, provider.clone(), calls.clone());
        let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
            "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
        })).unwrap().unwrap();
        let id = opened["browserSession"]["handoffId"].as_str().unwrap();
        let issuance = consumer.resolve_tab_handoff_view(id, 2000).unwrap();
        let before = consumer.state().clone();
        drop(consumer);
        let mut changed = before.clone();
        let grant = &mut changed
            .remote_view_tab_handoffs
            .get_mut(id)
            .unwrap()
            .view
            .as_mut()
            .unwrap()
            .issuance
            .as_mut()
            .unwrap()
            .grant;
        match invalid {
            "revoked" => grant.revoked = true,
            "application" => grant.request.application = "foreign".into(),
            "target" => grant.request.target.viewing_generation += 1,
            _ => unreachable!(),
        }
        fixture
            .store(false)
            .compare_and_save_session_state(&before, &changed)
            .unwrap();
        let mut restarted = host(&fixture, provider.clone(), calls.clone());
        assert_eq!(
            restarted
                .resolve_tab_handoff_view(id, issuance.grant.expires_at)
                .unwrap_err(),
            "remote_view_tab_view_issuance_invalid"
        );
        assert_eq!(restarted.state(), &changed);
        assert_eq!(provider.borrow().view_requests.len(), 1);
    }
}

#[test]
fn published_view_expiry_preserves_handoff_when_renewal_cannot_complete() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let issuance = consumer.resolve_tab_handoff_view(id, 2000).unwrap();
    let retained = consumer.state().remote_view_tab_handoffs[id].clone();
    provider.borrow_mut().all_views_terminal = true;
    assert_eq!(
        consumer
            .resolve_tab_handoff_view(id, issuance.grant.expires_at)
            .unwrap_err(),
        "remote_view_tab_view_grant_terminal"
    );
    assert_eq!(
        consumer.state().remote_view_tab_handoffs[id].id,
        retained.id
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn terminal_pending_view_renews_once_preserving_handoff_and_mutation_history() {
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
    let old_key = consumer.state().remote_view_tab_handoffs[id]
        .view
        .as_ref()
        .unwrap()
        .idempotency_key
        .clone();
    drop(consumer);
    provider.borrow_mut().lose_view_reply = false;
    provider.borrow_mut().terminal_view_key = Some(old_key.clone());
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    restarted.resolve_tab_handoff_view(id, 2001).unwrap();
    let new_key = &restarted.state().remote_view_tab_handoffs[id]
        .view
        .as_ref()
        .unwrap()
        .idempotency_key;
    assert_ne!(*new_key, old_key);
    assert_eq!(
        provider.borrow().view_requests,
        vec![old_key.clone(), old_key.clone(), new_key.clone()]
    );
    let mut connection = fixture.store(false);
    let terminal_records: i64 = connection
        .remote_view_mutation_connection()
        .query_row(
            "SELECT count(*) FROM state_documents WHERE json LIKE '%issue_view_terminal%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(terminal_records, 1);
    assert_eq!(calls.get(), 1);
}

#[test]
fn terminal_view_renewal_stops_after_one_replacement() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    provider.borrow_mut().all_views_terminal = true;
    assert_eq!(
        consumer.resolve_tab_handoff_view(id, 2000).unwrap_err(),
        "remote_view_tab_view_grant_terminal"
    );
    assert_eq!(provider.borrow().view_requests.len(), 2);
    assert!(consumer.state().remote_view_tab_handoffs[id]
        .view
        .as_ref()
        .unwrap()
        .issuance
        .is_none());
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

#[tokio::test]
async fn queued_action_executor_resolves_managed_handoff_through_the_registered_host() {
    use crate::native::browser_session_remote_view::RuntimeSessionEffects;
    use crate::native::browser_session_runtime::{
        BrowserManagerRuntime, BrowserManagerRuntimeConfig, BrowserSessionEffectAdapter,
    };
    use crate::native::remote_view_application_http::RemoteViewApplicationHttp;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let fixture = Fixture::new();
    let cdp_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let cdp_endpoint = format!(
        "ws://{}/devtools/browser/fixture",
        cdp_listener.local_addr().unwrap()
    );
    std::fs::write(
        fixture.root.join("live-cdp-endpoint.json"),
        serde_json::json!({"pid":std::process::id(), "endpoint":cdp_endpoint}).to_string(),
    )
    .unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut source = host(&fixture, provider.clone(), calls.clone());
    let opened = source.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_string();
    drop(source);
    let before = fixture.store(false).load_session_state().unwrap();
    let target = before.tabs.values().next().unwrap().target_id.clone();
    let cdp_server = tokio::spawn(async move {
        use futures_util::{SinkExt, StreamExt};
        let (stream, _) = cdp_listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        while let Some(Ok(message)) = socket.next().await {
            let tokio_tungstenite::tungstenite::Message::Text(text) = message else {
                continue;
            };
            let request: Value = serde_json::from_str(&text).unwrap();
            let result = match request["method"].as_str().unwrap() {
                "Target.getTargets" => serde_json::json!({"targetInfos":[{
                    "targetId":target, "type":"page", "title":"Fixture",
                    "url":"https://synthetic.example/", "attached":false
                }]}),
                "Browser.getWindowForTarget" => serde_json::json!({"windowId":42}),
                "Target.attachToTarget" => serde_json::json!({"sessionId":"fixture-session"}),
                "Runtime.evaluate" => {
                    serde_json::json!({"result":{"type":"string", "value":"https://synthetic.example/"}})
                }
                _ => serde_json::json!({}),
            };
            socket
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    serde_json::json!({"id":request["id"], "result":result})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let fixture = wire();
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let (header_end, length) = loop {
                let mut chunk = [0_u8; 4096];
                let count = stream.read(&mut chunk).await.unwrap();
                assert!(count > 0 && bytes.len() + count < 65536);
                bytes.extend_from_slice(&chunk[..count]);
                if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    let header = std::str::from_utf8(&bytes[..end]).unwrap();
                    let length = header
                        .lines()
                        .find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break (end + 4, length);
                    }
                }
            };
            let envelope: Value =
                serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap();
            let response = match envelope["request"]["operation"].as_str().unwrap() {
                "inventory" => fixture["inventory"].clone(),
                "observe_assignment" => fixture["assignmentObservation"].clone(),
                "windows" => {
                    let mut windows = fixture["windows"].clone();
                    windows["windows"][0]["pid"] = serde_json::json!(std::process::id());
                    windows
                }
                "activate" => fixture["activation"].clone(),
                _ => panic!("queued resolver must not launch or acquire"),
            }
            .to_string();
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).as_bytes()).await.unwrap();
        }
    });
    let runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig {
        headless: true,
        executable_path: None,
        display: None,
        remote_headed: false,
        maximum_browser_processes: None,
        remote_view_desktop_contexts: Vec::new(),
    })
    .unwrap();
    let transport =
        RemoteViewApplicationHttp::new(&origin, std::time::Duration::from_secs(2)).unwrap();
    let effects = RemoteViewSessionEffects::new(
        BrowserSessionEffectAdapter::new(runtime),
        RemoteViewApplicationAdapter::new("agent-browser".into(), transport).unwrap(),
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
    let registered = BrowserSessionHost::load(
        fixture.store(false),
        RuntimeSessionEffects::Remote(Box::new(effects)),
        &fixture.root.join("unused.json"),
        BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: Vec::new(),
            default_disposable_policy: None,
            exact_url_history_maximum_bytes: 1024,
        },
    )
    .unwrap();
    let shared = std::sync::Arc::new(std::sync::Mutex::new(Some(registered)));
    let mut state =
        crate::native::action_runtime::DaemonState::new().with_managed_session_host(shared.clone());
    let response = crate::native::actions::execute_command(
        &serde_json::json!({
            "id":"queued-resolve", "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "activityAtMs":2001,
        }),
        &mut state,
    )
    .await;
    assert_eq!(response["success"], true, "{response}");
    assert_eq!(response["data"]["handoffId"], id);
    assert_eq!(response["data"]["operatorVisible"]["state"], "ready");
    let after = fixture.store(false).load_session_state().unwrap();
    assert!(after.remote_view_tab_handoffs[&id].view.is_none());
    assert_eq!(after.browsers, before.browsers);
    let mut expected_sessions = before.sessions.clone();
    for session in expected_sessions.values_mut() {
        session.last_activity_at_ms = 2001;
        session.expires_at_ms = 302001;
    }
    assert_eq!(after.sessions, expected_sessions);
    let mut expected_tabs = before.tabs.clone();
    for tab in expected_tabs.values_mut() {
        tab.last_activity_at_ms = 2001;
    }
    assert_eq!(after.tabs, expected_tabs);
    assert_eq!(after.navigation_history, before.navigation_history);
    assert_eq!(shared.lock().unwrap().as_ref().unwrap().state(), &after);
    let repeated = crate::native::actions::execute_command(
        &serde_json::json!({
            "id":"queued-repeat", "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "activityAtMs":2001,
        }),
        &mut state,
    )
    .await;
    assert_eq!(repeated["success"], true, "{repeated}");
    assert_eq!(fixture.store(false).load_session_state().unwrap(), after);
    server.abort();
    cdp_server.abort();
    let cold = serde_json::json!({
        "id":"queued-cold", "action":"snapshot", "runtimeProfile":"missing-profile",
        "sessionName":"ColdService", "serviceName":"synthetic-service", "agentName":"fixture", "taskName":"selector-check",
    });
    state.confirm_actions = Some(crate::native::policy::ConfirmActions {
        categories: std::collections::HashSet::from(["snapshot".to_string()]),
    });
    let confirmation = crate::native::actions::execute_command(&cold, &mut state).await;
    assert_eq!(confirmation["data"]["confirmation_required"], true);
    state.confirm_actions = None;
    let policy_path = fixture.root.join("deny-snapshot.json");
    std::fs::write(&policy_path, r#"{"default":"allow","deny":["snapshot"]}"#).unwrap();
    state.policy =
        Some(crate::native::policy::ActionPolicy::load(policy_path.to_str().unwrap()).unwrap());
    let denied = crate::native::actions::execute_command(&cold, &mut state).await;
    assert_eq!(denied["success"], false);
    assert!(denied["error"]
        .as_str()
        .unwrap()
        .contains("denied by policy"));
    state.policy = None;
    let queue = crate::native::control_plane::ControlPlaneWorker::start(state);
    let rejected = queue.submit(cold).await;
    assert_eq!(
        rejected["error"], "browser_session_profile_selector_unknown",
        "{rejected}"
    );
    let request = serde_json::json!({
        "action":"snapshot", "runtimeProfile":"missing-profile", "sessionName":"ColdService",
        "serviceName":"synthetic-service", "agentName":"fixture", "taskName":"selector-check",
    });
    let http =
        crate::native::stream::service_request_adapter_fixture(&request.to_string()).unwrap();
    let mcp = crate::mcp::service_request_adapter_fixture_for_session(&request, "service-default")
        .unwrap();
    for command in [http, mcp] {
        assert_eq!(command["runtimeProfile"], "missing-profile");
        assert_eq!(command["sessionName"], "ColdService");
        assert_eq!(command["taskName"], "selector-check");
        let id = command["id"].clone();
        let rejected = queue.submit(command).await;
        assert_eq!(rejected["id"], id);
        assert_eq!(
            rejected["error"], "browser_session_profile_selector_unknown",
            "{rejected}"
        );
    }
    let mut tab_request = request.clone();
    tab_request["action"] = "tab_new".into();
    tab_request["params"] = serde_json::json!({"tabRequestId":"stable-client-retry"});
    let saved_origin = std::env::var_os("AGENT_BROWSER_REMOTE_VIEW_ORIGIN");
    std::env::set_var("AGENT_BROWSER_REMOTE_VIEW_ORIGIN", "http://127.0.0.1:1");
    let service_state = ServiceState::default();
    let normalized = crate::native::service_request::normalize_service_request(
        crate::native::service_request::ServiceRequestNormalization {
            request: &tab_request,
            service_state: Some(&service_state),
            authenticated_principal: None,
            fallback_principal: None,
            request_id: "normalized-tab-request",
            effective_session: Some("service-default"),
        },
    );
    match saved_origin {
        Some(value) => std::env::set_var("AGENT_BROWSER_REMOTE_VIEW_ORIGIN", value),
        None => std::env::remove_var("AGENT_BROWSER_REMOTE_VIEW_ORIGIN"),
    }
    let mut normalized = normalized.unwrap().command;
    normalized["id"] = "normalized-tab-request".into();
    assert_eq!(normalized["runtimeProfile"], "missing-profile");
    assert_eq!(normalized["sessionName"], "ColdService");
    assert_eq!(normalized["tabRequestId"], "stable-client-retry");
    let rejected = queue.submit(normalized).await;
    assert_eq!(
        rejected["error"], "browser_session_profile_selector_unknown",
        "{rejected}"
    );
    assert_eq!(fixture.store(false).load_session_state().unwrap(), after);
    queue.shutdown().await;
}

#[test]
fn ordinary_tab_creation_replays_exact_result_and_retains_unknown_outcome_across_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let first = serde_json::json!({"id":"tab-first", "action":"tab_new", "runtimeProfile":"profile-a",
        "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &first)
        .unwrap()
        .unwrap();
    assert_eq!(opened["success"], true);
    assert_eq!(consumer.state().tabs.len(), 1);
    assert!(consumer.state().navigation_history.is_empty());
    let mut next = first.clone();
    next["id"] = "tab-next".into();
    next["tabRequestId"] = "stable-next".into();
    next["url"] = "https://synthetic.example/second".into();
    next["activityAtMs"] = 101.into();
    let second = consumer
        .execute_managed_command("Alice", &next)
        .unwrap()
        .unwrap();
    assert_eq!(second["success"], true);
    assert_ne!(
        opened["browserSession"]["tabId"],
        second["browserSession"]["tabId"]
    );
    assert_ne!(
        opened["browserSession"]["handoffId"],
        second["browserSession"]["handoffId"]
    );
    assert_eq!(consumer.state().tabs.len(), 2);
    assert_eq!(calls.get(), 1);
    let state = consumer.state().clone();
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    assert_eq!(
        restarted
            .execute_managed_command("Alice", &next)
            .unwrap()
            .unwrap(),
        second
    );
    assert_eq!(restarted.state(), &state);
    next["id"] = "transport-retry".into();
    let mut replay = second.clone();
    replay["id"] = "transport-retry".into();
    assert_eq!(
        restarted
            .execute_managed_command("Alice", &next)
            .unwrap()
            .unwrap(),
        replay
    );
    assert_eq!(restarted.state(), &state);
    let session_id = second["browserSession"]["sessionId"].as_str().unwrap();
    restarted.close_current_tab(session_id, 102).unwrap();
    assert_eq!(
        restarted
            .execute_managed_command("Alice", &next)
            .unwrap_err(),
        "browser_session_tab_request_target_closed"
    );
    assert_eq!(restarted.state().tabs.len(), 1);
    next["url"] = "https://synthetic.example/changed".into();
    assert_eq!(
        restarted
            .execute_managed_command("Alice", &next)
            .unwrap_err(),
        "browser_session_tab_request_conflict"
    );
    drop(restarted);
    let mut unknown = host_mode(&fixture, provider.clone(), calls.clone(), Mode::UnknownTab);
    next["id"] = "tab-unknown".into();
    next.as_object_mut().unwrap().remove("tabRequestId");
    assert_eq!(
        unknown.execute_managed_command("Alice", &next).unwrap_err(),
        "synthetic_tab_reply_lost"
    );
    let pending = fixture.store(false).load_session_state().unwrap();
    assert!(pending.managed_tab_requests["tab-unknown"]
        .response
        .is_none());
    assert_eq!(pending.tabs.len(), 1);
    drop(unknown);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    assert_eq!(
        restarted
            .execute_managed_command("Alice", &next)
            .unwrap_err(),
        "browser_session_tab_creation_readback_required"
    );
    next["id"] = "another-attempt".into();
    assert_eq!(
        restarted
            .execute_managed_command("Alice", &next)
            .unwrap_err(),
        "browser_session_tab_creation_readback_required"
    );
    assert_eq!(
        restarted.execute_managed_command("Bob", &next).unwrap_err(),
        "browser_session_tab_creation_readback_required"
    );
    assert_eq!(fixture.store(false).load_session_state().unwrap(), pending);
    assert_eq!(calls.get(), 1);
}

#[test]
fn remote_view_focus_requires_the_owned_active_window() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut consumer = host(&fixture, provider.clone(), Rc::new(Cell::new(0)));
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let pid = consumer.state().browsers[&opened.browser_id].pid;
    provider.borrow_mut().window_pid = Some(pid.saturating_add(1));
    assert_eq!(
        consumer
            .focus_browser(&opened.browser_id, None, 101)
            .unwrap_err(),
        "remote_view_focus_owned_window_unproven"
    );
    provider.borrow_mut().window_pid = Some(pid);
    provider.borrow_mut().inactive_window = true;
    assert_eq!(
        consumer
            .focus_browser(&opened.browser_id, None, 102)
            .unwrap_err(),
        "remote_view_focus_owned_window_unproven"
    );
    provider.borrow_mut().inactive_window = false;
    consumer
        .focus_browser(&opened.browser_id, None, 103)
        .unwrap();
}

#[test]
fn handoff_link_retention_extension_and_expiry_do_not_change_browser_or_issue_viewer_grants() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a",
                "activityAtMs":100,"handoffTtlMs":10,
            }),
        )
        .unwrap()
        .unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(opened["browserSession"]["handoffExpiresAtMs"], 110);
    let browsers = consumer.state().browsers.clone();
    let sessions = consumer.state().sessions.clone();
    let tabs = consumer.state().tabs.clone();
    let history = consumer.state().navigation_history.clone();
    let extended = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,
        "handoffOperation":"extend","ttlMs":20,"activityAtMs":105,
    }));
    assert_eq!(extended["success"], true, "{extended}");
    assert_eq!(extended["data"]["expiresAtMs"], 125);
    drop(consumer);
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    assert_eq!(
        consumer.state().remote_view_tab_handoffs[&id].expires_at_ms,
        Some(125)
    );
    for operation in [None, Some("extend")] {
        let mut request = serde_json::json!({"action":"service_remote_view_handoff_resolve",
            "handoffId":id,"nativeDesktop":true,"activityAtMs":125,"ttlMs":20});
        if let Some(operation) = operation {
            request["handoffOperation"] = serde_json::json!(operation);
        }
        let expired = consumer.handle_command(&request);
        assert_eq!(expired["success"], false);
        assert_eq!(expired["error"], "remote_view_handoff_expired");
    }
    let inspected = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,
        "handoffOperation":"inspect","activityAtMs":125}));
    assert_eq!(inspected["data"]["expired"], true);
    assert_eq!(consumer.state().browsers, browsers);
    assert_eq!(consumer.state().sessions, sessions);
    assert_eq!(consumer.state().tabs, tabs);
    assert_eq!(consumer.state().navigation_history, history);
    assert!(provider.borrow().view_requests.is_empty());
    assert_eq!(calls.get(), 1);
    let reopened = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate","url":"https://synthetic.example/", "profileId":"profile-a",
                "activityAtMs":126,"handoffTtlMs":10,
            }),
        )
        .unwrap()
        .unwrap();
    assert_ne!(reopened["browserSession"]["handoffId"], id);
    assert_eq!(
        reopened["browserSession"]["targetId"],
        opened["browserSession"]["targetId"]
    );
    assert_eq!(reopened["browserSession"]["handoffExpiresAtMs"], 136);
    assert!(consumer.state().remote_view_tab_handoffs[&id]
        .check_link_at(126)
        .is_err());
}

#[test]
fn durable_named_handoff_has_no_default_expiry_across_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(opened["browserSession"]["handoffExpiresAtMs"].is_null());
    drop(consumer);
    let consumer = host(&fixture, provider, calls);
    let link = &consumer.state().remote_view_tab_handoffs[&id];
    assert_eq!(link.expires_at_ms, None);
    link.check_link_at(86_400_101).unwrap();
}

#[test]
fn configured_durable_retention_survives_restart_and_applies_to_new_links() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut store = fixture.store(false);
    let original = store.load_session_state().unwrap();
    let mut configured = serde_json::to_value(&original).unwrap();
    configured["retentionPolicy"] = serde_json::json!({"durableHandoffTtlMs":10, "disposableHandoffTtlMs":20, "disposableInactivityMs":30});
    let configured: BrowserSessionState = serde_json::from_value(configured).unwrap();
    store
        .compare_and_save_session_state(&original, &configured)
        .unwrap();
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(opened["browserSession"]["handoffExpiresAtMs"], 110);
    drop(consumer);
    let consumer = host(&fixture, provider, calls);
    assert_eq!(
        consumer.state().remote_view_tab_handoffs[&id].expires_at_ms,
        Some(110)
    );
    assert_eq!(
        consumer.state().remote_view_tab_handoffs[&id]
            .check_link_at(110)
            .unwrap_err(),
        "remote_view_handoff_expired"
    );
}

#[test]
fn disposable_inactivity_expires_work_and_reaps_only_unreferenced_profile_data() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut store = fixture.store(false);
    let original = store.load_session_state().unwrap();
    let mut configured = serde_json::to_value(&original).unwrap();
    configured["retentionPolicy"] = serde_json::json!({"durableHandoffTtlMs":null, "disposableHandoffTtlMs":20, "disposableInactivityMs":30});
    store
        .compare_and_save_session_state(&original, &serde_json::from_value(configured).unwrap())
        .unwrap();
    let mut consumer = host(&fixture, provider, calls);
    let opened = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    let identity = &opened["browserSession"];
    let session = identity["sessionId"].as_str().unwrap().to_owned();
    let profile = identity["profileId"].as_str().unwrap().to_owned();
    assert_eq!(identity["handoffExpiresAtMs"], 120);
    assert!(consumer.reap(129).unwrap().expired_session_ids.is_empty());
    assert_eq!(
        consumer.reap(130).unwrap().expired_session_ids,
        vec![session]
    );
    assert!(consumer.state().disposable_profiles.contains_key(&profile));
    assert!(consumer
        .reap(300_129)
        .unwrap()
        .deleted_disposable_profile_ids
        .is_empty());
    assert_eq!(
        consumer
            .reap(300_130)
            .unwrap()
            .deleted_disposable_profile_ids,
        vec![profile]
    );
    assert!(consumer.state().disposable_profiles.is_empty());
}

#[test]
fn native_overflow_wraps_after_restart_without_same_profile_advancing_cursor() {
    let fixture = Fixture::new();
    let mut catalog = fixture.store(false).load_profile_catalog().unwrap().catalog;
    for id in ["profile-c", "profile-d", "profile-e"] {
        catalog.profiles.insert(
            id.into(),
            BrowserProfileCatalogEntry {
                id: id.into(),
                name: id.into(),
                user_data_dir: fixture.root.join(id).to_string_lossy().into_owned(),
                kind: BrowserProfileKind::Named,
            },
        );
    }
    fixture.store(false).save_profile_catalog(&catalog).unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let mut opened = Vec::new();
    for (i, profile) in ["profile-a", "profile-b", "profile-c", "profile-d"]
        .iter()
        .enumerate()
    {
        let item = consumer
            .open(OpenBrowserSession::exact_profile(
                format!("Task{i}"),
                *profile,
                100 + i as u64,
            ))
            .unwrap();
        let desktop = consumer.state().browsers[&item.browser_id]
            .desktop
            .as_ref()
            .unwrap()
            .desktop_id
            .clone();
        provider
            .borrow_mut()
            .occupied_desktops
            .insert(desktop.clone());
        opened.push((item, desktop));
    }
    assert_ne!(opened[0].1, opened[1].1);
    assert_eq!(opened[2].1, opened[0].1);
    assert_eq!(opened[3].1, opened[1].1);
    consumer
        .close_session(
            &opened[1].0.session_id,
            SessionEndReason::ExplicitClose,
            110,
        )
        .unwrap();
    let peer = consumer
        .open(OpenBrowserSession::exact_profile("Peer", "profile-a", 111))
        .unwrap();
    assert_eq!(peer.browser_id, opened[0].0.browser_id);
    drop(consumer);
    let mut consumer = host(&fixture, provider.clone(), calls);
    let next = consumer
        .open(OpenBrowserSession::exact_profile("Next", "profile-e", 112))
        .unwrap();
    assert_eq!(
        consumer.state().browsers[&next.browser_id]
            .desktop
            .as_ref()
            .unwrap()
            .desktop_id,
        opened[0].1
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
}

#[test]
fn background_pool_replenishes_one_spare_without_launching_or_duplicating() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    let mut maintenance = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    assert!(
        replenish_remote_view_session_pool(&mut maintenance, &mut store, &pool)
            .unwrap()
            .is_none()
    );
    assert!(provider.borrow().acquisitions.is_empty());
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let first = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let desktop = consumer.state().browsers[&first.browser_id]
        .desktop
        .as_ref()
        .unwrap()
        .desktop_id
        .clone();
    provider.borrow_mut().occupied_desktops.insert(desktop);
    let spare = replenish_remote_view_session_pool(&mut maintenance, &mut store, &pool)
        .unwrap()
        .unwrap();
    assert_eq!(spare.assignments.len(), 2);
    assert_eq!(spare.desktops.len(), 1);
    assert_eq!(calls.get(), 1);
    let second = replenish_remote_view_session_pool(&mut maintenance, &mut store, &pool)
        .unwrap()
        .unwrap();
    assert_eq!(second.desktops, spare.desktops);
    assert_eq!(provider.borrow().acquisitions.len(), 2);
    let next = consumer
        .open(OpenBrowserSession::exact_profile("Bob", "profile-b", 101))
        .unwrap();
    assert_eq!(
        consumer.state().browsers[&next.browser_id]
            .desktop
            .as_ref()
            .unwrap()
            .desktop_id,
        spare.desktops[0].desktop.desktop_id
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
}

#[test]
fn unknown_spare_acquisition_cannot_block_a_separately_confirmed_free_desktop() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let first = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let desktop = consumer.state().browsers[&first.browser_id]
        .desktop
        .as_ref()
        .unwrap()
        .desktop_id
        .clone();
    provider
        .borrow_mut()
        .occupied_desktops
        .insert(desktop.clone());
    provider.borrow_mut().lose_reply = true;
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    assert!(replenish_remote_view_session_pool(
        &mut adapter(&fixture, provider.clone()),
        &mut fixture.store(false),
        &pool
    )
    .is_err());
    assert_eq!(provider.borrow().acquisitions.len(), 2);
    consumer
        .close_session(&first.session_id, SessionEndReason::ExplicitClose, 101)
        .unwrap();
    provider.borrow_mut().occupied_desktops.remove(&desktop);
    let next = consumer
        .open(OpenBrowserSession::exact_profile("Bob", "profile-b", 102))
        .unwrap();
    assert_eq!(
        consumer.state().browsers[&next.browser_id]
            .desktop
            .as_ref()
            .unwrap()
            .desktop_id,
        desktop
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
    assert_eq!(
        fixture
            .store(false)
            .pending_pool_acquisitions("agent-browser", "main")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn configured_handoff_overflow_is_rejected_before_profile_or_browser_effects() {
    let fixture = Fixture::new();
    let mut store = fixture.store(false);
    let original = store.load_session_state().unwrap();
    let mut configured = original.clone();
    configured.retention_policy.durable_handoff_ttl_ms = Some(u64::MAX);
    store
        .compare_and_save_session_state(&original, &configured)
        .unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    assert!(consumer.execute_managed_command("Alice",&serde_json::json!({
        "action":"navigate","url":"https://synthetic.example/","profileId":"profile-a","activityAtMs":100
    })).is_err());
    assert_eq!(calls.get(), 0);
    assert!(provider.borrow().acquisitions.is_empty());
    assert!(consumer.state().sessions.is_empty());
}

#[test]
fn native_idle_pool_returns_capacity_and_recovers_same_logical_handoff() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        pool_maximum: Some(3),
        ..Provider::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let mut opened = Vec::new();
    for (name, profile) in [("Alice", "profile-a"), ("Bob", "profile-b")] {
        let response = consumer
            .execute_managed_command(
                name,
                &serde_json::json!({"action":"navigate",
            "url":"https://synthetic.example/","profileId":profile,"activityAtMs":100}),
            )
            .unwrap()
            .unwrap();
        let browser = response["browserSession"]["browserId"].as_str().unwrap();
        provider.borrow_mut().occupied_desktops.insert(
            consumer.state().browsers[browser]
                .desktop
                .as_ref()
                .unwrap()
                .desktop_id
                .clone(),
        );
        opened.push(response);
    }
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    replenish_remote_view_session_pool(
        &mut adapter(&fixture, provider.clone()),
        &mut fixture.store(false),
        &pool,
    )
    .unwrap();
    assert_eq!(provider.borrow().acquisitions.len(), 3);
    assert_eq!(consumer.reap(300_100).unwrap().closed_browser_ids.len(), 2);
    provider.borrow_mut().occupied_desktops.clear();
    super::super::browser_session_pool_maintenance::maintain_pool(
        &mut adapter(&fixture, provider.clone()),
        &mut fixture.store(false),
        &pool,
    )
    .unwrap();
    assert_eq!(
        provider
            .borrow()
            .assignments
            .iter()
            .filter(|a| a.state == RemoteViewAssignmentState::Active)
            .count(),
        1
    );
    drop(consumer);
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let resumed = consumer
        .execute_managed_command(
            "Bob",
            &serde_json::json!({"action":"get_url","activityAtMs":300_101}),
        )
        .unwrap()
        .unwrap();
    for field in ["handoffId", "browserId", "sessionId", "tabId"] {
        assert_eq!(
            resumed["browserSession"][field],
            opened[1]["browserSession"][field]
        );
    }
    assert_ne!(
        resumed["browserSession"]["targetId"],
        opened[1]["browserSession"]["targetId"]
    );
    assert_eq!(calls.get(), 3);
}

#[test]
fn native_idle_cleanup_keeps_viewer_attached_browser_running() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(&fixture, provider.clone(), calls, Mode::IdleRecovery);
    consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({"action":"navigate","profileId":"profile-a",
        "url":"https://synthetic.example/","activityAtMs":100}),
        )
        .unwrap()
        .unwrap();
    let desktop = consumer
        .state()
        .browsers
        .values()
        .next()
        .unwrap()
        .desktop
        .as_ref()
        .unwrap()
        .desktop_id
        .clone();
    provider
        .borrow_mut()
        .viewer_active_desktops
        .insert(desktop.clone());
    assert!(consumer
        .reap(300_100)
        .unwrap()
        .closed_browser_ids
        .is_empty());
    assert!(!fixture.root.join("idle-process-absent").exists());
    provider
        .borrow_mut()
        .viewer_active_desktops
        .remove(&desktop);
    assert_eq!(consumer.reap(300_101).unwrap().closed_browser_ids.len(), 1);
}

#[test]
fn native_idle_release_deferral_and_lost_reply_preserve_exact_custody() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        pool_maximum: Some(3),
        ..Provider::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls);
    let first = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    provider.borrow_mut().occupied_desktops.insert(
        consumer.state().browsers[&first.browser_id]
            .desktop
            .as_ref()
            .unwrap()
            .desktop_id
            .clone(),
    );
    let second = consumer
        .open(OpenBrowserSession::exact_profile("Bob", "profile-b", 100))
        .unwrap();
    let desktop = consumer.state().browsers[&second.browser_id]
        .desktop
        .as_ref()
        .unwrap()
        .desktop_id
        .clone();
    provider
        .borrow_mut()
        .occupied_desktops
        .insert(desktop.clone());
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    let mut coordinator = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    replenish_remote_view_session_pool(&mut coordinator, &mut store, &pool).unwrap();
    consumer
        .close_session(&second.session_id, SessionEndReason::ExplicitClose, 101)
        .unwrap();
    provider.borrow_mut().occupied_desktops.remove(&desktop);
    provider.borrow_mut().defer_idle_release = true;
    super::super::browser_session_pool_maintenance::maintain_pool(
        &mut coordinator,
        &mut store,
        &pool,
    )
    .unwrap();
    assert!(store.idle_release_obligations().unwrap().is_empty());
    assert_eq!(
        provider
            .borrow()
            .assignments
            .iter()
            .filter(|a| a.state == RemoteViewAssignmentState::Active)
            .count(),
        3
    );
    provider.borrow_mut().defer_idle_release = false;
    provider.borrow_mut().lose_idle_release_reply = true;
    assert_eq!(
        super::super::browser_session_pool_maintenance::maintain_pool(
            &mut coordinator,
            &mut store,
            &pool
        )
        .unwrap_err(),
        "remote_view_release_readback_required"
    );
    assert_eq!(store.idle_release_obligations().unwrap().len(), 1);
    provider.borrow_mut().lose_idle_release_reply = false;
    super::super::browser_session_pool_maintenance::maintain_pool(
        &mut coordinator,
        &mut store,
        &pool,
    )
    .unwrap();
    assert!(store.idle_release_obligations().unwrap().is_empty());
    let requests = &provider.borrow().idle_release_requests;
    assert_eq!(requests.len(), 3);
    assert_ne!(requests[0], requests[1]);
    assert_eq!(requests[1], requests[2]);
    assert_eq!(provider.borrow().acquisitions.len(), 3);
}

#[test]
fn canceled_idle_release_permit_cannot_submit_after_work_resumes() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut coordinator = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    let prepared = prepare_remote_view_session_pool(
        &mut coordinator,
        &mut store,
        &RemoteViewSessionPool {
            name: "main".into(),
            desired_desktops: 1,
        },
    )
    .unwrap();
    let assignment = &prepared.assignments[0];
    let observation = coordinator.observe_idle(assignment).unwrap();
    let target = observation.release_target(assignment).unwrap();
    let clear = RemoteViewApplicationObligationInventory::Complete(Vec::new());
    let snapshot = RemoteViewApplicationCleanupSnapshot {
        schema_version: 1,
        assignment_id: assignment.assignment_id.clone(),
        desktop_id: assignment.desktop_id.clone(),
        lifecycle_generation: assignment.generation,
        pending_recovery: clear.clone(),
        foreground_leases: clear.clone(),
        cleanup_tasks: clear,
    };
    let release_id = uuid::Uuid::new_v4().to_string();
    let permit = store
        .admit_idle_assignment_release(&target, &snapshot, &release_id)
        .unwrap();
    assert!(!store.pool_assignment_available(assignment).unwrap());
    store
        .cancel_unsubmitted_or_deferred_idle_release(&release_id)
        .unwrap();
    assert!(store.pool_assignment_available(assignment).unwrap());
    assert!(coordinator
        .release_idle(
            assignment,
            &observation,
            permit,
            release_id.into(),
            &mut store
        )
        .is_err());
    assert!(provider.borrow().idle_release_requests.is_empty());
    assert_eq!(
        provider.borrow().assignments[0].state,
        RemoteViewAssignmentState::Active
    );
}

#[test]
fn native_idle_viewer_protects_disposable_expiry_and_profile_data() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut store = fixture.store(false);
    let original = store.load_session_state().unwrap();
    let mut configured = serde_json::to_value(&original).unwrap();
    configured["retentionPolicy"] = serde_json::json!({
        "durableHandoffTtlMs":null, "disposableHandoffTtlMs":20, "disposableInactivityMs":30});
    store
        .compare_and_save_session_state(&original, &serde_json::from_value(configured).unwrap())
        .unwrap();
    let mut consumer = host_mode(&fixture, provider.clone(), calls, Mode::IdleRecovery);
    let opened = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    let browser_id = opened["browserSession"]["browserId"].as_str().unwrap();
    let desktop = consumer.state().browsers[browser_id]
        .desktop
        .as_ref()
        .unwrap()
        .desktop_id
        .clone();
    provider
        .borrow_mut()
        .viewer_active_desktops
        .insert(desktop.clone());
    assert!(consumer.reap(130).unwrap().expired_session_ids.is_empty());
    assert!(!fixture.root.join("idle-process-absent").exists());
    assert_eq!(consumer.state().disposable_profiles.len(), 1);
    provider
        .borrow_mut()
        .viewer_active_desktops
        .remove(&desktop);
    assert_eq!(consumer.reap(131).unwrap().expired_session_ids.len(), 1);
    assert_eq!(
        consumer
            .reap(300_131)
            .unwrap()
            .deleted_disposable_profile_ids
            .len(),
        1
    );
}

#[test]
fn pool_demand_regrows_after_exact_native_release_without_local_release_receipt() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut adapter = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    let first = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    provider.borrow_mut().assignments[0].state = RemoteViewAssignmentState::Released;
    let next = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_ne!(
        first.assignments[0].assignment_id,
        next.assignments[0].assignment_id
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
    let history = store
        .confirmed_pool_acquisitions("agent-browser", "main")
        .unwrap();
    assert_eq!(history[0].state, RemoteViewAssignmentState::Active);
    let mut changed = provider.borrow_mut();
    changed.assignments[1].state = RemoteViewAssignmentState::Released;
    changed.assignments[1].generation += 1;
    drop(changed);
    assert_eq!(
        prepare_remote_view_session_pool(&mut adapter, &mut store, &pool)
            .err()
            .unwrap(),
        "remote_view_pool_acquisition_readback_required"
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
}
