//! Provider-free candidate test execution and receipt coordination.
//!
//! Only named repository suites are accepted. Each run receives isolated
//! process state and output roots below the source checkout's `cli/target`.
//! Neither installed runtime is inspected or mutated.

use agent_browser_candidate::{
    coordinate_test_run, CandidateManifest, ExecutableInputClosure, SealedArtifact,
    TestResourceClass, TestRunDecision, TestRunIdentity, TestRunRecord, TestRunState,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use uuid::Uuid;

const TEST_RESULT_SCHEMA_VERSION: &str = "agent-browser.candidate-test-result.v1";
const CANDIDATE_BINARY_PROGRAM: &str = "@candidate-binary";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestMode {
    DryRun,
    Apply,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestArguments {
    repo_root: PathBuf,
    binary: PathBuf,
    manifest: PathBuf,
    input_closure: PathBuf,
    sealed_artifact: PathBuf,
    suite_revision: String,
    selections: Vec<String>,
    mode: TestMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestCommand {
    program: String,
    args: Vec<String>,
}

pub(crate) fn run_candidate_test(args: &[String], json_output: bool) -> ! {
    let result = parse_test_arguments(args).and_then(execute_candidate_test);
    match result {
        Ok(report) if json_output => println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .unwrap_or_else(|_| r#"{"success":false,"error":"serialization failed"}"#.into())
        ),
        Ok(report) => {
            println!(
                "Candidate test outcome: {}",
                report
                    .get("outcome")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            );
            if let Some(run_id) = report.get("runId").and_then(Value::as_str) {
                println!("Run: {run_id}");
            }
            if let Some(locator) = report.get("receiptLocator").and_then(Value::as_str) {
                println!("Receipt: {locator}");
            }
        }
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
            eprintln!("Candidate test failed: {error}");
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}

fn parse_test_arguments(args: &[String]) -> Result<TestArguments, String> {
    let mut repo_root = None;
    let mut binary = None;
    let mut manifest = None;
    let mut input_closure = None;
    let mut sealed_artifact = None;
    let mut suite_revision = None;
    let mut selections = Vec::new();
    let mut mode = None;
    let mut index = 2;
    while index < args.len() {
        let argument = args[index].as_str();
        let value_slot = match argument {
            "--repo-root" => Some((&mut repo_root, "--repo-root")),
            "--binary" => Some((&mut binary, "--binary")),
            "--manifest" => Some((&mut manifest, "--manifest")),
            "--input-closure" => Some((&mut input_closure, "--input-closure")),
            "--sealed-artifact" => Some((&mut sealed_artifact, "--sealed-artifact")),
            "--suite-revision" => Some((&mut suite_revision, "--suite-revision")),
            "--selection" => {
                index += 1;
                selections.push(
                    args.get(index)
                        .filter(|value| !value.trim().is_empty())
                        .cloned()
                        .ok_or_else(|| "candidate test requires --selection <suite>".to_string())?,
                );
                None
            }
            "--dry-run" => {
                set_test_mode(&mut mode, TestMode::DryRun)?;
                None
            }
            "--apply" => {
                set_test_mode(&mut mode, TestMode::Apply)?;
                None
            }
            unknown => return Err(format!("Unknown candidate test argument: {unknown}")),
        };
        if let Some((slot, name)) = value_slot {
            if slot.is_some() {
                return Err(format!("{name} may be specified only once"));
            }
            index += 1;
            *slot = Some(
                args.get(index)
                    .filter(|value| !value.trim().is_empty())
                    .cloned()
                    .ok_or_else(|| format!("candidate test requires {name} <value>"))?,
            );
        }
        index += 1;
    }
    selections.sort();
    selections.dedup();
    if selections.is_empty() {
        return Err("candidate test requires at least one --selection <suite>".to_string());
    }
    for selection in &selections {
        commands_for_selection(selection)?;
    }
    Ok(TestArguments {
        repo_root: PathBuf::from(
            repo_root.ok_or_else(|| "candidate test requires --repo-root <path>".to_string())?,
        ),
        binary: PathBuf::from(
            binary.ok_or_else(|| "candidate test requires --binary <path>".to_string())?,
        ),
        manifest: PathBuf::from(
            manifest.ok_or_else(|| "candidate test requires --manifest <path>".to_string())?,
        ),
        input_closure: PathBuf::from(
            input_closure
                .ok_or_else(|| "candidate test requires --input-closure <path>".to_string())?,
        ),
        sealed_artifact: PathBuf::from(
            sealed_artifact
                .ok_or_else(|| "candidate test requires --sealed-artifact <path>".to_string())?,
        ),
        suite_revision: suite_revision
            .ok_or_else(|| "candidate test requires --suite-revision <commit>".to_string())?,
        selections,
        mode: mode.ok_or_else(|| {
            "candidate test requires exactly one of --dry-run or --apply".to_string()
        })?,
    })
}

fn set_test_mode(mode: &mut Option<TestMode>, requested: TestMode) -> Result<(), String> {
    if mode.is_some() {
        return Err("candidate test accepts exactly one mode".to_string());
    }
    *mode = Some(requested);
    Ok(())
}

fn commands_for_selection(selection: &str) -> Result<Vec<TestCommand>, String> {
    let command = |program: &str, args: &[&str]| TestCommand {
        program: program.to_string(),
        args: args.iter().map(|value| (*value).to_string()).collect(),
    };
    match selection {
        "candidate-kernel" => Ok(vec![command(
            "scripts/ci/rust-tests.sh",
            &["--compartment", "candidate"],
        )]),
        "candidate-build-adapter" => Ok([
            "test:candidate-executable-input",
            "test:candidate-build-executor",
            "test:candidate-build-filesystem-adapter",
            "test:candidate-build-command",
            "test:candidate-crate-architecture",
        ]
        .into_iter()
        .map(|script| command("pnpm", &[script]))
        .collect()),
        "candidate-cli" => Ok(vec![
            command(
                "scripts/ci/rust-tests.sh",
                &["--focused", "candidate::tests"],
            ),
            command(
                "scripts/ci/rust-tests.sh",
                &["--focused", "candidate_build::tests"],
            ),
        ]),
        _ => Err(format!(
            "candidate_test_selection_unsupported:{selection}; expected candidate-kernel, candidate-build-adapter, or candidate-cli"
        )),
    }
}

fn execute_candidate_test(args: TestArguments) -> Result<Value, String> {
    let repo_root = args
        .repo_root
        .canonicalize()
        .map_err(|error| format!("candidate_test_repo_invalid:{error}"))?;
    let observed_revision = command_text(&repo_root, "git", &["rev-parse", "HEAD"])?;
    if observed_revision.trim() != args.suite_revision {
        return Err(format!(
            "candidate_test_suite_revision_changed:{}:{}",
            args.suite_revision,
            observed_revision.trim()
        ));
    }
    let manifest_bytes = fs::read(&args.manifest)
        .map_err(|error| format!("candidate_test_manifest_read_failed:{error}"))?;
    let closure_bytes = fs::read(&args.input_closure)
        .map_err(|error| format!("candidate_test_closure_read_failed:{error}"))?;
    let sealed_bytes = fs::read(&args.sealed_artifact)
        .map_err(|error| format!("candidate_test_seal_read_failed:{error}"))?;
    let binary_path = args
        .binary
        .canonicalize()
        .map_err(|error| format!("candidate_test_binary_path_invalid:{error}"))?;
    let binary_bytes = fs::read(&binary_path)
        .map_err(|error| format!("candidate_test_binary_read_failed:{error}"))?;
    let manifest: CandidateManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("candidate_test_manifest_invalid:{error}"))?;
    let closure: ExecutableInputClosure = serde_json::from_slice(&closure_bytes)
        .map_err(|error| format!("candidate_test_closure_invalid:{error}"))?;
    let sealed: SealedArtifact = serde_json::from_slice(&sealed_bytes)
        .map_err(|error| format!("candidate_test_seal_invalid:{error}"))?;
    let manifest_sha256 = sha256(&manifest_bytes);
    sealed
        .validate_candidate_manifest(&manifest, &closure, &manifest_sha256)
        .map_err(|error| error.to_string())?;
    let binary_sha256 = sha256(&binary_bytes);
    if binary_sha256 != manifest.binary_sha256 {
        return Err("candidate_test_binary_digest_mismatch".to_string());
    }

    let mut commands = vec![TestCommand {
        program: CANDIDATE_BINARY_PROGRAM.to_string(),
        args: vec!["candidate".to_string(), "--help".to_string()],
    }];
    commands.extend(
        args.selections
            .iter()
            .map(|selection| commands_for_selection(selection))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
    );
    let fixture_sha256 = digest_json(&("agent-browser.candidate-test-fixture.v1", &commands))?;
    let runtime_capability_sha256 = digest_json(&(
        "agent-browser.candidate-test-runtime.v1",
        std::env::consts::OS,
        std::env::consts::ARCH,
        &commands,
    ))?;
    let environment_input_sha256 = digest_json(&serde_json::Map::<String, Value>::new())?;
    let identity = TestRunIdentity::new(
        manifest_sha256,
        binary_sha256,
        args.suite_revision,
        args.selections,
        fixture_sha256,
        manifest.target,
        runtime_capability_sha256,
        environment_input_sha256,
        TestResourceClass::IsolatedProviderFree,
    )
    .map_err(|error| error.to_string())?;
    let store = CandidateTestStore::new(&repo_root);
    coordinate_and_execute(
        &store,
        identity,
        binary_path,
        commands,
        args.mode,
        run_test_commands,
    )
}

fn command_text(root: &Path, program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("candidate_test_command_start_failed:{program}:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "candidate_test_command_failed:{program}:{}",
            output.status
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("candidate_test_command_output_invalid:{program}:{error}"))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn digest_json(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("candidate_test_identity_serialize_failed:{error}"))
}

#[derive(Debug)]
struct CandidateTestStore {
    root: PathBuf,
    output_root: PathBuf,
}

impl CandidateTestStore {
    fn new(repo_root: &Path) -> Self {
        Self {
            root: repo_root.join("cli/target/candidate-test-state"),
            output_root: repo_root.join("cli/target/candidate-tests"),
        }
    }

    fn active_path(&self, digest: &str) -> PathBuf {
        self.root.join("active").join(format!("{digest}.json"))
    }

    fn completed_path(&self, digest: &str) -> PathBuf {
        self.root.join("completed").join(format!("{digest}.json"))
    }

    fn failed_path(&self, run_id: &str) -> PathBuf {
        self.root.join("failed").join(format!("{run_id}.json"))
    }
}

type TestRunner = fn(&Path, &Path, &Path, &[TestCommand]) -> Result<(), String>;

fn coordinate_and_execute(
    store: &CandidateTestStore,
    identity: TestRunIdentity,
    candidate_binary: PathBuf,
    commands: Vec<TestCommand>,
    mode: TestMode,
    runner: TestRunner,
) -> Result<Value, String> {
    let digest = identity.digest();
    let completed = read_record(&store.completed_path(&digest))?
        .into_iter()
        .collect::<Vec<_>>();
    let active = if completed.is_empty() {
        read_record(&store.active_path(&digest))?
            .into_iter()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let decision =
        coordinate_test_run(&identity, &active, &completed).map_err(|error| error.to_string())?;
    if mode == TestMode::DryRun {
        return Ok(test_report(
            "test_recommended",
            None,
            &identity,
            &decision,
            None,
        ));
    }
    match decision {
        TestRunDecision::JoinActive { run_id } => Ok(test_report(
            "joined_existing",
            Some(&run_id),
            &identity,
            &TestRunDecision::JoinActive {
                run_id: run_id.clone(),
            },
            None,
        )),
        TestRunDecision::ReuseReceipt {
            run_id,
            receipt_locator,
        } => Ok(test_report(
            "reused_receipt",
            Some(&run_id),
            &identity,
            &TestRunDecision::ReuseReceipt {
                run_id: run_id.clone(),
                receipt_locator: receipt_locator.clone(),
            },
            Some(&receipt_locator),
        )),
        TestRunDecision::WaitForSharedResource { .. } => {
            Err("candidate_test_unexpected_shared_resource".to_string())
        }
        TestRunDecision::StartIsolated { .. } => {
            let run_id = format!("candidate-test-{}", Uuid::new_v4());
            let active_path = store.active_path(&digest);
            let active_record = TestRunRecord::active(&run_id, identity.clone());
            if !write_new_record(&active_path, &active_record)? {
                let joined = read_record(&active_path)?
                    .ok_or_else(|| "candidate_test_active_claim_disappeared".to_string())?;
                if joined.state == TestRunState::Active && joined.identity == identity {
                    return Ok(test_report(
                        "joined_existing",
                        Some(&joined.run_id),
                        &identity,
                        &TestRunDecision::JoinActive {
                            run_id: joined.run_id.clone(),
                        },
                        None,
                    ));
                }
                return Err("candidate_test_active_claim_conflict".to_string());
            }
            let output_root = store.output_root.join(&digest[..32]).join(&run_id);
            let result = runner(&output_root, &store.root, &candidate_binary, &commands);
            let receipt_path = if result.is_ok() {
                store.completed_path(&digest)
            } else {
                store.failed_path(&run_id)
            };
            let terminal = TestRunRecord {
                run_id: run_id.clone(),
                identity: identity.clone(),
                state: if result.is_ok() {
                    TestRunState::Passed
                } else {
                    TestRunState::Failed
                },
                hermetic: result.is_ok(),
                terminal_cleanup_proven: result.is_ok(),
                receipt_locator: Some(receipt_path.display().to_string()),
            };
            if !write_new_record(&receipt_path, &terminal)? {
                return Err("candidate_test_terminal_receipt_exists".to_string());
            }
            remove_claim(&active_path)?;
            if let Err(error) = result {
                return Err(format!("{error}:receipt={}", receipt_path.display()));
            }
            Ok(test_report(
                "test_passed",
                Some(&run_id),
                &identity,
                &TestRunDecision::ReuseReceipt {
                    run_id: run_id.clone(),
                    receipt_locator: receipt_path.display().to_string(),
                },
                Some(&receipt_path.display().to_string()),
            ))
        }
    }
}

fn run_test_commands(
    output_root: &Path,
    state_root: &Path,
    candidate_binary: &Path,
    commands: &[TestCommand],
) -> Result<(), String> {
    let original_home =
        std::env::var_os("HOME").ok_or_else(|| "candidate_test_home_missing".to_string())?;
    let cargo_home = std::env::var_os("CARGO_HOME").unwrap_or_else(|| {
        PathBuf::from(&original_home)
            .join(".cargo")
            .into_os_string()
    });
    let rustup_home = std::env::var_os("RUSTUP_HOME").unwrap_or_else(|| {
        PathBuf::from(&original_home)
            .join(".rustup")
            .into_os_string()
    });
    for directory in [
        output_root.to_path_buf(),
        output_root.join("home"),
        output_root.join("config"),
        output_root.join("data"),
        output_root.join("state"),
        output_root.join("cache"),
        output_root.join("runtime"),
        output_root.join("tmp"),
        output_root.join("cargo-target"),
    ] {
        fs::create_dir_all(&directory)
            .map_err(|error| format!("candidate_test_output_create_failed:{error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
                .map_err(|error| format!("candidate_test_output_protect_failed:{error}"))?;
        }
    }
    let log_path = output_root.join("test.log");
    let log = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&log_path)
        .map_err(|error| format!("candidate_test_log_create_failed:{error}"))?;
    for command in commands {
        let stdout = log
            .try_clone()
            .map_err(|error| format!("candidate_test_log_clone_failed:{error}"))?;
        let stderr = log
            .try_clone()
            .map_err(|error| format!("candidate_test_log_clone_failed:{error}"))?;
        let program = if command.program == CANDIDATE_BINARY_PROGRAM {
            candidate_binary.as_os_str()
        } else {
            std::ffi::OsStr::new(&command.program)
        };
        let mut process = Command::new(program);
        process
            .args(&command.args)
            .current_dir(
                state_root
                    .ancestors()
                    .nth(3)
                    .ok_or_else(|| "candidate_test_repo_root_missing".to_string())?,
            )
            .env("HOME", output_root.join("home"))
            .env("XDG_CONFIG_HOME", output_root.join("config"))
            .env("XDG_DATA_HOME", output_root.join("data"))
            .env("XDG_STATE_HOME", output_root.join("state"))
            .env("XDG_CACHE_HOME", output_root.join("cache"))
            .env("TMPDIR", output_root.join("tmp"))
            .env("CARGO_TARGET_DIR", output_root.join("cargo-target"))
            .env("CARGO_HOME", &cargo_home)
            .env("RUSTUP_HOME", &rustup_home)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        if command.program != "scripts/ci/cargo-safe.sh"
            && command.program != "scripts/ci/rust-tests.sh"
        {
            process.env("XDG_RUNTIME_DIR", output_root.join("runtime"));
        }
        let status = process.status().map_err(|error| {
            format!(
                "candidate_test_command_start_failed:{}:{error}",
                command.program
            )
        })?;
        if !status.success() {
            return Err(format!(
                "candidate_test_command_failed:{}:{status}:{}",
                command.program,
                log_path.display()
            ));
        }
    }
    log.sync_all()
        .map_err(|error| format!("candidate_test_log_sync_failed:{error}"))?;
    Ok(())
}

fn read_record(path: &Path) -> Result<Option<TestRunRecord>, String> {
    match fs::read(path) {
        Ok(bytes) => {
            let record: TestRunRecord = serde_json::from_slice(&bytes).map_err(|error| {
                format!("candidate_test_record_invalid:{}:{error}", path.display())
            })?;
            record.validate().map_err(|error| {
                format!("candidate_test_record_invalid:{}:{error}", path.display())
            })?;
            Ok(Some(record))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "candidate_test_record_read_failed:{}:{error}",
            path.display()
        )),
    }
}

fn write_new_record(path: &Path, record: &TestRunRecord) -> Result<bool, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "candidate_test_record_parent_missing".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("candidate_test_record_directory_failed:{error}"))?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(false),
        Err(error) => return Err(format!("candidate_test_record_create_failed:{error}")),
    };
    let mut bytes = serde_json::to_vec_pretty(record)
        .map_err(|error| format!("candidate_test_record_serialize_failed:{error}"))?;
    bytes.push(b'\n');
    file.write_all(&bytes)
        .map_err(|error| format!("candidate_test_record_write_failed:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("candidate_test_record_sync_failed:{error}"))?;
    sync_directory(parent)?;
    Ok(true)
}

fn remove_claim(path: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|error| format!("candidate_test_claim_remove_failed:{error}"))?;
    sync_directory(
        path.parent()
            .ok_or_else(|| "candidate_test_claim_parent_missing".to_string())?,
    )
}

fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("candidate_test_directory_sync_failed:{error}"))?;
    Ok(())
}

