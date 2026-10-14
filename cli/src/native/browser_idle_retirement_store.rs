//! Idle retirement custody in the existing Browser Runtime SQLite owner.
use super::browser_session_store::{
    load_optional_document, save_document, BrowserSessionSqliteStore, SESSION_STATE_DOCUMENT,
};
use agent_browser_service_model::*;
use rusqlite::{Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const DOCUMENT: &str = "browser_idle_retirement_custody";
const SCHEMA: &str = "agent-browser.browser-idle-retirement-custody.v1";
type Error = BrowserIdleRetirementStoreError;
type Result<T> = std::result::Result<T, Error>;

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    records: BTreeMap<String, BrowserIdleRetirementRecord>,
}

fn wire_identity(intent: &BrowserIdleRetirementIntent) -> Result<String> {
    let provider = &intent.provider;
    serde_json::to_vec(&(
        &provider.target,
        &provider.idempotency_key,
        provider.expected_revision,
        &provider.expected_retirement,
    ))
    .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
    .map_err(|_| Error::InvalidRecord)
}
fn conflicts(a: &BrowserIdleRetirementIntent, b: &BrowserIdleRetirementIntent) -> bool {
    a.provider.target.viewing_desktop_id == b.provider.target.viewing_desktop_id
        || a.scopes.iter().any(|a| {
            b.scopes
                .iter()
                .any(|b| a.browser.profile_id == b.browser.profile_id)
        })
}
fn ledger(connection: &Connection) -> Result<Ledger> {
    let ledger: Ledger =
        load_optional_document(connection, DOCUMENT, SCHEMA).map_err(|_| Error::InvalidRecord)?;
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| Error::Unavailable)?;
    if !ledger.records.is_empty() && version != 2 {
        return Err(Error::InvalidRecord);
    }
    let mut operations = BTreeSet::new();
    let mut pending: Vec<&BrowserIdleRetirementIntent> = vec![];
    for (id, record) in &ledger.records {
        record.validate()?;
        if id != &record.intent.id || !operations.insert(wire_identity(&record.intent)?) {
            return Err(Error::InvalidRecord);
        }
        if !record.is_terminal() {
            if pending.iter().any(|old| conflicts(old, &record.intent)) {
                return Err(Error::InvalidRecord);
            }
            pending.push(&record.intent);
        }
    }
    Ok(ledger)
}
fn current(connection: &Connection) -> Result<BrowserSessionState> {
    let state = load_optional_document::<Option<BrowserSessionState>>(
        connection,
        SESSION_STATE_DOCUMENT,
        BROWSER_SESSION_STATE_SCHEMA_V1,
    )
    .map_err(|_| Error::InvalidRecord)?
    .ok_or(Error::InvalidRecord)?;
    if state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
        return Err(Error::InvalidRecord);
    }
    Ok(state)
}

