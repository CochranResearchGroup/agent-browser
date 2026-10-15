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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    activities: BTreeMap<String, BrowserActivityRecord>,
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
    if !ledger.records.is_empty() && !(2..=4).contains(&version) {
        return Err(Error::InvalidRecord);
    }
    if !ledger.activities.is_empty() && !(3..=4).contains(&version) {
        return Err(Error::InvalidRecord);
    }
    for (id, activity) in &ledger.activities {
        if id != &activity.id
            || uuid::Uuid::parse_str(id).is_err()
            || activity.browser.pid == 0
            || activity.browser.profile_id.is_empty()
            || ((activity.process_identity.is_some() || activity.original_ended) && version != 4)
            || activity.process_identity.as_ref().is_some_and(|identity| {
                identity.pid != activity.browser.pid
                    || identity.start_token.trim().is_empty()
                    || identity
                        .executable_path
                        .as_ref()
                        .is_none_or(|path| path.trim().is_empty())
            })
        {
            return Err(Error::InvalidRecord);
        }
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

/// Check under the caller's immediate transaction, before admitting a launch
/// or release. Pending native completion still protects its original profile.
pub(super) fn reject_retirement_admission(
    connection: &Connection,
    profile_id: &str,
    assignment: &RemoteViewAssignmentRecord,
) -> Result<()> {
    if ledger(connection)?
        .activities
        .values()
        .any(|activity| !activity.original_ended && activity.browser.profile_id == profile_id)
    {
        return Err(Error::ActivityActive);
    }
    if ledger(connection)?.records.values().any(|record| {
        !record.is_terminal()
            && (record.intent.provider.assignment.desktop_id == assignment.desktop_id
                || record
                    .intent
                    .scopes
                    .iter()
                    .any(|scope| scope.browser.profile_id == profile_id))
    }) {
        return Err(Error::Conflict);
    }
    Ok(())
}

/// Ordinary writers may advance unrelated work, but cannot replace the active
/// physical projection of a pending operation. The exact atomic retirement
/// publisher names the operations it settles; every other scope stays fenced.
pub(super) fn reject_retirement_publication(
    connection: &Connection,
    before: &BrowserSessionState,
    after: &BrowserSessionState,
    admitted_ids: &[String],
    activity_ids: &[String],
) -> Result<()> {
    for activity in ledger(connection)?
        .activities
        .values()
        .filter(|activity| !activity.original_ended && !activity_ids.contains(&activity.id))
    {
        let root = &activity.browser;
        if after.browsers.get(&root.id).is_none_or(|candidate| {
            candidate.pid != root.pid
                || candidate.cdp_endpoint != root.cdp_endpoint
                || candidate.profile_id != root.profile_id
                || candidate.desktop != root.desktop
        }) || before.idle_closed_browsers.get(&root.id)
            != after.idle_closed_browsers.get(&root.id)
            || before.shared_browser_ids.contains(&root.id)
                != after.shared_browser_ids.contains(&root.id)
        {
            return Err(Error::ActivityActive);
        }
    }
    for record in ledger(connection)?
        .records
        .values()
        .filter(|record| !record.is_terminal() && !admitted_ids.contains(&record.intent.id))
    {
        let profiles = record
            .intent
            .scopes
            .iter()
            .map(|scope| &scope.browser.profile_id)
            .collect::<BTreeSet<_>>();
        let mut browser_ids = record
            .intent
            .scopes
            .iter()
            .map(|scope| scope.browser.id.clone())
            .collect::<BTreeSet<_>>();
        for browser in before.browsers.values().chain(after.browsers.values()) {
            if profiles.contains(&browser.profile_id)
                || browser.desktop.as_ref().is_some_and(|desktop| {
                    desktop.desktop_id == record.intent.provider.assignment.desktop_id
                })
            {
                browser_ids.insert(browser.id.clone());
            }
        }
        let tab_ids = record
            .intent
            .scopes
            .iter()
            .flat_map(|scope| &scope.tabs)
            .map(|tab| &tab.id)
            .collect::<BTreeSet<_>>();
        let session_ids = record
            .intent
            .scopes
            .iter()
            .flat_map(|scope| &scope.tabs)
            .map(|tab| &tab.session_id)
            .collect::<BTreeSet<_>>();
        let browsers = |state: &BrowserSessionState| {
            state
                .browsers
                .iter()
                .filter(|(id, _)| browser_ids.contains(*id))
                .map(|(id, browser)| (id.clone(), browser.clone()))
                .collect::<BTreeMap<_, _>>()
        };
        let tabs = |state: &BrowserSessionState| {
            state
                .tabs
                .iter()
                .filter(|(id, tab)| tab_ids.contains(*id) || browser_ids.contains(&tab.browser_id))
                .map(|(id, tab)| (id.clone(), tab.clone()))
                .collect::<BTreeMap<_, _>>()
        };
        let sessions = |state: &BrowserSessionState| {
            state
                .sessions
                .iter()
                .filter(|(id, session)| {
                    session_ids.contains(*id)
                        || profiles.contains(&session.profile_id)
                        || browser_ids.contains(&session.browser_id)
                })
                .map(|(id, session)| (id.clone(), session.clone()))
                .collect::<BTreeMap<_, _>>()
        };
        let closed = |state: &BrowserSessionState| {
            state
                .idle_closed_browsers
                .iter()
                .filter(|(id, _)| browser_ids.contains(*id))
                .map(|(id, at)| (id.clone(), *at))
                .collect::<BTreeMap<_, _>>()
        };
        if browsers(before) != browsers(after)
            || tabs(before) != tabs(after)
            || sessions(before) != sessions(after)
            || closed(before) != closed(after)
        {
            return Err(Error::Conflict);
        }
    }
    Ok(())
}

impl BrowserIdleRetirementCustodyStore for BrowserSessionSqliteStore {
    fn browser_activity_records(&mut self) -> Result<Vec<BrowserActivityRecord>> {
        Ok(ledger(self.remote_view_mutation_connection())?
            .activities
            .into_values()
            .collect())
    }
    fn settle_browser_activity_resource(&mut self, previous: &BrowserActivityRecord) -> Result<()> {
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let mut records = ledger(&transaction)?;
        let record = records
            .activities
            .get_mut(&previous.id)
            .ok_or(Error::Conflict)?;
        if record != previous {
            return Err(Error::Conflict);
        }
        record.original_ended = true;
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction
            .pragma_update(None, "user_version", 4i64)
            .map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)
    }
    fn admit_browser_activity(
        &mut self,
        id: &str,
        browser: &ManagedBrowserInstance,
        expected: &BrowserSessionState,
    ) -> Result<()> {
        if uuid::Uuid::parse_str(id).is_err() {
            return Err(Error::InvalidRecord);
        }
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let mut records = ledger(&transaction)?;
        if current(&transaction)? != *expected
            || expected.browsers.get(&browser.id).is_none_or(|root| {
                root.pid != browser.pid
                    || root.cdp_endpoint != browser.cdp_endpoint
                    || root.profile_id != browser.profile_id
                    || root.desktop != browser.desktop
            })
            || expected.idle_closed_browsers.contains_key(&browser.id)
            || records.activities.contains_key(id)
            || records.records.values().any(|record| {
                !record.is_terminal()
                    && record
                        .intent
                        .scopes
                        .iter()
                        .any(|scope| scope.browser.profile_id == browser.profile_id)
            })
        {
            return Err(Error::Conflict);
        }
        let root = expected.browsers.get(&browser.id).ok_or(Error::Conflict)?;
        let process_identity =
            if root.desktop.is_none() || expected.shared_browser_ids.contains(&root.id) {
                None
            } else {
                super::browser_launch_custody_store::published_process_identity(&transaction, root)
                    .map_err(|_| Error::Unavailable)?
            };
        let minimum_version = if process_identity.is_some() { 4 } else { 3 };
        records.activities.insert(
            id.into(),
            BrowserActivityRecord {
                id: id.into(),
                browser: browser.clone(),
                acknowledged: false,
                process_identity,
                original_ended: false,
            },
        );
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction
            .pragma_update(
                None,
                "user_version",
                transaction
                    .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                    .map_err(|_| Error::Unavailable)?
                    .max(minimum_version),
            )
            .map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)
    }
    fn acknowledge_browser_activity(&mut self, id: &str) -> Result<()> {
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let mut records = ledger(&transaction)?;
        let record = records.activities.get_mut(id).ok_or(Error::Conflict)?;
        if record.original_ended {
            return Err(Error::Conflict);
        }
        record.acknowledged = true;
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)
    }

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
        if records.activities.values().any(|activity| {
            !activity.original_ended
                && intent
                    .scopes
                    .iter()
                    .any(|scope| scope.browser.profile_id == activity.browser.profile_id)
        }) {
            return Err(Error::ActivityActive);
        }
        super::browser_launch_custody_store::reject_pending_launch_for_retirement(
            &transaction,
            intent,
        )
        .map_err(|_| Error::Conflict)?;
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
                || super::browser_launch_custody_store::published_process_identity(
                    &transaction,
                    &scope.browser,
                )
                .map_err(|_| Error::Conflict)?
                    != scope.process_identity
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
            .pragma_update(
                None,
                "user_version",
                transaction
                    .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                    .map_err(|_| Error::Unavailable)?
                    .max(2),
            )
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
    fn admit_idle_retirement_preparation(
        &mut self,
        intent: &BrowserIdleRetirementIntent,
    ) -> Result<(BrowserIdleRetirementRecord, bool)> {
        intent.validate()?;
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let mut records = ledger(&transaction)?;
        let record = records.records.get_mut(&intent.id).ok_or(Error::Conflict)?;
        if record.intent != *intent
            || record.is_terminal()
            || record.published_state_digest.is_some()
        {
            return Err(Error::Conflict);
        }
        let admitted = !record.preparation_started();
        if admitted {
            record.begin_preparation()?;
        }
        let current = record.clone();
        if admitted {
            save_document(&transaction, DOCUMENT, SCHEMA, &records)
                .map_err(|_| Error::Unavailable)?;
        }
        transaction.commit().map_err(|_| Error::Unavailable)?;
        Ok((current, admitted))
    }
    fn admit_idle_retirement_effect(
        &mut self,
        intent: &BrowserIdleRetirementIntent,
        effect: BrowserIdleRetirementEffect,
    ) -> Result<(
        BrowserIdleRetirementRecord,
        BrowserIdleRetirementEffectAdmission,
    )> {
        intent.validate()?;
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let mut records = ledger(&transaction)?;
        let record = records.records.get_mut(&intent.id).ok_or(Error::Conflict)?;
        if record.intent != *intent {
            return Err(Error::Conflict);
        }
        let admission = record.begin_effect(effect)?;
        let admitted = record.clone();
        if admission == BrowserIdleRetirementEffectAdmission::New {
            save_document(&transaction, DOCUMENT, SCHEMA, &records)
                .map_err(|_| Error::Unavailable)?;
        }
        transaction.commit().map_err(|_| Error::Unavailable)?;
        Ok((admitted, admission))
    }
    fn publish_idle_retirement_state(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
        ids: &[String],
    ) -> Result<()> {
        self.publish_session_retirements(expected, state, ids, None, &[])
    }
}

