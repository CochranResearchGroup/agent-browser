//! SQLite-backed admission for ordinary remote requests. Only the caller that
//! owns the admitted attempt may execute; duplicate waiters observe its result.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use agent_browser_service_model::{PresentationRequestPriority, PresentationRequestState};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::browser_session_store::{BrowserRuntimeOperationState, BrowserRuntimeSqliteStore};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresentationQueueStatus {
    pub host_generation: u64,
    pub maximum_depth: u32,
    pub queued: usize,
    pub admitted: usize,
    pub completed: usize,
    pub retryable: usize,
    pub recovery_required: usize,
}

pub(crate) fn queue_status(
    store: &BrowserRuntimeSqliteStore,
) -> Result<PresentationQueueStatus, String> {
    let queue = store.load_presentation_queue()?;
    let mut status = PresentationQueueStatus {
        host_generation: queue.host_generation,
        maximum_depth: store.load_runtime_config()?.maximum_queue_depth,
        queued: 0,
        admitted: 0,
        completed: 0,
        retryable: 0,
        recovery_required: 0,
    };
    let now = now_ms();
    for entry in queue.entries.values() {
        match &entry.state {
            PresentationRequestState::Queued if entry.deadline_ms > now => status.queued += 1,
            PresentationRequestState::Queued
            | PresentationRequestState::Retryable {
                recovery_required: false,
            } => status.retryable += 1,
            PresentationRequestState::Admitted { .. } => status.admitted += 1,
            PresentationRequestState::Completed { .. } => status.completed += 1,
            PresentationRequestState::Retryable {
                recovery_required: true,
            } => status.recovery_required += 1,
        }
    }
    Ok(status)
}

pub(crate) enum PresentationAdmission {
    Execute(PresentationAdmissionPermit),
    Replay(Value),
}

pub(crate) struct PresentationAdmissionRequest {
    path: PathBuf,
    key: String,
    generation: u64,
    sequence: u64,
    attempt_token: String,
    pub deadline_at_ms: u64,
}

pub(crate) struct PresentationAdmissionPermit {
    request: PresentationAdmissionRequest,
    effects_started: bool,
    completed: bool,
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

impl PresentationAdmissionRequest {
    pub(crate) fn enqueue(
        command: &Value,
        generation: u64,
        existing_handoff: bool,
    ) -> Result<Self, String> {
        let path = BrowserRuntimeSqliteStore::default_sqlite_path()?;
        Self::enqueue_at(path, command, generation, existing_handoff, now_ms())
    }

    fn enqueue_at(
        path: PathBuf,
        command: &Value,
        generation: u64,
        existing_handoff: bool,
        now: u64,
    ) -> Result<Self, String> {
        let id = command
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "presentation_queue_request_id_missing".to_string())?;
        let key = format!(
            "{}:{id}",
            if existing_handoff {
                "handoff"
            } else {
                "browser"
            }
        );
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(command).map_err(|_| "presentation_queue_request_invalid")?
            )
        );
        let mut store = BrowserRuntimeSqliteStore::open(&path)?;
        let config = store.load_runtime_config()?;
        let priority = if store
            .find_operation(id)?
            .is_some_and(|operation| operation.state != BrowserRuntimeOperationState::Committed)
        {
            PresentationRequestPriority::Recovery
        } else if existing_handoff
            || super::presentation_runtime_capacity::command_reuses_browser(
                command,
                &store.load_session_state()?,
            )
        {
            PresentationRequestPriority::ExistingHandoff
        } else {
            PresentationRequestPriority::NewOpen
        };
        let entry = store.mutate_presentation_queue(|queue| {
            queue.advance_generation(generation)?;
            queue.enqueue(
                key.clone(),
                fingerprint,
                priority,
                now,
                now.saturating_add(config.request_deadline_ms),
                config.maximum_queue_depth as usize,
            )
        })?;
        Ok(Self {
            path,
            key,
            generation,
            sequence: entry.sequence,
            attempt_token: uuid::Uuid::new_v4().to_string(),
            deadline_at_ms: entry.deadline_ms,
        })
    }

    /// Observing readiness never reserves a slot. The immediate SQLite
    /// transaction is the sole admission decision and fences concurrent waiters.
    pub(crate) fn poll(
        &self,
        readiness: Option<bool>,
    ) -> Result<Option<PresentationAdmission>, String> {
        let now = now_ms();
        let mut store = BrowserRuntimeSqliteStore::open(&self.path)?;
        let queue = store.load_presentation_queue()?;
        if queue.host_generation != self.generation {
            return Err("presentation_queue_generation_stale".to_string());
        }
        let mut entry = queue
            .entries
            .get(&self.key)
            .cloned()
            .ok_or_else(|| "presentation_queue_request_missing".to_string())?;
        if entry.sequence != self.sequence {
            return Err("presentation_queue_request_replaced".to_string());
        }
        if matches!(entry.state, PresentationRequestState::Queued) {
            if let Some(capacity_available) = readiness {
                match store.mutate_presentation_queue(|queue| {
                    if queue
                        .entries
                        .get(&self.key)
                        .is_none_or(|entry| entry.sequence != self.sequence)
                    {
                        return Err("presentation_queue_request_replaced".to_string());
                    }
                    queue.try_admit(
                        &self.key,
                        self.generation,
                        self.attempt_token.clone(),
                        now,
                        capacity_available,
                    )
                }) {
                    Ok(admitted) => entry = admitted,
                    Err(error)
                        if [
                            "presentation_queue_admission_active:",
                            "presentation_queue_capacity_pending:",
                            "presentation_queue_not_next:",
                            "presentation_queue_no_eligible_request:",
                        ]
                        .iter()
                        .any(|prefix| error.starts_with(prefix)) =>
                    {
                        return Ok(None)
                    }
                    Err(error) => return Err(error),
                }
            }
        }
        match entry.state {
            PresentationRequestState::Completed { response } => {
                Ok(Some(PresentationAdmission::Replay(response)))
            }
            PresentationRequestState::Admitted { attempt_token }
                if attempt_token == self.attempt_token =>
            {
                Ok(Some(PresentationAdmission::Execute(
                    PresentationAdmissionPermit {
                        request: Self {
                            path: self.path.clone(),
                            key: self.key.clone(),
                            generation: self.generation,
                            sequence: self.sequence,
                            attempt_token: self.attempt_token.clone(),
                            deadline_at_ms: self.deadline_at_ms,
                        },
                        effects_started: false,
                        completed: false,
                    },
                )))
            }
            PresentationRequestState::Retryable {
                recovery_required: true,
            } => Err("presentation_queue_effect_recovery_required".to_string()),
            _ if now >= self.deadline_at_ms => {
                Err("presentation_queue_deadline_exceeded".to_string())
            }
            _ => Ok(None),
        }
    }
}

