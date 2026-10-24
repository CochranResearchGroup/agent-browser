// The retained pre-adapter intent is a separate compatibility fixture.
#[allow(dead_code)]
#[path = "../src/native/profile_duplicate_operation.rs"]
mod profile_duplicate_operation;
use profile_duplicate_operation::*;

fn intent() -> DuplicateIntent {
    DuplicateIntent {
        operation_id: "copy-1".into(),
        source_profile_id: "source".into(),
        destination_profile_id: "independent".into(),
        browser_build: "chrome-for-testing-123".into(),
        source_generation: 7,
    }
}

#[test]
fn running_source_offers_owner_choices_without_peer_policy_veto() {
    let operation =
        ProfileDuplicateOperation::prepare(intent(), vec!["own-task".into(), "peer-task".into()])
            .unwrap();
    assert_eq!(operation.phase, DuplicatePhase::AwaitingOwnerChoice);
    assert_eq!(operation.affected_task_ids, ["own-task", "peer-task"]);
    assert_eq!(operation.intent.destination_profile_id, "independent");
}

#[path = "../src/native/profile_duplicate_copy.rs"]
mod profile_duplicate_copy;
use profile_duplicate_copy as copy;
struct Runtime;
impl copy::DuplicationRuntime for Runtime {
    type Guard = ();
    fn inspect_source(
        &mut self,
        source: &copy::SourceProfile,
    ) -> Result<copy::SourceObservation, String> {
        Ok(copy::SourceObservation {
            generation: source.generation,
            running: false,
            affected_task_ids: vec![],
        })
    }
    fn acquire_stopped_source(&mut self, _: &copy::SourceProfile) -> Result<(), String> {
        Ok(())
    }
    fn focus_source(&mut self, _: &copy::SourceProfile) -> Result<(), String> {
        panic!("stopped source must not be focused")
    }
    fn graceful_close_source(
        &mut self,
        _: &copy::SourceProfile,
        _: &str,
    ) -> Result<copy::CloseOutcome, String> {
        panic!("stopped source must not be closed again")
    }
    fn publish_profile(&mut self, _: &copy::DuplicateProfile) -> Result<(), String> {
        Ok(())
    }
    fn launch_duplicate(
        &mut self,
        _: &copy::DuplicateProfile,
    ) -> Result<copy::LaunchReceipt, String> {
        Ok(copy::LaunchReceipt {
            task_id: "new-task".into(),
            browser_id: "new-browser".into(),
            handoff_id: "new-handoff".into(),
        })
    }
    fn reconcile_launch(
        &mut self,
        _: &copy::DuplicateProfile,
    ) -> Result<Option<copy::LaunchReceipt>, String> {
        Ok(None)
    }
}
#[test]
fn clean_stopped_profile_copies_privately_and_independently_without_runtime_ownership() {
    let root = std::env::temp_dir().join(format!("duplicate-copy-{}", uuid::Uuid::new_v4()));
    let source = root.join("source");
    let destination = root.join("destination");
    std::fs::create_dir_all(source.join("Default/Network")).unwrap();
    std::fs::write(source.join("Default/Network/Cookies"), b"synthetic-cookie").unwrap();
    std::fs::write(source.join("SingletonLock"), b"must-not-clone").unwrap();
    let request = copy::DuplicateRequest {
        operation_id: "duplicate-1".into(),
        source: copy::SourceProfile {
            id: "source".into(),
            name: "Source".into(),
            user_data_dir: source.clone(),
            browser_build: "chrome-exact-build".into(),
            generation: 1,
        },
        destination: copy::DestinationProfile {
            id: "independent".into(),
            name: "Independent".into(),
            user_data_dir: destination.clone(),
        },
        launch: true,
    };
    let database = root.join("operations.sqlite3");
    let mut store = copy::DuplicationStore::open(&database).unwrap();
    store.prepare(request.clone()).unwrap();
    let completed = store
        .advance("duplicate-1", copy::Choice::Proceed, &mut Runtime)
        .unwrap();
    assert_eq!(completed.phase, copy::Phase::Completed);
    assert_eq!(completed.duplicate.unwrap().source_profile_id, "source");
    assert_eq!(completed.launch.unwrap().browser_id, "new-browser");
    assert_eq!(
        std::fs::read(destination.join("Default/Network/Cookies")).unwrap(),
        b"synthetic-cookie"
    );
    assert!(!destination.join("SingletonLock").exists());
    std::fs::write(
        destination.join("Default/Network/Cookies"),
        b"destination-only",
    )
    .unwrap();
    assert_eq!(
        std::fs::read(source.join("Default/Network/Cookies")).unwrap(),
        b"synthetic-cookie"
    );
    drop(store);
    assert_eq!(
        copy::DuplicationStore::open(&database)
            .unwrap()
            .prepare(request)
            .unwrap()
            .phase,
        copy::Phase::Completed
    );
    std::fs::remove_dir_all(root).unwrap();
}