fn test_report(
    outcome: &str,
    run_id: Option<&str>,
    identity: &TestRunIdentity,
    decision: &TestRunDecision,
    receipt_locator: Option<&str>,
) -> Value {
    json!({
        "schemaVersion": TEST_RESULT_SCHEMA_VERSION,
        "success": true,
        "outcome": outcome,
        "runId": run_id,
        "identity": identity,
        "decision": decision,
        "receiptLocator": receipt_locator,
        "consequences": if outcome == "test_recommended" {
            vec!["no_effect_performed"]
        } else {
            vec!["provider_free_test_only", "no_runtime_effect_performed"]
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn identity(selection: &[&str]) -> TestRunIdentity {
        TestRunIdentity::new(
            digest('a'),
            digest('b'),
            "suite-revision",
            selection.iter().map(|value| (*value).to_string()).collect(),
            digest('c'),
            "x86_64-unknown-linux-gnu",
            digest('d'),
            digest('e'),
            TestResourceClass::IsolatedProviderFree,
        )
        .unwrap()
    }

    fn passing_runner(
        root: &Path,
        _state: &Path,
        _binary: &Path,
        _commands: &[TestCommand],
    ) -> Result<(), String> {
        fs::create_dir_all(root).map_err(|error| error.to_string())
    }

    fn failing_runner(
        root: &Path,
        _state: &Path,
        _binary: &Path,
        _commands: &[TestCommand],
    ) -> Result<(), String> {
        fs::create_dir_all(root).map_err(|error| error.to_string())?;
        Err("fixture test failure".to_string())
    }

    #[test]
    fn dry_run_is_zero_effect_and_apply_reuses_exact_receipt() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-candidate-test-store-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        let store = CandidateTestStore {
            root: root.join("state"),
            output_root: root.join("outputs"),
        };
        let requested = identity(&["candidate-kernel"]);
        let commands = commands_for_selection("candidate-kernel").unwrap();
        let dry_run = coordinate_and_execute(
            &store,
            requested.clone(),
            PathBuf::from("/fixture/candidate"),
            commands.clone(),
            TestMode::DryRun,
            passing_runner,
        )
        .unwrap();
        assert_eq!(dry_run["outcome"], "test_recommended");
        assert!(!root.exists());

        let applied = coordinate_and_execute(
            &store,
            requested.clone(),
            PathBuf::from("/fixture/candidate"),
            commands.clone(),
            TestMode::Apply,
            passing_runner,
        )
        .unwrap();
        assert_eq!(applied["outcome"], "test_passed");
        let reused = coordinate_and_execute(
            &store,
            requested,
            PathBuf::from("/fixture/candidate"),
            commands,
            TestMode::Apply,
            passing_runner,
        )
        .unwrap();
        assert_eq!(reused["outcome"], "reused_receipt");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn active_run_joins_and_failed_receipt_does_not_block_retry() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-candidate-test-recovery-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        let store = CandidateTestStore {
            root: root.join("state"),
            output_root: root.join("outputs"),
        };
        let requested = identity(&["candidate-build-adapter"]);
        let digest = requested.digest();
        let active = TestRunRecord::active("active-fixture", requested.clone());
        assert!(write_new_record(&store.active_path(&digest), &active).unwrap());
        let joined = coordinate_and_execute(
            &store,
            requested.clone(),
            PathBuf::from("/fixture/candidate"),
            Vec::new(),
            TestMode::Apply,
            passing_runner,
        )
        .unwrap();
        assert_eq!(joined["outcome"], "joined_existing");
        remove_claim(&store.active_path(&digest)).unwrap();

        let failed = coordinate_and_execute(
            &store,
            requested.clone(),
            PathBuf::from("/fixture/candidate"),
            Vec::new(),
            TestMode::Apply,
            failing_runner,
        )
        .unwrap_err();
        assert!(failed.contains("fixture test failure"));
        assert!(failed.contains("receipt="));
        let failed_count = fs::read_dir(store.root.join("failed")).unwrap().count();
        assert_eq!(failed_count, 1);

        let retry = coordinate_and_execute(
            &store,
            requested,
            PathBuf::from("/fixture/candidate"),
            Vec::new(),
            TestMode::Apply,
            passing_runner,
        )
        .unwrap();
        assert_eq!(retry["outcome"], "test_passed");
        assert_eq!(fs::read_dir(store.root.join("failed")).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parser_canonicalizes_named_provider_free_selections() {
        let args = vec![
            "candidate".to_string(),
            "test".to_string(),
            "--repo-root".to_string(),
            "/repo".to_string(),
            "--binary".to_string(),
            "/binary".to_string(),
            "--manifest".to_string(),
            "/manifest".to_string(),
            "--input-closure".to_string(),
            "/closure".to_string(),
            "--sealed-artifact".to_string(),
            "/seal".to_string(),
            "--suite-revision".to_string(),
            "revision".to_string(),
            "--selection".to_string(),
            "candidate-kernel".to_string(),
            "--selection".to_string(),
            "candidate-build-adapter".to_string(),
            "--dry-run".to_string(),
        ];
        let parsed = parse_test_arguments(&args).unwrap();
        assert_eq!(
            parsed.selections,
            vec![
                "candidate-build-adapter".to_string(),
                "candidate-kernel".to_string()
            ]
        );
        assert_eq!(parsed.mode, TestMode::DryRun);
    }

    #[test]
    fn public_dry_run_validates_exact_artifact_without_creating_state() {
        use agent_browser_candidate::{
            ArtifactClass, BuildIdentity, BuildProfileConfiguration, ExecutableInput,
            ExecutableInputContext, InputCategory, SourceProvenance, SourceTreeState,
        };
        use std::collections::BTreeMap;

        let root = std::env::temp_dir().join(format!(
            "agent-browser-candidate-test-dry-run-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        fs::create_dir_all(root.join("fixture")).unwrap();
        fs::write(root.join("tracked.txt"), "tracked\n").unwrap();
        let git = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(&root)
                .status()
                .unwrap();
            assert!(status.success());
        };
        git(&["init", "-q"]);
        git(&["add", "tracked.txt"]);
        git(&[
            "-c",
            "user.name=Candidate Fixture",
            "-c",
            "user.email=candidate@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "fixture",
        ]);
        let revision = command_text(&root, "git", &["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();
        let binary_path = root.join("fixture/agent-browser");
        fs::write(&binary_path, b"candidate-binary").unwrap();
        let binary_sha256 = sha256(b"candidate-binary");
        let closure = ExecutableInputClosure::new(
            ExecutableInputContext {
                target: "x86_64-unknown-linux-gnu".to_string(),
                toolchain: "rustc fixture".to_string(),
                cargo_profile: "ci".to_string(),
                resolved_build_profile: BuildProfileConfiguration {
                    opt_level: "3".to_string(),
                    lto: "thin".to_string(),
                    codegen_units: 16,
                    strip: true,
                },
                features: Vec::new(),
                reviewed_environment_inputs: BTreeMap::new(),
            },
            vec![ExecutableInput {
                path: "cli/src/main.rs".to_string(),
                sha256: digest('a'),
                category: InputCategory::RustSource,
            }],
        )
        .unwrap();
        let support_sha256 = digest('b');
        let manifest = CandidateManifest::new(
            SourceProvenance {
                commit: revision.clone(),
                tree: digest('c'),
                state: SourceTreeState::Clean,
            },
            &closure,
            ArtifactClass::FastIteration,
            binary_sha256.clone(),
            support_sha256.clone(),
            "2026-09-16T12:00:00Z".to_string(),
        )
        .unwrap();
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        let sealed = SealedArtifact::new(
            "build-fixture",
            BuildIdentity::new(&closure, ArtifactClass::FastIteration),
            binary_sha256,
            support_sha256,
            sha256(&manifest_bytes),
        )
        .unwrap();
        let manifest_path = root.join("fixture/candidate-manifest.json");
        let closure_path = root.join("fixture/executable-input-closure.json");
        let sealed_path = root.join("fixture/sealed-artifact.json");
        fs::write(&manifest_path, manifest_bytes).unwrap();
        fs::write(&closure_path, serde_json::to_vec_pretty(&closure).unwrap()).unwrap();
        fs::write(&sealed_path, serde_json::to_vec_pretty(&sealed).unwrap()).unwrap();
        let parsed = TestArguments {
            repo_root: root.clone(),
            binary: binary_path,
            manifest: manifest_path,
            input_closure: closure_path,
            sealed_artifact: sealed_path,
            suite_revision: revision,
            selections: vec!["candidate-kernel".to_string()],
            mode: TestMode::DryRun,
        };
        let report = execute_candidate_test(parsed).unwrap();
        assert_eq!(report["outcome"], "test_recommended");
        assert!(!root.join("cli/target/candidate-test-state").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
