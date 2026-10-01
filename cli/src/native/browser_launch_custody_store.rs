//! Durable process intent before launch and atomic session publication afterward.
use std::collections::BTreeMap;

use agent_browser_service_model::{
    prepare_remote_view_application_cleanup, BrowserLaunch, BrowserLaunchCustodyRecord,
    BrowserLaunchCustodyStore, BrowserLaunchIntent, BrowserReleaseCustodyStore,
    BrowserSessionState, LaunchCustodyAdmission, LaunchCustodyStoreError,
    RemoteViewApplicationCleanupPermit, RemoteViewApplicationCleanupSnapshot,
    RemoteViewApplicationReleaseTarget, RemoteViewJoinedReleaseOutcome,
    BROWSER_SESSION_STATE_SCHEMA_V1,
};
use rusqlite::{Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

use super::browser_session_store::{
    load_optional_document, save_document, BrowserSessionSqliteStore, SESSION_STATE_DOCUMENT,
};

const DOCUMENT: &str = "browser_launch_custody";
const SCHEMA: &str = "agent-browser.browser-launch-custody.v1";

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    records: BTreeMap<String, BrowserLaunchCustodyRecord>,
    #[serde(default)]
    release_fences: BTreeMap<String, RemoteViewApplicationReleaseTarget>,
    #[serde(default)]
    release_outcomes: BTreeMap<String, RemoteViewJoinedReleaseOutcome>,
}

fn ledger(connection: &Connection) -> Result<Ledger, LaunchCustodyStoreError> {
    let ledger: Ledger = load_optional_document::<Ledger>(connection, DOCUMENT, SCHEMA)
        .map_err(|_| LaunchCustodyStoreError::InvalidRecord)?;
    for (id, record) in &ledger.records {
        record
            .validate()
            .map_err(|_| LaunchCustodyStoreError::InvalidRecord)?;
        if id != &record.intent.intent_id {
            return Err(LaunchCustodyStoreError::InvalidRecord);
        }
    }
    for (id, target) in &ledger.release_fences {
        if uuid::Uuid::parse_str(id).is_err() || target.validate().is_err() {
            return Err(LaunchCustodyStoreError::InvalidRecord);
        }
    }
    for (id, outcome) in &ledger.release_outcomes {
        ledger
            .release_fences
            .get(id)
            .ok_or(LaunchCustodyStoreError::InvalidRecord)?
            .validate_outcome(outcome)
            .map_err(|_| LaunchCustodyStoreError::InvalidRecord)?;
    }
    Ok(ledger)
}
fn assignment_fenced(
    ledger: &Ledger,
    assignment: &agent_browser_service_model::RemoteViewAssignmentRecord,
) -> bool {
    ledger.release_fences.iter().any(|(id, target)| {
        target.assignment.assignment_id == assignment.assignment_id
            || (target.assignment.desktop_id == assignment.desktop_id
                && (!ledger.release_outcomes.contains_key(id)
                    || assignment.generation < target.assignment.generation))
    })
}
fn current(connection: &Connection) -> Result<BrowserSessionState, LaunchCustodyStoreError> {
    load_optional_document::<Option<BrowserSessionState>>(
        connection,
        SESSION_STATE_DOCUMENT,
        BROWSER_SESSION_STATE_SCHEMA_V1,
    )
    .map_err(|_| LaunchCustodyStoreError::InvalidRecord)?
    .ok_or(LaunchCustodyStoreError::InvalidRecord)
}

