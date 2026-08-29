//! Unified retention policy for package-owned profiles and runtime generations.
//!
//! The authority is effect-free: it joins durable references and filesystem
//! observations into decisions that effect adapters must recheck before apply.

use crate::runtime_adoption::{UpgradeTransaction, UpgradeTransactionState};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(crate) const EPHEMERAL_PROFILE_RETENTION: Duration = Duration::hours(24);
pub(crate) const FAILED_PROFILE_RETENTION: Duration = Duration::days(7);
pub(crate) const ACCEPTED_ROLLBACK_RETENTION: Duration = Duration::hours(24);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetentionDisposition {
    Protected,
    Reviewable,
    AutomaticallyReclaimable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileRetentionClass {
    Ephemeral,
    FailedOrQuarantined,
    Persistent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileRetentionEvidence {
    pub(crate) profile_id: String,
    pub(crate) user_data_dir: PathBuf,
    pub(crate) class: ProfileRetentionClass,
    pub(crate) terminal_since: Option<DateTime<Utc>>,
    pub(crate) projected_bytes: u64,
    pub(crate) reference_reasons: BTreeSet<String>,
    pub(crate) process_observed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetentionDecision {
    pub(crate) resource_id: String,
    pub(crate) disposition: RetentionDisposition,
    pub(crate) reason_codes: Vec<String>,
    pub(crate) projected_bytes: u64,
}

pub(crate) fn decide_profile_retention(
    evidence: &ProfileRetentionEvidence,
    now: DateTime<Utc>,
) -> RetentionDecision {
    let mut reasons = evidence
        .reference_reasons
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    if evidence.process_observed {
        reasons.push("live_process".to_string());
    }
    if evidence.class == ProfileRetentionClass::Persistent {
        reasons.push("persistent_profile_never_automatic".to_string());
    }
    if !reasons.is_empty() {
        reasons.sort();
        reasons.dedup();
        return decision(evidence, RetentionDisposition::Protected, reasons);
    }
    let Some(terminal_since) = evidence.terminal_since else {
        return decision(
            evidence,
            RetentionDisposition::Protected,
            vec!["terminal_age_unknown".to_string()],
        );
    };
    let age = now - terminal_since;
    let (minimum_age, eligible, elapsed, active) = match evidence.class {
        ProfileRetentionClass::Ephemeral => (
            EPHEMERAL_PROFILE_RETENTION,
            RetentionDisposition::AutomaticallyReclaimable,
            "ephemeral_terminal_retention_elapsed",
            "ephemeral_terminal_retention_active",
        ),
        ProfileRetentionClass::FailedOrQuarantined => (
            FAILED_PROFILE_RETENTION,
            RetentionDisposition::Reviewable,
            "failed_quarantine_retention_elapsed",
            "failed_quarantine_retention_active",
        ),
        ProfileRetentionClass::Persistent => unreachable!("handled above"),
    };
    if age >= minimum_age {
        decision(evidence, eligible, vec![elapsed.to_string()])
    } else {
        decision(
            evidence,
            RetentionDisposition::Protected,
            vec![active.to_string()],
        )
    }
}

fn decision(
    evidence: &ProfileRetentionEvidence,
    disposition: RetentionDisposition,
    reason_codes: Vec<String>,
) -> RetentionDecision {
    RetentionDecision {
        resource_id: evidence.profile_id.clone(),
        disposition,
        reason_codes,
        projected_bytes: evidence.projected_bytes,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GenerationRetentionPlan {
    pub(crate) references: BTreeMap<String, Vec<String>>,
    pub(crate) finalizable_transaction_ids: Vec<String>,
    pub(crate) previous_healthy_generation_id: Option<String>,
}

/// Keeps active transaction references and one healthy rollback generation,
/// while preserving terminal transaction files as metadata only.
pub(crate) fn plan_generation_retention(
    selected: Option<&str>,
    transactions: &[UpgradeTransaction],
    now: DateTime<Utc>,
) -> GenerationRetentionPlan {
    let mut plan = GenerationRetentionPlan::default();
    if let Some(selected) = selected {
        add_reference(&mut plan.references, selected, "selected_generation");
    }
    let mut accepted = transactions
        .iter()
        .filter(|transaction| healthy_accepted_transaction(transaction))
        .filter_map(|transaction| transaction_timestamp(transaction).map(|at| (at, transaction)))
        .collect::<Vec<_>>();
    accepted.sort_by_key(|(at, _)| *at);
    let active_accepted_transaction_id = accepted.last().and_then(|(_, latest)| {
        (Some(latest.candidate_generation_id.as_str()) == selected)
            .then_some(latest.transaction_id.as_str())
    });
    if let Some((_, latest)) = accepted.last() {
        if Some(latest.transaction_id.as_str()) == active_accepted_transaction_id {
            plan.previous_healthy_generation_id = latest.old_generation_id.clone();
        }
    }
    if let Some(previous) = plan.previous_healthy_generation_id.as_deref() {
        add_reference(
            &mut plan.references,
            previous,
            "previous_healthy_rollback_generation",
        );
    }
    for transaction in transactions {
        match transaction.state {
            UpgradeTransactionState::Accepted => {
                if !healthy_accepted_transaction(transaction) {
                    add_transaction_references(&mut plan.references, transaction);
                    continue;
                }
                if Some(transaction.transaction_id.as_str()) != active_accepted_transaction_id {
                    plan.finalizable_transaction_ids
                        .push(transaction.transaction_id.clone());
                    continue;
                }
                let open = transaction_timestamp(transaction)
                    .map(|at| now - at < ACCEPTED_ROLLBACK_RETENTION)
                    .unwrap_or(true);
                if open {
                    add_transaction_references(&mut plan.references, transaction);
                } else {
                    plan.finalizable_transaction_ids
                        .push(transaction.transaction_id.clone());
                }
            }
            UpgradeTransactionState::OldGenerationRetirable
            | UpgradeTransactionState::BlockedAmbiguousRuntime
            | UpgradeTransactionState::BlockedInflightEffect
            | UpgradeTransactionState::BlockedCandidateIncompatible
            | UpgradeTransactionState::FailedPreservedOldGeneration
            | UpgradeTransactionState::FailedEffectUncertain => {}
            UpgradeTransactionState::RollbackBeforeCommit
            | UpgradeTransactionState::RollbackAfterCommit => {
                if let Some(old) = transaction.old_generation_id.as_deref() {
                    add_reference(&mut plan.references, old, "transaction_rollback_generation");
                }
            }
            _ => add_transaction_references(&mut plan.references, transaction),
        }
    }
    plan.finalizable_transaction_ids.sort();
    for reasons in plan.references.values_mut() {
        reasons.sort();
        reasons.dedup();
    }
    plan
}

fn healthy_accepted_transaction(transaction: &UpgradeTransaction) -> bool {
    transaction.state == UpgradeTransactionState::Accepted
        && transaction.terminal_result.as_deref() == Some("accepted")
        && transaction.dashboard_validation_summary.is_some()
        && transaction.presentation_validation_summary.is_some()
}

fn add_transaction_references(
    references: &mut BTreeMap<String, Vec<String>>,
    transaction: &UpgradeTransaction,
) {
    if let Some(old) = transaction.old_generation_id.as_deref() {
        add_reference(references, old, "transaction_old_generation");
    }
    add_reference(
        references,
        &transaction.candidate_generation_id,
        "transaction_candidate_generation",
    );
}

fn add_reference(references: &mut BTreeMap<String, Vec<String>>, id: &str, reason: &str) {
    references
        .entry(id.to_string())
        .or_default()
        .push(reason.to_string());
}

pub(crate) fn transaction_timestamp(transaction: &UpgradeTransaction) -> Option<DateTime<Utc>> {
    transaction.checkpoints.last().and_then(|checkpoint| {
        DateTime::parse_from_rfc3339(&checkpoint.recorded_at)
            .ok()
            .map(|value| value.with_timezone(&Utc))
    })
}

/// Rejects root deletion, symlinks, and paths outside exact approved roots.
pub(crate) fn validate_reclaim_target(
    target: &Path,
    allowed_roots: &[PathBuf],
) -> Result<PathBuf, String> {
    let metadata = std::fs::symlink_metadata(target).map_err(|error| {
        format!(
            "retention_target_unavailable: {}: {error}",
            target.display()
        )
    })?;
    if metadata.file_type().is_symlink() {
        return Err("retention_target_symlink_rejected".to_string());
    }
    let canonical_target = target.canonicalize().map_err(|error| {
        format!(
            "retention_target_unresolvable: {}: {error}",
            target.display()
        )
    })?;
    for root in allowed_roots {
        let Ok(canonical_root) = root.canonicalize() else {
            continue;
        };
        if canonical_target != canonical_root && canonical_target.starts_with(&canonical_root) {
            return Ok(canonical_target);
        }
    }
    Err("retention_target_outside_approved_root".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuarantineManifest {
    schema_version: String,
    operation_id: String,
    resource_id: String,
    source_path: String,
    quarantine_path: String,
    projected_bytes: u64,
    state: String,
}

/// Filesystem adapter for a retention decision. The deterministic manifest
/// makes replay idempotent and lets a later invocation finish an interrupted
/// quarantine before deleting the same exact directory.
pub(crate) fn apply_profile_reclaim(
    evidence: &ProfileRetentionEvidence,
    decision: &RetentionDecision,
    approved_root: &Path,
) -> Result<u64, String> {
    if !matches!(
        decision.disposition,
        RetentionDisposition::AutomaticallyReclaimable | RetentionDisposition::Reviewable
    ) {
        return Err("retention_decision_not_reclaimable".to_string());
    }
    if !evidence.reference_reasons.is_empty() || evidence.process_observed {
        return Err("retention_reference_restored".to_string());
    }
    let operation_id = retention_operation_id(&evidence.profile_id, &evidence.user_data_dir);
    let quarantine_root = profile_quarantine_root(approved_root)?;
    let manifest_dir = quarantine_root.join("manifests");
    let quarantine_path = quarantine_root.join("payloads").join(&operation_id);
    let manifest_path = manifest_dir.join(format!("{operation_id}.json"));

    if let Ok(body) = std::fs::read(&manifest_path) {
        let manifest: QuarantineManifest = serde_json::from_slice(&body)
            .map_err(|error| format!("retention_quarantine_manifest_invalid: {error}"))?;
        if manifest.resource_id != evidence.profile_id
            || manifest.source_path != evidence.user_data_dir.display().to_string()
            || manifest.quarantine_path != quarantine_path.display().to_string()
        {
            return Err("retention_quarantine_manifest_identity_mismatch".to_string());
        }
        if manifest.state == "completed" && !evidence.user_data_dir.exists() {
            return Ok(manifest.projected_bytes);
        }
        if manifest.state == "prepared"
            && quarantine_path.exists()
            && !evidence.user_data_dir.exists()
        {
            std::fs::remove_dir_all(&quarantine_path)
                .map_err(|error| format!("retention_quarantine_resume_failed: {error}"))?;
            write_manifest(
                &manifest_path,
                &QuarantineManifest {
                    state: "completed".to_string(),
                    ..manifest
                },
            )?;
            return Ok(decision.projected_bytes);
        }
    }

    let source = validate_reclaim_target(&evidence.user_data_dir, &[approved_root.to_path_buf()])?;
    std::fs::create_dir_all(quarantine_path.parent().expect("payload parent"))
        .map_err(|error| format!("retention_quarantine_create_failed: {error}"))?;
    std::fs::create_dir_all(&manifest_dir)
        .map_err(|error| format!("retention_manifest_create_failed: {error}"))?;
    let mut manifest = QuarantineManifest {
        schema_version: "agent-browser.retention-quarantine.v1".to_string(),
        operation_id,
        resource_id: evidence.profile_id.clone(),
        source_path: evidence.user_data_dir.display().to_string(),
        quarantine_path: quarantine_path.display().to_string(),
        projected_bytes: decision.projected_bytes,
        state: "prepared".to_string(),
    };
    write_manifest(&manifest_path, &manifest)?;
    std::fs::rename(&source, &quarantine_path)
        .map_err(|error| format!("retention_quarantine_move_failed: {error}"))?;
    std::fs::remove_dir_all(&quarantine_path)
        .map_err(|error| format!("retention_quarantine_delete_failed: {error}"))?;
    manifest.state = "completed".to_string();
    write_manifest(&manifest_path, &manifest)?;
    Ok(decision.projected_bytes)
}

/// Completes exact prepared quarantine manifests left by an interrupted apply.
/// Completed manifests and missing quarantine roots make replay a no-op.
pub(crate) fn resume_profile_reclaims(approved_root: &Path) -> Result<Vec<String>, String> {
    let quarantine_root = profile_quarantine_root(approved_root)?;
    let manifest_dir = quarantine_root.join("manifests");
    let entries = match std::fs::read_dir(&manifest_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("retention_manifest_inventory_failed: {error}")),
    };
    let payload_root = quarantine_root.join("payloads");
    let mut resumed = Vec::new();
    for entry in entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
    {
        let manifest_path = entry.path();
        let body = std::fs::read(&manifest_path)
            .map_err(|error| format!("retention_quarantine_manifest_unreadable: {error}"))?;
        let mut manifest: QuarantineManifest = serde_json::from_slice(&body)
            .map_err(|error| format!("retention_quarantine_manifest_invalid: {error}"))?;
        if manifest.schema_version != "agent-browser.retention-quarantine.v1" {
            return Err("retention_quarantine_manifest_schema_unsupported".to_string());
        }
        if manifest.state == "completed" {
            continue;
        }
        let source = PathBuf::from(&manifest.source_path);
        let quarantine = PathBuf::from(&manifest.quarantine_path);
        if source.exists() {
            return Err("retention_quarantine_source_restored".to_string());
        }
        if quarantine.parent() != Some(payload_root.as_path())
            || quarantine.file_name().and_then(|value| value.to_str())
                != Some(manifest.operation_id.as_str())
        {
            return Err("retention_quarantine_manifest_path_mismatch".to_string());
        }
        if quarantine.exists() {
            let metadata = std::fs::symlink_metadata(&quarantine)
                .map_err(|error| format!("retention_quarantine_inspect_failed: {error}"))?;
            if metadata.file_type().is_symlink() {
                return Err("retention_quarantine_symlink_rejected".to_string());
            }
            std::fs::remove_dir_all(&quarantine)
                .map_err(|error| format!("retention_quarantine_resume_failed: {error}"))?;
        }
        manifest.state = "completed".to_string();
        write_manifest(&manifest_path, &manifest)?;
        resumed.push(manifest.resource_id);
    }
    resumed.sort();
    Ok(resumed)
}

fn profile_quarantine_root(approved_root: &Path) -> Result<PathBuf, String> {
    if approved_root == std::env::temp_dir() {
        Ok(approved_root
            .join("agent-browser")
            .join("retention-quarantine"))
    } else {
        Ok(approved_root
            .parent()
            .ok_or_else(|| "retention_approved_root_has_no_parent".to_string())?
            .join("retention-quarantine"))
    }
}

fn write_manifest(path: &Path, manifest: &QuarantineManifest) -> Result<(), String> {
    let staged = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let body = serde_json::to_vec_pretty(manifest)
        .map_err(|error| format!("retention_manifest_serialize_failed: {error}"))?;
    std::fs::write(&staged, body)
        .map_err(|error| format!("retention_manifest_stage_failed: {error}"))?;
    std::fs::rename(&staged, path)
        .map_err(|error| format!("retention_manifest_commit_failed: {error}"))
}

fn retention_operation_id(profile_id: &str, path: &Path) -> String {
    let mut digest = Sha256::new();
    digest.update(profile_id.as_bytes());
    digest.update([0]);
    digest.update(path.as_os_str().as_encoded_bytes());
    format!("profile-{}", hex::encode(digest.finalize()))
}

pub(crate) fn directory_projected_bytes(path: &Path) -> Result<u64, String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("retention_inventory_failed: {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err("retention_inventory_symlink_rejected".to_string());
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    let mut bytes = 0u64;
    for entry in std::fs::read_dir(path)
        .map_err(|error| format!("retention_inventory_failed: {}: {error}", path.display()))?
    {
        let entry = entry.map_err(|error| format!("retention_inventory_entry_failed: {error}"))?;
        bytes = bytes.saturating_add(directory_projected_bytes(&entry.path())?);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_adoption::{UpgradeCheckpoint, RUNTIME_ADOPTION_SCHEMA_VERSION};

    fn accepted(id: &str, old: &str, candidate: &str, at: &str) -> UpgradeTransaction {
        UpgradeTransaction {
            schema_version: RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
            transaction_id: id.to_string(),
            requested_by: "test".to_string(),
            old_generation_id: Some(old.to_string()),
            candidate_generation_id: candidate.to_string(),
            candidate_binary_sha256: "a".repeat(64),
            candidate_support_manifest_sha256: "b".repeat(64),
            runtime_census_digest: None,
            runtime_migrations: Vec::new(),
            runtime_handoffs: Vec::new(),
            runtime_host_convergence: None,
            service_state_migration: None,
            state: UpgradeTransactionState::Accepted,
            revision: 1,
            checkpoints: vec![UpgradeCheckpoint {
                name: "accepted".to_string(),
                transaction_revision: 1,
                recorded_at: at.to_string(),
            }],
            dashboard_validation_summary: Some("ready".to_string()),
            presentation_validation_summary: Some("ready".to_string()),
            terminal_result: Some("accepted".to_string()),
            stop_reason: None,
            successor_fields: std::collections::BTreeMap::new(),
        }
    }

    fn at(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn profile_windows_separate_automatic_reviewed_and_persistent() {
        let now = at("2026-08-20T12:00:00Z");
        let evidence = |class, hours| ProfileRetentionEvidence {
            profile_id: format!("profile-{hours}"),
            user_data_dir: PathBuf::from(format!("/profiles/{hours}")),
            class,
            terminal_since: Some(now - Duration::hours(hours)),
            projected_bytes: 42,
            reference_reasons: BTreeSet::new(),
            process_observed: false,
        };
        assert_eq!(
            decide_profile_retention(&evidence(ProfileRetentionClass::Ephemeral, 25), now)
                .disposition,
            RetentionDisposition::AutomaticallyReclaimable
        );
        assert_eq!(
            decide_profile_retention(
                &evidence(ProfileRetentionClass::FailedOrQuarantined, 169),
                now
            )
            .disposition,
            RetentionDisposition::Reviewable
        );
        assert_eq!(
            decide_profile_retention(&evidence(ProfileRetentionClass::Persistent, 10_000), now)
                .disposition,
            RetentionDisposition::Protected
        );
    }

    #[test]
    fn restored_reference_protects_expired_ephemeral_profile() {
        let now = Utc::now();
        let mut evidence = ProfileRetentionEvidence {
            profile_id: "profile".to_string(),
            user_data_dir: PathBuf::from("/profiles/profile"),
            class: ProfileRetentionClass::Ephemeral,
            terminal_since: Some(now - Duration::days(2)),
            projected_bytes: 42,
            reference_reasons: BTreeSet::from(["durable_handoff".to_string()]),
            process_observed: false,
        };
        assert_eq!(
            decide_profile_retention(&evidence, now).disposition,
            RetentionDisposition::Protected
        );
        evidence.reference_reasons.clear();
        assert_eq!(
            decide_profile_retention(&evidence, now).disposition,
            RetentionDisposition::AutomaticallyReclaimable
        );
    }

    #[test]
    fn historical_transactions_do_not_pin_ancient_generations() {
        let transactions = vec![
            accepted("new", "previous", "current", "2026-08-18T00:00:00Z"),
            accepted("old", "ancient", "previous", "2026-08-01T00:00:00Z"),
        ];
        let plan =
            plan_generation_retention(Some("current"), &transactions, at("2026-08-20T12:00:00Z"));
        assert_eq!(plan.references.len(), 2);
        assert_eq!(
            plan.references["previous"],
            vec!["previous_healthy_rollback_generation"]
        );
        assert!(!plan.references.contains_key("ancient"));
        assert_eq!(plan.finalizable_transaction_ids, vec!["new", "old"]);
    }

    #[test]
    fn superseded_accepted_transactions_do_not_multiply_rollback_generations() {
        let transactions = vec![
            accepted(
                "current",
                "generation-previous",
                "generation-current",
                "2026-08-20T11:00:00Z",
            ),
            accepted(
                "superseded",
                "generation-ancient",
                "generation-previous",
                "2026-08-20T10:00:00Z",
            ),
        ];

        let plan = plan_generation_retention(
            Some("generation-current"),
            &transactions,
            at("2026-08-20T12:00:00Z"),
        );

        assert_eq!(plan.references.len(), 2);
        assert!(plan.references.contains_key("generation-current"));
        assert_eq!(
            plan.references["generation-previous"],
            vec![
                "previous_healthy_rollback_generation",
                "transaction_old_generation",
            ]
        );
        assert!(!plan.references.contains_key("generation-ancient"));
        assert_eq!(plan.finalizable_transaction_ids, vec!["superseded"]);
    }

    #[test]
    fn live_shaped_history_converges_twenty_one_generations_to_current_and_previous() {
        let mut transactions = vec![
            accepted(
                "accepted-current",
                "generation-19",
                "generation-20",
                "2026-08-18T00:00:00Z",
            ),
            accepted(
                "accepted-previous",
                "generation-18",
                "generation-19",
                "2026-08-01T00:00:00Z",
            ),
        ];
        for index in 0..26 {
            let mut transaction = accepted(
                &format!("blocked-{index}"),
                &format!("generation-{}", index % 21),
                &format!("generation-{}", (index + 1) % 21),
                "2026-07-01T00:00:00Z",
            );
            transaction.state = UpgradeTransactionState::BlockedAmbiguousRuntime;
            transactions.push(transaction);
        }
        for index in 0..20 {
            let mut transaction = accepted(
                &format!("failed-{index}"),
                &format!("generation-{}", index % 21),
                &format!("generation-{}", (index + 2) % 21),
                "2026-07-02T00:00:00Z",
            );
            transaction.state = UpgradeTransactionState::FailedPreservedOldGeneration;
            transactions.push(transaction);
        }
        let mut inflight = accepted(
            "blocked-inflight",
            "generation-4",
            "generation-5",
            "2026-07-03T00:00:00Z",
        );
        inflight.state = UpgradeTransactionState::BlockedInflightEffect;
        transactions.push(inflight);
        assert_eq!(transactions.len(), 49);

        let plan = plan_generation_retention(
            Some("generation-20"),
            &transactions,
            at("2026-08-20T12:00:00Z"),
        );
        assert_eq!(plan.references.len(), 2);
        assert!(plan.references.contains_key("generation-20"));
        assert!(plan.references.contains_key("generation-19"));
        assert_eq!(plan.finalizable_transaction_ids.len(), 2);
        assert_eq!(transactions.len(), 49, "durable metadata remains intact");
    }

    #[test]
    fn exact_root_validation_rejects_root_and_symlink_targets() {
        let temp = std::env::temp_dir().join(format!("retention-root-{}", uuid::Uuid::new_v4()));
        let root = temp.join("profiles");
        let target = root.join("candidate");
        std::fs::create_dir_all(&target).unwrap();
        assert_eq!(
            validate_reclaim_target(&target, std::slice::from_ref(&root)).unwrap(),
            target.canonicalize().unwrap()
        );
        assert_eq!(
            validate_reclaim_target(&root, std::slice::from_ref(&root)).unwrap_err(),
            "retention_target_outside_approved_root"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link = root.join("link");
            symlink(&target, &link).unwrap();
            assert_eq!(
                validate_reclaim_target(&link, std::slice::from_ref(&root)).unwrap_err(),
                "retention_target_symlink_rejected"
            );
        }
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn quarantine_apply_is_idempotent_and_retains_a_completed_manifest() {
        let temp = std::env::temp_dir().join(format!("retention-apply-{}", uuid::Uuid::new_v4()));
        let root = temp.join("runtime-profiles");
        let target = root.join("candidate");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("payload"), b"fixture").unwrap();
        let evidence = ProfileRetentionEvidence {
            profile_id: "candidate".to_string(),
            user_data_dir: target.clone(),
            class: ProfileRetentionClass::Ephemeral,
            terminal_since: Some(Utc::now() - Duration::days(2)),
            projected_bytes: 7,
            reference_reasons: BTreeSet::new(),
            process_observed: false,
        };
        let decision = decide_profile_retention(&evidence, Utc::now());
        assert_eq!(
            apply_profile_reclaim(&evidence, &decision, &root).unwrap(),
            7
        );
        assert!(!target.exists());
        assert_eq!(
            apply_profile_reclaim(&evidence, &decision, &root).unwrap(),
            7
        );
        let manifest = temp.join("retention-quarantine/manifests");
        let body =
            std::fs::read(manifest.read_dir().unwrap().next().unwrap().unwrap().path()).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["state"],
            "completed"
        );
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn interrupted_quarantine_is_resumed_without_a_profile_record() {
        let temp = std::env::temp_dir().join(format!("retention-resume-{}", uuid::Uuid::new_v4()));
        let root = temp.join("runtime-profiles");
        let source = root.join("candidate");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("payload"), b"fixture").unwrap();
        let operation_id = retention_operation_id("candidate", &source);
        let quarantine_root = profile_quarantine_root(&root).unwrap();
        let quarantine = quarantine_root.join("payloads").join(&operation_id);
        let manifest_path = quarantine_root
            .join("manifests")
            .join(format!("{operation_id}.json"));
        std::fs::create_dir_all(quarantine.parent().unwrap()).unwrap();
        std::fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();
        std::fs::rename(&source, &quarantine).unwrap();
        write_manifest(
            &manifest_path,
            &QuarantineManifest {
                schema_version: "agent-browser.retention-quarantine.v1".to_string(),
                operation_id,
                resource_id: "candidate".to_string(),
                source_path: source.display().to_string(),
                quarantine_path: quarantine.display().to_string(),
                projected_bytes: 7,
                state: "prepared".to_string(),
            },
        )
        .unwrap();

        assert_eq!(resume_profile_reclaims(&root).unwrap(), vec!["candidate"]);
        assert!(!quarantine.exists());
        assert_eq!(
            resume_profile_reclaims(&root).unwrap(),
            Vec::<String>::new()
        );
        std::fs::remove_dir_all(temp).unwrap();
    }
}
