//! Intent-level MCP adapters over the existing CLI transport and managed-session host.
//! Profiles and task lifetimes remain owned by their existing durable stores.
use std::process::Command;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{optional_string_argument, required_string_argument, JsonRpcError};
use crate::connection::{ensure_daemon, send_command, DaemonOptions};
use crate::native::browser_session_store::BrowserSessionSqliteStore;
use crate::native::service_model::ServiceState;
use crate::native::service_store::load_default_service_state_snapshot;

pub(super) fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "browser_open", "title": "Open a browser task",
            "description": "Create or reuse a persistent, existing or disposable profile and open one task tab. Reuse taskKey to replay without another tab. Use the returned taskId for subsequent page actions and scoped close.",
            "inputSchema": {"type":"object", "additionalProperties":false,
                "properties": {
                    "profile":{"type":"string","description":"Named profile for persistent or existing mode."},
                    "profileMode":{"type":"string","enum":["persistent","existing","disposable"],"default":"persistent"},
                    "taskKey":{"type":"string","description":"Stable task key. Distinct keys receive independent tabs; reuse the key to continue this task."},
                "url":{"type":"string","description":"Initial URL for a new task. Replaying a task returns its current page without navigating again."},
                    "serviceName":{"type":"string"},"agentName":{"type":"string"},"taskName":{"type":"string"}
                }, "required":["taskKey","url"]}
        }),
        json!({
            "name":"browser_close", "title":"Close a browser task",
            "description":"End only the addressed task and its tabs. Preserve peers, borrowed browsers and persistent profile data.",
            "inputSchema":{"type":"object","additionalProperties":false,
                "properties":{"taskId":{"type":"string"}},"required":["taskId"]}
        }),
        json!({
            "name":"browser_handoff", "title":"Hand a browser task to a person",
            "description":"Focus this task's current page and return its durable authenticated HTTPS Remote View link. Preserve the task, profile and page for later automation.",
            "inputSchema":{"type":"object","additionalProperties":false,
                "properties":{"taskId":{"type":"string"}},"required":["taskId"]}
        }),
        json!({
            "name":"browser_continue", "title":"Continue a browser task",
            "description":"Resume ordinary automation in this task's current page after human assistance. Return its current identity and URL without navigating or creating another task.",
            "inputSchema":{"type":"object","additionalProperties":false,
                "properties":{"taskId":{"type":"string"}},"required":["taskId"]}
        }),
    ]
}

fn issue(message: impl Into<String>) -> JsonRpcError {
    JsonRpcError::invalid_params(&message.into())
}

/// Use the current executable's existing configuration and startup path.
/// This adapter never constructs a second daemon or browser owner.
fn cli(args: &[&str]) -> Result<Value, JsonRpcError> {
    let executable = std::env::current_exe().map_err(|error| issue(error.to_string()))?;
    let output = Command::new(executable)
        .arg("--json")
        .args(args)
        .output()
        .map_err(|error| issue(format!("workflow_transport_start_failed:{error}")))?;
    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| issue("workflow_cli_response_invalid"))?;
    if !output.status.success() || value.get("success") == Some(&Value::Bool(false)) {
        return Err(issue(format!(
            "workflow_cli_failed:{}",
            value.get("error").unwrap_or(&Value::Null)
        )));
    }
    Ok(value)
}

fn ensure_transport(session: &str) -> Result<(), JsonRpcError> {
    let flags = crate::flags::parse_flags(&["--session".to_string(), session.to_string()]);
    let proxy = flags.proxy.as_deref().map(crate::parse_proxy);
    let keychain_password = std::env::var("AGENT_BROWSER_KEYCHAIN_PASSWORD").ok();
    let use_real_keychain = keychain_password.is_some()
        || std::env::var("AGENT_BROWSER_USE_REAL_KEYCHAIN")
            .is_ok_and(|value| value == "1" || value == "true");
    let options = DaemonOptions {
        // Each managed command selects its exact profile. Startup defaults are not task intent.
        runtime_profile: None,
        profile: None,
        proxy: proxy.as_ref().map(|value| value.server.as_str()),
        proxy_username: proxy.as_ref().and_then(|value| value.username.as_deref()),
        proxy_password: proxy.as_ref().and_then(|value| value.password.as_deref()),
        keychain_password: keychain_password.as_deref(),
        use_real_keychain,
        ..DaemonOptions::from_flags(&flags)
    };
    ensure_daemon(session, &options)
        .map_err(|error| issue(format!("workflow_transport_start_failed:{error}")))?;
    Ok(())
}

