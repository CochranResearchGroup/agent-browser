//! Durable process intent before launch and atomic session publication afterward.
use std::collections::{BTreeMap, BTreeSet};

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
    reconciled_absent: BTreeSet<String>,
    #[serde(default)]
    release_fences: BTreeMap<String, RemoteViewApplicationReleaseTarget>,
    #[serde(default)]
    release_outcomes: BTreeMap<String, RemoteViewJoinedReleaseOutcome>,
    #[serde(default)]
    idle_release_fences: BTreeSet<String>,
}

fn ledger(connection: &Connection) -> Result<Ledger, LaunchCustodyStoreError> {
    let ledger: Ledger = load_optional_document::<Ledger>(connection, DOCUMENT, SCHEMA)
        .map_err(|_| LaunchCustodyStoreError::InvalidRecord)?;
    if ledger
        .records
        .values()
        .any(|record| record.observed_process_identity.is_some())
    {
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|_| LaunchCustodyStoreError::Unavailable)?;
        if !(2..=4).contains(&version) {
            return Err(LaunchCustodyStoreError::InvalidRecord);
        }
    }
    for (id, record) in &ledger.records {
        record
            .validate()
            .map_err(|_| LaunchCustodyStoreError::InvalidRecord)?;
        if id != &record.intent.intent_id {
            return Err(LaunchCustodyStoreError::InvalidRecord);
        }
    }
    if ledger
        .reconciled_absent
        .iter()
        .any(|id| ledger.records.get(id).is_none_or(|record| record.published))
    {
        return Err(LaunchCustodyStoreError::InvalidRecord);
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
    if ledger
        .idle_release_fences
        .iter()
        .any(|id| !ledger.release_fences.contains_key(id))
    {
        return Err(LaunchCustodyStoreError::InvalidRecord);
    }
    Ok(ledger)
}
/// Exact terminal retirement evidence, used to advance acquisition request
/// custody. Provider inventory absence alone cannot establish this condition.
pub(super) fn assignment_release_completed(
    connection: &Connection,
    assignment: &agent_browser_service_model::RemoteViewAssignmentRecord,
) -> Result<bool, LaunchCustodyStoreError> {
    let records = ledger(connection)?;
    Ok(records.release_fences.iter().any(|(id, target)| {
        target.assignment == *assignment && records.release_outcomes.contains_key(id)
    }))
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

pub(super) fn assignment_available(
    connection: &Connection,
    assignment: &agent_browser_service_model::RemoteViewAssignmentRecord,
) -> Result<bool, LaunchCustodyStoreError> {
    Ok(!assignment_fenced(&ledger(connection)?, assignment))
}

pub(crate) struct IdleReleaseObligation {
    pub release_id: String,
    pub target: RemoteViewApplicationReleaseTarget,
    pub record: Option<agent_browser_service_model::RemoteViewApplicationMutationRecord>,
}

/// A permit cannot submit after another process cancels its unsubmitted fence.
pub(super) fn idle_release_admitted(
    connection: &Connection,
    envelope: &agent_browser_service_model::RemoteViewApplicationEnvelope,
) -> Result<bool, LaunchCustodyStoreError> {
    use agent_browser_service_model::RemoteViewApplicationRequest as Request;
    let Request::ReleaseIdle {
        assignment_id,
        expected_generation,
        idempotency_key,
        expected_retirement,
        cleanup,
        ..
    } = &envelope.request
    else {
        return Ok(false);
    };
    let records = ledger(connection)?;
    Ok(records.idle_release_fences.contains(idempotency_key)
        && records
            .release_fences
            .get(idempotency_key)
            .is_some_and(|target| {
                target.assignment.assignment_id == *assignment_id
                    && target.assignment.generation == *expected_generation
                    && expected_retirement.desktop_id == target.assignment.desktop_id
                    && expected_retirement.generation == *expected_generation
                    && expected_retirement.routes == target.route_ids
                    && expected_retirement.sessions == target.viewer_session_ids
                    && cleanup.validates_target(assignment_id, *expected_generation)
            }))
}

impl BrowserSessionSqliteStore {
    /// Settle only unpublished exact-profile occupancy after a current OS absence
    /// proof. Retain every intent as history and preserve all published claims.
    pub(crate) fn settle_unpublished_profile_launch_absence(
        &mut self,
        profile: &agent_browser_service_model::BrowserProfileCatalogEntry,
        expected: &BrowserSessionState,
        expected_browser: Option<&agent_browser_service_model::ManagedBrowserInstance>,
    ) -> Result<(), LaunchCustodyStoreError> {
        use LaunchCustodyStoreError as Error;
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let state = current(&transaction)?;
        if state != *expected
            || match expected_browser {
                Some(browser) => {
                    browser.profile_id != profile.id
                        || state.browsers.get(&browser.id) != Some(browser)
                }
                None => {
                    state
                        .browsers
                        .values()
                        .any(|browser| browser.profile_id == profile.id)
                        || state
                            .sessions
                            .values()
                            .any(|task| task.profile_id == profile.id)
                }
            }
        {
            return Err(Error::Conflict);
        }
        super::browser_session_runtime::prove_profile_launch_absence(profile)
            .map_err(|_| Error::Unavailable)?;
        let mut records = ledger(&transaction)?;
        let claims = records
            .records
            .values()
            .filter(|record| {
                record.intent.profile_id == profile.id
                    && !record.published
                    && !records.reconciled_absent.contains(&record.intent.intent_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        if claims.len() > 1 {
            return Err(Error::Conflict);
        }
        for record in claims {
            if expected_browser.is_some_and(|browser| {
                record
                    .observed_browser_id
                    .as_ref()
                    .is_some_and(|id| id != &browser.id)
            }) {
                return Err(Error::Conflict);
            }
            records.reconciled_absent.insert(record.intent.intent_id);
        }
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)
    }

    pub(crate) fn idle_release_obligations(
        &mut self,
    ) -> Result<Vec<IdleReleaseObligation>, String> {
        use agent_browser_service_model::RemoteViewApplicationRequest as Request;
        let records = ledger(self.remote_view_mutation_connection())
            .map_err(|_| "remote_view_release_custody_unavailable")?;
        let mutations =
            super::remote_view_pool_request_store::records(self.remote_view_mutation_connection())
                .map_err(|_| "remote_view_release_custody_unavailable")?;
        Ok(records.idle_release_fences.iter().filter(|id| !records.release_outcomes.contains_key(*id)).map(|id|
            IdleReleaseObligation { release_id:id.clone(),target:records.release_fences[id].clone(),
                record:mutations.iter().find(|record| matches!(&record.envelope.request,Request::ReleaseIdle { idempotency_key,.. } if idempotency_key == id)).cloned(),
            }).collect())
    }

    /// Only a never-submitted request or an exact owner deferral can lift this
    /// fence. Ambiguous transport and partial release keep their obligation.
    pub(crate) fn cancel_unsubmitted_or_deferred_idle_release(
        &mut self,
        release_id: &str,
    ) -> Result<(), String> {
        use agent_browser_service_model::{
            RemoteViewApplicationMutationOutcome as Outcome,
            RemoteViewApplicationRequest as Request,
        };
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "remote_view_release_custody_unavailable")?;
        let mut records =
            ledger(&transaction).map_err(|_| "remote_view_release_custody_unavailable")?;
        if !records.idle_release_fences.contains(release_id)
            || records.release_outcomes.contains_key(release_id)
        {
            return Err("remote_view_release_cancellation_invalid".into());
        }
        let mutations = super::remote_view_pool_request_store::records(&transaction)
            .map_err(|_| "remote_view_release_custody_unavailable")?;
        let mutation = mutations.iter().find(|record| matches!(&record.envelope.request,Request::ReleaseIdle { idempotency_key,.. } if idempotency_key == release_id));
        if mutation.is_some_and(|record| record.outcome != Some(Outcome::ReleaseIdleDeferred)) {
            return Err("remote_view_release_readback_required".into());
        }
        records.release_fences.remove(release_id);
        records.idle_release_fences.remove(release_id);
        save_document(&transaction, DOCUMENT, SCHEMA, &records)
            .map_err(|_| "remote_view_release_custody_unavailable")?;
        transaction
            .commit()
            .map_err(|_| "remote_view_release_custody_unavailable".to_string())
    }
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

/// Retirement admission must see process claims that won the same SQLite
/// writer lock first, including claims whose physical outcome is still unknown.
/// Do not change retention while a launch still owns an unpublished profile effect.
pub(super) fn reject_profile_metadata_mutation(
    connection: &Connection,
    profile_id: &str,
) -> Result<(), LaunchCustodyStoreError> {
    let ledger = ledger(connection)?;
    if ledger.records.values().any(|record| {
        record.intent.profile_id == profile_id
            && !record.published
            && !ledger.reconciled_absent.contains(&record.intent.intent_id)
    }) {
        return Err(LaunchCustodyStoreError::Conflict);
    }
    Ok(())
}

pub(super) fn reject_pending_launch_for_retirement(
    connection: &Connection,
    intent: &agent_browser_service_model::BrowserIdleRetirementIntent,
) -> Result<(), LaunchCustodyStoreError> {
    let records = ledger(connection)?;
    if records.records.values().any(|record| {
        !record.published
            && !records.reconciled_absent.contains(&record.intent.intent_id)
            && (record.intent.assignment.desktop_id == intent.provider.assignment.desktop_id
                || intent
                    .scopes
                    .iter()
                    .any(|scope| scope.browser.profile_id == record.intent.profile_id))
    }) {
        return Err(LaunchCustodyStoreError::Conflict);
    }
    Ok(())
}

pub(super) fn published_assignment(
    connection: &Connection,
    browser: &agent_browser_service_model::ManagedBrowserInstance,
) -> Result<agent_browser_service_model::RemoteViewAssignmentRecord, LaunchCustodyStoreError> {
    Ok(published_launch_record(connection, browser)?
        .intent
        .assignment)
}

pub(super) fn published_process_identity(
    connection: &Connection,
    browser: &agent_browser_service_model::ManagedBrowserInstance,
) -> Result<Option<agent_browser_service_model::RecordedProcessIdentity>, LaunchCustodyStoreError> {
    Ok(published_launch_record(connection, browser)?.observed_process_identity)
}

fn published_launch_record(
    connection: &Connection,
    browser: &agent_browser_service_model::ManagedBrowserInstance,
) -> Result<BrowserLaunchCustodyRecord, LaunchCustodyStoreError> {
    let state = current(connection)?;
    if state.browsers.get(&browser.id) != Some(browser) {
        return Err(LaunchCustodyStoreError::Conflict);
    }
    let ledger = ledger(connection)?;
    let records = ledger
        .records
        .values()
        .filter(|record| {
            record.published
                && record.observed_browser_id.as_deref() == Some(browser.id.as_str())
                && record.observed_pid == Some(browser.pid)
        })
        .collect::<Vec<_>>();
    let mut record = match records.as_slice() {
        [record] => (*record).clone(),
        _ => return Err(LaunchCustodyStoreError::Conflict),
    };
    record
        .confirm_publication(&state)
        .map_err(|_| LaunchCustodyStoreError::Conflict)?;
    if assignment_fenced(&ledger, &record.intent.assignment)
        && !assignment_release_completed(connection, &record.intent.assignment)?
    {
        return Err(LaunchCustodyStoreError::ReleaseFenced);
    }
    Ok(record)
}

impl BrowserLaunchCustodyStore for BrowserSessionSqliteStore {
    fn assignment_release_confirmed(
        &mut self,
        assignment: &agent_browser_service_model::RemoteViewAssignmentRecord,
    ) -> Result<bool, LaunchCustodyStoreError> {
        assignment_release_completed(self.remote_view_mutation_connection(), assignment)
    }
    fn published_launch_assignment(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> Result<agent_browser_service_model::RemoteViewAssignmentRecord, LaunchCustodyStoreError>
    {
        published_assignment(self.remote_view_mutation_connection(), browser)
    }

    fn published_presentation_assignment(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> Result<agent_browser_service_model::RemoteViewAssignmentRecord, LaunchCustodyStoreError>
    {
        let state = self
            .load_session_state()
            .map_err(|_| LaunchCustodyStoreError::Unavailable)?;
        if let Some(attached) = state.app_browser_attachments.get(&browser.id) {
            if state.browsers.get(&browser.id) != Some(browser)
                || attached.profile_id != browser.profile_id
                || attached.request.process.pid != browser.pid
                || attached.request.cdp_endpoint != browser.cdp_endpoint
                || browser.desktop.as_ref() != Some(&attached.desktop)
            {
                return Err(LaunchCustodyStoreError::Conflict);
            }
            return attached
                .assignment
                .clone()
                .ok_or(LaunchCustodyStoreError::Unavailable);
        }
        self.published_launch_assignment(browser)
    }

    fn published_launch_process_identity(
        &mut self,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> Result<Option<agent_browser_service_model::RecordedProcessIdentity>, LaunchCustodyStoreError>
    {
        published_process_identity(self.remote_view_mutation_connection(), browser)
    }

    fn unpublished_launch_records(
        &mut self,
    ) -> Result<Vec<BrowserLaunchCustodyRecord>, LaunchCustodyStoreError> {
        let records = ledger(self.remote_view_mutation_connection())?;
        Ok(records
            .records
            .values()
            .filter(|record| {
                !record.published && !records.reconciled_absent.contains(&record.intent.intent_id)
            })
            .cloned()
            .collect())
    }
    fn admit_launch_intent(
        &mut self,
        intent: &BrowserLaunchIntent,
        expected: &BrowserSessionState,
    ) -> Result<LaunchCustodyAdmission, LaunchCustodyStoreError> {
        self.admit_launch_with_recovery(intent, expected, None)
    }
    fn admit_recovery_launch_intent(
        &mut self,
        intent: &BrowserLaunchIntent,
        expected: &BrowserSessionState,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> Result<LaunchCustodyAdmission, LaunchCustodyStoreError> {
        self.admit_launch_with_recovery(intent, expected, Some(browser))
    }
    /// Called only after the runtime proves the exact retained profile absent.
    /// Preserve the claim as history while retiring its occupancy.
    fn reconcile_absent_recovery(
        &mut self,
        claim: &BrowserLaunchCustodyRecord,
        expected: &BrowserSessionState,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> Result<(), LaunchCustodyStoreError> {
        use LaunchCustodyStoreError as Error;
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let state = current(&transaction)?;
        let mut records = ledger(&transaction)?;
        if state != *expected
            || state.browsers.get(&browser.id) != Some(browser)
            || claim.published
            || claim.intent.profile_id != browser.profile_id
            || records.records.get(&claim.intent.intent_id) != Some(claim)
        {
            return Err(Error::Conflict);
        }
        records
            .reconciled_absent
            .insert(claim.intent.intent_id.clone());
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)
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
        if launch.process_identity.is_some() {
            // The original kernel evidence and reader fence commit together.
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
        }
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
        let actual = current(&transaction)?;
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
        if actual != *expected {
            // Concurrent or interrupted publication of the identical result is
            // already complete. A different state still requires fresh readback.
            if record.published && actual == *state {
                record
                    .confirm_publication(&actual)
                    .map_err(|_| Error::Conflict)?;
                return Ok(());
            }
            return Err(Error::Conflict);
        }
        super::browser_idle_retirement_store::reject_retirement_publication(
            &transaction,
            expected,
            state,
            &[],
            &[],
        )
        .map_err(|_| Error::Conflict)?;
        confirm_launch_publication(&transaction, intent, state)?;

        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            state,
        )
        .map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)
    }
}

impl BrowserReleaseCustodyStore for BrowserSessionSqliteStore {
    fn admit_idle_assignment_release(
        &mut self,
        target: &RemoteViewApplicationReleaseTarget,
        snapshot: &RemoteViewApplicationCleanupSnapshot,
        release_id: &str,
    ) -> Result<RemoteViewApplicationCleanupPermit, LaunchCustodyStoreError> {
        self.admit_release(target, snapshot, release_id, true)
    }
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
        self.admit_release(target, snapshot, release_id, false)
    }
}

impl BrowserSessionSqliteStore {
    fn admit_release(
        &mut self,
        target: &RemoteViewApplicationReleaseTarget,
        snapshot: &RemoteViewApplicationCleanupSnapshot,
        release_id: &str,
        idle: bool,
    ) -> Result<RemoteViewApplicationCleanupPermit, LaunchCustodyStoreError> {
        use LaunchCustodyStoreError as Error;
        if uuid::Uuid::parse_str(release_id).is_err() || target.validate().is_err() {
            return Err(Error::InvalidRecord);
        }
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        super::browser_idle_retirement_store::reject_retirement_admission(
            &transaction,
            "",
            &target.assignment,
        )
        .map_err(|_| Error::Conflict)?;
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
                && !records.reconciled_absent.contains(&record.intent.intent_id)
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
        if idle {
            records.idle_release_fences.insert(release_id.into());
        }
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)?;
        Ok(permit)
    }
}

fn validate_browser_publication(
    records: &Ledger,
    state: &BrowserSessionState,
) -> Result<(), LaunchCustodyStoreError> {
    // Launch history does not veto ordinary state publication. New process
    // effects use publish_launch_intent; current desktop retirement fences below
    // protect only the exact resources being retired.
    if state.browsers.values().any(|browser| {
        if state.idle_closed_browsers.contains_key(&browser.id) {
            return false;
        }
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

/// Join launch acknowledgement to the caller's existing aggregate transaction.
pub(super) fn confirm_launch_publication(
    connection: &Connection,
    intent: &BrowserLaunchIntent,
    state: &BrowserSessionState,
) -> Result<(), LaunchCustodyStoreError> {
    use LaunchCustodyStoreError as Error;
    let mut records = ledger(connection)?;
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
    save_document(connection, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)
}

/// Ordinary publication reads current retirement fences independently of
/// historical launch records. Destructive effects retain the complete ledger.
#[derive(Default, Deserialize)]
struct PublicationReleaseState {
    #[serde(default)]
    release_fences: BTreeMap<String, RemoteViewApplicationReleaseTarget>,
    #[serde(default)]
    release_outcomes: BTreeMap<String, RemoteViewJoinedReleaseOutcome>,
}

pub(super) fn reject_fenced_browser_publication(
    connection: &Connection,
    expected: &BrowserSessionState,
    state: &BrowserSessionState,
) -> Result<(), LaunchCustodyStoreError> {
    let changed_desktops = state
        .browsers
        .iter()
        .filter_map(|(id, browser)| {
            if state.idle_closed_browsers.contains_key(id) {
                return None;
            }
            let desktop = browser.desktop.as_ref()?;
            let changed = expected.browsers.get(id).is_none_or(|previous| {
                previous.profile_id != browser.profile_id
                    || previous.pid != browser.pid
                    || previous.cdp_endpoint != browser.cdp_endpoint
                    || previous
                        .desktop
                        .as_ref()
                        .map(|d| (&d.desktop_id, d.generation))
                        != Some((&desktop.desktop_id, desktop.generation))
                    || expected.idle_closed_browsers.contains_key(id)
            });
            changed.then_some(desktop)
        })
        .collect::<Vec<_>>();
    if changed_desktops.is_empty() {
        return Ok(());
    }
    let releases: PublicationReleaseState = load_optional_document(connection, DOCUMENT, SCHEMA)
        .map_err(|_| LaunchCustodyStoreError::InvalidRecord)?;
    for desktop in changed_desktops {
        for (id, target) in &releases.release_fences {
            if target.assignment.desktop_id != desktop.desktop_id
                || target.assignment.generation != desktop.generation
            {
                continue;
            }
            let completed = releases
                .release_outcomes
                .get(id)
                .is_some_and(|outcome| target.validate_outcome(outcome).is_ok());
            if !completed {
                return Err(LaunchCustodyStoreError::ReleaseFenced);
            }
        }
    }
    Ok(())
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
    #[test]
    fn exact_absent_unpublished_launch_settlement_preserves_history_and_checks_state() {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let profile = agent_browser_service_model::BrowserProfileCatalogEntry {
            id: "profile-a".into(),
            name: "Synthetic destination".into(),
            user_data_dir: fixture.0.to_string_lossy().into_owned(),
            kind: agent_browser_service_model::BrowserProfileKind::Named,
            manual_login_url: None,
        };
        let expected = current(store.remote_view_mutation_connection()).unwrap();
        let retained = intent();
        store.admit_launch_intent(&retained, &expected).unwrap();
        let mut stale = expected.clone();
        stale
            .profile_operation_ids
            .insert("foreign".into(), "foreign-operation".into());
        assert_eq!(
            store.settle_unpublished_profile_launch_absence(&profile, &stale, None),
            Err(LaunchCustodyStoreError::Conflict)
        );
        assert_eq!(store.unpublished_launch_records().unwrap().len(), 1);
        store
            .settle_unpublished_profile_launch_absence(&profile, &expected, None)
            .unwrap();
        assert!(store.unpublished_launch_records().unwrap().is_empty());
        let history = ledger(store.remote_view_mutation_connection()).unwrap();
        assert_eq!(history.records[&retained.intent_id].intent, retained);
        assert!(history.reconciled_absent.contains(&retained.intent_id));
        store
            .settle_unpublished_profile_launch_absence(&profile, &expected, None)
            .unwrap();
        let fresh = intent();
        assert_eq!(
            store.admit_launch_intent(&fresh, &expected).unwrap(),
            LaunchCustodyAdmission::New
        );
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
    fn profile_properties_refuse_unpublished_launch_and_allow_unrelated_profiles() {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let expected = store.load_session_state().unwrap();
        let launch = intent();
        store.admit_launch_intent(&launch, &expected).unwrap();
        assert_eq!(
            reject_profile_metadata_mutation(store.remote_view_mutation_connection(), "profile-a"),
            Err(LaunchCustodyStoreError::Conflict)
        );
        assert!(reject_profile_metadata_mutation(
            store.remote_view_mutation_connection(),
            "unrelated-profile"
        )
        .is_ok());
    }

    #[test]
    fn ordinary_remote_startup_ignores_malformed_history() {
        struct NoProcessEffects;
        impl agent_browser_service_model::RemoteViewBrowserProcessEffects for NoProcessEffects {
            fn launch(
                &mut self,
                _: &agent_browser_service_model::BrowserProfileCatalogEntry,
                _: &BrowserLaunchIntent,
                _: agent_browser_service_model::RemoteViewPrivateLaunchEnvironment,
                _: &agent_browser_service_model::RemoteViewAssignmentObservation,
            ) -> Result<BrowserLaunch, agent_browser_service_model::RemoteViewBrowserProcessError>
            {
                panic!("startup must not launch a process");
            }
        }
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        store
            .remote_view_mutation_connection()
            .execute(
                "INSERT INTO state_documents(kind, schema_version, json) VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    DOCUMENT,
                    SCHEMA,
                    r#"{"records":{"broken":{"old":"history"}}}"#
                ],
            )
            .unwrap();
        super::super::browser_session_remote_view::reconcile_pending_recoveries(
            &mut NoProcessEffects,
            &mut store,
        )
        .expect("historical diagnostics must not block native host startup");
        assert!(ledger(store.remote_view_mutation_connection()).is_err());
    }

    #[test]
    fn ordinary_rebinding_ignores_completed_release_history() {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let initial = store.load_session_state().unwrap();
        let intent = intent();
        let (target, snapshot) = release_evidence(&intent);
        let release_id = uuid::Uuid::new_v4().to_string();
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
        store
            .complete_assignment_release(&target, &release_id, &outcome)
            .unwrap();
        let mut observed = initial.clone();
        observed.browsers.insert(
            "observed".into(),
            ManagedBrowserInstance {
                id: "observed".into(),
                profile_id: "current-profile".into(),
                pid: 91,
                cdp_endpoint: "http://127.0.0.1:9222".into(),
                desktop: Some(RemoteViewFixedDesktop {
                    desktop_id: target.assignment.desktop_id,
                    generation: target.assignment.generation,
                    friendly_route_label: "current observation".into(),
                }),
                active_session_ids: vec![],
            },
        );
        store
            .compare_and_save_session_state(&initial, &observed)
            .expect("completed release history must not require old launch publication proof");
    }

    #[test]
    fn ordinary_publication_ignores_malformed_history() {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let initial = store.load_session_state().unwrap();
        store
            .remote_view_mutation_connection()
            .execute(
                "INSERT INTO state_documents(kind, schema_version, json) VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    DOCUMENT,
                    SCHEMA,
                    r#"{"records":{"broken":{"old":"history"}}}"#
                ],
            )
            .unwrap();
        let mut changed = initial.clone();
        changed.next_session_sequence += 1;
        store
            .compare_and_save_session_state(&initial, &changed)
            .expect("historical custody must not veto an ordinary task update");
        assert_eq!(store.load_session_state().unwrap(), changed);
        let mut observed = changed.clone();
        observed.browsers.insert(
            "current".into(),
            ManagedBrowserInstance {
                id: "current".into(),
                profile_id: "profile-current".into(),
                pid: 91,
                cdp_endpoint: "http://127.0.0.1:9222".into(),
                desktop: Some(RemoteViewFixedDesktop {
                    desktop_id: uuid::Uuid::new_v4().to_string(),
                    generation: 1,
                    friendly_route_label: "current".into(),
                }),
                active_session_ids: vec![],
            },
        );
        store
            .compare_and_save_session_state(&changed, &observed)
            .expect("current observations must not need historical launch proof");
        assert_eq!(store.load_session_state().unwrap(), observed);
        assert!(ledger(store.remote_view_mutation_connection()).is_err());
    }

    #[test]
    fn ordinary_publication_preserves_peer_with_release_fence() {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let intent = intent();
        let mut initial = store.load_session_state().unwrap();
        initial.browsers.insert(
            "peer".into(),
            ManagedBrowserInstance {
                id: "peer".into(),
                profile_id: "peer-profile".into(),
                pid: 42,
                cdp_endpoint: "http://127.0.0.1:9222".into(),
                desktop: Some(RemoteViewFixedDesktop {
                    desktop_id: intent.assignment.desktop_id.clone(),
                    generation: intent.assignment.generation,
                    friendly_route_label: "peer".into(),
                }),
                active_session_ids: vec![],
            },
        );
        store.save_session_state(&initial).unwrap();
        let (target, _) = release_evidence(&intent);
        let history = Ledger {
            release_fences: BTreeMap::from([(uuid::Uuid::new_v4().to_string(), target)]),
            ..Ledger::default()
        };
        save_document(
            store.remote_view_mutation_connection(),
            DOCUMENT,
            SCHEMA,
            &history,
        )
        .unwrap();
        let mut changed = initial.clone();
        changed.next_session_sequence += 1;
        store
            .compare_and_save_session_state(&initial, &changed)
            .expect("a peer desktop fence must not veto this task update");
        assert_eq!(store.load_session_state().unwrap(), changed);
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
            process_identity: None,
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
    fn sqlite_published_launch_keeps_original_kernel_identity_after_restart() {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let intent = intent();
        let initial = store.load_session_state().unwrap();
        store.admit_launch_intent(&intent, &initial).unwrap();
        let original = agent_browser_service_model::RecordedProcessIdentity {
            pid: 42,
            start_token: "linux:boot:original".into(),
            executable_path: Some("/synthetic/chrome".into()),
            browser_family: None,
        };
        let launch = BrowserLaunch {
            process_identity: Some(original.clone()),
            browser_id: "retained-browser".into(),
            pid: 42,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: intent.assignment.desktop_id.clone(),
                generation: intent.assignment.generation,
                friendly_route_label: "synthetic".into(),
            }),
        };
        let mut invalid = launch.clone();
        invalid
            .process_identity
            .as_mut()
            .unwrap()
            .start_token
            .clear();
        assert_eq!(
            store.observe_launch_intent(&intent, &invalid).unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        let before_version: i64 = store
            .remote_view_mutation_connection()
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            before_version, 1,
            "failed identity observation must not promote the reader version"
        );
        assert!(store.unpublished_launch_records().unwrap()[0]
            .observed_process_identity
            .is_none());
        store.observe_launch_intent(&intent, &launch).unwrap();
        let version: i64 = store
            .remote_view_mutation_connection()
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
        let browser = ManagedBrowserInstance {
            id: launch.browser_id.clone(),
            profile_id: intent.profile_id.clone(),
            pid: launch.pid,
            cdp_endpoint: launch.cdp_endpoint.clone(),
            desktop: launch.desktop.clone(),
            active_session_ids: vec![],
        };
        let mut state = initial.clone();
        state.browsers.insert(browser.id.clone(), browser.clone());
        store
            .publish_launch_intent(&intent, &initial, &state)
            .unwrap();
        drop(store);
        let mut restarted = fixture.open(false);
        assert_eq!(
            restarted
                .published_launch_process_identity(&browser)
                .unwrap(),
            Some(original.clone())
        );
        let mut reused = launch;
        reused.process_identity.as_mut().unwrap().start_token = "linux:boot:replacement".into();
        assert_eq!(
            restarted
                .observe_launch_intent(&intent, &reused)
                .unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        assert_eq!(
            restarted
                .published_launch_process_identity(&browser)
                .unwrap(),
            Some(original)
        );
        assert_eq!(restarted.load_session_state().unwrap(), state);
    }

    #[test]
    fn sqlite_absent_unobserved_recovery_retains_history_and_unblocks_exact_profile() {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let old = intent();
        let initial = store.load_session_state().unwrap();
        store.admit_launch_intent(&old, &initial).unwrap();
        let launch = BrowserLaunch {
            process_identity: None,
            browser_id: "retained-browser".into(),
            pid: 42,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: old.assignment.desktop_id.clone(),
                generation: old.assignment.generation,
                friendly_route_label: "synthetic".into(),
            }),
        };
        store.observe_launch_intent(&old, &launch).unwrap();
        let browser = ManagedBrowserInstance {
            id: launch.browser_id.clone(),
            profile_id: old.profile_id.clone(),
            pid: launch.pid,
            cdp_endpoint: launch.cdp_endpoint.clone(),
            desktop: launch.desktop.clone(),
            active_session_ids: vec![],
        };
        let mut retained = initial.clone();
        retained
            .browsers
            .insert(browser.id.clone(), browser.clone());
        store
            .publish_launch_intent(&old, &initial, &retained)
            .unwrap();
        let mut interrupted = old.clone();
        interrupted.intent_id = uuid::Uuid::new_v4().to_string();
        store
            .admit_recovery_launch_intent(&interrupted, &retained, &browser)
            .unwrap();
        let claim = store.unpublished_launch_records().unwrap().pop().unwrap();
        assert!(store
            .reconcile_absent_recovery(&claim, &initial, &browser)
            .is_err());
        let mut observed = claim.clone();
        observed.observe(&launch).unwrap();
        assert!(store
            .reconcile_absent_recovery(&observed, &retained, &browser)
            .is_err());
        store
            .reconcile_absent_recovery(&claim, &retained, &browser)
            .unwrap();
        store
            .reconcile_absent_recovery(&claim, &retained, &browser)
            .unwrap();
        drop(store);
        let mut store = fixture.open(false);
        assert!(store.unpublished_launch_records().unwrap().is_empty());
        let history = ledger(store.remote_view_mutation_connection()).unwrap();
        assert_eq!(history.records[&interrupted.intent_id], claim);
        assert!(history.reconciled_absent.contains(&interrupted.intent_id));
        assert_eq!(store.load_session_state().unwrap(), retained);
        let mut fresh = old;
        fresh.intent_id = uuid::Uuid::new_v4().to_string();
        assert!(matches!(
            store
                .admit_recovery_launch_intent(&fresh, &retained, &browser)
                .unwrap(),
            LaunchCustodyAdmission::New
        ));
    }

    #[test]
    fn sqlite_recovery_launch_preserves_prior_custody_and_fences_competing_replay() {
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let old = intent();
        let initial = store.load_session_state().unwrap();
        store.admit_launch_intent(&old, &initial).unwrap();
        let launch = BrowserLaunch {
            process_identity: None,
            browser_id: "retained-browser".into(),
            pid: 42,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: old.assignment.desktop_id.clone(),
                generation: old.assignment.generation,
                friendly_route_label: "synthetic".into(),
            }),
        };
        store.observe_launch_intent(&old, &launch).unwrap();
        let browser = ManagedBrowserInstance {
            id: launch.browser_id.clone(),
            profile_id: old.profile_id.clone(),
            pid: launch.pid,
            cdp_endpoint: launch.cdp_endpoint.clone(),
            desktop: launch.desktop.clone(),
            active_session_ids: vec![],
        };
        let mut retained = initial.clone();
        retained
            .browsers
            .insert(browser.id.clone(), browser.clone());
        store
            .publish_launch_intent(&old, &initial, &retained)
            .unwrap();
        let old_record = ledger(store.remote_view_mutation_connection())
            .unwrap()
            .records[&old.intent_id]
            .clone();
        let mut replacement = old.clone();
        replacement.intent_id = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            store
                .admit_launch_intent(&replacement, &retained)
                .unwrap_err(),
            LaunchCustodyStoreError::ProfileOccupied
        );
        let mut stale = browser.clone();
        stale.pid += 1;
        assert_eq!(
            store
                .admit_recovery_launch_intent(&replacement, &retained, &stale)
                .unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        assert_eq!(
            store
                .admit_recovery_launch_intent(&replacement, &retained, &browser)
                .unwrap(),
            LaunchCustodyAdmission::New
        );
        let mut competing = replacement.clone();
        competing.intent_id = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            store
                .admit_recovery_launch_intent(&competing, &retained, &browser)
                .unwrap_err(),
            LaunchCustodyStoreError::ProfileOccupied
        );
        // Pending history must not veto ordinary state or heartbeat updates.
        store
            .compare_and_save_session_state(&retained, &retained)
            .unwrap();
        drop(store);
        let mut store = fixture.open(false);
        assert_eq!(store.unpublished_launch_records().unwrap().len(), 1);
        assert!(matches!(
            store
                .admit_recovery_launch_intent(&replacement, &retained, &browser)
                .unwrap(),
            LaunchCustodyAdmission::Existing(_)
        ));
        let mut new_launch = launch.clone();
        new_launch.pid = 43;
        new_launch.cdp_endpoint = "http://127.0.0.1:9322".into();
        store
            .observe_launch_intent(&replacement, &new_launch)
            .unwrap();
        let mut recovered = retained.clone();
        let current = recovered.browsers.get_mut(&browser.id).unwrap();
        current.pid = new_launch.pid;
        current.cdp_endpoint = new_launch.cdp_endpoint;
        store
            .publish_launch_intent(&replacement, &retained, &recovered)
            .unwrap();
        assert_eq!(
            ledger(store.remote_view_mutation_connection())
                .unwrap()
                .records[&old.intent_id],
            old_record
        );
        assert_eq!(
            store
                .published_launch_assignment(&recovered.browsers[&browser.id])
                .unwrap(),
            old.assignment
        );
        assert!(store.unpublished_launch_records().unwrap().is_empty());
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
            process_identity: None,
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
        // A second publisher replays the same observed outcome after the first
        // transaction committed, retaining the exact result without a new effect.
        reopened
            .publish_launch_intent(&intent, &newer, &published)
            .unwrap();
        let LaunchCustodyAdmission::Existing(record) =
            reopened.admit_launch_intent(&intent, &published).unwrap()
        else {
            panic!("claim missing");
        };
        assert!(record.published);
        assert_eq!(record.observed_pid, Some(42));
    }
    #[test]
    fn hosted_reconnect_sqlite_exact_recovery_target_preserves_original_custody_and_peer_fences() {
        use agent_browser_service_model::*;
        let fixture = Fixture::new();
        let mut store = fixture.open(true);
        let old = intent();
        let initial = store.load_session_state().unwrap();
        store.admit_launch_intent(&old, &initial).unwrap();
        let identity = RecordedProcessIdentity {
            pid: u32::MAX,
            start_token: "linux:synthetic:original".into(),
            executable_path: Some("/synthetic/chrome".into()),
            browser_family: None,
        };
        let desktop = RemoteViewFixedDesktop {
            desktop_id: old.assignment.desktop_id.clone(),
            generation: old.assignment.generation,
            friendly_route_label: "original".into(),
        };
        let launch = BrowserLaunch {
            process_identity: Some(identity.clone()),
            browser_id: "hosted-original".into(),
            pid: u32::MAX,
            cdp_endpoint: "ws://127.0.0.1:9222/devtools/browser/original".into(),
            desktop: Some(desktop.clone()),
        };
        store.observe_launch_intent(&old, &launch).unwrap();
        let browser = ManagedBrowserInstance {
            id: launch.browser_id.clone(),
            profile_id: old.profile_id.clone(),
            pid: launch.pid,
            cdp_endpoint: launch.cdp_endpoint.clone(),
            desktop: Some(desktop.clone()),
            active_session_ids: vec!["task".into()],
        };
        let mut retained = initial.clone();
        retained
            .browsers
            .insert(browser.id.clone(), browser.clone());
        retained.sessions.insert(
            "task".into(),
            ManagedBrowserSession {
                id: "task".into(),
                name: "hosted".into(),
                profile_id: old.profile_id.clone(),
                browser_id: browser.id.clone(),
                created_at_ms: 0,
                last_activity_at_ms: 0,
                expires_at_ms: u64::MAX,
                current_tab_id: None,
            },
        );
        retained.host_bindings.insert(
            "task".into(),
            BrowserHostTaskBinding {
                request: BrowserHostBinding {
                    version: 1,
                    host_id: "app".into(),
                    participant_id: "participant".into(),
                    session_id: "session".into(),
                    placement: BrowserPlacement::Required {
                        desktop: BrowserDesktopBinding {
                            desktop_id: desktop.desktop_id.clone(),
                            generation: desktop.generation,
                        },
                    },
                },
                desktop: desktop.clone(),
            },
        );
        store
            .publish_launch_intent(&old, &initial, &retained)
            .unwrap();
        let mut proposed = old.clone();
        proposed.intent_id = uuid::Uuid::new_v4().to_string();
        proposed.assignment.generation += 1;
        assert_eq!(
            store
                .admit_recovery_launch_intent(&proposed, &retained, &browser)
                .unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        let mut target = retained.clone();
        let continuity = target.hosted_continuity.entry("task".into()).or_default();
        continuity.recovery_desktop = Some(RemoteViewFixedDesktop {
            generation: proposed.assignment.generation,
            ..desktop.clone()
        });
        continuity.state = BrowserHostedConnectionState::BrowserLost;
        store
            .compare_and_save_session_state(&retained, &target)
            .unwrap();
        assert_eq!(
            store
                .admit_recovery_launch_intent(&proposed, &target, &browser)
                .unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        let mut pending = target.clone();
        let continuity = pending.hosted_continuity.get_mut("task").unwrap();
        continuity.state = BrowserHostedConnectionState::Pending;
        continuity.record("reopen", "fingerprint", "reopen", true);
        store
            .compare_and_save_session_state(&target, &pending)
            .unwrap();
        assert!(explicit_hosted_recovery_target(
            &pending,
            &browser,
            &proposed.assignment
        ));
        let mut foreign = browser.clone();
        foreign.pid = std::process::id();
        assert!(!explicit_hosted_recovery_target(
            &pending,
            &foreign,
            &proposed.assignment
        ));
        let mut peer_state = pending.clone();
        let mut peer = browser.clone();
        peer.id = "unrelated-peer".into();
        peer.profile_id = "other-profile".into();
        peer.pid = u32::MAX - 1;
        peer_state.browsers.insert(peer.id.clone(), peer);
        store
            .compare_and_save_session_state(&pending, &peer_state)
            .unwrap();
        assert_eq!(
            store
                .admit_recovery_launch_intent(&proposed, &peer_state, &browser)
                .unwrap_err(),
            LaunchCustodyStoreError::Conflict
        );
        store
            .compare_and_save_session_state(&peer_state, &pending)
            .unwrap();
        assert!(matches!(
            store
                .admit_recovery_launch_intent(&proposed, &pending, &browser)
                .unwrap(),
            LaunchCustodyAdmission::New
        ));
        assert_eq!(
            store.load_session_state().unwrap().browsers[&browser.id],
            browser
        );
        assert_eq!(
            store.published_launch_process_identity(&browser).unwrap(),
            Some(identity)
        );
        let records = store.unpublished_launch_records().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].intent.assignment, proposed.assignment);
    }
}