impl PresentationAdmissionPermit {
    /// Call after the host mutex is acquired, immediately before browser work.
    pub(crate) fn require_current(&mut self) -> Result<(), String> {
        let queue =
            BrowserRuntimeSqliteStore::open(&self.request.path)?.load_presentation_queue()?;
        if queue.host_generation != self.request.generation {
            return Err("presentation_queue_generation_stale".to_string());
        }
        if now_ms() >= self.request.deadline_at_ms {
            return Err("presentation_queue_deadline_exceeded".to_string());
        }
        match queue
            .entries
            .get(&self.request.key)
            .filter(|entry| entry.sequence == self.request.sequence)
            .map(|entry| &entry.state)
        {
            Some(PresentationRequestState::Admitted { attempt_token })
                if attempt_token == &self.request.attempt_token =>
            {
                self.effects_started = true;
                Ok(())
            }
            _ => Err("presentation_queue_admission_stale".to_string()),
        }
    }

    /// Persist terminal evidence inside the blocking effect task, even if its
    /// async client disconnected. A lost completion cannot authorize a replay.
    pub(crate) fn finish(mut self, result: Result<Value, String>) -> Result<Value, String> {
        let response =
            result.unwrap_or_else(|error| serde_json::json!({"success":false,"error":error}));
        BrowserRuntimeSqliteStore::open(&self.request.path)?.mutate_presentation_queue(
            |queue| {
                if queue
                    .entries
                    .get(&self.request.key)
                    .is_none_or(|entry| entry.sequence != self.request.sequence)
                {
                    return Err("presentation_queue_request_replaced".to_string());
                }
                queue.complete(
                    &self.request.key,
                    self.request.generation,
                    &self.request.attempt_token,
                    response.clone(),
                )
            },
        )?;
        self.completed = true;
        Ok(response)
    }
}