impl BrowserSessionSqliteStore {
    /// Atomically publish launch, retirement custody and session state when
    /// one command retires old work before starting an unrelated profile.
    pub(super) fn publish_session_retirements(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
        ids: &[String],
        launch: Option<&BrowserLaunchIntent>,
        activity_ids: &[String],
    ) -> Result<()> {
        if (ids.is_empty() && activity_ids.is_empty())
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
        reject_retirement_publication(&transaction, expected, state, ids, activity_ids)?;
        if activity_ids.iter().collect::<BTreeSet<_>>().len() != activity_ids.len() {
            return Err(Error::InvalidRecord);
        }
        for id in activity_ids {
            if !records
                .activities
                .get(id)
                .is_some_and(|record| record.acknowledged || record.original_ended)
            {
                return Err(Error::Conflict);
            }
        }

        for id in ids {
            let record = records.records.get(id).ok_or(Error::Conflict)?;
            for scope in &record.intent.scopes {
                if expected.idle_closed_browsers.get(&scope.browser.id)
                    != state.idle_closed_browsers.get(&scope.browser.id)
                    && !record.has_settled_browser_close(&scope.browser.id)
                {
                    return Err(Error::Conflict);
                }
            }
        }
        if let Some(intent) = launch {
            super::browser_launch_custody_store::confirm_launch_publication(
                &transaction,
                intent,
                state,
            )
            .map_err(|_| Error::Conflict)?;
        } else {
            super::browser_launch_custody_store::reject_fenced_browser_publication(
                &transaction,
                state,
            )
            .map_err(|_| Error::Conflict)?;
        }
        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            state,
        )
        .map_err(|_| Error::Unavailable)?;
        for id in activity_ids {
            // Resource-ended history is immutable outcome evidence, not a
            // transient reservation to delete. A delayed publisher can still
            // discard its local ID without erasing the retained record.
            if records
                .activities
                .get(id)
                .is_some_and(|record| !record.original_ended)
            {
                records.activities.remove(id);
            }
        }
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

