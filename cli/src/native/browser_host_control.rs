//! Durable cooperative browser control. Native viewing alone never calls this API.
//! Admission and takeover share a short file lock; dispatched effects retain
//! uncertainty through takeover rather than being replayed. Unmanaged CDP/OS
//! controllers must cooperate; this is not an OS input interception mechanism.
use std::collections::BTreeMap;
#[cfg(unix)]
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use agent_browser_service_model::BrowserSessionState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::browser_session_store::BrowserSessionSqliteStore;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Control {
    pub(crate) epoch: u64,
    pub(crate) human: bool,
    pub(crate) pause_reads: bool,
    pub(crate) awaiting_observation: bool,
    actor: String,
    reason: String,
    flights: BTreeMap<String, String>,
    #[serde(default)]
    owners: BTreeMap<String, (u32, String)>,
    audit: Vec<Value>,
}

pub(crate) fn path(browser_id: &str) -> Result<PathBuf, String> {
    let database = BrowserSessionSqliteStore::default_sqlite_path()?;
    Ok(database
        .parent()
        .ok_or("browser_control_directory_missing")?
        .join(format!(
            "host-control-{:x}.json",
            Sha256::digest(browser_id.as_bytes())
        )))
}

fn transaction<T>(
    path: &Path,
    update: impl FnOnce(&mut Control) -> Result<T, String>,
) -> Result<T, String> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let lock = options
        .open(path.with_extension("lock"))
        .map_err(|_| "browser_control_store_unavailable")?;
    lock.lock()
        .map_err(|_| "browser_control_lock_unavailable")?;
    let mut control: Control = match std::fs::read(path) {
        Ok(raw) => serde_json::from_slice(&raw).map_err(|_| "browser_control_state_invalid")?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Control::default(),
        Err(_) => return Err("browser_control_read_failed".into()),
    };
    let result = update(&mut control)?;
    let bytes = serde_json::to_vec(&control).map_err(|_| "browser_control_encode_failed")?;
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut output = options
        .open(&temporary)
        .map_err(|_| "browser_control_write_failed")?;
    output
        .write_all(&bytes)
        .map_err(|_| "browser_control_write_failed")?;
    output
        .sync_all()
        .map_err(|_| "browser_control_sync_failed")?;
    std::fs::rename(&temporary, path).map_err(|_| "browser_control_write_failed")?;
    #[cfg(unix)]
    File::open(path.parent().ok_or("browser_control_directory_missing")?)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| "browser_control_sync_failed")?;
    Ok(result)
}

pub(crate) fn read(browser_id: &str) -> Result<Control, String> {
    let path = path(browser_id)?;
    match std::fs::read(&path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|_| "browser_control_state_invalid".into())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Control::default()),
        Err(_) => Err("browser_control_read_failed".into()),
    }
}

pub(crate) fn metadata(browser_id: &str) -> Result<Value, String> {
    let control = read(browser_id)?;
    let pending = control
        .flights
        .iter()
        .filter(|(_, state)| {
            !matches!(
                state.as_str(),
                "observed_completed" | "observed_uncertain_no_replay"
            )
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"epoch":control.epoch,"owner":if control.human {"human"} else if control.awaiting_observation {"awaiting_observation"} else {"agent"},
        "pauseReads":control.pause_reads,"cooperativeOnly":true,"actor":control.actor,
        "pendingOutcomeCount":pending.len(),"pendingOutcomes":pending.iter().take(32).map(|(id,state)|json!({"requestId":id,"state":state})).collect::<Vec<_>>(),"outcomesTruncated":pending.len()>32,"lastChange":control.audit.last()}),
    )
}