pub(super) fn execute(command: Value, session: &str) -> Result<Value, JsonRpcError> {
    // Runtime-host dispatch selects the managed lane before action execution.
    // Keep that transport lane bound to the addressed task, as page tools do.
    let session = command
        .get("sessionName")
        .and_then(Value::as_str)
        .unwrap_or(session)
        .to_string();
    ensure_transport(&session)?;
    let response = send_command(command, &session)
        .map_err(|error| issue(format!("workflow_transport_failed:{error}")))?;
    if !response.success {
        return Err(issue(
            response
                .error
                .unwrap_or_else(|| "workflow_operation_failed".into()),
        ));
    }
    Ok(response.data.unwrap_or(Value::Null))
}

/// An opaque task identity resolves through the existing durable session state.
pub(super) fn task_session(task_id: &str) -> Result<String, JsonRpcError> {
    let path = BrowserSessionSqliteStore::default_sqlite_path().map_err(issue)?;
    let store = BrowserSessionSqliteStore::open_read_only(&path).map_err(issue)?;
    let state = store.load_session_state().map_err(issue)?;
    state
        .sessions
        .get(task_id)
        .map(|task| task.name.clone())
        .ok_or_else(|| issue(format!("browser_task_not_found:{task_id}")))
}

pub(super) fn open(
    arguments: &Value,
    transport_session: &str,
    configured: &ServiceState,
) -> Result<Value, JsonRpcError> {
    let task_key = required_string_argument(arguments, "taskKey")?;
    let url = required_string_argument(arguments, "url")?;
    let mode = optional_string_argument(arguments, "profileMode")?.unwrap_or("persistent");
    let profile = optional_string_argument(arguments, "profile")?;
    if !matches!(mode, "persistent" | "existing" | "disposable") {
        return Err(issue("browser_profile_mode_invalid"));
    }
    if mode == "disposable" && profile.is_some() {
        return Err(issue(
            "disposable_profile_must_not_name_a_persistent_profile",
        ));
    }
    if mode != "disposable" && profile.is_none() {
        return Err(issue("browser_profile_name_required"));
    }
    let service = optional_string_argument(arguments, "serviceName")?.unwrap_or("mcp");
    let agent = optional_string_argument(arguments, "agentName")?.unwrap_or("local-agent");
    let task = optional_string_argument(arguments, "taskName")?.unwrap_or(task_key);
    let session_name = format!(
        "workflow-{}",
        hex::encode(Sha256::digest(
            serde_json::to_vec(&(service, agent, task_key, profile))
                .expect("string tuple serializes")
        ))
    );
    if mode == "persistent" {
        cli(&["runtime", "create", profile.expect("profile validated")])?;
    }
    let mut command = json!({"id":uuid::Uuid::new_v4().to_string(),
        "action":"browser_session_open","sessionName":session_name,
        "url":url,"serviceName":service,"agentName":agent,"taskName":task});
    if let Some(profile_id) = profile {
        let mut state = load_default_service_state_snapshot().map_err(issue)?;
        state.overlay_configured_entities(configured.clone());
        // Older runtime registrations predate the Service catalog join. Reconcile
        // a positively configured profile through the same creation adapter.
        if mode == "existing" && !state.profiles.contains_key(profile_id) {
            let flags = crate::flags::parse_flags(&[]);
            if flags.configured_runtime_profiles.contains_key(profile_id) {
                cli(&["runtime", "create", profile_id])?;
                state = load_default_service_state_snapshot().map_err(issue)?;
                state.overlay_configured_entities(configured.clone());
            }
        }
        let selected = state
            .profiles
            .get(profile_id)
            .ok_or_else(|| issue(format!("browser_profile_not_found:{profile_id}")))?;
        let path = selected
            .user_data_dir
            .as_deref()
            .ok_or_else(|| issue(format!("browser_profile_directory_missing:{profile_id}")))?;
        command["profileId"] = json!(profile_id);
        command["runtimeProfile"] = json!(profile_id);
        command["profile"] = json!(path);
    }
    // Opening verifies/reuses the existing managed browser before reading its task.
    // A retained current tab is live task state; navigation history is not a replay gate.
    let opened = execute(command.clone(), transport_session)?;
    let session_id = opened["sessionId"]
        .as_str()
        .ok_or_else(|| issue("browser_task_identity_missing"))?;
    let path = BrowserSessionSqliteStore::default_sqlite_path().map_err(issue)?;
    let store = BrowserSessionSqliteStore::open_read_only(&path).map_err(issue)?;
    let state = store.load_session_state().map_err(issue)?;
    let current = state
        .sessions
        .get(session_id)
        .ok_or_else(|| issue("browser_task_not_found_after_open"))?;
    let mut data = if let Some(tab_id) = current.current_tab_id.as_deref() {
        if !state.tabs.contains_key(tab_id) {
            return Err(issue("browser_task_current_tab_missing"));
        }
        command["action"] = json!(super::BROWSER_GET_URL_TOOL.action);
        command
            .as_object_mut()
            .expect("command object")
            .remove("url");
        let observed = execute(command, transport_session)?;
        let current_url = observed["url"]
            .as_str()
            .ok_or_else(|| issue("browser_task_current_url_missing"))?;
        json!({"sessionId":session_id,"profileId":current.profile_id,
            "browserId":current.browser_id,"tabId":tab_id,"url":current_url})
    } else {
        command["action"] = json!("browser_session_navigate");
        command["sessionId"] = json!(session_id);
        execute(command, transport_session)?
    };
    data["taskId"] = data["sessionId"].clone();
    if let Some(object) = data.as_object_mut() {
        object.remove("sessionId");
        object.remove("targetId");
        object.remove("visitedAtMs");
    }
    Ok(super::tool_response_from_payload(
        "browser_open",
        transport_session,
        json!({}),
        data,
        false,
    ))
}

