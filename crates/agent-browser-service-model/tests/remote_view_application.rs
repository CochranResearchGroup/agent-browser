use agent_browser_service_model::{
    RemoteViewApplicationAdapter, RemoteViewApplicationAdapterError, RemoteViewApplicationCleanup,
    RemoteViewApplicationEnvelope, RemoteViewApplicationResponseError,
    RemoteViewApplicationTransport, RemoteViewApplicationTransportError,
    RemoteViewAssignmentObservation, RemoteViewAssignmentRecord,
    RemoteViewPrivateLaunchEnvironment, REMOTE_VIEW_APPLICATION_SOURCE_CHECKPOINT,
};
use serde_json::{json, Value};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

type TransportStep = (Value, Result<Value, RemoteViewApplicationTransportError>);
struct ScriptedTransport(Rc<RefCell<VecDeque<TransportStep>>>);

impl RemoteViewApplicationTransport for ScriptedTransport {
    fn request(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<Value, RemoteViewApplicationTransportError> {
        let (expected, response) = self
            .0
            .borrow_mut()
            .pop_front()
            .expect("unexpected additional request");
        assert_eq!(serde_json::to_value(envelope).unwrap(), expected);
        response
    }
}

#[test]
fn launch_transport_refreshes_exact_join_and_never_retries_unknown_outcomes() {
    let fixture = fixture();
    let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
    for changed_after in [false, true] {
        let mut after = fixture["assignmentObservation"].clone();
        if changed_after {
            after["target"]["viewingGeneration"] = json!(20);
        }
        let steps = Rc::new(RefCell::new(VecDeque::from([
            (
                fixture["requests"][2].clone(),
                Ok(fixture["assignmentObservation"].clone()),
            ),
            (
                fixture["requests"][3].clone(),
                Ok(fixture["launchEnvironment"].clone()),
            ),
            (fixture["requests"][2].clone(), Ok(after)),
        ])));
        let mut adapter = RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            ScriptedTransport(steps.clone()),
        )
        .unwrap();
        let result = adapter.launch_environment(&assignment);
        if changed_after {
            assert_eq!(
                result.unwrap_err(),
                RemoteViewApplicationAdapterError::Response(
                    RemoteViewApplicationResponseError::StaleTarget
                )
            );
        } else {
            assert!(result.is_ok());
        }
        assert!(steps.borrow().is_empty());
    }
    let steps = Rc::new(RefCell::new(VecDeque::from([(
        fixture["requests"][2].clone(),
        Err(RemoteViewApplicationTransportError::OutcomeUnknown),
    )])));
    let mut adapter =
        RemoteViewApplicationAdapter::new("agent-browser".into(), ScriptedTransport(steps.clone()))
            .unwrap();
    assert_eq!(
        adapter.launch_environment(&assignment).unwrap_err(),
        RemoteViewApplicationAdapterError::Transport(
            RemoteViewApplicationTransportError::OutcomeUnknown
        )
    );
    assert!(steps.borrow().is_empty());
}

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap()
}

#[test]
fn window_adapter_accepts_current_provider_dialog_metadata() {
    let fixture = fixture();
    let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let mut windows = fixture["windows"].clone();
    // Current Remote View desktop.windows includes these native observations.
    windows["windows"][0]["dialog"] = json!(false);
    windows["windows"][0]["modal"] = json!(false);
    windows["windows"][0]["transientFor"] = Value::Null;
    let steps = Rc::new(RefCell::new(VecDeque::from([
        (
            fixture["requests"][2].clone(),
            Ok(fixture["assignmentObservation"].clone()),
        ),
        (fixture["requests"][4].clone(), Ok(windows)),
        (
            fixture["requests"][2].clone(),
            Ok(fixture["assignmentObservation"].clone()),
        ),
    ])));
    let mut adapter =
        RemoteViewApplicationAdapter::new("agent-browser".into(), ScriptedTransport(steps.clone()))
            .unwrap();
    let observed = adapter
        .windows(&assignment)
        .expect("current provider windows must deserialize and pass both generation checks");
    assert_eq!(observed.windows[0].dialog, Some(false));
    assert_eq!(observed.windows[0].modal, Some(false));
    assert_eq!(observed.windows[0].transient_for, None);
    assert_eq!(
        observed.windows[0].id,
        fixture["windows"]["windows"][0]["id"].as_u64().unwrap() as u32
    );
    assert_eq!(
        observed.windows[0].pid,
        fixture["windows"]["windows"][0]["pid"]
            .as_u64()
            .map(|pid| pid as u32)
    );
    assert!(steps.borrow().is_empty());
}

