use super::*;
use crate::runtime_owner_transfer::{
    CleanupObligationState, ProfileOwner, ProfileOwnerState, RuntimeLaneLifecycleState,
    RuntimeLifecycleRecord, RuntimeOwnerRegistry,
};
use serde::Deserialize;
use std::collections::BTreeMap;

const P137_TERMINAL_OWNER_FIXTURE: &str =
    include_str!("../../../../docs/dev/fixtures/profile-recovery/plan-0137-terminal-owner.v1.json");
const P137_BLOCKER_FIXTURES: &[&str] = &[
    include_str!("../../../../docs/dev/fixtures/profile-recovery/plan-0137-odollo-contractor-portal.v1.json"),
    include_str!("../../../../docs/dev/fixtures/profile-recovery/plan-0137-soylei-identity-inconsistent.v1.json"),
    include_str!("../../../../docs/dev/fixtures/profile-recovery/plan-0137-soylei-legacy-principal.v1.json"),
    include_str!("../../../../docs/dev/fixtures/profile-recovery/plan-0137-soylei-owner-binding-missing.v1.json"),
    include_str!("../../../../docs/dev/fixtures/profile-recovery/plan-0137-soylei-owner-generation-mismatch.v1.json"),
    include_str!("../../../../docs/dev/fixtures/profile-recovery/plan-0137-fictitious-browser-cdp.v1.json"),
    include_str!("../../../../docs/dev/fixtures/profile-recovery/plan-0137-fictitious-odollo-ups.v1.json"),
    include_str!("../../../../docs/dev/fixtures/profile-recovery/plan-0137-cdp-free-seeding-route.v1.json"),
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TerminalOwnerFixture {
    schema_version: String,
    fixture_id: String,
    consumer_shape: String,
    service_name: String,
    agent_name: String,
    task_name: String,
    target_service_id: String,
    profile_id: String,
    profile_path: String,
    principal_id: String,
    owner_id: String,
    owner_generation: u64,
    durable_browser_id: String,
    daemon_session_route: String,
    lifecycle_state: String,
    cleanup_obligation_state: String,
    terminal_evidence: Vec<String>,
    expected_dominant_blocker: String,
    expected_recovery_class: String,
}

fn fixture() -> TerminalOwnerFixture {
    serde_json::from_str(P137_TERMINAL_OWNER_FIXTURE)
        .expect("Plan 0137 terminal-owner fixture must parse")
}

fn state_for_fixture(fixture: &TerminalOwnerFixture) -> ServiceState {
    let profile_identity_digest = crate::runtime_profile::canonical_profile_identity_digest(
        std::path::Path::new(&fixture.profile_path),
    )
    .expect("fixture profile path must canonicalize");
    ServiceState {
        profiles: BTreeMap::from([(
            fixture.profile_id.clone(),
            BrowserProfile {
                id: fixture.profile_id.clone(),
                name: fixture.consumer_shape.clone(),
                user_data_dir: Some(fixture.profile_path.clone()),
                target_service_ids: vec![fixture.target_service_id.clone()],
                authenticated_service_ids: vec![fixture.target_service_id.clone()],
                shared_service_ids: vec![fixture.service_name.clone()],
                ..BrowserProfile::default()
            },
        )]),
        runtime_owner_registry: RuntimeOwnerRegistry {
            revision: 137,
            owners: BTreeMap::from([(
                profile_identity_digest.clone(),
                ProfileOwner {
                    owner_id: fixture.owner_id.clone(),
                    profile_identity_digest: profile_identity_digest.clone(),
                    state: ProfileOwnerState::Ready,
                    owner_generation: fixture.owner_generation,
                    browser_id: fixture.durable_browser_id.clone(),
                    daemon_session_route: fixture.daemon_session_route.clone(),
                    process_instance_digest: digest("terminal-process"),
                    browser_family: "chrome".to_string(),
                    cdp_endpoint_identity_digest: digest("terminal-cdp"),
                    target_set_digest: digest("terminal-targets"),
                    pending_transfer: None,
                    last_transition: None,
                },
            )]),
            principal_bindings: BTreeMap::new(),
            lifecycle_records: BTreeMap::from([(
                fixture.durable_browser_id.clone(),
                RuntimeLifecycleRecord {
                    logical_browser_id: fixture.durable_browser_id.clone(),
                    profile_identity_digest,
                    owner_generation: fixture.owner_generation,
                    lifecycle_state: RuntimeLaneLifecycleState::Terminal,
                    cleanup_obligation_state: CleanupObligationState::Satisfied,
                    terminal_evidence: fixture.terminal_evidence.clone(),
                    ..RuntimeLifecycleRecord::default()
                },
            )]),
        },
        ..ServiceState::default()
    }
}

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[test]
fn p137_blocker_fixtures_have_one_dominant_blocker_and_recovery_class() {
    let mut fixture_ids = std::collections::BTreeSet::new();
    let mut failures = std::collections::BTreeSet::new();
    for encoded in P137_BLOCKER_FIXTURES {
        let fixture: serde_json::Value = serde_json::from_str(encoded).unwrap();
        let fixture_id = fixture["fixtureId"].as_str().unwrap().to_string();
        let current_failure = fixture["currentFailure"].as_str().unwrap().to_string();
        let blocker = fixture["expectedDominantBlocker"]
            .as_str()
            .unwrap()
            .to_string();
        let recovery_class = fixture["expectedRecoveryClass"].as_str().unwrap();
        assert!(fixture_ids.insert(fixture_id));
        assert_eq!(current_failure, blocker);
        assert!(!recovery_class.is_empty());
        failures.insert(current_failure);
    }
    assert!(failures.contains("existing_session_profile_identity_inconsistent"));
    assert!(failures.contains("existing_session_profile_identity_unproven"));
    assert!(failures.contains("legacy_principal_unproven"));
    assert!(failures.contains("runtime_owner_principal_binding_missing"));
    assert!(failures.contains("owner_generation_or_binding_mismatch"));
    assert!(failures.contains("live_browser_missing_pid"));
    assert!(failures.contains("presentation_route_identity_unproven"));
}

#[test]
fn p137_generation_55_transferred_terminal_owner_plans_without_mutating_state() {
    let fixture = fixture();
    assert_eq!(
        fixture.schema_version,
        "agent-browser.plan-0137-terminal-owner-fixture.v1"
    );
    assert_eq!(
        fixture.fixture_id,
        "last30days-generation-55-transferred-terminal-owner"
    );
    assert_eq!(fixture.lifecycle_state, "terminal");
    assert_eq!(fixture.cleanup_obligation_state, "satisfied");
    assert_eq!(
        fixture.expected_dominant_blocker,
        "terminal_owner_cleanup_satisfied"
    );
    assert_eq!(fixture.expected_recovery_class, "supersede_terminal_owner");
    assert_ne!(
        fixture.durable_browser_id,
        format!("session:{}", fixture.daemon_session_route),
        "the regression fixture must preserve transferred route divergence"
    );

    let state = state_for_fixture(&fixture);
    let before = state.clone();
    let plan = service_access_plan_for_state(
        &state,
        ServiceAccessPlanRequest {
            service_name: Some(fixture.service_name.clone()),
            agent_name: Some(fixture.agent_name.clone()),
            task_name: Some(fixture.task_name.clone()),
            target_service_ids: vec![fixture.target_service_id.clone()],
            runtime_profile: Some(fixture.profile_id.clone()),
            ..ServiceAccessPlanRequest::default()
        },
    );

    assert_eq!(state, before, "access planning must remain zero-effect");
    assert_eq!(
        plan["decision"]["lifecycleReplacement"]["replacementEligible"],
        true
    );
    assert_eq!(
        plan["decision"]["lifecycleReplacement"]["reason"],
        "terminal_cleanup_satisfied"
    );
    assert_eq!(
        plan["decision"]["lifecycleReplacement"]["processAbsenceProven"],
        true
    );
    assert_eq!(
        plan["decision"]["lifecycleReplacement"]["replacementBrowserId"],
        fixture.durable_browser_id
    );
    assert_eq!(
        plan["decision"]["lifecycleReplacement"]["replacementSessionName"],
        fixture.daemon_session_route
    );
    let launch_session = plan["decision"]["serviceRequest"]["request"]["sessionName"]
        .as_str()
        .expect("terminal replacement must provide an executable launch session");
    assert_ne!(launch_session, fixture.daemon_session_route);
    assert!(launch_session.starts_with("terminal-profile-"));
    assert!(crate::validation::is_valid_session_name(launch_session));
    assert_eq!(fixture.principal_id, "principal:last30days");
}

#[test]
fn p137_terminal_replacement_requires_exact_process_exit_evidence() {
    let fixture = fixture();
    let mut state = state_for_fixture(&fixture);
    state
        .runtime_owner_registry
        .lifecycle_records
        .get_mut(&fixture.durable_browser_id)
        .unwrap()
        .terminal_evidence = vec!["profile_lock_released".to_string()];

    let plan = service_access_plan_for_state(
        &state,
        ServiceAccessPlanRequest {
            runtime_profile: Some(fixture.profile_id),
            target_service_ids: vec![fixture.target_service_id],
            ..ServiceAccessPlanRequest::default()
        },
    );

    assert_eq!(
        plan["decision"]["lifecycleReplacement"]["replacementEligible"],
        false
    );
    assert_eq!(
        plan["decision"]["lifecycleReplacement"]["processAbsenceProven"],
        false
    );
}

#[test]
fn p137_terminal_history_without_current_owner_cannot_emit_a_replacement_route() {
    let fixture = fixture();
    let mut state = state_for_fixture(&fixture);
    state.runtime_owner_registry.owners.clear();

    let plan = service_access_plan_for_state(
        &state,
        ServiceAccessPlanRequest {
            service_name: Some(fixture.service_name),
            agent_name: Some(fixture.agent_name),
            task_name: Some(fixture.task_name),
            runtime_profile: Some(fixture.profile_id),
            target_service_ids: vec![fixture.target_service_id],
            ..ServiceAccessPlanRequest::default()
        },
    );

    assert_eq!(
        plan["decision"]["lifecycleReplacement"]["replacementEligible"],
        true
    );
    assert_eq!(
        plan["decision"]["lifecycleReplacement"]["reason"],
        "no_lifecycle_owner"
    );
    assert!(plan["decision"]["lifecycleReplacement"]["ownerId"].is_null());
    assert!(plan["decision"]["lifecycleReplacement"]["replacementSessionName"].is_null());
    assert_eq!(plan["decision"]["serviceRequest"]["available"], true);
    assert!(plan["decision"]["serviceRequest"]["acquisitionBlocker"].is_null());
    assert!(plan["decision"]["serviceRequest"]["request"]["sessionName"].is_null());
}

#[test]
fn p137_terminal_replacement_accepts_reconciled_absent_process_and_stale_lock() {
    let fixture = fixture();
    let mut state = state_for_fixture(&fixture);
    state
        .runtime_owner_registry
        .lifecycle_records
        .get_mut(&fixture.durable_browser_id)
        .unwrap()
        .terminal_evidence = vec![
        "service_reconcile_process_group_absent:74831".to_string(),
        "service_reconcile_profile_lock_stale_pid_absent:74831".to_string(),
        "service_reconcile_browser_projection_absent".to_string(),
        "service_reconcile_transfer_authority_absent".to_string(),
    ];

    let plan = service_access_plan_for_state(
        &state,
        ServiceAccessPlanRequest {
            runtime_profile: Some(fixture.profile_id),
            target_service_ids: vec![fixture.target_service_id],
            ..ServiceAccessPlanRequest::default()
        },
    );

    assert_eq!(
        plan["decision"]["lifecycleReplacement"]["replacementEligible"],
        true
    );
    assert_eq!(
        plan["decision"]["lifecycleReplacement"]["processAbsenceProven"],
        true
    );
    assert_eq!(
        plan["decision"]["lifecycleReplacement"]["reason"],
        "terminal_cleanup_satisfied"
    );
}