pub(super) fn close(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let task_id = required_string_argument(arguments, "taskId")?;
    task_session(task_id)?;
    let data = execute(
        json!({"id":uuid::Uuid::new_v4().to_string(),
        "action":"browser_session_close","sessionId":task_id,"leaveOpen":false}),
        session,
    )?;
    Ok(super::tool_response_from_payload(
        "browser_close",
        session,
        json!({}),
        data,
        false,
    ))
}

/// Continuation is an ordinary addressed page read through the existing owner,
/// not executable transfer, viewer admission, or a separate profile acquisition.
pub(super) fn continue_task(
    arguments: &Value,
    transport_session: &str,
) -> Result<Value, JsonRpcError> {
    let task_id = required_string_argument(arguments, "taskId")?;
    let session_name = task_session(task_id)?;
    let observed = execute(
        json!({"id":uuid::Uuid::new_v4().to_string(),
        "action":"url","sessionName":session_name}),
        transport_session,
    )?;
    let current_url = observed["url"]
        .as_str()
        .ok_or_else(|| issue("browser_task_current_url_missing"))?;
    let path = BrowserSessionSqliteStore::default_sqlite_path().map_err(issue)?;
    let store = BrowserSessionSqliteStore::open_read_only(&path).map_err(issue)?;
    let state = store.load_session_state().map_err(issue)?;
    let task = state
        .sessions
        .get(task_id)
        .ok_or_else(|| issue("browser_task_not_found_after_continue"))?;
    let tab_id = task
        .current_tab_id
        .as_deref()
        .filter(|id| state.tabs.contains_key(*id))
        .ok_or_else(|| issue("browser_task_current_tab_missing"))?;
    Ok(super::tool_response_from_payload(
        "browser_continue",
        transport_session,
        json!({}),
        json!({"taskId":task_id,"profileId":task.profile_id,"browserId":task.browser_id,
            "tabId":tab_id,"url":current_url}),
        false,
    ))
}

/// Complete the owner's durable link using server configuration, never a
/// provider route or a caller-supplied URL. Remote View retains viewer ownership.
pub(super) fn handoff(arguments: &Value, transport_session: &str) -> Result<Value, JsonRpcError> {
    let task_id = required_string_argument(arguments, "taskId")?;
    let configured_origin = std::env::var("AGENT_BROWSER_PUBLIC_ORIGIN").map_err(|_| {
        issue("remote_view_public_origin_missing: configure AGENT_BROWSER_PUBLIC_ORIGIN with this Agent Browser service's authenticated HTTPS origin")
    })?;
    let origin = crate::native::browser_session_remote_view::public_origin(&configured_origin)
        .map_err(issue)?;
    let session_name = task_session(task_id)?;
    // A harmless current-URL read retains the task's handoff through its existing
    // owner. It neither navigates the page nor issues a temporary viewer grant.
    execute(
        json!({"id":uuid::Uuid::new_v4().to_string(),"action":"url",
        "sessionName":session_name}),
        transport_session,
    )?;
    let path = BrowserSessionSqliteStore::default_sqlite_path().map_err(issue)?;
    let store = BrowserSessionSqliteStore::open_read_only(&path).map_err(issue)?;
    let state = store.load_session_state().map_err(issue)?;
    let task = state
        .sessions
        .get(task_id)
        .ok_or_else(|| issue("browser_task_not_found_after_handoff"))?;
    let tab_id = task
        .current_tab_id
        .as_deref()
        .ok_or_else(|| issue("browser_task_current_tab_missing"))?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| issue("remote_view_clock_invalid"))?
        .as_millis() as u64;
    let retained = state
        .remote_view_tab_handoffs
        .values()
        .find(|link| {
            link.session_id == task_id
                && link.tab_id == tab_id
                && link.check_link_at(now_ms).is_ok()
        })
        .ok_or_else(|| issue("remote_view_tab_handoff_missing"))?;
    let presentation = execute(
        json!({"id":uuid::Uuid::new_v4().to_string(),
        "action":"service_remote_view_handoff_resolve","handoffId":retained.id,
        "nativeDesktop":true}),
        transport_session,
    )?;
    if presentation["operatorVisible"]["state"] != "ready" {
        return Err(issue("browser_handoff_presentation_pending"));
    }
    let handoff_url = origin
        .join(&format!("/remote-view/{}", retained.id))
        .map_err(|_| issue("remote_view_handoff_url_invalid"))?;
    Ok(super::tool_response_from_payload(
        "browser_handoff",
        transport_session,
        json!({}),
        json!({"taskId":task_id,"handoffId":retained.id,"handoffUrl":handoff_url.as_str(),
            "operatorVisible":presentation["operatorVisible"]}),
        false,
    ))
}

