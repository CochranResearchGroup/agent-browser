//! CLI adapters for candidate orchestration status, manifest inspection,
//! explicit workstation installation, and exact transaction recovery.
//!
//! Status, inspect, and install dry-run are read-only. Install apply delegates
//! exact sealed bytes to the existing workstation transaction and candidate
//! coordination fence; this module does not implement another installer.
//! Coordinate mutates only the compare-and-swap candidate ledger.

use agent_browser_candidate::{
    CandidateManifest, CoordinationAction, CoordinationLedger, CoordinationRequest,
    ExecutableInputClosure,
};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

const ADVISORY_SCHEMA_VERSION: &str = "agent-browser.candidate-advisory.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
struct InspectArguments {
    manifest: String,
    input_closure: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstallArguments {
    binary: String,
    manifest: String,
    input_closure: String,
    sealed_artifact: String,
    mode: CandidateInstallMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateInstallMode {
    DryRun,
    Apply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateCoordinationAction {
    Queue,
    CancelActive,
    DiscardQueued,
    Supersede,
    ActivateQueued,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CoordinationArguments {
    action: CandidateCoordinationAction,
    request_id: String,
    candidate_id: String,
    artifact_id: String,
    operation_id: Option<String>,
    expected_revision: u64,
    expected_fencing_generation: u64,
}

/// Run a candidate command without starting or connecting to a browser daemon.
pub(crate) fn run_candidate_command(args: &[String], json_output: bool) {
    let operation = args.get(1).map(String::as_str).unwrap_or("status");
    let result = match operation {
        "status" if args.len() <= 2 => (|| {
            let root = crate::workstation_install::workstation_root()?;
            let coordination =
                crate::candidate_coordination::CandidateCoordinationStore::production(&root)
                    .read()?;
            candidate_status_from_sources(coordination)
        })(),
        "status" => Err(format!(
            "Unknown candidate status argument: {}",
            args.get(2).map(String::as_str).unwrap_or("unknown")
        )),
        "inspect" => parse_inspect_arguments(args).and_then(|parsed| {
            let manifest = fs::read(&parsed.manifest)
                .map_err(|error| format!("failed to read candidate manifest: {error}"))?;
            let closure = fs::read(&parsed.input_closure)
                .map_err(|error| format!("failed to read executable-input closure: {error}"))?;
            inspect_candidate_documents(&manifest, &closure)
        }),
        "build" => crate::candidate_build::run_candidate_build(args, json_output),
        "test" => crate::candidate_test::run_candidate_test(args, json_output),
        "install" => parse_install_arguments(args).and_then(|parsed| {
            let manifest = fs::read(&parsed.manifest)
                .map_err(|error| format!("failed to read candidate manifest: {error}"))?;
            let closure = fs::read(&parsed.input_closure)
                .map_err(|error| format!("failed to read executable-input closure: {error}"))?;
            let sealed_artifact = fs::read(&parsed.sealed_artifact)
                .map_err(|error| format!("failed to read sealed artifact: {error}"))?;
            match parsed.mode {
                CandidateInstallMode::DryRun => install_candidate_documents(
                    Path::new(&parsed.binary),
                    &manifest,
                    &closure,
                    &sealed_artifact,
                ),
                CandidateInstallMode::Apply => {
                    crate::workstation_install::run_reviewed_candidate_install(
                        Path::new(&parsed.binary),
                        &manifest,
                        &closure,
                        &sealed_artifact,
                        json_output,
                    )
                }
            }
        }),
        "coordinate" => parse_coordination_arguments(args).and_then(|parsed| {
            let root = crate::workstation_install::workstation_root()?;
            apply_coordination_transition(&root, parsed)
        }),
        unknown => Err(format!("Unknown candidate operation: {unknown}")),
    };

    match result {
        Ok(report) if json_output => println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        ),
        Ok(report) => print_human_report(&report),
        Err(error) if json_output => {
            println!(
                "{}",
                serde_json::to_string(&json!({"success": false, "error": error})).unwrap_or_else(
                    |_| r#"{"success":false,"error":"serialization failed"}"#.into()
                )
            );
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("Candidate inspection failed: {error}");
            std::process::exit(1);
        }
    }
}

fn parse_coordination_arguments(args: &[String]) -> Result<CoordinationArguments, String> {
    let action = match args.get(2).map(String::as_str) {
        Some("queue") => CandidateCoordinationAction::Queue,
        Some("cancel-active") => CandidateCoordinationAction::CancelActive,
        Some("discard-queued") => CandidateCoordinationAction::DiscardQueued,
        Some("supersede") => CandidateCoordinationAction::Supersede,
        Some("activate-queued") => CandidateCoordinationAction::ActivateQueued,
        Some(action) => return Err(format!("Unknown candidate coordination action: {action}")),
        None => return Err("candidate coordinate requires an action".to_string()),
    };
    let mut request_id = None;
    let mut candidate_id = None;
    let mut artifact_id = None;
    let mut operation_id = None;
    let mut expected_revision = None;
    let mut expected_fencing_generation = None;
    let mut index = 3;
    while index < args.len() {
        let (slot, name) = match args[index].as_str() {
            "--request-id" => (&mut request_id, "--request-id"),
            "--candidate-id" => (&mut candidate_id, "--candidate-id"),
            "--artifact-id" => (&mut artifact_id, "--artifact-id"),
            "--operation-id" => (&mut operation_id, "--operation-id"),
            "--expected-revision" => (&mut expected_revision, "--expected-revision"),
            "--expected-fencing-generation" => (
                &mut expected_fencing_generation,
                "--expected-fencing-generation",
            ),
            unknown => return Err(format!("Unknown candidate coordinate argument: {unknown}")),
        };
        if slot.is_some() {
            return Err(format!("{name} may be specified only once"));
        }
        index += 1;
        *slot = Some(
            args.get(index)
                .filter(|value| !value.trim().is_empty())
                .cloned()
                .ok_or_else(|| format!("candidate coordinate requires {name} <value>"))?,
        );
        index += 1;
    }
    let parse_counter = |value: Option<String>, name: &str| -> Result<u64, String> {
        value
            .ok_or_else(|| format!("candidate coordinate requires {name} <integer>"))?
            .parse::<u64>()
            .map_err(|_| format!("{name} must be a nonnegative integer"))
    };
    let operation_required = action != CandidateCoordinationAction::Queue;
    if operation_required && operation_id.is_none() {
        return Err("candidate coordinate action requires --operation-id <id>".to_string());
    }
    if !operation_required && operation_id.is_some() {
        return Err("candidate coordinate queue does not accept --operation-id".to_string());
    }
    Ok(CoordinationArguments {
        action,
        request_id: request_id
            .ok_or_else(|| "candidate coordinate requires --request-id <id>".to_string())?,
        candidate_id: candidate_id
            .ok_or_else(|| "candidate coordinate requires --candidate-id <id>".to_string())?,
        artifact_id: artifact_id
            .ok_or_else(|| "candidate coordinate requires --artifact-id <id>".to_string())?,
        operation_id,
        expected_revision: parse_counter(expected_revision, "--expected-revision")?,
        expected_fencing_generation: parse_counter(
            expected_fencing_generation,
            "--expected-fencing-generation",
        )?,
    })
}

fn apply_coordination_transition(
    root: &Path,
    parsed: CoordinationArguments,
) -> Result<Value, String> {
    let action = match parsed.action {
        CandidateCoordinationAction::Queue => CoordinationAction::Queue,
        CandidateCoordinationAction::CancelActive => CoordinationAction::CancelActive {
            operation_id: parsed.operation_id.clone().expect("validated operation ID"),
        },
        CandidateCoordinationAction::DiscardQueued => CoordinationAction::DiscardQueued {
            operation_id: parsed.operation_id.clone().expect("validated operation ID"),
        },
        CandidateCoordinationAction::Supersede => CoordinationAction::Supersede {
            operation_id: parsed.operation_id.clone().expect("validated operation ID"),
        },
        CandidateCoordinationAction::ActivateQueued => CoordinationAction::ActivateQueued {
            operation_id: parsed.operation_id.clone().expect("validated operation ID"),
        },
    };
    let store = crate::candidate_coordination::CandidateCoordinationStore::production(root);
    let receipt = store.apply(CoordinationRequest {
        request_id: parsed.request_id,
        candidate_id: parsed.candidate_id,
        artifact_id: parsed.artifact_id,
        expected_revision: parsed.expected_revision,
        expected_fencing_generation: parsed.expected_fencing_generation,
        action,
    })?;
    let ledger = store.read()?;
    Ok(json!({
        "schemaVersion": ADVISORY_SCHEMA_VERSION,
        "success": true,
        "observedState": "coordination_updated",
        "recommendation": "inspect",
        "alternatives": ["status"],
        "consequences": ["coordination_state_mutated", "no_runtime_effect_performed"],
        "integrityPreconditions": ["exact_revision", "exact_fencing_generation"],
        "reasonCodes": ["candidate_coordination_transition_committed"],
        "activeOperation": ledger.active(),
        "coordinationReceipt": receipt,
        "coordinationLedger": ledger,
        "buildProvenance": build_provenance(),
    }))
}

fn print_human_report(report: &Value) {
    println!(
        "Candidate state: {}",
        report
            .get("observedState")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    );
    println!(
        "Recommendation: {}",
        report
            .get("recommendation")
            .and_then(Value::as_str)
            .unwrap_or("inspect")
    );
    if let Some(candidate_id) = report
        .get("candidate")
        .and_then(|candidate| candidate.get("candidateId"))
        .and_then(Value::as_str)
    {
        println!("Candidate: {candidate_id}");
    }
    println!("No runtime effect was performed.");
}

fn parse_inspect_arguments(args: &[String]) -> Result<InspectArguments, String> {
    let mut manifest = None;
    let mut input_closure = None;
    let mut index = 0;
    while index < args.len() {
        let argument = args[index].as_str();
        let slot = match argument {
            "candidate" | "inspect" => None,
            "--manifest" => Some((&mut manifest, "--manifest")),
            "--input-closure" => Some((&mut input_closure, "--input-closure")),
            unknown => return Err(format!("Unknown candidate inspect argument: {unknown}")),
        };
        if let Some((slot, name)) = slot {
            if slot.is_some() {
                return Err(format!("{name} may be specified only once"));
            }
            index += 1;
            *slot = Some(
                args.get(index)
                    .filter(|value| !value.trim().is_empty())
                    .cloned()
                    .ok_or_else(|| format!("candidate inspect requires {name} <path>"))?,
            );
        }
        index += 1;
    }
    Ok(InspectArguments {
        manifest: manifest
            .ok_or_else(|| "candidate inspect requires --manifest <path>".to_string())?,
        input_closure: input_closure
            .ok_or_else(|| "candidate inspect requires --input-closure <path>".to_string())?,
    })
}

fn parse_install_arguments(args: &[String]) -> Result<InstallArguments, String> {
    let mut binary = None;
    let mut manifest = None;
    let mut input_closure = None;
    let mut sealed_artifact = None;
    let mut mode = None;
    let mut index = 0;
    while index < args.len() {
        let argument = args[index].as_str();
        let slot = match argument {
            "candidate" | "install" => None,
            "--binary" => Some((&mut binary, "--binary")),
            "--manifest" => Some((&mut manifest, "--manifest")),
            "--input-closure" => Some((&mut input_closure, "--input-closure")),
            "--sealed-artifact" => Some((&mut sealed_artifact, "--sealed-artifact")),
            "--dry-run" => {
                set_install_mode(&mut mode, CandidateInstallMode::DryRun)?;
                None
            }
            "--apply" => {
                set_install_mode(&mut mode, CandidateInstallMode::Apply)?;
                None
            }
            unknown => return Err(format!("Unknown candidate install argument: {unknown}")),
        };
        if let Some((slot, name)) = slot {
            if slot.is_some() {
                return Err(format!("{name} may be specified only once"));
            }
            index += 1;
            *slot = Some(
                args.get(index)
                    .filter(|value| !value.trim().is_empty())
                    .cloned()
                    .ok_or_else(|| format!("candidate install requires {name} <path>"))?,
            );
        }
        index += 1;
    }
    let mode = mode.ok_or_else(|| {
        "candidate install requires exactly one of --dry-run or --apply".to_string()
    })?;
    Ok(InstallArguments {
        binary: binary.ok_or_else(|| "candidate install requires --binary <path>".to_string())?,
        manifest: manifest
            .ok_or_else(|| "candidate install requires --manifest <path>".to_string())?,
        input_closure: input_closure
            .ok_or_else(|| "candidate install requires --input-closure <path>".to_string())?,
        sealed_artifact: sealed_artifact
            .ok_or_else(|| "candidate install requires --sealed-artifact <path>".to_string())?,
        mode,
    })
}

fn set_install_mode(
    mode: &mut Option<CandidateInstallMode>,
    requested: CandidateInstallMode,
) -> Result<(), String> {
    if let Some(current) = mode {
        if *current != requested {
            return Err(
                "candidate install --dry-run and --apply are mutually exclusive".to_string(),
            );
        }
        return Err("candidate install mode may be specified only once".to_string());
    }
    *mode = Some(requested);
    Ok(())
}

fn candidate_status_from_sources(coordination: CoordinationLedger) -> Result<Value, String> {
    Ok(json!({
        "schemaVersion": ADVISORY_SCHEMA_VERSION,
        "success": true,
        "observedState": "idle",
        "recommendation": "build",
        "alternatives": ["inspect"],
        "consequences": ["no_effect_performed"],
        "integrityPreconditions": [],
        "reasonCodes": ["no_active_candidate"],
        "activeOperation": coordination.active(),
        "artifactReuseEligibility": {
            "eligible": false,
            "reason": "no_candidate_manifest_supplied"
        },
        "rebuildReasons": [],
        "receiptLocators": [],
        "buildProvenance": build_provenance(),
        "coordinationLedger": coordination,
    }))
}

fn inspect_candidate_documents(manifest: &[u8], closure: &[u8]) -> Result<Value, String> {
    let manifest: CandidateManifest = serde_json::from_slice(manifest)
        .map_err(|error| format!("invalid candidate manifest JSON: {error}"))?;
    let closure: ExecutableInputClosure = serde_json::from_slice(closure)
        .map_err(|error| format!("invalid executable-input closure JSON: {error}"))?;
    manifest
        .validate_against_closure(&closure)
        .map_err(|error| error.to_string())?;

    Ok(json!({
        "schemaVersion": ADVISORY_SCHEMA_VERSION,
        "success": true,
        "observedState": "artifact_sealed",
        "recommendation": "install",
        "alternatives": ["inspect", "discard"],
        "consequences": ["install_requires_explicit_apply", "no_effect_performed"],
        "integrityPreconditions": [],
        "reasonCodes": ["candidate_manifest_valid"],
        "activeOperation": Value::Null,
        "artifactReuseEligibility": {
            "eligible": true,
            "executableInputSha256": manifest.executable_input_sha256,
            "artifactClass": manifest.artifact_class,
        },
        "rebuildReasons": [],
        "receiptLocators": manifest.validation_receipts,
        "buildProvenance": build_provenance(),
        "candidate": manifest,
    }))
}

fn install_candidate_documents(
    binary_path: &std::path::Path,
    manifest: &[u8],
    closure: &[u8],
    sealed_artifact: &[u8],
) -> Result<Value, String> {
    let candidate = crate::workstation_install::review_candidate_payload_documents(
        binary_path,
        manifest,
        closure,
        sealed_artifact,
    )?;
    let receipt_locators = candidate
        .get("validationReceipts")
        .cloned()
        .unwrap_or_else(|| json!([]));

    Ok(json!({
        "schemaVersion": ADVISORY_SCHEMA_VERSION,
        "success": true,
        "observedState": "artifact_sealed",
        "recommendation": "install",
        "alternatives": ["inspect", "discard"],
        "consequences": [
            "candidate_install_requires_explicit_apply",
            "no_effect_performed"
        ],
        "integrityPreconditions": [],
        "reasonCodes": ["sealed_candidate_payload_valid"],
        "activeOperation": Value::Null,
        "artifactReuseEligibility": {
            "eligible": true,
            "executableInputSha256": candidate.get("executableInputSha256").cloned().unwrap_or(Value::Null),
            "artifactClass": candidate.get("artifactClass").cloned().unwrap_or(Value::Null),
        },
        "rebuildReasons": [],
        "receiptLocators": receipt_locators,
        "buildProvenance": build_provenance(),
        "candidate": candidate,
    }))
}

fn build_provenance() -> Value {
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "sourceRevision": option_env!("AGENT_BROWSER_BUILD_SOURCE_REVISION").unwrap_or("unknown"),
        "sourceTreeState": option_env!("AGENT_BROWSER_BUILD_SOURCE_TREE_STATE").unwrap_or("unknown"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_candidate::{
        ArtifactClass, BuildIdentity, BuildProfileConfiguration, CandidateManifest,
        ExecutableInput, ExecutableInputClosure, ExecutableInputContext, InputCategory,
        SealedArtifact, SourceProvenance, SourceTreeState,
    };
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::path::Path;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn fixture_closure() -> Vec<u8> {
        serde_json::to_vec(
            &ExecutableInputClosure::new(
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
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn parses_exact_inspect_inputs_in_either_order() {
        let parsed = parse_inspect_arguments(&args(&[
            "candidate",
            "inspect",
            "--input-closure",
            "/tmp/closure.json",
            "--manifest",
            "/tmp/manifest.json",
        ]))
        .unwrap();

        assert_eq!(parsed.manifest, "/tmp/manifest.json");
        assert_eq!(parsed.input_closure, "/tmp/closure.json");
    }

    #[test]
    fn rejects_unknown_or_missing_inspect_inputs() {
        assert!(parse_inspect_arguments(&args(&[
            "candidate",
            "inspect",
            "--manifest",
            "/tmp/manifest.json",
            "--surprise",
        ]))
        .unwrap_err()
        .contains("Unknown"));
        assert!(parse_inspect_arguments(&args(&[
            "candidate",
            "inspect",
            "--manifest",
            "/tmp/manifest.json",
        ]))
        .unwrap_err()
        .contains("--input-closure"));
    }

    #[test]
    fn maps_active_workstation_transaction_to_observe_advice() {
        let report = candidate_status_from_workstation(json!({
            "schemaVersion": "agent-browser.workstation-upgrade-status.v1",
            "success": true,
            "latestTransaction": {
                "transactionId": "upgrade-1",
                "candidateGenerationId": "generation-1",
                "revision": 4,
                "classification": "active_convergence",
                "safeActions": ["inspect", "resume"]
            }
        }))
        .unwrap();

        assert_eq!(report["observedState"], "install_active");
        assert_eq!(report["recommendation"], "observe");
        assert_eq!(report["activeOperation"]["operationId"], "upgrade-1");
        assert_eq!(report["activeOperation"]["revision"], 4);
        assert_eq!(report["consequences"][0], "no_effect_performed");
        assert_eq!(report["coordinationLedger"]["environmentId"], "production");
    }

    #[test]
    fn validates_checked_in_manifest_against_its_input_closure() {
        let manifest = include_bytes!(
            "../../docs/dev/fixtures/candidate-orchestration/candidate-manifest.v1.json"
        );
        let closure = fixture_closure();

        let report = inspect_candidate_documents(manifest, &closure).unwrap();

        assert_eq!(report["observedState"], "artifact_sealed");
        assert_eq!(report["recommendation"], "install");
        assert_eq!(report["artifactReuseEligibility"]["eligible"], true);
        assert_eq!(report["rebuildReasons"], json!([]));
    }

    #[test]
    fn rejects_a_manifest_that_does_not_match_the_closure() {
        let mut manifest: Value = serde_json::from_slice(include_bytes!(
            "../../docs/dev/fixtures/candidate-orchestration/candidate-manifest.v1.json"
        ))
        .unwrap();
        manifest["binarySha256"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let manifest = serde_json::to_vec(&manifest).unwrap();
        let closure = fixture_closure();

        let error = inspect_candidate_documents(&manifest, &closure).unwrap_err();

        assert!(error.contains("candidate_id_mismatch"));
    }

    #[test]
    fn install_dry_run_validates_exact_sealed_binary_without_effects() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-candidate-install-dry-run-{}",
            uuid::Uuid::new_v4()
        ));
        let binary_path = root.join("agent-browser");
        fs::create_dir_all(&root).unwrap();
        fs::write(&binary_path, b"production candidate bytes").unwrap();
        let binary_sha256 = sha256_file(&binary_path);
        let closure: ExecutableInputClosure = serde_json::from_slice(&fixture_closure()).unwrap();
        let build_support_manifest_sha256 = digest('f');
        let mut manifest = CandidateManifest::new(
            SourceProvenance {
                commit: "1".repeat(40),
                tree: digest('2'),
                state: SourceTreeState::Clean,
            },
            &closure,
            ArtifactClass::ProductionShaped,
            binary_sha256.clone(),
            build_support_manifest_sha256.clone(),
            "2026-09-16T02:00:00Z".to_string(),
        )
        .unwrap();
        manifest.validation_receipts = vec!["receipt://qualification/provider-free".to_string()];
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let sealed = SealedArtifact::new(
            "build-operation-install-1",
            BuildIdentity::new(&closure, ArtifactClass::ProductionShaped),
            binary_sha256.clone(),
            build_support_manifest_sha256,
            sha256_bytes(&manifest_bytes),
        )
        .unwrap();

        let report = install_candidate_documents(
            &binary_path,
            &manifest_bytes,
            &serde_json::to_vec(&closure).unwrap(),
            &serde_json::to_vec(&sealed).unwrap(),
        )
        .unwrap();

        assert_eq!(report["observedState"], "artifact_sealed");
        assert_eq!(report["recommendation"], "install");
        assert_eq!(report["candidate"]["candidateId"], manifest.candidate_id);
        assert_eq!(report["candidate"]["binarySha256"], binary_sha256);
        assert_eq!(
            report["consequences"],
            json!([
                "candidate_install_requires_explicit_apply",
                "no_effect_performed"
            ])
        );
        assert!(!root.join(".agent-browser").exists());

        fs::write(&binary_path, b"changed bytes").unwrap();
        let error = install_candidate_documents(
            &binary_path,
            &manifest_bytes,
            &serde_json::to_vec(&closure).unwrap(),
            &serde_json::to_vec(&sealed).unwrap(),
        )
        .unwrap_err();
        assert!(error.contains("workstation_payload_source_digest_mismatch"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn install_command_requires_one_explicit_mode() {
        let parsed = parse_install_arguments(&args(&[
            "candidate",
            "install",
            "--sealed-artifact",
            "/tmp/sealed.json",
            "--binary",
            "/tmp/agent-browser",
            "--input-closure",
            "/tmp/closure.json",
            "--manifest",
            "/tmp/manifest.json",
            "--dry-run",
        ]))
        .unwrap();

        assert_eq!(parsed.binary, "/tmp/agent-browser");
        assert_eq!(parsed.manifest, "/tmp/manifest.json");
        assert_eq!(parsed.input_closure, "/tmp/closure.json");
        assert_eq!(parsed.sealed_artifact, "/tmp/sealed.json");
        assert_eq!(parsed.mode, CandidateInstallMode::DryRun);

        let apply = parse_install_arguments(&args(&[
            "candidate",
            "install",
            "--binary",
            "/tmp/agent-browser",
            "--manifest",
            "/tmp/manifest.json",
            "--input-closure",
            "/tmp/closure.json",
            "--sealed-artifact",
            "/tmp/sealed.json",
            "--apply",
        ]))
        .unwrap();
        assert_eq!(apply.mode, CandidateInstallMode::Apply);
        assert!(parse_install_arguments(&args(&[
            "candidate",
            "install",
            "--binary",
            "/tmp/agent-browser",
            "--manifest",
            "/tmp/manifest.json",
            "--input-closure",
            "/tmp/closure.json",
            "--sealed-artifact",
            "/tmp/sealed.json",
            "--dry-run",
            "--apply",
        ]))
        .unwrap_err()
        .contains("mutually exclusive"));
    }

    #[test]
    fn recovery_routes_only_exact_existing_transaction_actions() {
        let forwarded = candidate_recovery_args(
            &args(&[
                "candidate",
                "recover",
                "resume",
                "--transaction-id",
                "upgrade-1",
                "--expected-revision",
                "7",
                "--candidate-generation",
                "generation-a",
                "--census-digest",
                "none",
            ]),
            true,
        )
        .unwrap();

        assert_eq!(
            forwarded,
            args(&[
                "install",
                "transactions",
                "resume",
                "--transaction-id",
                "upgrade-1",
                "--expected-revision",
                "7",
                "--candidate-generation",
                "generation-a",
                "--census-digest",
                "none",
                "--json",
            ])
        );
        assert!(candidate_recovery_args(
            &args(&[
                "candidate",
                "recover",
                "retry",
                "--transaction-id",
                "upgrade-1"
            ]),
            false,
        )
        .unwrap_err()
        .contains("expected resume, rollback, or close"));
    }

    #[test]
    fn coordination_choices_commit_through_exact_compare_and_swap() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-candidate-coordinate-{}",
            uuid::Uuid::new_v4()
        ));
        let store = crate::candidate_coordination::CandidateCoordinationStore::production(&root);
        let active = store
            .start_or_join_install("install-a", "candidate-a", "artifact-a")
            .unwrap();
        let current = store.read().unwrap();
        let queued = parse_coordination_arguments(&args(&[
            "candidate",
            "coordinate",
            "queue",
            "--request-id",
            "queue-b",
            "--candidate-id",
            "candidate-b",
            "--artifact-id",
            "artifact-b",
            "--expected-revision",
            &current.revision.to_string(),
            "--expected-fencing-generation",
            &current.fencing_generation.to_string(),
        ]))
        .unwrap();
        let queued_report = apply_coordination_transition(&root, queued).unwrap();
        assert_eq!(queued_report["coordinationReceipt"]["outcome"], "queued");
        assert_eq!(
            queued_report["consequences"][1],
            "no_runtime_effect_performed"
        );

        let current = store.read().unwrap();
        let cancelled = parse_coordination_arguments(&args(&[
            "candidate",
            "coordinate",
            "cancel-active",
            "--request-id",
            "cancel-a",
            "--candidate-id",
            "candidate-a",
            "--artifact-id",
            "artifact-a",
            "--operation-id",
            &active.operation_id,
            "--expected-revision",
            &current.revision.to_string(),
            "--expected-fencing-generation",
            &current.fencing_generation.to_string(),
        ]))
        .unwrap();
        let cancelled_report = apply_coordination_transition(&root, cancelled).unwrap();
        assert_eq!(
            cancelled_report["coordinationReceipt"]["outcome"],
            "cancelled"
        );
        assert!(store.read().unwrap().active().is_none());

        let current = store.read().unwrap();
        let queued_operation = current.queue()[0].clone();
        let activated = parse_coordination_arguments(&args(&[
            "candidate",
            "coordinate",
            "activate-queued",
            "--request-id",
            "activate-b",
            "--candidate-id",
            "candidate-b",
            "--artifact-id",
            "artifact-b",
            "--operation-id",
            &queued_operation.operation_id,
            "--expected-revision",
            &current.revision.to_string(),
            "--expected-fencing-generation",
            &current.fencing_generation.to_string(),
        ]))
        .unwrap();
        let activated_report = apply_coordination_transition(&root, activated).unwrap();
        assert_eq!(
            activated_report["coordinationReceipt"]["outcome"],
            "activated_queued"
        );
        assert_eq!(
            store.read().unwrap().active().unwrap().candidate_id,
            "candidate-b"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn coordination_choice_parser_requires_operation_identity_by_action() {
        let common = [
            "candidate",
            "coordinate",
            "supersede",
            "--request-id",
            "supersede-a",
            "--candidate-id",
            "candidate-b",
            "--artifact-id",
            "artifact-b",
            "--expected-revision",
            "2",
            "--expected-fencing-generation",
            "1",
        ];
        assert!(parse_coordination_arguments(&args(&common))
            .unwrap_err()
            .contains("requires --operation-id"));
        let mut queued = common.to_vec();
        queued[2] = "queue";
        queued.extend(["--operation-id", "operation-a"]);
        assert!(parse_coordination_arguments(&args(&queued))
            .unwrap_err()
            .contains("does not accept --operation-id"));
    }

    fn sha256_file(path: &Path) -> String {
        sha256_bytes(&fs::read(path).unwrap())
    }

    fn sha256_bytes(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};

        hex::encode(Sha256::digest(bytes))
    }
}
