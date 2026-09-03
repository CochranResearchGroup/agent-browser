//! Read-only validation for an explicitly selected Service State document.
//!
//! This surface is intentionally separate from the durable store. It reads
//! only the supplied absolute path and the running executable, never acquires
//! a Service State lock, performs recovery, starts a service, or writes files.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

pub(crate) const SERVICE_STATE_VALIDATION_SCHEMA_VERSION: &str =
    "agent-browser.service-state-validation.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ServiceStateValidationClassification {
    Accepted,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ServiceStateValidationErrorCode {
    ParserError,
    InvariantError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceStateValidationError {
    pub(crate) code: ServiceStateValidationErrorCode,
    pub(crate) message: String,
}

/// Receipt proving which installed executable parsed which exact state bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceStateValidationReceipt {
    pub(crate) schema_version: &'static str,
    pub(crate) path: String,
    pub(crate) state_sha256: String,
    pub(crate) parser_identity_sha256: String,
    pub(crate) accepted: bool,
    pub(crate) classification: ServiceStateValidationClassification,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<ServiceStateValidationError>,
}

fn sha256_file(path: &Path, label: &str) -> Result<String, String> {
    let file = File::open(path)
        .map_err(|error| format!("Failed to open {label} {}: {error}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("Failed to read {label} {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn parser_identity_sha256() -> Result<String, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("Failed to resolve running parser executable: {error}"))?;
    sha256_file(&executable, "running parser executable")
}

fn error_receipt(
    path: &Path,
    state_sha256: String,
    parser_identity_sha256: String,
    code: ServiceStateValidationErrorCode,
    message: String,
) -> ServiceStateValidationReceipt {
    ServiceStateValidationReceipt {
        schema_version: SERVICE_STATE_VALIDATION_SCHEMA_VERSION,
        path: path.display().to_string(),
        state_sha256,
        parser_identity_sha256,
        accepted: false,
        classification: ServiceStateValidationClassification::Error,
        error: Some(ServiceStateValidationError { code, message }),
    }
}

/// Validate one absolute-path Service State file with the installed parser and
/// cross-record invariant checker. The input bytes and executable are hashed,
/// but neither is modified.
pub(crate) fn validate_service_state_path(
    path: &Path,
) -> Result<ServiceStateValidationReceipt, String> {
    if !path.is_absolute() {
        return Err("Service State validation path must be absolute".to_string());
    }

    let state_bytes = std::fs::read(path)
        .map_err(|error| format!("Failed to read Service State {}: {error}", path.display()))?;
    let state_sha256 = format!("{:x}", Sha256::digest(&state_bytes));
    let parser_identity_sha256 = parser_identity_sha256()?;
    let raw = match String::from_utf8(state_bytes) {
        Ok(raw) => raw,
        Err(error) => {
            return Ok(error_receipt(
                path,
                state_sha256,
                parser_identity_sha256,
                ServiceStateValidationErrorCode::ParserError,
                format!("Invalid Service State UTF-8: {error}"),
            ));
        }
    };

    let state = match super::service_store::parse_service_state_json(raw, path) {
        Ok(state) => state,
        Err(message) => {
            return Ok(error_receipt(
                path,
                state_sha256,
                parser_identity_sha256,
                ServiceStateValidationErrorCode::ParserError,
                message,
            ));
        }
    };
    if let Err(message) = super::service_state_migration::validate_service_state_invariants(&state)
    {
        return Ok(error_receipt(
            path,
            state_sha256,
            parser_identity_sha256,
            ServiceStateValidationErrorCode::InvariantError,
            message,
        ));
    }

    Ok(ServiceStateValidationReceipt {
        schema_version: SERVICE_STATE_VALIDATION_SCHEMA_VERSION,
        path: path.display().to_string(),
        state_sha256,
        parser_identity_sha256,
        accepted: true,
        classification: ServiceStateValidationClassification::Accepted,
        error: None,
    })
}

/// Dispatch the local Service State validator action without constructing a
/// daemon runtime or consulting the default Service State store.
pub(crate) fn dispatch_service_state_validation(
    command: &serde_json::Value,
) -> Option<Result<ServiceStateValidationReceipt, String>> {
    if command.get("action").and_then(serde_json::Value::as_str) != Some("service_state_validate") {
        return None;
    }
    let Some(path) = command.get("path").and_then(serde_json::Value::as_str) else {
        return Some(Err("Service State validation requires path".to_string()));
    };
    Some(validate_service_state_path(&PathBuf::from(path)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempState {
        root: PathBuf,
    }

    impl TempState {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "agent-browser-service-state-validation-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ));
            fs::create_dir_all(&root).expect("temporary validator root");
            Self { root }
        }

        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.root.join(name);
            fs::write(&path, bytes).expect("state fixture");
            path
        }
    }

    impl Drop for TempState {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn accepts_exact_bytes_with_running_executable_identity() {
        let temp = TempState::new();
        let bytes = br#"{"schemaVersion":"agent-browser.service-state.v2"}"#;
        let path = temp.write("state.json", bytes);

        let receipt = validate_service_state_path(&path).expect("validation receipt");

        assert!(receipt.accepted);
        assert_eq!(
            receipt.classification,
            ServiceStateValidationClassification::Accepted
        );
        assert_eq!(receipt.state_sha256, format!("{:x}", Sha256::digest(bytes)));
        assert_eq!(
            receipt.parser_identity_sha256,
            sha256_file(&std::env::current_exe().unwrap(), "test executable").unwrap()
        );
        assert_eq!(receipt.error, None);
    }

    #[test]
    fn returns_hashed_invariant_error_receipt() {
        let temp = TempState::new();
        let bytes = br#"{"schemaVersion":"agent-browser.service-state.v2","profiles":{"profile-key":{"id":"other"}}}"#;
        let path = temp.write("state.json", bytes);

        let receipt = validate_service_state_path(&path).expect("validation receipt");

        assert!(!receipt.accepted);
        assert_eq!(
            receipt.classification,
            ServiceStateValidationClassification::Error
        );
        assert_eq!(receipt.state_sha256, format!("{:x}", Sha256::digest(bytes)));
        let error = receipt.error.expect("typed validation error");
        assert_eq!(error.code, ServiceStateValidationErrorCode::InvariantError);
        assert!(error.message.contains("service_state_profile_key_mismatch"));
    }

    #[test]
    fn returns_hashed_parser_error_receipt() {
        let temp = TempState::new();
        let bytes = br#"{"schemaVersion":"future"}"#;
        let path = temp.write("state.json", bytes);

        let receipt = validate_service_state_path(&path).expect("validation receipt");

        assert!(!receipt.accepted);
        let error = receipt.error.expect("typed validation error");
        assert_eq!(error.code, ServiceStateValidationErrorCode::ParserError);
        assert!(error
            .message
            .contains("service_state_schema_unsupported:future"));
    }
}
