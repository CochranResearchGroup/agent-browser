//! Generation-aware runtime adoption authority models.
//!
//! Plan 0116 Slice A intentionally keeps these models provider-free and
//! effect-free. They freeze the census decision vocabulary and durable receipt
//! shapes before installer, daemon, dashboard, or browser behavior changes.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const RUNTIME_ADOPTION_SCHEMA_VERSION: &str = "agent-browser.runtime-adoption.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeGenerationState {
    Staged,
    Validating,
    Candidate,
    Current,
    Rollback,
    Retired,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeGeneration {
    pub(crate) schema_version: String,
    pub(crate) generation_id: String,
    pub(crate) package_version: String,
    pub(crate) binary_sha256: String,
    pub(crate) support_manifest_sha256: String,
    pub(crate) controller_compatibility_version: u32,
    pub(crate) schema_compatibility_version: u32,
    pub(crate) immutable_installation_path: String,
    pub(crate) created_at: String,
    pub(crate) accepted_at: Option<String>,
    pub(crate) state: RuntimeGenerationState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeClassification {
    CooperativeLiveOwner,
    OrphanAdoptable,
    ManualPreserveOnly,
    IdleDaemon,
    StaleMetadata,
    ExternalObserved,
    ConflictingOwner,
    InsufficientEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeDisposition {
    CooperativeTransfer,
    OrphanAdoption,
    ManualPreservation,
    RetiredIdle,
    RejectedAmbiguity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UpgradeTransactionState {
    Planned,
    CandidateStaged,
    CandidatePreflightReady,
    CensusStable,
    AdmissionDraining,
    RuntimesTransferring,
    PresentationsRebinding,
    CandidateReady,
    GenerationCommitted,
    PostCommitValidating,
    Accepted,
    OldGenerationRetirable,
    BlockedAmbiguousRuntime,
    BlockedInflightEffect,
    BlockedCandidateIncompatible,
    RollbackBeforeCommit,
    RollbackAfterCommit,
    OperatorRecoveryRequired,
    FailedPreservedOldGeneration,
    FailedEffectUncertain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeMigrationRecord {
    pub(crate) logical_browser_id: String,
    pub(crate) profile_identity_digest: String,
    pub(crate) classification: RuntimeClassification,
    pub(crate) disposition: RuntimeDisposition,
    pub(crate) adoption_receipt_id: Option<String>,
    pub(crate) reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpgradeCheckpoint {
    pub(crate) name: String,
    pub(crate) transaction_revision: u64,
    pub(crate) recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpgradeTransaction {
    pub(crate) schema_version: String,
    pub(crate) transaction_id: String,
    pub(crate) requested_by: String,
    pub(crate) old_generation_id: Option<String>,
    pub(crate) candidate_generation_id: String,
    pub(crate) candidate_binary_sha256: String,
    pub(crate) candidate_support_manifest_sha256: String,
    pub(crate) runtime_census_digest: Option<String>,
    pub(crate) runtime_migrations: Vec<RuntimeMigrationRecord>,
    pub(crate) state: UpgradeTransactionState,
    pub(crate) revision: u64,
    pub(crate) checkpoints: Vec<UpgradeCheckpoint>,
    pub(crate) dashboard_validation_summary: Option<String>,
    pub(crate) presentation_validation_summary: Option<String>,
    pub(crate) terminal_result: Option<String>,
    pub(crate) stop_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BrowserAdoptionMode {
    CooperativeTransfer,
    OrphanAdoption,
    ManualPreservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BrowserAdoptionDecision {
    Authorized,
    PreservedWithoutAutomation,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BrowserAdoptionReceipt {
    pub(crate) schema_version: String,
    pub(crate) receipt_id: String,
    pub(crate) logical_browser_id: String,
    pub(crate) profile_identity_digest: String,
    pub(crate) browser_pid: u32,
    pub(crate) process_instance_digest: String,
    pub(crate) executable_family: String,
    pub(crate) executable_identity_digest: String,
    pub(crate) cdp_endpoint_identity_digest: String,
    pub(crate) browser_version_digest: String,
    pub(crate) target_set_digest: String,
    pub(crate) selected_target_identity_digest: String,
    pub(crate) runtime_profile_identity_digest: String,
    pub(crate) runtime_profile_agreement: bool,
    pub(crate) service_state_revision: u64,
    pub(crate) service_state_agreement: bool,
    pub(crate) display_identity_digest: Option<String>,
    pub(crate) geometry_epoch: Option<String>,
    pub(crate) route_identity_digest: Option<String>,
    pub(crate) stream_provider: Option<String>,
    pub(crate) presentation_agreement: Option<bool>,
    pub(crate) previous_owner_generation: u64,
    pub(crate) candidate_owner_generation: u64,
    pub(crate) adoption_mode: BrowserAdoptionMode,
    pub(crate) decision: BrowserAdoptionDecision,
    pub(crate) reason_codes: Vec<String>,
    pub(crate) recorded_at: String,
    pub(crate) retention_posture: String,
    pub(crate) redaction_posture: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PresentationState {
    Ready,
    Converging,
    Blocked,
    WrongProvider,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PresentationReceipt {
    pub(crate) schema_version: String,
    pub(crate) receipt_id: String,
    pub(crate) dashboard_deployment_generation: String,
    pub(crate) coordinator_generation: String,
    pub(crate) daemon_generation: String,
    pub(crate) logical_browser_id: String,
    pub(crate) process_instance_digest: String,
    pub(crate) selected_target_generation: u64,
    pub(crate) selected_target_identity_digest: String,
    pub(crate) required_stream_provider: String,
    pub(crate) display_allocation_id: String,
    pub(crate) geometry_epoch: String,
    pub(crate) route_generation: u64,
    pub(crate) guacamole_connection_generation: Option<u64>,
    pub(crate) authenticated_ingress_probe_at: String,
    pub(crate) operator_surface_load_result: String,
    pub(crate) state: PresentationState,
    pub(crate) reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeCensusSource {
    ServiceBrowserRecords,
    RuntimeProfileState,
    ProfileOwnerReservations,
    NamedSessionSupervisors,
    DaemonMetadata,
    OperatingSystemProcessIdentity,
    ProfileLockAndDevtools,
    CdpBrowserAndTargets,
    DisplayAllocationsAndVisibleWindowProof,
    ViewStreamsRoutePoolGuacamoleAndHandoffs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeCensusSourceEntry {
    pub(crate) source: RuntimeCensusSource,
    pub(crate) observation_only: bool,
    pub(crate) authority_limit: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceAgreement {
    Match,
    Mismatch,
    Missing,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeEvidenceSummary {
    pub(crate) browser_live: bool,
    pub(crate) daemon_live: bool,
    pub(crate) daemon_cooperative: bool,
    pub(crate) manual_browser: bool,
    pub(crate) externally_owned: bool,
    pub(crate) metadata_present: bool,
    pub(crate) observation_rounds_agree: bool,
    pub(crate) registry_revision_stable: bool,
    pub(crate) owner_generations: Vec<u64>,
    pub(crate) process_identity: EvidenceAgreement,
    pub(crate) profile_identity: EvidenceAgreement,
    pub(crate) browser_family: EvidenceAgreement,
    pub(crate) cdp_endpoint: EvidenceAgreement,
    pub(crate) target_set: EvidenceAgreement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeCensusFixture {
    pub(crate) fixture_id: String,
    pub(crate) observed_sources: Vec<RuntimeCensusSource>,
    pub(crate) evidence: RuntimeEvidenceSummary,
    pub(crate) expected_classification: RuntimeClassification,
    pub(crate) expected_reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeCensusCorpus {
    pub(crate) schema_version: String,
    pub(crate) source_ledger: Vec<RuntimeCensusSourceEntry>,
    pub(crate) fixtures: Vec<RuntimeCensusFixture>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeCensusSourceSnapshot {
    pub(crate) source: RuntimeCensusSource,
    pub(crate) source_revision: String,
    pub(crate) logical_browser_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeCensusCandidate {
    pub(crate) logical_browser_id: String,
    pub(crate) profile_identity_digest: String,
    pub(crate) observed_sources: Vec<RuntimeCensusSource>,
    pub(crate) evidence: RuntimeEvidenceSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeCensusRound {
    pub(crate) registry_revision: u64,
    pub(crate) source_snapshots: Vec<RuntimeCensusSourceSnapshot>,
    pub(crate) candidates: Vec<RuntimeCensusCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeCensusRecord {
    pub(crate) logical_browser_id: String,
    pub(crate) profile_identity_digest: String,
    pub(crate) observed_sources: Vec<RuntimeCensusSource>,
    pub(crate) classification: RuntimeClassification,
    pub(crate) disposition: RuntimeDisposition,
    pub(crate) reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StableRuntimeCensus {
    pub(crate) schema_version: String,
    pub(crate) digest: String,
    pub(crate) registry_revision: u64,
    pub(crate) activation_allowed: bool,
    pub(crate) records: Vec<RuntimeCensusRecord>,
}

pub(crate) fn collect_runtime_census_round(
    registry_revision: u64,
    mut source_snapshots: Vec<RuntimeCensusSourceSnapshot>,
    mut candidates: Vec<RuntimeCensusCandidate>,
) -> Result<RuntimeCensusRound, String> {
    let expected_sources = runtime_census_sources()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let actual_sources = source_snapshots
        .iter()
        .map(|snapshot| snapshot.source)
        .collect::<BTreeSet<_>>();
    if source_snapshots.len() != expected_sources.len() || actual_sources != expected_sources {
        return Err("runtime census source set is not closed".to_string());
    }

    let mut observed_by_runtime = BTreeMap::<String, BTreeSet<RuntimeCensusSource>>::new();
    for snapshot in &mut source_snapshots {
        if snapshot.source_revision.trim().is_empty() {
            return Err("runtime census source revision is missing".to_string());
        }
        snapshot.logical_browser_ids.sort();
        if snapshot
            .logical_browser_ids
            .windows(2)
            .any(|pair| pair[0] == pair[1])
            || snapshot
                .logical_browser_ids
                .iter()
                .any(|runtime_id| runtime_id.trim().is_empty())
        {
            return Err("runtime census source contains an invalid runtime set".to_string());
        }
        for runtime_id in &snapshot.logical_browser_ids {
            observed_by_runtime
                .entry(runtime_id.clone())
                .or_default()
                .insert(snapshot.source);
        }
    }

    candidates.sort_by(|left, right| left.logical_browser_id.cmp(&right.logical_browser_id));
    if candidates
        .windows(2)
        .any(|pair| pair[0].logical_browser_id == pair[1].logical_browser_id)
        || candidates.iter().any(|candidate| {
            candidate.logical_browser_id.trim().is_empty()
                || !is_sha256(&candidate.profile_identity_digest)
        })
    {
        return Err(
            "runtime census candidate set contains duplicates or missing identity".to_string(),
        );
    }
    let candidate_ids = candidates
        .iter()
        .map(|candidate| candidate.logical_browser_id.clone())
        .collect::<BTreeSet<_>>();
    if candidate_ids != observed_by_runtime.keys().cloned().collect::<BTreeSet<_>>() {
        return Err(
            "runtime census candidate set does not match the observed runtime union".to_string(),
        );
    }
    for candidate in &mut candidates {
        let expected = observed_by_runtime
            .get(&candidate.logical_browser_id)
            .expect("candidate union was checked");
        let actual = candidate
            .observed_sources
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if candidate.observed_sources.len() != actual.len() || &actual != expected {
            return Err("runtime census candidate source attribution is incomplete".to_string());
        }
        candidate.observed_sources = actual.into_iter().collect();
    }
    source_snapshots.sort_by_key(|snapshot| snapshot.source);
    Ok(RuntimeCensusRound {
        registry_revision,
        source_snapshots,
        candidates,
    })
}

pub(crate) fn build_stable_runtime_census(
    first: &RuntimeCensusRound,
    second: &RuntimeCensusRound,
) -> Result<StableRuntimeCensus, String> {
    validate_runtime_census_round(first)?;
    validate_runtime_census_round(second)?;
    let first_candidates = first
        .candidates
        .iter()
        .map(|candidate| (candidate.logical_browser_id.as_str(), candidate))
        .collect::<BTreeMap<_, _>>();
    let second_candidates = second
        .candidates
        .iter()
        .map(|candidate| (candidate.logical_browser_id.as_str(), candidate))
        .collect::<BTreeMap<_, _>>();
    let runtime_ids = first_candidates
        .keys()
        .chain(second_candidates.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let registry_revision_stable = first.registry_revision == second.registry_revision;
    let source_revisions_stable = first.source_snapshots == second.source_snapshots;
    let mut records = Vec::with_capacity(runtime_ids.len());

    for runtime_id in runtime_ids {
        let first_candidate = first_candidates.get(runtime_id).copied();
        let second_candidate = second_candidates.get(runtime_id).copied();
        let selected = second_candidate
            .or(first_candidate)
            .expect("runtime id came from a round");
        let mut evidence = selected.evidence.clone();
        evidence.observation_rounds_agree &= first_candidate == second_candidate;
        evidence.registry_revision_stable &= registry_revision_stable && source_revisions_stable;
        let decision = if !evidence.observation_rounds_agree || !evidence.registry_revision_stable {
            decision(
                RuntimeClassification::InsufficientEvidence,
                &["census_changed_during_classification"],
            )
        } else {
            classify_runtime(&evidence)
        };
        records.push(RuntimeCensusRecord {
            logical_browser_id: selected.logical_browser_id.clone(),
            profile_identity_digest: selected.profile_identity_digest.clone(),
            observed_sources: selected.observed_sources.clone(),
            classification: decision.classification,
            disposition: disposition_for_classification(decision.classification),
            reason_codes: decision
                .reason_codes
                .into_iter()
                .map(str::to_string)
                .collect(),
        });
    }

    let activation_allowed = records.iter().all(|record| {
        !matches!(
            record.classification,
            RuntimeClassification::ConflictingOwner | RuntimeClassification::InsufficientEvidence
        )
    });
    let registry_revision = second.registry_revision;
    let digest_payload = serde_json::to_vec(&serde_json::json!({
        "schemaVersion": RUNTIME_ADOPTION_SCHEMA_VERSION,
        "registryRevision": registry_revision,
        "sourceSnapshots": second.source_snapshots,
        "records": records,
    }))
    .expect("runtime census digest payload must serialize");
    let digest = format!("{:x}", Sha256::digest(digest_payload));
    Ok(StableRuntimeCensus {
        schema_version: RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
        digest,
        registry_revision,
        activation_allowed,
        records,
    })
}

pub(crate) fn persist_runtime_census(
    transaction: &mut UpgradeTransaction,
    census: &StableRuntimeCensus,
    recorded_at: &str,
) {
    transaction.runtime_census_digest = Some(census.digest.clone());
    transaction.runtime_migrations = census
        .records
        .iter()
        .map(|record| RuntimeMigrationRecord {
            logical_browser_id: record.logical_browser_id.clone(),
            profile_identity_digest: record.profile_identity_digest.clone(),
            classification: record.classification,
            disposition: record.disposition,
            adoption_receipt_id: None,
            reason_codes: record.reason_codes.clone(),
        })
        .collect();
    transaction.revision = transaction.revision.saturating_add(1);
    transaction.state = if census.activation_allowed {
        UpgradeTransactionState::CensusStable
    } else {
        UpgradeTransactionState::BlockedAmbiguousRuntime
    };
    transaction.stop_reason =
        (!census.activation_allowed).then(|| "runtime_census_ambiguous".to_string());
    transaction.checkpoints.push(UpgradeCheckpoint {
        name: if census.activation_allowed {
            "census_stable"
        } else {
            "census_blocked_ambiguous_runtime"
        }
        .to_string(),
        transaction_revision: transaction.revision,
        recorded_at: recorded_at.to_string(),
    });
}

fn validate_runtime_census_round(round: &RuntimeCensusRound) -> Result<(), String> {
    collect_runtime_census_round(
        round.registry_revision,
        round.source_snapshots.clone(),
        round.candidates.clone(),
    )
    .map(|_| ())
}

fn disposition_for_classification(classification: RuntimeClassification) -> RuntimeDisposition {
    match classification {
        RuntimeClassification::CooperativeLiveOwner => RuntimeDisposition::CooperativeTransfer,
        RuntimeClassification::OrphanAdoptable => RuntimeDisposition::OrphanAdoption,
        RuntimeClassification::ManualPreserveOnly | RuntimeClassification::ExternalObserved => {
            RuntimeDisposition::ManualPreservation
        }
        RuntimeClassification::IdleDaemon | RuntimeClassification::StaleMetadata => {
            RuntimeDisposition::RetiredIdle
        }
        RuntimeClassification::ConflictingOwner | RuntimeClassification::InsufficientEvidence => {
            RuntimeDisposition::RejectedAmbiguity
        }
    }
}

fn runtime_census_sources() -> [RuntimeCensusSource; 10] {
    [
        RuntimeCensusSource::ServiceBrowserRecords,
        RuntimeCensusSource::RuntimeProfileState,
        RuntimeCensusSource::ProfileOwnerReservations,
        RuntimeCensusSource::NamedSessionSupervisors,
        RuntimeCensusSource::DaemonMetadata,
        RuntimeCensusSource::OperatingSystemProcessIdentity,
        RuntimeCensusSource::ProfileLockAndDevtools,
        RuntimeCensusSource::CdpBrowserAndTargets,
        RuntimeCensusSource::DisplayAllocationsAndVisibleWindowProof,
        RuntimeCensusSource::ViewStreamsRoutePoolGuacamoleAndHandoffs,
    ]
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeClassificationDecision {
    pub(crate) classification: RuntimeClassification,
    pub(crate) reason_codes: Vec<&'static str>,
}

pub(crate) fn classify_runtime(evidence: &RuntimeEvidenceSummary) -> RuntimeClassificationDecision {
    let unique_owner_generations = evidence
        .owner_generations
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if unique_owner_generations.len() > 1 {
        return decision(
            RuntimeClassification::ConflictingOwner,
            &["multiple_owner_generations"],
        );
    }
    if !evidence.observation_rounds_agree || !evidence.registry_revision_stable {
        return decision(
            RuntimeClassification::InsufficientEvidence,
            &["census_changed_during_classification"],
        );
    }
    if !evidence.browser_live && !evidence.daemon_live && evidence.metadata_present {
        return decision(
            RuntimeClassification::StaleMetadata,
            &["metadata_without_live_runtime"],
        );
    }
    if evidence.daemon_live && !evidence.browser_live {
        return decision(
            RuntimeClassification::IdleDaemon,
            &["daemon_without_live_browser"],
        );
    }
    if evidence.externally_owned {
        return decision(
            RuntimeClassification::ExternalObserved,
            &["external_owner_preserved"],
        );
    }
    if evidence.browser_live
        && evidence.manual_browser
        && evidence.cdp_endpoint == EvidenceAgreement::Missing
    {
        return decision(
            RuntimeClassification::ManualPreserveOnly,
            &["manual_browser_without_cdp"],
        );
    }

    let identity_evidence = [
        (evidence.process_identity, "process_identity"),
        (evidence.profile_identity, "profile_identity"),
        (evidence.browser_family, "browser_family"),
        (evidence.cdp_endpoint, "cdp_endpoint"),
        (evidence.target_set, "target_set"),
    ];
    let mismatches = identity_evidence
        .iter()
        .filter_map(|(agreement, name)| {
            (*agreement == EvidenceAgreement::Mismatch).then_some(match *name {
                "process_identity" => "process_identity_mismatch",
                "profile_identity" => "profile_identity_mismatch",
                "browser_family" => "browser_family_mismatch",
                "cdp_endpoint" => "cdp_endpoint_mismatch",
                "target_set" => "target_set_mismatch",
                _ => unreachable!(),
            })
        })
        .collect::<Vec<_>>();
    if !mismatches.is_empty() {
        return RuntimeClassificationDecision {
            classification: RuntimeClassification::InsufficientEvidence,
            reason_codes: mismatches,
        };
    }
    if identity_evidence
        .iter()
        .any(|(agreement, _)| *agreement == EvidenceAgreement::Missing)
    {
        return decision(
            RuntimeClassification::InsufficientEvidence,
            &["required_identity_evidence_missing"],
        );
    }
    if evidence.browser_live && evidence.daemon_live && evidence.daemon_cooperative {
        return decision(
            RuntimeClassification::CooperativeLiveOwner,
            &["cooperative_owner_verified"],
        );
    }
    if evidence.browser_live && !evidence.daemon_live && unique_owner_generations.is_empty() {
        return decision(
            RuntimeClassification::OrphanAdoptable,
            &["verified_browser_without_live_daemon"],
        );
    }
    decision(
        RuntimeClassification::InsufficientEvidence,
        &["runtime_authority_not_proven"],
    )
}

fn decision(
    classification: RuntimeClassification,
    reason_codes: &[&'static str],
) -> RuntimeClassificationDecision {
    RuntimeClassificationDecision {
        classification,
        reason_codes: reason_codes.to_vec(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UnsafeSeamFixture {
    pub(crate) fixture_id: String,
    pub(crate) source_anchors: Vec<String>,
    pub(crate) current_sequence: Vec<String>,
    pub(crate) required_sequence: Vec<String>,
    pub(crate) expected_red_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UnsafeSeamCorpus {
    pub(crate) schema_version: String,
    pub(crate) fixtures: Vec<UnsafeSeamFixture>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnsafeSeamDecision {
    pub(crate) passes_required_sequence: bool,
    pub(crate) reason: Option<String>,
}

pub(crate) fn evaluate_unsafe_seam(fixture: &UnsafeSeamFixture) -> UnsafeSeamDecision {
    let mut current_index = 0;
    for required in &fixture.required_sequence {
        let Some(offset) = fixture.current_sequence[current_index..]
            .iter()
            .position(|step| step == required)
        else {
            return UnsafeSeamDecision {
                passes_required_sequence: false,
                reason: Some(fixture.expected_red_reason.clone()),
            };
        };
        current_index += offset + 1;
    }
    UnsafeSeamDecision {
        passes_required_sequence: true,
        reason: None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::Value;

    use super::*;

    const CENSUS_CORPUS: &str =
        include_str!("../../docs/dev/fixtures/runtime-adoption/census-classification.v1.json");
    const UNSAFE_SEAMS: &str =
        include_str!("../../docs/dev/fixtures/runtime-adoption/unsafe-seams.v1.json");
    const SCHEMA_SAMPLES: &str =
        include_str!("../../docs/dev/fixtures/runtime-adoption/schema-samples.v1.json");

    #[test]
    fn census_source_ledger_is_closed_world_and_observation_only() {
        let corpus: RuntimeCensusCorpus = serde_json::from_str(CENSUS_CORPUS).unwrap();
        assert_eq!(corpus.schema_version, RUNTIME_ADOPTION_SCHEMA_VERSION);
        let actual = corpus
            .source_ledger
            .iter()
            .map(|entry| entry.source)
            .collect::<BTreeSet<_>>();
        let expected = [
            RuntimeCensusSource::ServiceBrowserRecords,
            RuntimeCensusSource::RuntimeProfileState,
            RuntimeCensusSource::ProfileOwnerReservations,
            RuntimeCensusSource::NamedSessionSupervisors,
            RuntimeCensusSource::DaemonMetadata,
            RuntimeCensusSource::OperatingSystemProcessIdentity,
            RuntimeCensusSource::ProfileLockAndDevtools,
            RuntimeCensusSource::CdpBrowserAndTargets,
            RuntimeCensusSource::DisplayAllocationsAndVisibleWindowProof,
            RuntimeCensusSource::ViewStreamsRoutePoolGuacamoleAndHandoffs,
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);
        assert_eq!(corpus.source_ledger.len(), expected.len());
        assert!(corpus
            .source_ledger
            .iter()
            .all(|entry| { entry.observation_only && !entry.authority_limit.trim().is_empty() }));
    }

    #[test]
    fn census_fixture_matrix_classifies_every_runtime_exactly_once() {
        let corpus: RuntimeCensusCorpus = serde_json::from_str(CENSUS_CORPUS).unwrap();
        let mut fixture_ids = BTreeSet::new();
        let mut classifications = BTreeSet::new();
        for fixture in corpus.fixtures {
            assert!(fixture_ids.insert(fixture.fixture_id.clone()));
            assert!(
                !fixture.observed_sources.is_empty(),
                "{}",
                fixture.fixture_id
            );
            let decision = classify_runtime(&fixture.evidence);
            assert_eq!(
                decision.classification, fixture.expected_classification,
                "{}",
                fixture.fixture_id
            );
            assert_eq!(
                decision.reason_codes,
                fixture
                    .expected_reason_codes
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                "{}",
                fixture.fixture_id
            );
            classifications.insert(decision.classification);
        }
        assert_eq!(fixture_ids.len(), 13);
        assert_eq!(classifications.len(), 8);
    }

    #[test]
    fn stable_census_joins_all_sources_once_and_persists_the_transaction_digest() {
        let corpus: RuntimeCensusCorpus = serde_json::from_str(CENSUS_CORPUS).unwrap();
        let first = census_round_from_corpus(&corpus, 17);
        let second = census_round_from_corpus(&corpus, 17);
        let census = build_stable_runtime_census(&first, &second).unwrap();

        assert_eq!(census.records.len(), 13);
        assert_eq!(
            census
                .records
                .iter()
                .map(|record| record.logical_browser_id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            13
        );
        assert_eq!(census.digest.len(), 64);
        assert!(!census.activation_allowed);

        let samples: Value = serde_json::from_str(SCHEMA_SAMPLES).unwrap();
        let mut transaction: UpgradeTransaction =
            serde_json::from_value(samples["upgradeTransaction"].clone()).unwrap();
        persist_runtime_census(&mut transaction, &census, "2026-08-15T14:00:00Z");
        assert_eq!(transaction.runtime_census_digest, Some(census.digest));
        assert_eq!(transaction.runtime_migrations.len(), 13);
        assert_eq!(
            transaction.state,
            UpgradeTransactionState::BlockedAmbiguousRuntime
        );
        assert_eq!(
            transaction.stop_reason.as_deref(),
            Some("runtime_census_ambiguous")
        );
    }

    #[test]
    fn census_requires_every_source_and_every_observed_runtime() {
        let corpus: RuntimeCensusCorpus = serde_json::from_str(CENSUS_CORPUS).unwrap();
        let mut snapshots = census_source_snapshots(&corpus);
        snapshots.pop();
        let candidates = census_candidates(&corpus);
        assert_eq!(
            collect_runtime_census_round(17, snapshots, candidates).unwrap_err(),
            "runtime census source set is not closed"
        );

        let snapshots = census_source_snapshots(&corpus);
        let mut candidates = census_candidates(&corpus);
        candidates.pop();
        assert_eq!(
            collect_runtime_census_round(17, snapshots, candidates).unwrap_err(),
            "runtime census candidate set does not match the observed runtime union"
        );

        let snapshots = census_source_snapshots(&corpus);
        let mut candidates = census_candidates(&corpus);
        candidates[0].profile_identity_digest = "/private/profile/path".to_string();
        assert_eq!(
            collect_runtime_census_round(17, snapshots, candidates).unwrap_err(),
            "runtime census candidate set contains duplicates or missing identity"
        );
    }

    #[test]
    fn changed_second_round_classifies_each_runtime_once_and_blocks_activation() {
        let corpus: RuntimeCensusCorpus = serde_json::from_str(CENSUS_CORPUS).unwrap();
        let first = census_round_from_corpus(&corpus, 17);
        let mut second = census_round_from_corpus(&corpus, 18);
        second.candidates[0].evidence.browser_live = false;
        let census = build_stable_runtime_census(&first, &second).unwrap();

        assert!(!census.activation_allowed);
        assert!(census.records.iter().all(|record| {
            record.classification == RuntimeClassification::InsufficientEvidence
                && record
                    .reason_codes
                    .contains(&"census_changed_during_classification".to_string())
        }));
    }

    fn census_round_from_corpus(
        corpus: &RuntimeCensusCorpus,
        registry_revision: u64,
    ) -> RuntimeCensusRound {
        collect_runtime_census_round(
            registry_revision,
            census_source_snapshots(corpus),
            census_candidates(corpus),
        )
        .unwrap()
    }

    fn census_source_snapshots(corpus: &RuntimeCensusCorpus) -> Vec<RuntimeCensusSourceSnapshot> {
        corpus
            .source_ledger
            .iter()
            .map(|entry| RuntimeCensusSourceSnapshot {
                source: entry.source,
                source_revision: "fixture-revision-1".to_string(),
                logical_browser_ids: corpus
                    .fixtures
                    .iter()
                    .filter(|fixture| fixture.observed_sources.contains(&entry.source))
                    .map(|fixture| fixture.fixture_id.clone())
                    .collect(),
            })
            .collect()
    }

    fn census_candidates(corpus: &RuntimeCensusCorpus) -> Vec<RuntimeCensusCandidate> {
        corpus
            .fixtures
            .iter()
            .map(|fixture| RuntimeCensusCandidate {
                logical_browser_id: fixture.fixture_id.clone(),
                profile_identity_digest: format!(
                    "{:x}",
                    Sha256::digest(fixture.fixture_id.as_bytes())
                ),
                observed_sources: fixture.observed_sources.clone(),
                evidence: fixture.evidence.clone(),
            })
            .collect()
    }

    #[test]
    fn current_upgrade_and_orphan_paths_are_intentionally_red() {
        let corpus: UnsafeSeamCorpus = serde_json::from_str(UNSAFE_SEAMS).unwrap();
        assert_eq!(corpus.schema_version, RUNTIME_ADOPTION_SCHEMA_VERSION);
        assert_eq!(corpus.fixtures.len(), 2);
        for fixture in corpus.fixtures {
            assert!(!fixture.source_anchors.is_empty(), "{}", fixture.fixture_id);
            let decision = evaluate_unsafe_seam(&fixture);
            assert!(!decision.passes_required_sequence, "{}", fixture.fixture_id);
            assert_eq!(
                decision.reason.as_deref(),
                Some(fixture.expected_red_reason.as_str())
            );
        }
    }

    #[test]
    fn red_seam_sequences_remain_bound_to_current_source_ordering() {
        let workstation = include_str!("workstation_install.rs");
        assert_source_order(
            workstation,
            &[
                "match materialize_payload(&paths, &parsed)",
                "crate::install::install_remote_view_privileges(true, parsed.json)",
                "reconcile_workstation_locked(&root, &paths)",
            ],
        );

        let handoff = include_str!("native/action_runtime/runtime/navigation.rs");
        assert_source_order(
            handoff,
            &[
                "let descriptor = read_runtime_handoff(&state.session_id)?;",
                "BrowserManager::connect_cdp_for_handoff(",
            ],
        );
        assert_source_order(
            handoff,
            &[
                "let path = write_runtime_handoff(&descriptor)?;",
                "manager.relinquish_browser_for_handoff();",
                "state.browser = None;",
            ],
        );
    }

    #[test]
    fn frozen_schema_samples_round_trip_without_private_runtime_evidence() {
        let samples: Value = serde_json::from_str(SCHEMA_SAMPLES).unwrap();
        let generation: RuntimeGeneration =
            serde_json::from_value(samples["runtimeGeneration"].clone()).unwrap();
        let transaction: UpgradeTransaction =
            serde_json::from_value(samples["upgradeTransaction"].clone()).unwrap();
        let adoption: BrowserAdoptionReceipt =
            serde_json::from_value(samples["browserAdoptionReceipt"].clone()).unwrap();
        let presentation: PresentationReceipt =
            serde_json::from_value(samples["presentationReceipt"].clone()).unwrap();

        assert_eq!(generation.schema_version, RUNTIME_ADOPTION_SCHEMA_VERSION);
        assert_eq!(transaction.schema_version, RUNTIME_ADOPTION_SCHEMA_VERSION);
        assert_eq!(adoption.schema_version, RUNTIME_ADOPTION_SCHEMA_VERSION);
        assert_eq!(presentation.schema_version, RUNTIME_ADOPTION_SCHEMA_VERSION);

        let serialized =
            serde_json::to_value((&generation, &transaction, &adoption, &presentation)).unwrap();
        let forbidden = [
            "cdpUrl",
            "cdpCredentials",
            "userDataDir",
            "profilePath",
            "targetUrl",
            "pageContent",
            "cookies",
            "providerSecret",
        ];
        assert_no_forbidden_keys(&serialized, &forbidden);
    }

    fn assert_no_forbidden_keys(value: &Value, forbidden: &[&str]) {
        match value {
            Value::Array(values) => {
                for value in values {
                    assert_no_forbidden_keys(value, forbidden);
                }
            }
            Value::Object(values) => {
                for (key, value) in values {
                    assert!(!forbidden.contains(&key.as_str()), "forbidden key {key}");
                    assert_no_forbidden_keys(value, forbidden);
                }
            }
            _ => {}
        }
    }

    fn assert_source_order(source: &str, needles: &[&str]) {
        let mut offset = 0;
        for needle in needles {
            let relative = source[offset..]
                .find(needle)
                .unwrap_or_else(|| panic!("missing source anchor: {needle}"));
            offset += relative + needle.len();
        }
    }
}
