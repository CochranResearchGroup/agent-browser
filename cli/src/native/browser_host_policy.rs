//! Scoped app/task restrictions over existing authenticated dispatch authority.
//! This is cooperative operation policy, not interception of arbitrary OS/CDP input.
use super::browser_session_store::BrowserSessionSqliteStore;
use agent_browser_service_model::*;
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};

fn task_ids(
    state: &BrowserSessionState,
    name: &str,
    command: &Value,
) -> Result<Vec<String>, String> {
    if let Some(id) = command["taskId"]
        .as_str()
        .or_else(|| command["sessionId"].as_str())
    {
        return state
            .sessions
            .get(id)
            .map(|_| vec![id.into()])
            .ok_or_else(|| "browser_policy_task_missing".into());
    }
    let name = command["sessionName"].as_str().unwrap_or(name);
    if !name.is_empty() {
        let found = state
            .sessions
            .values()
            .filter(|task| task.name == name || task.id == name)
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        if found.len() > 1 {
            return Err("browser_policy_task_ambiguous".into());
        }
        if !found.is_empty() {
            return Ok(found);
        }
    }
    let selected = state
        .sessions
        .values()
        .filter(|task| {
            command["browserId"].as_str() == Some(task.browser_id.as_str())
                || command["profileId"].as_str() == Some(task.profile_id.as_str())
                || command["runtimeProfile"].as_str() == Some(task.profile_id.as_str())
                || state.tabs.values().any(|tab| {
                    tab.session_id == task.id
                        && (command["tabId"].as_str() == Some(tab.id.as_str())
                            || command["targetId"].as_str() == Some(tab.target_id.as_str()))
                })
        })
        .map(|task| task.id.clone())
        .collect();
    Ok(selected)
}

/// Pure admission against an exact current state snapshot. No rules means no ceremony.
pub(crate) fn check_state(
    state: &BrowserSessionState,
    name: &str,
    command: &Value,
) -> Result<(), String> {
    if !state.host_policies.has_rules() {
        return Ok(());
    }
    let action = command["action"].as_str().unwrap_or_default();
    if policy_status_action(action) {
        return Ok(());
    }
    for id in task_ids(state, name, command)? {
        let Some(binding) = state.host_bindings.get(&id) else {
            continue;
        };
        state.host_policies.check(
            &binding.request.host_id,
            &id,
            action,
            command["id"].as_str().unwrap_or_default(),
            command["policyOverrideId"].as_str(),
        )?;
    }
    Ok(())
}

fn artifact_action(action: &str) -> bool {
    matches!(
        canonical_browser_policy_action(action),
        "screenshot"
            | "download"
            | "waitfordownload"
            | "pdf"
            | "desktop_capture"
            | "state_save"
            | "trace_stop"
            | "profiler_stop"
            | "recording_start"
            | "recording_restart"
    )
}

/// Resolve an authorized handle to one new local file, never a publication URL.
/// Existing leaf paths and symlinked parents are refused before capture/download.
pub(crate) fn artifact_path(
    policy: &BrowserTaskPolicy,
    handle: &str,
    name: &str,
) -> Result<PathBuf, String> {
    let root = policy
        .artifact_destinations
        .get(handle)
        .ok_or("browser_artifact_handle_not_authorized")?;
    let relative = Path::new(name);
    if name.is_empty()
        || name.len() > 512
        || name.chars().any(char::is_control)
        || relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("browser_artifact_name_invalid".into());
    }
    let root = Path::new(root);
    let canonical = root
        .canonicalize()
        .map_err(|_| "browser_artifact_destination_unavailable")?;
    if canonical != root {
        return Err("browser_artifact_destination_not_canonical".into());
    }
    let mut destination = root.to_path_buf();
    let parts = relative.components().collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        destination.push(part.as_os_str());
        match std::fs::symlink_metadata(&destination) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink()
                    || index + 1 == parts.len()
                    || !metadata.is_dir()
                {
                    return Err("browser_artifact_destination_conflict".into());
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound && index + 1 == parts.len() => {}
            Err(_) => return Err("browser_artifact_parent_unavailable".into()),
        }
    }
    Ok(destination)
}

