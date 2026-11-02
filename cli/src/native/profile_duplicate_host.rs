//! Hosted profile duplication joins the existing browser owner and private copy
//! journal. Public requests carry intent, never process identity or profile paths.
use super::browser_build_selection::{verify_build, ExecutableEvidence};
use super::*;
#[path = "profile_duplicate_copy.rs"]
pub(super) mod profile_duplicate_copy;
#[path = "profile_duplicate_custody.rs"]
mod profile_duplicate_custody;
use agent_browser_service_model::ManagedBrowserCommandEffects;
use agent_browser_service_model::{RecordedProcessIdentity, RemoteViewTabHandoffTarget};
use profile_duplicate_copy::*;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[derive(Clone, Serialize, Deserialize)]
struct PublicIntent {
    operation_id: String,
    source_profile_id: String,
    destination_profile_id: String,
    destination_name: String,
    launch: bool,
    #[serde(default)]
    target_build: Option<ExecutableEvidence>,
    #[serde(default)]
    target_mode: Option<String>,
    #[serde(default)]
    profile_data_version: Option<String>,
    #[serde(default)]
    cancelled: bool,
}
impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    BrowserSessionHost<P, E>
{
    pub(crate) fn profile_duplicate(&mut self, command: &Value) -> Result<Value, String> {
        let operation_id = required_string(command, "operationId")?;
        if operation_id.len() > 128 {
            return Err("profile_duplicate_operation_id_invalid".into());
        }
        let database = self.persistence.runtime_database_path()?;
        let directory = database
            .parent()
            .ok_or("profile_duplicate_runtime_directory_missing")?
            .join("profile-duplication");
        std::fs::create_dir_all(&directory)
            .map_err(|_| "profile_duplicate_runtime_directory_unavailable")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| "profile_duplicate_runtime_permissions_failed")?;
        }
        let connection = rusqlite::Connection::open(directory.join("operations.sqlite3"))
            .map_err(|_| "profile_duplicate_journal_unavailable")?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS profile_duplicate_host_intents (id TEXT PRIMARY KEY, intent TEXT NOT NULL)").map_err(|_|"profile_duplicate_journal_unavailable")?;
        use rusqlite::OptionalExtension;
        let retained: Option<String> = connection
            .query_row(
                "SELECT intent FROM profile_duplicate_host_intents WHERE id=?1",
                [operation_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| "profile_duplicate_intent_read_failed")?;
        let mut intent: PublicIntent = if let Some(retained) = retained {
            serde_json::from_str(&retained).map_err(|_| "profile_duplicate_intent_invalid")?
        } else {
            let source_profile_id = required_string(command, "sourceProfileId")?.to_owned();
            let source = self
                .catalog
                .profiles
                .get(&source_profile_id)
                .ok_or("profile_duplicate_source_profile_missing")?;
            if source.kind != agent_browser_service_model::BrowserProfileKind::Named {
                return Err(
                    "profile_duplicate_named_source_required:promote this profile first".into(),
                );
            }
            let target_build = self.requested_profile_build(command, &source_profile_id)?;
            let intent = PublicIntent {
                operation_id: operation_id.to_owned(),
                source_profile_id,
                destination_profile_id: format!("duplicate:{}", uuid::Uuid::new_v4()),
                destination_name: optional_string(command, "destinationName")
                    .unwrap_or("Independent copy")
                    .to_owned(),
                launch: command
                    .get("launch")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                profile_data_version: super::browser_build_selection::profile_data_version(source),
                target_build,
                target_mode: match command.get("mode") {
                    None => None,
                    Some(Value::String(mode)) if matches!(mode.as_str(), "cdp" | "cdp_free") => {
                        Some(mode.clone())
                    }
                    _ => return Err("browser_build_mode_invalid".into()),
                },
                cancelled: false,
            };
            connection
                .execute(
                    "INSERT INTO profile_duplicate_host_intents VALUES(?1,?2)",
                    rusqlite::params![
                        operation_id,
                        serde_json::to_string(&intent)
                            .map_err(|_| "profile_duplicate_intent_encode_failed")?
                    ],
                )
                .map_err(|_| "profile_duplicate_intent_conflict")?;
            intent
        };
        if optional_string(command, "sourceProfileId")
            .is_some_and(|id| id != intent.source_profile_id)
            || optional_string(command, "destinationName")
                .is_some_and(|name| name != intent.destination_name)
        {
            return Err("profile_duplicate_operation_id_conflict".into());
        }
        if let Some(path) = optional_string(command, "executablePath") {
            let canonical = Path::new(path).canonicalize().ok();
            if intent.target_build.as_ref().is_none_or(|build| {
                canonical
                    .as_ref()
                    .is_some_and(|path| path != Path::new(&build.path))
                    || optional_string(command, "expectedSha256") != build.sha256.as_deref()
            }) {
                return Err(
                    "profile_duplicate_operation_id_conflict:retain the original selected build"
                        .into(),
                );
            }
        }
        if optional_string(command, "mode")
            .is_some_and(|mode| Some(mode) != intent.target_mode.as_deref())
        {
            return Err("profile_duplicate_operation_id_conflict".into());
        }
        if optional_string(command, "buildId").is_some_and(|id| {
            intent
                .target_build
                .as_ref()
                .is_none_or(|build| build.id != id)
        }) {
            return Err("profile_duplicate_operation_id_conflict".into());
        }
        let choice = optional_string(command, "choice").unwrap_or("prepare");
        connection.execute_batch("CREATE TABLE IF NOT EXISTS profile_duplicate_owner_bindings(id TEXT PRIMARY KEY,binding TEXT NOT NULL)").map_err(|_|"profile_duplicate_journal_unavailable")?;
        let binding_exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM profile_duplicate_owner_bindings WHERE id=?1)",
                [operation_id],
                |r| r.get(0),
            )
            .map_err(|_| "profile_duplicate_binding_read_failed")?;
        if intent.cancelled {
            return Ok(build_view(
                json!({"schemaVersion":"agent-browser.profile-duplication.v1","operationId":operation_id,"state":"cancelled","phase":"cancelled","actions":[]}),
                &intent,
            ));
        }
        if binding_exists {
            return self
                .advance_profile_duplicate(command, &intent, &directory, &database)
                .map(|view| build_view(view, &intent));
        }
        if choice == "cancel" {
            intent.cancelled = true;
            connection
                .execute(
                    "UPDATE profile_duplicate_host_intents SET intent=?1 WHERE id=?2",
                    rusqlite::params![
                        serde_json::to_string(&intent)
                            .map_err(|_| "profile_duplicate_intent_encode_failed")?,
                        operation_id
                    ],
                )
                .map_err(|_| "profile_duplicate_intent_write_failed")?;
            return Ok(build_view(
                json!({"schemaVersion":"agent-browser.profile-duplication.v1","operationId":operation_id,"state":"cancelled","phase":"cancelled","actions":[]}),
                &intent,
            ));
        }
        if !self
            .state
            .browsers
            .values()
            .any(|browser| browser.profile_id == intent.source_profile_id)
        {
            if choice == "open_source" {
                if command["ownerConfirmed"] != true {
                    return Err("profile_duplicate_owner_confirmation_required".into());
                }
                let _ = self.open(OpenBrowserSession::exact_profile(
                    duplicate_session_name(operation_id, true),
                    &intent.source_profile_id,
                    current_unix_ms(),
                ))?;
                let mut next = command.clone();
                next["choice"] = json!("prepare");
                return self.profile_duplicate(&next);
            }
            return Ok(build_view(
                json!({"schemaVersion":"agent-browser.profile-duplication.v1","operationId":operation_id,"state":"source_preparation_required","phase":"source_preparation_required","source":{"id":intent.source_profile_id},"destination":{"id":intent.destination_profile_id,"name":intent.destination_name},"authenticationContinuity":"unverified","actions":[{"action":"open_source","requiresConfirmation":true},{"action":"cancel","requiresConfirmation":false}],"warning":"Open the source with its retained build for clean-close custody. The selected destination build is tested only on the independent copy."}),
                &intent,
            ));
        }
        self.advance_profile_duplicate(command, &intent, &directory, &database)
            .map(|view| build_view(view, &intent))
    }
}

