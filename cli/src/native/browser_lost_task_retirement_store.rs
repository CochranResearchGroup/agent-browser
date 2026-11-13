//! Owning-store boundary for exact cross-boot retirement. Provider cleanup and
//! credential authority are independent of the local terminal receipt.
use super::BrowserSessionSqliteStore;
use agent_browser_service_model::{
    BrowserProfileCatalogEntry, BrowserTaskOperation, LostTaskRetirementObservation,
    LostTaskRetirementRequest, RecordedProcessIdentity,
};
use serde_json::Value;

#[cfg(target_os = "linux")]
#[path = "browser_lost_task_linux_observer.rs"]
mod linux_observer;

trait OwnerObserver {
    fn control_epoch(&self, browser_id: &str) -> Result<u64, String>;
    fn observe_loss(
        &self,
        profile: &BrowserProfileCatalogEntry,
        original: &RecordedProcessIdentity,
        control_epoch: u64,
    ) -> Result<LostTaskRetirementObservation, String>;
}
fn held() -> String {
    "browser_task_retirement_owner_unverified".into()
}

fn completed_receipt(
    state: &agent_browser_service_model::BrowserSessionState,
    request: &LostTaskRetirementRequest,
) -> Result<Option<BrowserTaskOperation>, String> {
    use sha2::{Digest, Sha256};
    if request.task_id.is_empty()
        || request.task_id.len() > 256
        || !crate::native::browser_task_operations::valid_request_id(&request.operation_id)
        || request.expected_binding_ref.len() != 64
        || !request
            .expected_binding_ref
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
    {
        return Err(held());
    }
    let Some(receipt) = state
        .task_operations
        .get(&request.task_id)
        .and_then(|rows| rows.get(&request.operation_id))
    else {
        return Ok(None);
    };
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(request).map_err(|_| held())?)
    );
    if receipt.task_id != request.task_id
        || receipt.request_id != request.operation_id
        || receipt.action != "lost_task_retirement"
        || receipt.fingerprint != fingerprint
        || receipt.state
            != agent_browser_service_model::BrowserTaskOperationState::OwnerAcknowledged
        || receipt
            .evidence_references
            .get("retirementResult")
            .map(String::as_str)
            != Some("retired_lost")
        || receipt.evidence_references.get("ownerBindingRef") != Some(&request.expected_binding_ref)
    {
        return Err("browser_task_retirement_operation_conflict".into());
    }
    Ok(Some(receipt.clone()))
}

impl BrowserSessionSqliteStore {
    /// Freeze only the owning store's current task while control is reserved.
    #[cfg(target_os = "linux")]
    pub(crate) fn inspect_task_retirement(&self, task_id: &str) -> Result<Value, String> {
        let state = self.load_session_state().map_err(|_| held())?;
        let browser = state
            .sessions
            .get(task_id)
            .ok_or_else(held)?
            .browser_id
            .clone();
        let owner = linux_observer::LinuxOwner::acquire_binding(&browser)?;
        self.inspect_task_retirement_with(task_id, &owner)
    }

    /// No signals or provider cleanup: the guard spans the entire SQLite commit.
    /// Exact completed receipts are read back before any control/kernel access.
    #[cfg(target_os = "linux")]
    pub(crate) fn retire_lost_task(
        &mut self,
        request: &LostTaskRetirementRequest,
        now_ms: u64,
    ) -> Result<BrowserTaskOperation, String> {
        let state = self.load_session_state().map_err(|_| held())?;
        if let Some(receipt) = completed_receipt(&state, request)? {
            return Ok(receipt);
        }
        let browser = state
            .sessions
            .get(&request.task_id)
            .ok_or_else(held)?
            .browser_id
            .clone();
        let owner = linux_observer::LinuxOwner::acquire(&browser)?;
        self.retire_lost_task_with(request, now_ms, &owner)
    }

