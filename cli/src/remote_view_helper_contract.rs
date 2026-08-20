//! Typed compatibility contract for the root-owned remote-view helper.
//!
//! Route-user password updates are unattended only when the helper advertises
//! the fixed SHA-512 crypt path that bypasses `chpasswd`'s PAM default.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub(crate) const FIXED_HELPER_PATH: &str =
    "/usr/local/libexec/agent-browser/agent-browser-privileged-helper";
const REQUIRED_COMMANDS: [&str; 5] = [
    "check",
    "status-json",
    "ensure-rdp-route-user",
    "restart-xrdp",
    "grant-display-access",
];
const OPTIONAL_COMMANDS: [&str; 1] = ["verify-install"];

pub(crate) fn status_contract_ready(report: &Value) -> bool {
    report.get("success").and_then(Value::as_bool) == Some(true)
        && report
            .pointer("/parsed/schemaVersion")
            .and_then(Value::as_i64)
            == Some(1)
        && report
            .pointer("/parsed/helperVersion")
            .and_then(Value::as_str)
            .is_some_and(|value| value.starts_with("2026-06-23.p44-route-desktop-v"))
        && report
            .pointer("/parsed/routeDesktopSession/ready")
            .and_then(Value::as_bool)
            == Some(true)
        && report
            .pointer("/parsed/routeDesktopSession/terminalStartupDetected")
            .and_then(Value::as_bool)
            == Some(false)
        && report
            .pointer("/parsed/displayAccess/supportsFilesystemX11Socket")
            .and_then(Value::as_bool)
            == Some(true)
        && report
            .pointer("/parsed/displayAccess/supportsAbstractX11Socket")
            .and_then(Value::as_bool)
            == Some(true)
        && report
            .pointer("/parsed/displayAccess/boundedXhostTimeoutSeconds")
            .and_then(Value::as_i64)
            .is_some_and(|value| value > 0 && value <= 2)
        && report
            .pointer("/parsed/routeUserCredentialUpdate/pamBypassed")
            .and_then(Value::as_bool)
            == Some(true)
        && report
            .pointer("/parsed/routeUserCredentialUpdate/cryptMethod")
            .and_then(Value::as_str)
            == Some("SHA512")
        && report
            .pointer("/parsed/routeUserCredentialUpdate/shaRounds")
            .and_then(Value::as_i64)
            == Some(100_000)
}

pub(crate) fn helper_contract_report(
    helper_path: &str,
    helper_check: &Value,
    helper_status: &Value,
    sudoers_ready: bool,
) -> Value {
    let source = fs::read_to_string(helper_path).unwrap_or_default();
    let provenance = helper_provenance(helper_path);
    let provenance_ready = provenance
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut report = evaluate_helper_contract(
        &source,
        helper_status,
        helper_check.get("success").and_then(Value::as_bool) == Some(true),
        sudoers_ready,
        provenance_ready,
    );
    if let Some(object) = report.as_object_mut() {
        object.insert("provenance".to_string(), provenance);
    }
    report
}

fn evaluate_helper_contract(
    source: &str,
    helper_status: &Value,
    check_ready: bool,
    sudoers_ready: bool,
    provenance_ready: bool,
) -> Value {
    let command_set = command_set(source);
    let missing_required_commands = REQUIRED_COMMANDS
        .iter()
        .filter(|command| !command_set.iter().any(|candidate| *candidate == **command))
        .copied()
        .collect::<Vec<_>>();
    let missing_optional_commands = OPTIONAL_COMMANDS
        .iter()
        .filter(|command| !command_set.iter().any(|candidate| *candidate == **command))
        .copied()
        .collect::<Vec<_>>();
    let capability_ready = status_contract_ready(helper_status);
    let ready = missing_required_commands.is_empty()
        && capability_ready
        && check_ready
        && sudoers_ready
        && provenance_ready;
    json!({
        "schemaVersion": "agent-browser.remote-view-helper-contract.v1",
        "contractVersion": helper_status.pointer("/parsed/schemaVersion"),
        "helperVersion": helper_status.pointer("/parsed/helperVersion"),
        "commandSet": command_set,
        "requiredCommands": REQUIRED_COMMANDS,
        "optionalCommands": OPTIONAL_COMMANDS,
        "missingRequiredCommands": missing_required_commands,
        "missingOptionalCommands": missing_optional_commands,
        "verifyInstallSupported": !missing_optional_commands.contains(&"verify-install"),
        "capabilities": {
            "ready": capability_ready,
            "routeDesktopSession": helper_status.pointer("/parsed/routeDesktopSession"),
            "displayAccess": helper_status.pointer("/parsed/displayAccess"),
            "routeUserCredentialUpdate": helper_status.pointer("/parsed/routeUserCredentialUpdate"),
            "managedChromeSandboxPolicy": helper_status.pointer("/parsed/managedChromeSandboxPolicy"),
        },
        "checkReady": check_ready,
        "sudoersReady": sudoers_ready,
        "provenanceReady": provenance_ready,
        "ready": ready,
        "requiresInteractiveSudo": !ready && (!check_ready || !sudoers_ready),
    })
}

