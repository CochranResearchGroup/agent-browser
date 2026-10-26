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
    let desktop = json!({"type":"object","additionalProperties":false,
        "properties":{"desktopId":{"type":"string","minLength":1},
        "generation":{"type":"integer","minimum":1}},"required":["desktopId","generation"]});
    let placement = json!({"oneOf":[
        {"type":"object","additionalProperties":false,"properties":{
            "mode":{"const":"required"},"desktop":desktop},"required":["mode","desktop"]},
        {"type":"object","additionalProperties":false,"properties":{
            "mode":{"const":"preferred"},"desktop":desktop,
            "alternatives":{"type":"array","items":desktop}},
            "required":["mode","desktop","alternatives"]}
    ]});
    vec![
        json!({
            "name": "browser_open", "title": "Open a browser task",
            "description": "Create or reuse a persistent, existing or disposable profile and open one task tab. Reuse taskKey to replay without another tab. Supply serviceName, agentName and a concrete taskName; origin is recorded as agent. Use the returned taskId for subsequent page actions and scoped close.",
            "inputSchema": {"type":"object", "additionalProperties":false,
                "properties": {
                    "profile":{"type":"string","description":"Named profile for persistent or existing mode."},
                    "profileMode":{"type":"string","enum":["persistent","existing","disposable"],"default":"persistent"},
                    "mode":{"type":"string","enum":["automation","manual_login"],"description":"Omit to retain profile mode; new profiles use automation. manual_login has no CDP: use native_capture/native_input; DOM needs automation."},
                    "buildSelectionOperationId":{"type":"string","description":"Bind an earlier build selection."},
                    "taskKey":{"type":"string","description":"Stable task key. Distinct keys receive independent tabs; reuse the key to continue this task."},
                "url":{"type":"string","description":"Initial URL for a new task. Replaying a task returns its current page without navigating again."},
                    "serviceName":{"type":"string"},"agentName":{"type":"string"},"taskName":{"type":"string"},
                    "taskPolicy":{"type":"object","additionalProperties":false,"properties":{"version":{"const":1},"deniedActions":{"type":"array","items":{"type":"string"}},"artifactDestinations":{"type":"object","additionalProperties":{"type":"string"}}},"required":["version"]},
                    "attachment":{"type":"object","additionalProperties":false,"description":"Borrow an exact existing app browser, with no launch or initial navigation. Requires hostBinding and existing profile; omit url.","properties":{
                        "version":{"const":1},"process":{"type":"object","properties":{"pid":{"type":"integer","minimum":1},"startToken":{"type":"string"},"executablePath":{"type":"string"},"browserFamily":{"type":"string"}},"required":["pid","startToken","executablePath"]},
                        "profilePath":{"type":"string"},"cdpEndpoint":{"type":"string"},"targetId":{"type":"string"},
                        "sharedWith":{"type":"array","items":{"type":"object","additionalProperties":false,"properties":{"hostId":{"type":"string"},"participantId":{"type":"string"},"sessionId":{"type":"string"}},"required":["hostId","participantId","sessionId"]}}},"required":["version","process","profilePath","cdpEndpoint","targetId"]},
                    "hostBinding":{"type":"object","additionalProperties":false,"description":"Configured app context and exact required/preferred placement; never a viewer URL.","properties":{"version":{"const":1},"hostId":{"type":"string"},"participantId":{"type":"string"},"sessionId":{"type":"string"},"placement":placement},"required":["version","hostId","participantId","sessionId","placement"]}
                }, "required":["taskKey","serviceName","agentName","taskName"],"oneOf":[{"required":["url"],"not":{"required":["attachment"]}},{"required":["attachment","hostBinding","profile"],"properties":{"mode":{"enum":["automation"]}},"not":{"required":["url"]}}]}
        }),
        json!({
            "name":"browser_close", "title":"Close a browser task",
            "description":"End only the addressed task and its tabs. Preserve peers, borrowed browsers and persistent profile data.",
            "inputSchema":{"type":"object","additionalProperties":false,
                "properties":{"taskId":{"type":"string"},"scope":{"enum":["task","browser"],"default":"task"},"actor":{"type":"string"},"reason":{"type":"string"}},"required":["taskId"]}
        }),
        json!({
            "name":"browser_handoff", "title":"Hand a browser task to a person",
            "description":"Focus this task's current page and return its durable authenticated HTTPS Remote View link. Preserve the task, profile and page for later automation.",
            "inputSchema":{"type":"object","additionalProperties":false,
                "properties":{"taskId":{"type":"string"}},"required":["taskId"]}
        }),
        json!({
            "name":"browser_continue", "title":"Continue a browser task",
            "description":"Read this task's current identity and URL without navigation or another task. After explicit human takeover, use browser_control return before mutating automation; continuation never implicitly returns control.",
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

/// Read the existing owner store and OS only; do not create a transport or attach CDP.
pub(super) fn inspect_mode(task_id: &str) -> Result<Value, String> {
    let path = BrowserSessionSqliteStore::default_sqlite_path()?;
    BrowserSessionSqliteStore::open_read_only(&path)?.inspect_task_mode(task_id)
}

/// Read a compact briefing from the existing owner stores. No transport, live
/// browser observation, credentials, page bodies or human-use acknowledgement.
pub(super) fn inspect_briefing(task_id: &str) -> Result<Value, String> {
    let path = BrowserSessionSqliteStore::default_sqlite_path()?;
    let store = BrowserSessionSqliteStore::open_read_only(&path)?;
    let state = store.load_session_state()?;
    let task = state
        .sessions
        .get(task_id)
        .filter(|task| task.id == task_id)
        .ok_or_else(|| format!("browser_task_not_found:{task_id}"))?;
    let tab = task
        .current_tab_id
        .as_ref()
        .and_then(|id| state.tabs.get(id))
        .filter(|tab| tab.session_id == task_id && tab.browser_id == task.browser_id);
    let metadata = tab.and_then(|tab| state.tab_metadata.get(&tab.id));
    let goal = metadata
        .map(|m| {
            json!({"status":"caller_supplied", "attribution":m.attribution,
        "taskName":m.task_name,"purpose":m.purpose,"serviceName":m.service_name,
        "agentName":m.agent_name,"originKind":m.origin_kind,
        "requestId":m.request_id,"jobId":m.job_id})
        })
        .unwrap_or(json!({"status":"missing"}));
    let last = state
        .navigation_history
        .iter()
        .filter(|row| {
            row.session_id == task_id
                && row.browser_id == task.browser_id
                && row.profile_id == task.profile_id
        })
        .max_by_key(|row| row.visited_at_ms);
    // Navigation records prove a historical navigation, never task completion.
    // Deliberately omit URL/title/body: they can contain tokens or private data.
    let outcome = last
        .map(|row| {
            json!({"status":"historical_verified_navigation",
        "tabId":row.tab_id,"targetId":row.target_id,"observedAtMs":row.visited_at_ms,
        "taskCompletion":"unverified"})
        })
        .unwrap_or(json!({"status":"missing"}));
    let control_path = crate::native::browser_host_control::path(&task.browser_id)?;
    let control_recorded = match std::fs::symlink_metadata(control_path) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => return Err("browser_task_control_read_failed".into()),
    };
    let control = crate::native::browser_host_control::metadata(&task.browser_id)?;
    let original_uncertainty =
        crate::native::browser_host_control::uncertain_operation_references(&task.browser_id)?;
    let mut uncertain = original_uncertainty["operations"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(continuity) = state.hosted_continuity.get(task_id) {
        uncertain.extend(continuity.requests.iter().filter(|(_, row)| row.pending)
            .map(|(id, _)| json!({"source":"hosted_reconnect","operationId":id,"retryAllowed":false})));
    }
    // Legacy tab requests bind a lane name. Only an unambiguous retained lane
    // can be attributed; duplicate names never substitute a peer's operations.
    let unique_lane = state
        .sessions
        .values()
        .filter(|row| row.name == task.name)
        .count()
        == 1;
    if unique_lane {
        uncertain.extend(
            state
                .managed_tab_requests
                .iter()
                .filter(|(_, row)| {
                    row.session_name == task.name
                        && row.response.is_none()
                        && row
                            .profile_id
                            .as_ref()
                            .is_none_or(|id| id == &task.profile_id)
                })
                .map(
                    |(id, _)| json!({"source":"tab_request","operationId":id,"retryAllowed":false}),
                ),
        );
    }
    let uncertainty_truncated = uncertain.len() > 32 || original_uncertainty["truncated"] == true;
    uncertain.truncate(32);
    let assistance = tab
        .and_then(|tab| state.pending_human_tabs.get(&tab.id))
        .map(|hold| {
            json!({"status":"retained_pending_human","handoffId":hold.handoff_id,
            "requestedAtMs":hold.requested_at_ms,"expiresAtMs":hold.expires_at_ms,
            "humanUse":"unverified"})
        })
        .unwrap_or(json!({"status":"none_recorded"}));
    let after = store.load_session_state()?;
    if after != state {
        return Err("browser_task_briefing_changed_retry_read".into());
    }
    Ok(
        json!({"schemaVersion":"agent-browser.task-briefing.v1","taskId":task_id,
        "profileId":task.profile_id,"browserId":task.browser_id,
        "tabId":tab.map(|tab| &tab.id),"targetId":tab.map(|tab| &tab.target_id),
        "contextEvidence":"retained_not_live","currentTargetState":if tab.is_some(){"retained_not_live"}else{"missing_or_conflicting"},
        "lastActivityAtMs":task.last_activity_at_ms,"goal":goal,"lastVerifiedOutcome":outcome,
        "artifacts":{"status":if state.task_artifact_references.contains_key(task_id){"recorded_references_not_revalidated"}else{"not_recorded"},
            "references":state.task_artifact_references.get(task_id)},
        "assistance":assistance,"control":{"epoch":control["epoch"],"owner":if control_recorded {control["owner"].clone()} else {json!("unverified")},
            "pauseReads":control["pauseReads"],"cooperativeOnly":true},
        "uncertainOperations":uncertain,"uncertaintyTruncated":uncertainty_truncated,
        "legacyRequestAttribution":if unique_lane {"exact_retained_lane"} else {"ambiguous_excluded"},
        "browserEffects":"none","liveObservation":"not_performed"}),
    )
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

/// Join opaque task identity to its transport lane without name-based selection.
pub(super) fn address_task_command(command: &mut Value, task_id: &str) -> Result<String, String> {
    let path = BrowserSessionSqliteStore::default_sqlite_path()?;
    let store = BrowserSessionSqliteStore::open_read_only(&path)?;
    let state = store.load_session_state()?;
    let task = state
        .sessions
        .get(task_id)
        .ok_or_else(|| format!("browser_task_not_found:{task_id}"))?;
    command["sessionId"] = json!(task.id);
    command["sessionName"] = json!(task.name);
    command["profileId"] = json!(task.profile_id);
    Ok(task.name.clone())
}

/// Read posture and positive copy publication without creating a runtime profile.
fn profile_catalog_for_open(
    id: &str,
) -> Result<
    (
        Option<agent_browser_service_model::BrowserProfileCatalogEntry>,
        Option<agent_browser_service_model::BrowserProfileCatalogEntry>,
    ),
    JsonRpcError,
> {
    let path = BrowserSessionSqliteStore::default_sqlite_path().map_err(issue)?;
    match std::fs::metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((None, None)),
        Err(_) => return Err(issue("browser_profile_catalog_unavailable")),
        Ok(_) => {}
    }
    let store = BrowserSessionSqliteStore::open_read_only(&path).map_err(issue)?;
    let entry = store
        .load_profile_catalog()
        .map_err(issue)?
        .catalog
        .profiles
        .get(id)
        .cloned();
    let copied = store.published_copy_profile(id).map_err(issue)?;
    Ok((entry, copied))
}

pub(super) fn open(
    arguments: &Value,
    transport_session: &str,
    configured: &ServiceState,
) -> Result<Value, JsonRpcError> {
    if arguments.get("attachment").is_some() {
        if optional_string_argument(arguments, "mode")?.unwrap_or("automation") != "automation" {
            return Err(issue("manual_login_requires_owned_browser: attachment cannot remove an existing CDP listener"));
        }
        return open_attachment(arguments, transport_session);
    }
    let task_key = required_string_argument(arguments, "taskKey")?;
    let url = required_string_argument(arguments, "url")?;
    let mode = optional_string_argument(arguments, "profileMode")?.unwrap_or("persistent");
    let profile = optional_string_argument(arguments, "profile")?;
    let explicit_control_mode = optional_string_argument(arguments, "mode")?;
    let control_mode = explicit_control_mode.unwrap_or("automation");
    if !matches!(control_mode, "automation" | "manual_login") {
        return Err(issue("browser_login_mode_invalid"));
    }
    let mut manual = control_mode == "manual_login";
    if manual && mode == "disposable" {
        return Err(issue("manual_login_requires_persistent_profile"));
    }
    if manual && !crate::native::browser_session_remote_view::remote_view_settings_present() {
        return Err(issue("manual_login_requires_native_remote_desktop"));
    }

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
    let service = required_string_argument(arguments, "serviceName")?;
    let agent = required_string_argument(arguments, "agentName")?;
    let task = required_string_argument(arguments, "taskName")?;
    let (catalog_profile, copied_profile) = profile
        .map(profile_catalog_for_open)
        .transpose()?
        .unwrap_or((None, None));
    if explicit_control_mode.is_none() {
        manual = catalog_profile
            .as_ref()
            .is_some_and(|profile| profile.manual_login_url.is_some());
    }
    if manual && mode == "disposable" {
        return Err(issue("manual_login_requires_persistent_profile"));
    }
    if manual && !crate::native::browser_session_remote_view::remote_view_settings_present() {
        return Err(issue("manual_login_requires_native_remote_desktop"));
    }
    let hosted = arguments
        .get("hostBinding")
        .map(|value| {
            serde_json::from_value::<agent_browser_service_model::BrowserHostBinding>(value.clone())
                .map_err(|_| issue("browser_host_binding_invalid"))
        })
        .transpose()?;
    if let Some(binding) = &hosted {
        crate::native::browser_host_context::validate_provider(binding).map_err(issue)?;
    }
    if manual && hosted.is_none() {
        let origin = std::env::var("AGENT_BROWSER_PUBLIC_ORIGIN")
            .map_err(|_| issue("remote_view_public_origin_missing"))?;
        crate::native::browser_session_remote_view::public_origin(&origin).map_err(issue)?;
    }
    let identity = if let Some(binding) = &hosted {
        serde_json::to_vec(&(
            service,
            agent,
            task_key,
            profile,
            &binding.host_id,
            &binding.participant_id,
            &binding.session_id,
        ))
    } else {
        // Preserve existing standalone task keys byte-for-byte.
        serde_json::to_vec(&(service, agent, task_key, profile))
    }
    .expect("string tuple serializes");
    let session_name = format!("workflow-{}", hex::encode(Sha256::digest(identity)));
    if mode == "persistent" && copied_profile.is_none() {
        cli(&["runtime", "create", profile.expect("profile validated")])?;
    }
    let mut command = json!({"id":uuid::Uuid::new_v4().to_string(),
        "action":"browser_session_open","sessionName":session_name,
        "url":url,"serviceName":service,"agentName":agent,"taskName":task,"originKind":"agent"});
    command["manualLogin"] = json!(manual);
    if let Some(receipt) = optional_string_argument(arguments, "buildSelectionOperationId")? {
        command["buildSelectionOperationId"] = json!(receipt);
    }
    if let Some(binding) = &hosted {
        command["hostBinding"] = json!(binding);
    }
    if let Some(policy) = arguments.get("taskPolicy") {
        command["taskPolicy"] = policy.clone();
    }
    if let Some(profile_id) = profile {
        let mut state = load_default_service_state_snapshot().map_err(issue)?;
        state.overlay_configured_entities(configured.clone());
        if let Some(copied) = &copied_profile {
            if state
                .profiles
                .get(profile_id)
                .and_then(|profile| profile.user_data_dir.as_deref())
                .is_some_and(|path| {
                    std::fs::canonicalize(path).ok().as_deref()
                        != Some(std::path::Path::new(&copied.user_data_dir))
                })
            {
                return Err(issue("browser_profile_selector_conflict"));
            }
            command["profileId"] = json!(profile_id);
            command["profile"] = json!(copied.user_data_dir);
        } else {
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
        command["sessionId"] = json!(session_id);
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
    if hosted.is_some() {
        // The read-only projection separates recorded inventory from Native readiness.
        // Initial navigation may have created the first tab since the open read.
        let published = store.load_session_state().map_err(issue)?;
        data["host"] =
            crate::native::browser_host_context::snapshot(&published, session_id).map_err(issue)?;
    }
    data["taskId"] = data["sessionId"].clone();
    if let Some(object) = data.as_object_mut() {
        object.remove("sessionId");
        object.remove("targetId");
        object.remove("visitedAtMs");
    }
    if manual {
        let handoff = handoff(&json!({"taskId":data["taskId"]}), transport_session)?;
        let text = handoff["content"][0]["text"]
            .as_str()
            .ok_or_else(|| issue("browser_handoff_payload_missing"))?;
        let payload: Value =
            serde_json::from_str(text).map_err(|_| issue("browser_handoff_payload_invalid"))?;
        data["handoffUrl"] = payload["data"]["handoffUrl"].clone();
        data["operatorVisible"] = payload["data"]["operatorVisible"].clone();
        data["mode"] = json!("manual_login");
        data["urlObserved"] = json!(false);
        data["authentication"] = json!("not_verified");
    }
    Ok(super::tool_response_from_payload(
        "browser_open",
        transport_session,
        json!({}),
        data,
        false,
    ))
}

/// Attach retains the existing app process and target without initial navigation.
fn open_attachment(arguments: &Value, transport_session: &str) -> Result<Value, JsonRpcError> {
    let task_key = required_string_argument(arguments, "taskKey")?;
    let profile = required_string_argument(arguments, "profile")?;
    let service = required_string_argument(arguments, "serviceName")?;
    let agent = required_string_argument(arguments, "agentName")?;
    let task = required_string_argument(arguments, "taskName")?;
    if arguments.get("url").is_some()
        || optional_string_argument(arguments, "profileMode")?
            .is_some_and(|mode| mode != "existing")
    {
        return Err(issue("browser_attachment_does_not_navigate_or_allocate_profile: omit url and use existing mode"));
    }
    let binding: agent_browser_service_model::BrowserHostBinding = serde_json::from_value(
        arguments
            .get("hostBinding")
            .cloned()
            .ok_or_else(|| issue("browser_attachment_host_binding_required"))?,
    )
    .map_err(|_| issue("browser_host_binding_invalid"))?;
    crate::native::browser_host_context::validate_provider(&binding).map_err(issue)?;
    let attachment: agent_browser_service_model::BrowserAttachmentRequest =
        serde_json::from_value(arguments["attachment"].clone())
            .map_err(|_| issue("browser_attachment_invalid"))?;
    attachment.validate().map_err(issue)?;
    let identity = serde_json::to_vec(&(
        service,
        agent,
        task_key,
        Some(profile),
        &binding.host_id,
        &binding.participant_id,
        &binding.session_id,
    ))
    .expect("string tuple");
    let session_name = format!("workflow-{}", hex::encode(Sha256::digest(identity)));
    let mut command = json!({"id":uuid::Uuid::new_v4().to_string(),"action":"browser_session_open","sessionName":session_name,
        "profileId":profile,"attachment":attachment,"hostBinding":binding,
        "serviceName":service,"agentName":agent,"taskName":task,"originKind":"agent"});
    if let Some(policy) = arguments.get("taskPolicy") {
        command["taskPolicy"] = policy.clone();
    }
    let opened = execute(command, transport_session)?;
    let task_id = opened["sessionId"]
        .as_str()
        .ok_or_else(|| issue("browser_task_identity_missing"))?;
    let session = task_session(task_id)?;
    let observed = execute(
        json!({"id":uuid::Uuid::new_v4().to_string(),"action":"url","sessionName":session,"sessionId":task_id}),
        transport_session,
    )?;
    let store = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path().map_err(issue)?,
    )
    .map_err(issue)?;
    let state = store.load_session_state().map_err(issue)?;
    let host = crate::native::browser_host_context::snapshot(&state, task_id).map_err(issue)?;
    Ok(super::tool_response_from_payload(
        "browser_open",
        transport_session,
        json!({}),
        json!({"taskId":task_id,
        "profileId":profile,"browserId":host["browserId"],"tabId":host["tabId"],"url":observed["url"],"host":host}),
        false,
    ))
}

pub(super) fn forward_policy_override(
    arguments: &Value,
    command: &mut Value,
) -> Result<(), JsonRpcError> {
    if let Some(token) = optional_string_argument(arguments, "policyOverrideId")? {
        command["policyOverrideId"] = json!(token);
        command["id"] = json!(required_string_argument(arguments, "requestId")?);
    }
    Ok(())
}

pub(super) fn close(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let task_id = required_string_argument(arguments, "taskId")?;
    task_session(task_id)?;
    let mut command = json!({"id":uuid::Uuid::new_v4().to_string(),
        "action":"browser_session_close","sessionId":task_id,"leaveOpen":false});
    for field in ["scope", "actor", "reason"] {
        if let Some(value) = arguments.get(field) {
            command[field] = value.clone();
        }
    }
    forward_policy_override(arguments, &mut command)?;
    let data = execute(command, session)?;
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
        "action":"url","sessionName":session_name,"sessionId":task_id}),
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
            "tabId":tab_id,"url":current_url,"urlObserved":observed.get("urlObserved").cloned().unwrap_or(json!(true)),
            "mode":observed.get("mode").cloned().unwrap_or(json!("automation"))}),
        false,
    ))
}

