use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "agent-browser-service-state-validation-contract-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("temporary validation root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn run_validator(state_path: &Path, home: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_agent-browser"))
        .args([
            "service",
            "state",
            "validate",
            "--path",
            state_path.to_str().expect("UTF-8 fixture path"),
            "--json",
        ])
        .env("HOME", home)
        .env("AGENT_BROWSER_SOCKET_DIR", home.join("sockets"))
        .output()
        .expect("validator command")
}

#[test]
fn installed_validator_accepts_exact_bytes_without_runtime_writes() {
    let root = TempRoot::new();
    let home = root.path.join("home");
    fs::create_dir(&home).expect("isolated home");
    let state_path = root.path.join("candidate-state.json");
    let state_bytes = br#"{"schemaVersion":"agent-browser.service-state.v2"}"#;
    fs::write(&state_path, state_bytes).expect("candidate state");

    let output = run_validator(&state_path, &home);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON receipt");
    assert_eq!(response["success"], true);
    assert_eq!(response["data"]["accepted"], true);
    assert_eq!(response["data"]["classification"], "accepted");
    assert_eq!(response["data"]["stateSha256"], sha256(state_bytes));
    assert_eq!(
        response["data"]["parserIdentitySha256"],
        sha256(&fs::read(env!("CARGO_BIN_EXE_agent-browser")).expect("candidate executable"))
    );
    assert_eq!(
        response["data"]["schemaVersion"],
        "agent-browser.service-state-validation.v1"
    );
    assert!(
        fs::read_dir(&home)
            .expect("isolated home listing")
            .next()
            .is_none(),
        "validator wrote under the isolated home"
    );
}

#[test]
fn installed_validator_returns_exact_hashed_invariant_error() {
    let root = TempRoot::new();
    let home = root.path.join("home");
    fs::create_dir(&home).expect("isolated home");
    let state_path = root.path.join("invalid-state.json");
    let state_bytes = br#"{"schemaVersion":"agent-browser.service-state.v2","profiles":{"expected":{"id":"wrong"}}}"#;
    fs::write(&state_path, state_bytes).expect("invalid candidate state");

    let output = run_validator(&state_path, &home);

    assert_eq!(output.status.code(), Some(1));
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON receipt");
    assert_eq!(response["success"], false);
    assert_eq!(response["data"]["accepted"], false);
    assert_eq!(response["data"]["classification"], "error");
    assert_eq!(response["data"]["error"]["code"], "invariant_error");
    assert_eq!(response["data"]["stateSha256"], sha256(state_bytes));
    assert!(response["data"]["error"]["message"]
        .as_str()
        .is_some_and(|message| message.contains("service_state_profile_key_mismatch:expected")));
    assert!(
        fs::read_dir(&home)
            .expect("isolated home listing")
            .next()
            .is_none(),
        "rejected validation wrote under the isolated home"
    );
}