fn build_view(mut view: Value, intent: &PublicIntent) -> Value {
    if let Some(build) = &intent.target_build {
        view["selectedBuild"] = build.public();
        view["compatibility"] = super::browser_build_selection::compatibility(
            intent.profile_data_version.as_deref(),
            None,
            build,
        );
        view["profileDataVersion"] = json!(intent.profile_data_version);
        view["compatibilityEvidenceSource"] = json!("prepare_snapshot");
        view["profileDataReadable"] = json!("unverified");
        view["credentialStorage"] = json!("unverified");
        view["buildWarning"]=json!("Only the independent destination uses the selected build. Copying may not preserve login; verify authentication independently and sign in again if needed. Credential encryption is unchanged.");
    }
    if let Some(mode) = &intent.target_mode {
        view["requestedMode"] = json!(mode);
    }
    view
}
pub(super) fn decorate_retained_build(
    mut view: Value,
    raw: &str,
    binding: Option<&Value>,
) -> Result<Value, String> {
    view["sourceObservation"] = binding
        .and_then(|binding| binding.get("source_observation"))
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or_else(unknown_source_observation);
    let intent: PublicIntent =
        serde_json::from_str(raw).map_err(|_| "browser_build_copy_readback_invalid")?;
    Ok(build_view(view, &intent))
}

fn unknown_source_observation() -> Value {
    json!({"mode":"unverified","reason":"not_retained","build":null,"buildEvidence":"unverified"})
}

/// Freeze process posture independently of next-launch configuration. Version
/// discovery executes only the existing isolated executable inspection, never a profile.
fn observe_source_binding(
    browser: &agent_browser_service_model::ManagedBrowserInstance,
    original: &RecordedProcessIdentity,
    executable: &str,
    digest: &str,
    ownership: &str,
) -> Value {
    let inspect = || {
        super::super::browser_session_store::mode_inspection::inspect(
            browser,
            Some(original),
            ownership,
            crate::process_identity::observe_process,
        )
    };
    let mut observation = inspect();
    observation["build"] = Value::Null;
    observation["buildEvidence"] = json!("unverified");
    if !matches!(observation["mode"].as_str(), Some("cdp" | "cdp_free")) {
        return observation;
    }
    let executable = super::browser_build_selection::inspect_executable(Path::new(executable));
    let after = inspect();
    let stable = after["mode"] == observation["mode"]
        && matches!(after["mode"].as_str(), Some("cdp" | "cdp_free"))
        && executable.available
        && executable.sha256.as_deref() == Some(digest);
    #[cfg(target_os = "linux")]
    let loaded_matches = executable_digest(Path::new(&format!("/proc/{}/exe", original.pid)))
        .is_ok_and(|value| value == digest);
    #[cfg(not(target_os = "linux"))]
    let loaded_matches = true;
    if stable && loaded_matches {
        observation["build"] = executable.public();
        observation["buildEvidence"] = json!(if cfg!(target_os = "linux") {
            "original_loaded_digest"
        } else {
            "original_executable_on_disk"
        });
    }
    // A process change during version discovery invalidates posture too.
    if after["mode"] != observation["mode"] || after["reason"] != Value::Null {
        observation["mode"] = json!("unverified");
        observation["reason"] = json!("process_changed_during_build_observation");
    }
    observation
}

