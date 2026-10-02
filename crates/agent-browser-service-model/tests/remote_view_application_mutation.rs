use agent_browser_service_model::*;
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};

type Records = Rc<RefCell<BTreeMap<String, RemoteViewApplicationMutationRecord>>>;
type Step = (Value, Result<Value, RemoteViewApplicationTransportError>);

struct Store {
    records: Records,
    fail_claim: bool,
    fail_complete: bool,
}
impl RemoteViewApplicationMutationStore for Store {
    fn claim(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<RemoteViewApplicationMutationClaim, RemoteViewApplicationMutationStoreError> {
        if self.fail_claim {
            return Err(RemoteViewApplicationMutationStoreError::Unavailable);
        }
        let key = envelope.mutation_key()?;
        let mut records = self.records.borrow_mut();
        if let Some(record) = records.get(&key) {
            if record.envelope != *envelope {
                return Err(RemoteViewApplicationMutationStoreError::Conflict);
            }
            return Ok(RemoteViewApplicationMutationClaim::Existing(Box::new(
                record.clone(),
            )));
        }
        records.insert(
            key,
            RemoteViewApplicationMutationRecord {
                schema_version: 1,
                envelope: envelope.clone(),
                outcome: None,
            },
        );
        Ok(RemoteViewApplicationMutationClaim::New)
    }
    fn complete(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
        outcome: &RemoteViewApplicationMutationOutcome,
    ) -> Result<(), RemoteViewApplicationMutationStoreError> {
        if self.fail_complete {
            return Err(RemoteViewApplicationMutationStoreError::Unavailable);
        }
        let key = envelope.mutation_key()?;
        let mut records = self.records.borrow_mut();
        let record = records
            .get_mut(&key)
            .ok_or(RemoteViewApplicationMutationStoreError::InvalidRecord)?;
        if record.envelope != *envelope {
            return Err(RemoteViewApplicationMutationStoreError::Conflict);
        }
        record.outcome = Some(outcome.clone());
        Ok(())
    }
}

struct Transport {
    steps: Rc<RefCell<VecDeque<Step>>>,
    records: Records,
}
impl RemoteViewApplicationTransport for Transport {
    fn request(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<Value, RemoteViewApplicationTransportError> {
        if let Ok(key) = envelope.mutation_key() {
            let records = self.records.borrow();
            let record = records.get(&key).expect("mutation sent before custody");
            assert_eq!(record.envelope, *envelope);
            assert!(record.outcome.is_none());
        }
        let (expected, response) = self
            .steps
            .borrow_mut()
            .pop_front()
            .expect("duplicate or unexpected transport request");
        assert_eq!(serde_json::to_value(envelope).unwrap(), expected);
        response
    }
}

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap()
}
fn setup(
    steps: Vec<Step>,
    records: Records,
) -> (
    RemoteViewApplicationAdapter<Transport>,
    Store,
    Rc<RefCell<VecDeque<Step>>>,
) {
    let steps = Rc::new(RefCell::new(steps.into()));
    let transport = Transport {
        steps: steps.clone(),
        records: records.clone(),
    };
    (
        RemoteViewApplicationAdapter::new("agent-browser".into(), transport).unwrap(),
        Store {
            records,
            fail_claim: false,
            fail_complete: false,
        },
        steps,
    )
}
fn release_target(f: &Value) -> RemoteViewApplicationReleaseTarget {
    RemoteViewApplicationReleaseTarget {
        assignment: serde_json::from_value(f["assignment"].clone()).unwrap(),
        route_ids: vec![f["grant"]["routeId"].as_str().unwrap().into()],
        viewer_session_ids: vec!["synthetic-viewer-1".into()],
    }
}
fn snapshot(target: &RemoteViewApplicationReleaseTarget) -> RemoteViewApplicationCleanupSnapshot {
    RemoteViewApplicationCleanupSnapshot {
        schema_version: 1,
        assignment_id: target.assignment.assignment_id.clone(),
        desktop_id: target.assignment.desktop_id.clone(),
        lifecycle_generation: target.assignment.generation,
        pending_recovery: RemoteViewApplicationObligationInventory::Complete(vec![]),
        foreground_leases: RemoteViewApplicationObligationInventory::Complete(vec![]),
        cleanup_tasks: RemoteViewApplicationObligationInventory::Complete(vec![]),
    }
}
fn cleanup(target: &RemoteViewApplicationReleaseTarget) -> RemoteViewApplicationCleanupPermit {
    prepare_remote_view_application_cleanup(
        &BrowserSessionState::default(),
        target,
        &snapshot(target),
    )
    .unwrap()
}

#[test]
fn external_mutations_record_custody_before_transport_and_replay_without_effects() {
    let f = fixture();
    let mut steps = Vec::new();
    for (request, response) in [
        (0, "inventory"),
        (1, "assignment"),
        (2, "assignmentObservation"),
        (5, "activation"),
        (2, "assignmentObservation"),
        (2, "assignmentObservation"),
        (7, "viewIssuance"),
        (10, "viewRevocation"),
        (11, "joinedRelease"),
        (0, "inventory"),
        (2, "assignmentObservation"),
        (2, "assignmentObservation"),
        (2, "assignmentObservation"),
    ] {
        steps.push((f["requests"][request].clone(), Ok(f[response].clone())));
    }
    let records: Records = Rc::default();
    let (mut adapter, mut store, steps) = setup(steps, records.clone());
    let target = release_target(&f);
    let grant = serde_json::from_value(f["grant"].clone()).unwrap();
    for _ in 0..2 {
        let acquired = adapter
            .acquire("main".into(), "browser-capacity-1".into(), &mut store)
            .unwrap();
        assert_eq!(serde_json::to_value(acquired).unwrap(), f["assignment"]);
        let activated = adapter
            .activate(&target.assignment, 42, "focus-1".into(), &mut store)
            .unwrap();
        assert_eq!(serde_json::to_value(activated).unwrap(), f["activation"]);
        let view = adapter
            .issue_view(
                &target.assignment,
                RemoteViewApplicationViewOptions {
                    audience: "remote_view".into(),
                    capability: RemoteViewApplicationViewCapability::Control,
                    lifetime_seconds: 300,
                    idempotency_key: "view-1".into(),
                },
                || 2000,
                &mut store,
            )
            .unwrap();
        assert_eq!(serde_json::to_value(view).unwrap(), f["viewIssuance"]);
        let revoked = adapter.revoke_view(&grant, &mut store).unwrap();
        assert_eq!(serde_json::to_value(revoked).unwrap(), f["viewRevocation"]);
        let released = adapter
            .release(&target, cleanup(&target), "release-1".into(), &mut store)
            .unwrap();
        assert_eq!(serde_json::to_value(released).unwrap(), f["joinedRelease"]);
    }
    assert!(steps.borrow().is_empty());
    assert_eq!(records.borrow().len(), 5);
}

#[test]
fn interrupted_unknown_and_inexact_release_preserve_pending_identity_across_restart() {
    let f = fixture();
    let mut partial = f["joinedRelease"].clone();
    partial["retirement"]["sessions"] = json!([]);
    for response in [
        Err(RemoteViewApplicationTransportError::OutcomeUnknown),
        Ok(partial),
    ] {
        let records: Records = Rc::default();
        let (mut adapter, mut store, steps) =
            setup(vec![(f["requests"][11].clone(), response)], records.clone());
        let target = release_target(&f);
        assert!(adapter
            .release(&target, cleanup(&target), "release-1".into(), &mut store)
            .is_err());
        assert!(steps.borrow().is_empty());
        assert!(records
            .borrow()
            .values()
            .all(|record| record.outcome.is_none()));
        // Simulated provider-free store restart, not installed durability proof.
        let bytes = serde_json::to_vec(&*records.borrow()).unwrap();
        let restored: Records = Rc::new(RefCell::new(serde_json::from_slice(&bytes).unwrap()));
        let (mut restarted, mut store, _) = setup(vec![], restored);
        assert_eq!(
            restarted
                .release(&target, cleanup(&target), "release-1".into(), &mut store)
                .unwrap_err(),
            RemoteViewApplicationAdapterError::MutationReadbackRequired
        );
        let mut changed = target.clone();
        changed.assignment.generation = 8;
        let acknowledgement = cleanup(&changed);
        assert_eq!(
            restarted
                .release(&changed, acknowledgement, "release-1".into(), &mut store)
                .unwrap_err(),
            RemoteViewApplicationAdapterError::MutationStore(
                RemoteViewApplicationMutationStoreError::Conflict
            )
        );
    }
}

#[test]
fn mismatched_cleanup_and_unavailable_custody_send_no_release_and_completion_failure_blocks_replay()
{
    let f = fixture();
    let target = release_target(&f);
    let mut changed = target.clone();
    changed.route_ids.clear();
    let records: Records = Rc::default();
    let (mut adapter, mut store, _) = setup(vec![], records.clone());
    assert!(adapter
        .release(&changed, cleanup(&target), "release-1".into(), &mut store)
        .is_err());
    assert!(records.borrow().is_empty());
    let records: Records = Rc::default();
    let (mut adapter, mut store, _) = setup(vec![], records.clone());
    store.fail_claim = true;
    assert_eq!(
        adapter
            .release(&target, cleanup(&target), "release-1".into(), &mut store)
            .unwrap_err(),
        RemoteViewApplicationAdapterError::MutationStore(
            RemoteViewApplicationMutationStoreError::Unavailable
        )
    );
    assert!(records.borrow().is_empty());

    let (mut adapter, mut store, _) = setup(
        vec![(f["requests"][11].clone(), Ok(f["joinedRelease"].clone()))],
        records.clone(),
    );
    store.fail_complete = true;
    assert_eq!(
        adapter
            .release(&target, cleanup(&target), "release-1".into(), &mut store)
            .unwrap_err(),
        RemoteViewApplicationAdapterError::MutationStore(
            RemoteViewApplicationMutationStoreError::Unavailable
        )
    );
    assert_eq!(
        adapter
            .release(&target, cleanup(&target), "release-1".into(), &mut store)
            .unwrap_err(),
        RemoteViewApplicationAdapterError::MutationReadbackRequired
    );
}

#[test]
fn interrupted_acquisition_reuses_original_key_and_requires_current_assignment() {
    let f = fixture();
    let envelope: RemoteViewApplicationEnvelope =
        serde_json::from_value(f["requests"][1].clone()).unwrap();
    let record = RemoteViewApplicationMutationRecord {
        schema_version: 1,
        envelope: envelope.clone(),
        outcome: None,
    };
    let records: Records = Rc::default();
    records
        .borrow_mut()
        .insert(envelope.mutation_key().unwrap(), record.clone());
    let (mut adapter, mut store, steps) = setup(
        vec![
            (f["requests"][1].clone(), Ok(f["assignment"].clone())),
            (f["requests"][0].clone(), Ok(f["inventory"].clone())),
        ],
        records.clone(),
    );
    adapter.reconcile_acquisition(&record, &mut store).unwrap();
    assert!(steps.borrow().is_empty());
    assert!(records.borrow()[&envelope.mutation_key().unwrap()]
        .outcome
        .is_some());
}

#[test]
fn cached_acquisition_cannot_restore_an_assignment_now_released_by_provider() {
    let f = fixture();
    let envelope: RemoteViewApplicationEnvelope =
        serde_json::from_value(f["requests"][1].clone()).unwrap();
    let records: Records = Rc::default();
    records.borrow_mut().insert(
        envelope.mutation_key().unwrap(),
        RemoteViewApplicationMutationRecord {
            schema_version: 1,
            envelope,
            outcome: Some(RemoteViewApplicationMutationOutcome::Acquire(
                serde_json::from_value(f["assignment"].clone()).unwrap(),
            )),
        },
    );
    let mut inventory = f["inventory"].clone();
    inventory["assignments"][0]["state"] = json!("released");
    let (mut adapter, mut store, _) =
        setup(vec![(f["requests"][0].clone(), Ok(inventory))], records);
    assert_eq!(
        adapter
            .acquire("main".into(), "browser-capacity-1".into(), &mut store)
            .unwrap_err(),
        RemoteViewApplicationAdapterError::Response(
            RemoteViewApplicationResponseError::StaleTarget
        )
    );
}

#[test]
fn cleanup_requires_complete_clear_independent_obligation_inventories() {
    use RemoteViewApplicationCleanupError as Error;
    use RemoteViewApplicationObligationInventory as Inventory;
    let f = fixture();
    let target = release_target(&f);
    let state = BrowserSessionState::default();
    assert_eq!(
        serde_json::to_value(cleanup(&target).acknowledgement()).unwrap(),
        f["requests"][11]["request"]["cleanup"]
    );
    for (index, blocked) in [
        Error::PendingRecoveryRemains,
        Error::ForegroundLeasesRemain,
        Error::CleanupTasksRemain,
    ]
    .into_iter()
    .enumerate()
    {
        for (inventory, expected) in [
            (Inventory::Unknown, Error::EvidenceIncomplete),
            (Inventory::Complete(vec!["obligation-1".into()]), blocked),
            (
                Inventory::Complete(vec!["duplicate".into(), "duplicate".into()]),
                Error::InvalidEvidence,
            ),
        ] {
            let mut evidence = snapshot(&target);
            *match index {
                0 => &mut evidence.pending_recovery,
                1 => &mut evidence.foreground_leases,
                _ => &mut evidence.cleanup_tasks,
            } = inventory;
            assert_eq!(
                prepare_remote_view_application_cleanup(&state, &target, &evidence).unwrap_err(),
                expected
            );
        }
    }
    let mut evidence = snapshot(&target);
    evidence.lifecycle_generation += 1;
    assert_eq!(
        prepare_remote_view_application_cleanup(&state, &target, &evidence).unwrap_err(),
        Error::IdentityConflict
    );
}

#[test]
fn cleanup_counts_live_browser_affinity_without_retained_presentation() {
    use RemoteViewApplicationCleanupError as Error;
    let target = release_target(&fixture());
    let mut state = BrowserSessionState::default();
    state.browsers.insert(
        "browser-1".into(),
        ManagedBrowserInstance {
            id: "browser-1".into(),
            profile_id: "profile-1".into(),
            pid: 42,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            active_session_ids: vec![],
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: target.assignment.desktop_id.clone(),
                friendly_route_label: "synthetic".into(),
                generation: target.assignment.generation,
            }),
        },
    );
    assert!(state.remote_view_presentations.is_empty());
    assert_eq!(
        prepare_remote_view_application_cleanup(&state, &target, &snapshot(&target)).unwrap_err(),
        Error::ApplicationReferencesRemain
    );
    state
        .browsers
        .get_mut("browser-1")
        .unwrap()
        .desktop
        .as_mut()
        .unwrap()
        .generation += 1;
    assert_eq!(
        prepare_remote_view_application_cleanup(&state, &target, &snapshot(&target)).unwrap_err(),
        Error::IdentityConflict
    );
    state.browsers.clear();
    state.sessions.insert(
        "session-1".into(),
        ManagedBrowserSession {
            id: "session-1".into(),
            name: "synthetic".into(),
            profile_id: "profile-1".into(),
            browser_id: "browser-1".into(),
            created_at_ms: 1,
            last_activity_at_ms: 2,
            expires_at_ms: 10,
            current_tab_id: None,
        },
    );
    assert_eq!(
        prepare_remote_view_application_cleanup(&state, &target, &snapshot(&target)).unwrap_err(),
        Error::EvidenceIncomplete
    );
}

