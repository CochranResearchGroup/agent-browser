//! Transactional Agent Browser request custody inside the Browser Runtime DB.
use agent_browser_service_model::{
    RemoteViewApplicationEnvelope, RemoteViewApplicationMutationClaim,
    RemoteViewApplicationMutationOutcome, RemoteViewApplicationMutationRecord,
    RemoteViewApplicationMutationStore, RemoteViewApplicationMutationStoreError,
};
use rusqlite::TransactionBehavior;
use sha2::{Digest, Sha256};

use super::browser_session_store::{
    load_optional_document, save_document, BrowserSessionSqliteStore,
};

pub(super) const SCHEMA: &str = "agent-browser.remote-view-mutation.v1";

pub(super) fn matching_kind(
    envelope: &RemoteViewApplicationEnvelope,
    outcome: &RemoteViewApplicationMutationOutcome,
) -> bool {
    use agent_browser_service_model::RemoteViewApplicationRequest as Request;
    use RemoteViewApplicationMutationOutcome as Outcome;
    matches!(
        (&envelope.request, outcome),
        (Request::Acquire { .. }, Outcome::Acquire(_))
            | (Request::Activate { .. }, Outcome::Activate(_))
            | (
                Request::IssueView { .. },
                Outcome::IssueView(_) | Outcome::IssueViewTerminal
            )
            | (Request::RevokeView { .. }, Outcome::RevokeView(_))
            | (Request::Release { .. }, Outcome::Release(_))
    )
}

pub(super) fn document(
    envelope: &RemoteViewApplicationEnvelope,
) -> Result<String, RemoteViewApplicationMutationStoreError> {
    Ok(format!(
        "remote_view_mutation:{:x}",
        Sha256::digest(envelope.mutation_key()?.as_bytes())
    ))
}

impl RemoteViewApplicationMutationStore for BrowserSessionSqliteStore {
    fn claim(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<RemoteViewApplicationMutationClaim, RemoteViewApplicationMutationStoreError> {
        use RemoteViewApplicationMutationStoreError as Error;
        let key = document(envelope)?;
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let existing: Option<RemoteViewApplicationMutationRecord> =
            load_optional_document(&transaction, &key, SCHEMA).map_err(|_| Error::InvalidRecord)?;
        let claim = if let Some(record) = existing {
            if record.schema_version != 1 {
                return Err(Error::InvalidRecord);
            }
            if record.envelope != *envelope {
                return Err(Error::Conflict);
            }
            if record
                .outcome
                .as_ref()
                .is_some_and(|outcome| !matching_kind(envelope, outcome))
            {
                return Err(Error::InvalidRecord);
            }
            RemoteViewApplicationMutationClaim::Existing(Box::new(record))
        } else {
            let record = RemoteViewApplicationMutationRecord {
                schema_version: 1,
                envelope: envelope.clone(),
                outcome: None,
            };
            save_document(&transaction, &key, SCHEMA, &record).map_err(|_| Error::Unavailable)?;
            RemoteViewApplicationMutationClaim::New
        };
        transaction.commit().map_err(|_| Error::Unavailable)?;
        Ok(claim)
    }

    fn complete(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
        outcome: &RemoteViewApplicationMutationOutcome,
    ) -> Result<(), RemoteViewApplicationMutationStoreError> {
        use RemoteViewApplicationMutationStoreError as Error;
        if !matching_kind(envelope, outcome) {
            return Err(Error::InvalidRecord);
        }
        let key = document(envelope)?;
        let transaction = self
            .remote_view_mutation_connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let mut record: RemoteViewApplicationMutationRecord =
            load_optional_document::<Option<_>>(&transaction, &key, SCHEMA)
                .map_err(|_| Error::InvalidRecord)?
                .ok_or(Error::InvalidRecord)?;
        if record.schema_version != 1 {
            return Err(Error::InvalidRecord);
        }
        if record.envelope != *envelope {
            return Err(Error::Conflict);
        }
        if record
            .outcome
            .as_ref()
            .is_some_and(|existing| existing != outcome)
        {
            return Err(Error::Conflict);
        }
        record.outcome = Some(outcome.clone());
        save_document(&transaction, &key, SCHEMA, &record).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TemporaryStore(PathBuf);
    impl TemporaryStore {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("p220-mutation-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn open(&self) -> BrowserSessionSqliteStore {
            BrowserSessionSqliteStore::open_or_migrate(&self.0, &self.0.join("legacy.json"))
                .unwrap()
                .0
        }
    }
    impl Drop for TemporaryStore {
        fn drop(&mut self) {
            make_fixture_writable(&self.0);
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn make_fixture_writable(path: &std::path::Path) {
        let metadata = std::fs::symlink_metadata(path).unwrap();
        assert!(!metadata.file_type().is_symlink());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                path,
                std::fs::Permissions::from_mode(if metadata.is_dir() { 0o700 } else { 0o600 }),
            )
            .unwrap();
        }
        #[cfg(not(unix))]
        {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(false);
            std::fs::set_permissions(path, permissions).unwrap();
        }
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path).unwrap() {
                make_fixture_writable(&entry.unwrap().path());
            }
        }
    }