pub(crate) fn observation(action: &str) -> bool {
    matches!(
        action,
        "__observe_owned_popup_targets"
            | "url"
            | "title"
            | "snapshot"
            | "screenshot"
            | "content"
            | "gettext"
            | "inputvalue"
            | "isvisible"
            | "getattribute"
            | "innerhtml"
            | "styles"
            | "count"
            | "boundingbox"
            | "isenabled"
            | "ischecked"
            | "cookies_get"
            | "storage_get"
            | "console"
            | "errors"
            | "tab_list"
            | "browser_pid"
            | "requests"
            | "request_detail"
            | "desktop_capture"
            | "desktop_evidence_observe"
    )
}
fn status(action: &str) -> bool {
    matches!(
        action,
        "browser_session_status"
            | "service_status"
            | "ping"
            | "status"
            | "browser_control"
            | "browser_session_reconnect"
            | "browser_session_policy"
            | "browser_policy"
    )
}
fn permits(control: &Control, action: &str) -> Result<(), String> {
    if status(action) {
        return Ok(());
    }
    if observation(action) {
        if control.human && control.pause_reads {
            return Err("browser_control_observation_paused: return control or explicitly allow observation".into());
        }
        return Ok(());
    }
    if control.human {
        return Err("browser_control_human: explicitly return control before automation".into());
    }
    if control.awaiting_observation {
        return Err("browser_control_fresh_observation_required".into());
    }
    Ok(())
}

pub(crate) fn check(browser_id: &str, action: &str) -> Result<(), String> {
    permits(&read(browser_id)?, action)
}
pub(crate) fn check_state(
    state: &BrowserSessionState,
    name: &str,
    command: &Value,
) -> Result<(), String> {
    let action = command["action"].as_str().unwrap_or_default();
    let name = command["sessionName"].as_str().unwrap_or(name);
    for task in state.sessions.values().filter(|task| {
        task.name == name
            || command["sessionId"].as_str() == Some(task.id.as_str())
            || command["browserId"].as_str() == Some(task.browser_id.as_str())
            || command["profileId"].as_str() == Some(task.profile_id.as_str())
            || command["runtimeProfile"].as_str() == Some(task.profile_id.as_str())
    }) {
        if state.host_bindings.keys().any(|id| {
            state
                .sessions
                .get(id)
                .is_some_and(|owned| owned.browser_id == task.browser_id)
        }) {
            check(&task.browser_id, action)?;
        }
    }
    Ok(())
}

/// Takeover requires an exact current epoch and attributable actor. Overrides
/// use the caller's existing authority and remain scoped to the addressed browser.
pub(crate) fn change(
    browser_id: &str,
    operation: &str,
    expected: u64,
    actor: &str,
    reason: &str,
    pause_reads: bool,
) -> Result<u64, String> {
    if actor.trim().is_empty()
        || actor.len() > 160
        || reason.trim().is_empty()
        || reason.len() > 320
    {
        return Err("browser_control_attribution_required".into());
    }
    transaction(&path(browser_id)?, |control| {
        if control.epoch != expected {
            return Err("browser_control_epoch_changed".into());
        }
        match operation {
            "takeover" => {
                control.human = true;
                control.pause_reads = pause_reads;
                control.awaiting_observation = false;
                for outcome in control
                    .flights
                    .values_mut()
                    .filter(|outcome| outcome.as_str() == "dispatched")
                {
                    *outcome = "uncertain_in_flight".into();
                }
            }
            "return" | "override" => {
                control.human = false;
                control.pause_reads = false;
                control.awaiting_observation = true;
            }
            _ => return Err("browser_control_operation_invalid".into()),
        }
        control.epoch = control
            .epoch
            .checked_add(1)
            .ok_or("browser_control_epoch_exhausted")?;
        control.actor = actor.into();
        control.reason = reason.into();
        control.audit.push(
            json!({"epoch":control.epoch,"operation":operation,"actor":actor,"reason":reason}),
        );
        Ok(control.epoch)
    })
}

pub(crate) fn observed(browser_id: &str, expected: u64) -> Result<(), String> {
    transaction(&path(browser_id)?, |control| {
        if control.epoch != expected || control.human || !control.awaiting_observation {
            return Err("browser_control_epoch_changed".into());
        }
        if control
            .flights
            .values()
            .any(|state| state == "dispatched" || state == "uncertain_in_flight")
        {
            return Err("browser_control_in_flight_requires_reconciliation".into());
        }
        control.awaiting_observation = false;
        Ok(())
    })
}