/// Resolve tab ownership from the native durable state before any selection or
/// close. The close expectation fences an intervening selection at the owner.
/// Replay of new tabs uses the native owner's existing request journal.
pub(super) fn tab(arguments: &Value, transport_session: &str) -> Result<Value, JsonRpcError> {
    let task_id = required_string_argument(arguments, "taskId")?;
    let operation = required_string_argument(arguments, "operation")?;
    let session_name = task_session(task_id)?;
    if operation == "list" {
        execute(
            json!({"id":uuid::Uuid::new_v4().to_string(),
            "action":"url","sessionName":session_name,"observeChildTabs":true}),
            transport_session,
        )?;
    }
    let path = BrowserSessionSqliteStore::default_sqlite_path().map_err(issue)?;
    let store = BrowserSessionSqliteStore::open_read_only(&path).map_err(issue)?;
    let state = store.load_session_state().map_err(issue)?;
    let task = state
        .sessions
        .get(task_id)
        .ok_or_else(|| issue("browser_task_not_found"))?;
    let payload = match operation {
        "list" => json!({"taskId":task_id,"currentTabId":task.current_tab_id,
            "tabs":state.tabs.values().filter(|tab| tab.session_id == task_id)
                .map(|tab| json!({"tabId":tab.id})).collect::<Vec<_>>()}),
        "new" => {
            let key = required_string_argument(arguments, "tabKey")?;
            let request_id = format!("mcp-tab-{:x}", Sha256::digest(format!("{task_id}\0{key}")));
            let mut command = json!({"id":uuid::Uuid::new_v4().to_string(),
                "action":"tab_new","sessionName":session_name,"tabRequestId":request_id});
            if let Some(url) = optional_string_argument(arguments, "url")? {
                command["url"] = json!(url);
            }
            let result = execute(command, transport_session)?;
            json!({"taskId":task_id,"tabId":result["tabId"],"url":result["url"]})
        }
        "select" | "close" => {
            let tab_id = optional_string_argument(arguments, "tabId")?
                .or(task.current_tab_id.as_deref())
                .ok_or_else(|| issue("browser_task_current_tab_missing"))?;
            let tab = state
                .tabs
                .get(tab_id)
                .filter(|tab| tab.session_id == task_id)
                .ok_or_else(|| issue("browser_tab_not_owned"))?;
            execute(
                json!({"id":uuid::Uuid::new_v4().to_string(),
                "action":"browser_session_focus","sessionName":session_name,
                "browserId":task.browser_id,"targetId":tab.target_id}),
                transport_session,
            )?;
            if operation == "close" {
                let result = execute(
                    json!({"id":uuid::Uuid::new_v4().to_string(),
                    "action":"browser_session_tab_close","sessionName":session_name,
                    "sessionId":task_id,"expectedTabId":tab_id}),
                    transport_session,
                )?;
                json!({"taskId":task_id,"closedTabId":result["closedTabId"],
                    "currentTabId":result["currentTabId"]})
            } else {
                json!({"taskId":task_id,"tabId":tab_id})
            }
        }
        _ => return Err(issue("browser_tab_operation_invalid")),
    };
    Ok(super::tool_response_from_payload(
        "browser_tab",
        transport_session,
        json!({}),
        payload,
        false,
    ))
}
