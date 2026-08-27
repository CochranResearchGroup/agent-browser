use super::*;
use crate::runtime_owner_transfer::{
    CleanupObligationState, ProfileOwner, ProfileOwnerState, RuntimeLaneLifecycleState,
    RuntimeLifecycleRecord, RuntimeOwnerRegistry,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

const P134_RED_FIXTURES: &str =
    include_str!("../../../../docs/dev/fixtures/profile-lifecycle/plan-0134-red-fixtures.v1.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureCorpus {
    schema_version: String,
    access_plan_cases: Vec<AccessPlanCase>,
    unscoped_existing_session: Value,
    crash_epoch: Value,
    public_lease_contract: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AccessPlanCase {
    fixture_id: String,
    consumer_shape: String,
    service_name: String,
    agent_name: String,
    task_name: String,
    target_service_id: String,
    profile_id: String,
    holder_session_id: String,
    holder_browser_id: String,
    holder_browser_profile_id: Option<String>,
    requester_principal_id: String,
    holder_principal_id: String,
    same_principal: bool,
    retained_owner_evidence: bool,
    expected_current_action: String,
    expected_current_reusable_browser_count: u64,
    required_future_action: String,
}

fn corpus() -> FixtureCorpus {
    serde_json::from_str(P134_RED_FIXTURES).expect("Plan 0134 fixtures must parse")
}

fn state_for_case(case: &AccessPlanCase) -> ServiceState {
    let profile_path = format!("/tmp/agent-browser-p134/{}", case.profile_id);
    let mut state = ServiceState {
        profiles: BTreeMap::from([(
            case.profile_id.clone(),
            BrowserProfile {
                id: case.profile_id.clone(),
                name: case.consumer_shape.clone(),
                user_data_dir: Some(profile_path.clone()),
                target_service_ids: vec![case.target_service_id.clone()],
                authenticated_service_ids: vec![case.target_service_id.clone()],
                shared_service_ids: vec![case.service_name.clone()],
                ..BrowserProfile::default()
            },
        )]),
        sessions: BTreeMap::from([(
            case.holder_session_id.clone(),
            BrowserSession {
                id: case.holder_session_id.clone(),
                service_name: Some(case.service_name.clone()),
                agent_name: Some(case.agent_name.clone()),
                profile_id: Some(case.profile_id.clone()),
                browser_ids: vec![case.holder_browser_id.clone()],
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        )]),
        ..ServiceState::default()
    };

    if let Some(browser_profile_id) = &case.holder_browser_profile_id {
        state.browsers.insert(
            case.holder_browser_id.clone(),
            BrowserProcess {
                id: case.holder_browser_id.clone(),
                profile_id: Some(browser_profile_id.clone()),
                health: BrowserHealth::Ready,
                active_session_ids: vec![case.holder_session_id.clone()],
                ..BrowserProcess::default()
            },
        );
    }

    if case.retained_owner_evidence {
        let profile_identity_digest = crate::runtime_profile::canonical_profile_identity_digest(
            std::path::Path::new(&profile_path),
        )
        .expect("fixture profile path must canonicalize");
        state.runtime_owner_registry = RuntimeOwnerRegistry {
            revision: 1,
            owners: BTreeMap::from([(
                profile_identity_digest.clone(),
                ProfileOwner {
                    owner_id: format!("owner:{}", case.holder_principal_id),
                    profile_identity_digest: profile_identity_digest.clone(),
                    state: ProfileOwnerState::Ready,
                    owner_generation: 7,
                    browser_id: case.holder_browser_id.clone(),
                    daemon_session_route: case.holder_session_id.clone(),
                    process_instance_digest: "synthetic-process-digest".to_string(),
                    browser_family: "chrome".to_string(),
                    cdp_endpoint_identity_digest: "synthetic-cdp-digest".to_string(),
                    target_set_digest: "synthetic-target-digest".to_string(),
                    pending_transfer: None,
                    last_transition: None,
                },
            )]),
            principal_bindings: BTreeMap::new(),
            lifecycle_records: BTreeMap::from([(
                case.holder_browser_id.clone(),
                RuntimeLifecycleRecord {
                    logical_browser_id: case.holder_browser_id.clone(),
                    profile_identity_digest,
                    owner_generation: 7,
                    lifecycle_state: RuntimeLaneLifecycleState::Ready,
                    cleanup_obligation_state: CleanupObligationState::Owned,
                    ..RuntimeLifecycleRecord::default()
                },
            )]),
        };
    }

    state
}

#[test]
fn p134_slice_a_public_access_plan_reproduces_principal_blind_self_waits() {
    let corpus = corpus();
    assert_eq!(
        corpus.schema_version,
        "agent-browser.plan-0134-red-fixtures.v1"
    );
    assert_eq!(corpus.access_plan_cases.len(), 5);

    for case in &corpus.access_plan_cases {
        let plan = service_access_plan_for_state(
            &state_for_case(case),
            ServiceAccessPlanRequest {
                service_name: Some(case.service_name.clone()),
                agent_name: Some(case.agent_name.clone()),
                task_name: Some(case.task_name.clone()),
                target_service_ids: vec![case.target_service_id.clone()],
                runtime_profile: Some(case.profile_id.clone()),
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"], case.expected_current_action,
            "fixture {} did not reproduce its frozen current action",
            case.fixture_id
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["compatibleLiveBrowserCount"],
            case.expected_current_reusable_browser_count,
            "fixture {} unexpectedly found a reusable browser",
            case.fixture_id
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["activeLeaseSessionIds"],
            serde_json::json!([case.holder_session_id]),
            "fixture {} lost the blocking holder",
            case.fixture_id
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["reusableBrowserId"],
            Value::Null,
            "fixture {} unexpectedly returned retained-browser routing",
            case.fixture_id
        );

        if case.same_principal {
            assert_eq!(case.requester_principal_id, case.holder_principal_id);
            assert_ne!(case.required_future_action, "wait_for_foreign_principal");
        } else {
            assert_ne!(case.requester_principal_id, case.holder_principal_id);
            assert_eq!(case.required_future_action, "wait_for_foreign_principal");
        }
    }
}

#[test]
fn p134_slice_a_public_access_plan_rejects_principal_authority_input() {
    let error = parse_service_access_plan_query(vec![(
        "principalId".to_string(),
        "principal:synthetic-owner".to_string(),
    )])
    .expect_err("principal authority is not part of the current public request");

    assert_eq!(
        error,
        "Unknown service access plan query parameter: principalId"
    );
}

#[test]
fn p134_slice_a_freezes_default_attribution_crash_and_lease_contract_gaps() {
    let corpus = corpus();
    let resolved = crate::runtime_profile::resolve_profile(None, None)
        .expect("unscoped profile resolution must remain reproducible");

    assert_eq!(resolved.runtime_profile.as_deref(), Some("default"));
    assert_eq!(
        corpus.unscoped_existing_session["currentResolvedRuntimeProfile"],
        "default"
    );
    assert_eq!(
        corpus.unscoped_existing_session["firstAttributionWrite"]["sourceAnchor"],
        "cli/src/native/cdp/chrome.rs write_runtime_state"
    );
    assert_eq!(
        corpus.crash_epoch["currentFailures"]
            .as_array()
            .map(Vec::len),
        Some(5)
    );
    assert_eq!(
        corpus.public_lease_contract["currentFirstClassOperations"],
        serde_json::json!([])
    );

    let serialized_state = serde_json::to_value(ServiceState::default())
        .expect("default Service State must serialize");
    assert!(serialized_state.get("schemaVersion").is_none());
    assert!(serialized_state.get("profileLeases").is_none());
    assert!(serialized_state.get("bootEpoch").is_none());
}
