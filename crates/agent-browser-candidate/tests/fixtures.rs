use agent_browser_candidate::{
    advise, ActiveOperation, AdvisoryInput, AdvisoryOutcome, AdvisoryResult,
    BuildProfileConfiguration, CandidateManifest, CandidateRelation, ExecutableInput,
    ExecutableInputClosure, ExecutableInputContext, InputCategory, IntegrityPreconditions,
    ObservedState, OperationState, OperatorAction,
};
use std::collections::BTreeMap;

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn closure() -> ExecutableInputClosure {
    ExecutableInputClosure::new(
        ExecutableInputContext {
            target: "x86_64-unknown-linux-gnu".to_string(),
            toolchain: "rustc 1.90.0".to_string(),
            cargo_profile: "release".to_string(),
            resolved_build_profile: BuildProfileConfiguration::production_release(),
            features: vec!["service".to_string()],
            reviewed_environment_inputs: BTreeMap::new(),
        },
        vec![
            ExecutableInput {
                path: "cli/src/main.rs".to_string(),
                sha256: digest('a'),
                category: InputCategory::RustSource,
            },
            ExecutableInput {
                path: "packages/dashboard/out/index.html".to_string(),
                sha256: digest('d'),
                category: InputCategory::EmbeddedDashboard,
            },
            ExecutableInput {
                path: "scripts/install-agent-browser-privileges.sh".to_string(),
                sha256: digest('e'),
                category: InputCategory::EmbeddedAsset,
            },
        ],
    )
    .expect("fixture closure")
}

#[test]
fn candidate_manifest_fixture_is_canonical_and_validated() {
    let manifest: CandidateManifest = serde_json::from_str(include_str!(
        "../../../docs/dev/fixtures/candidate-orchestration/candidate-manifest.v1.json"
    ))
    .expect("candidate manifest fixture");
    manifest
        .validate_against_closure(&closure())
        .expect("canonical manifest");

    let mut tampered = manifest;
    tampered.candidate_id = "candidate-tampered".to_string();
    assert_eq!(
        tampered
            .validate_against_closure(&closure())
            .expect_err("tampered manifest")
            .code(),
        "candidate_id_mismatch"
    );
}

#[test]
fn advisory_result_fixture_matches_the_frozen_competing_candidate_contract() {
    let fixture: AdvisoryResult = serde_json::from_str(include_str!(
        "../../../docs/dev/fixtures/candidate-orchestration/advisory-result.v1.json"
    ))
    .expect("advisory result fixture");
    let generated = advise(AdvisoryInput {
        observed_state: ObservedState::InstallActive,
        requested_candidate_id: "candidate-b".to_string(),
        candidate_relation: CandidateRelation::CompetingCandidate,
        active_operation: Some(ActiveOperation {
            operation_id: "operation-1".to_string(),
            candidate_id: "candidate-a".to_string(),
            state: OperationState::Installing,
            revision: 3,
            fencing_generation: 7,
        }),
        integrity: IntegrityPreconditions::satisfied(),
    });

    assert_eq!(fixture, generated);
    assert_eq!(fixture.outcome, AdvisoryOutcome::Queued);
    assert_eq!(fixture.recommendation, OperatorAction::Wait);
    assert_eq!(
        fixture.alternatives.last(),
        Some(&OperatorAction::Supersede)
    );
}
