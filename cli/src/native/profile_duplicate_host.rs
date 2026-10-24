//! Hosted profile duplication joins the existing browser owner and private copy
//! journal. Public requests carry intent, never process identity or profile paths.
use super::*;
#[path = "profile_duplicate_copy.rs"]
mod profile_duplicate_copy;
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
            return Ok(
                json!({"schemaVersion":"agent-browser.profile-duplication.v1","operationId":operation_id,"state":"cancelled","phase":"cancelled","actions":[]}),
            );
        }
        if binding_exists {
            return self.advance_profile_duplicate(command, &intent, &directory, &database);
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
            return Ok(
                json!({"schemaVersion":"agent-browser.profile-duplication.v1","operationId":operation_id,"state":"cancelled","phase":"cancelled","actions":[]}),
            );
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
                    format!("profile-duplicate-source:{}", operation_id),
                    &intent.source_profile_id,
                    current_unix_ms(),
                ))?;
                let mut next = command.clone();
                next["choice"] = json!("prepare");
                return self.profile_duplicate(&next);
            }
            return Ok(
                json!({"schemaVersion":"agent-browser.profile-duplication.v1","operationId":operation_id,"state":"source_preparation_required","phase":"source_preparation_required","source":{"id":intent.source_profile_id},"destination":{"id":intent.destination_profile_id,"name":intent.destination_name},"authenticationContinuity":"unverified","actions":[{"action":"open_source","requiresConfirmation":true},{"action":"cancel","requiresConfirmation":false}],"warning":"Open the source with the selected build to establish current clean-close custody before copying."}),
            );
        }
        self.advance_profile_duplicate(command, &intent, &directory, &database)
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct OwnerBinding {
    original: RecordedProcessIdentity,
    browser: agent_browser_service_model::ManagedBrowserInstance,
    session: agent_browser_service_model::ManagedBrowserSession,
    tab: agent_browser_service_model::ManagedBrowserTab,
    executable: String,
    digest: String,
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
                        .published_launch_process_identity(&browser).map_err(|_|"profile_duplicate_original_identity_unavailable")?
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
            let binding = OwnerBinding {
                original,
                browser,
                session,
                tab,
                executable,
                digest,
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
            "cancel" => Choice::Cancel,
            _ => return Err("profile_duplicate_choice_invalid".into()),
        };
        let choice = if first_review && choice == Choice::CloseAndDuplicate { Choice::Reconcile } else { choice };
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
                view["state"] = view["phase"].clone();
                Ok(view)
            }
            Err(error) => {
                if let Some(operation) = journal.read(&intent.operation_id)? {
                    let mut view = operation.public_view();
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
    DuplicationRuntime for HostedDuplication<'_, P, E>
{
    type Guard = Rc<RefCell<Option<profile_duplicate_custody::SourceCustody>>>;
    fn inspect_source(&mut self, source: &SourceProfile) -> Result<SourceObservation, String> {
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
        *self.guard.borrow_mut() = Some(SourceCustody::acquire(
            &self.database,
            source,
            &evidence,
            |pid| match crate::process_identity::observe_process(pid) {
                crate::process_identity::ProcessObservation::Missing => {
                    OriginalProcessState::Absent
                }
                crate::process_identity::ProcessObservation::Observed(_) => {
                    OriginalProcessState::Live
                }
                _ => OriginalProcessState::Unknown,
            },
        )?);
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
                let retirement = (|| {
                    for id in &self.binding.reviewed_tasks {
                        self.host.manager()?.close_session(id, SessionEndReason::ExplicitClose, current_unix_ms())?;
                    }
                    Ok::<(),String>(())
                })();
                self.host.state.profile_operation_ids.insert(source.id.clone(), self.operation_id.clone());
                if let Err(error) = retirement {self.host.state=before_retirement;return Err(error);}
                self.host.commit_state()?;
                return Ok(CloseOutcome::Stopped);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Ok(CloseOutcome::Pending)
    }
    fn publish_profile(&mut self, profile: &DuplicateProfile) -> Result<(), String> {
        if let Some(guard) = self.guard.borrow_mut().as_mut() {
            guard.publish_and_release(profile)?;
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
            profile_duplicate_custody::publish_recovered_copy(&self.database, &operation)?;
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
        self.host.state.profile_executables.insert(
            profile.destination.id.clone(),
            self.binding.executable.clone(),
        );
        self.host.commit_state()?;
        Ok(())
    }
    fn launch_duplicate(&mut self, profile: &DuplicateProfile) -> Result<LaunchReceipt, String> {
        if executable_digest(Path::new(&self.binding.executable))? != self.binding.digest {
            return Err("profile_duplicate_build_changed".into());
        }
        self.host.effects.select_profile_browser_executable(
            profile.destination.id.clone(),
            self.binding.executable.clone(),
        )?;
        let task = self.host.open(OpenBrowserSession::exact_profile(
            format!("profile-duplicate:{}", profile.operation_id),
            &profile.destination.id,
            current_unix_ms(),
        ))?;
        let tab = self
            .host
            .state
            .sessions
            .get(&task.session_id)
            .and_then(|task| task.current_tab_id.clone())
            .ok_or("profile_duplicate_destination_tab_missing")?;
        let handoff_id = uuid::Uuid::new_v4().to_string();
        agent_browser_service_model::retain_remote_view_tab_handoff_at(
            &mut self.host.state,
            &handoff_id,
            &task.session_id,
            &tab,
            current_unix_ms(),
        )?;
        self.host.commit_state()?;
        Ok(LaunchReceipt {
            task_id: task.session_id,
            browser_id: task.browser_id,
            handoff_id,
        })
    }
    fn reconcile_launch(
        &mut self,
        profile: &DuplicateProfile,
    ) -> Result<Option<LaunchReceipt>, String> {
        let Some(task) = self.host.state.sessions.values().find(|task| {
            task.profile_id == profile.destination.id
                && task.name == format!("profile-duplicate:{}", profile.operation_id)
        }) else {
            return Ok(None);
        };
        Ok(self
            .host
            .state
            .remote_view_tab_handoffs
            .values()
            .find(|link| link.session_id == task.id)
            .map(|link| LaunchReceipt {
                task_id: task.id.clone(),
                browser_id: task.browser_id.clone(),
                handoff_id: link.id.clone(),
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::browser_session_runtime::BrowserRuntimeDriver;
    use agent_browser_service_model::*;
    struct NoRuntime;
    impl BrowserRuntimeDriver for NoRuntime {
        fn browser_is_live(&mut self, _: &ManagedBrowserInstance) -> Result<bool, String> {
            Err("unexpected runtime effect".into())
        }
        fn launch_browser(
            &mut self,
            _: &BrowserProfileCatalogEntry,
            _: Option<&RemoteViewFixedDesktop>,
        ) -> Result<BrowserLaunch, String> {
            Err("unexpected launch".into())
        }
        fn close_browser(&mut self, _: &ManagedBrowserInstance) -> Result<(), String> {
            Err("unexpected close".into())
        }
        fn acquire_initial_tab(
            &mut self,
            _: &ManagedBrowserInstance,
            _: &[String],
        ) -> Result<BrowserTabAcquisition, String> {
            Err("unexpected tab".into())
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
            BrowserSessionEffectAdapter::new(NoRuntime),
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
            BrowserSessionEffectAdapter::new(NoRuntime),
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
            close_dispatched: false,
            clean_close: false,
            reviewed_tasks: vec!["task".into()],
        };
        save_binding(&connection, "review", &binding).unwrap();
        let review = host
            .profile_duplicate(&json!({"operationId":"review"}))
            .unwrap();
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
}