/// Canonical preparation happens before queuing or page dispatch on all routes.
pub(crate) fn prepare_command(
    state: &BrowserSessionState,
    name: &str,
    command: &mut Value,
) -> Result<(), String> {
    check_state(state, name, command)?;
    if !artifact_action(command["action"].as_str().unwrap_or_default()) {
        return Ok(());
    }
    let ids = task_ids(state, name, command)?;
    let policies = ids
        .iter()
        .filter_map(|id| {
            state
                .host_bindings
                .get(id)
                .and_then(|binding| state.host_policies.effective(&binding.request.host_id, id))
        })
        .filter(|policy| !policy.artifact_destinations.is_empty())
        .collect::<Vec<_>>();
    let explicit = command.get("artifactHandle").is_some();
    if policies.is_empty() && !explicit {
        return Ok(());
    }
    if !matches!(
        canonical_browser_policy_action(command["action"].as_str().unwrap_or_default()),
        "screenshot" | "download" | "waitfordownload" | "pdf"
    ) {
        return Err("browser_artifact_route_unsupported: use authorized screenshot/download handles or explicitly change task policy".into());
    }
    if ids.len() != 1 {
        return Err("browser_artifact_exact_task_required".into());
    }
    let id = &ids[0];
    let binding = state
        .host_bindings
        .get(id)
        .ok_or("browser_artifact_hosted_task_required")?;
    let policy = state
        .host_policies
        .effective(&binding.request.host_id, id)
        .ok_or("browser_artifact_handle_not_authorized")?;
    let handle = command["artifactHandle"]
        .as_str()
        .ok_or("browser_artifact_handle_required")?;
    let name = command["artifactName"]
        .as_str()
        .ok_or("browser_artifact_name_required")?;
    let destination = artifact_path(policy, handle, name)?;
    let path = destination
        .to_str()
        .ok_or("browser_artifact_name_invalid")?;
    if command["path"]
        .as_str()
        .is_some_and(|provided| provided != path)
    {
        return Err("browser_artifact_path_conflict".into());
    }
    command["path"] = json!(path);
    Ok(())
}

/// Explicit broader close is admitted only for the addressed browser with no peer
/// tasks and no borrowed process. Attachment alone never grants termination.
pub(crate) fn broader_close_plan(
    state: &BrowserSessionState,
    task_id: &str,
) -> Result<String, String> {
    let task = state
        .sessions
        .get(task_id)
        .ok_or("browser_policy_task_missing")?;
    if state.app_browser_attachments.contains_key(&task.browser_id)
        || state.shared_browser_ids.contains(&task.browser_id)
    {
        return Err("browser_lifecycle_borrowed_process_preserved: detach the task; broader app-owned termination requires its native owner".into());
    }
    if state
        .sessions
        .values()
        .any(|peer| peer.browser_id == task.browser_id && peer.id != task_id)
    {
        return Err(
            "browser_lifecycle_peer_scope_conflict: close only this task's created tabs".into(),
        );
    }
    Ok(task.browser_id.clone())
}

pub(crate) fn apply(state: &mut BrowserSessionState, command: &Value) -> Result<Value, String> {
    let task_id = command["taskId"]
        .as_str()
        .ok_or("browser_policy_task_required")?;
    let binding = state
        .host_bindings
        .get(task_id)
        .ok_or("browser_policy_hosted_task_required")?;
    if !state.sessions.contains_key(task_id) {
        return Err("browser_policy_task_missing".into());
    }
    let host_id = binding.request.host_id.clone();
    let operation = command["operation"].as_str().unwrap_or("status");
    let mut token = None;
    if operation != "status" {
        let expected = command["expectedRevision"]
            .as_u64()
            .ok_or("browser_policy_revision_required")?;
        let actor = command["actor"]
            .as_str()
            .ok_or("browser_policy_attribution_required")?;
        let reason = command["reason"]
            .as_str()
            .ok_or("browser_policy_attribution_required")?;
        if operation == "override" {
            let id = uuid::Uuid::new_v4().to_string();
            state.host_policies.issue_override(
                expected,
                id.clone(),
                &host_id,
                task_id,
                command["overrideAction"]
                    .as_str()
                    .ok_or("browser_policy_override_action_required")?,
                command["requestId"]
                    .as_str()
                    .ok_or("browser_policy_override_request_required")?,
                actor,
                reason,
            )?;
            token = Some(id);
        } else {
            let policy = command
                .get("policy")
                .map(|policy| {
                    serde_json::from_value(policy.clone()).map_err(|_| "browser_policy_invalid")
                })
                .transpose()?;
            state.host_policies.update(
                expected, &host_id, task_id, operation, policy, actor, reason,
            )?;
        }
    }
    Ok(
        json!({"taskId":task_id,"hostId":host_id,"revision":state.host_policies.revision,"policy":state.host_policies.effective(&host_id,task_id),"overrideId":token,"lastChange":state.host_policies.audit.last()}),
    )
}

