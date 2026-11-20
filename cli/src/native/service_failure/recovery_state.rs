//! Recovery accounting uses the existing runtime database. Attempts are separate
//! documents, so extending a budget never deletes or reloads lifetime history.
use crate::native::browser_session_store::{
    load_optional_document, save_document, BrowserSessionSqliteStore,
};
use rusqlite::TransactionBehavior;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const SCHEMA: &str = "agent-browser.failure-recovery.v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Policy {
    pub max_attempts: u64,
    pub max_no_progress: u64,
    /// Optional qualified-service interval for dispatching recovery steps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_duration_ms: Option<u64>,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            max_no_progress: 2,
            max_duration_ms: None,
        }
    }
}
impl Policy {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.max_no_progress == 0 || self.max_duration_ms == Some(0) {
            return Err("failure_recovery_policy_invalid: maxNoProgress and any maxDurationMs must be positive; maxAttempts=0 disables automatic recovery".into());
        }
        Ok(())
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Summary {
    pub occurrence_id: String,
    pub task_id: Option<String>,
    pub attempts: u64,
    pub no_progress: u64,
    pub diagnosis_hash: Option<String>,
    pub latest_attempt_id: Option<String>,
    pub in_flight: Option<String>,
    pub acknowledged_resume_reports: u64,
}

/// Association is caller-attributed; the adapter independently verifies the
/// referenced owner's acknowledgement. This is observability, never authority.
pub(crate) fn record_resume(
    occurrence: &str,
    attempt: &str,
    task: &str,
    request: &str,
    actor: &str,
) -> Result<Value, String> {
    let occurrence_key = occurrence_key(occurrence)?;
    let attempt_key = attempt_key(occurrence, attempt)?;
    let report_key = format!(
        "{attempt_key}:resume:{}",
        hex::encode(Sha256::digest(request))
    );
    let mut store = BrowserSessionSqliteStore::default_sqlite()?;
    let transaction = store
        .remote_view_mutation_connection()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| "failure_recovery_completion_unavailable".to_string())?;
    let original: Option<Value> = load_optional_document(&transaction, &attempt_key, SCHEMA)?;
    if original.as_ref().and_then(|value| value["taskId"].as_str()) != Some(task) {
        return Err("failure_recovery_scope_conflict: report only a retained recovery attempt in the same task".into());
    }
    if let Some(mut prior) =
        load_optional_document::<Option<Value>>(&transaction, &report_key, SCHEMA)?
    {
        prior["replayedResumeReport"] = json!(true);
        return Ok(prior);
    }
    let mut summary: Summary = load_optional_document(&transaction, &occurrence_key, SCHEMA)?;
    summary.acknowledged_resume_reports = summary.acknowledged_resume_reports.saturating_add(1);
    let report = json!({"status":"resume_reported_owner_acknowledgement_verified",
        "occurrenceId":occurrence,"attemptId":attempt,"taskId":task,"requestId":request,
        "association":"caller_attributed","actor":actor,"ownerAcknowledgement":"verified_from_original_journal",
        "chronology":"not_inferred_from_guest_timestamps","taskCompletion":"unverified",
        "originalOperationReplayed":false});
    save_document(&transaction, &occurrence_key, SCHEMA, &summary)?;
    save_document(&transaction, &report_key, SCHEMA, &report)?;
    transaction
        .commit()
        .map_err(|_| "failure_recovery_completion_unavailable".to_string())?;
    Ok(report)
}

fn policy_key(task: Option<&str>) -> String {
    match task {
        Some(task) => format!(
            "failure_recovery_policy:{}",
            hex::encode(Sha256::digest(task))
        ),
        None => "failure_recovery_policy:defaults".into(),
    }
}
fn occurrence_key(id: &str) -> Result<String, String> {
    uuid::Uuid::parse_str(id).map_err(|_| "failure_occurrence_id_invalid".to_string())?;
    Ok(format!("failure_recovery:{id}"))
}
fn attempt_key(occurrence: &str, attempt: &str) -> Result<String, String> {
    uuid::Uuid::parse_str(attempt).map_err(|_| {
        "failure_recovery_attempt_id_invalid: use a stable UUID for this recovery invocation"
            .to_string()
    })?;
    Ok(format!("{}:attempt:{attempt}", occurrence_key(occurrence)?))
}