    fn inspect_task_retirement_with(
        &self,
        task_id: &str,
        observer: &impl OwnerObserver,
    ) -> Result<Value, String> {
        use super::{load_document, BROWSER_SESSION_STATE_SCHEMA_V1, SESSION_STATE_DOCUMENT};
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|_| held())?;
        let state: agent_browser_service_model::BrowserSessionState = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )
        .map_err(|_| held())?;
        let task = state.sessions.get(task_id).ok_or_else(held)?;
        let browser = state.browsers.get(&task.browser_id).ok_or_else(held)?;
        let original = super::super::browser_launch_custody_store::published_process_identity(
            &transaction,
            browser,
        )
        .map_err(|_| held())?
        .ok_or_else(held)?;
        let epoch = observer.control_epoch(&browser.id)?;
        let reference =
            agent_browser_service_model::freeze_task_retirement(&state, task_id, &original, epoch)?;
        if observer.control_epoch(&browser.id)? != epoch {
            return Err(held());
        }
        let result = serde_json::json!({"schemaVersion":"agent-browser.task-retirement-binding.v1",
            "taskId":task.id,"browserId":browser.id,"profileId":task.profile_id,"processPid":browser.pid,
            "controlEpoch":epoch,"ownerBindingRef":reference,"liveObservation":"not_performed","credentialAuthority":"not_granted"});
        transaction.commit().map_err(|_| held())?;
        Ok(result)
    }

    fn retire_lost_task_with(
        &mut self,
        request: &LostTaskRetirementRequest,
        now_ms: u64,
        observer: &impl OwnerObserver,
    ) -> Result<BrowserTaskOperation, String> {
        use super::super::browser_launch_custody_store as custody;
        use super::{
            load_document, save_document, BROWSER_PROFILE_CATALOG_SCHEMA_V1,
            BROWSER_SESSION_STATE_SCHEMA_V1, PROFILE_CATALOG_DOCUMENT, SESSION_STATE_DOCUMENT,
        };
        let transaction = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| held())?;
        let mut state: agent_browser_service_model::BrowserSessionState = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )
        .map_err(|_| held())?;
        // A lost commit acknowledgement is read back before requiring a task or
        // current process. No second browser-close dispatch exists in this path.
        if let Some(receipt) = completed_receipt(&state, request)? {
            transaction.commit().map_err(|_| held())?;
            return Ok(receipt);
        }
        let task = state.sessions.get(&request.task_id).ok_or_else(held)?;
        let browser = state
            .browsers
            .get(&task.browser_id)
            .ok_or_else(held)?
            .clone();
        let catalog: agent_browser_service_model::BrowserProfileCatalog = load_document(
            &transaction,
            PROFILE_CATALOG_DOCUMENT,
            BROWSER_PROFILE_CATALOG_SCHEMA_V1,
        )
        .map_err(|_| held())?;
        let profile = catalog
            .profiles
            .get(&task.profile_id)
            .filter(|p| p.id == task.profile_id)
            .ok_or_else(held)?;
        custody::reject_profile_metadata_mutation(&transaction, &profile.id).map_err(|_| held())?;
        // This lookup is inside the SAME writer transaction as the state edit.
        let original = custody::published_process_identity(&transaction, &browser)
            .map_err(|_| held())?
            .ok_or_else(held)?;
        let epoch = observer.control_epoch(&browser.id)?;
        let observed = observer.observe_loss(profile, &original, epoch)?;
        if observed.original_process != original || observed.control_epoch != epoch {
            return Err(held());
        }
        let previous = state.clone();
        let receipt =
            agent_browser_service_model::retire_lost_task(&mut state, request, &observed, now_ms)?;
        if observer.control_epoch(&browser.id)? != epoch {
            return Err(held());
        }
        // Keep existing independent release/idle-retirement reservations intact.
        super::super::browser_idle_retirement_store::reject_retirement_publication(
            &transaction,
            &previous,
            &state,
            &[],
            &[],
        )
        .map_err(|_| held())?;
        custody::reject_fenced_browser_publication(&transaction, &previous, &state)
            .map_err(|_| held())?;
        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            &state,
        )
        .map_err(|_| held())?;
        transaction
            .commit()
            .map_err(|_| "browser_task_retirement_commit_unverified".to_string())?;
        Ok(receipt)
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use agent_browser_service_model::{
        BrowserLaunch, BrowserLaunchCustodyStore, BrowserLaunchIntent, BrowserProfileCatalog,
        BrowserProfileKind, ManagedBrowserInstance, ManagedBrowserSession, ManagedBrowserTab,
        RemoteViewFixedDesktop,
    };
    use rusqlite::Connection;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    struct SyntheticOwner(RecordedProcessIdentity);
    impl OwnerObserver for SyntheticOwner {
        fn control_epoch(&self, _: &str) -> Result<u64, String> {
            Ok(3)
        }
        fn observe_loss(
            &self,
            profile: &BrowserProfileCatalogEntry,
            original: &RecordedProcessIdentity,
            epoch: u64,
        ) -> Result<LostTaskRetirementObservation, String> {
            assert_eq!(profile.id, "profile-a");
            assert_eq!(original, &self.0);
            Ok(LostTaskRetirementObservation {
                original_process: original.clone(),
                current_boot_id: "00000000-0000-0000-0000-000000000002".into(),
                original_pid_absent: true,
                profile_unoccupied: true,
                control_epoch: epoch,
            })
        }
    }
    fn fixture() -> (Fixture, BrowserSessionSqliteStore, SyntheticOwner) {
        let path =
            std::env::temp_dir().join(format!("auth650-retirement-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut store = BrowserSessionSqliteStore::launch_custody_fixture(
            Connection::open(path.join("runtime.sqlite3")).unwrap(),
            true,
        )
        .unwrap();
        let f: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let intent = BrowserLaunchIntent {
            intent_id: uuid::Uuid::new_v4().to_string(),
            profile_id: "profile-a".into(),
            assignment: serde_json::from_value(f["assignment"].clone()).unwrap(),
        };
        let original = RecordedProcessIdentity {
            pid: 4100,
            start_token: "linux:00000000-0000-0000-0000-000000000001:17".into(),
            executable_path: Some("/synthetic/chromium".into()),
            browser_family: None,
        };
        let initial = store.load_session_state().unwrap();
        store.admit_launch_intent(&intent, &initial).unwrap();
        let launch = BrowserLaunch {
            process_identity: Some(original.clone()),
            browser_id: "browser-a".into(),
            pid: 4100,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: intent.assignment.desktop_id.clone(),
                generation: intent.assignment.generation,
                friendly_route_label: "synthetic".into(),
            }),
        };
        store.observe_launch_intent(&intent, &launch).unwrap();
        let mut state = initial.clone();
        state.browsers.insert(
            "browser-a".into(),
            ManagedBrowserInstance {
                id: "browser-a".into(),
                profile_id: "profile-a".into(),
                pid: 4100,
                cdp_endpoint: launch.cdp_endpoint.clone(),
                desktop: launch.desktop.clone(),
                active_session_ids: vec!["task-a".into()],
            },
        );
        state.sessions.insert(
            "task-a".into(),
            ManagedBrowserSession {
                id: "task-a".into(),
                name: "synthetic".into(),
                profile_id: "profile-a".into(),
                browser_id: "browser-a".into(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
                expires_at_ms: 9000,
                current_tab_id: Some("tab-a".into()),
            },
        );
        state.tabs.insert(
            "tab-a".into(),
            ManagedBrowserTab {
                id: "tab-a".into(),
                target_id: "target-a".into(),
                browser_id: "browser-a".into(),
                session_id: "task-a".into(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
            },
        );
        store
            .publish_launch_intent(&intent, &initial, &state)
            .unwrap();
        let profile = BrowserProfileCatalogEntry {
            id: "profile-a".into(),
            name: "Synthetic".into(),
            user_data_dir: path.to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
            manual_login_url: None,
        };
        store
            .save_profile_catalog(&BrowserProfileCatalog {
                profiles: std::collections::BTreeMap::from([("profile-a".into(), profile)]),
                ..Default::default()
            })
            .unwrap();
        (Fixture(path), store, SyntheticOwner(original))
    }
    #[test]
    fn lost_task_retirement_owner_sqlite_receipt_survives_restart_and_task_removal() {
        let (fixture, mut store, owner) = fixture();
        let inspection = store
            .inspect_task_retirement_with("task-a", &owner)
            .unwrap();
        let request = LostTaskRetirementRequest {
            task_id: "task-a".into(),
            operation_id: "auth-retire-synthetic".into(),
            expected_binding_ref: inspection["ownerBindingRef"].as_str().unwrap().into(),
        };
        let receipt = store.retire_lost_task_with(&request, 100, &owner).unwrap();
        assert_eq!(
            receipt.evidence_references["retirementResult"],
            "retired_lost"
        );
        assert!(!store
            .load_session_state()
            .unwrap()
            .sessions
            .contains_key("task-a"));
        drop(store);
        let mut restarted = BrowserSessionSqliteStore::open(&fixture.0).unwrap();
        let before = restarted.load_session_state().unwrap();
        assert_eq!(
            restarted
                .retire_lost_task_with(&request, 101, &owner)
                .unwrap(),
            receipt
        );
        assert_eq!(restarted.load_session_state().unwrap(), before);
    }
    struct NoProbe;
    impl OwnerObserver for NoProbe {
        fn control_epoch(&self, _: &str) -> Result<u64, String> {
            panic!("completed replay must not consult control")
        }
        fn observe_loss(
            &self,
            _: &BrowserProfileCatalogEntry,
            _: &RecordedProcessIdentity,
            _: u64,
        ) -> Result<LostTaskRetirementObservation, String> {
            panic!("completed replay must not consult kernel")
        }
    }
    struct ChangedEpoch<'a>(&'a SyntheticOwner, std::cell::Cell<u64>);
    impl OwnerObserver for ChangedEpoch<'_> {
        fn control_epoch(&self, _: &str) -> Result<u64, String> {
            let epoch = self.1.get();
            self.1.set(epoch + 1);
            Ok(epoch)
        }
        fn observe_loss(
            &self,
            p: &BrowserProfileCatalogEntry,
            o: &RecordedProcessIdentity,
            e: u64,
        ) -> Result<LostTaskRetirementObservation, String> {
            self.0.observe_loss(p, o, e)
        }
    }
    fn request(
        store: &BrowserSessionSqliteStore,
        owner: &SyntheticOwner,
    ) -> LostTaskRetirementRequest {
        let inspection = store.inspect_task_retirement_with("task-a", owner).unwrap();
        LostTaskRetirementRequest {
            task_id: "task-a".into(),
            operation_id: "auth-retire-synthetic".into(),
            expected_binding_ref: inspection["ownerBindingRef"].as_str().unwrap().into(),
        }
    }
    #[test]
    fn lost_task_retirement_owner_sqlite_refusals_preserve_entire_state() {
        let (_fixture, mut store, owner) = fixture();
        let request = request(&store, &owner);
        let before = store.load_session_state().unwrap();
        let changed = ChangedEpoch(&owner, std::cell::Cell::new(3));
        assert!(store
            .retire_lost_task_with(&request, 100, &changed)
            .is_err());
        assert_eq!(store.load_session_state().unwrap(), before);
        let mut wrong = request.clone();
        wrong.expected_binding_ref = "0".repeat(64);
        assert!(store.retire_lost_task_with(&wrong, 100, &owner).is_err());
        assert_eq!(store.load_session_state().unwrap(), before);
        let mut replaced = before.clone();
        replaced.tabs.get_mut("tab-a").unwrap().target_id = "new-target".into();
        store.save_session_state(&replaced).unwrap();
        assert!(store.retire_lost_task_with(&request, 100, &owner).is_err());
        assert_eq!(store.load_session_state().unwrap(), replaced);
    }
    #[test]
    fn lost_task_retirement_owner_sqlite_write_failure_rolls_back_receipt_and_task() {
        let (_fixture, mut store, owner) = fixture();
        let request = request(&store, &owner);
        let before = store.load_session_state().unwrap();
        // SQLite rejects the document write after model transition, before commit.
        store.connection.execute_batch("CREATE TEMP TRIGGER reject_retirement_write BEFORE UPDATE ON state_documents BEGIN SELECT RAISE(ABORT, 'synthetic-write-refusal'); END;").unwrap();
        assert!(store.retire_lost_task_with(&request, 100, &owner).is_err());
        assert_eq!(store.load_session_state().unwrap(), before);
        store
            .connection
            .execute_batch("DROP TRIGGER reject_retirement_write;")
            .unwrap();
        let receipt = store.retire_lost_task_with(&request, 100, &owner).unwrap();
        let completed = store.load_session_state().unwrap();
        assert_eq!(
            store
                .retire_lost_task_with(&request, 101, &NoProbe)
                .unwrap(),
            receipt
        );
        assert_eq!(store.load_session_state().unwrap(), completed);
        let mut collision = request.clone();
        collision.expected_binding_ref = "0".repeat(64);
        assert!(store
            .retire_lost_task_with(&collision, 102, &NoProbe)
            .is_err());
        assert_eq!(store.load_session_state().unwrap(), completed);
    }
    #[test]
    fn lost_task_retirement_guard_kernel_and_sqlite_compose_without_replay() {
        use std::os::unix::fs::symlink;
        let (fixture, mut store, _synthetic) = fixture();
        std::fs::write(fixture.0.join("control.json"),r#"{"epoch":3,"human":false,"pauseReads":false,"awaitingObservation":false,"actor":"fixture","reason":"fixture","flights":{},"owners":{},"audit":[]}"#).unwrap();
        std::fs::set_permissions(
            fixture.0.join("control.json"),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        std::fs::create_dir(fixture.0.join("proc")).unwrap();
        std::fs::write(
            fixture.0.join("boot"),
            "00000000-0000-0000-0000-000000000002",
        )
        .unwrap();
        std::fs::write(fixture.0.join("hostname"), "synthetic-host").unwrap();
        symlink("synthetic-host-4100", fixture.0.join("SingletonLock")).unwrap();
        let owner = linux_observer::LinuxOwner::fixture("browser-a", &fixture.0).unwrap();
        let inspection = store
            .inspect_task_retirement_with("task-a", &owner)
            .unwrap();
        let request = LostTaskRetirementRequest {
            task_id: "task-a".into(),
            operation_id: "composed-retirement".into(),
            expected_binding_ref: inspection["ownerBindingRef"].as_str().unwrap().into(),
        };
        // The real control reservation excludes a same-epoch concurrent admission.
        assert!(linux_observer::LinuxOwner::fixture("browser-a", &fixture.0).is_err());
        let receipt = store.retire_lost_task_with(&request, 100, &owner).unwrap();
        assert!(linux_observer::LinuxOwner::fixture("browser-a", &fixture.0).is_err());
        drop(owner);
        drop(store);
        std::fs::remove_file(fixture.0.join("control.json")).unwrap();
        let mut restarted = BrowserSessionSqliteStore::open(&fixture.0).unwrap();
        // Public production wrapper resolves the terminal receipt despite absent
        // control and task. It cannot acquire or act on a replacement browser.
        assert_eq!(restarted.retire_lost_task(&request, 101).unwrap(), receipt);
        assert!(!restarted
            .load_session_state()
            .unwrap()
            .sessions
            .contains_key("task-a"));
    }
    #[test]
    fn lost_task_retirement_missing_or_pending_launch_proof_preserves_task() {
        for pending in [false, true] {
            let (_fixture, mut store, owner) = fixture();
            let request = request(&store, &owner);
            let before = store.load_session_state().unwrap();
            if pending {
                let raw: String = store
                    .connection
                    .query_row(
                        "SELECT json FROM state_documents WHERE kind='browser_launch_custody'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                let mut ledger: Value = serde_json::from_str(&raw).unwrap();
                for record in ledger["records"].as_object_mut().unwrap().values_mut() {
                    record["published"] = serde_json::json!(false);
                }
                store
                    .connection
                    .execute(
                        "UPDATE state_documents SET json=?1 WHERE kind='browser_launch_custody'",
                        [ledger.to_string()],
                    )
                    .unwrap();
            } else {
                store
                    .connection
                    .execute(
                        "DELETE FROM state_documents WHERE kind='browser_launch_custody'",
                        [],
                    )
                    .unwrap();
            }
            assert!(store.retire_lost_task_with(&request, 100, &owner).is_err());
            assert_eq!(store.load_session_state().unwrap(), before);
        }
    }
}
