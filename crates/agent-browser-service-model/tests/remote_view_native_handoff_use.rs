use agent_browser_service_model::{
    RemoteViewApplicationAdapter, RemoteViewApplicationEnvelope, RemoteViewApplicationTransport,
    RemoteViewApplicationTransportError, RemoteViewNativeHandoffObservation,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

#[test]
fn native_handoff_use_joins_existing_consumer_wire_without_a_viewer_grant() {
    for request in [
        json!({
            "operation":"register_native_handoff_observation",
            "assignment_id":"synthetic-assignment", "expected_generation":7,
            "expected_viewing_generation":41, "target_digest":"synthetic-target",
            "idempotency_key":"synthetic-handoff", "lifetime_ms":300_000
        }),
        json!({
            "operation":"observe_native_handoff_use",
            "correlation_id":"a".repeat(64), "target_digest":"synthetic-target"
        }),
    ] {
        let wire = json!({"application":"agent-browser", "request":request});
        let parsed: RemoteViewApplicationEnvelope = serde_json::from_value(wire.clone())
            .expect("integrated native-use contract must be available to the existing consumer");
        assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
    }
    for operation in ["record_native_handoff_use", "confirm_human_use"] {
        assert!(
            serde_json::from_value::<RemoteViewApplicationEnvelope>(json!({
                "application":"agent-browser", "request":{
                    "operation":operation, "correlation_id":"a".repeat(64),
                    "operator":"forged", "successful":true
                }
            }))
            .is_err(),
            "a consumer must not assert successful use"
        );
    }
}

struct Transport {
    replies: VecDeque<Result<Value, RemoteViewApplicationTransportError>>,
    requests: Rc<RefCell<Vec<Value>>>,
}

impl RemoteViewApplicationTransport for Transport {
    fn request(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<Value, RemoteViewApplicationTransportError> {
        self.requests
            .borrow_mut()
            .push(serde_json::to_value(envelope).unwrap());
        self.replies.pop_front().expect("unexpected provider retry")
    }
}

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap()
}

fn registration() -> Value {
    let mut hash = Sha256::new();
    hash.update(("agent-browser".len() as u64).to_be_bytes());
    hash.update(b"agent-browser");
    hash.update(b"synthetic-handoff");
    json!({
        "correlationId":"a".repeat(64), "requestDigest":format!("{:x}", hash.finalize()),
        "application":"agent-browser", "target":fixture()["assignmentObservation"]["target"],
        "targetDigest":"synthetic-target", "clock":{"bootId":"synthetic-boot","elapsedMs":100},
        "lifetimeMs":300_000, "used":null
    })
}

fn used(mut record: Value) -> Value {
    record["used"] = json!({
        "recordedAtMs":1000, "evidence":{
            "frameSync":12, "context":{
                "correlationId":record["correlationId"], "operator":"native-operator",
                "viewerConnectionId":"native-connection",
                "desktopId":record["target"]["viewingDesktopId"],
                "lifecycleGeneration":record["target"]["lifecycleGeneration"],
                "viewingGeneration":record["target"]["viewingGeneration"],
                "targetDigest":record["targetDigest"]
            }
        }
    });
    record
}

#[test]
fn native_registration_binds_request_and_rechecks_both_generations() {
    let fixture = fixture();
    let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
    for path in [
        "",
        "/application",
        "/requestDigest",
        "/targetDigest",
        "/lifetimeMs",
        "/target/viewingGeneration",
    ] {
        let mut record = registration();
        if !path.is_empty() {
            let value = record.pointer_mut(path).unwrap();
            *value = if value.is_number() {
                json!(999)
            } else {
                json!("wrong")
            };
        }
        let requests = Rc::new(RefCell::new(Vec::new()));
        let mut adapter = RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            Transport {
                replies: VecDeque::from([
                    Ok(fixture["assignmentObservation"].clone()),
                    Ok(json!({"schemaVersion":1,"observation":record})),
                    Ok(fixture["assignmentObservation"].clone()),
                ]),
                requests: requests.clone(),
            },
        )
        .unwrap();
        let result = adapter.register_native_handoff_observation(
            &assignment,
            "synthetic-target".into(),
            "synthetic-handoff".into(),
            300_000,
        );
        assert_eq!(result.is_ok(), path.is_empty(), "{path}");
        let requests = requests.borrow();
        assert_eq!(
            requests[1]["request"]["operation"],
            "register_native_handoff_observation"
        );
        assert_eq!(
            requests[1]["request"]["expected_viewing_generation"],
            fixture["assignmentObservation"]["target"]["viewingGeneration"]
        );
    }
    let mut changed = fixture["assignmentObservation"].clone();
    changed["target"]["viewingGeneration"] = json!(999);
    let requests = Rc::new(RefCell::new(Vec::new()));
    let mut adapter = RemoteViewApplicationAdapter::new(
        "agent-browser".into(),
        Transport {
            replies: VecDeque::from([
                Ok(fixture["assignmentObservation"].clone()),
                Ok(json!({"schemaVersion":1,"observation":registration()})),
                Ok(changed),
            ]),
            requests,
        },
    )
    .unwrap();
    assert!(adapter
        .register_native_handoff_observation(
            &assignment,
            "synthetic-target".into(),
            "synthetic-handoff".into(),
            300_000,
        )
        .is_err());
}

