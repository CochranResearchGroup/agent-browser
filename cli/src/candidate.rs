//! Read-only CLI adapters for candidate orchestration status and manifest inspection.
//!
//! This module may inspect installed workstation records and caller-supplied
//! documents. It must not build, install, recover, or otherwise mutate either
//! runtime namespace.

use agent_browser_candidate::{CandidateManifest, CoordinationLedger, ExecutableInputClosure};
use serde_json::{json, Value};
use std::fs;

const ADVISORY_SCHEMA_VERSION: &str = "agent-browser.candidate-advisory.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
struct InspectArguments {
    manifest: String,
    input_closure: String,
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
            crate::workstation_install::workstation_upgrade_status_json()
                .and_then(|workstation| candidate_status_from_sources(workstation, coordination))
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

#[cfg(test)]
fn candidate_status_from_workstation(workstation: Value) -> Result<Value, String> {
    candidate_status_from_sources(workstation, CoordinationLedger::new("production"))
}

fn candidate_status_from_sources(
    workstation: Value,
    coordination: CoordinationLedger,
) -> Result<Value, String> {
    if workstation.get("success").and_then(Value::as_bool) != Some(true) {
        return Err("workstation status did not return a successful projection".to_string());
    }
    let transaction = workstation
        .get("latestTransaction")
        .filter(|value| !value.is_null());
    let classification = transaction
        .and_then(|value| value.get("classification"))
        .and_then(Value::as_str);
    let (observed_state, recommendation, reason_codes) = match classification {
        Some("active_convergence") => (
            "install_active",
            "observe",
            vec!["active_install_transaction"],
        ),
        Some("accepted" | "rollback_window_reviewed") => {
            ("accepted", "inspect", vec!["candidate_already_accepted"])
        }
        Some("failed_operator_required" | "terminal_blocked_open") => (
            "recovery_required",
            "recover",
            vec!["install_transaction_requires_recovery"],
        ),
        Some("terminal_rolled_back_history" | "terminal_zero_effect_history") | None => {
            ("idle", "build", vec!["no_active_candidate"])
        }
        Some(_) => (
            "recovery_required",
            "inspect",
            vec!["unknown_install_state"],
        ),
    };
    let alternatives = transaction
        .and_then(|value| value.get("safeActions"))
        .cloned()
        .unwrap_or_else(|| json!(["inspect"]));
    let active_operation = transaction.and_then(|value| {
        (observed_state == "install_active" || observed_state == "recovery_required").then(|| {
            json!({
                "kind": "install_transaction",
                "operationId": value.get("transactionId").cloned().unwrap_or(Value::Null),
                "candidateId": value.get("candidateGenerationId").cloned().unwrap_or(Value::Null),
                "revision": value.get("revision").cloned().unwrap_or(Value::Null),
                "fencingGeneration": Value::Null,
            })
        })
    });

    Ok(json!({
        "schemaVersion": ADVISORY_SCHEMA_VERSION,
        "success": true,
        "observedState": observed_state,
        "recommendation": recommendation,
        "alternatives": alternatives,
        "consequences": ["no_effect_performed"],
        "integrityPreconditions": [],
        "reasonCodes": reason_codes,
        "activeOperation": active_operation,
        "artifactReuseEligibility": {
            "eligible": false,
            "reason": "no_candidate_manifest_supplied"
        },
        "rebuildReasons": [],
        "receiptLocators": [],
        "buildProvenance": build_provenance(),
        "workstationUpgrade": workstation,
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
        BuildProfileConfiguration, ExecutableInput, ExecutableInputClosure, ExecutableInputContext,
        InputCategory,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

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
}