/// Explicit observation reconciliation records uncertainty without replaying the
/// effect. The caller first freshly observes the same existing page.
pub(crate) fn reconcile(
    browser_id: &str,
    expected: u64,
    request_id: &str,
    actor: &str,
    reason: &str,
) -> Result<(), String> {
    if actor.trim().is_empty()
        || reason.trim().is_empty()
        || actor.len() > 160
        || reason.len() > 320
    {
        return Err("browser_control_attribution_required".into());
    }
    transaction(&path(browser_id)?, |control| {
        if control.epoch != expected || control.human || !control.awaiting_observation {
            return Err("browser_control_return_required".into());
        }
        let retained = control
            .flights
            .get(request_id)
            .ok_or("browser_control_request_missing")?;
        if matches!(retained.as_str(), "dispatched" | "uncertain_in_flight") {
            let (pid, start) = control
                .owners
                .get(request_id)
                .ok_or("browser_control_dispatch_owner_unknown")?;
            match crate::process_identity::observe_process(*pid) {
                crate::process_identity::ProcessObservation::Missing => {}
                crate::process_identity::ProcessObservation::Observed(process)
                    if process
                        .start_token
                        .as_ref()
                        .is_some_and(|actual| actual != start) => {}
                _ => return Err("browser_control_dispatch_still_in_flight".into()),
            }
        }
        let outcome = control
            .flights
            .get_mut(request_id)
            .ok_or("browser_control_request_missing")?;
        if !matches!(
            outcome.as_str(),
            "dispatched" | "uncertain_in_flight" | "uncertain_no_replay"
        ) {
            return Err("browser_control_outcome_already_observed".into());
        }
        *outcome = "observed_uncertain_no_replay".into();
        control.audit.push(json!({"epoch":control.epoch,"operation":"reconcile","requestId":request_id,"actor":actor,"reason":reason}));
        Ok(())
    })
}

pub(crate) struct Admission {
    path: PathBuf,
    id: String,
}
impl Admission {
    pub(crate) fn undispatched(self) -> Result<(), String> {
        transaction(&self.path, |control| {
            control.flights.remove(&self.id);
            control.owners.remove(&self.id);
            Ok(())
        })
    }
    pub(crate) fn finish(self, success: bool) -> Result<(), String> {
        transaction(&self.path, |control| {
            control.flights.insert(
                self.id.clone(),
                if success {
                    "observed_completed"
                } else {
                    "uncertain_no_replay"
                }
                .into(),
            );
            Ok(())
        })
    }
}
/// Each cooperating route calls this immediately before actual page dispatch.
pub(crate) fn admit(browser_id: &str, action: &str, id: &str) -> Result<Option<Admission>, String> {
    if status(action) || observation(action) {
        check(browser_id, action)?;
        return Ok(None);
    }
    let path = path(browser_id)?;
    let id = if id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        id.to_string()
    };
    let owner = match crate::process_identity::observe_process(std::process::id()) {
        crate::process_identity::ProcessObservation::Observed(process) => process
            .start_token
            .ok_or("browser_control_dispatch_owner_unknown")?,
        _ => return Err("browser_control_dispatch_owner_unknown".into()),
    };
    transaction(&path, |control| {
        permits(control, action)?;
        if control.flights.contains_key(&id) {
            return Err("browser_control_request_already_dispatched: inspect retained outcome; do not replay".into());
        }
        control.flights.insert(id.clone(), "dispatched".into());
        control
            .owners
            .insert(id.clone(), (std::process::id(), owner));
        Ok(())
    })?;
    Ok(Some(Admission { path, id }))
}

/// Last managed-runtime admission, before target activation or special operator
/// routes. Runtime-owned inner action dispatch performs check-only admission.
pub(crate) fn guarded(
    browser_id: &str,
    command: &Value,
    run: impl FnOnce() -> Result<Value, String>,
) -> Result<Value, String> {
    let mut addressed = command.clone();
    addressed["browserId"] = json!(browser_id);
    let admission = admit_command(None, None, &addressed, false)?;
    let result = run();
    if let Some(admission) = admission {
        if result
            .as_ref()
            .is_ok_and(|value| value["data"]["confirmation_required"] == true)
        {
            admission.undispatched()?;
        } else {
            admission.finish(result.as_ref().is_ok_and(|value| value["success"] != false))?;
        }
    }
    result
}