impl BrowserLaunchCustodyStore for BrowserSessionSqliteStore {
    fn admit_launch_intent(
        &mut self,
        intent: &BrowserLaunchIntent,
        expected: &BrowserSessionState,
    ) -> Result<LaunchCustodyAdmission, LaunchCustodyStoreError> {
        use LaunchCustodyStoreError as Error;
        let pending = BrowserLaunchCustodyRecord::pending(intent.clone())
            .map_err(|_| Error::InvalidRecord)?;
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let state = current(&transaction)?;
        if state != *expected {
            return Err(Error::Conflict);
        }
        let mut records = ledger(&transaction)?;
        if assignment_fenced(&records, &intent.assignment) {
            return Err(Error::ReleaseFenced);
        }

        if let Some(record) = records.records.get(&intent.intent_id) {
            if record.intent != *intent {
                return Err(Error::Conflict);
            }
            return Ok(LaunchCustodyAdmission::Existing(record.clone()));
        }
        if records.records.values().any(|record| {
            !record.published
                && (record.intent.assignment.assignment_id == intent.assignment.assignment_id
                    || record.intent.assignment.desktop_id == intent.assignment.desktop_id)
                && record.intent.assignment != intent.assignment
        }) || state.browsers.values().any(|browser| {
            browser.desktop.as_ref().is_some_and(|desktop| {
                desktop.desktop_id == intent.assignment.desktop_id
                    && desktop.generation != intent.assignment.generation
            })
        }) {
            return Err(Error::Conflict);
        }
        if state
            .browsers
            .values()
            .any(|browser| browser.profile_id == intent.profile_id)
            || records
                .records
                .values()
                .any(|record| !record.published && record.intent.profile_id == intent.profile_id)
        {
            return Err(Error::ProfileOccupied);
        }
        records.records.insert(intent.intent_id.clone(), pending);
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)?;
        Ok(LaunchCustodyAdmission::New)
    }
    fn observe_launch_intent(
        &mut self,
        intent: &BrowserLaunchIntent,
        launch: &BrowserLaunch,
    ) -> Result<(), LaunchCustodyStoreError> {
        use LaunchCustodyStoreError as Error;
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let mut records = ledger(&transaction)?;
        if assignment_fenced(&records, &intent.assignment) {
            return Err(Error::ReleaseFenced);
        }
        let record = records
            .records
            .get_mut(&intent.intent_id)
            .ok_or(Error::InvalidRecord)?;
        if record.intent != *intent {
            return Err(Error::Conflict);
        }
        record.observe(launch).map_err(|_| Error::Conflict)?;
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)
    }
    fn publish_launch_intent(
        &mut self,
        intent: &BrowserLaunchIntent,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
    ) -> Result<(), LaunchCustodyStoreError> {
        use LaunchCustodyStoreError as Error;
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        if current(&transaction)? != *expected {
            return Err(Error::Conflict);
        }
        let mut records = ledger(&transaction)?;
        if assignment_fenced(&records, &intent.assignment) {
            return Err(Error::ReleaseFenced);
        }
        let record = records
            .records
            .get_mut(&intent.intent_id)
            .ok_or(Error::InvalidRecord)?;
        if record.intent != *intent {
            return Err(Error::Conflict);
        }
        record
            .confirm_publication(state)
            .map_err(|_| Error::Conflict)?;
        validate_browser_publication(&records, state)?;
        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            state,
        )
        .map_err(|_| Error::Unavailable)?;
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)
    }
}