/// Complete the owner's durable link using server configuration, never a
/// provider route or a caller-supplied URL. Remote View retains viewer ownership.
pub(super) fn handoff(arguments: &Value, transport_session: &str) -> Result<Value, JsonRpcError> {
    let task_id = required_string_argument(arguments, "taskId")?;
    let path = BrowserSessionSqliteStore::default_sqlite_path().map_err(issue)?;
    let store = BrowserSessionSqliteStore::open_read_only(&path).map_err(issue)?;
    let before = store.load_session_state().map_err(issue)?;
    let hosted = before.host_bindings.get(task_id).cloned();
    let origin = if let Some(binding) = &hosted {
        crate::native::browser_host_context::validate_provider(&binding.request).map_err(issue)?;
        None
    } else {
        let configured_origin = std::env::var("AGENT_BROWSER_PUBLIC_ORIGIN").map_err(|_| {
            issue("remote_view_public_origin_missing: configure AGENT_BROWSER_PUBLIC_ORIGIN with this Agent Browser service's authenticated HTTPS origin")
        })?;
        Some(
            crate::native::browser_session_remote_view::public_origin(&configured_origin)
                .map_err(issue)?,
        )
    };
    let session_name = task_session(task_id)?;
    // A harmless current-URL read retains the task's handoff through its existing
    // owner. It neither navigates the page nor issues a temporary viewer grant.
    execute(
        json!({"id":uuid::Uuid::new_v4().to_string(),"action":"url",
        "sessionName":session_name,"sessionId":task_id}),
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
    let handoff_url = if let Some(binding) = &hosted {
        if state.host_bindings.get(task_id) != Some(binding) {
            return Err(issue("browser_host_binding_changed"));
        }
        crate::native::browser_host_context::handoff_url(&binding.request, &retained.id)
            .map_err(issue)?
    } else {
        origin
            .expect("standalone origin checked")
            .join(&format!("/remote-view/{}", retained.id))
            .map_err(|_| issue("remote_view_handoff_url_invalid"))?
            .to_string()
    };
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
        let initial = BrowserSessionSqliteStore::open_read_only(
            &BrowserSessionSqliteStore::default_sqlite_path().map_err(issue)?,
        )
        .map_err(issue)?
        .load_session_state()
        .map_err(issue)?;
        crate::native::browser_host_policy::check_state(
            &initial,
            &session_name,
            &json!({"taskId":task_id,"action":"tab_list"}),
        )
        .map_err(issue)?;
        execute(
            json!({"id":uuid::Uuid::new_v4().to_string(),
            "action":"url","sessionName":session_name,"sessionId":task_id,"observeChildTabs":true}),
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
            command["sessionId"] = json!(task_id);
            forward_policy_override(arguments, &mut command)?;
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
                let mut command = json!({"id":uuid::Uuid::new_v4().to_string(),
                    "action":"browser_session_tab_close","sessionName":session_name,
                    "sessionId":task_id,"expectedTabId":tab_id});
                forward_policy_override(arguments, &mut command)?;
                let result = execute(command, transport_session)?;
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

#[cfg(test)]
mod context_tests {
    use super::*;

    #[test]
    fn manual_login_rejects_cdp_attachment_before_profile_or_transport_effects() {
        let error = open(
            &json!({"mode":"manual_login", "attachment":{}}),
            "unused",
            &ServiceState::default(),
        )
        .unwrap_err();
        assert!(error.data.as_ref().unwrap()["message"]
            .as_str()
            .unwrap()
            .starts_with("manual_login_requires_owned_browser"));
    }

    #[test]
    fn manual_login_mode_is_exposed_without_internal_launch_plumbing() {
        let tools = tools();
        let open = tools
            .iter()
            .find(|tool| tool["name"] == "browser_open")
            .unwrap();
        assert_eq!(
            open["inputSchema"]["properties"]["mode"]["enum"],
            json!(["automation", "manual_login"])
        );
        assert!(open["inputSchema"]["properties"]
            .get("browserBuild")
            .is_none());
    }

    #[test]
    fn hosted_input_rejects_invalid_context_before_profile_or_transport_effects() {
        let complete = json!({"taskKey":"hosted","url":"data:text/html,synthetic",
            "profile":"never-created","serviceName":"fixture","agentName":"fixture",
            "taskName":"Validate placement","hostBinding":{"version":1,"hostId":"app",
            "participantId":"bot","sessionId":"session","placement":{"mode":"required",
            "desktop":{"desktopId":"desktop","generation":1}}}});
        for invalid in [
            json!(null),
            json!({"version":1,"hostId":"app","participantId":"bot","sessionId":"session",
                "placement":{"mode":"required","desktop":{"desktopId":"desktop","generation":1}},
                "handoffUrl":"https://arbitrary.example/"}),
            json!({"version":2,"hostId":"app","participantId":"bot","sessionId":"session",
                "placement":{"mode":"required","desktop":{"desktopId":"desktop","generation":1}}}),
        ] {
            let mut input = complete.clone();
            input["hostBinding"] = invalid;
            let error = open(&input, "unused", &ServiceState::default()).unwrap_err();
            assert!(error.data.as_ref().unwrap()["message"]
                .as_str()
                .unwrap()
                .contains("browser_host_binding"));
        }
    }

    #[test]
    fn handoff_creation_context_mcp_rejects_before_profile_or_transport_effects() {
        let complete = json!({"taskKey":"example","url":"https://example.test/",
            "profile":"nonexistent-test-profile","serviceName":"codex",
            "agentName":"Codex","taskName":"Open accounting"});
        for field in ["serviceName", "agentName", "taskName"] {
            let mut missing = complete.clone();
            missing.as_object_mut().unwrap().remove(field);
            let error = open(&missing, "unused", &ServiceState::default()).unwrap_err();
            assert!(error.data.as_ref().unwrap()["message"]
                .as_str()
                .unwrap()
                .contains(field));
            missing[field] = json!("   ");
            assert!(open(&missing, "unused", &ServiceState::default()).is_err());
            let schema = tools().remove(0);
            assert!(schema["inputSchema"]["required"]
                .as_array()
                .unwrap()
                .contains(&json!(field)));
        }
    }
}
