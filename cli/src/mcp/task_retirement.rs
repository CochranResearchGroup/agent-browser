//! Exact owning-store cross-boot retirement; never starts a browser transport.
use super::{required_string_argument, JsonRpcError};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(
    tag = "choice",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Request {
    Inspect,
    Retire {
        operation_id: String,
        expected_binding_ref: String,
    },
    Status {
        operation_id: String,
    },
}
pub(super) fn call(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let task_id = required_string_argument(arguments, "taskId")?;
    let request: Request = serde_json::from_value(arguments["params"].clone())
        .map_err(|_| JsonRpcError::invalid_params("browser_task_retirement_parameters_invalid"))?;
    let data = match request {
        Request::Status { operation_id } => {
            if !crate::native::browser_task_operations::valid_request_id(&operation_id) {
                return Err(JsonRpcError::invalid_params(
                    "browser_task_retirement_parameters_invalid",
                ));
            }
            let data = super::task_operations::lookup(task_id, &operation_id).map_err(|_| {
                JsonRpcError::invalid_params("browser_task_retirement_receipt_unavailable")
            })?;
            if data["operation"]["action"] != "lost_task_retirement" {
                return Err(JsonRpcError::invalid_params(
                    "browser_task_retirement_receipt_unavailable",
                ));
            }
            data
        }
        other => {
            owning_call(task_id, other).map_err(|error| JsonRpcError::invalid_params(&error))?
        }
    };
    Ok(super::tool_response_from_payload(
        "browser_command",
        session,
        json!({}),
        data,
        false,
    ))
}
#[cfg(target_os = "linux")]
fn owning_call(task_id: &str, request: Request) -> Result<Value, String> {
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    match request {
        Request::Inspect => {
            let path = BrowserSessionSqliteStore::default_sqlite_path()
                .map_err(|_| "browser_task_retirement_store_unavailable")?;
            let store = BrowserSessionSqliteStore::open_read_only(&path)
                .map_err(|_| "browser_task_retirement_store_unavailable")?;
            store.inspect_task_retirement(task_id)
        }
        Request::Retire {
            operation_id,
            expected_binding_ref,
        } => {
            let request = agent_browser_service_model::LostTaskRetirementRequest {
                task_id: task_id.into(),
                operation_id,
                expected_binding_ref,
            };
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| "browser_task_retirement_clock_unavailable")?;
            let now_ms = u64::try_from(now.as_millis())
                .map_err(|_| "browser_task_retirement_clock_unavailable")?;
            let mut store = BrowserSessionSqliteStore::default_sqlite()
                .map_err(|_| "browser_task_retirement_store_unavailable")?;
            let receipt = store.retire_lost_task(&request, now_ms)?;
            Ok(
                json!({"schemaVersion":"agent-browser.task-retirement.v1","operation":super::task_operations::projection(&receipt),"providerAuthentication":"unverified","providerCleanup":"not_asserted","retryAllowed":false}),
            )
        }
        Request::Status { .. } => Err("browser_task_retirement_parameters_invalid".into()),
    }
}
#[cfg(not(target_os = "linux"))]
fn owning_call(_: &str, _: Request) -> Result<Value, String> {
    Err("browser_task_retirement_platform_unsupported".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lost_task_retirement_public_params_reject_caller_proofs_before_store() {
        for params in [
            json!({"choice":"inspect","originalPidAbsent":true}),
            json!({"choice":"retire","operationId":"op","expectedBindingRef":"00","currentBootId":"fake"}),
            json!({"choice":"status","operationId":"op","profilePath":"/private"}),
            json!({"choice":"inspect","operationId":"unexpected"}),
        ] {
            assert!(call(&json!({"taskId":"missing-task","params":params}), "unused").is_err());
        }
    }
    #[test]
    fn lost_task_retirement_public_status_after_task_removal_never_starts_transport() {
        use crate::native::browser_session_store::BrowserSessionSqliteStore;
        use agent_browser_service_model::BrowserSessionState;
        let directory =
            std::env::temp_dir().join(format!("auth650-public-{}", uuid::Uuid::new_v4()));
        let guard = crate::test_utils::EnvGuard::new(&[
            "HOME",
            "AGENT_BROWSER_HOME",
            "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
            "AGENT_BROWSER_SOCKET_DIR",
        ]);
        guard.set("HOME", directory.to_str().unwrap());
        guard.set("AGENT_BROWSER_HOME", directory.to_str().unwrap());
        guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
        guard.set(
            "AGENT_BROWSER_SOCKET_DIR",
            directory.join("never-started").to_str().unwrap(),
        );
        let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
        let mut value = serde_json::to_value(BrowserSessionState::default()).unwrap();
        value["taskOperations"] = json!({"session:removed":{"retired-original":{
            "requestId":"retired-original","taskId":"session:removed","profileId":"profile","browserId":"old-browser","tabId":"old-tab","targetId":"old-target","processPid":4100,"controlEpoch":3,"action":"lost_task_retirement","fingerprint":"private-fingerprint","processIdentity":{"pid":4100,"startToken":"private-kernel-token","executablePath":"/private/executable"},"state":"owner_acknowledged","createdAtMs":10,"completedAtMs":11,"evidenceReferences":{"retirementResult":"retired_lost"}
        }}});
        let state: BrowserSessionState = serde_json::from_value(value).unwrap();
        store.save_session_state(&state).unwrap();
        let request = json!({"jsonrpc":"2.0","id":650,"method":"tools/call","params":{"name":"browser_command","arguments":{"taskId":"session:removed","action":"task_retirement","params":{"choice":"status","operationId":"retired-original"}}}});
        let response = super::super::handle_jsonrpc_line(&request.to_string(), "unused").unwrap();
        assert!(response.get("error").is_none(), "{response}");
        let payload: Value =
            serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(
            payload["data"]["operation"]["action"],
            "lost_task_retirement"
        );
        assert_eq!(payload["data"]["currentContext"], "changed_or_missing");
        for secret in [
            "private-fingerprint",
            "private-kernel-token",
            "/private/executable",
        ] {
            assert!(!response.to_string().contains(secret));
        }
        assert!(!directory.join("never-started").exists());
        assert_eq!(store.load_session_state().unwrap(), state);
        let mut other = state.clone();
        other
            .task_operations
            .get_mut("session:removed")
            .unwrap()
            .get_mut("retired-original")
            .unwrap()
            .action = "navigate".into();
        store.save_session_state(&other).unwrap();
        let refused = super::super::handle_jsonrpc_line(&request.to_string(), "unused").unwrap();
        assert!(refused.get("error").is_some());
        assert!(!directory.join("never-started").exists());
        drop(store);
        fn unseal_owned_directories(path: &std::path::Path) {
            if !std::fs::symlink_metadata(path).unwrap().is_dir() {
                return;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
            }
            for entry in std::fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    unseal_owned_directories(&entry.path());
                }
            }
        }
        unseal_owned_directories(&directory);
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