pub(crate) fn policy(task: Option<&str>) -> Result<(Policy, &'static str, Value), String> {
    let mut store = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path()?,
    )?;
    let connection = store.remote_view_mutation_connection();
    if task.is_some() {
        if let Some(policy) =
            load_optional_document::<Option<Policy>>(connection, &policy_key(task), SCHEMA)?
        {
            policy.validate()?;
            let provenance = load_optional_document(
                connection,
                &format!("{}:provenance", policy_key(task)),
                SCHEMA,
            )?;
            return Ok((policy, "task_override", provenance));
        }
    }
    let policy: Policy = load_optional_document(connection, &policy_key(None), SCHEMA)?;
    policy.validate()?;
    let provenance = load_optional_document(
        connection,
        &format!("{}:provenance", policy_key(None)),
        SCHEMA,
    )?;
    Ok((policy, "system_defaults", provenance))
}

pub(crate) fn configure(
    task: Option<&str>,
    policy: &Policy,
    actor: &str,
    reason: &str,
) -> Result<(), String> {
    policy.validate()?;
    let mut store = BrowserSessionSqliteStore::default_sqlite()?;
    let transaction = store
        .remote_view_mutation_connection()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| "failure_recovery_configuration_unavailable".to_string())?;
    save_document(&transaction, &policy_key(task), SCHEMA, policy)?;
    save_document(
        &transaction,
        &format!("{}:provenance", policy_key(task)),
        SCHEMA,
        &json!({"actor":actor,"reason":reason,"attribution":"caller_supplied_not_authentication"}),
    )?;
    transaction
        .commit()
        .map_err(|_| "failure_recovery_configuration_unavailable".to_string())
}

pub(crate) fn inspect(occurrence: &str, attempt: Option<&str>) -> Result<Value, String> {
    let mut store = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path()?,
    )?;
    let connection = store.remote_view_mutation_connection();
    let summary: Summary =
        load_optional_document(connection, &occurrence_key(occurrence)?, SCHEMA)?;
    let selected = attempt.or(summary.latest_attempt_id.as_deref());
    let attempt = match selected {
        Some(id) => load_optional_document::<Option<Value>>(
            connection,
            &attempt_key(occurrence, id)?,
            SCHEMA,
        )?,
        None => None,
    };
    Ok(
        json!({"summary":summary,"attempt":attempt,"historyRetention":"separate_durable_attempt_documents","requiresRestart":false}),
    )
}

pub(crate) fn replay(
    occurrence: &str,
    attempt: &str,
    task: Option<&str>,
    intent_tag: &str,
) -> Result<Option<Value>, String> {
    let key = attempt_key(occurrence, attempt)?;
    let mut store = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path()?,
    )?;
    let connection = store.remote_view_mutation_connection();
    let prior: Option<Value> = load_optional_document(connection, &key, SCHEMA)?;
    if let Some(prior) = &prior {
        let recorded: Option<String> =
            load_optional_document(connection, &format!("{key}:private_intent"), SCHEMA)?;
        if prior["taskId"].as_str() != task || recorded.as_deref() != Some(intent_tag) {
            return Err("failure_recovery_attempt_conflict: retain the original task and arguments; choose a new explicit recovery intent after inspection".into());
        }
    }
    Ok(prior)
}