#[test]
fn read_adapter_consumes_exact_inventory_windows_events_and_retained_views() {
    let fixture = fixture();
    let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let steps = Rc::new(RefCell::new(VecDeque::from([
        (
            fixture["requests"][0].clone(),
            Ok(fixture["inventory"].clone()),
        ),
        (
            fixture["requests"][2].clone(),
            Ok(fixture["assignmentObservation"].clone()),
        ),
        (
            fixture["requests"][4].clone(),
            Ok(fixture["windows"].clone()),
        ),
        (
            fixture["requests"][2].clone(),
            Ok(fixture["assignmentObservation"].clone()),
        ),
        (
            fixture["requests"][6].clone(),
            Ok(fixture["events"].clone()),
        ),
        (
            fixture["requests"][2].clone(),
            Ok(fixture["assignmentObservation"].clone()),
        ),
        (fixture["requests"][8].clone(), Ok(fixture["grant"].clone())),
    ])));
    let mut adapter =
        RemoteViewApplicationAdapter::new("agent-browser".into(), ScriptedTransport(steps.clone()))
            .unwrap();
    assert_eq!(
        serde_json::to_value(adapter.inventory().unwrap()).unwrap(),
        fixture["inventory"]
    );
    assert_eq!(
        serde_json::to_value(adapter.windows(&assignment).unwrap()).unwrap(),
        fixture["windows"]
    );
    assert_eq!(
        serde_json::to_value(adapter.events(&assignment, Some(0), 32).unwrap()).unwrap(),
        fixture["events"]
    );
    let grant = serde_json::from_value(fixture["grant"].clone()).unwrap();
    assert_eq!(
        serde_json::to_value(
            adapter
                .resolve_view(&assignment, &grant, || 2000, false)
                .unwrap()
        )
        .unwrap(),
        fixture["grant"]
    );
    assert!(steps.borrow().is_empty());
}

#[test]
fn resolution_rechecks_expiry_after_transport_using_current_clock() {
    let fixture = fixture();
    let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let grant = serde_json::from_value(fixture["grant"].clone()).unwrap();
    let steps = Rc::new(RefCell::new(VecDeque::from([
        (
            fixture["requests"][2].clone(),
            Ok(fixture["assignmentObservation"].clone()),
        ),
        (fixture["requests"][8].clone(), Ok(fixture["grant"].clone())),
    ])));
    let mut adapter =
        RemoteViewApplicationAdapter::new("agent-browser".into(), ScriptedTransport(steps.clone()))
            .unwrap();
    let times = RefCell::new(VecDeque::from([2000, 301000]));
    let clock = || times.borrow_mut().pop_front().unwrap();
    assert_eq!(
        adapter
            .resolve_view(&assignment, &grant, clock, false)
            .unwrap_err(),
        RemoteViewApplicationAdapterError::Response(
            RemoteViewApplicationResponseError::InvalidTarget
        )
    );
    assert!(steps.borrow().is_empty());
}

#[test]
fn joins_independent_generations_and_preserves_private_full_environment() {
    let fixture = fixture();
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let observed: RemoteViewAssignmentObservation =
        serde_json::from_value(fixture["assignmentObservation"].clone()).unwrap();
    observed.validate_live_assignment(&assignment).unwrap();
    assert_ne!(
        observed.target.lifecycle_generation,
        observed.target.viewing_generation
    );
    assert_ne!(
        observed.target.desktop_id,
        observed.target.viewing_desktop_id
    );
    let environment = RemoteViewPrivateLaunchEnvironment::from_response(
        fixture["launchEnvironment"].clone(),
        &observed,
        &assignment,
    )
    .unwrap();
    let debug = format!("{environment:?}");
    for field in [
        "DISPLAY",
        "XAUTHORITY",
        "DBUS_SESSION_BUS_ADDRESS",
        "PULSE_SERVER",
    ] {
        assert!(!debug.contains(
            fixture["launchEnvironment"]["environment"][field]
                .as_str()
                .unwrap()
        ));
    }
    assert!(debug.contains("environment: \"redacted\""));
    assert_eq!(
        serde_json::to_value(
            environment
                .into_environment(&observed, &assignment)
                .unwrap()
        )
        .unwrap(),
        fixture["launchEnvironment"]["environment"]
    );
}