#[derive(Clone, Serialize, Deserialize)]
struct OwnerBinding {
    #[serde(default)]
    source_observation: Option<Value>,
    #[serde(default)]
    borrowed_anchor: Option<profile_duplicate_custody::BorrowedSourceAnchor>,
    original: RecordedProcessIdentity,
    browser: agent_browser_service_model::ManagedBrowserInstance,
    session: agent_browser_service_model::ManagedBrowserSession,
    tab: agent_browser_service_model::ManagedBrowserTab,
    executable: String,
    digest: String,
    #[serde(default)]
    target_build: Option<ExecutableEvidence>,
    #[serde(default)]
    target_mode: Option<String>,
    close_dispatched: bool,
    clean_close: bool,
    reviewed_tasks: Vec<String>,
}
impl OwnerBinding {
    fn target(&self) -> RemoteViewTabHandoffTarget {
        RemoteViewTabHandoffTarget {
            browser: self.browser.clone(),
            session: self.session.clone(),
            tab: self.tab.clone(),
        }
    }
}
fn duplicate_session_name(operation_id: &str, source: bool) -> String {
    format!(
        "pd{}-{:x}",
        if source { "s" } else { "d" },
        Sha256::digest(operation_id.as_bytes())
    )
}
fn executable_digest(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|_| "profile_duplicate_build_unavailable")?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| "profile_duplicate_build_unavailable")?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
fn source_locks_released(path: &Path) -> bool {
    ["SingletonLock", "SingletonSocket", "SingletonCookie"]
        .iter()
        .all(|name| {
            std::fs::symlink_metadata(path.join(name))
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        })
}
impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    BrowserSessionHost<P, E>
{
    fn advance_profile_duplicate(
        &mut self,
        command: &Value,
        intent: &PublicIntent,
        directory: &Path,
        database: &Path,
    ) -> Result<Value, String> {
        // Repair only this operation's legacy task label. Task/browser/handoff
        // identities stay stable, and its existing browser is never relaunched.
        let safe_name = duplicate_session_name(&intent.operation_id, false);
        let legacy_name = format!("profile-duplicate:{}", intent.operation_id);
        let mut repaired = false;
        for task in self.state.sessions.values_mut().filter(|task| {
            task.profile_id == intent.destination_profile_id && task.name == legacy_name
        }) {
            task.name = safe_name.clone();
            repaired = true;
        }
        if repaired {
            self.commit_state()?;
        }
        let connection = rusqlite::Connection::open(directory.join("operations.sqlite3"))
            .map_err(|_| "profile_duplicate_journal_unavailable")?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS profile_duplicate_owner_bindings(id TEXT PRIMARY KEY,binding TEXT NOT NULL)").map_err(|_|"profile_duplicate_journal_unavailable")?;
        use rusqlite::OptionalExtension;
        let retained: Option<String> = connection
            .query_row(
                "SELECT binding FROM profile_duplicate_owner_bindings WHERE id=?1",
                [&intent.operation_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| "profile_duplicate_binding_read_failed")?;
        let first_review = retained.is_none();
        let binding: OwnerBinding = if let Some(value) = retained {
            serde_json::from_str(&value).map_err(|_| "profile_duplicate_binding_invalid")?
        } else {
            let browser = self
                .state
                .browsers
                .values()
                .find(|browser| browser.profile_id == intent.source_profile_id)
                .cloned()
                .ok_or("profile_duplicate_source_browser_missing")?;
            let session = self
                .state
                .sessions
                .values()
                .find(|session| session.browser_id == browser.id)
                .cloned()
                .ok_or("profile_duplicate_source_task_missing")?;
            let tab = session
                .current_tab_id
                .as_ref()
                .and_then(|id| self.state.tabs.get(id))
                .cloned()
                .ok_or("profile_duplicate_source_tab_missing")?;
            let original =
                if let Some(attachment) = self.state.app_browser_attachments.get(&browser.id) {
                    attachment.request.process.clone()
                } else {
                    BrowserSessionSqliteStore::open_read_only(database)?
                        .published_launch_process_identity(&browser)
                        .map_err(|_| "profile_duplicate_original_identity_unavailable")?
                        .ok_or("profile_duplicate_original_identity_missing")?
                };
            if !crate::process_identity::recorded_process_is_running(&original).unwrap_or(false) {
                return Err("profile_duplicate_original_identity_changed".into());
            }
            let executable = original
                .executable_path
                .clone()
                .ok_or("profile_duplicate_source_build_missing")?;
            let digest = executable_digest(Path::new(&executable))?;
            #[cfg(target_os = "linux")]
            if executable_digest(Path::new(&format!("/proc/{}/exe", original.pid)))? != digest {
                return Err("profile_duplicate_loaded_build_changed".into());
            }
            let reviewed_tasks = self
                .state
                .sessions
                .values()
                .filter(|task| task.profile_id == intent.source_profile_id)
                .map(|task| task.id.clone())
                .collect();
            let borrowed_anchor = if self.state.app_browser_attachments.contains_key(&browser.id) {
                let profile = self
                    .catalog
                    .profiles
                    .get(&intent.source_profile_id)
                    .ok_or("profile_duplicate_source_profile_missing")?;
                let source = SourceProfile {
                    id: profile.id.clone(),
                    name: profile.name.clone(),
                    user_data_dir: PathBuf::from(&profile.user_data_dir),
                    browser_build: digest.clone(),
                    generation: browser
                        .desktop
                        .as_ref()
                        .map(|desktop| desktop.generation)
                        .unwrap_or(1),
                };
                Some(profile_duplicate_custody::BorrowedSourceAnchor::capture(
                    &intent.operation_id,
                    &source,
                    &self.state,
                    &browser.id,
                )?)
            } else {
                None
            };
            let source_observation = Some(observe_source_binding(
                &browser,
                &original,
                &executable,
                &digest,
                if borrowed_anchor.is_some() {
                    "app"
                } else {
                    "managed"
                },
            ));
            let binding = OwnerBinding {
                source_observation,
                borrowed_anchor,
                original,
                browser,
                session,
                tab,
                executable,
                digest,
                target_build: intent.target_build.clone(),
                target_mode: intent.target_mode.clone(),
                close_dispatched: false,
                clean_close: false,
                reviewed_tasks,
            };
            save_binding(&connection, &intent.operation_id, &binding)?;
            binding
        };
        let source = self
            .catalog
            .profiles
            .get(&intent.source_profile_id)
            .ok_or("profile_duplicate_source_profile_missing")?;
        let request = DuplicateRequest {
            operation_id: intent.operation_id.clone(),
            source: SourceProfile {
                id: source.id.clone(),
                name: source.name.clone(),
                user_data_dir: PathBuf::from(&source.user_data_dir),
                browser_build: binding.digest.clone(),
                generation: binding
                    .browser
                    .desktop
                    .as_ref()
                    .map(|desktop| desktop.generation)
                    .unwrap_or(1),
            },
            destination: DestinationProfile {
                id: intent.destination_profile_id.clone(),
                name: intent.destination_name.clone(),
                user_data_dir: directory
                    .join("profiles")
                    .join(intent.destination_profile_id.replace(':', "-")),
            },
            launch: intent.launch,
        };
        std::fs::create_dir_all(directory.join("profiles"))
            .map_err(|_| "profile_duplicate_destination_parent_unavailable")?;
        let mut journal = DuplicationStore::open(&directory.join("operations.sqlite3"))?;
        journal.prepare(request)?;
        let choice = match optional_string(command, "choice").unwrap_or("prepare") {
            "prepare" | "reconcile" => Choice::Reconcile,
            "focus_source" => Choice::FocusSource,
            "close_and_duplicate" => {
                if command["ownerConfirmed"] != true {
                    return Err("profile_duplicate_owner_confirmation_required".into());
                }
                Choice::CloseAndDuplicate
            }
            "retry_launch" | "finish_without_opening" => {
                if command["ownerConfirmed"] != true {
                    return Err("profile_duplicate_owner_confirmation_required".into());
                }
                if command["choice"] == "retry_launch" {
                    Choice::RetryLaunch
                } else {
                    Choice::FinishWithoutOpening
                }
            }
            "cancel" => Choice::Cancel,
            _ => return Err("profile_duplicate_choice_invalid".into()),
        };
        let choice = if first_review && choice == Choice::CloseAndDuplicate {
            Choice::Reconcile
        } else {
            choice
        };
        let mut runtime = HostedDuplication {
            host: self,
            database: database.to_path_buf(),
            binding,
            connection,
            operation_id: intent.operation_id.clone(),
            guard: Rc::new(RefCell::new(None)),
        };
        match journal.advance(&intent.operation_id, choice, &mut runtime) {
            Ok(operation) => {
                if operation.phase == Phase::Cancelled
                    && runtime
                        .host
                        .state
                        .profile_operation_ids
                        .get(&intent.source_profile_id)
                        == Some(&intent.operation_id)
                {
                    runtime
                        .host
                        .state
                        .profile_operation_ids
                        .remove(&intent.source_profile_id);
                    runtime.host.commit_state()?;
                }
                let mut view = operation.public_view();
                view["sourceObservation"] = runtime
                    .binding
                    .source_observation
                    .clone()
                    .unwrap_or_else(unknown_source_observation);
                view["state"] = view["phase"].clone();
                Ok(view)
            }
            Err(error) => {
                if let Some(operation) = journal.read(&intent.operation_id)? {
                    let mut view = operation.public_view();
                    view["sourceObservation"] = runtime
                        .binding
                        .source_observation
                        .clone()
                        .unwrap_or_else(unknown_source_observation);
                    view["state"] = view["phase"].clone();
                    view["failure"] = json!(error
                        .split(':')
                        .next()
                        .unwrap_or("profile_duplicate_readback_required"));
                    Ok(view)
                } else {
                    Err(error)
                }
            }
        }
    }
}
fn save_binding(
    connection: &rusqlite::Connection,
    id: &str,
    binding: &OwnerBinding,
) -> Result<(), String> {
    connection.execute("INSERT INTO profile_duplicate_owner_bindings VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET binding=excluded.binding",rusqlite::params![id,serde_json::to_string(binding).map_err(|_|"profile_duplicate_binding_encode_failed")?]).map_err(|_|"profile_duplicate_binding_write_failed")?;
    Ok(())
}
struct HostedDuplication<'a, P, E> {
    host: &'a mut BrowserSessionHost<P, E>,
    database: PathBuf,
    binding: OwnerBinding,
    connection: rusqlite::Connection,
    operation_id: String,
    guard: Rc<RefCell<Option<profile_duplicate_custody::SourceCustody>>>,
}
impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    HostedDuplication<'_, P, E>
{
    fn ready_receipt(
        &mut self,
        task_id: String,
        browser_id: String,
        handoff_id: String,
    ) -> Result<LaunchReceipt, String> {
        let session = self
            .host
            .state
            .sessions
            .get(&task_id)
            .cloned()
            .ok_or("profile_duplicate_destination_task_missing")?;
        let tab = session
            .current_tab_id
            .as_ref()
            .and_then(|id| self.host.state.tabs.get(id))
            .cloned()
            .ok_or("profile_duplicate_destination_tab_missing")?;
        let browser = self
            .host
            .state
            .browsers
            .get(&browser_id)
            .cloned()
            .ok_or("profile_duplicate_destination_browser_missing")?;
        if self.binding.target_build.is_some() || self.binding.target_mode.is_some() {
            let identity = BrowserSessionSqliteStore::open_read_only(&self.database)?
                .published_launch_process_identity(&browser)
                .map_err(|_| "browser_build_destination_identity_unavailable")?
                .ok_or("browser_build_destination_identity_unavailable")?;
            if let Some(build) = &self.binding.target_build {
                if identity.executable_path.as_deref() != Some(build.path.as_str()) {
                    return Err("browser_build_loaded_path_mismatch".into());
                }
                #[cfg(target_os = "linux")]
                if Some(executable_digest(Path::new(&format!(
                    "/proc/{}/exe",
                    identity.pid
                )))?)
                    != build.sha256
                {
                    return Err("browser_build_loaded_bytes_mismatch".into());
                }
                #[cfg(not(target_os = "linux"))]
                {
                    return Err(
                        "browser_build_loaded_bytes_unverified:platform qualification required"
                            .into(),
                    );
                }
            }
            if let Some(mode) = &self.binding.target_mode {
                let proof = super::super::browser_session_store::mode_inspection::inspect(
                    &browser,
                    Some(&identity),
                    "managed",
                    crate::process_identity::observe_process,
                );
                if proof["mode"] != mode.as_str() {
                    return Err("browser_build_destination_mode_unverified".into());
                }
            }
        }
        let target = RemoteViewTabHandoffTarget {
            session,
            tab,
            browser,
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            match self
                .host
                .native_browser_command(&target, &json!({"action":"native_ready"}))
            {
                Ok(observation) if observation["ready"] == true => {
                    return Ok(LaunchReceipt {
                        task_id,
                        browser_id,
                        handoff_id,
                    })
                }
                Ok(_) => return Err("profile_duplicate_native_readiness_unverified".into()),
                Err(error)
                    if error.starts_with("browser_native_window_unverified")
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(100))
                }
                Err(error) => return Err(error),
            }
        }
    }
}
impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    DuplicationRuntime for HostedDuplication<'_, P, E>
{
    type Guard = Rc<RefCell<Option<profile_duplicate_custody::SourceCustody>>>;
    fn inspect_source(&mut self, source: &SourceProfile) -> Result<SourceObservation, String> {
        // Reconcile a positively acknowledged close without dispatching again.
        // The callback skips Native input once this private receipt is retained;
        // it only reaps/observes the original and retires its logical records.
        if self.binding.close_dispatched
            && self
                .host
                .state
                .browsers
                .contains_key(&self.binding.browser.id)
        {
            let operation_id = self.operation_id.clone();
            self.graceful_close_source(source, &operation_id)?;
        }
        Ok(SourceObservation {
            generation: source.generation,
            running: self
                .host
                .state
                .browsers
                .values()
                .any(|b| b.profile_id == source.id),
            affected_task_ids: self
                .host
                .state
                .sessions
                .values()
                .filter(|task| task.profile_id == source.id)
                .map(|task| task.id.clone())
                .collect(),
        })
    }
    fn acquire_stopped_source(&mut self, source: &SourceProfile) -> Result<Self::Guard, String> {
        use profile_duplicate_custody::*;
        if !self.binding.clean_close {
            return Err("profile_duplicate_clean_close_evidence_required".into());
        }
        let evidence = BackendGracefulCloseEvidence {
            owner_operation_id: self.operation_id.clone(),
            source_profile_id: source.id.clone(),
            source_generation: source.generation,
            original: self.binding.original.clone(),
            polite_close_attempted: true,
            polite_close_succeeded: true,
            polite_close_failed: false,
            exact_process_exited: true,
            profile_lock_released: source_locks_released(&source.user_data_dir),
            force_kill_attempted: false,
        };
        let observe = |pid| match crate::process_identity::observe_process(pid) {
            crate::process_identity::ProcessObservation::Missing => OriginalProcessState::Absent,
            crate::process_identity::ProcessObservation::Observed(_) => OriginalProcessState::Live,
            _ => OriginalProcessState::Unknown,
        };
        let guard = if let Some(anchor) = self.binding.borrowed_anchor.as_ref() {
            SourceCustody::acquire_with_borrowed_anchor(
                &self.database,
                source,
                &evidence,
                anchor,
                observe,
            )?
        } else {
            SourceCustody::acquire(&self.database, source, &evidence, observe)?
        };
        *self.guard.borrow_mut() = Some(guard);
        Ok(self.guard.clone())
    }
    fn focus_source(&mut self, _: &SourceProfile) -> Result<(), String> {
        self.host
            .native_browser_command(&self.binding.target(), &json!({"action":"native_focus"}))?;
        Ok(())
    }
    fn graceful_close_source(
        &mut self,
        source: &SourceProfile,
        _: &str,
    ) -> Result<CloseOutcome, String> {
        let tasks: Vec<String> = self
            .host
            .state
            .sessions
            .values()
            .filter(|task| task.profile_id == source.id)
            .map(|task| task.id.clone())
            .collect();
        if tasks != self.binding.reviewed_tasks {
            self.binding.reviewed_tasks = tasks;
            save_binding(&self.connection, &self.operation_id, &self.binding)?;
            return Err("profile_duplicate_affected_tasks_changed:review again".into());
        }
        if self
            .host
            .state
            .profile_operation_ids
            .get(&source.id)
            .is_some_and(|id| id != &self.operation_id)
        {
            return Err("profile_duplicate_source_operation_busy".into());
        }
        if !self.binding.close_dispatched {
            self.host
                .state
                .profile_operation_ids
                .insert(source.id.clone(), self.operation_id.clone());
            self.host.commit_state()?;
            let dispatch = self.host.native_browser_command(
                &self.binding.target(),
                &json!({"action":"native_close"}),
            )?;
            if !matches!(dispatch["delivery"].as_str(), Some("dispatched")) {
                return Err("profile_duplicate_close_dispatch_unconfirmed".into());
            }
            self.binding.close_dispatched = true;
            save_binding(&self.connection, &self.operation_id, &self.binding)?;
        }
        for _ in 0..20 {
            let _ = self.host.effects.browser_is_live(&self.binding.browser)?;
            if matches!(
                crate::process_identity::observe_process(self.binding.original.pid),
                crate::process_identity::ProcessObservation::Missing
            ) && source_locks_released(&source.user_data_dir)
            {
                // Exit can race the first child poll. Reap a retained owned
                // child after the independent kernel/lock readback as well.
                let _ = self.host.effects.browser_is_live(&self.binding.browser)?;
                self.binding.clean_close = true;
                save_binding(&self.connection, &self.operation_id, &self.binding)?;
                self.host
                    .state
                    .idle_closed_browsers
                    .insert(self.binding.browser.id.clone(), current_unix_ms());
                self.host
                    .state
                    .shared_browser_ids
                    .remove(&self.binding.browser.id);
                // The durable snapshot stays fenced throughout retirement.
                // Only this local owner projection omits its own fence so the
                // ordinary competing-client close guard cannot veto the owner.
                let before_retirement = self.host.state.clone();
                self.host.state.profile_operation_ids.remove(&source.id);
                if self.binding.borrowed_anchor.is_some() {
                    self.host
                        .state
                        .app_browser_attachments
                        .remove(&self.binding.browser.id);
                    for id in &self.binding.reviewed_tasks {
                        self.host.state.attached_task_targets.remove(id);
                    }
                }
                let retirement = (|| {
                    for id in &self.binding.reviewed_tasks {
                        self.host.manager()?.close_session(
                            id,
                            SessionEndReason::ExplicitClose,
                            current_unix_ms(),
                        )?;
                    }
                    Ok::<(), String>(())
                })();
                self.host
                    .state
                    .profile_operation_ids
                    .insert(source.id.clone(), self.operation_id.clone());
                if let Err(error) = retirement {
                    self.host.state = before_retirement;
                    return Err(error);
                }
                self.host.commit_state()?;
                return Ok(CloseOutcome::Stopped);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Ok(CloseOutcome::Pending)
    }
    fn publish_profile(&mut self, profile: &DuplicateProfile) -> Result<(), String> {
        let configured = self.binding.target_build.is_some() || self.binding.target_mode.is_some();
        let executable = self
            .binding
            .target_build
            .as_ref()
            .map(|build| build.path.as_str())
            .unwrap_or(&self.binding.executable);
        let configuration =
            configured.then_some(profile_duplicate_custody::DestinationLaunchConfiguration {
                executable,
                mode: self.binding.target_mode.as_deref(),
            });
        if let Some(guard) = self.guard.borrow_mut().as_mut() {
            guard.publish_and_release(profile, configuration)?;
        } else {
            let journal = DuplicationStore::open(
                &self
                    .database
                    .parent()
                    .ok_or("profile_duplicate_runtime_directory_missing")?
                    .join("profile-duplication/operations.sqlite3"),
            )?;
            let operation = journal
                .read(&profile.operation_id)?
                .ok_or("profile_duplicate_not_found")?;
            profile_duplicate_custody::publish_recovered_copy(
                &self.database,
                &operation,
                configuration,
            )?;
        }
        self.host.catalog = self
            .host
            .persistence
            .load_or_import_profile_catalog(Path::new(""))?
            .catalog;
        self.host.state = self.host.persistence.load_session_state()?;
        self.host.persisted_state = self.host.state.clone();
        self.host
            .state
            .profile_operation_ids
            .remove(&profile.source_profile_id);
        if !configured {
            self.host.state.profile_executables.insert(
                profile.destination.id.clone(),
                self.binding.executable.clone(),
            );
        }
        self.host.commit_state()?;
        Ok(())
    }
    fn launch_duplicate(&mut self, profile: &DuplicateProfile) -> Result<LaunchReceipt, String> {
        let executable = if let Some(build) = &self.binding.target_build {
            verify_build(&build.path, build.sha256.as_deref())?;
            if self
                .host
                .state
                .profile_executables
                .get(&profile.destination.id)
                != Some(&build.path)
            {
                return Err("browser_build_destination_configuration_changed:inspect the copy or finish without opening".into());
            }
            build.path.clone()
        } else {
            if executable_digest(Path::new(&self.binding.executable))? != self.binding.digest {
                return Err("profile_duplicate_build_changed".into());
            }
            self.binding.executable.clone()
        };
        self.host
            .effects
            .select_profile_browser_executable(profile.destination.id.clone(), executable)?;
        let task = self.host.open(OpenBrowserSession::exact_profile(
            duplicate_session_name(&profile.operation_id, false),
            &profile.destination.id,
            current_unix_ms(),
        ))?;
        let tab = self
            .host
            .tab_for_navigation(&task.session_id, current_unix_ms())?
            .tab_id;
        let handoff_id = uuid::Uuid::new_v4().to_string();
        agent_browser_service_model::retain_remote_view_tab_handoff_at(
            &mut self.host.state,
            &handoff_id,
            &task.session_id,
            &tab,
            current_unix_ms(),
        )?;
        self.host.commit_state()?;
        self.ready_receipt(task.session_id, task.browser_id, handoff_id)
    }
    fn destination_task(
        &self,
        profile: &DuplicateProfile,
    ) -> Result<Option<DestinationTask>, String> {
        let matches = self
            .host
            .state
            .sessions
            .values()
            .filter(|task| {
                task.profile_id == profile.destination.id
                    && task.name == duplicate_session_name(&profile.operation_id, false)
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Ok(None),
            [task] => Ok(Some(DestinationTask {
                task_id: task.id.clone(),
                browser_id: task.browser_id.clone(),
            })),
            _ => Err("profile_duplicate_destination_task_ambiguous".into()),
        }
    }
    fn prove_launch_absent(&mut self, profile: &DuplicateProfile) -> Result<(), String> {
        if self
            .host
            .state
            .browsers
            .values()
            .any(|browser| browser.profile_id == profile.destination.id)
            || self
                .host
                .state
                .sessions
                .values()
                .any(|task| task.profile_id == profile.destination.id)
        {
            return Err("profile_duplicate_destination_present".into());
        }
        let destination = self
            .host
            .catalog
            .profiles
            .get(&profile.destination.id)
            .ok_or("profile_duplicate_destination_profile_missing")?;
        super::super::browser_session_runtime::prove_profile_launch_absence(destination)?;
        BrowserSessionSqliteStore::open(
            self.database
                .parent()
                .ok_or("profile_duplicate_runtime_directory_missing")?,
        )?
        .settle_unpublished_profile_launch_absence(destination, &self.host.state, None)
        .map_err(|_| "profile_duplicate_launch_custody_unresolved".to_string())
    }
    fn reconcile_launch(
        &mut self,
        profile: &DuplicateProfile,
    ) -> Result<Option<LaunchReceipt>, String> {
        let Some(task) = self
            .host
            .state
            .sessions
            .values()
            .find(|task| {
                task.profile_id == profile.destination.id
                    && task.name == duplicate_session_name(&profile.operation_id, false)
            })
            .cloned()
        else {
            // An observed but unpublished launch is resumed through the ordinary
            // owner join. The runtime keeps an unresolved claim fail-closed, so
            // this cannot fall through to an implicit replacement launch.
            use agent_browser_service_model::BrowserLaunchCustodyStore;
            let mut store = BrowserSessionSqliteStore::open_read_only(&self.database)?;
            let claims = store
                .unpublished_launch_records()
                .map_err(|_| "profile_duplicate_launch_readback_required")?;
            let observed = claims
                .iter()
                .filter(|record| {
                    record.intent.profile_id == profile.destination.id
                        && record.observed_process_identity.is_some()
                        && record.observed_browser_id.is_some()
                })
                .collect::<Vec<_>>();
            if observed.is_empty() {
                return Ok(None);
            }
            if observed.len() != 1 {
                return Err("profile_duplicate_launch_ambiguous".into());
            }
            return self.launch_duplicate(profile).map(Some);
        };
        if let Some(link) = self
            .host
            .state
            .remote_view_tab_handoffs
            .values()
            .find(|link| {
                link.session_id == task.id && link.check_link_at(current_unix_ms()).is_ok()
            })
        {
            let handoff_id = link.id.clone();
            return self
                .ready_receipt(task.id, task.browser_id, handoff_id)
                .map(Some);
        }
        // The browser/task may have published before its reply or handoff was
        // retained. Complete that same task; never issue another browser launch.
        let tab = self
            .host
            .tab_for_navigation(&task.id, current_unix_ms())?
            .tab_id;
        let handoff_id = uuid::Uuid::new_v4().to_string();
        agent_browser_service_model::retain_remote_view_tab_handoff_at(
            &mut self.host.state,
            &handoff_id,
            &task.id,
            &tab,
            current_unix_ms(),
        )?;
        self.host.commit_state()?;
        self.ready_receipt(task.id, task.browser_id, handoff_id)
            .map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::browser_session_runtime::BrowserRuntimeDriver;
    use agent_browser_service_model::*;
    struct NoRuntime {
        launches: Option<Rc<std::cell::Cell<u32>>>,
    }
    impl BrowserRuntimeDriver for NoRuntime {
        fn select_profile_browser_executable(
            &mut self,
            _: String,
            _: String,
        ) -> Result<(), String> {
            if self.launches.is_some() {
                Ok(())
            } else {
                Err("unexpected build selection".into())
            }
        }
        fn browser_is_live(&mut self, _: &ManagedBrowserInstance) -> Result<bool, String> {
            if self.launches.is_some() {
                Ok(true)
            } else {
                Err("unexpected runtime effect".into())
            }
        }
        fn launch_browser(
            &mut self,
            _: &BrowserProfileCatalogEntry,
            _: Option<&RemoteViewFixedDesktop>,
        ) -> Result<BrowserLaunch, String> {
            let launches = self.launches.as_ref().ok_or("unexpected launch")?;
            launches.set(launches.get() + 1);
            Ok(BrowserLaunch {
                process_identity: None,
                browser_id: "independent-browser".into(),
                pid: 4242,
                cdp_endpoint: "ws://127.0.0.1:9422/devtools/browser/synthetic".into(),
                desktop: None,
            })
        }
        fn close_browser(&mut self, _: &ManagedBrowserInstance) -> Result<(), String> {
            Err("unexpected close".into())
        }
        fn acquire_initial_tab(
            &mut self,
            _: &ManagedBrowserInstance,
            _: &[String],
        ) -> Result<BrowserTabAcquisition, String> {
            if self.launches.is_some() {
                Ok(BrowserTabAcquisition {
                    tab_id: "independent-tab".into(),
                    target_id: "independent-target".into(),
                    source: BrowserTabSource::Bootstrap,
                })
            } else {
                Err("unexpected tab".into())
            }
        }
        fn create_tab(
            &mut self,
            _: &ManagedBrowserInstance,
        ) -> Result<BrowserTabAcquisition, String> {
            Err("unexpected tab".into())
        }
        fn close_tab(
            &mut self,
            _: &ManagedBrowserInstance,
            _: &ManagedBrowserTab,
        ) -> Result<(), String> {
            Err("unexpected close".into())
        }
        fn navigate(
            &mut self,
            _: &ManagedBrowserInstance,
            _: &ManagedBrowserTab,
            _: &str,
        ) -> Result<(), String> {
            Err("unexpected navigate".into())
        }
        fn focus_browser(
            &mut self,
            _: &ManagedBrowserInstance,
            _: Option<&ManagedBrowserTab>,
        ) -> Result<(), String> {
            Err("unexpected focus".into())
        }
    }
    #[test]
    fn profile_duplicate_host_offline_source_offers_executable_preparation_without_guessing_clean_stop(
    ) {
        let root = std::env::temp_dir().join(format!("duplicate-host-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("source")).unwrap();
        let store = BrowserSessionSqliteStore::launch_custody_fixture(
            rusqlite::Connection::open(root.join("runtime.sqlite3")).unwrap(),
            true,
        )
        .unwrap();
        store
            .save_session_state(&BrowserSessionState::default())
            .unwrap();
        let catalog:BrowserProfileCatalog=serde_json::from_value(serde_json::json!({"schemaVersion":"agent-browser.browser-profile-catalog.v1","profiles":{"source":{"id":"source","name":"Source","userDataDir":root.join("source"),"kind":"named"}},"disposablePolicies":{}})).unwrap();
        store.save_profile_catalog(&catalog).unwrap();
        let mut host = BrowserSessionHost::load(
            store,
            BrowserSessionEffectAdapter::new(NoRuntime { launches: None }),
            Path::new(""),
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 60000,
                remote_view_desktops: vec![],
                default_disposable_policy: None,
                exact_url_history_maximum_bytes: 1024,
            },
        )
        .unwrap();
        let result=host.profile_duplicate(&serde_json::json!({"operationId":"copy-one","sourceProfileId":"source","destinationName":"Independent","launch":false})).unwrap();
        assert_eq!(result["state"], "source_preparation_required");
        assert_eq!(result["actions"][0]["action"], "open_source");
        assert!(host.state().browsers.is_empty());
        assert!(host.state().sessions.is_empty());
        host.profile_duplicate(&json!({"operationId":"copy-one","choice":"cancel"}))
            .unwrap();
        assert_eq!(
            host.profile_duplicate(&json!({"operationId":"copy-one"}))
                .unwrap()["state"],
            "cancelled"
        );
        drop(host);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn profile_duplicate_host_reviews_current_peer_tasks_and_cancels_durably() {
        let root =
            std::env::temp_dir().join(format!("duplicate-host-review-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("source")).unwrap();
        let store = BrowserSessionSqliteStore::launch_custody_fixture(
            rusqlite::Connection::open(root.join("runtime.sqlite3")).unwrap(),
            true,
        )
        .unwrap();
        store
            .save_session_state(&BrowserSessionState::default())
            .unwrap();
        let catalog:BrowserProfileCatalog=serde_json::from_value(json!({"schemaVersion":"agent-browser.browser-profile-catalog.v1","profiles":{"source":{"id":"source","name":"Source","userDataDir":root.join("source"),"kind":"named"}},"disposablePolicies":{}})).unwrap();
        store.save_profile_catalog(&catalog).unwrap();
        let mut host = BrowserSessionHost::load(
            store,
            BrowserSessionEffectAdapter::new(NoRuntime { launches: None }),
            Path::new(""),
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 60000,
                remote_view_desktops: vec![],
                default_disposable_policy: None,
                exact_url_history_maximum_bytes: 1024,
            },
        )
        .unwrap();
        host.profile_duplicate(
            &json!({"operationId":"review","sourceProfileId":"source","launch":false}),
        )
        .unwrap();
        let browser = ManagedBrowserInstance {
            id: "browser".into(),
            profile_id: "source".into(),
            pid: std::process::id(),
            cdp_endpoint: String::new(),
            desktop: None,
            active_session_ids: vec!["task".into()],
        };
        let session = ManagedBrowserSession {
            id: "task".into(),
            name: "Source work".into(),
            profile_id: "source".into(),
            browser_id: browser.id.clone(),
            created_at_ms: 1,
            last_activity_at_ms: 1,
            expires_at_ms: u64::MAX,
            current_tab_id: Some("tab".into()),
        };
        let tab = ManagedBrowserTab {
            id: "tab".into(),
            target_id: "target".into(),
            browser_id: browser.id.clone(),
            session_id: session.id.clone(),
            created_at_ms: 1,
            last_activity_at_ms: 1,
        };
        host.state
            .browsers
            .insert(browser.id.clone(), browser.clone());
        host.state
            .sessions
            .insert(session.id.clone(), session.clone());
        host.state.tabs.insert(tab.id.clone(), tab.clone());
        host.persistence.save_session_state(&host.state).unwrap();
        host.persisted_state = host.state.clone();
        let connection =
            rusqlite::Connection::open(root.join("profile-duplication/operations.sqlite3"))
                .unwrap();
        let binding = OwnerBinding {
            source_observation: None,
            borrowed_anchor: None,
            original: RecordedProcessIdentity {
                pid: browser.pid,
                start_token: "synthetic-review-only".into(),
                executable_path: Some("/synthetic/build".into()),
                browser_family: None,
            },
            browser,
            session,
            tab,
            executable: "/synthetic/build".into(),
            digest: "review-only".into(),
            target_build: None,
            target_mode: None,
            close_dispatched: false,
            clean_close: false,
            reviewed_tasks: vec!["task".into()],
        };
        save_binding(&connection, "review", &binding).unwrap();
        // Owner journal fixture: historical process observation, not next-launch intent.
        let source_observation = json!({"mode":"cdp_free","observedAtMs":17,"build":{"name":"Chromium","version":"130.7","sha256":"review-only"}});
        let mut frozen = serde_json::to_value(&binding).unwrap();
        frozen["source_observation"] = source_observation.clone();
        connection
            .execute(
                "UPDATE profile_duplicate_owner_bindings SET binding=?1 WHERE id='review'",
                [frozen.to_string()],
            )
            .unwrap();
        let review = host
            .profile_duplicate(&json!({"operationId":"review"}))
            .unwrap();
        assert_eq!(review["sourceObservation"], source_observation);
        let lookup = host.operation_request_readback(&json!({"action":"browser_session_profile_duplicate","choice":"inspect","sourceProfileId":"source"})).unwrap().unwrap();
        assert_eq!(
            lookup["operations"][0]["sourceObservation"],
            source_observation
        );
        assert_eq!(review["phase"], "awaiting_owner_choice");
        assert_eq!(review["affectedTaskIds"], json!(["task"]));
        assert_eq!(review["actions"][1]["action"], "close_and_duplicate");
        let mut peer = binding.session.clone();
        peer.id = "peer".into();
        peer.current_tab_id = None;
        host.state.sessions.insert(peer.id.clone(), peer);
        host.state
            .browsers
            .get_mut("browser")
            .unwrap()
            .active_session_ids
            .push("peer".into());
        host.persistence.save_session_state(&host.state).unwrap();
        host.persisted_state = host.state.clone();
        let changed=host.profile_duplicate(&json!({"operationId":"review","choice":"close_and_duplicate","ownerConfirmed":true})).unwrap();
        assert_eq!(
            changed["failure"],
            "profile_duplicate_close_readback_required"
        );
        assert_eq!(changed["affectedTaskIds"], json!(["peer", "task"]));
        host.state
            .profile_operation_ids
            .insert("source".into(), "review".into());
        host.persistence.save_session_state(&host.state).unwrap();
        host.persisted_state = host.state.clone();
        assert_eq!(
            host.profile_duplicate(&json!({"operationId":"review","choice":"cancel"}))
                .unwrap()["phase"],
            "cancelled"
        );
        assert!(!host.state.profile_operation_ids.contains_key("source"));
        assert_eq!(
            host.profile_duplicate(&json!({"operationId":"review"}))
                .unwrap()["phase"],
            "cancelled"
        );
        drop(host);
        drop(connection);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn profile_duplicate_launch_retains_bootstrap_and_reconciles_without_fabricating_readiness() {
        let root =
            std::env::temp_dir().join(format!("duplicate-host-launch-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("destination")).unwrap();
        std::fs::write(root.join("build"), b"synthetic build identity").unwrap();
        let store = BrowserSessionSqliteStore::launch_custody_fixture(
            rusqlite::Connection::open(root.join("runtime.sqlite3")).unwrap(),
            true,
        )
        .unwrap();
        store
            .save_session_state(&BrowserSessionState::default())
            .unwrap();
        let catalog:BrowserProfileCatalog=serde_json::from_value(json!({"schemaVersion":"agent-browser.browser-profile-catalog.v1","profiles":{"destination":{"id":"destination","name":"Independent","userDataDir":root.join("destination"),"kind":"named"}},"disposablePolicies":{}})).unwrap();
        store.save_profile_catalog(&catalog).unwrap();
        let launches = Rc::new(std::cell::Cell::new(0));
        let mut host = BrowserSessionHost::load(
            store,
            BrowserSessionEffectAdapter::new(NoRuntime {
                launches: Some(launches.clone()),
            }),
            Path::new(""),
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 60000,
                remote_view_desktops: vec![],
                default_disposable_policy: None,
                exact_url_history_maximum_bytes: 1024,
            },
        )
        .unwrap();
        let binding:OwnerBinding=serde_json::from_value(json!({"original":{"pid":42,"startToken":"synthetic","executablePath":"/synthetic"},"browser":{"id":"source-browser","profileId":"source","pid":42,"cdpEndpoint":"","activeSessionIds":["source-task"]},"session":{"id":"source-task","name":"Source","profileId":"source","browserId":"source-browser","createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":9999,"currentTabId":"source-tab"},"tab":{"id":"source-tab","targetId":"source-target","browserId":"source-browser","sessionId":"source-task","createdAtMs":1,"lastActivityAtMs":1},"executable":root.join("build"),"digest":executable_digest(&root.join("build")).unwrap(),"close_dispatched":true,"clean_close":true,"reviewed_tasks":[]})).unwrap();
        let profile = DuplicateProfile {
            destination: DestinationProfile {
                id: "destination".into(),
                name: "Independent".into(),
                user_data_dir: root.join("destination"),
            },
            source_profile_id: "source".into(),
            operation_id: "launch-resume".into(),
            browser_build: binding.digest.clone(),
        };
        let mut runtime = HostedDuplication {
            host: &mut host,
            database: root.join("runtime.sqlite3"),
            binding,
            connection: rusqlite::Connection::open_in_memory().unwrap(),
            operation_id: "launch-resume".into(),
            guard: Rc::new(RefCell::new(None)),
        };
        let first_error = runtime.launch_duplicate(&profile).unwrap_err();
        assert!(
            first_error.starts_with("browser_native_original_identity_"),
            "unexpected readiness failure: {first_error}"
        );
        assert_eq!(launches.get(), 1);
        let task = runtime
            .host
            .state
            .sessions
            .values()
            .find(|task| task.profile_id == "destination")
            .unwrap()
            .clone();
        assert!(
            task.current_tab_id.is_some(),
            "bootstrap tab must precede readiness qualification"
        );
        let original_handoff = runtime
            .host
            .state
            .remote_view_tab_handoffs
            .keys()
            .next()
            .unwrap()
            .clone();
        runtime.host.state.remote_view_tab_handoffs.clear();
        runtime.host.commit_state().unwrap();
        let resumed_error = runtime.reconcile_launch(&profile).unwrap_err();
        assert!(resumed_error.starts_with("browser_native_original_identity_"));
        let resumed_handoff = runtime
            .host
            .state
            .remote_view_tab_handoffs
            .keys()
            .next()
            .unwrap()
            .clone();
        assert_ne!(resumed_handoff, original_handoff);
        assert_eq!(
            runtime
                .host
                .state
                .sessions
                .values()
                .find(|current| current.profile_id == "destination")
                .unwrap()
                .id,
            task.id
        );
        assert_eq!(launches.get(), 1);
        assert!(runtime
            .reconcile_launch(&profile)
            .unwrap_err()
            .starts_with("browser_native_original_identity_"));
        assert_eq!(
            runtime.host.state.remote_view_tab_handoffs.keys().next(),
            Some(&resumed_handoff)
        );
        assert_eq!(launches.get(), 1);
        drop(runtime);
        drop(host);
        std::fs::remove_dir_all(root).unwrap();
    }
}
