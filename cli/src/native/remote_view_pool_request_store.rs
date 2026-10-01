//! Acquisition request continuity inside the existing Browser Runtime database.
use super::browser_launch_custody_store::assignment_release_completed;
use super::browser_session_store::{
    load_optional_document, save_document, BrowserSessionSqliteStore,
};
use super::remote_view_application_store::{document, matching_kind, SCHEMA};
use agent_browser_service_model::*;
use rusqlite::{Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const HEAD_SCHEMA: &str = "agent-browser.remote-view-pool-request.v1";

/// This points to the latest client request, not an allocation or desktop map.
/// Its previous requests remain in the immutable mutation history.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RequestHead {
    schema_version: u32,
    application: String,
    pool: String,
    idempotency_key: String,
}

fn records(
    connection: &Connection,
) -> Result<Vec<RemoteViewApplicationMutationRecord>, RemoteViewApplicationMutationStoreError> {
    use RemoteViewApplicationMutationStoreError as Error;
    let mut statement = connection.prepare(
        "SELECT kind, schema_version, json FROM state_documents WHERE kind LIKE 'remote_view_mutation:%'"
    ).map_err(|_| Error::Unavailable)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|_| Error::Unavailable)?;
    let mut result = Vec::new();
    for row in rows {
        let (kind, schema, json) = row.map_err(|_| Error::Unavailable)?;
        if schema != SCHEMA {
            return Err(Error::InvalidRecord);
        }
        let record: RemoteViewApplicationMutationRecord =
            serde_json::from_str(&json).map_err(|_| Error::InvalidRecord)?;
        if record.schema_version != 1
            || document(&record.envelope)? != kind
            || record
                .outcome
                .as_ref()
                .is_some_and(|outcome| !matching_kind(&record.envelope, outcome))
        {
            return Err(Error::InvalidRecord);
        }
        result.push(record);
    }
    Ok(result)
}

impl RemoteViewPoolRequestStore for BrowserSessionSqliteStore {
    fn pending_pool_acquisitions(
        &mut self,
        application: &str,
        pool: &str,
    ) -> Result<Vec<RemoteViewApplicationMutationRecord>, RemoteViewApplicationMutationStoreError>
    {
        Ok(records(self.remote_view_mutation_connection())?.into_iter().filter(|record|
            record.envelope.application == application && record.outcome.is_none()
                && matches!(&record.envelope.request, RemoteViewApplicationRequest::Acquire { pool_name, .. } if pool_name == pool)
        ).collect())
    }

    fn pool_acquisition_request_key(
        &mut self,
        application: &str,
        pool: &str,
        active_assignments: &[RemoteViewAssignmentRecord],
    ) -> Result<String, RemoteViewApplicationMutationStoreError> {
        use RemoteViewApplicationMutationStoreError as Error;
        if pool.is_empty() {
            return Err(Error::InvalidRecord);
        }
        let identity =
            serde_json::to_vec(&(application, pool)).map_err(|_| Error::InvalidRecord)?;
        let kind = format!("remote_view_pool_request:{:x}", Sha256::digest(identity));
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        // Check all pending acquisitions, including an earlier interrupted head.
        if let Some(pending) = records(&transaction)?.into_iter().find(|record|
            record.envelope.application == application && record.outcome.is_none()
                && matches!(&record.envelope.request, RemoteViewApplicationRequest::Acquire { pool_name, .. } if pool_name == pool)
        ) {
            if let RemoteViewApplicationRequest::Acquire { idempotency_key, .. } = pending.envelope.request {
                transaction.commit().map_err(|_| Error::Unavailable)?;
                return Ok(idempotency_key);
            }
        }
        let head: Option<RequestHead> = load_optional_document(&transaction, &kind, HEAD_SCHEMA)
            .map_err(|_| Error::InvalidRecord)?;
        if let Some(head) = head {
            if head.schema_version != 1 || head.application != application || head.pool != pool {
                return Err(Error::InvalidRecord);
            }
            let envelope = RemoteViewApplicationEnvelope {
                application: application.into(),
                request: RemoteViewApplicationRequest::Acquire {
                    pool_name: pool.into(),
                    idempotency_key: head.idempotency_key.clone(),
                },
            };
            let record: Option<RemoteViewApplicationMutationRecord> =
                load_optional_document(&transaction, &document(&envelope)?, SCHEMA)
                    .map_err(|_| Error::InvalidRecord)?;
            match record {
                None => {
                    // Interruption before mutation custody: preserve the unused key.
                    transaction.commit().map_err(|_| Error::Unavailable)?;
                    return Ok(head.idempotency_key);
                }
                Some(record) => {
                    if record.schema_version != 1 || record.envelope != envelope {
                        return Err(Error::InvalidRecord);
                    }
                    match record.outcome {
                        Some(RemoteViewApplicationMutationOutcome::Acquire(assignment)) => {
                            if !active_assignments.contains(&assignment)
                                && !assignment_release_completed(&transaction, &assignment)
                                    .map_err(|_| Error::InvalidRecord)?
                            {
                                return Err(Error::Conflict);
                            }
                        }
                        _ => return Err(Error::InvalidRecord),
                    }
                }
            }
        }
        let head = RequestHead {
            schema_version: 1,
            application: application.into(),
            pool: pool.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
        };
        RemoteViewApplicationEnvelope {
            application: head.application.clone(),
            request: RemoteViewApplicationRequest::Acquire {
                pool_name: head.pool.clone(),
                idempotency_key: head.idempotency_key.clone(),
            },
        }
        .mutation_key()?;
        save_document(&transaction, &kind, HEAD_SCHEMA, &head).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)?;
        Ok(head.idempotency_key)
    }
}
