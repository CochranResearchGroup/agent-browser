use agent_browser_candidate::{
    evaluate_promotion, ArtifactClass, BuildProfileConfiguration, CandidateManifest,
    ExecutableInput, ExecutableInputClosure, ExecutableInputContext, InputCategory,
    PromotionEvidence, PromotionOutcome, PromotionReason, SourceProvenance, SourceTreeState,
    ValidationReceipt,
};
use std::collections::BTreeMap;

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn commit(byte: char) -> String {
    std::iter::repeat_n(byte, 40).collect()
}

fn closure(source_digest: char, profile: BuildProfileConfiguration) -> ExecutableInputClosure {
    ExecutableInputClosure::new(
        ExecutableInputContext {
            target: "x86_64-unknown-linux-gnu".to_string(),
            toolchain: "rustc 1.90.0".to_string(),
            cargo_profile: "release".to_string(),
            resolved_build_profile: profile,
            features: vec!["service".to_string()],
            reviewed_environment_inputs: BTreeMap::new(),
        },
        vec![
            ExecutableInput {
                path: "cli/src/main.rs".to_string(),
                sha256: digest(source_digest),
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
    .expect("input closure")
}

fn manifest(class: ArtifactClass) -> CandidateManifest {
    let closure = closure('a', BuildProfileConfiguration::production_release());
    let mut manifest = CandidateManifest::new(
        SourceProvenance {
            commit: commit('b'),
            tree: digest('c'),
            state: SourceTreeState::Clean,
        },
        &closure,
        class,
        digest('f'),
        digest('1'),
        "2026-09-15T12:00:00Z".to_string(),
    )
    .expect("manifest");
    manifest.validation_receipts = vec!["receipt://provider-free".to_string()];
    manifest
}

fn evidence() -> PromotionEvidence {
    PromotionEvidence {
        source_commit_is_ancestor: true,
        observed_binary_sha256: digest('f'),
        observed_support_manifest_sha256: digest('1'),
        required_receipt_locators: vec!["receipt://provider-free".to_string()],
        receipts: vec![ValidationReceipt {
            locator: "receipt://provider-free".to_string(),
            binary_sha256: digest('f'),
            terminal_success: true,
            scope_current: true,
            residue_clear: true,
        }],
        development_doctor_ready: true,
        task_residue_clear: true,
    }
}

#[test]
fn exact_production_artifact_is_promotable_without_rebuilding() {
    let manifest = manifest(ArtifactClass::ProductionShaped);
    let current = closure('a', BuildProfileConfiguration::production_release());
    let decision = evaluate_promotion(&manifest, &current, &evidence());

    assert_eq!(decision.outcome, PromotionOutcome::Eligible);
    assert_eq!(decision.reusable_binary_sha256, Some(digest('f')));
    assert!(decision.reasons.is_empty());
}

#[test]
fn fast_iteration_and_changed_inputs_require_a_rebuild() {
    let current = closure('a', BuildProfileConfiguration::production_release());
    let fast = evaluate_promotion(
        &manifest(ArtifactClass::FastIteration),
        &current,
        &evidence(),
    );
    assert_eq!(fast.outcome, PromotionOutcome::RebuildRequired);
    assert!(fast
        .reasons
        .contains(&PromotionReason::ArtifactClassNotPromotable));

    let changed = closure('9', BuildProfileConfiguration::production_release());
    let changed_decision = evaluate_promotion(
        &manifest(ArtifactClass::ProductionShaped),
        &changed,
        &evidence(),
    );
    assert_eq!(changed_decision.outcome, PromotionOutcome::RebuildRequired);
    assert!(changed_decision
        .reasons
        .contains(&PromotionReason::ExecutableInputChanged));
}

#[test]
fn wrong_release_profile_is_not_promotable() {
    let thin_profile = BuildProfileConfiguration {
        opt_level: "3".to_string(),
        lto: "thin".to_string(),
        codegen_units: 16,
        strip: true,
    };
    let thin_closure = closure('a', thin_profile);
    let mut thin_manifest = manifest(ArtifactClass::ProductionShaped);
    thin_manifest.executable_input_sha256 = thin_closure.digest();
    thin_manifest.resolved_build_profile = thin_closure.context.resolved_build_profile.clone();
    thin_manifest.resolved_build_profile_sha256 =
        thin_closure.context.resolved_build_profile.digest();

    let decision = evaluate_promotion(&thin_manifest, &thin_closure, &evidence());
    assert_eq!(decision.outcome, PromotionOutcome::RebuildRequired);
    assert!(decision
        .reasons
        .contains(&PromotionReason::ProductionProfileMismatch));
}

#[test]
fn tampered_artifact_or_missing_evidence_fails_integrity() {
    let manifest = manifest(ArtifactClass::ProductionShaped);
    let current = closure('a', BuildProfileConfiguration::production_release());
    let mut bad_evidence = evidence();
    bad_evidence.observed_binary_sha256 = digest('9');
    bad_evidence.receipts.clear();

    let decision = evaluate_promotion(&manifest, &current, &bad_evidence);
    assert_eq!(
        decision.outcome,
        PromotionOutcome::IntegrityPreconditionFailed
    );
    assert!(decision
        .reasons
        .contains(&PromotionReason::BinaryDigestMismatch));
    assert!(decision
        .reasons
        .contains(&PromotionReason::RequiredReceiptMissing));
    assert_eq!(decision.reusable_binary_sha256, None);
}