/// Mutable boundary used immediately before actual dispatch on supported routes.
/// This rereads policy instead of relying on the calling daemon's cached host.
pub(crate) fn prepare_dispatch(
    name: Option<&str>,
    pid: Option<u32>,
    command: &mut Value,
) -> Result<(), String> {
    let Some(state) = BrowserSessionSqliteStore::default_session_state_read_only()? else {
        return Ok(());
    };
    if command["browserId"].is_null() {
        if let Some(browser) =
            pid.and_then(|pid| state.browsers.values().find(|browser| browser.pid == pid))
        {
            command["browserId"] = json!(browser.id);
        }
    }
    prepare_command(&state, name.unwrap_or_default(), command)
}

/// Execute existing managed close only after exact peer/custody admission, then
/// independently read the OS again. A failed readback is retained, never replayed.
pub(crate) fn close_browser_scope(
    state: &BrowserSessionState,
    task_id: &str,
    command: &Value,
    close: impl FnOnce() -> Result<Value, String>,
) -> Result<Value, String> {
    close_browser_scope_with_observation(
        state,
        task_id,
        command,
        crate::process_identity::observe_process,
        close,
    )
}
fn close_browser_scope_with_observation(
    state: &BrowserSessionState,
    task_id: &str,
    command: &Value,
    mut observe: impl FnMut(u32) -> crate::process_identity::ProcessObservation,
    close: impl FnOnce() -> Result<Value, String>,
) -> Result<Value, String> {
    let browser_id = broader_close_plan(state, task_id)?;
    let actor = command["actor"]
        .as_str()
        .filter(|value| {
            !value.trim().is_empty() && value.len() <= 160 && !value.chars().any(char::is_control)
        })
        .ok_or("browser_lifecycle_attribution_required")?;
    let reason = command["reason"]
        .as_str()
        .filter(|value| {
            !value.trim().is_empty() && value.len() <= 320 && !value.chars().any(char::is_control)
        })
        .ok_or("browser_lifecycle_attribution_required")?;
    let mut addressed = command.clone();
    addressed["taskId"] = json!(task_id);
    addressed["action"] = json!("close");
    check_state(state, "", &addressed)?;
    let browser = state
        .browsers
        .get(&browser_id)
        .ok_or("browser_lifecycle_browser_missing")?;
    use crate::process_identity::ProcessObservation;
    let before = match observe(browser.pid) {
        ProcessObservation::Missing => None,
        ProcessObservation::Observed(identity)
            if identity
                .start_token
                .as_ref()
                .is_some_and(|value| !value.is_empty())
                && identity.executable_path.is_some() =>
        {
            Some(identity)
        }
        _ => return Err("browser_lifecycle_process_observation_unavailable".into()),
    };
    let result = close()?;
    let absent=match observe(browser.pid) {
        ProcessObservation::Missing=>true,
        ProcessObservation::Observed(after)=>before.as_ref().is_some_and(|before|before.start_token!=after.start_token),
        ProcessObservation::Failed{..}=>return Err("browser_lifecycle_residue_readback_unavailable: close outcome must be inspected; do not replay".into()),
    };
    if !absent {
        return Err("browser_lifecycle_owned_process_residue: inspect the original close outcome; do not replay".into());
    }
    Ok(
        json!({"taskId":task_id,"browserId":browser_id,"scope":"browser","actor":actor,"reason":reason,"result":result,"residue":{"originalProcess":"absent","source":"fresh_os_observation","pid":browser.pid},"peerResources":"preserved"}),
    )
}

