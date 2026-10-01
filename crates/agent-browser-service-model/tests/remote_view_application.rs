use agent_browser_service_model::{
    RemoteViewApplicationCleanup, RemoteViewApplicationEnvelope,
    REMOTE_VIEW_APPLICATION_SOURCE_CHECKPOINT,
};
use serde_json::{json, Value};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap()
}

#[test]
fn replays_published_external_request_shapes_without_managed_lifecycle() {
    let fixture = fixture();
    assert_eq!(
        fixture["sourceCheckpoint"],
        REMOTE_VIEW_APPLICATION_SOURCE_CHECKPOINT
    );
    let requests = fixture["requests"].as_array().unwrap();
    assert_eq!(requests.len(), 12);
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