#[test]
fn native_use_read_requires_exact_provider_context_and_never_retries_lost_reply() {
    let fixture = fixture();
    let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let registered: RemoteViewNativeHandoffObservation =
        serde_json::from_value(registration()).unwrap();
    for path in [
        "",
        "/correlationId",
        "/clock/elapsedMs",
        "/targetDigest",
        "/used/recordedAtMs",
        "/used/evidence/context/operator",
        "/used/evidence/context/correlationId",
        "/used/evidence/context/desktopId",
        "/used/evidence/context/lifecycleGeneration",
        "/used/evidence/context/viewingGeneration",
        "/used/evidence/context/targetDigest",
    ] {
        let mut record = used(registration());
        if !path.is_empty() {
            let value = record.pointer_mut(path).unwrap();
            *value = if value.is_number() {
                json!(0)
            } else {
                json!("")
            };
        }
        let requests = Rc::new(RefCell::new(Vec::new()));
        let mut adapter = RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            Transport {
                replies: VecDeque::from([
                    Ok(fixture["assignmentObservation"].clone()),
                    Ok(json!({"schemaVersion":1,"state":"used","observation":record})),
                    Ok(fixture["assignmentObservation"].clone()),
                ]),
                requests: requests.clone(),
            },
        )
        .unwrap();
        let result = adapter.observe_native_handoff_use(&assignment, &registered);
        assert_eq!(result.is_ok(), path.is_empty(), "{path}");
        assert_eq!(
            requests.borrow()[1]["request"],
            json!({
                "operation":"observe_native_handoff_use", "correlation_id":"a".repeat(64),
                "target_digest":"synthetic-target"
            })
        );
    }
    for state in ["pending", "expired", "used", "connected"] {
        let requests = Rc::new(RefCell::new(Vec::new()));
        let mut adapter = RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            Transport {
                replies: VecDeque::from([
                    Ok(fixture["assignmentObservation"].clone()),
                    Ok(json!({"schemaVersion":1,"state":state,"observation":registration()})),
                    Ok(fixture["assignmentObservation"].clone()),
                ]),
                requests,
            },
        )
        .unwrap();
        let result = adapter.observe_native_handoff_use(&assignment, &registered);
        assert_eq!(result.is_ok(), matches!(state, "pending" | "expired"));
        if let Ok(record) = result {
            assert!(record.used.is_none());
        }
    }
    let requests = Rc::new(RefCell::new(Vec::new()));
    let mut adapter = RemoteViewApplicationAdapter::new(
        "agent-browser".into(),
        Transport {
            replies: VecDeque::from([
                Ok(fixture["assignmentObservation"].clone()),
                Err(RemoteViewApplicationTransportError::OutcomeUnknown),
            ]),
            requests: requests.clone(),
        },
    )
    .unwrap();
    assert!(adapter
        .observe_native_handoff_use(&assignment, &registered)
        .is_err());
    assert_eq!(requests.borrow().len(), 2);
}