impl BrowserReleaseCustodyStore for BrowserSessionSqliteStore {
    fn complete_assignment_release(
        &mut self,
        target: &RemoteViewApplicationReleaseTarget,
        release_id: &str,
        outcome: &RemoteViewJoinedReleaseOutcome,
    ) -> Result<(), LaunchCustodyStoreError> {
        use LaunchCustodyStoreError as Error;
        target
            .validate_outcome(outcome)
            .map_err(|_| Error::Conflict)?;
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let mut records = ledger(&transaction)?;
        if records.release_fences.get(release_id) != Some(target) {
            return Err(Error::Conflict);
        }
        if let Some(existing) = records.release_outcomes.get(release_id) {
            if existing != outcome {
                return Err(Error::Conflict);
            }
            return Ok(());
        }
        records
            .release_outcomes
            .insert(release_id.into(), outcome.clone());
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)
    }
    fn admit_assignment_release(
        &mut self,
        target: &RemoteViewApplicationReleaseTarget,
        snapshot: &RemoteViewApplicationCleanupSnapshot,
        release_id: &str,
    ) -> Result<RemoteViewApplicationCleanupPermit, LaunchCustodyStoreError> {
        use LaunchCustodyStoreError as Error;
        if uuid::Uuid::parse_str(release_id).is_err() || target.validate().is_err() {
            return Err(Error::InvalidRecord);
        }
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let state = current(&transaction)?;
        let mut records = ledger(&transaction)?;
        if let Some(existing) = records.release_fences.get(release_id) {
            if existing != target {
                return Err(Error::Conflict);
            }
        }
        if records.release_fences.iter().any(|(id, other)| {
            id != release_id
                && (other.assignment.assignment_id == target.assignment.assignment_id
                    || (other.assignment.desktop_id == target.assignment.desktop_id
                        && (!records.release_outcomes.contains_key(id)
                            || target.assignment.generation < other.assignment.generation)))
        }) {
            return Err(Error::Conflict);
        }
        if records.records.values().any(|record| {
            !record.published
                && (record.intent.assignment.assignment_id == target.assignment.assignment_id
                    || record.intent.assignment.desktop_id == target.assignment.desktop_id)
        }) {
            return Err(Error::LaunchPending);
        }
        let permit = prepare_remote_view_application_cleanup(&state, target, snapshot)
            .map_err(|_| Error::CleanupBlocked)?;
        records
            .release_fences
            .insert(release_id.into(), target.clone());
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)?;
        Ok(permit)
    }
}

fn validate_browser_publication(
    records: &Ledger,
    state: &BrowserSessionState,
) -> Result<(), LaunchCustodyStoreError> {
    // An observed or ambiguous launch cannot bypass atomic custody publication
    // through the ordinary whole-state writer. Co-located other profiles remain
    // admissible; assignment return is governed separately below.
    if records.records.values().any(|record| {
        !record.published
            && state.browsers.values().any(|browser| {
                browser.profile_id == record.intent.profile_id
                    || record.observed_browser_id.as_ref() == Some(&browser.id)
            })
    }) {
        return Err(LaunchCustodyStoreError::LaunchPending);
    }
    if state.browsers.values().any(|browser| {
        browser.desktop.as_ref().is_some_and(|desktop| {
            records.release_fences.iter().any(|(id, target)| {
                if target.assignment.desktop_id != desktop.desktop_id {
                    return false;
                }
                if !records.release_outcomes.contains_key(id)
                    || desktop.generation < target.assignment.generation
                {
                    return true;
                }
                !records.records.values().any(|record| {
                    record.published
                        && record.intent.assignment.assignment_id != target.assignment.assignment_id
                        && record.intent.assignment.desktop_id == desktop.desktop_id
                        && record.intent.assignment.generation == desktop.generation
                        && record.intent.profile_id == browser.profile_id
                        && record.observed_browser_id.as_ref() == Some(&browser.id)
                        && record.observed_pid == Some(browser.pid)
                })
            })
        })
    }) {
        return Err(LaunchCustodyStoreError::ReleaseFenced);
    }

    Ok(())
}