/// Claim before a remedy. Inspection remains available without consuming budget.
/// A lost reply leaves an inspectable claim;
/// another process cannot silently repeat it or reset the original counters.
pub(crate) fn claim(
    occurrence: &str,
    task: Option<&str>,
    attempt: &str,
    intent_tag: &str,
    diagnosis: &str,
    policy: &Policy,
) -> Result<Option<Value>, String> {
    let key = occurrence_key(occurrence)?;
    let attempt_key = attempt_key(occurrence, attempt)?;
    let mut store = BrowserSessionSqliteStore::default_sqlite()?;
    let transaction = store
        .remote_view_mutation_connection()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| "failure_recovery_claim_unavailable".to_string())?;
    if let Some(prior) =
        load_optional_document::<Option<Value>>(&transaction, &attempt_key, SCHEMA)?
    {
        let recorded: Option<String> = load_optional_document(
            &transaction,
            &format!("{attempt_key}:private_intent"),
            SCHEMA,
        )?;
        if prior["taskId"].as_str() != task || recorded.as_deref() != Some(intent_tag) {
            return Err("failure_recovery_scope_conflict: inspect the original recovery attempt; never reuse its ID for another task".into());
        }
        return Ok(Some(prior));
    }
    let mut summary: Summary = load_optional_document(&transaction, &key, SCHEMA)?;
    if !summary.occurrence_id.is_empty() && summary.task_id.as_deref() != task {
        return Err(
            "failure_recovery_scope_conflict: retain the original occurrence and task binding"
                .into(),
        );
    }
    if summary.in_flight.is_some() {
        return Err("failure_recovery_attempt_unresolved: inspect the retained in-flight attempt and verify its prerequisite; do not repeat its remedy".into());
    }
    if summary
        .diagnosis_hash
        .as_deref()
        .is_some_and(|old| old != diagnosis)
    {
        summary.no_progress = 0;
    }
    if summary.attempts >= policy.max_attempts {
        return Err("failure_recovery_budget_exhausted: inspect counters and configure a larger maxAttempts under existing authority; originals and inspection remain available".into());
    }
    if summary.no_progress >= policy.max_no_progress {
        return Err("failure_recovery_no_progress: repair the reported prerequisite or configure the reviewed maxNoProgress budget; blind retry cannot establish progress".into());
    }
    summary.occurrence_id = occurrence.into();
    summary.task_id = task.map(str::to_owned);
    summary.attempts = summary.attempts.saturating_add(1);
    summary.latest_attempt_id = Some(attempt.into());
    summary.in_flight = Some(attempt.into());
    summary.diagnosis_hash = Some(diagnosis.into());
    let intent = json!({"schemaVersion":SCHEMA,"occurrenceId":occurrence,"attemptId":attempt,
        "taskId":task,"state":"in_flight","physicalInputDispatched":false,
        "originalOperationReplayed":false});
    save_document(&transaction, &key, SCHEMA, &summary)?;
    save_document(&transaction, &attempt_key, SCHEMA, &intent)?;
    // An intent digest is replay identity only, not permission evidence. Keep it
    // private and separate from all public readback and escalation packets.
    save_document(
        &transaction,
        &format!("{attempt_key}:private_intent"),
        SCHEMA,
        &intent_tag,
    )?;
    transaction
        .commit()
        .map_err(|_| "failure_recovery_claim_unavailable".to_string())?;
    Ok(None)
}