/// A new Native generation is recovery intent, never a rewrite of the original
/// browser/launch proof. Only a current hosted task's published pending reopen
/// may select it, and the exact original PID must be absent at admission.
fn explicit_hosted_recovery_target(
    state: &BrowserSessionState,
    browser: &agent_browser_service_model::ManagedBrowserInstance,
    assignment: &agent_browser_service_model::RemoteViewAssignmentRecord,
) -> bool {
    if !matches!(
        crate::process_identity::observe_process(browser.pid),
        crate::process_identity::ProcessObservation::Missing
    ) {
        return false;
    }
    state
        .sessions
        .values()
        .filter(|task| task.browser_id == browser.id)
        .any(|task| {
            let Some(binding) = state.host_bindings.get(&task.id) else {
                return false;
            };
            if binding.request.validate().is_err() {
                return false;
            }
            let Some(continuity) = state.hosted_continuity.get(&task.id) else {
                return false;
            };
            if continuity.state
                != agent_browser_service_model::BrowserHostedConnectionState::Pending
            {
                return false;
            }
            let Some(desktop) = continuity.recovery_desktop.as_ref() else {
                return false;
            };
            if desktop.desktop_id != assignment.desktop_id
                || desktop.generation != assignment.generation
            {
                return false;
            }
            continuity
                .last_request_id
                .as_ref()
                .and_then(|id| continuity.requests.get(id))
                .is_some_and(|request| {
                    request.pending
                        && request.operation == "reopen"
                        && request.revision == continuity.revision
                        && !request.fingerprint.is_empty()
                })
        })
}