/// Join a daemon lane/PID to the same durable hosted-browser identity.
pub(crate) fn admit_command(
    name: Option<&str>,
    pid: Option<u32>,
    command: &Value,
    check_only: bool,
) -> Result<Option<Admission>, String> {
    let name = command["sessionName"].as_str().or(name);
    let Some(state) = BrowserSessionSqliteStore::default_session_state_read_only()? else {
        return Ok(None);
    };
    super::browser_host_reconnect::check_state(&state, name.unwrap_or_default(), command)?;
    super::browser_host_policy::check_state(&state, name.unwrap_or_default(), command)?;
    let browser = state.sessions.values().find(|task| {
        name == Some(task.name.as_str())
            || name == Some(task.id.as_str())
            || command["sessionId"].as_str() == Some(task.id.as_str())
            || command["browserId"].as_str() == Some(task.browser_id.as_str())
            || state
                .browsers
                .get(&task.browser_id)
                .is_some_and(|browser| Some(browser.pid) == pid)
    });
    let Some(task) = browser else {
        return Ok(None);
    };
    if !state.host_bindings.keys().any(|id| {
        state
            .sessions
            .get(id)
            .is_some_and(|peer| peer.browser_id == task.browser_id)
    }) {
        return Ok(None);
    }
    if check_only {
        check(
            &task.browser_id,
            command["action"].as_str().unwrap_or_default(),
        )?;
        return Ok(None);
    }
    admit(
        &task.browser_id,
        command["action"].as_str().unwrap_or_default(),
        command["id"].as_str().unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn takeover_blocks_mutations_and_optionally_reads() {
        let control = Control {
            human: true,
            ..Default::default()
        };
        assert!(permits(&control, "click").is_err());
        assert!(permits(&control, "evaluate").is_err());
        assert!(permits(&control, "url").is_ok());
        assert!(permits(
            &Control {
                pause_reads: true,
                ..control.clone()
            },
            "snapshot"
        )
        .is_err());
        assert!(permits(&control, "browser_session_status").is_ok());
        assert!(permits(&control, "browser_session_attach").is_err());
    }
    #[test]
    fn view_only_default_does_not_pause() {
        assert!(permits(&Control::default(), "click").is_ok());
        assert!(permits(
            &Control {
                awaiting_observation: true,
                ..Default::default()
            },
            "click"
        )
        .is_err());
    }
    #[test]
    fn durable_journal_retains_pending_and_epoch() {
        let directory = std::env::temp_dir().join(format!("control-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("state.json");
        transaction(&path, |c| {
            c.human = true;
            c.epoch = 3;
            c.flights
                .insert("effect".into(), "uncertain_in_flight".into());
            Ok(())
        })
        .unwrap();
        let loaded: Control = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(loaded.human);
        assert_eq!(loaded.epoch, 3);
        assert_eq!(loaded.flights["effect"], "uncertain_in_flight");
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::test_utils::EnvGuard;
    fn fixture(run: impl FnOnce()) {
        let guard = EnvGuard::new(&[
            "HOME",
            "AGENT_BROWSER_HOME",
            "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        ]);
        let directory = std::env::temp_dir().join(format!("host-control-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(directory.join(".agent-browser/service")).unwrap();
        guard.set("HOME", directory.to_str().unwrap());
        guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
        guard.set(
            "AGENT_BROWSER_HOME",
            directory.join(".agent-browser").to_str().unwrap(),
        );
        run();
        // default_sqlite intentionally seals its one-time migration archive.
        // Restore deletion permission only for this disposable fixture's exact
        // archive directories, never a runtime path or a sibling /tmp fixture.
        let service_directory = directory.join(".agent-browser/service");
        for entry in std::fs::read_dir(&service_directory).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name();
            let name = name.to_str().unwrap();
            let Some(digest) = name.strip_prefix("migration-archive-") else {
                continue;
            };
            if digest.len() != 16 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                continue;
            }
            let metadata = std::fs::symlink_metadata(entry.path()).unwrap();
            assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
            #[cfg(unix)]
            {
                use std::os::unix::fs::{MetadataExt, PermissionsExt};
                assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
                let mode = metadata.permissions().mode() & 0o777;
                assert!(
                    matches!(mode, 0o500 | 0o700),
                    "Unexpected fixture archive mode"
                );
                if mode == 0o500 {
                    std::fs::set_permissions(entry.path(), std::fs::Permissions::from_mode(0o700))
                        .unwrap();
                }
            }
            #[cfg(windows)]
            for archived in std::fs::read_dir(entry.path()).unwrap() {
                let archived = archived.unwrap();
                let metadata = std::fs::symlink_metadata(archived.path()).unwrap();
                assert!(metadata.is_file() && !metadata.file_type().is_symlink());
                let mut permissions = metadata.permissions();
                permissions.set_readonly(false);
                std::fs::set_permissions(archived.path(), permissions).unwrap();
            }
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn host_control_atomic_takeover_retains_dispatch_and_requires_fresh_return() {
        fixture(|| {
            let flight = admit("browser", "click", "effect").unwrap().unwrap();
            let epoch = change(
                "browser",
                "takeover",
                0,
                "operator",
                "Human assistance",
                false,
            )
            .unwrap();
            assert!(admit("browser", "fill", "after-takeover").is_err());
            assert!(admit("browser", "snapshot", "read").unwrap().is_none());
            assert_eq!(
                read("browser").unwrap().flights["effect"],
                "uncertain_in_flight"
            );
            let epoch = change("browser", "return", epoch, "operator", "Resume", false).unwrap();
            assert!(observed("browser", epoch).is_err());
            assert!(
                reconcile(
                    "browser",
                    epoch,
                    "effect",
                    "operator",
                    "Observed current page"
                )
                .is_err(),
                "Cannot clear a still-live dispatcher"
            );
            flight.finish(true).unwrap();
            assert!(admit("browser", "click", "before-observation").is_err());
            observed("browser", epoch).unwrap();
            let next = admit("browser", "click", "after-observation")
                .unwrap()
                .unwrap();
            next.finish(false).unwrap();
            assert_eq!(
                read("browser").unwrap().flights["after-observation"],
                "uncertain_no_replay"
            );
            assert!(
                admit("browser", "click", "after-observation").is_err(),
                "Never replay an uncertain website effect"
            );
            let next_epoch = change(
                "browser",
                "takeover",
                epoch,
                "operator",
                "Second assistance",
                false,
            )
            .unwrap();
            assert_eq!(
                read("browser").unwrap().flights["effect"],
                "observed_completed"
            );
            let final_epoch = change(
                "browser",
                "return",
                next_epoch,
                "operator",
                "Resume again",
                false,
            )
            .unwrap();
            observed("browser", final_epoch).unwrap();
        });
    }
    #[test]
    fn host_control_restart_preserves_human_capture_pause_and_attributable_override() {
        fixture(|| {
            change("browser", "takeover", 0, "human", "Assist", true).unwrap();
            assert!(check("browser", "screenshot").is_err());
            assert!(check("browser", "status").is_ok());
            assert!(change(
                "browser",
                "override",
                0,
                "operator",
                "Stale override",
                false
            )
            .is_err());
            let epoch = change(
                "browser",
                "override",
                1,
                "operator",
                "Explicit local operator override",
                false,
            )
            .unwrap();
            assert!(check("browser", "click").is_err());
            observed("browser", epoch).unwrap();
            assert!(check("browser", "click").is_ok());
            let state = read("browser").unwrap();
            assert_eq!(state.audit[1]["actor"], "operator");
            assert_eq!(state.audit[1]["operation"], "override");
        });
    }
    #[test]
    fn host_control_concurrent_takeovers_have_one_winner() {
        fixture(|| {
            let workers = (0..2)
                .map(|_| {
                    std::thread::spawn(|| {
                        change("browser", "takeover", 0, "operator", "Concurrent", false)
                    })
                })
                .collect::<Vec<_>>();
            let wins = workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .filter(Result::is_ok)
                .count();
            assert_eq!(wins, 1);
        });
    }
    #[test]
    fn host_control_in_flight_effect_and_takeover_run_concurrently() {
        fixture(|| {
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let (finish_tx, finish_rx) = std::sync::mpsc::channel();
            let worker = std::thread::spawn(move || {
                let admission = admit("browser", "click", "live-effect").unwrap().unwrap();
                started_tx.send(()).unwrap();
                finish_rx.recv().unwrap();
                admission.finish(true).unwrap();
            });
            started_rx.recv().unwrap();
            let epoch = change(
                "browser",
                "takeover",
                0,
                "human",
                "Concurrent assistance",
                false,
            )
            .unwrap();
            assert!(admit("browser", "fill", "blocked-followup").is_err());
            assert_eq!(
                read("browser").unwrap().flights["live-effect"],
                "uncertain_in_flight"
            );
            let epoch = change(
                "browser",
                "return",
                epoch,
                "human",
                "Explicit return",
                false,
            )
            .unwrap();
            assert!(observed("browser", epoch).is_err());
            finish_tx.send(()).unwrap();
            worker.join().unwrap();
            observed("browser", epoch).unwrap();
            assert_eq!(
                read("browser").unwrap().flights["live-effect"],
                "observed_completed"
            );
        });
    }
    #[test]
    fn host_control_shared_dispatch_rejects_mutation_before_browser_launch() {
        fixture(|| {
            use agent_browser_service_model::{
                BrowserHostTaskBinding, ManagedBrowserInstance, ManagedBrowserSession,
            };
            let mut state = BrowserSessionState::default();
            state.sessions.insert(
                "task".into(),
                ManagedBrowserSession {
                    id: "task".into(),
                    name: "hosted-lane".into(),
                    profile_id: "profile".into(),
                    browser_id: "browser".into(),
                    created_at_ms: 1,
                    last_activity_at_ms: 1,
                    expires_at_ms: u64::MAX,
                    current_tab_id: None,
                },
            );
            state.browsers.insert(
                "browser".into(),
                ManagedBrowserInstance {
                    id: "browser".into(),
                    profile_id: "profile".into(),
                    pid: 999999,
                    cdp_endpoint: "ws://127.0.0.1:1".into(),
                    desktop: None,
                    active_session_ids: vec!["task".into()],
                },
            );
            let binding:BrowserHostTaskBinding=serde_json::from_value(json!({"request":{"version":1,"hostId":"app","participantId":"participant","sessionId":"session","placement":{"mode":"required","desktop":{"desktopId":"desktop","generation":1}}},"desktop":{"desktop_id":"desktop","friendly_route_label":"Desktop 1","generation":1}})).unwrap();
            state.host_bindings.insert("task".into(), binding);
            BrowserSessionSqliteStore::default_sqlite()
                .unwrap()
                .save_session_state(&state)
                .unwrap();
            change("browser", "takeover", 0, "human", "Assist", true).unwrap();
            // MCP, CLI and service requests meet this shared native dispatcher.
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let mut daemon = super::super::action_runtime::runtime::DaemonState::new();
            daemon.session_name = Some("hosted-lane".into());
            for action in ["click", "evaluate", "desktop_interact", "screenshot"] {
                let response = runtime.block_on(super::super::actions::execute_command(
                    &json!({"id":format!("blocked-{action}"),"action":action}),
                    &mut daemon,
                ));
                assert_eq!(response["success"], false, "{action}");
                assert!(
                    response["error"]
                        .as_str()
                        .unwrap()
                        .starts_with("browser_control_"),
                    "{response}"
                );
                assert!(daemon.browser.is_none());
            }
            // Exercise the canonical public Service request body normalization
            // shared by HTTP /api/service/request and service MCP ingress, then
            // its actual native action dispatcher. This does not claim an HTTP
            // socket/authentication acceptance result.
            let service_request = json!({"action":"click","sessionName":"hosted-lane","serviceName":"fixture-service","agentName":"control-acceptance","taskName":"Reject automation during human assistance","params":{"selector":"#synthetic-control"}});
            let normalized = crate::native::service_request::normalize_service_request(
                crate::native::service_request::ServiceRequestNormalization {
                    request: &service_request,
                    service_state: None,
                    authenticated_principal: None,
                    fallback_principal: None,
                    request_id: "service-control-rejection",
                    effective_session: Some("hosted-lane"),
                },
            )
            .unwrap();
            assert_eq!(normalized.command["action"], "click");
            let service_response = runtime.block_on(super::super::actions::execute_command(
                &normalized.command,
                &mut daemon,
            ));
            assert_eq!(service_response["success"], false);
            assert!(
                service_response["error"]
                    .as_str()
                    .unwrap()
                    .starts_with("browser_control_human"),
                "{service_response}"
            );
            assert!(
                daemon.browser.is_none(),
                "Service refusal must precede launch or browser input"
            );
            assert!(check_state(
                &state,
                "hosted-lane",
                &json!({"action":"browser_session_attach"})
            )
            .is_err());
            assert!(check_state(
                &state,
                "",
                &json!({"action":"browser_session_close","sessionId":"task"})
            )
            .is_err());
            assert!(check_state(&state, "foreign-lane", &json!({"action":"click"})).is_ok());
        });
    }
}