pub(crate) fn complete(
    occurrence: &str,
    attempt: &str,
    diagnosis: &str,
    ready: bool,
    result: &Value,
) -> Result<(), String> {
    let key = occurrence_key(occurrence)?;
    let attempt_key = attempt_key(occurrence, attempt)?;
    let mut store = BrowserSessionSqliteStore::default_sqlite()?;
    let transaction = store
        .remote_view_mutation_connection()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| "failure_recovery_completion_unavailable".to_string())?;
    let mut summary: Summary = load_optional_document(&transaction, &key, SCHEMA)?;
    if summary.in_flight.as_deref() != Some(attempt) {
        return Err("failure_recovery_attempt_conflict: inspect the retained attempt; no remedy was repeated".into());
    }
    summary.no_progress = if ready
        || summary
            .diagnosis_hash
            .as_deref()
            .is_some_and(|old| old != diagnosis)
    {
        0
    } else {
        summary.no_progress.saturating_add(1)
    };
    summary.diagnosis_hash = Some(diagnosis.into());
    summary.in_flight = None;
    save_document(&transaction, &key, SCHEMA, &summary)?;
    save_document(&transaction, &attempt_key, SCHEMA, result)?;
    transaction
        .commit()
        .map_err(|_| "failure_recovery_completion_unavailable".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_budget_extension_preserves_original_attempts_and_in_flight_fence() {
        let directory =
            std::env::temp_dir().join(format!("recovery-accounting-{}", uuid::Uuid::new_v4()));
        let guard = crate::test_utils::EnvGuard::new(&["HOME", "AGENT_BROWSER_HOME"]);
        guard.set("HOME", directory.to_str().unwrap());
        guard.set("AGENT_BROWSER_HOME", directory.to_str().unwrap());
        let occurrence = uuid::Uuid::new_v4().to_string();
        let first = uuid::Uuid::new_v4().to_string();
        let second = uuid::Uuid::new_v4().to_string();
        let initial = Policy {
            max_attempts: 1,
            max_no_progress: 1,
            max_duration_ms: None,
        };
        configure(
            Some("original-task"),
            &initial,
            "fixture",
            "initial bounded budget",
        )
        .unwrap();
        assert!(claim(
            &occurrence,
            Some("original-task"),
            &first,
            "intent-one",
            "unchanged",
            &initial
        )
        .unwrap()
        .is_none());
        assert_eq!(
            claim(
                &occurrence,
                Some("original-task"),
                &first,
                "intent-one",
                "unchanged",
                &initial
            )
            .unwrap()
            .unwrap()["state"],
            "in_flight"
        );
        assert!(claim(
            &occurrence,
            Some("original-task"),
            &second,
            "intent-two",
            "unchanged",
            &initial
        )
        .unwrap_err()
        .contains("attempt_unresolved"));
        complete(
            &occurrence,
            &first,
            "unchanged",
            false,
            &json!({"taskId":"original-task","status":"unresolved"}),
        )
        .unwrap();
        assert!(claim(
            &occurrence,
            Some("original-task"),
            &second,
            "intent-two",
            "unchanged",
            &initial
        )
        .unwrap_err()
        .contains("budget_exhausted"));
        let extended = Policy {
            max_attempts: 2,
            max_no_progress: 2,
            max_duration_ms: None,
        };
        configure(
            Some("original-task"),
            &extended,
            "fixture",
            "extend without losing originals",
        )
        .unwrap();
        assert_eq!(policy(Some("original-task")).unwrap().0.max_attempts, 2);
        assert_eq!(
            claim(
                &occurrence,
                Some("original-task"),
                &first,
                "intent-one",
                "unchanged",
                &extended
            )
            .unwrap()
            .unwrap()["status"],
            "unresolved"
        );
        assert!(claim(
            &occurrence,
            Some("original-task"),
            &second,
            "intent-two",
            "changed_prerequisite",
            &extended
        )
        .unwrap()
        .is_none());
        complete(
            &occurrence,
            &second,
            "repaired",
            true,
            &json!({"taskId":"original-task","status":"ready_for_explicit_resume"}),
        )
        .unwrap();
        assert_eq!(
            inspect(&occurrence, Some(&first)).unwrap()["attempt"]["status"],
            "unresolved"
        );
        assert_eq!(
            inspect(&occurrence, None).unwrap()["summary"]["attempts"],
            2
        );
        assert_eq!(
            inspect(&occurrence, None).unwrap()["summary"]["noProgress"],
            0
        );
        assert!(claim(
            &occurrence,
            Some("peer-task"),
            &first,
            "intent-one",
            "repaired",
            &extended
        )
        .unwrap_err()
        .contains("scope_conflict"));
        drop(guard);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