fn command_set(source: &str) -> Vec<&'static str> {
    REQUIRED_COMMANDS
        .into_iter()
        .chain(OPTIONAL_COMMANDS)
        .filter(|command| source.contains(&format!("\n  {command})")))
        .collect()
}

fn helper_provenance(helper_path: &str) -> Value {
    let path = Path::new(helper_path);
    let metadata = fs::metadata(path).ok();
    let sha256 = fs::read(path)
        .ok()
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
    #[cfg(unix)]
    let (owner_uid, owner_gid, mode, root_owned, mode_ready) = {
        use std::os::unix::fs::MetadataExt;
        let owner_uid = metadata.as_ref().map(MetadataExt::uid);
        let owner_gid = metadata.as_ref().map(MetadataExt::gid);
        let mode = metadata.as_ref().map(|value| value.mode() & 0o777);
        (
            owner_uid,
            owner_gid,
            mode,
            owner_uid == Some(0),
            mode == Some(0o755),
        )
    };
    #[cfg(not(unix))]
    let (owner_uid, owner_gid, mode, root_owned, mode_ready): (
        Option<u32>,
        Option<u32>,
        Option<u32>,
        bool,
        bool,
    ) = (None, None, None, false, false);
    let fixed_path = helper_path == FIXED_HELPER_PATH;
    json!({
        "path": helper_path,
        "exists": metadata.is_some(),
        "fixedPath": fixed_path,
        "ownerUid": owner_uid,
        "ownerGid": owner_gid,
        "mode": mode.map(|value| format!("{value:o}")),
        "rootOwned": root_owned,
        "modeReady": mode_ready,
        "sha256": sha256,
        "ready": metadata.is_some() && fixed_path && root_owned && mode_ready,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compatible_status() -> Value {
        json!({
            "success": true,
            "parsed": {
                "schemaVersion": 1,
                "helperVersion": "2026-06-23.p44-route-desktop-v4",
                "routeDesktopSession": {
                    "ready": true,
                    "terminalStartupDetected": false
                },
                "displayAccess": {
                    "supportsFilesystemX11Socket": true,
                    "supportsAbstractX11Socket": true,
                    "boundedXhostTimeoutSeconds": 2
                },
                "routeUserCredentialUpdate": {
                    "pamBypassed": true,
                    "cryptMethod": "SHA512",
                    "shaRounds": 100000
                }
            }
        })
    }

    #[test]
    fn compatible_helper_without_optional_verify_install_remains_ready() {
        let source = "\n  check)\n  status-json)\n  ensure-rdp-route-user)\n  restart-xrdp)\n  grant-display-access)\n";
        let report = evaluate_helper_contract(source, &compatible_status(), true, true, true);

        assert_eq!(report["ready"], true);
        assert_eq!(report["verifyInstallSupported"], false);
        assert_eq!(report["missingRequiredCommands"], json!([]));
        assert_eq!(report["missingOptionalCommands"], json!(["verify-install"]));
        assert_eq!(report["requiresInteractiveSudo"], false);
    }

    #[test]
    fn missing_required_helper_capability_is_blocking() {
        let source = "\n  check)\n  status-json)\n  restart-xrdp)\n  grant-display-access)\n";
        let report = evaluate_helper_contract(source, &compatible_status(), true, true, true);

        assert_eq!(report["ready"], false);
        assert_eq!(
            report["missingRequiredCommands"],
            json!(["ensure-rdp-route-user"])
        );
    }

    #[test]
    fn legacy_pam_password_update_contract_is_blocking() {
        let source = "\n  check)\n  status-json)\n  ensure-rdp-route-user)\n  restart-xrdp)\n  grant-display-access)\n";
        let mut status = compatible_status();
        status["parsed"]
            .as_object_mut()
            .expect("parsed helper status must be an object")
            .remove("routeUserCredentialUpdate");

        let report = evaluate_helper_contract(source, &status, true, true, true);

        assert_eq!(report["ready"], false);
        assert_eq!(report["capabilities"]["ready"], false);
        assert_eq!(report["requiresInteractiveSudo"], false);
    }
}