#[test]
fn rejects_stale_target_and_record_only_launch_at_each_boundary() {
    let fixture = fixture();
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let observed: RemoteViewAssignmentObservation =
        serde_json::from_value(fixture["assignmentObservation"].clone()).unwrap();
    for (field, replacement) in [
        ("assignmentId", json!("assignment-other")),
        ("registrationId", json!("registration-other")),
        ("desktopId", json!("44444444-4444-4444-8444-444444444444")),
        ("lifecycleGeneration", json!(8)),
        (
            "viewingDesktopId",
            json!("55555555-5555-4555-8555-555555555555"),
        ),
        ("viewingGeneration", json!(20)),
    ] {
        let mut stale = fixture["launchEnvironment"].clone();
        stale["target"][field] = replacement.clone();
        assert_eq!(
            RemoteViewPrivateLaunchEnvironment::from_response(stale, &observed, &assignment)
                .unwrap_err(),
            RemoteViewApplicationResponseError::StaleTarget,
            "{field}"
        );

        let environment = RemoteViewPrivateLaunchEnvironment::from_response(
            fixture["launchEnvironment"].clone(),
            &observed,
            &assignment,
        )
        .unwrap();
        let mut current = fixture["assignmentObservation"].clone();
        current["target"][field] = replacement;
        let current: RemoteViewAssignmentObservation = serde_json::from_value(current).unwrap();
        assert_eq!(
            environment
                .into_environment(&current, &assignment)
                .unwrap_err(),
            RemoteViewApplicationResponseError::StaleTarget,
            "{field}"
        );
    }
    for location in ["assignmentObservation", "launchEnvironment"] {
        let mut changed = fixture.clone();
        changed[location]["readinessScope"] = json!("provider_record");
        let observed = serde_json::from_value(changed["assignmentObservation"].clone()).unwrap();
        assert_eq!(
            RemoteViewPrivateLaunchEnvironment::from_response(
                changed["launchEnvironment"].clone(),
                &observed,
                &assignment
            )
            .unwrap_err(),
            RemoteViewApplicationResponseError::LiveResourceRequired
        );
        changed[location]
            .as_object_mut()
            .unwrap()
            .remove("readinessScope");
        if location == "assignmentObservation" {
            assert!(serde_json::from_value::<RemoteViewAssignmentObservation>(
                changed[location].clone()
            )
            .is_err());
        } else {
            assert_eq!(
                RemoteViewPrivateLaunchEnvironment::from_response(
                    changed[location].clone(),
                    &observed,
                    &assignment
                )
                .unwrap_err(),
                RemoteViewApplicationResponseError::InvalidShape
            );
        }
    }
}