impl Drop for PresentationAdmissionPermit {
    fn drop(&mut self) {
        if self.effects_started || self.completed {
            return;
        }
        // Cancellation between SQLite admission and the effect task did not
        // perform browser work. Release only this exact unstarted attempt.
        if let Ok(mut store) = BrowserRuntimeSqliteStore::open(&self.request.path) {
            let _ = store.mutate_presentation_queue(|queue| {
                if queue.host_generation == self.request.generation {
                    if let Some(entry) = queue.entries.get_mut(&self.request.key) {
                        if entry.sequence == self.request.sequence && matches!(&entry.state, PresentationRequestState::Admitted { attempt_token } if attempt_token == &self.request.attempt_token) {
                            entry.state = PresentationRequestState::Queued;
                        }
                    }
                }
                Ok(())
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::browser_session_store::LegacyBrowserRuntimeSources;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("presentation-queue-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            BrowserRuntimeSqliteStore::migrate_from_legacy(
                &root.join("runtime.sqlite3"),
                LegacyBrowserRuntimeSources {
                    session_state_path: &root.join("sessions.json"),
                    profile_catalog_path: &root.join("profiles.json"),
                    service_state_path: &root.join("service.json"),
                },
            )
            .unwrap();
            Self(root)
        }
        fn request(&self, id: &str, handoff: bool) -> PresentationAdmissionRequest {
            PresentationAdmissionRequest::enqueue_at(
                self.0.join("runtime.sqlite3"),
                &serde_json::json!({"id":id,"action":"browser_session_open","profileId":"fixture"}),
                1,
                handoff,
                now_ms(),
            )
            .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn sqlite_admission_coalesces_and_releases_only_unstarted_attempts() {
        let fixture = Fixture::new();
        let first = fixture.request("first", false);
        let duplicate = fixture.request("first", false);
        let second = fixture.request("second", false);
        assert!(second.poll(Some(true)).unwrap().is_none());
        let Some(PresentationAdmission::Execute(mut permit)) = first.poll(Some(true)).unwrap()
        else {
            panic!("first request must own admission")
        };
        assert!(duplicate.poll(Some(true)).unwrap().is_none());
        assert!(second.poll(Some(true)).unwrap().is_none());
        permit.require_current().unwrap();
        let status = queue_status(
            &BrowserRuntimeSqliteStore::open(&fixture.0.join("runtime.sqlite3")).unwrap(),
        )
        .unwrap();
        assert_eq!((status.queued, status.admitted), (1, 1));
        let response = serde_json::json!({"success":true,"fixtureEffectCount":1});
        assert_eq!(permit.finish(Ok(response.clone())).unwrap(), response);
        let Some(PresentationAdmission::Replay(replayed)) = duplicate.poll(None).unwrap() else {
            panic!("duplicate must replay")
        };
        assert_eq!(replayed, response);
        let Some(PresentationAdmission::Execute(unstarted)) = second.poll(Some(true)).unwrap()
        else {
            panic!("next request must admit")
        };
        drop(unstarted);
        let Some(PresentationAdmission::Execute(mut resumed)) = second.poll(Some(true)).unwrap()
        else {
            panic!("cancelled pre-effect permit must release")
        };
        resumed.require_current().unwrap();
        drop(resumed);
        assert!(
            fixture
                .request("third", false)
                .poll(Some(true))
                .unwrap()
                .is_none(),
            "an interrupted started effect must not silently release authority"
        );
    }

    #[test]
    fn expired_waiter_cannot_claim_a_same_generation_retry() {
        let fixture = Fixture::new();
        let path = fixture.0.join("runtime.sqlite3");
        let command =
            serde_json::json!({"id":"retry","action":"browser_session_open","profileId":"fixture"});
        let stale = PresentationAdmissionRequest::enqueue_at(
            path.clone(),
            &command,
            1,
            false,
            now_ms() - 100_000,
        )
        .unwrap();
        let fresh =
            PresentationAdmissionRequest::enqueue_at(path, &command, 1, false, now_ms()).unwrap();
        assert_ne!(stale.sequence, fresh.sequence);
        assert!(
            matches!(stale.poll(Some(true)), Err(error) if error == "presentation_queue_request_replaced")
        );
        assert!(matches!(
            fresh.poll(Some(true)).unwrap(),
            Some(PresentationAdmission::Execute(_))
        ));
    }

    #[test]
    fn existing_handoff_bypasses_capacity_wait_but_old_generation_cannot_finish() {
        let fixture = Fixture::new();
        let new_open = fixture.request("open", false);
        let handoff = fixture.request("handoff", true);
        assert!(new_open.poll(Some(false)).unwrap().is_none());
        let Some(PresentationAdmission::Execute(mut permit)) = handoff.poll(Some(false)).unwrap()
        else {
            panic!("handoff does not need new capacity")
        };
        permit.require_current().unwrap();
        let path = fixture.0.join("runtime.sqlite3");
        BrowserRuntimeSqliteStore::open(&path)
            .unwrap()
            .mutate_presentation_queue(|queue| queue.advance_generation(2))
            .unwrap();
        assert!(permit
            .finish(Ok(serde_json::json!({"success":true})))
            .is_err());
        let queue = BrowserRuntimeSqliteStore::open(&path)
            .unwrap()
            .load_presentation_queue()
            .unwrap();
        assert!(matches!(
            queue.entries["browser:open"].state,
            PresentationRequestState::Retryable {
                recovery_required: false
            }
        ));
        assert!(matches!(
            queue.entries["handoff:handoff"].state,
            PresentationRequestState::Retryable {
                recovery_required: true
            }
        ));
    }
}
