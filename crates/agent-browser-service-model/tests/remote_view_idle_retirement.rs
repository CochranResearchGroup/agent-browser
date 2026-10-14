use agent_browser_service_model::*;
use serde_json::{json, Value};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

type Step = (Value, Result<Value, RemoteViewApplicationTransportError>);
struct Transport(Rc<RefCell<VecDeque<Step>>>);
impl RemoteViewApplicationTransport for Transport {
    fn request(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<Value, RemoteViewApplicationTransportError> {
        let (expected, response) = self
            .0
            .borrow_mut()
            .pop_front()
            .expect("unexpected provider request");
        assert_eq!(serde_json::to_value(envelope).unwrap(), expected);
        response
    }
}

#[test]
fn native_retirement_completion_requires_settlement_and_the_exact_prepared_fence() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap();
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let target = fixture["assignmentObservation"]["target"].clone();
    let retirement = json!({"schemaVersion":1,"desktopId":assignment.desktop_id,
        "generation":assignment.generation,"routes":[],"sessions":[]});
    let intent: RemoteViewIdleRetirementIntent = serde_json::from_value(json!({
        "schemaVersion":1,"assignment":assignment,"target":target,"idempotencyKey":"close-one",
        "expectedRevision":0,"expectedRetirement":retirement
    }))
    .unwrap();
    let prepared_json = json!({"schemaVersion":1,"target":target,"readinessScope":"native_admission",
        "state":"prepared","fence":{"schema_version":1,"installation_id":"0123456789abcdef",
            "config_hash":"a".repeat(64),"slot":2,"generation":19,
            "operation_id":"5deb7e63650d881f18b5d68f85ec45bcd7523255499618982a3c2995400637cc","revision":1}});
    let prepared: RemoteViewIdleRetirementReceipt =
        serde_json::from_value(prepared_json.clone()).unwrap();
    let empty = Rc::new(RefCell::new(VecDeque::new()));
    let mut adapter =
        RemoteViewApplicationAdapter::new("agent-browser".into(), Transport(empty.clone()))
            .unwrap();
    assert_eq!(
        adapter
            .complete_idle_retirement(&intent, &prepared, false)
            .unwrap_err(),
        RemoteViewApplicationAdapterError::MutationReadbackRequired
    );
    assert!(
        empty.borrow().is_empty(),
        "unsettled effects must never acknowledge completion"
    );
    let expected = json!({"application":"agent-browser","request":{
        "operation":"complete_idle_retirement","assignment_id":assignment.assignment_id,
        "expected_generation":assignment.generation,"expected_viewing_generation":19,
        "idempotency_key":"close-one","expected_retirement":retirement,
        "fence":prepared_json["fence"],"effects_settled":true
    }});
    for case in 0..5 {
        let mut response = prepared_json.clone();
        response["state"] = json!("completed");
        match case {
            1 => response["fence"]["config_hash"] = json!("b".repeat(64)),
            2 => response["fence"]["operation_id"] = json!("b".repeat(64)),
            3 => response["state"] = json!("prepared"),
            _ => (),
        }
        let mut steps = VecDeque::from([
            (
                fixture["requests"][2].clone(),
                Ok(fixture["assignmentObservation"].clone()),
            ),
            (
                expected.clone(),
                if case == 4 {
                    Err(RemoteViewApplicationTransportError::OutcomeUnknown)
                } else {
                    Ok(response)
                },
            ),
        ]);
        if case == 0 {
            steps.push_back((
                fixture["requests"][2].clone(),
                Ok(fixture["assignmentObservation"].clone()),
            ));
        }
        let steps = Rc::new(RefCell::new(steps));
        let mut adapter =
            RemoteViewApplicationAdapter::new("agent-browser".into(), Transport(steps.clone()))
                .unwrap();
        let result = adapter.complete_idle_retirement(&intent, &prepared, true);
        assert_eq!(
            result.is_ok(),
            case == 0,
            "incorrect completion result for case {case}"
        );
        assert!(steps.borrow().is_empty());
    }
}

