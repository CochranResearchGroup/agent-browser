use agent_browser_candidate::{
    ArtifactClass, BuildProfileConfiguration, CandidateManifest, ExecutableInput,
    ExecutableInputClosure, ExecutableInputContext, InputCategory, SourceProvenance,
    SourceTreeState,
};
use std::collections::BTreeMap;

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn commit(byte: char) -> String {
    std::iter::repeat_n(byte, 40).collect()
}

fn context() -> ExecutableInputContext {
    ExecutableInputContext {
        target: "x86_64-unknown-linux-gnu".to_string(),
        toolchain: "rustc 1.90.0".to_string(),
        cargo_profile: "release".to_string(),
        resolved_build_profile: BuildProfileConfiguration::production_release(),
        features: vec!["desktop".to_string(), "service".to_string()],
        reviewed_environment_inputs: BTreeMap::from([
            ("CARGO_PROFILE_RELEASE_LTO".to_string(), digest('b')),
            ("RUSTFLAGS".to_string(), digest('c')),
        ]),
    }
}

fn inputs() -> Vec<ExecutableInput> {
    vec![
        ExecutableInput {
            path: "cli/src/main.rs".to_string(),
            sha256: digest('d'),
            category: InputCategory::RustSource,
        },
        ExecutableInput {
            path: "packages/dashboard/out/index.html".to_string(),
            sha256: digest('e'),
            category: InputCategory::EmbeddedDashboard,
        },
        ExecutableInput {
            path: "Cargo.lock".to_string(),
            sha256: digest('f'),
            category: InputCategory::CargoLock,
        },
        ExecutableInput {
            path: "scripts/install-agent-browser-privileges.sh".to_string(),
            sha256: digest('8'),
            category: InputCategory::EmbeddedAsset,
        },
    ]
}

fn manifest(source_commit: &str, closure: &ExecutableInputClosure) -> CandidateManifest {
    CandidateManifest::new(
        SourceProvenance {
            commit: source_commit.to_string(),
            tree: digest('1'),
            state: SourceTreeState::Clean,
        },
        closure,
        ArtifactClass::ProductionShaped,
        digest('2'),
        digest('3'),
        "2026-09-15T12:00:00Z".to_string(),
    )
    .expect("candidate manifest")
}

#[test]
fn canonical_input_identity_ignores_order_but_detects_build_changes() {
    let first = ExecutableInputClosure::new(context(), inputs()).expect("first closure");
    let mut reordered = inputs();
    reordered.reverse();
    let second = ExecutableInputClosure::new(context(), reordered).expect("second closure");

    assert_eq!(first.digest(), second.digest());
    assert!(first.is_equivalent_to(&second));

    let mut changed_inputs = inputs();
    changed_inputs[0].sha256 = digest('9');
    let changed = ExecutableInputClosure::new(context(), changed_inputs).expect("changed closure");
    assert_ne!(first.digest(), changed.digest());
    assert!(!first.is_equivalent_to(&changed));
}

#[test]
fn schema_v1_reads_legacy_source_control_metadata_without_new_emission() {
    let current = ExecutableInputClosure::new(context(), inputs()).expect("current closure");
    let current_json = serde_json::to_string(&current).expect("serialize current closure");
    assert!(!current_json.contains("source_control_metadata"));

    let new_legacy = ExecutableInputClosure::new(
        context(),
        vec![ExecutableInput {
            path: ".git/HEAD".to_string(),
            sha256: digest('9'),
            category: InputCategory::SourceControlMetadata,
        }],
    )
    .expect_err("new closures must not emit legacy source-control metadata");
    assert_eq!(
        new_legacy.code(),
        "legacy_source_control_metadata_not_emittable"
    );

    let mut legacy_json = serde_json::to_value(&current).expect("serialize legacy fixture");
    let legacy_inputs = legacy_json["inputs"].as_array_mut().expect("input array");
    legacy_inputs.push(serde_json::json!({
            "path": ".git/HEAD",
            "sha256": digest('9'),
            "category": "source_control_metadata"
    }));
    legacy_inputs.sort_by(|left, right| {
        left["path"]
            .as_str()
            .expect("left input path")
            .cmp(right["path"].as_str().expect("right input path"))
    });
    let legacy: ExecutableInputClosure =
        serde_json::from_value(legacy_json).expect("legacy v1 closure must deserialize");
    assert!(legacy
        .inputs
        .iter()
        .any(|input| input.category == InputCategory::SourceControlMetadata));
    legacy.validate().expect("legacy v1 closure remains valid");
}

#[test]
fn merge_provenance_can_change_without_forcing_an_equivalent_rebuild() {
    let closure = ExecutableInputClosure::new(context(), inputs()).expect("closure");
    let before_merge = manifest(&commit('4'), &closure);
    let after_merge = manifest(&commit('5'), &closure);

    assert_ne!(before_merge.source.commit, after_merge.source.commit);
    assert_eq!(
        before_merge.executable_input_sha256,
        after_merge.executable_input_sha256
    );
    assert!(before_merge.can_reuse_artifact_for(&after_merge));
}

#[test]
fn malformed_or_ambiguous_inputs_fail_closed() {
    let mut duplicate_inputs = inputs();
    duplicate_inputs.push(duplicate_inputs[0].clone());
    let duplicate = ExecutableInputClosure::new(context(), duplicate_inputs)
        .expect_err("duplicate path must fail");
    assert_eq!(duplicate.code(), "duplicate_input_path");

    let mut malformed_context = context();
    malformed_context.reviewed_environment_inputs =
        BTreeMap::from([("RUSTFLAGS".to_string(), "secret value".to_string())]);
    let malformed = ExecutableInputClosure::new(malformed_context, inputs())
        .expect_err("raw environment values must fail");
    assert_eq!(malformed.code(), "invalid_sha256");
}

#[test]
fn production_shaped_manifest_requires_dashboard_and_support_assets() {
    let source = SourceProvenance {
        commit: commit('6'),
        tree: digest('7'),
        state: SourceTreeState::Clean,
    };
    let rust_only = ExecutableInputClosure::new(
        context(),
        vec![ExecutableInput {
            path: "cli/src/main.rs".to_string(),
            sha256: digest('d'),
            category: InputCategory::RustSource,
        }],
    )
    .expect("rust-only closure");

    let error = CandidateManifest::new(
        source,
        &rust_only,
        ArtifactClass::ProductionShaped,
        digest('2'),
        digest('3'),
        "2026-09-15T12:00:00Z".to_string(),
    )
    .expect_err("production candidate must bind embedded output");
    assert_eq!(error.code(), "production_embedded_output_incomplete");
}