#[test]
fn rejects_incomplete_environment_without_exposing_private_values_in_errors() {
    let fixture = fixture();
    let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let observed = serde_json::from_value(fixture["assignmentObservation"].clone()).unwrap();
    for field in ["DISPLAY", "XAUTHORITY", "REMOTE_VIEW_SLOT_GENERATION"] {
        let mut response = fixture["launchEnvironment"].clone();
        response["environment"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert_eq!(
            RemoteViewPrivateLaunchEnvironment::from_response(response, &observed, &assignment)
                .unwrap_err(),
            RemoteViewApplicationResponseError::InvalidEnvironment
        );
    }
    let mut response = fixture["launchEnvironment"].clone();
    response["environment"]["REMOTE_VIEW_SLOT_GENERATION"] = json!("7");
    assert_eq!(
        RemoteViewPrivateLaunchEnvironment::from_response(response, &observed, &assignment)
            .unwrap_err(),
        RemoteViewApplicationResponseError::InvalidEnvironment
    );
    let mut response = fixture["launchEnvironment"].clone();
    response["privateCredential"] = json!("synthetic-do-not-echo");
    let error = RemoteViewPrivateLaunchEnvironment::from_response(response, &observed, &assignment)
        .unwrap_err();
    assert_eq!(error, RemoteViewApplicationResponseError::InvalidShape);
    assert!(!format!("{error:?}").contains("synthetic-do-not-echo"));
}

#[test]
fn replays_published_external_request_shapes_without_managed_lifecycle() {
    let fixture = fixture();
    assert_eq!(
        fixture["sourceCheckpoint"],
        REMOTE_VIEW_APPLICATION_SOURCE_CHECKPOINT
    );
    let requests = fixture["requests"].as_array().unwrap();
    assert_eq!(requests.len(), 13);
    for request in requests {
        let envelope: RemoteViewApplicationEnvelope =
            serde_json::from_value(request.clone()).unwrap();
        assert_eq!(serde_json::to_value(envelope).unwrap(), *request);
    }
    for operation in ["place", "stop"] {
        assert!(
            serde_json::from_value::<RemoteViewApplicationEnvelope>(json!({
                "application": "agent-browser", "request": {"operation": operation}
            }))
            .is_err()
        );
    }
    let mut request = requests[3].clone();
    request["request"]["expectedGeneration"] = json!(7);
    assert!(serde_json::from_value::<RemoteViewApplicationEnvelope>(request).is_err());
    let mut request = requests[0].clone();
    request["principal"] = json!("obsolete-role");
    assert!(serde_json::from_value::<RemoteViewApplicationEnvelope>(request).is_err());
}

#[test]
fn release_requires_every_obligation_and_exact_assignment_generation() {
    let acknowledgement = fixture()["requests"][11]["request"]["cleanup"].clone();
    let cleanup: RemoteViewApplicationCleanup =
        serde_json::from_value(acknowledgement.clone()).unwrap();
    assert!(cleanup.validates_target("assignment-external-1", 7));
    assert!(!cleanup.validates_target("assignment-external-2", 7));
    assert!(!cleanup.validates_target("assignment-external-1", 19));
    for field in [
        "applicationReferencesClear",
        "pendingRecoveryClear",
        "foregroundLeasesClear",
        "cleanupTasksClear",
    ] {
        let mut invalid = acknowledgement.clone();
        invalid[field] = json!(false);
        let invalid: RemoteViewApplicationCleanup = serde_json::from_value(invalid).unwrap();
        assert!(
            !invalid.validates_target("assignment-external-1", 7),
            "{field}"
        );
        let mut missing = acknowledgement.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<RemoteViewApplicationCleanup>(missing).is_err());
    }
    let mut invalid = cleanup;
    invalid.schema_version = 2;
    assert!(!invalid.validates_target("assignment-external-1", 7));
}

#[test]
fn transport_observation_fences_retained_grant_generation_and_post_read_expiry() {
    let fixture = fixture();
    let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let grant = serde_json::from_value(fixture["grant"].clone()).unwrap();
    for case in [
        "ready",
        "pending",
        "different_grant",
        "missing_origin",
        "generation_drift",
        "expired_after_read",
    ] {
        let mut observation = json!({"schemaVersion":1,"state":"ready","grant":fixture["grant"],"publicOrigin":"https://remote-view.example"});
        let mut after = fixture["assignmentObservation"].clone();
        match case {
            "pending" => {
                observation["state"] = json!("pending");
                observation["publicOrigin"] = Value::Null;
            }
            "different_grant" => {
                observation["grant"]["routeId"] = json!("22222222222222222222222222222222")
            }
            "missing_origin" => observation["publicOrigin"] = Value::Null,
            "generation_drift" => after["target"]["viewingGeneration"] = json!(20),
            _ => {}
        }
        let mut script = VecDeque::from([
            (
                fixture["requests"][2].clone(),
                Ok(fixture["assignmentObservation"].clone()),
            ),
            (
                json!({"application":"agent-browser","request":{"operation":"observe_view","route_id":fixture["grant"]["routeId"],"audience":"remote_view"}}),
                Ok(observation),
            ),
        ]);
        if !matches!(case, "different_grant" | "missing_origin") {
            script.push_back((fixture["requests"][2].clone(), Ok(after)));
        }
        let steps = Rc::new(RefCell::new(script));
        let mut adapter = RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            ScriptedTransport(steps.clone()),
        )
        .unwrap();
        let times = RefCell::new(VecDeque::from([
            2000,
            if case == "expired_after_read" {
                301000
            } else {
                2000
            },
        ]));
        let result = adapter.observe_view(&assignment, &grant, || {
            times.borrow_mut().pop_front().unwrap()
        });
        assert_eq!(
            result.is_ok(),
            matches!(case, "ready" | "pending"),
            "{case}"
        );
        if let Ok(observation) = result {
            assert_eq!(
                observation.state
                    == agent_browser_service_model::RemoteViewApplicationViewReadiness::Ready,
                case == "ready"
            );
        }
        assert!(steps.borrow().is_empty(), "{case}");
    }
}
