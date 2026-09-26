//! Durable, provider-free admission ordering for presentation requests.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const AGING_PROMOTION_MS: u64 = 30_000;
const MAX_TERMINAL_ENTRIES: usize = 128;
const MAX_RECOVERY_REQUIRED_ENTRIES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationRequestPriority {
    Recovery,
    ExistingHandoff,
    NewOpen,
}

impl PresentationRequestPriority {
    fn rank(self) -> u64 {
        match self {
            Self::Recovery => 0,
            Self::ExistingHandoff => 1,
            Self::NewOpen => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PresentationRequestState {
    Queued,
    Admitted { attempt_token: String },
    Completed { response: Value },
    Retryable { recovery_required: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationRequestEntry {
    pub key: String,
    pub fingerprint: String,
    pub priority: PresentationRequestPriority,
    pub enqueued_at_ms: u64,
    pub deadline_ms: u64,
    pub sequence: u64,
    pub state: PresentationRequestState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationRequestQueue {
    pub host_generation: u64,
    pub next_sequence: u64,
    pub entries: BTreeMap<String, PresentationRequestEntry>,
}

impl PresentationRequestQueue {
    pub fn validate(&self) -> Result<(), String> {
        let mut sequences = BTreeSet::new();
        let mut admitted_count = 0usize;
        let mut safe_terminal_count = 0usize;
        let mut recovery_required_count = 0usize;
        for (map_key, entry) in &self.entries {
            if entry.key.is_empty() {
                return Err("presentation_queue_key_missing".to_string());
            }
            if map_key != &entry.key {
                return Err(format!(
                    "presentation_queue_entry_key_mismatch:{map_key}:{}",
                    entry.key
                ));
            }
            if entry.fingerprint.is_empty() {
                return Err(format!("presentation_queue_fingerprint_missing:{map_key}"));
            }
            if entry.deadline_ms <= entry.enqueued_at_ms {
                return Err(format!("presentation_queue_deadline_invalid:{map_key}"));
            }
            if !sequences.insert(entry.sequence) {
                return Err(format!(
                    "presentation_queue_sequence_duplicate:{}",
                    entry.sequence
                ));
            }
            if entry.sequence >= self.next_sequence {
                return Err(format!(
                    "presentation_queue_next_sequence_invalid:{}:{}",
                    entry.sequence, self.next_sequence
                ));
            }
            match &entry.state {
                PresentationRequestState::Admitted { attempt_token } => {
                    admitted_count += 1;
                    if attempt_token.is_empty() {
                        return Err(format!(
                            "presentation_queue_attempt_token_missing:{map_key}"
                        ));
                    }
                }
                PresentationRequestState::Completed { .. }
                | PresentationRequestState::Retryable {
                    recovery_required: false,
                } => safe_terminal_count += 1,
                PresentationRequestState::Retryable {
                    recovery_required: true,
                } => recovery_required_count += 1,
                PresentationRequestState::Queued => {}
            }
        }
        if admitted_count > 1 {
            return Err("presentation_queue_multiple_admitted".to_string());
        }
        if safe_terminal_count > MAX_TERMINAL_ENTRIES {
            return Err(format!(
                "presentation_queue_terminal_retention_exceeded:{safe_terminal_count}"
            ));
        }
        if recovery_required_count > MAX_RECOVERY_REQUIRED_ENTRIES {
            return Err(format!(
                "presentation_queue_recovery_retention_exceeded:{recovery_required_count}"
            ));
        }
        Ok(())
    }

    pub fn enqueue(
        &mut self,
        key: String,
        fingerprint: String,
        priority: PresentationRequestPriority,
        now_ms: u64,
        deadline_ms: u64,
        max_depth: usize,
    ) -> Result<PresentationRequestEntry, String> {
        if key.is_empty() {
            return Err("presentation_queue_key_missing".to_string());
        }
        if fingerprint.is_empty() {
            return Err(format!("presentation_queue_fingerprint_missing:{key}"));
        }
        if deadline_ms <= now_ms {
            return Err(format!("presentation_queue_deadline_invalid:{key}"));
        }
        if max_depth == 0 {
            return Err("presentation_queue_max_depth_invalid".to_string());
        }

        self.reclaim_expired(now_ms);
        self.prune_safe_terminal();

        if let Some(existing) = self.entries.get(&key) {
            if existing.fingerprint != fingerprint {
                return Err(format!("presentation_queue_payload_conflict:{key}"));
            }
            match existing.state {
                PresentationRequestState::Retryable {
                    recovery_required: true,
                } => {
                    return Err(format!("presentation_queue_recovery_required:{key}"));
                }
                PresentationRequestState::Retryable {
                    recovery_required: false,
                } => {}
                _ => return Ok(existing.clone()),
            }
        }
        if self.recovery_required_count() >= MAX_RECOVERY_REQUIRED_ENTRIES {
            return Err("presentation_queue_recovery_retention_full".to_string());
        }

        if self.active_depth(now_ms) >= max_depth {
            return Err("presentation_queue_full".to_string());
        }
        let sequence = self.allocate_sequence()?;
        let entry = PresentationRequestEntry {
            key: key.clone(),
            fingerprint,
            priority,
            enqueued_at_ms: now_ms,
            deadline_ms,
            sequence,
            state: PresentationRequestState::Queued,
        };
        self.entries.insert(key, entry.clone());
        Ok(entry)
    }

    pub fn try_admit(
        &mut self,
        key: &str,
        host_generation: u64,
        attempt_token: String,
        now_ms: u64,
        new_capacity_available: bool,
    ) -> Result<PresentationRequestEntry, String> {
        self.require_generation(host_generation)?;
        if attempt_token.is_empty() {
            return Err("presentation_queue_attempt_token_missing".to_string());
        }

        let requested = self
            .entries
            .get(key)
            .ok_or_else(|| format!("presentation_queue_entry_missing:{key}"))?;
        if matches!(
            requested.state,
            PresentationRequestState::Retryable {
                recovery_required: false
            }
        ) {
            return Err(format!("presentation_queue_deadline_exceeded:{key}"));
        }
        if matches!(requested.state, PresentationRequestState::Queued)
            && requested.deadline_ms <= now_ms
        {
            let requested = self.entries.get_mut(key).expect("entry checked above");
            requested.state = PresentationRequestState::Retryable {
                recovery_required: false,
            };
            self.prune_safe_terminal();
            return Err(format!("presentation_queue_deadline_exceeded:{key}"));
        }
        if let Some(active_key) = self.entries.values().find_map(|entry| {
            matches!(entry.state, PresentationRequestState::Admitted { .. })
                .then_some(entry.key.as_str())
        }) {
            return Err(format!("presentation_queue_admission_active:{active_key}"));
        }

        let next_key = self
            .entries
            .values()
            .filter(|entry| matches!(entry.state, PresentationRequestState::Queued))
            .filter(|entry| entry.deadline_ms > now_ms)
            .filter(|entry| {
                new_capacity_available || entry.priority != PresentationRequestPriority::NewOpen
            })
            .min_by_key(|entry| {
                let age_promotions =
                    now_ms.saturating_sub(entry.enqueued_at_ms) / AGING_PROMOTION_MS;
                (
                    entry.priority.rank().saturating_sub(age_promotions),
                    entry.sequence,
                )
            })
            .map(|entry| entry.key.clone());

        let next_key = next_key.ok_or_else(|| {
            if requested.priority == PresentationRequestPriority::NewOpen && !new_capacity_available
            {
                format!("presentation_queue_capacity_pending:{key}")
            } else {
                format!("presentation_queue_no_eligible_request:{key}")
            }
        })?;
        if next_key != key {
            return Err(format!("presentation_queue_not_next:{next_key}"));
        }

        let entry = self.entries.get_mut(key).expect("selected entry exists");
        if !matches!(entry.state, PresentationRequestState::Queued) {
            return Err(format!("presentation_queue_entry_not_queued:{key}"));
        }
        entry.state = PresentationRequestState::Admitted { attempt_token };
        Ok(entry.clone())
    }

    pub fn complete(
        &mut self,
        key: &str,
        host_generation: u64,
        attempt_token: &str,
        response: Value,
    ) -> Result<PresentationRequestEntry, String> {
        self.require_generation(host_generation)?;
        let entry = self
            .entries
            .get_mut(key)
            .ok_or_else(|| format!("presentation_queue_entry_missing:{key}"))?;
        match &entry.state {
            PresentationRequestState::Admitted {
                attempt_token: expected,
            } if expected == attempt_token => {}
            PresentationRequestState::Admitted { .. } => {
                return Err(format!("presentation_queue_attempt_token_mismatch:{key}"));
            }
            _ => return Err(format!("presentation_queue_entry_not_admitted:{key}")),
        }
        entry.state = PresentationRequestState::Completed { response };
        let completed = entry.clone();
        self.prune_safe_terminal();
        Ok(completed)
    }

    pub fn cancel_queued(
        &mut self,
        key: &str,
        fingerprint: &str,
    ) -> Result<PresentationRequestEntry, String> {
        let entry = self
            .entries
            .get(key)
            .ok_or_else(|| format!("presentation_queue_entry_missing:{key}"))?;
        if entry.fingerprint != fingerprint {
            return Err(format!("presentation_queue_payload_conflict:{key}"));
        }
        if !matches!(entry.state, PresentationRequestState::Queued) {
            return Err(format!("presentation_queue_entry_not_queued:{key}"));
        }
        self.entries
            .remove(key)
            .ok_or_else(|| format!("presentation_queue_entry_missing:{key}"))
    }

    pub fn resume_reconciled(
        &mut self,
        expected: &PresentationRequestEntry,
        host_generation: u64,
        now_ms: u64,
        deadline_ms: u64,
        max_depth: usize,
    ) -> Result<PresentationRequestEntry, String> {
        self.require_generation(host_generation)?;
        if deadline_ms <= now_ms {
            return Err(format!(
                "presentation_queue_deadline_invalid:{}",
                expected.key
            ));
        }
        if max_depth == 0 {
            return Err("presentation_queue_max_depth_invalid".to_string());
        }
        let stored = self
            .entries
            .get(&expected.key)
            .ok_or_else(|| format!("presentation_queue_entry_missing:{}", expected.key))?;
        if stored != expected {
            return Err(format!(
                "presentation_queue_reconciled_entry_mismatch:{}",
                expected.key
            ));
        }
        if !matches!(
            stored.state,
            PresentationRequestState::Retryable {
                recovery_required: true
            }
        ) {
            return Err(format!(
                "presentation_queue_recovery_not_required:{}",
                expected.key
            ));
        }
        if self.recovery_required_count() > MAX_RECOVERY_REQUIRED_ENTRIES {
            return Err("presentation_queue_recovery_retention_full".to_string());
        }
        if self.active_depth(now_ms) >= max_depth {
            return Err("presentation_queue_full".to_string());
        }

        let next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| "presentation_queue_sequence_exhausted".to_string())?;
        let resumed = PresentationRequestEntry {
            key: expected.key.clone(),
            fingerprint: expected.fingerprint.clone(),
            priority: PresentationRequestPriority::Recovery,
            enqueued_at_ms: now_ms,
            deadline_ms,
            sequence: self.next_sequence,
            state: PresentationRequestState::Queued,
        };
        self.next_sequence = next_sequence;
        self.entries.insert(expected.key.clone(), resumed.clone());
        Ok(resumed)
    }

    pub fn advance_generation(&mut self, new_host_generation: u64) -> Result<(), String> {
        if new_host_generation < self.host_generation {
            return Err(format!(
                "presentation_queue_generation_regression:{}:{}",
                self.host_generation, new_host_generation
            ));
        }
        if new_host_generation == self.host_generation {
            return Ok(());
        }
        self.host_generation = new_host_generation;
        for entry in self.entries.values_mut() {
            entry.state = match &entry.state {
                PresentationRequestState::Queued => PresentationRequestState::Retryable {
                    recovery_required: false,
                },
                PresentationRequestState::Admitted { .. } => PresentationRequestState::Retryable {
                    recovery_required: true,
                },
                PresentationRequestState::Completed { response } => {
                    PresentationRequestState::Completed {
                        response: response.clone(),
                    }
                }
                PresentationRequestState::Retryable { recovery_required } => {
                    PresentationRequestState::Retryable {
                        recovery_required: *recovery_required,
                    }
                }
            };
        }
        self.prune_safe_terminal();
        Ok(())
    }

    fn require_generation(&self, host_generation: u64) -> Result<(), String> {
        if host_generation != self.host_generation {
            return Err(format!(
                "presentation_queue_generation_mismatch:{host_generation}:{}",
                self.host_generation
            ));
        }
        Ok(())
    }

    fn active_depth(&self, now_ms: u64) -> usize {
        self.entries
            .values()
            .filter(|entry| {
                matches!(entry.state, PresentationRequestState::Admitted { .. })
                    || matches!(entry.state, PresentationRequestState::Queued)
                        && entry.deadline_ms > now_ms
            })
            .count()
    }

    fn allocate_sequence(&mut self) -> Result<u64, String> {
        let sequence = self.next_sequence;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| "presentation_queue_sequence_exhausted".to_string())?;
        Ok(sequence)
    }

    fn reclaim_expired(&mut self, now_ms: u64) {
        for entry in self.entries.values_mut() {
            if matches!(entry.state, PresentationRequestState::Queued)
                && entry.deadline_ms <= now_ms
            {
                entry.state = PresentationRequestState::Retryable {
                    recovery_required: false,
                };
            }
        }
    }

    fn recovery_required_count(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| {
                matches!(
                    entry.state,
                    PresentationRequestState::Retryable {
                        recovery_required: true
                    }
                )
            })
            .count()
    }

    fn prune_safe_terminal(&mut self) {
        let mut terminal = self
            .entries
            .values()
            .filter(|entry| {
                matches!(
                    entry.state,
                    PresentationRequestState::Completed { .. }
                        | PresentationRequestState::Retryable {
                            recovery_required: false
                        }
                )
            })
            .map(|entry| (entry.sequence, entry.key.clone()))
            .collect::<Vec<_>>();
        if terminal.len() <= MAX_TERMINAL_ENTRIES {
            return;
        }
        terminal.sort_unstable();
        let remove_count = terminal.len() - MAX_TERMINAL_ENTRIES;
        for (_, key) in terminal.into_iter().take(remove_count) {
            self.entries.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn enqueue(
        queue: &mut PresentationRequestQueue,
        key: &str,
        priority: PresentationRequestPriority,
        now_ms: u64,
    ) -> PresentationRequestEntry {
        queue
            .enqueue(
                key.to_string(),
                format!("fingerprint-{key}"),
                priority,
                now_ms,
                now_ms + 120_000,
                32,
            )
            .unwrap()
    }

    fn recovery_required_queue() -> (PresentationRequestQueue, PresentationRequestEntry) {
        let mut queue = PresentationRequestQueue::default();
        queue.advance_generation(1).unwrap();
        enqueue(
            &mut queue,
            "recover",
            PresentationRequestPriority::Recovery,
            10,
        );
        queue
            .try_admit("recover", 1, "attempt".to_string(), 20, true)
            .unwrap();
        queue.advance_generation(2).unwrap();
        let expected = queue.entries["recover"].clone();
        (queue, expected)
    }

    #[test]
    fn duplicate_coalesces_and_payload_mismatch_conflicts() {
        let mut queue = PresentationRequestQueue::default();
        let first = enqueue(
            &mut queue,
            "open-a",
            PresentationRequestPriority::NewOpen,
            10,
        );
        let duplicate = queue
            .enqueue(
                "open-a".to_string(),
                "fingerprint-open-a".to_string(),
                PresentationRequestPriority::Recovery,
                20,
                200_000,
                32,
            )
            .unwrap();

        assert_eq!(duplicate.sequence, first.sequence);
        assert_eq!(duplicate.deadline_ms, first.deadline_ms);
        assert_eq!(duplicate.priority, PresentationRequestPriority::NewOpen);
        assert_eq!(
            queue.enqueue(
                "open-a".to_string(),
                "changed".to_string(),
                PresentationRequestPriority::NewOpen,
                20,
                200_000,
                32,
            ),
            Err("presentation_queue_payload_conflict:open-a".to_string())
        );
    }

    #[test]
    fn priority_aging_and_fifo_choose_one_ticket() {
        let mut queue = PresentationRequestQueue::default();
        queue.advance_generation(7).unwrap();
        enqueue(
            &mut queue,
            "new-old",
            PresentationRequestPriority::NewOpen,
            0,
        );
        enqueue(
            &mut queue,
            "existing",
            PresentationRequestPriority::ExistingHandoff,
            59_999,
        );
        enqueue(
            &mut queue,
            "recovery",
            PresentationRequestPriority::Recovery,
            59_999,
        );

        assert_eq!(
            queue
                .try_admit("new-old", 7, "attempt-a".to_string(), 60_000, true)
                .unwrap()
                .state,
            PresentationRequestState::Admitted {
                attempt_token: "attempt-a".to_string()
            }
        );
        assert_eq!(
            queue.try_admit("recovery", 7, "attempt-b".to_string(), 60_000, true),
            Err("presentation_queue_admission_active:new-old".to_string())
        );
    }

    #[test]
    fn new_open_waits_for_capacity_while_existing_work_can_admit() {
        let mut queue = PresentationRequestQueue::default();
        queue.advance_generation(3).unwrap();
        enqueue(&mut queue, "new", PresentationRequestPriority::NewOpen, 10);
        enqueue(
            &mut queue,
            "handoff",
            PresentationRequestPriority::ExistingHandoff,
            20,
        );

        assert_eq!(
            queue.try_admit("new", 3, "new-attempt".to_string(), 30, false),
            Err("presentation_queue_not_next:handoff".to_string())
        );
        assert!(matches!(
            queue
                .try_admit("handoff", 3, "handoff-attempt".to_string(), 30, false)
                .unwrap()
                .state,
            PresentationRequestState::Admitted { .. }
        ));
    }

    #[test]
    fn completion_is_fenced_and_replays_the_exact_response() {
        let mut queue = PresentationRequestQueue::default();
        queue.advance_generation(4).unwrap();
        enqueue(&mut queue, "open", PresentationRequestPriority::NewOpen, 10);
        queue
            .try_admit("open", 4, "attempt".to_string(), 20, true)
            .unwrap();

        assert_eq!(
            queue.complete("open", 3, "attempt", json!({"ok": true})),
            Err("presentation_queue_generation_mismatch:3:4".to_string())
        );
        assert_eq!(
            queue.complete("open", 4, "wrong", json!({"ok": true})),
            Err("presentation_queue_attempt_token_mismatch:open".to_string())
        );
        let completed = queue
            .complete("open", 4, "attempt", json!({"ok": true, "id": 7}))
            .unwrap();
        assert_eq!(
            completed.state,
            PresentationRequestState::Completed {
                response: json!({"ok": true, "id": 7})
            }
        );
        assert_eq!(
            queue
                .enqueue(
                    "open".to_string(),
                    "fingerprint-open".to_string(),
                    PresentationRequestPriority::NewOpen,
                    30,
                    130_000,
                    32,
                )
                .unwrap(),
            completed
        );
    }

    #[test]
    fn generation_advance_requires_explicit_recovery_for_admitted_work() {
        let mut queue = PresentationRequestQueue::default();
        queue.advance_generation(8).unwrap();
        enqueue(
            &mut queue,
            "queued",
            PresentationRequestPriority::NewOpen,
            10,
        );
        enqueue(
            &mut queue,
            "admitted",
            PresentationRequestPriority::Recovery,
            20,
        );
        queue
            .try_admit("admitted", 8, "attempt".to_string(), 30, true)
            .unwrap();

        queue.advance_generation(9).unwrap();

        assert_eq!(
            queue.entries["queued"].state,
            PresentationRequestState::Retryable {
                recovery_required: false
            }
        );
        assert_eq!(
            queue.entries["admitted"].state,
            PresentationRequestState::Retryable {
                recovery_required: true
            }
        );
        assert_eq!(
            queue.enqueue(
                "admitted".to_string(),
                "fingerprint-admitted".to_string(),
                PresentationRequestPriority::Recovery,
                40,
                140_000,
                32,
            ),
            Err("presentation_queue_recovery_required:admitted".to_string())
        );

        let retried = queue
            .enqueue(
                "queued".to_string(),
                "fingerprint-queued".to_string(),
                PresentationRequestPriority::NewOpen,
                40,
                140_000,
                32,
            )
            .unwrap();
        assert!(matches!(retried.state, PresentationRequestState::Queued));
        assert_eq!(retried.enqueued_at_ms, 40);
        assert_eq!(retried.deadline_ms, 140_000);
    }

    #[test]
    fn expired_exact_request_becomes_retryable_without_admission() {
        let mut queue = PresentationRequestQueue::default();
        queue.advance_generation(2).unwrap();
        queue
            .enqueue(
                "expired".to_string(),
                "fingerprint-expired".to_string(),
                PresentationRequestPriority::Recovery,
                10,
                20,
                1,
            )
            .unwrap();

        assert_eq!(
            queue.try_admit("expired", 2, "attempt".to_string(), 20, true),
            Err("presentation_queue_deadline_exceeded:expired".to_string())
        );
        assert_eq!(
            queue.entries["expired"].state,
            PresentationRequestState::Retryable {
                recovery_required: false
            }
        );
    }

    #[test]
    fn depth_excludes_retryable_and_completed_but_counts_admitted() {
        let mut queue = PresentationRequestQueue::default();
        queue.advance_generation(1).unwrap();
        enqueue(
            &mut queue,
            "active",
            PresentationRequestPriority::Recovery,
            10,
        );
        queue
            .try_admit("active", 1, "attempt".to_string(), 20, true)
            .unwrap();
        assert_eq!(
            queue.enqueue(
                "blocked".to_string(),
                "fingerprint-blocked".to_string(),
                PresentationRequestPriority::NewOpen,
                20,
                120_000,
                1,
            ),
            Err("presentation_queue_full".to_string())
        );

        queue
            .complete("active", 1, "attempt", json!({"ok": true}))
            .unwrap();
        assert!(queue
            .enqueue(
                "next".to_string(),
                "fingerprint-next".to_string(),
                PresentationRequestPriority::NewOpen,
                30,
                130_000,
                1,
            )
            .is_ok());
    }

    #[test]
    fn abandoned_requests_are_bounded_and_safe_retry_cannot_bypass_recovery_limit() {
        let mut queue = PresentationRequestQueue::default();
        queue.advance_generation(1).unwrap();
        for index in 0..140 {
            queue
                .enqueue(
                    format!("expired-{index}"),
                    format!("hash-{index}"),
                    PresentationRequestPriority::NewOpen,
                    index * 10,
                    index * 10 + 1,
                    1,
                )
                .unwrap();
        }
        assert!(queue.entries.len() <= MAX_TERMINAL_ENTRIES + 1);
        queue.validate().unwrap();

        let mut queue = PresentationRequestQueue::default();
        queue.advance_generation(1).unwrap();
        enqueue(
            &mut queue,
            "safe-retry",
            PresentationRequestPriority::NewOpen,
            1,
        );
        queue.advance_generation(2).unwrap();
        for index in 0..MAX_RECOVERY_REQUIRED_ENTRIES {
            let key = format!("effect-{index}");
            let generation = queue.host_generation;
            enqueue(&mut queue, &key, PresentationRequestPriority::Recovery, 2);
            queue
                .try_admit(&key, generation, "attempt".to_string(), 3, true)
                .unwrap();
            queue.advance_generation(generation + 1).unwrap();
        }
        assert_eq!(
            queue.enqueue(
                "safe-retry".to_string(),
                "fingerprint-safe-retry".to_string(),
                PresentationRequestPriority::NewOpen,
                4,
                100_000,
                32
            ),
            Err("presentation_queue_recovery_retention_full".to_string())
        );
        assert_eq!(
            queue.recovery_required_count(),
            MAX_RECOVERY_REQUIRED_ENTRIES
        );
        queue.validate().unwrap();
    }

    #[test]
    fn reconciled_resume_rejects_stale_proof_without_mutation() {
        let (mut queue, expected) = recovery_required_queue();
        let before = queue.clone();
        assert_eq!(
            queue.resume_reconciled(&expected, 1, 30, 130_000, 32),
            Err("presentation_queue_generation_mismatch:1:2".to_string())
        );
        assert_eq!(queue, before);

        let mut stale_sequence = expected.clone();
        stale_sequence.sequence += 1;
        assert_eq!(
            queue.resume_reconciled(&stale_sequence, 2, 30, 130_000, 32),
            Err("presentation_queue_reconciled_entry_mismatch:recover".to_string())
        );
        assert_eq!(queue, before);

        let mut wrong_payload = expected.clone();
        wrong_payload.fingerprint = "other".to_string();
        assert_eq!(
            queue.resume_reconciled(&wrong_payload, 2, 30, 130_000, 32),
            Err("presentation_queue_reconciled_entry_mismatch:recover".to_string())
        );
        assert_eq!(queue, before);
    }

    #[test]
    fn reconciled_resume_requeues_with_fresh_recovery_sequence() {
        let (mut queue, expected) = recovery_required_queue();
        let resumed = queue
            .resume_reconciled(&expected, 2, 30, 130_000, 32)
            .unwrap();

        assert_eq!(resumed.key, expected.key);
        assert_eq!(resumed.fingerprint, expected.fingerprint);
        assert!(resumed.sequence > expected.sequence);
        assert_eq!(resumed.priority, PresentationRequestPriority::Recovery);
        assert_eq!(resumed.enqueued_at_ms, 30);
        assert_eq!(resumed.deadline_ms, 130_000);
        assert_eq!(resumed.state, PresentationRequestState::Queued);
        assert_eq!(queue.entries["recover"], resumed);
    }

    #[test]
    fn reconciled_resume_respects_depth_without_mutation() {
        let (mut queue, expected) = recovery_required_queue();
        enqueue(
            &mut queue,
            "active",
            PresentationRequestPriority::NewOpen,
            30,
        );
        let before = queue.clone();

        assert_eq!(
            queue.resume_reconciled(&expected, 2, 40, 140_000, 1),
            Err("presentation_queue_full".to_string())
        );
        assert_eq!(queue, before);
    }

    #[test]
    fn validation_rejects_multiple_admitted_entries() {
        let mut queue = PresentationRequestQueue {
            host_generation: 1,
            next_sequence: 2,
            entries: BTreeMap::new(),
        };
        for (sequence, key) in ["a", "b"].into_iter().enumerate() {
            queue.entries.insert(
                key.to_string(),
                PresentationRequestEntry {
                    key: key.to_string(),
                    fingerprint: format!("fingerprint-{key}"),
                    priority: PresentationRequestPriority::Recovery,
                    enqueued_at_ms: 1,
                    deadline_ms: 2,
                    sequence: sequence as u64,
                    state: PresentationRequestState::Admitted {
                        attempt_token: format!("attempt-{key}"),
                    },
                },
            );
        }

        assert_eq!(
            queue.validate(),
            Err("presentation_queue_multiple_admitted".to_string())
        );
    }
}