    #[test]
    fn browser_activity_and_retirement_admission_are_mutually_exclusive() {
        let fixture = Fixture::new();
        let mut activity_owner = fixture.open(true);
        let (state, intent, _) = setup(&mut activity_owner);
        let mut retirement_owner = fixture.open(false);
        let id = uuid::Uuid::new_v4().to_string();
        let browser = &intent.scopes[0].browser;
        activity_owner
            .admit_browser_activity(&id, browser, &state)
            .unwrap();
        assert_eq!(
            retirement_owner.claim_idle_retirement(&intent, &state),
            Err(BrowserIdleRetirementStoreError::ActivityActive)
        );
        activity_owner.acknowledge_browser_activity(&id).unwrap();
        drop(activity_owner);
        assert_eq!(
            retirement_owner.claim_idle_retirement(&intent, &state),
            Err(BrowserIdleRetirementStoreError::ActivityActive)
        );
        let version: i64 = retirement_owner
            .remote_view_mutation_connection()
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 3);

        let other = Fixture::new();
        let mut retirement_owner = other.open(true);
        let (state, intent, _) = setup(&mut retirement_owner);
        retirement_owner
            .claim_idle_retirement(&intent, &state)
            .unwrap();
        let mut activity_owner = other.open(false);
        assert_eq!(
            activity_owner.admit_browser_activity(
                &uuid::Uuid::new_v4().to_string(),
                &intent.scopes[0].browser,
                &state
            ),
            Err(BrowserIdleRetirementStoreError::Conflict)
        );
    }

    #[test]
    fn activity_release_is_atomic_with_successful_session_publication() {
        let fixture = Fixture::new();
        let mut owner = fixture.open(true);
        let (state, intent, _) = setup(&mut owner);
        let id = uuid::Uuid::new_v4().to_string();
        owner
            .admit_browser_activity(&id, &intent.scopes[0].browser, &state)
            .unwrap();
        assert_eq!(
            owner.publish_session_retirements(&state, &state, &[], None, std::slice::from_ref(&id)),
            Err(BrowserIdleRetirementStoreError::Conflict)
        );
        owner.acknowledge_browser_activity(&id).unwrap();
        let mut next = state.clone();
        next.next_session_sequence += 1;
        owner
            .remote_view_mutation_connection()
            .execute_batch(
                "CREATE TRIGGER reject_activity_publication BEFORE UPDATE ON state_documents
             BEGIN SELECT RAISE(ABORT, 'fixture failure'); END;",
            )
            .unwrap();
        assert!(owner
            .publish_session_retirements(&state, &next, &[], None, std::slice::from_ref(&id))
            .is_err());
        assert_eq!(owner.load_session_state().unwrap(), state);
        let mut peer = fixture.open(false);
        assert_eq!(
            peer.claim_idle_retirement(&intent, &state),
            Err(BrowserIdleRetirementStoreError::ActivityActive)
        );
        owner
            .remote_view_mutation_connection()
            .execute_batch("DROP TRIGGER reject_activity_publication;")
            .unwrap();
        owner
            .publish_session_retirements(&state, &next, &[], None, &[id])
            .unwrap();
        assert_eq!(peer.load_session_state().unwrap(), next);
        let next_intent = BrowserIdleRetirementIntent::new(
            &intent.id,
            intent.provider.clone(),
            &next,
            &[intent.scopes[0].browser.id.clone()],
        )
        .unwrap();
        assert_eq!(
            peer.claim_idle_retirement(&next_intent, &next).unwrap(),
            BrowserIdleRetirementAdmission::New
        );
    }

    #[test]
    fn unresolved_activity_fences_physical_replacement_but_allows_metadata() {
        let fixture = Fixture::new();
        let mut owner = fixture.open(true);
        let (state, intent, _) = setup(&mut owner);
        owner
            .admit_browser_activity(
                &uuid::Uuid::new_v4().to_string(),
                &intent.scopes[0].browser,
                &state,
            )
            .unwrap();
        let mut replacement = state.clone();
        replacement.browsers.get_mut("browser-a").unwrap().pid += 1;
        assert!(
            owner
                .compare_and_save_session_state(&state, &replacement)
                .is_err(),
            "ordinary publication replaced a root with an unresolved command"
        );
        let mut metadata = state.clone();
        metadata.next_session_sequence += 1;
        owner
            .compare_and_save_session_state(&state, &metadata)
            .unwrap();
        assert_eq!(owner.load_session_state().unwrap(), metadata);
    }

    #[test]
    fn activity_original_evidence_is_immutable_and_resource_settlement_is_retained() {
        let fixture = Fixture::new();
        let mut owner = fixture.open(true);
        let original = RecordedProcessIdentity {
            pid: 42,
            start_token: "original-kernel".into(),
            executable_path: Some("/synthetic/chrome".into()),
            browser_family: None,
        };
        let (state, intent, _) = setup_with_identity(&mut owner, Some(original.clone()));
        let id = uuid::Uuid::new_v4().to_string();
        owner
            .admit_browser_activity(&id, &intent.scopes[0].browser, &state)
            .unwrap();
        let record = owner.browser_activity_records().unwrap().remove(0);
        assert_eq!(record.process_identity, Some(original));
        let version: i64 = owner
            .remote_view_mutation_connection()
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 4);
        let mut substituted = record.clone();
        substituted.process_identity.as_mut().unwrap().start_token = "replacement-kernel".into();
        assert_eq!(
            owner.settle_browser_activity_resource(&substituted),
            Err(BrowserIdleRetirementStoreError::Conflict)
        );
        assert_eq!(
            owner.browser_activity_records().unwrap(),
            vec![record.clone()]
        );
        owner.settle_browser_activity_resource(&record).unwrap();
        drop(owner);
        let mut restarted = fixture.open(false);
        let retained = restarted.browser_activity_records().unwrap().remove(0);
        assert!(retained.original_ended);
        assert!(!retained.acknowledged);
        assert_eq!(retained.process_identity, record.process_identity);
        assert_eq!(
            restarted.acknowledge_browser_activity(&id),
            Err(BrowserIdleRetirementStoreError::Conflict)
        );
        assert_eq!(
            restarted.claim_idle_retirement(&intent, &state).unwrap(),
            BrowserIdleRetirementAdmission::New
        );
    }

    fn setup(
        store: &mut BrowserSessionSqliteStore,
    ) -> (
        BrowserSessionState,
        BrowserIdleRetirementIntent,
        RemoteViewIdleRetirementReceipt,
    ) {
        setup_with_identity(store, None)
    }

    fn setup_with_identity(
        store: &mut BrowserSessionSqliteStore,
        process_identity: Option<RecordedProcessIdentity>,
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
            process_identity: process_identity.clone(),
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
        let mut intent = BrowserIdleRetirementIntent::new(
            "11111111-1111-4111-8111-111111111111",
            provider,
            &state,
            &["browser-a".into()],
        )
        .unwrap();
        let prepared = serde_json::from_value(json!({"schemaVersion":1,"target":f["assignmentObservation"]["target"],
            "readinessScope":"native_admission","state":"prepared","fence":{"schema_version":1,"installation_id":"0123456789abcdef",
                "config_hash":"a".repeat(64),"slot":2,"generation":19,"operation_id":"5deb7e63650d881f18b5d68f85ec45bcd7523255499618982a3c2995400637cc","revision":1}})).unwrap();
        intent.scopes[0].process_identity = process_identity;
        (state, intent, prepared)
    }

    #[test]
    fn retirement_claim_binds_original_kernel_identity_inside_its_transaction() {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let original = RecordedProcessIdentity {
            pid: 42,
            start_token: "linux:boot:original".into(),
            executable_path: Some("/synthetic/chrome".into()),
            browser_family: None,
        };
        let (state, intent, _) = setup_with_identity(&mut store, Some(original.clone()));
        intent.matches_baseline(&state).unwrap();
        let mut replacement = intent.clone();
        replacement.scopes[0]
            .process_identity
            .as_mut()
            .unwrap()
            .start_token = "linux:boot:replacement".into();
        assert_eq!(
            store
                .claim_idle_retirement(&replacement, &state)
                .unwrap_err(),
            BrowserIdleRetirementStoreError::Conflict
        );
        let mut missing = intent.clone();
        missing.scopes[0].process_identity = None;
        assert_eq!(
            store.claim_idle_retirement(&missing, &state).unwrap_err(),
            BrowserIdleRetirementStoreError::Conflict
        );
        assert!(store.idle_retirement_records().unwrap().is_empty());
        store.claim_idle_retirement(&intent, &state).unwrap();
        drop(store);
        let mut restarted = fixture.open(false);
        let retained = restarted.idle_retirement_records().unwrap();
        assert_eq!(
            retained[0].intent.scopes[0].process_identity,
            Some(original)
        );
        assert_eq!(
            restarted
                .claim_idle_retirement(&replacement, &state)
                .unwrap_err(),
            BrowserIdleRetirementStoreError::Conflict
        );
        assert_eq!(restarted.idle_retirement_records().unwrap(), retained);
    }

    #[test]
    fn sqlite_idle_retirement_blocks_conflicting_writers_but_allows_unrelated_progress() {
        let fixture = Fixture::new();
        let mut owner = fixture.open(true);
        let (baseline, intent, _) = setup(&mut owner);
        owner.claim_idle_retirement(&intent, &baseline).unwrap();
        let mut peer = fixture.open(false);
        let mut erased = baseline.clone();
        erased.tabs.remove("tab-a");
        assert!(
            peer.compare_and_save_session_state(&baseline, &erased)
                .is_err(),
            "ordinary publication must not bypass an unresolved close operation"
        );
        assert_eq!(owner.load_session_state().unwrap(), baseline);
        let recovery = BrowserLaunchIntent {
            intent_id: "55555555-5555-4555-8555-555555555555".into(),
            profile_id: "profile-a".into(),
            assignment: intent.provider.assignment.clone(),
        };
        assert!(
            peer.admit_recovery_launch_intent(
                &recovery,
                &baseline,
                &baseline.browsers["browser-a"]
            )
            .is_err(),
            "a competing recovery must not borrow a profile with unresolved retirement custody"
        );
        assert!(peer.unpublished_launch_records().unwrap().is_empty());
        let mut unrelated = baseline.clone();
        unrelated.next_session_sequence += 1;
        peer.compare_and_save_session_state(&baseline, &unrelated)
            .unwrap();
        assert_eq!(owner.load_session_state().unwrap(), unrelated);
        let mut other_assignment = intent.provider.assignment.clone();
        other_assignment.assignment_id = "assignment-other".into();
        other_assignment.desktop_id = "66666666-6666-4666-8666-666666666666".into();
        let launch = BrowserLaunchIntent {
            intent_id: "77777777-7777-4777-8777-777777777777".into(),
            profile_id: "profile-b".into(),
            assignment: other_assignment,
        };
        assert_eq!(
            peer.admit_launch_intent(&launch, &unrelated).unwrap(),
            LaunchCustodyAdmission::New
        );
        let process = BrowserLaunch {
            process_identity: None,
            browser_id: "browser-b".into(),
            pid: 43,
            cdp_endpoint: "http://127.0.0.1:9223".into(),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: launch.assignment.desktop_id.clone(),
                generation: launch.assignment.generation,
                friendly_route_label: "other".into(),
            }),
        };
        peer.observe_launch_intent(&launch, &process).unwrap();
        let mut published = unrelated.clone();
        published.browsers.insert(
            "browser-b".into(),
            ManagedBrowserInstance {
                id: process.browser_id,
                profile_id: "profile-b".into(),
                pid: process.pid,
                cdp_endpoint: process.cdp_endpoint,
                desktop: process.desktop,
                active_session_ids: vec![],
            },
        );
        peer.publish_launch_intent(&launch, &unrelated, &published)
            .unwrap();
        assert_eq!(owner.load_session_state().unwrap(), published);
        assert_eq!(owner.idle_retirement_records().unwrap()[0].intent, intent);
    }

    #[test]
    fn sqlite_idle_retirement_and_recovery_admissions_exclude_each_other() {
        let fixture = Fixture::new();
        let mut owner = fixture.open(true);
        let (baseline, intent, _) = setup(&mut owner);
        let recovery = BrowserLaunchIntent {
            intent_id: "55555555-5555-4555-8555-555555555555".into(),
            profile_id: "profile-a".into(),
            assignment: intent.provider.assignment.clone(),
        };
        let mut peer = fixture.open(false);
        peer.admit_recovery_launch_intent(&recovery, &baseline, &baseline.browsers["browser-a"])
            .unwrap();
        assert_eq!(owner.claim_idle_retirement(&intent, &baseline).unwrap_err(),
            BrowserIdleRetirementStoreError::Conflict,
            "retirement must observe an already admitted competing launch under its own transaction");
        assert!(owner.idle_retirement_records().unwrap().is_empty());
        for _ in 0..8 {
            let fixture = Fixture::new();
            let mut owner = fixture.open(true);
            let (baseline, intent, _) = setup(&mut owner);
            let recovery = BrowserLaunchIntent {
                assignment: intent.provider.assignment.clone(),
                ..recovery.clone()
            };
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
            let mut threads = Vec::new();
            for retiring in [true, false] {
                let path = fixture.0.join("runtime.sqlite3");
                let baseline = baseline.clone();
                let intent = intent.clone();
                let recovery = recovery.clone();
                let barrier = barrier.clone();
                threads.push(std::thread::spawn(move || {
                    let connection = Connection::open(path).unwrap();
                    connection
                        .busy_timeout(std::time::Duration::from_secs(5))
                        .unwrap();
                    let mut store =
                        BrowserSessionSqliteStore::launch_custody_fixture(connection, false)
                            .unwrap();
                    barrier.wait();
                    if retiring {
                        match store.claim_idle_retirement(&intent, &baseline) {
                            Ok(BrowserIdleRetirementAdmission::New) => true,
                            Err(BrowserIdleRetirementStoreError::Conflict) => false,
                            other => panic!("unexpected retirement admission: {other:?}"),
                        }
                    } else {
                        match store.admit_recovery_launch_intent(
                            &recovery,
                            &baseline,
                            &baseline.browsers["browser-a"],
                        ) {
                            Ok(LaunchCustodyAdmission::New) => true,
                            Err(LaunchCustodyStoreError::Conflict) => false,
                            other => panic!("unexpected recovery admission: {other:?}"),
                        }
                    }
                }));
            }
            barrier.wait();
            let admitted = threads
                .into_iter()
                .map(|thread| thread.join().unwrap())
                .filter(|admitted| *admitted)
                .count();
            assert_eq!(
                admitted, 1,
                "one original profile cannot admit both operations"
            );
            assert_eq!(
                owner.idle_retirement_records().unwrap().len()
                    + owner.unpublished_launch_records().unwrap().len(),
                1
            );
        }
    }

    #[test]
    fn sqlite_idle_retirement_publication_requires_evidence_for_idle_closed_marker() {
        let fixture = Fixture::new();
        let mut owner = fixture.open(true);
        let (baseline, intent, prepared) = setup(&mut owner);
        owner.claim_idle_retirement(&intent, &baseline).unwrap();
        let admitted = owner.idle_retirement_records().unwrap().pop().unwrap();
        let mut started = admitted.clone();
        started.begin_preparation().unwrap();
        owner.update_idle_retirement(&admitted, &started).unwrap();
        let mut ready = started.clone();
        ready.record_prepared(prepared).unwrap();
        owner.update_idle_retirement(&started, &ready).unwrap();
        let mut invented = baseline.clone();
        invented
            .idle_closed_browsers
            .insert("browser-a".into(), 100);
        assert!(owner.publish_idle_retirement_state(&baseline, &invented, std::slice::from_ref(&intent.id)).is_err(),
            "a prepared fence cannot invent browser closure without an attempted and settled effect");
        assert_eq!(owner.load_session_state().unwrap(), baseline);
        assert_eq!(owner.idle_retirement_records().unwrap(), vec![ready]);
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
