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
fn cleanup(f: &Value) -> RemoteViewApplicationCleanup {
    serde_json::from_value(f["requests"][11]["request"]["cleanup"].clone()).unwrap()
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
            .release(&target, cleanup(&f), "release-1".into(), &mut store)
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
            .release(&target, cleanup(&f), "release-1".into(), &mut store)
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
                .release(&target, cleanup(&f), "release-1".into(), &mut store)
                .unwrap_err(),
            RemoteViewApplicationAdapterError::MutationReadbackRequired
        );
        let mut changed = target.clone();
        changed.assignment.generation = 8;
        let mut acknowledgement = cleanup(&f);
        acknowledgement.lifecycle_generation = 8;
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
fn false_cleanup_and_unavailable_custody_send_no_release_and_completion_failure_blocks_replay() {
    let f = fixture();
    let target = release_target(&f);
    for field in [
        "applicationReferencesClear",
        "pendingRecoveryClear",
        "foregroundLeasesClear",
        "cleanupTasksClear",
    ] {
        let records: Records = Rc::default();
        let (mut adapter, mut store, _) = setup(vec![], records.clone());
        let mut acknowledgement = f["requests"][11]["request"]["cleanup"].clone();
        acknowledgement[field] = json!(false);
        let acknowledgement = serde_json::from_value(acknowledgement).unwrap();
        assert!(adapter
            .release(&target, acknowledgement, "release-1".into(), &mut store)
            .is_err());
        assert!(records.borrow().is_empty());
    }
    let records: Records = Rc::default();
    let (mut adapter, mut store, _) = setup(vec![], records.clone());
    store.fail_claim = true;
    assert_eq!(
        adapter
            .release(&target, cleanup(&f), "release-1".into(), &mut store)
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
            .release(&target, cleanup(&f), "release-1".into(), &mut store)
            .unwrap_err(),
        RemoteViewApplicationAdapterError::MutationStore(
            RemoteViewApplicationMutationStoreError::Unavailable
        )
    );
    assert_eq!(
        adapter
            .release(&target, cleanup(&f), "release-1".into(), &mut store)
            .unwrap_err(),
        RemoteViewApplicationAdapterError::MutationReadbackRequired
    );
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