#[test]
fn native_use_replay_preserves_first_record_and_rejects_post_read_generation_change() {
    let fixture = fixture();
    let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let first = used(registration());
    let registered: RemoteViewNativeHandoffObservation =
        serde_json::from_value(first.clone()).unwrap();
    for case in ["exact", "rewritten", "lost", "stale", "unexpected-field"] {
        let mut record = first.clone();
        let mut after = fixture["assignmentObservation"].clone();
        let mut state = "used";
        match case {
            "rewritten" => record["used"]["recordedAtMs"] = json!(1001),
            "lost" => {
                record["used"] = Value::Null;
                state = "pending";
            }
            "stale" => after["target"]["viewingGeneration"] = json!(999),
            "unexpected-field" => record["successful"] = json!(true),
            _ => (),
        }
        let requests = Rc::new(RefCell::new(Vec::new()));
        let mut adapter = RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            Transport {
                replies: VecDeque::from([
                    Ok(fixture["assignmentObservation"].clone()),
                    Ok(json!({"schemaVersion":1,"state":state,"observation":record})),
                    Ok(after),
                ]),
                requests,
            },
        )
        .unwrap();
        let result = adapter.observe_native_handoff_use(&assignment, &registered);
        assert_eq!(result.is_ok(), case == "exact", "{case}");
        if let Ok(record) = result {
            assert_eq!(record, registered);
        }
    }
}

#[test]
fn native_registration_lost_reply_is_not_reposted_or_upgraded_to_use() {
    let fixture = fixture();
    let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let mut adapter = RemoteViewApplicationAdapter::new(
        "agent-browser".into(),
        Transport {
            replies: VecDeque::from([
                Ok(fixture["assignmentObservation"].clone()),
                Err(RemoteViewApplicationTransportError::OutcomeUnknown),
            ]),
            requests: requests.clone(),
        },
    )
    .unwrap();
    assert!(adapter
        .register_native_handoff_observation(
            &assignment,
            "synthetic-target".into(),
            "synthetic-handoff".into(),
            300_000,
        )
        .is_err());
    assert_eq!(requests.borrow().len(), 2);
}

#[test]
fn missing_native_context_refreshes_only_after_exact_assignment_readback() {
    use agent_browser_service_model::RemoteViewNativeHandoffUseState;
    let fixture = fixture();
    let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let registered = serde_json::from_value(registration()).unwrap();
    for stale in [false, true] {
        let mut after = fixture["assignmentObservation"].clone();
        if stale {
            after["target"]["viewingGeneration"] = json!(999);
        }
        let requests = Rc::new(RefCell::new(Vec::new()));
        let mut adapter = RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            Transport {
                replies: VecDeque::from([
                    Ok(fixture["assignmentObservation"].clone()),
                    Err(RemoteViewApplicationTransportError::NativeObservationMissing),
                    Ok(after),
                ]),
                requests: requests.clone(),
            },
        )
        .unwrap();
        let read = adapter.observe_native_handoff_use_state(&assignment, &registered);
        if stale {
            assert!(read.is_err());
        } else {
            let read = read.expect("explicitly missing context must allow fresh preparation");
            assert_eq!(read.state, RemoteViewNativeHandoffUseState::Expired);
            assert_eq!(read.observation, registered);
            assert!(read.observation.used.is_none());
        }
        assert_eq!(requests.borrow().len(), 3);
    }
}