impl BrowserSessionSqliteStore {
    fn admit_launch_with_recovery(
        &mut self,
        intent: &BrowserLaunchIntent,
        expected: &BrowserSessionState,
        recovery: Option<&agent_browser_service_model::ManagedBrowserInstance>,
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
        super::browser_idle_retirement_store::reject_retirement_admission(
            &transaction,
            &intent.profile_id,
            &intent.assignment,
        )
        .map_err(|_| Error::Conflict)?;
        let mut records = ledger(&transaction)?;
        if assignment_fenced(&records, &intent.assignment) {
            return Err(Error::ReleaseFenced);
        }

        if let Some(browser) = recovery {
            let prior = records
                .records
                .values()
                .filter(|record| {
                    record.published
                        && record.observed_browser_id.as_ref() == Some(&browser.id)
                        && record.observed_pid == Some(browser.pid)
                })
                .collect::<Vec<_>>();
            let explicit_target =
                explicit_hosted_recovery_target(&state, browser, &intent.assignment);
            let prior_conflict = if let [prior] = prior.as_slice() {
                (prior.intent.assignment != intent.assignment
                    && !explicit_target
                    && (!state.idle_closed_browsers.contains_key(&browser.id)
                        || !assignment_release_completed(&transaction, &prior.intent.assignment)?))
                    || (*prior).clone().confirm_publication(&state).is_err()
            } else {
                !prior.is_empty()
                    || (browser.desktop.is_some()
                        && !state.shared_browser_ids.contains(&browser.id))
            };
            if state.browsers.get(&browser.id) != Some(browser)
                || browser.profile_id != intent.profile_id
                || prior_conflict
            {
                return Err(Error::Conflict);
            }
        }
        if let Some(record) = records.records.get(&intent.intent_id) {
            if record.intent != *intent {
                return Err(Error::Conflict);
            }
            return Ok(LaunchCustodyAdmission::Existing(Box::new(record.clone())));
        }
        if records.records.values().any(|record| {
            !record.published
                && !records.reconciled_absent.contains(&record.intent.intent_id)
                && (record.intent.assignment.assignment_id == intent.assignment.assignment_id
                    || record.intent.assignment.desktop_id == intent.assignment.desktop_id)
                && record.intent.assignment != intent.assignment
        }) || state.browsers.values().any(|browser| {
            if state.idle_closed_browsers.contains_key(&browser.id)
                || (recovery == Some(browser)
                    && explicit_hosted_recovery_target(&state, browser, &intent.assignment))
            {
                return false;
            }
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
            .any(|browser| browser.profile_id == intent.profile_id && recovery != Some(browser))
            || records.records.values().any(|record| {
                !record.published
                    && !records.reconciled_absent.contains(&record.intent.intent_id)
                    && record.intent.profile_id == intent.profile_id
            })
        {
            return Err(Error::ProfileOccupied);
        }
        records.records.insert(intent.intent_id.clone(), pending);
        save_document(&transaction, DOCUMENT, SCHEMA, &records).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)?;
        Ok(LaunchCustodyAdmission::New)
    }
}
