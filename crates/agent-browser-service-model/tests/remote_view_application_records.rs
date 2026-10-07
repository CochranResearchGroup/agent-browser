use agent_browser_service_model::{
    RemoteViewApplicationActivation, RemoteViewApplicationEvents, RemoteViewApplicationGrant,
    RemoteViewApplicationInventory, RemoteViewApplicationResponseError,
    RemoteViewApplicationTarget, RemoteViewApplicationViewCapability,
    RemoteViewApplicationViewIssuance, RemoteViewApplicationViewRevocation,
    RemoteViewApplicationWindows, RemoteViewAssignmentRecord, RemoteViewJoinedReleaseOutcome,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap()
}

fn round_trip<T: DeserializeOwned + Serialize>(value: &Value) {
    let typed: T = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), *value);
}

#[test]
fn replays_public_inventory_window_event_view_and_release_response_shapes() {
    let fixture = fixture();
    round_trip::<RemoteViewApplicationInventory>(&fixture["inventory"]);
    round_trip::<RemoteViewApplicationWindows>(&fixture["windows"]);
    round_trip::<RemoteViewApplicationActivation>(&fixture["activation"]);
    round_trip::<RemoteViewApplicationEvents>(&fixture["events"]);
    round_trip::<RemoteViewApplicationGrant>(&fixture["grant"]);
    round_trip::<agent_browser_service_model::RemoteViewApplicationViewObservation>(
        &fixture["viewObservation"],
    );
    round_trip::<RemoteViewApplicationViewIssuance>(&fixture["viewIssuance"]);
    round_trip::<RemoteViewApplicationViewRevocation>(&fixture["viewRevocation"]);
    round_trip::<RemoteViewJoinedReleaseOutcome>(&fixture["joinedRelease"]);
}

#[test]
fn inventory_rejects_wrong_application_membership_and_managed_lifecycle() {
    let fixture = fixture();
    let inventory: RemoteViewApplicationInventory =
        serde_json::from_value(fixture["inventory"].clone()).unwrap();
    inventory.validate_external("agent-browser").unwrap();
    let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
    inventory.validate_acquisition("main", &assignment).unwrap();
    assert!(inventory.validate_external("other-app").is_err());
    // Provider inventory retains terminal assignments even after membership or
    // configured pools change. History must not become a current occupancy lock.
    let mut historical = fixture["inventory"].clone();
    historical["assignments"][0]["state"] = json!("released");
    historical["assignments"][0]["poolId"] = json!("retired-pool");
    let historical: RemoteViewApplicationInventory = serde_json::from_value(historical).unwrap();
    historical.validate_external("agent-browser").unwrap();
    for (path, value) in [
        ("/policy/application_mode", json!("managed")),
        ("/pools/0/registrationId", json!("wrong-registration")),
        ("/assignments/0/poolId", json!("wrong-pool")),
        ("/assignments/0/desktopId", json!("wrong-desktop")),
    ] {
        let mut changed = fixture["inventory"].clone();
        *changed.pointer_mut(path).unwrap() = value;
        let changed: RemoteViewApplicationInventory = serde_json::from_value(changed).unwrap();
        assert!(
            changed.validate_external("agent-browser").is_err(),
            "{path}"
        );
    }
}

#[test]
fn window_and_activation_results_fence_both_generations_and_exact_window() {
    let fixture = fixture();
    let target: RemoteViewApplicationTarget =
        serde_json::from_value(fixture["assignmentObservation"]["target"].clone()).unwrap();
    let windows: RemoteViewApplicationWindows =
        serde_json::from_value(fixture["windows"].clone()).unwrap();
    windows.validate_target(&target).unwrap();
    let activation: RemoteViewApplicationActivation =
        serde_json::from_value(fixture["activation"].clone()).unwrap();
    activation.validate_target(&target, 42).unwrap();
    assert!(activation.validate_target(&target, 43).is_err());
    for (field, value) in [
        ("generation", json!(8)),
        ("viewingGeneration", json!(20)),
        ("desktopId", json!("other-desktop")),
    ] {
        let mut changed = fixture["windows"].clone();
        changed[field] = value;
        let changed: RemoteViewApplicationWindows = serde_json::from_value(changed).unwrap();
        assert_eq!(
            changed.validate_target(&target),
            Err(RemoteViewApplicationResponseError::StaleTarget)
        );
    }
    let mut duplicate = windows;
    duplicate.windows.push(duplicate.windows[0].clone());
    assert!(duplicate.validate_target(&target).is_err());
}