#[test]
fn detached_retention_does_not_clear_current_references_but_terminal_history_can_remain() {
    use RemoteViewApplicationCleanupError as Error;
    let target = release_target(&fixture());
    let mut state = BrowserSessionState::default();
    state.remote_view_presentations.insert(
        "browser-1".into(),
        RemoteViewPresentationRetention {
            browser_id: "browser-1".into(),
            profile_id: "profile-1".into(),
            session_id: "session-1".into(),
            tab_id: "tab-1".into(),
            target_id: "target-1".into(),
            registration_id: target.assignment.registration_id.clone(),
            pool_id: target.assignment.pool_id.clone(),
            desktop_id: target.assignment.desktop_id.clone(),
            generation: target.assignment.generation,
            assignment_id: target.assignment.assignment_id.clone(),
            placement_id: String::new(),
            route_id: target.route_ids[0].clone(),
            viewer_session_ids: target.viewer_session_ids.clone(),
            state: RemoteViewPresentationRetentionState::Detached,
        },
    );
    state.tabs.insert(
        "tab-1".into(),
        ManagedBrowserTab {
            id: "tab-1".into(),
            target_id: "target-1".into(),
            browser_id: "browser-1".into(),
            session_id: "session-1".into(),
            created_at_ms: 1,
            last_activity_at_ms: 2,
        },
    );
    assert_eq!(
        prepare_remote_view_application_cleanup(&state, &target, &snapshot(&target)).unwrap_err(),
        Error::ApplicationReferencesRemain
    );
    state.tabs.clear();
    state
        .remote_view_presentations
        .get_mut("browser-1")
        .unwrap()
        .state = RemoteViewPresentationRetentionState::Released;
    assert!(prepare_remote_view_application_cleanup(&state, &target, &snapshot(&target)).is_ok());
}
