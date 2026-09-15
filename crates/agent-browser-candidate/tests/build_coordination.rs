use agent_browser_candidate::{
    coordinate_build, ArtifactClass, BuildDecision, BuildIdentity, BuildOperation,
    BuildOperationState, BuildProfileConfiguration, ExecutableInput, ExecutableInputClosure,
    ExecutableInputContext, InputCategory, SealedArtifact,
};
use std::collections::BTreeMap;

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn closure(source_digest: char) -> ExecutableInputClosure {
    ExecutableInputClosure::new(
        ExecutableInputContext {
            target: "x86_64-unknown-linux-gnu".to_string(),
            toolchain: "rustc 1.90.0".to_string(),
            cargo_profile: "release".to_string(),
            resolved_build_profile: BuildProfileConfiguration::production_release(),
            features: vec!["service".to_string()],
            reviewed_environment_inputs: BTreeMap::new(),
        },
        vec![ExecutableInput {
            path: "cli/src/main.rs".to_string(),
            sha256: digest(source_digest),
            category: InputCategory::RustSource,
        }],
    )
    .expect("input closure")
}

#[test]
fn equivalent_request_joins_the_active_build() {
    let identity = BuildIdentity::new(&closure('b'), ArtifactClass::ProductionShaped);
    let active = BuildOperation::new("build-1", identity.clone());

    let decision = coordinate_build(&identity, &[active], &[]).expect("build advice");
    assert_eq!(
        decision,
        BuildDecision::JoinActive {
            operation_id: "build-1".to_string(),
        }
    );
}

#[test]
fn distinct_inputs_receive_distinct_isolated_outputs() {
    let first = BuildIdentity::new(&closure('b'), ArtifactClass::ProductionShaped);
    let second = BuildIdentity::new(&closure('c'), ArtifactClass::ProductionShaped);

    let first_decision = coordinate_build(&first, &[], &[]).expect("first advice");
    let second_decision = coordinate_build(&second, &[], &[]).expect("second advice");
    let BuildDecision::StartIsolated {
        output_directory: first_output,
    } = first_decision
    else {
        panic!("first request did not start an isolated build")
    };
    let BuildDecision::StartIsolated {
        output_directory: second_output,
    } = second_decision
    else {
        panic!("second request did not start an isolated build")
    };

    assert_ne!(first.digest(), second.digest());
    assert_ne!(first_output, second_output);
    assert!(first_output.starts_with("candidate-builds/"));
}

#[test]
fn exact_sealed_artifact_is_reused_and_tampering_fails_closed() {
    let identity = BuildIdentity::new(&closure('b'), ArtifactClass::ProductionShaped);
    let sealed = SealedArtifact::new(
        "build-1",
        identity.clone(),
        digest('d'),
        digest('e'),
        digest('f'),
    )
    .expect("sealed artifact");

    let decision = coordinate_build(&identity, &[], std::slice::from_ref(&sealed))
        .expect("sealed reuse advice");
    assert_eq!(
        decision,
        BuildDecision::ReuseSealed {
            operation_id: "build-1".to_string(),
            binary_sha256: digest('d'),
        }
    );

    let mut tampered = sealed;
    tampered.binary_sha256 = digest('9');
    let error = coordinate_build(&identity, &[], &[tampered])
        .expect_err("tampered seal must not be reusable");
    assert_eq!(error.code(), "sealed_artifact_tampered");
}

#[test]
fn terminal_failed_build_is_not_joinable() {
    let identity = BuildIdentity::new(&closure('b'), ArtifactClass::FastIteration);
    let mut failed = BuildOperation::new("build-1", identity.clone());
    failed.state = BuildOperationState::Failed;

    assert!(matches!(
        coordinate_build(&identity, &[failed], &[]).expect("new build advice"),
        BuildDecision::StartIsolated { .. }
    ));
}