    #[test]
    fn sqlite_remote_view_mutation_custody_survives_restart_and_conflicts() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let envelope: RemoteViewApplicationEnvelope =
            serde_json::from_value(fixture["requests"][11].clone()).unwrap();
        let outcome = RemoteViewApplicationMutationOutcome::Release(
            serde_json::from_value(fixture["joinedRelease"].clone()).unwrap(),
        );
        let directory = TemporaryStore::new();
        let mut first = directory.open();
        assert_eq!(
            first.claim(&envelope).unwrap(),
            RemoteViewApplicationMutationClaim::New
        );
        drop(first);
        let mut second = directory.open();
        let RemoteViewApplicationMutationClaim::Existing(pending) =
            second.claim(&envelope).unwrap()
        else {
            panic!("duplicate admission after restart");
        };
        assert_eq!(pending.outcome, None);
        let wrong_kind = RemoteViewApplicationMutationOutcome::Acquire(
            serde_json::from_value(fixture["assignment"].clone()).unwrap(),
        );
        assert_eq!(
            second.complete(&envelope, &wrong_kind),
            Err(RemoteViewApplicationMutationStoreError::InvalidRecord)
        );
        let mut changed: RemoteViewApplicationEnvelope =
            serde_json::from_value(fixture["requests"][11].clone()).unwrap();
        if let agent_browser_service_model::RemoteViewApplicationRequest::Release {
            expected_generation,
            ..
        } = &mut changed.request
        {
            *expected_generation = 8;
        }
        assert_eq!(
            second.claim(&changed),
            Err(RemoteViewApplicationMutationStoreError::Conflict)
        );
        second.complete(&envelope, &outcome).unwrap();
        drop(second);
        let mut third = directory.open();
        let RemoteViewApplicationMutationClaim::Existing(completed) =
            third.claim(&envelope).unwrap()
        else {
            panic!("lost completed operation");
        };
        assert_eq!(completed.outcome.as_ref(), Some(&outcome));
        third.complete(&envelope, &outcome).unwrap();
        let mut contradictory = outcome.clone();
        if let RemoteViewApplicationMutationOutcome::Release(release) = &mut contradictory {
            release.retirement.sessions.clear();
        }
        assert_eq!(
            third.complete(&envelope, &contradictory),
            Err(RemoteViewApplicationMutationStoreError::Conflict)
        );
    }

    #[test]
    fn sqlite_remote_view_mutation_claim_has_one_winner_across_connections() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let envelope: RemoteViewApplicationEnvelope =
            serde_json::from_value(fixture["requests"][11].clone()).unwrap();
        let directory = TemporaryStore::new();
        drop(directory.open());
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let mut handles = Vec::new();
        for _ in 0..2 {
            let path = directory.0.clone();
            let request = envelope.clone();
            let barrier = barrier.clone();
            handles.push(std::thread::spawn(move || {
                let mut store = BrowserSessionSqliteStore::open(&path).unwrap();
                barrier.wait();
                store.claim(&request).unwrap()
            }));
        }
        let claims = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            claims
                .iter()
                .filter(|claim| matches!(claim, RemoteViewApplicationMutationClaim::New))
                .count(),
            1
        );
        assert_eq!(
            claims
                .iter()
                .filter(|claim| matches!(claim, RemoteViewApplicationMutationClaim::Existing(_)))
                .count(),
            1
        );
    }
}