#[test]
fn native_retirement_prepare_binds_operation_digest_revision_and_target_without_retry() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap();
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let target = fixture["assignmentObservation"]["target"].clone();
    let retirement = json!({"schemaVersion":1,"desktopId":assignment.desktop_id,
        "generation":assignment.generation,"routes":[],"sessions":[]});
    let idle: RemoteViewIdleAssignmentObservation = serde_json::from_value(json!({
        "schemaVersion":1,"target":target,"readinessScope":"live_resource",
        "retirement":retirement,"idle":false,"viewerActive":false
    }))
    .unwrap();
    let admission: RemoteViewIdleRetirementRead = serde_json::from_value(json!({
        "schemaVersion":1,"target":target,"readinessScope":"native_admission",
        "retirement":{"revision":0,"completed":true,"fence":null}
    }))
    .unwrap();
    let intent =
        RemoteViewIdleRetirementIntent::new(&assignment, &idle, &admission, "close-one").unwrap();
    let expected_digest = "5deb7e63650d881f18b5d68f85ec45bcd7523255499618982a3c2995400637cc";
    let prepared = json!({"schemaVersion":1,"target":target,"readinessScope":"native_admission",
        "state":"prepared","fence":{"schema_version":1,"installation_id":"0123456789abcdef",
            "config_hash":"a".repeat(64),"slot":2,"generation":19,"operation_id":expected_digest,"revision":1}});
    let expected = json!({"application":"agent-browser","request":{
        "operation":"prepare_idle_retirement","assignment_id":assignment.assignment_id,
        "expected_generation":assignment.generation,"expected_viewing_generation":19,
        "idempotency_key":"close-one","expected_revision":0,"expected_retirement":retirement
    }});
    for case in 0..6 {
        let mut response = prepared.clone();
        match case {
            1 => response["target"]["viewingGeneration"] = json!(20),
            2 => response["fence"]["operation_id"] = json!("b".repeat(64)),
            3 => response["fence"]["revision"] = json!(2),
            4 => response["readinessScope"] = json!("provider_record"),
            _ => (),
        }
        let mut steps = VecDeque::from([
            (
                fixture["requests"][2].clone(),
                Ok(fixture["assignmentObservation"].clone()),
            ),
            (
                expected.clone(),
                if case == 5 {
                    Err(RemoteViewApplicationTransportError::OutcomeUnknown)
                } else {
                    Ok(response)
                },
            ),
        ]);
        if case == 0 {
            steps.push_back((
                fixture["requests"][2].clone(),
                Ok(fixture["assignmentObservation"].clone()),
            ));
        }
        let steps = Rc::new(RefCell::new(steps));
        let mut adapter =
            RemoteViewApplicationAdapter::new("agent-browser".into(), Transport(steps.clone()))
                .unwrap();
        let result = adapter.prepare_idle_retirement(&intent);
        assert_eq!(
            result.is_ok(),
            case == 0,
            "incorrect preparation result for case {case}"
        );
        assert!(steps.borrow().is_empty());
    }
}

#[test]
fn native_retirement_read_requires_exact_current_target_and_native_admission_scope() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap();
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let target = fixture["assignmentObservation"]["target"].clone();
    let read_request = json!({"application":"agent-browser","request":{
        "operation":"read_idle_retirement","assignment_id":assignment.assignment_id,
        "expected_generation":assignment.generation,"expected_viewing_generation":target["viewingGeneration"]
    }});
    for case in 0..5 {
        let mut response = json!({"schemaVersion":1,"target":target,
            "readinessScope":"native_admission","retirement":{"revision":0,"fence":null,"completed":true}});
        let mut after = fixture["assignmentObservation"].clone();
        match case {
            1 => response["readinessScope"] = json!("provider_record"),
            2 => response["target"]["viewingGeneration"] = json!(20),
            3 => response["retirement"]["revision"] = json!(1),
            4 => after["target"]["viewingGeneration"] = json!(20),
            _ => (),
        }
        let mut steps = VecDeque::from([
            (
                fixture["requests"][2].clone(),
                Ok(fixture["assignmentObservation"].clone()),
            ),
            (read_request.clone(), Ok(response)),
        ]);
        if case == 0 || case == 4 {
            steps.push_back((fixture["requests"][2].clone(), Ok(after)));
        }
        let steps = Rc::new(RefCell::new(steps));
        let mut adapter =
            RemoteViewApplicationAdapter::new("agent-browser".into(), Transport(steps.clone()))
                .unwrap();
        let result = adapter.read_idle_retirement(&assignment);
        if case == 0 {
            let observed = result.unwrap();
            assert_eq!(observed.retirement.revision, 0);
            assert!(observed.retirement.completed);
        } else {
            assert!(
                result.is_err(),
                "invalid native retirement read was accepted for case {case}"
            );
        }
        assert!(
            steps.borrow().is_empty(),
            "validation made an unexpected additional request"
        );
    }
    let steps = Rc::new(RefCell::new(VecDeque::from([
        (
            fixture["requests"][2].clone(),
            Ok(fixture["assignmentObservation"].clone()),
        ),
        (
            read_request,
            Err(RemoteViewApplicationTransportError::OutcomeUnknown),
        ),
    ])));
    let mut adapter =
        RemoteViewApplicationAdapter::new("agent-browser".into(), Transport(steps.clone()))
            .unwrap();
    assert_eq!(
        adapter.read_idle_retirement(&assignment).unwrap_err(),
        RemoteViewApplicationAdapterError::Transport(
            RemoteViewApplicationTransportError::OutcomeUnknown
        )
    );
    assert!(steps.borrow().is_empty(), "unknown outcomes must not retry");
}