pub(super) fn reject_fenced_browser_publication(
    connection: &Connection,
    state: &BrowserSessionState,
) -> Result<(), LaunchCustodyStoreError> {
    validate_browser_publication(&ledger(connection)?, state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_service_model::{ManagedBrowserInstance, RemoteViewFixedDesktop};
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("p220-launch-custody-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn open(&self, initialize: bool) -> BrowserSessionSqliteStore {
            BrowserSessionSqliteStore::launch_custody_fixture(
                Connection::open(self.0.join("runtime.sqlite3")).unwrap(),
                initialize,
            )
            .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn intent() -> BrowserLaunchIntent {
        let f: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        BrowserLaunchIntent {
            intent_id: uuid::Uuid::new_v4().to_string(),
            profile_id: "profile-a".into(),
            assignment: serde_json::from_value(f["assignment"].clone()).unwrap(),
        }
    }
    fn release_evidence(
        intent: &BrowserLaunchIntent,
    ) -> (
        RemoteViewApplicationReleaseTarget,
        RemoteViewApplicationCleanupSnapshot,
    ) {
        use agent_browser_service_model::RemoteViewApplicationObligationInventory as Inventory;
        (
            RemoteViewApplicationReleaseTarget {
                assignment: intent.assignment.clone(),
                route_ids: vec![],
                viewer_session_ids: vec![],
            },
            RemoteViewApplicationCleanupSnapshot {
                schema_version: 1,
                assignment_id: intent.assignment.assignment_id.clone(),
                desktop_id: intent.assignment.desktop_id.clone(),
                lifecycle_generation: intent.assignment.generation,
                pending_recovery: Inventory::Complete(vec![]),
                foreground_leases: Inventory::Complete(vec![]),
                cleanup_tasks: Inventory::Complete(vec![]),
            },
        )
    }
    #[test]
    fn sqlite_release_completion_retains_exact_history_and_allows_fresh_same_generation_assignment()
    {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let old = intent();
        let (target, snapshot) = release_evidence(&old);
        let release_id = uuid::Uuid::new_v4().to_string();
        let initial = store.load_session_state().unwrap();
        store
            .admit_assignment_release(&target, &snapshot, &release_id)
            .unwrap();
        let mut released = target.assignment.clone();
        released.state = agent_browser_service_model::RemoteViewAssignmentState::Released;
        let outcome = RemoteViewJoinedReleaseOutcome {
            assignment: released,
            retirement: agent_browser_service_model::RemoteViewDesktopViewingRetirement {
                schema_version: 1,
                desktop_id: target.assignment.desktop_id.clone(),
                generation: target.assignment.generation,
                routes: vec![],
                sessions: vec![],
            },
        };
        let mut invalid = outcome.clone();
        invalid.retirement.generation += 1;
        assert_eq!(
            store
                .complete_assignment_release(&target, &release_id, &invalid)
                .unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        assert_eq!(
            store.admit_launch_intent(&old, &initial).unwrap_err(),
            LaunchCustodyStoreError::ReleaseFenced
        );
        store
            .complete_assignment_release(&target, &release_id, &outcome)
            .unwrap();
        store
            .complete_assignment_release(&target, &release_id, &outcome)
            .unwrap();
        assert_eq!(
            store.admit_launch_intent(&old, &initial).unwrap_err(),
            LaunchCustodyStoreError::ReleaseFenced
        );
        let mut fresh = old.clone();
        fresh.intent_id = uuid::Uuid::new_v4().to_string();
        fresh.assignment.assignment_id = "fresh-assignment".into();
        assert_eq!(
            store.admit_launch_intent(&fresh, &initial).unwrap(),
            LaunchCustodyAdmission::New
        );
        let launch = BrowserLaunch {
            browser_id: "fresh-browser".into(),
            pid: 42,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: fresh.assignment.desktop_id.clone(),
                generation: fresh.assignment.generation,
                friendly_route_label: "synthetic".into(),
            }),
        };
        store.observe_launch_intent(&fresh, &launch).unwrap();
        let mut second = fresh.clone();
        second.assignment.assignment_id = "other-assignment".into();
        second.assignment.desktop_id = uuid::Uuid::new_v4().to_string();
        let (other_target, other_snapshot) = release_evidence(&second);
        store
            .admit_assignment_release(
                &other_target,
                &other_snapshot,
                &uuid::Uuid::new_v4().to_string(),
            )
            .unwrap();
        let mut proposed = initial.clone();
        proposed.browsers.insert(
            launch.browser_id.clone(),
            ManagedBrowserInstance {
                id: launch.browser_id,
                profile_id: fresh.profile_id.clone(),
                pid: launch.pid,
                cdp_endpoint: launch.cdp_endpoint,
                desktop: launch.desktop,
                active_session_ids: vec![],
            },
        );
        let mut rogue = proposed.browsers["fresh-browser"].clone();
        rogue.id = "rogue-browser".into();
        rogue.desktop.as_mut().unwrap().desktop_id = second.assignment.desktop_id;
        proposed.browsers.insert(rogue.id.clone(), rogue);
        assert_eq!(
            store
                .publish_launch_intent(&fresh, &initial, &proposed)
                .unwrap_err(),
            LaunchCustodyStoreError::ReleaseFenced
        );
        assert_eq!(store.load_session_state().unwrap(), initial);
        proposed.browsers.remove("rogue-browser");
        store
            .publish_launch_intent(&fresh, &initial, &proposed)
            .unwrap();
        store
            .compare_and_save_session_state(&proposed, &proposed)
            .unwrap();
        drop(store);
        let mut reopened = fixture.open(false);
        assert_eq!(
            reopened.admit_launch_intent(&old, &proposed).unwrap_err(),
            LaunchCustodyStoreError::ReleaseFenced
        );
        assert_eq!(
            ledger(reopened.remote_view_mutation_connection())
                .unwrap()
                .release_outcomes[&release_id],
            outcome
        );
    }

    #[test]
    fn sqlite_release_custody_excludes_unknown_launch_and_survives_restart() {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let intent = intent();
        let initial = store.load_session_state().unwrap();
        let (target, snapshot) = release_evidence(&intent);
        let release_id = uuid::Uuid::new_v4().to_string();
        store.admit_launch_intent(&intent, &initial).unwrap();
        assert_eq!(
            store
                .admit_assignment_release(&target, &snapshot, &release_id)
                .unwrap_err(),
            LaunchCustodyStoreError::LaunchPending
        );
        // A separate fixture proves the opposite admission order.
        let second = Fixture::new();
        let mut clear = second.open(true);
        let permit = clear
            .admit_assignment_release(&target, &snapshot, &release_id)
            .unwrap();
        assert!(permit.acknowledgement().application_references_clear);
        drop(clear);
        let mut reopened = second.open(false);
        assert_eq!(
            reopened.admit_launch_intent(&intent, &initial).unwrap_err(),
            LaunchCustodyStoreError::ReleaseFenced
        );
        assert!(reopened
            .admit_assignment_release(&target, &snapshot, &release_id)
            .is_ok());
        assert_eq!(
            reopened
                .admit_assignment_release(&target, &snapshot, &uuid::Uuid::new_v4().to_string())
                .unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        let mut changed = target.clone();
        changed.assignment.generation += 1;
        assert_eq!(
            reopened
                .admit_assignment_release(&changed, &snapshot, &release_id)
                .unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        let mut proposed = initial.clone();
        proposed.browsers.insert(
            "browser-a".into(),
            ManagedBrowserInstance {
                id: "browser-a".into(),
                profile_id: intent.profile_id,
                pid: 42,
                cdp_endpoint: "http://127.0.0.1:9222".into(),
                desktop: Some(RemoteViewFixedDesktop {
                    desktop_id: intent.assignment.desktop_id,
                    generation: intent.assignment.generation,
                    friendly_route_label: "synthetic".into(),
                }),
                active_session_ids: vec![],
            },
        );
        assert_eq!(
            reopened
                .compare_and_save_session_state(&initial, &proposed)
                .unwrap_err(),
            "browser_session_publication_release_fenced"
        );
        assert_eq!(reopened.load_session_state().unwrap(), initial);
    }

    #[test]
    fn sqlite_launch_custody_retains_unknown_claim_across_connections_and_restart() {
        let fixture = Fixture::new();
        let mut first = fixture.open(true);
        let mut peer = fixture.open(false);
        let intent = intent();
        let state = first.load_session_state().unwrap();
        assert_eq!(
            first.admit_launch_intent(&intent, &state).unwrap(),
            LaunchCustodyAdmission::New
        );
        assert!(matches!(
            peer.admit_launch_intent(&intent, &state).unwrap(),
            LaunchCustodyAdmission::Existing(_)
        ));
        let mut different = intent.clone();
        different.intent_id = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            peer.admit_launch_intent(&different, &state).unwrap_err(),
            LaunchCustodyStoreError::ProfileOccupied
        );
        let mut peer_profile = intent.clone();
        peer_profile.intent_id = uuid::Uuid::new_v4().to_string();
        peer_profile.profile_id = "profile-b".into();
        peer_profile.assignment.generation += 1;
        assert_eq!(
            peer.admit_launch_intent(&peer_profile, &state).unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        peer_profile.assignment.generation -= 1;
        assert_eq!(
            peer.admit_launch_intent(&peer_profile, &state).unwrap(),
            LaunchCustodyAdmission::New
        );
        different = intent.clone();
        different.assignment.generation += 1;
        assert_eq!(
            peer.admit_launch_intent(&different, &state).unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        drop(first);
        drop(peer);
        let mut reopened = fixture.open(false);
        let LaunchCustodyAdmission::Existing(record) =
            reopened.admit_launch_intent(&intent, &state).unwrap()
        else {
            panic!("restart lost claim");
        };
        assert_eq!(record.observed_pid, None);
        assert!(!record.published);
        save_document(
            reopened.remote_view_mutation_connection(),
            DOCUMENT,
            SCHEMA,
            &serde_json::Value::Null,
        )
        .unwrap();
        assert_eq!(
            reopened.admit_launch_intent(&intent, &state).unwrap_err(),
            LaunchCustodyStoreError::InvalidRecord
        );
    }
    #[test]
    fn sqlite_launch_custody_publishes_session_and_observation_atomically() {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let intent = intent();
        let initial = store.load_session_state().unwrap();
        store.admit_launch_intent(&intent, &initial).unwrap();
        let launch = BrowserLaunch {
            browser_id: "browser-a".into(),
            pid: 42,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: intent.assignment.desktop_id.clone(),
                generation: intent.assignment.generation,
                friendly_route_label: "synthetic".into(),
            }),
        };
        store.observe_launch_intent(&intent, &launch).unwrap();
        let mut changed = launch.clone();
        changed.pid += 1;
        assert_eq!(
            store.observe_launch_intent(&intent, &changed).unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        let mut published = initial.clone();
        published.browsers.insert(
            launch.browser_id.clone(),
            ManagedBrowserInstance {
                id: launch.browser_id,
                profile_id: intent.profile_id.clone(),
                pid: launch.pid,
                cdp_endpoint: launch.cdp_endpoint,
                desktop: launch.desktop,
                active_session_ids: vec![],
            },
        );
        let mut newer = initial.clone();
        newer.next_session_sequence = 7;
        store
            .compare_and_save_session_state(&initial, &newer)
            .unwrap();
        assert_eq!(
            store
                .publish_launch_intent(&intent, &initial, &published)
                .unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        assert_eq!(store.load_session_state().unwrap(), newer);
        let LaunchCustodyAdmission::Existing(record) =
            store.admit_launch_intent(&intent, &newer).unwrap()
        else {
            panic!("claim missing");
        };
        assert!(!record.published);
        published.next_session_sequence = newer.next_session_sequence;
        store
            .publish_launch_intent(&intent, &newer, &published)
            .unwrap();
        drop(store);
        let mut reopened = fixture.open(false);
        assert_eq!(reopened.load_session_state().unwrap(), published);
        let LaunchCustodyAdmission::Existing(record) =
            reopened.admit_launch_intent(&intent, &published).unwrap()
        else {
            panic!("claim missing");
        };
        assert!(record.published);
        assert_eq!(record.observed_pid, Some(42));
    }
}