/// Apply reviewed app defaults to a new hosted open before launch/attachment.
/// Explicitly cleared app defaults are never silently imported again.
pub(crate) fn check_open(
    state: &BrowserSessionState,
    binding: &BrowserHostBinding,
    configured_defaults: Option<&BrowserTaskPolicy>,
    command: &Value,
) -> Result<(), String> {
    let existing = state
        .sessions
        .values()
        .find(|task| command["sessionName"].as_str() == Some(task.name.as_str()));
    let task_id = existing.map(|task| task.id.as_str()).unwrap_or("");
    if let Some(explicit) = command.get("taskPolicy") {
        let policy: BrowserTaskPolicy =
            serde_json::from_value(explicit.clone()).map_err(|_| "browser_policy_invalid")?;
        policy.validate()?;
        let mut policies = BrowserHostPolicies::default();
        policies.task_overrides.insert(task_id.into(), policy);
        return policies.check(
            &binding.host_id,
            task_id,
            command["action"].as_str().unwrap_or("browser_session_open"),
            command["id"].as_str().unwrap_or_default(),
            None,
        );
    }
    if state
        .host_policies
        .effective(&binding.host_id, task_id)
        .is_some()
        || state
            .host_policies
            .initialized_hosts
            .contains(&binding.host_id)
    {
        return state.host_policies.check(
            &binding.host_id,
            task_id,
            command["action"].as_str().unwrap_or("browser_session_open"),
            command["id"].as_str().unwrap_or_default(),
            command["policyOverrideId"].as_str(),
        );
    }
    let Some(policy) = configured_defaults else {
        return Ok(());
    };
    policy.validate()?;
    let mut policies = BrowserHostPolicies::default();
    policies
        .app_defaults
        .insert(binding.host_id.clone(), policy.clone());
    policies.check(
        &binding.host_id,
        task_id,
        command["action"].as_str().unwrap_or("browser_session_open"),
        command["id"].as_str().unwrap_or_default(),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet, VecDeque};
    fn binding() -> BrowserHostBinding {
        BrowserHostBinding {
            version: 1,
            host_id: "app".into(),
            participant_id: "participant".into(),
            session_id: "app-session".into(),
            placement: BrowserPlacement::Required {
                desktop: BrowserDesktopBinding {
                    desktop_id: "desktop".into(),
                    generation: 1,
                },
            },
        }
    }
    fn fixture() -> BrowserSessionState {
        let mut state = BrowserSessionState::default();
        state.sessions.insert(
            "alpha".into(),
            ManagedBrowserSession {
                id: "alpha".into(),
                name: "task-alpha".into(),
                profile_id: "profile".into(),
                browser_id: "browser".into(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
                expires_at_ms: 100,
                current_tab_id: None,
            },
        );
        state.browsers.insert(
            "browser".into(),
            ManagedBrowserInstance {
                id: "browser".into(),
                profile_id: "profile".into(),
                pid: 42,
                cdp_endpoint: "ws://synthetic".into(),
                desktop: None,
                active_session_ids: vec!["alpha".into()],
            },
        );
        state.host_bindings.insert(
            "alpha".into(),
            BrowserHostTaskBinding {
                request: binding(),
                desktop: RemoteViewFixedDesktop {
                    desktop_id: "desktop".into(),
                    generation: 1,
                    friendly_route_label: "App".into(),
                },
            },
        );
        state
    }
    #[test]
    fn initial_app_defaults_gate_before_launch_and_explicit_clear_is_retained() {
        let mut state = fixture();
        state.sessions.clear();
        state.host_bindings.clear();
        state.browsers.clear();
        let defaults = BrowserTaskPolicy {
            denied_actions: BTreeSet::from(["launch".into()]),
            ..Default::default()
        };
        assert!(check_open(
            &state,
            &binding(),
            Some(&defaults),
            &json!({"action":"browser_session_open","sessionName":"new-task"})
        )
        .is_err());
        state.host_policies.initialized_hosts.insert("app".into());
        check_open(
            &state,
            &binding(),
            Some(&defaults),
            &json!({"action":"browser_session_open","sessionName":"new-task"}),
        )
        .unwrap();
        assert!(!state.host_policies.has_rules());
    }
    #[test]
    fn addressed_task_rules_match_cli_and_public_aliases_without_affecting_unrestricted_peer() {
        let mut state = fixture();
        state
            .host_policies
            .update(
                0,
                "app",
                "alpha",
                "set_task",
                Some(BrowserTaskPolicy {
                    denied_actions: BTreeSet::from(["screenshot".into()]),
                    ..Default::default()
                }),
                "app",
                "No capture",
            )
            .unwrap();
        for action in ["screenshot", "browser_screenshot"] {
            assert!(check_state(&state, "task-alpha", &json!({"action":action})).is_err());
        }
        check_state(&state, "unrelated", &json!({"action":"screenshot"})).unwrap();
        check_state(
            &state,
            "task-alpha",
            &json!({"action":"browser_session_policy"}),
        )
        .unwrap();
    }
    #[test]
    fn artifact_handles_resolve_exact_new_file_and_refuse_escape_existing_file_or_symlink() {
        let root = std::env::temp_dir().join(format!("artifact-policy-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("nested")).unwrap();
        let root = root.canonicalize().unwrap();
        let policy = BrowserTaskPolicy {
            artifact_destinations: BTreeMap::from([(
                "exports".into(),
                root.to_string_lossy().into_owned(),
            )]),
            ..Default::default()
        };
        assert_eq!(
            artifact_path(&policy, "exports", "nested/capture.png").unwrap(),
            root.join("nested/capture.png")
        );
        for invalid in ["../escape", "/absolute", "nested/../escape", ""] {
            assert!(artifact_path(&policy, "exports", invalid).is_err());
        }
        assert!(artifact_path(&policy, "unknown", "capture.png").is_err());
        std::fs::write(root.join("existing"), b"peer").unwrap();
        assert!(artifact_path(&policy, "exports", "existing").is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("nested"), root.join("link")).unwrap();
            assert!(artifact_path(&policy, "exports", "link/file").is_err());
        }
        let mut state = fixture();
        state
            .host_policies
            .task_overrides
            .insert("alpha".into(), policy);
        let mut command = json!({"action":"screenshot","taskId":"alpha","artifactHandle":"exports","artifactName":"capture.png"});
        prepare_command(&state, "", &mut command).unwrap();
        assert_eq!(command["path"], root.join("capture.png").to_str().unwrap());
        assert!(!root.join("capture.png").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
    fn process() -> crate::process_identity::ProcessObservation {
        crate::process_identity::ProcessObservation::Observed(
            crate::process_identity::ObservedProcessIdentity {
                pid: 42,
                start_token: Some("original".into()),
                executable_path: Some("/synthetic/chrome".into()),
                browser_family: Some("chrome".into()),
                command_line: None,
            },
        )
    }
    #[test]
    fn explicit_broader_close_runs_once_and_requires_fresh_original_process_absence() {
        let state = fixture();
        let command = json!({"actor":"operator","reason":"Finish this exact managed browser"});
        let mut observations = VecDeque::from([
            process(),
            crate::process_identity::ProcessObservation::Missing,
        ]);
        let mut calls = 0;
        let receipt = close_browser_scope_with_observation(
            &state,
            "alpha",
            &command,
            |_| observations.pop_front().unwrap(),
            || {
                calls += 1;
                Ok(json!({"closed":true}))
            },
        )
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(receipt["residue"]["originalProcess"], "absent");
        assert!(observations.is_empty());
        let mut calls = 0;
        let error = close_browser_scope_with_observation(
            &state,
            "alpha",
            &command,
            |_| process(),
            || {
                calls += 1;
                Ok(json!({"closed":true}))
            },
        )
        .unwrap_err();
        assert!(error.starts_with("browser_lifecycle_owned_process_residue:"));
        assert_eq!(calls, 1);
    }
    #[test]
    fn broader_close_preserves_peers_and_borrowed_process_before_any_effect_or_process_query() {
        let mut state = fixture();
        let mut peer = state.sessions["alpha"].clone();
        peer.id = "peer".into();
        peer.name = "peer".into();
        state.sessions.insert(peer.id.clone(), peer);
        let command = json!({"actor":"operator","reason":"Explicit close"});
        assert!(close_browser_scope_with_observation(
            &state,
            "alpha",
            &command,
            |_| panic!("No process query for refused scope"),
            || panic!("No close for peer scope")
        )
        .is_err());
        state.sessions.remove("peer");
        state.shared_browser_ids.insert("browser".into());
        assert!(close_browser_scope_with_observation(
            &state,
            "alpha",
            &command,
            |_| panic!("No process query for borrowed browser"),
            || panic!("No borrowed process close")
        )
        .is_err());
    }
}
