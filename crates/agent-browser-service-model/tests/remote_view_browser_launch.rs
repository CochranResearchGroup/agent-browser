use agent_browser_service_model::*;
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};

type Record = Rc<RefCell<Option<BrowserLaunchCustodyRecord>>>;
type Events = Rc<RefCell<Vec<&'static str>>>;
struct Store {
    record: Record,
    events: Events,
    fail_observation: bool,
}
impl BrowserLaunchCustodyStore for Store {
    fn admit_launch_intent(
        &mut self,
        intent: &BrowserLaunchIntent,
        _: &BrowserSessionState,
    ) -> Result<LaunchCustodyAdmission, LaunchCustodyStoreError> {
        self.events.borrow_mut().push("claim");
        if let Some(record) = self.record.borrow().as_ref() {
            if record.intent != *intent {
                return Err(LaunchCustodyStoreError::Conflict);
            }
            return Ok(LaunchCustodyAdmission::Existing(record.clone()));
        }
        *self.record.borrow_mut() =
            Some(BrowserLaunchCustodyRecord::pending(intent.clone()).unwrap());
        Ok(LaunchCustodyAdmission::New)
    }
    fn observe_launch_intent(
        &mut self,
        _: &BrowserLaunchIntent,
        launch: &BrowserLaunch,
    ) -> Result<(), LaunchCustodyStoreError> {
        self.events.borrow_mut().push("record");
        if self.fail_observation {
            return Err(LaunchCustodyStoreError::Unavailable);
        }
        self.record
            .borrow_mut()
            .as_mut()
            .unwrap()
            .observe(launch)
            .unwrap();
        Ok(())
    }
    fn publish_launch_intent(
        &mut self,
        _: &BrowserLaunchIntent,
        _: &BrowserSessionState,
        _: &BrowserSessionState,
    ) -> Result<(), LaunchCustodyStoreError> {
        panic!("launch coordinator must not publish session state")
    }
}
struct Transport {
    record: Record,
    events: Events,
    steps: VecDeque<(Value, Value)>,
}
impl RemoteViewApplicationTransport for Transport {
    fn request(
        &mut self,
        request: &RemoteViewApplicationEnvelope,
    ) -> Result<Value, RemoteViewApplicationTransportError> {
        assert!(
            self.record.borrow().is_some(),
            "provider request preceded custody"
        );
        self.events.borrow_mut().push(match request.request {
            RemoteViewApplicationRequest::ObserveAssignment { .. } => "observe",
            RemoteViewApplicationRequest::LaunchEnvironment { .. } => "environment",
            _ => panic!("unexpected provider operation"),
        });
        let (expected, response) = self.steps.pop_front().expect("unexpected replay");
        assert_eq!(serde_json::to_value(request).unwrap(), expected);
        Ok(response)
    }
}
struct Process {
    events: Events,
    expected_environment: BTreeMap<String, String>,
    failure: Option<RemoteViewBrowserProcessError>,
    wrong_generation: bool,
}
impl RemoteViewBrowserProcessEffects for Process {
    fn launch(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        intent: &BrowserLaunchIntent,
        environment: RemoteViewPrivateLaunchEnvironment,
        observation: &RemoteViewAssignmentObservation,
    ) -> Result<BrowserLaunch, RemoteViewBrowserProcessError> {
        self.events.borrow_mut().push("launch");
        assert_eq!(profile.id, intent.profile_id);
        assert_eq!(
            environment
                .into_environment(observation, &intent.assignment)
                .unwrap(),
            self.expected_environment
        );
        if let Some(error) = self.failure {
            return Err(error);
        }
        Ok(BrowserLaunch {
            browser_id: "browser-a".into(),
            pid: 42,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: intent.assignment.desktop_id.clone(),
                generation: intent.assignment.generation + u64::from(self.wrong_generation),
                friendly_route_label: String::new(),
            }),
        })
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap()
}
fn inputs(f: &Value) -> (BrowserLaunchIntent, BrowserProfileCatalogEntry) {
    (
        BrowserLaunchIntent {
            intent_id: "33333333-3333-3333-3333-333333333333".into(),
            profile_id: "profile-a".into(),
            assignment: serde_json::from_value(f["assignment"].clone()).unwrap(),
        },
        BrowserProfileCatalogEntry {
            id: "profile-a".into(),
            name: "Synthetic".into(),
            user_data_dir: "/synthetic/profile-a".into(),
            kind: BrowserProfileKind::Named,
        },
    )
}
fn setup(
    f: &Value,
    stale: bool,
) -> (
    RemoteViewApplicationAdapter<Transport>,
    Store,
    Process,
    Events,
) {
    let record: Record = Rc::default();
    let events: Events = Rc::default();
    let mut last = f["assignmentObservation"].clone();
    if stale {
        last["target"]["viewingGeneration"] =
            Value::from(last["target"]["viewingGeneration"].as_u64().unwrap() + 1);
    }
    let steps = VecDeque::from([
        (f["requests"][2].clone(), f["assignmentObservation"].clone()),
        (f["requests"][3].clone(), f["launchEnvironment"].clone()),
        (f["requests"][2].clone(), f["assignmentObservation"].clone()),
        (f["requests"][2].clone(), last),
    ]);
    (
        RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            Transport {
                record: record.clone(),
                events: events.clone(),
                steps,
            },
        )
        .unwrap(),
        Store {
            record,
            events: events.clone(),
            fail_observation: false,
        },
        Process {
            events: events.clone(),
            expected_environment: serde_json::from_value(
                f["launchEnvironment"]["environment"].clone(),
            )
            .unwrap(),
            failure: None,
            wrong_generation: false,
        },
        events,
    )
}
#[test]
fn launch_claim_precedes_private_inputs_and_process_and_replay_never_launches_again() {
    let f = fixture();
    let (intent, profile) = inputs(&f);
    let (mut adapter, mut store, mut process, events) = setup(&f, false);
    launch_remote_view_browser(
        &mut adapter,
        &mut store,
        &mut process,
        &intent,
        &BrowserSessionState::default(),
        &profile,
    )
    .unwrap();
    assert_eq!(
        *events.borrow(),
        [
            "claim",
            "observe",
            "environment",
            "observe",
            "observe",
            "launch",
            "record"
        ]
    );
    assert_eq!(
        store.record.borrow().as_ref().unwrap().observed_pid,
        Some(42)
    );
    assert!(!store.record.borrow().as_ref().unwrap().published);
    assert_eq!(
        launch_remote_view_browser(
            &mut adapter,
            &mut store,
            &mut process,
            &intent,
            &BrowserSessionState::default(),
            &profile
        )
        .unwrap_err(),
        RemoteViewBrowserLaunchError::ReadbackRequired
    );
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|event| **event == "launch")
            .count(),
        1
    );
}
#[test]
fn launch_failure_generation_drift_and_failed_observation_retain_claim_without_retry() {
    let f = fixture();
    let (intent, profile) = inputs(&f);
    for mode in 0..4 {
        let (mut adapter, mut store, mut process, events) = setup(&f, mode == 0);
        if mode == 1 {
            process.failure = Some(RemoteViewBrowserProcessError::OutcomeUnknown);
        }
        if mode == 2 {
            process.wrong_generation = true;
        }
        if mode == 3 {
            store.fail_observation = true;
        }
        assert!(launch_remote_view_browser(
            &mut adapter,
            &mut store,
            &mut process,
            &intent,
            &BrowserSessionState::default(),
            &profile
        )
        .is_err());
        assert_eq!(store.record.borrow().as_ref().unwrap().observed_pid, None);
        let count = events
            .borrow()
            .iter()
            .filter(|event| **event == "launch")
            .count();
        assert_eq!(count, usize::from(mode != 0));
        assert_eq!(
            launch_remote_view_browser(
                &mut adapter,
                &mut store,
                &mut process,
                &intent,
                &BrowserSessionState::default(),
                &profile
            )
            .unwrap_err(),
            RemoteViewBrowserLaunchError::ReadbackRequired
        );
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == "launch")
                .count(),
            count
        );
    }
}