#[test]
fn current_window_metadata_preserves_strict_shape_and_target_fences() {
    let fixture = fixture();
    let target: RemoteViewApplicationTarget =
        serde_json::from_value(fixture["assignmentObservation"]["target"].clone()).unwrap();
    let mut value = fixture["windows"].clone();
    value["windows"][0]["dialog"] = json!(true);
    value["windows"][0]["modal"] = json!(true);
    value["windows"][0]["transientFor"] = json!(41);
    let parsed: RemoteViewApplicationWindows = serde_json::from_value(value.clone()).unwrap();
    parsed.validate_target(&target).unwrap();
    assert_eq!(parsed.windows[0].transient_for, Some(41));
    assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    for field in ["dialog", "modal", "transientFor"] {
        let mut malformed = value.clone();
        malformed["windows"][0][field] = json!("invalid");
        assert!(serde_json::from_value::<RemoteViewApplicationWindows>(malformed).is_err());
    }
    let mut unknown = value.clone();
    unknown["windows"][0]["unrecognized"] = json!(true);
    assert!(serde_json::from_value::<RemoteViewApplicationWindows>(unknown).is_err());
    let mut unknown = value.clone();
    unknown["unrecognized"] = json!(true);
    assert!(serde_json::from_value::<RemoteViewApplicationWindows>(unknown).is_err());
    for (field, changed) in [
        ("desktopId", json!("wrong-desktop")),
        ("generation", json!(8)),
        ("viewingGeneration", json!(20)),
    ] {
        let mut stale = value.clone();
        stale[field] = changed;
        let stale: RemoteViewApplicationWindows = serde_json::from_value(stale).unwrap();
        assert_eq!(
            stale.validate_target(&target),
            Err(RemoteViewApplicationResponseError::StaleTarget)
        );
    }
}

#[test]
fn event_pages_reject_wrong_binding_cursor_and_private_or_wrong_target_payloads() {
    let fixture = fixture();
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(fixture["assignment"].clone()).unwrap();
    let events: RemoteViewApplicationEvents =
        serde_json::from_value(fixture["events"].clone()).unwrap();
    events
        .validate_assignment(&assignment, Some(0), 32)
        .unwrap();
    assert!(events
        .validate_assignment(&assignment, Some(1), 32)
        .is_err());
    assert!(events.validate_assignment(&assignment, None, 1).is_err());
    for (path, value) in [
        ("/binding/assignmentId", json!("other-assignment")),
        ("/events/1/cursor", json!(1)),
        ("/events/1/state", json!("running")),
        ("/events/0/observation/target/lifecycleGeneration", json!(8)),
    ] {
        let mut changed = fixture["events"].clone();
        *changed.pointer_mut(path).unwrap() = value;
        let changed: RemoteViewApplicationEvents = serde_json::from_value(changed).unwrap();
        assert!(
            changed.validate_assignment(&assignment, None, 32).is_err(),
            "{path}"
        );
    }
    let mut private = fixture["events"].clone();
    private["events"][0]["observation"]["environment"] = json!({"XAUTHORITY":"synthetic-private"});
    assert!(serde_json::from_value::<RemoteViewApplicationEvents>(private).is_err());
}

#[test]
fn view_grants_reject_expiry_revocation_target_drift_and_raw_route_urls() {
    let fixture = fixture();
    let target =
        serde_json::from_value(fixture["assignmentObservation"]["target"].clone()).unwrap();
    let issued: RemoteViewApplicationViewIssuance =
        serde_json::from_value(fixture["viewIssuance"].clone()).unwrap();
    let validate = |issued: &RemoteViewApplicationViewIssuance| {
        issued.validate_target(
            &target,
            "agent-browser",
            "remote_view",
            RemoteViewApplicationViewCapability::Control,
            300,
            2000,
        )
    };
    validate(&issued).unwrap();
    let mut opaque = issued.clone();
    opaque.grant.route_id = "65ce32d6bbea4-c9e6243d89630364f8601d5694ef93e2".into();
    opaque.path = format!("/view/{}", opaque.grant.route_id);
    validate(&opaque).unwrap();
    for invalid in [
        "short",
        "../route",
        "https://provider.invalid/route",
        "gggggggggggggggggggggggggggggggg",
        &"a".repeat(129),
    ] {
        let mut invalid_grant = opaque.clone();
        invalid_grant.grant.route_id = invalid.into();
        invalid_grant.path = format!("/view/{invalid}");
        assert!(validate(&invalid_grant).is_err());
    }
    assert!(issued
        .grant
        .validate_target(&target, "agent-browser", "remote_view", 301000)
        .is_err());
    for (path, value) in [
        ("/grant/revoked", json!(true)),
        ("/grant/request/target/viewingGeneration", json!(20)),
        ("/grant/request/application", json!("other-app")),
        ("/grant/request/capability", json!("observe")),
        ("/path", json!("https://provider.invalid/raw")),
        ("/readinessScope", json!("provider_record")),
        ("/presentationState", json!("ready")),
    ] {
        let mut changed = fixture["viewIssuance"].clone();
        *changed.pointer_mut(path).unwrap() = value;
        let changed: RemoteViewApplicationViewIssuance = serde_json::from_value(changed).unwrap();
        assert!(validate(&changed).is_err(), "{path}");
    }
}