impl BrowserIdleRetirementCustodyStore for BrowserSessionSqliteStore {
    fn idle_retirement_records(&mut self) -> Result<Vec<BrowserIdleRetirementRecord>> {
        Ok(ledger(self.remote_view_mutation_connection())?
            .records
            .into_values()
            .collect())
    }
    fn claim_idle_retirement(
        &mut self,
        intent: &BrowserIdleRetirementIntent,
        expected: &BrowserSessionState,
    ) -> Result<BrowserIdleRetirementAdmission> {
        intent.validate()?;
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let mut records = ledger(&transaction)?;
        if let Some(existing) = records.records.get(&intent.id) {
            if existing.intent != *intent {
                return Err(Error::Conflict);
            }
            return Ok(BrowserIdleRetirementAdmission::Existing(Box::new(
                existing.clone(),
            )));
        }
        if current(&transaction)? != *expected {
            return Err(Error::Conflict);
        }
        intent.matches_baseline(expected)?;
        let identity = wire_identity(intent)?;
        for existing in records.records.values() {
            if wire_identity(&existing.intent)? == identity
                || (!existing.is_terminal() && conflicts(&existing.intent, intent))
            {
                return Err(Error::Conflict);
            }
        }
        for scope in &intent.scopes {
            if super::browser_launch_custody_store::published_assignment(
                &transaction,
                &scope.browser,
            )
            .map_err(|_| Error::Conflict)?
                != intent.provider.assignment
            {
                return Err(Error::Conflict);
            }
        }
        records.records.insert(
            intent.id.clone(),
            BrowserIdleRetirementRecord::new(intent.clone())?,
        );
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        // This marker and the first obligation commit together. Earlier
        // binaries reject schema 2 rather than ignoring pending close custody.
        transaction
            .pragma_update(None, "user_version", 2i64)
            .map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)?;
        Ok(BrowserIdleRetirementAdmission::New)
    }
    fn update_idle_retirement(
        &mut self,
        previous: &BrowserIdleRetirementRecord,
        next: &BrowserIdleRetirementRecord,
    ) -> Result<()> {
        previous.validate_successor(next)?;
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let mut records = ledger(&transaction)?;
        let current = records
            .records
            .get(&previous.intent.id)
            .ok_or(Error::Conflict)?;
        if current == next {
            return Ok(());
        }
        if current != previous {
            return Err(Error::Conflict);
        }
        records.records.insert(next.intent.id.clone(), next.clone());
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)
    }
    fn publish_idle_retirement_state(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
        ids: &[String],
    ) -> Result<()> {
        if ids.is_empty()
            || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
            || state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1
        {
            return Err(Error::InvalidRecord);
        }
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        if current(&transaction)? != *expected {
            return Err(Error::Conflict);
        }
        let mut records = ledger(&transaction)?;
        super::browser_launch_custody_store::reject_fenced_browser_publication(&transaction, state)
            .map_err(|_| Error::Conflict)?;
        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            state,
        )
        .map_err(|_| Error::Unavailable)?;
        // History projection may archive terminal rows in this transaction.
        // Bind custody to the actual retained session document, not a pre-archive copy.
        let published = current(&transaction)?;
        for id in ids {
            records
                .records
                .get_mut(id)
                .ok_or(Error::Conflict)?
                .record_publication(&published)?;
        }
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::super::browser_session_store::BrowserSessionSqliteStore;
    use agent_browser_service_model::*;
    use rusqlite::Connection;
    use serde_json::{json, Value};

    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir()
                .join(format!("p237-retirement-custody-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn open(&self, initialize: bool) -> BrowserSessionSqliteStore {
            let connection = Connection::open(self.0.join("runtime.sqlite3")).unwrap();
            connection
                .busy_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            BrowserSessionSqliteStore::launch_custody_fixture(connection, initialize).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn setup(
        store: &mut BrowserSessionSqliteStore,
    ) -> (
        BrowserSessionState,
        BrowserIdleRetirementIntent,
        RemoteViewIdleRetirementReceipt,
    ) {
        let f: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let assignment: RemoteViewAssignmentRecord =
            serde_json::from_value(f["assignment"].clone()).unwrap();
        let initial = store.load_session_state().unwrap();
        let launch = BrowserLaunchIntent {
            intent_id: uuid::Uuid::new_v4().to_string(),
            profile_id: "profile-a".into(),
            assignment: assignment.clone(),
        };
        store.admit_launch_intent(&launch, &initial).unwrap();
        let process = BrowserLaunch {
            browser_id: "browser-a".into(),
            pid: 42,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: assignment.desktop_id.clone(),
                generation: assignment.generation,
                friendly_route_label: "fixture".into(),
            }),
        };
        store.observe_launch_intent(&launch, &process).unwrap();
        let mut state = initial.clone();
        state.browsers.insert(
            "browser-a".into(),
            ManagedBrowserInstance {
                id: process.browser_id,
                profile_id: "profile-a".into(),
                pid: process.pid,
                cdp_endpoint: process.cdp_endpoint,
                desktop: process.desktop,
                active_session_ids: vec![],
            },
        );
        for id in ["tab-a", "tab-b"] {
            state.tabs.insert(
                id.into(),
                ManagedBrowserTab {
                    id: id.into(),
                    target_id: format!("target-{id}"),
                    browser_id: "browser-a".into(),
                    session_id: "session-a".into(),
                    created_at_ms: 1,
                    last_activity_at_ms: 1,
                },
            );
        }
        store
            .publish_launch_intent(&launch, &initial, &state)
            .unwrap();
        let provider = serde_json::from_value(json!({"schemaVersion":1,"assignment":assignment,
            "target":f["assignmentObservation"]["target"],"idempotencyKey":"close-one","expectedRevision":0,
            "expectedRetirement":{"schemaVersion":1,"desktopId":assignment.desktop_id,"generation":assignment.generation,"routes":[],"sessions":[]}})).unwrap();
        let intent = BrowserIdleRetirementIntent::new(
            "11111111-1111-4111-8111-111111111111",
            provider,
            &state,
            &["browser-a".into()],
        )
        .unwrap();
        let prepared = serde_json::from_value(json!({"schemaVersion":1,"target":f["assignmentObservation"]["target"],
            "readinessScope":"native_admission","state":"prepared","fence":{"schema_version":1,"installation_id":"0123456789abcdef",
                "config_hash":"a".repeat(64),"slot":2,"generation":19,"operation_id":"5deb7e63650d881f18b5d68f85ec45bcd7523255499618982a3c2995400637cc","revision":1}})).unwrap();
        (state, intent, prepared)
    }

    #[test]
    fn sqlite_idle_retirement_concurrent_claim_has_one_effect_admission() {
        let fixture = Fixture::new();
        let mut initial = fixture.open(true);
        let (baseline, intent, _) = setup(&mut initial);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let mut threads = vec![];
        for _ in 0..2 {
            let path = fixture.0.join("runtime.sqlite3");
            let expected = baseline.clone();
            let intent = intent.clone();
            let barrier = barrier.clone();
            threads.push(std::thread::spawn(move || {
                let connection = Connection::open(path).unwrap();
                connection
                    .busy_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                let mut store =
                    BrowserSessionSqliteStore::launch_custody_fixture(connection, false).unwrap();
                barrier.wait();
                store.claim_idle_retirement(&intent, &expected).unwrap()
            }));
        }
        barrier.wait();
        let results = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            results
                .iter()
                .filter(|result| **result == BrowserIdleRetirementAdmission::New)
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, BrowserIdleRetirementAdmission::Existing(_)))
                .count(),
            1
        );
        assert_eq!(initial.idle_retirement_records().unwrap().len(), 1);
    }

    #[test]
    fn sqlite_idle_retirement_failed_claim_does_not_promote_reader_version() {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let (baseline, intent, _) = setup(&mut store);
        store
            .remote_view_mutation_connection()
            .execute_batch(
                "CREATE TRIGGER reject_idle_custody BEFORE INSERT ON state_documents
             WHEN NEW.kind='browser_idle_retirement_custody'
             BEGIN SELECT RAISE(ABORT,'fixture'); END;",
            )
            .unwrap();
        assert_eq!(
            store.claim_idle_retirement(&intent, &baseline).unwrap_err(),
            BrowserIdleRetirementStoreError::Unavailable
        );
        assert!(store.idle_retirement_records().unwrap().is_empty());
        assert_eq!(
            store
                .remote_view_mutation_connection()
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(store.load_session_state().unwrap(), baseline);
        store
            .remote_view_mutation_connection()
            .execute_batch("DROP TRIGGER reject_idle_custody")
            .unwrap();
        assert_eq!(
            store.claim_idle_retirement(&intent, &baseline).unwrap(),
            BrowserIdleRetirementAdmission::New
        );
    }

    #[test]
    fn sqlite_idle_retirement_custody_cas_restart_and_atomic_publication() {
        let fixture = Fixture::new();
        let mut first = fixture.open(true);
        let (baseline, intent, prepared) = setup(&mut first);
        assert_eq!(
            first
                .remote_view_mutation_connection()
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            first.claim_idle_retirement(&intent, &baseline).unwrap(),
            BrowserIdleRetirementAdmission::New
        );
        assert_eq!(
            first
                .remote_view_mutation_connection()
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            2
        );
        let mut peer = fixture.open(false);
        assert!(matches!(
            peer.claim_idle_retirement(&intent, &baseline).unwrap(),
            BrowserIdleRetirementAdmission::Existing(_)
        ));
        let mut conflicting = intent.clone();
        conflicting.id = uuid::Uuid::new_v4().to_string();
        conflicting.provider.idempotency_key = "another-close".into();
        assert_eq!(
            peer.claim_idle_retirement(&conflicting, &baseline)
                .unwrap_err(),
            BrowserIdleRetirementStoreError::Conflict
        );
        let admitted = first.idle_retirement_records().unwrap().pop().unwrap();
        let mut started = admitted.clone();
        started.begin_preparation().unwrap();
        first.update_idle_retirement(&admitted, &started).unwrap();
        peer.update_idle_retirement(&admitted, &started).unwrap();
        let mut stale = started.clone();
        stale.defer_preparation().unwrap();
        assert_eq!(
            peer.update_idle_retirement(&admitted, &stale).unwrap_err(),
            BrowserIdleRetirementStoreError::Conflict
        );
        let mut ready = started.clone();
        ready.record_prepared(prepared.clone()).unwrap();
        first.update_idle_retirement(&started, &ready).unwrap();
        let effect = BrowserIdleRetirementEffect::CloseTab {
            browser_id: "browser-a".into(),
            tab_id: "tab-a".into(),
            target_id: "target-tab-a".into(),
        };
        let mut attempted = ready.clone();
        assert_eq!(
            attempted.begin_effect(effect.clone()).unwrap(),
            BrowserIdleRetirementEffectAdmission::New
        );
        first.update_idle_retirement(&ready, &attempted).unwrap();
        let mut closed = baseline.clone();
        closed.tabs.remove("tab-a");
        assert!(first
            .publish_idle_retirement_state(&baseline, &closed, std::slice::from_ref(&intent.id))
            .is_err());
        assert_eq!(peer.load_session_state().unwrap(), baseline);
        drop(first);
        let mut restarted = fixture.open(false);
        assert_eq!(
            restarted.idle_retirement_records().unwrap(),
            vec![attempted.clone()]
        );
        let mut settled = attempted.clone();
        settled.settle_effect(&effect).unwrap();
        restarted
            .update_idle_retirement(&attempted, &settled)
            .unwrap();
        let mut forged_publication = settled.clone();
        forged_publication.record_publication(&closed).unwrap();
        assert!(restarted
            .update_idle_retirement(&settled, &forged_publication)
            .is_err());
        assert!(restarted
            .publish_idle_retirement_state(&baseline, &baseline, std::slice::from_ref(&intent.id))
            .is_err());
        assert_eq!(peer.load_session_state().unwrap(), baseline);
        // Fail after the session write, at the custody write. Both documents
        // must roll back so another process cannot observe a partial publish.
        restarted
            .remote_view_mutation_connection()
            .execute_batch(
                "CREATE TRIGGER reject_idle_publication BEFORE INSERT ON state_documents
                 WHEN NEW.kind='browser_idle_retirement_custody'
                 BEGIN SELECT RAISE(ABORT,'fixture'); END;",
            )
            .unwrap();
        assert_eq!(
            restarted
                .publish_idle_retirement_state(
                    &baseline,
                    &closed,
                    std::slice::from_ref(&intent.id),
                )
                .unwrap_err(),
            BrowserIdleRetirementStoreError::Unavailable
        );
        assert_eq!(peer.load_session_state().unwrap(), baseline);
        assert_eq!(peer.idle_retirement_records().unwrap(), vec![settled]);
        restarted
            .remote_view_mutation_connection()
            .execute_batch("DROP TRIGGER reject_idle_publication")
            .unwrap();
        restarted
            .publish_idle_retirement_state(&baseline, &closed, std::slice::from_ref(&intent.id))
            .unwrap();
        assert_eq!(peer.load_session_state().unwrap(), closed);
        assert!(closed.tabs.contains_key("tab-b"));
        let published = peer.idle_retirement_records().unwrap().pop().unwrap();
        assert!(published.published_state_digest.is_some());
        let mut value = serde_json::to_value(prepared).unwrap();
        value["state"] = json!("completed");
        let mut completed = published.clone();
        completed
            .record_completed(serde_json::from_value(value).unwrap())
            .unwrap();
        restarted
            .update_idle_retirement(&published, &completed)
            .unwrap();
        assert!(peer.idle_retirement_records().unwrap()[0].is_terminal());
    }
}