struct RunningRuntime {
    running: bool,
    focused: bool,
    close_pending: bool,
}
impl copy::DuplicationRuntime for RunningRuntime {
    type Guard = ();
    fn inspect_source(
        &mut self,
        source: &copy::SourceProfile,
    ) -> Result<copy::SourceObservation, String> {
        Ok(copy::SourceObservation {
            generation: source.generation,
            running: self.running,
            affected_task_ids: if self.running {
                vec!["task".into(), "peer".into()]
            } else {
                vec![]
            },
        })
    }
    fn acquire_stopped_source(&mut self, _: &copy::SourceProfile) -> Result<(), String> {
        assert!(!self.running, "copy requires stopped source");
        Ok(())
    }
    fn focus_source(&mut self, _: &copy::SourceProfile) -> Result<(), String> {
        self.focused = true;
        Ok(())
    }
    fn graceful_close_source(
        &mut self,
        _: &copy::SourceProfile,
        _: &str,
    ) -> Result<copy::CloseOutcome, String> {
        if self.close_pending {
            Ok(copy::CloseOutcome::Pending)
        } else {
            self.running = false;
            Ok(copy::CloseOutcome::Stopped)
        }
    }
    fn publish_profile(&mut self, _: &copy::DuplicateProfile) -> Result<(), String> {
        Ok(())
    }
    fn launch_duplicate(
        &mut self,
        _: &copy::DuplicateProfile,
    ) -> Result<copy::LaunchReceipt, String> {
        panic!("test requests no launch")
    }
    fn reconcile_launch(
        &mut self,
        _: &copy::DuplicateProfile,
    ) -> Result<Option<copy::LaunchReceipt>, String> {
        Ok(None)
    }
}
#[test]
fn running_source_can_focus_close_wait_and_resume_same_copy_without_peer_veto() {
    let root = std::env::temp_dir().join(format!("duplicate-running-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("source")).unwrap();
    std::fs::write(root.join("source/Preferences"), b"saved").unwrap();
    let database = root.join("operations.sqlite3");
    let request = copy::DuplicateRequest {
        operation_id: "copy-running".into(),
        source: copy::SourceProfile {
            id: "source".into(),
            name: "Source".into(),
            user_data_dir: root.join("source"),
            browser_build: "same-build".into(),
            generation: 1,
        },
        destination: copy::DestinationProfile {
            id: "new".into(),
            name: "New".into(),
            user_data_dir: root.join("new"),
        },
        launch: false,
    };
    let mut runtime = RunningRuntime {
        running: true,
        focused: false,
        close_pending: true,
    };
    let mut store = copy::DuplicationStore::open(&database).unwrap();
    store.prepare(request).unwrap();
    let review = store
        .advance("copy-running", copy::Choice::Proceed, &mut runtime)
        .unwrap();
    assert_eq!(review.phase, copy::Phase::AwaitingOwnerChoice);
    assert_eq!(review.affected_task_ids, ["task", "peer"]);
    store
        .advance("copy-running", copy::Choice::FocusSource, &mut runtime)
        .unwrap();
    assert!(runtime.focused);
    let pending = store
        .advance(
            "copy-running",
            copy::Choice::CloseAndDuplicate,
            &mut runtime,
        )
        .unwrap();
    assert_eq!(pending.phase, copy::Phase::AwaitingGracefulClose);
    assert!(runtime.close_pending);
    assert!(!root.join("new").exists());
    drop(store);
    runtime.running = false;
    let result = copy::DuplicationStore::open(&database)
        .unwrap()
        .advance("copy-running", copy::Choice::Proceed, &mut runtime)
        .unwrap();
    assert_eq!(result.phase, copy::Phase::Completed);
    assert_eq!(
        std::fs::read(root.join("new/Preferences")).unwrap(),
        b"saved"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn interrupted_copy_never_publishes_partial_destination_and_same_operation_can_resume() {
    let root = std::env::temp_dir().join(format!("duplicate-recovery-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("source/Default")).unwrap();
    std::fs::write(root.join("source/Default/Cookies"), b"synthetic-auth-data").unwrap();
    std::fs::write(root.join("outside"), b"do-not-copy").unwrap();
    std::os::unix::fs::symlink(root.join("outside"), root.join("source/unsafe-link")).unwrap();
    let request = copy::DuplicateRequest {
        operation_id: "resume-copy".into(),
        source: copy::SourceProfile {
            id: "source".into(),
            name: "Source".into(),
            user_data_dir: root.join("source"),
            browser_build: "same".into(),
            generation: 1,
        },
        destination: copy::DestinationProfile {
            id: "duplicate".into(),
            name: "Duplicate".into(),
            user_data_dir: root.join("duplicate"),
        },
        launch: false,
    };
    let database = root.join("operations.sqlite3");
    let mut store = copy::DuplicationStore::open(&database).unwrap();
    store.prepare(request).unwrap();
    assert!(store
        .advance("resume-copy", copy::Choice::Proceed, &mut Runtime)
        .is_err());
    assert!(
        !root.join("duplicate").exists(),
        "partial profile must remain private staging, not published destination"
    );
    assert_eq!(
        std::fs::read(root.join("source/Default/Cookies")).unwrap(),
        b"synthetic-auth-data"
    );
    drop(store);
    std::fs::remove_file(root.join("source/unsafe-link")).unwrap();
    let result = copy::DuplicationStore::open(&database)
        .unwrap()
        .advance("resume-copy", copy::Choice::Reconcile, &mut Runtime)
        .unwrap();
    assert_eq!(result.phase, copy::Phase::Completed);
    assert_eq!(
        std::fs::read(root.join("duplicate/Default/Cookies")).unwrap(),
        b"synthetic-auth-data"
    );
    assert!(!root.join("duplicate/unsafe-link").exists());
    std::fs::remove_dir_all(root).unwrap();
}

struct LostLaunchRuntime {
    launched: bool,
}
impl copy::DuplicationRuntime for LostLaunchRuntime {
    type Guard = ();
    fn inspect_source(
        &mut self,
        source: &copy::SourceProfile,
    ) -> Result<copy::SourceObservation, String> {
        Runtime.inspect_source(source)
    }
    fn acquire_stopped_source(&mut self, source: &copy::SourceProfile) -> Result<(), String> {
        Runtime.acquire_stopped_source(source)
    }
    fn focus_source(&mut self, _: &copy::SourceProfile) -> Result<(), String> {
        panic!("no focus requested")
    }
    fn graceful_close_source(
        &mut self,
        _: &copy::SourceProfile,
        _: &str,
    ) -> Result<copy::CloseOutcome, String> {
        panic!("no close requested")
    }
    fn publish_profile(&mut self, _: &copy::DuplicateProfile) -> Result<(), String> {
        Ok(())
    }
    fn launch_duplicate(
        &mut self,
        _: &copy::DuplicateProfile,
    ) -> Result<copy::LaunchReceipt, String> {
        assert!(!self.launched, "lost launch must not replay");
        self.launched = true;
        Err("PRIVATE-LAUNCH-CANARY /private/profile".into())
    }
    fn reconcile_launch(
        &mut self,
        _: &copy::DuplicateProfile,
    ) -> Result<Option<copy::LaunchReceipt>, String> {
        Ok(self.launched.then(|| copy::LaunchReceipt {
            task_id: "new-task".into(),
            browser_id: "new-browser".into(),
            handoff_id: "new-handoff".into(),
        }))
    }
}
#[test]
fn lost_launch_reply_reconciles_original_operation_without_copy_or_launch_replay() {
    let root = std::env::temp_dir().join(format!("duplicate-lost-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("source")).unwrap();
    std::fs::write(root.join("source/Preferences"), b"initial").unwrap();
    let request = copy::DuplicateRequest {
        operation_id: "lost".into(),
        source: copy::SourceProfile {
            id: "source".into(),
            name: "Source".into(),
            user_data_dir: root.join("source"),
            browser_build: "same".into(),
            generation: 1,
        },
        destination: copy::DestinationProfile {
            id: "duplicate".into(),
            name: "Duplicate".into(),
            user_data_dir: root.join("duplicate"),
        },
        launch: true,
    };
    let db = root.join("operations.sqlite3");
    let mut store = copy::DuplicationStore::open(&db).unwrap();
    store.prepare(request).unwrap();
    let mut runtime = LostLaunchRuntime { launched: false };
    let error = store
        .advance("lost", copy::Choice::Proceed, &mut runtime)
        .unwrap_err();
    assert_eq!(error, "profile_duplicate_launch_readback_required");
    let failed = store.read("lost").unwrap().unwrap();
    assert_eq!(failed.phase, copy::Phase::LaunchDispatched);
    assert_eq!(
        failed.public_view()["failure"],
        "profile_duplicate_launch_readback_required"
    );
    assert!(!failed.public_view().to_string().contains("CANARY"));
    // A running duplicate can legitimately change its data before the lost reply
    // is reconciled. Recovery must not recopy/overwrite its independent writes.
    std::fs::write(root.join("duplicate/Preferences"), b"duplicate-diverged").unwrap();
    drop(store);
    let result = copy::DuplicationStore::open(&db)
        .unwrap()
        .advance("lost", copy::Choice::Reconcile, &mut runtime)
        .unwrap();
    assert_eq!(result.phase, copy::Phase::Completed);
    assert_eq!(result.launch.unwrap().task_id, "new-task");
    assert_eq!(
        std::fs::read(root.join("duplicate/Preferences")).unwrap(),
        b"duplicate-diverged"
    );
    assert_eq!(
        std::fs::read(root.join("source/Preferences")).unwrap(),
        b"initial"
    );
    std::fs::remove_dir_all(root).unwrap();
}

struct ConcurrentRuntime {
    database: std::path::PathBuf,
}
impl copy::DuplicationRuntime for ConcurrentRuntime {
    type Guard = ();
    fn inspect_source(
        &mut self,
        s: &copy::SourceProfile,
    ) -> Result<copy::SourceObservation, String> {
        Runtime.inspect_source(s)
    }
    fn acquire_stopped_source(&mut self, s: &copy::SourceProfile) -> Result<(), String> {
        Runtime.acquire_stopped_source(s)
    }
    fn focus_source(&mut self, _: &copy::SourceProfile) -> Result<(), String> {
        panic!("no focus")
    }
    fn graceful_close_source(
        &mut self,
        _: &copy::SourceProfile,
        _: &str,
    ) -> Result<copy::CloseOutcome, String> {
        panic!("no close")
    }
    fn publish_profile(&mut self, p: &copy::DuplicateProfile) -> Result<(), String> {
        let mut second = copy::DuplicationStore::open(&self.database)?;
        let error = second
            .advance(&p.operation_id, copy::Choice::Reconcile, &mut Runtime)
            .unwrap_err();
        assert!(error.starts_with("profile_duplicate_operation_busy"));
        Ok(())
    }
    fn launch_duplicate(
        &mut self,
        p: &copy::DuplicateProfile,
    ) -> Result<copy::LaunchReceipt, String> {
        Runtime.launch_duplicate(p)
    }
    fn reconcile_launch(
        &mut self,
        p: &copy::DuplicateProfile,
    ) -> Result<Option<copy::LaunchReceipt>, String> {
        Runtime.reconcile_launch(p)
    }
}
#[test]
fn concurrent_clients_cannot_replay_the_same_operation_effect() {
    let root = std::env::temp_dir().join(format!("duplicate-concurrent-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("source")).unwrap();
    let request = copy::DuplicateRequest {
        operation_id: "one-operation".into(),
        source: copy::SourceProfile {
            id: "source".into(),
            name: "Source".into(),
            user_data_dir: root.join("source"),
            browser_build: "same".into(),
            generation: 1,
        },
        destination: copy::DestinationProfile {
            id: "duplicate".into(),
            name: "Duplicate".into(),
            user_data_dir: root.join("duplicate"),
        },
        launch: true,
    };
    let db = root.join("operations.sqlite3");
    let mut store = copy::DuplicationStore::open(&db).unwrap();
    store.prepare(request).unwrap();
    let result = store
        .advance(
            "one-operation",
            copy::Choice::Proceed,
            &mut ConcurrentRuntime { database: db },
        )
        .unwrap();
    assert_eq!(result.phase, copy::Phase::Completed);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn public_operation_projection_exposes_actionable_choices_without_profile_paths_or_auth_claims() {
    let request = copy::DuplicateRequest {
        operation_id: "public-op".into(),
        source: copy::SourceProfile {
            id: "source".into(),
            name: "Source".into(),
            user_data_dir: "/private/CANARY_SOURCE".into(),
            browser_build: "same-build".into(),
            generation: 7,
        },
        destination: copy::DestinationProfile {
            id: "duplicate".into(),
            name: "Duplicate".into(),
            user_data_dir: "/private/CANARY_DESTINATION".into(),
        },
        launch: true,
    };
    let operation = copy::Operation {
        request,
        phase: copy::Phase::AwaitingOwnerChoice,
        affected_task_ids: vec!["own-task".into(), "peer-task".into()],
        duplicate: None,
        launch: None,
        failure: None,
    };
    let public = operation.public_view();
    assert_eq!(public["operationId"], "public-op");
    assert_eq!(public["source"]["id"], "source");
    assert_eq!(public["destination"]["id"], "duplicate");
    assert_eq!(public["authenticationContinuity"], "unverified");
    assert_eq!(public["actions"][1]["action"], "close_and_duplicate");
    assert_eq!(public["actions"][1]["requiresConfirmation"], true);
    assert!(!public.to_string().contains("CANARY"));
    assert!(!public.to_string().contains("/private"));
    let mut partial = operation.clone();
    partial.phase = copy::Phase::Copying;
    partial.failure = Some("profile_duplicate_destination_exists".into());
    assert!(partial.public_view()["actions"].as_array().unwrap().iter().any(|action|action["action"]=="cancel"),"failed staging must expose executable cancellation before selecting a different destination");
}

#[test]
fn destination_collision_can_cancel_without_deleting_foreign_data_and_choose_a_new_destination() {
    let root = std::env::temp_dir().join(format!("duplicate-collision-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("source")).unwrap();
    std::fs::create_dir(root.join("occupied")).unwrap();
    std::fs::write(root.join("occupied/foreign"), b"preserve").unwrap();
    let mut request = copy::DuplicateRequest {
        operation_id: "collision".into(),
        source: copy::SourceProfile {
            id: "source".into(),
            name: "Source".into(),
            user_data_dir: root.join("source"),
            browser_build: "same".into(),
            generation: 1,
        },
        destination: copy::DestinationProfile {
            id: "duplicate".into(),
            name: "Duplicate".into(),
            user_data_dir: root.join("occupied"),
        },
        launch: false,
    };
    let db = root.join("operations.sqlite3");
    let mut store = copy::DuplicationStore::open(&db).unwrap();
    store.prepare(request.clone()).unwrap();
    assert!(store
        .advance("collision", copy::Choice::Proceed, &mut Runtime)
        .is_err());
    let cancelled = store
        .advance("collision", copy::Choice::Cancel, &mut Runtime)
        .unwrap();
    assert_eq!(cancelled.phase, copy::Phase::Cancelled);
    assert_eq!(
        std::fs::read(root.join("occupied/foreign")).unwrap(),
        b"preserve"
    );
    request.operation_id = "replacement-choice".into();
    request.destination.user_data_dir = root.join("new");
    store.prepare(request).unwrap();
    assert_eq!(
        store
            .advance("replacement-choice", copy::Choice::Proceed, &mut Runtime)
            .unwrap()
            .phase,
        copy::Phase::Completed
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[path = "../src/native/profile_duplicate_custody.rs"]
mod profile_duplicate_custody;
use profile_duplicate_custody as custody;
#[test]
fn qualified_graceful_source_custody_fences_launch_writes_and_publishes_on_its_connection() {
    use rusqlite::{params, Connection};
    let root = std::env::temp_dir().join(format!("duplicate-custody-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("source")).unwrap();
    std::fs::create_dir(root.join("duplicate")).unwrap();
    let database = root.join("runtime.sqlite3");
    let db = Connection::open(&database).unwrap();
    db.execute_batch("CREATE TABLE state_documents(kind TEXT PRIMARY KEY,schema_version TEXT NOT NULL,json TEXT NOT NULL);PRAGMA user_version=2;").unwrap();
    let source = copy::SourceProfile {
        id: "source".into(),
        name: "Source".into(),
        user_data_dir: root.join("source"),
        browser_build: "same-build".into(),
        generation: 1,
    };
    let original = agent_browser_service_model::RecordedProcessIdentity {
        pid: 42,
        start_token: "boot:42:original".into(),
        executable_path: Some("/synthetic/chrome".into()),
        browser_family: Some("chrome".into()),
    };
    let assignment: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap();
    let mut record = agent_browser_service_model::BrowserLaunchCustodyRecord::pending(
        agent_browser_service_model::BrowserLaunchIntent {
            intent_id: uuid::Uuid::new_v4().to_string(),
            profile_id: "source".into(),
            assignment: serde_json::from_value(assignment["assignment"].clone()).unwrap(),
        },
    )
    .unwrap();
    record.observed_browser_id = Some("original-browser".into());
    record.observed_pid = Some(42);
    record.observed_process_identity = Some(original.clone());
    record.published = true;
    let catalog = serde_json::json!({"schemaVersion":"agent-browser.browser-profile-catalog.v1","profiles":{"source":{"id":"source","name":"Source","userDataDir":source.user_data_dir,"kind":"named"}},"disposablePolicies":{}});
    for (kind, schema, value) in [
        (
            "browser_profile_catalog",
            "agent-browser.browser-profile-catalog.v1",
            catalog,
        ),
        (
            "browser_session_state",
            "agent-browser.browser-session-state.v1",
            serde_json::to_value(agent_browser_service_model::BrowserSessionState::default())
                .unwrap(),
        ),
        (
            "browser_launch_custody",
            "agent-browser.browser-launch-custody.v1",
            serde_json::json!({"records":{record.intent.intent_id.clone():record},"reconciled_absent":[]}),
        ),
    ] {
        db.execute(
            "INSERT INTO state_documents VALUES(?1,?2,?3)",
            params![kind, schema, value.to_string()],
        )
        .unwrap();
    }
    let evidence = custody::BackendGracefulCloseEvidence {
        owner_operation_id: "actual-native-close-operation".into(),
        source_profile_id: "source".into(),
        source_generation: 1,
        original,
        polite_close_attempted: true,
        polite_close_succeeded: true,
        polite_close_failed: false,
        exact_process_exited: true,
        profile_lock_released: true,
        force_kill_attempted: false,
    };
    let mut guard = custody::SourceCustody::acquire(&database, &source, &evidence, |pid| {
        assert_eq!(pid, 42);
        custody::OriginalProcessState::Absent
    })
    .unwrap();
    db.busy_timeout(std::time::Duration::ZERO).unwrap();
    assert!(
        db.execute_batch("BEGIN IMMEDIATE").is_err(),
        "normal launch claims must not pass the runtime writer fence"
    );
    let duplicate = copy::DuplicateProfile {
        destination: copy::DestinationProfile {
            id: "duplicate".into(),
            name: "Duplicate".into(),
            user_data_dir: root.join("duplicate"),
        },
        source_profile_id: "source".into(),
        operation_id: "dup-op".into(),
        browser_build: "same-build".into(),
    };
    guard.publish_and_release(&duplicate).unwrap();
    db.execute_batch("BEGIN IMMEDIATE;ROLLBACK;").unwrap();
    let catalog: String = db
        .query_row(
            "SELECT json FROM state_documents WHERE kind='browser_profile_catalog'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let catalog: serde_json::Value = serde_json::from_str(&catalog).unwrap();
    assert_eq!(catalog["profiles"]["source"]["name"], "Source");
    assert_eq!(
        catalog["profiles"]["duplicate"]["userDataDir"],
        serde_json::json!(root.join("duplicate"))
    );
    drop(guard);
    let mut forced = evidence.clone();
    forced.force_kill_attempted = true;
    assert!(
        custody::SourceCustody::acquire(&database, &source, &forced, |_| {
            custody::OriginalProcessState::Absent
        })
        .err()
        .unwrap()
        .starts_with("profile_duplicate_clean_close_evidence_required")
    );
    for observation in [
        custody::OriginalProcessState::Live,
        custody::OriginalProcessState::Unknown,
    ] {
        assert!(
            custody::SourceCustody::acquire(&database, &source, &evidence, |_| observation)
                .err()
                .unwrap()
                .starts_with("profile_duplicate_original_exit_readback_required")
        );
    }
    db.execute_batch("BEGIN IMMEDIATE;ROLLBACK;").unwrap();
    // The copy is independent now. Catalog-reply recovery must not require
    // closing its source again or overwrite a subsequently edited label.
    let mut catalog: serde_json::Value = serde_json::from_str(
        &db.query_row::<String, _, _>(
            "SELECT json FROM state_documents WHERE kind='browser_profile_catalog'",
            [],
            |row| row.get(0),
        )
        .unwrap(),
    )
    .unwrap();
    catalog["profiles"]["duplicate"]["name"] = serde_json::json!("Owner edited label");
    db.execute(
        "UPDATE state_documents SET json=?1 WHERE kind='browser_profile_catalog'",
        [catalog.to_string()],
    )
    .unwrap();
    let mut state: serde_json::Value =
        serde_json::to_value(agent_browser_service_model::BrowserSessionState::default()).unwrap();
    state["browsers"]["reopened-source"] = serde_json::json!({"id":"reopened-source","profileId":"source","pid":43,"cdpEndpoint":"ws://synthetic-source","desktop":null,"activeSessionIds":[]});
    db.execute(
        "UPDATE state_documents SET json=?1 WHERE kind='browser_session_state'",
        [state.to_string()],
    )
    .unwrap();
    let operation = copy::Operation {
        request: copy::DuplicateRequest {
            operation_id: "dup-op".into(),
            source: source.clone(),
            destination: duplicate.destination.clone(),
            launch: true,
        },
        phase: copy::Phase::Copied,
        affected_task_ids: vec![],
        duplicate: Some(duplicate),
        launch: None,
        failure: Some("profile_duplicate_catalog_readback_required".into()),
    };
    custody::publish_recovered_copy(&database, &operation).unwrap();
    let catalog: serde_json::Value = serde_json::from_str(
        &db.query_row::<String, _, _>(
            "SELECT json FROM state_documents WHERE kind='browser_profile_catalog'",
            [],
            |row| row.get(0),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        catalog["profiles"]["duplicate"]["name"],
        "Owner edited label"
    );
    drop(db);
    std::fs::remove_dir_all(root).unwrap();
}
