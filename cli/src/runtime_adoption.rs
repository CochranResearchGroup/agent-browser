//! Generation-aware runtime adoption authority models.
//!
//! Plan 0116 Slice A intentionally keeps these models provider-free and
//! effect-free. They freeze the census decision vocabulary and durable receipt
//! shapes before installer, daemon, dashboard, or browser behavior changes.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeAdmissionDrain {
    pub(crate) schema_version: String,
    pub(crate) transaction_id: String,
    pub(crate) candidate_generation_id: String,
    pub(crate) transaction_revision: u64,
    pub(crate) recorded_at: String,
}

pub(crate) fn runtime_admission_drain_path() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| {
            home.join(".agent-browser/runtime-adoption")
                .join("admission-drain.json")
        })
        .ok_or_else(|| "runtime_admission_home_unavailable".to_string())
}

/// Rejects ordinary browser effects while an upgrade owns the admission
/// drain. Exact handoff lifecycle commands remain available so the installer
/// can transfer or reverse ownership without reopening general admission.
pub(crate) fn require_runtime_admission(drain_path: &Path, action: &str) -> Result<(), String> {
    if runtime_admission_action_allowed(action) || !drain_path.exists() {
        return Ok(());
    }
    let payload = fs::read(drain_path).map_err(|error| {
        format!(
            "runtime_admission_drain_unreadable: cannot read {}: {error}",
            drain_path.display()
        )
    })?;
    let drain: RuntimeAdmissionDrain = serde_json::from_slice(&payload).map_err(|error| {
        format!(
            "runtime_admission_drain_invalid: cannot parse {}: {error}",
            drain_path.display()
        )
    })?;
    Err(format!(
        "runtime_admission_draining: transaction '{}' is transferring runtime ownership at revision {}",
        drain.transaction_id, drain.transaction_revision
    ))
}

fn runtime_admission_action_allowed(action: &str) -> bool {
    matches!(
        action,
        "runtime_handoff_abort"
            | "runtime_handoff_finalize"
            | "runtime_handoff_prepare"
            | "runtime_handoff_resume"
            | "runtime_handoff_rollback"
    ) || !crate::runtime_owner_transfer::action_requires_owner_effect_authority(action)
}

/// Advances one durable upgrade transaction through the frozen state machine.
/// The caller must persist the transaction after every successful transition.
pub(crate) fn transition_upgrade_transaction(
    transaction: &mut UpgradeTransaction,
    expected_revision: u64,
    next_state: UpgradeTransactionState,
    checkpoint_name: &str,
    recorded_at: &str,
) -> Result<(), String> {
    if transaction.revision != expected_revision {
        return Err("upgrade_transaction_revision_mismatch".to_string());
    }
    if checkpoint_name.trim().is_empty() || recorded_at.trim().is_empty() {
        return Err("upgrade_transaction_checkpoint_invalid".to_string());
    }
    if !upgrade_transition_allowed(transaction.state, next_state) {
        return Err(format!(
            "upgrade_transaction_transition_invalid:{:?}->{next_state:?}",
            transaction.state
        ));
    }
    if next_state == UpgradeTransactionState::CandidateReady
        && !upgrade_runtime_preservation_proven(transaction)
    {
        return Err("upgrade_runtime_preservation_unproven".to_string());
    }

    transaction.revision = transaction
        .revision
        .checked_add(1)
        .ok_or_else(|| "upgrade_transaction_revision_exhausted".to_string())?;
    transaction.state = next_state;
    transaction.checkpoints.push(UpgradeCheckpoint {
        name: checkpoint_name.to_string(),
        transaction_revision: transaction.revision,
        recorded_at: recorded_at.to_string(),
    });
    Ok(())
}

pub(crate) fn upgrade_runtime_preservation_proven(transaction: &UpgradeTransaction) -> bool {
    transaction.runtime_census_digest.is_some()
        && transaction
            .runtime_migrations
            .iter()
            .all(|migration| match migration.disposition {
                RuntimeDisposition::CooperativeTransfer | RuntimeDisposition::OrphanAdoption => {
                    migration
                        .adoption_receipt_id
                        .as_deref()
                        .is_some_and(|receipt| !receipt.trim().is_empty())
                }
                RuntimeDisposition::ManualPreservation | RuntimeDisposition::RetiredIdle => {
                    !migration.reason_codes.is_empty()
                }
                RuntimeDisposition::RejectedAmbiguity => false,
            })
}

fn upgrade_transition_allowed(
    current: UpgradeTransactionState,
    next: UpgradeTransactionState,
) -> bool {
    use UpgradeTransactionState::*;
    matches!(
        (current, next),
        (Planned, CandidateStaged)
            | (CandidateStaged, CandidatePreflightReady)
            | (CandidatePreflightReady, CensusStable)
            | (CensusStable, AdmissionDraining)
            | (AdmissionDraining, RuntimesTransferring)
            | (RuntimesTransferring, PresentationsRebinding)
            | (PresentationsRebinding, CandidateReady)
            | (CandidateReady, GenerationCommitted)
            | (GenerationCommitted, PostCommitValidating)
            | (PostCommitValidating, Accepted)
            | (Accepted, OldGenerationRetirable)
            | (
                Planned
                    | CandidateStaged
                    | CandidatePreflightReady
                    | CensusStable
                    | AdmissionDraining
                    | RuntimesTransferring
                    | PresentationsRebinding
                    | CandidateReady,
                RollbackBeforeCommit
            )
            | (
                GenerationCommitted | PostCommitValidating,
                RollbackAfterCommit
            )
            | (RollbackBeforeCommit, FailedPreservedOldGeneration)
            | (RollbackBeforeCommit, OperatorRecoveryRequired)
            | (RollbackAfterCommit, FailedPreservedOldGeneration)
            | (RollbackAfterCommit, OperatorRecoveryRequired)
            | (Accepted, OperatorRecoveryRequired)
            | (
                Planned | CandidatePreflightReady,
                BlockedCandidateIncompatible
            )
            | (Planned | CensusStable, BlockedInflightEffect)
            | (Planned | CandidatePreflightReady, BlockedAmbiguousRuntime)
    )
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceAgreement {
    Match,
    Mismatch,
    Missing,
    #[default]
    NotApplicable,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

/// One source-specific, observation-only view of a possible runtime.
///
/// Aliases are opaque join keys such as logical browser, runtime profile,
/// session, process-instance, or CDP endpoint digests. Raw profile paths,
/// provider URLs, tokens, and page data must never enter this structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeCensusObservation {
    pub(crate) logical_browser_id_hint: Option<String>,
    pub(crate) aliases: Vec<String>,
    pub(crate) profile_identity_digest: Option<String>,
    pub(crate) evidence: RuntimeEvidenceSummary,
}

/// Stable readback from exactly one frozen census source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeCensusSourceReadback {
    pub(crate) source: RuntimeCensusSource,
    pub(crate) source_revision: String,
    pub(crate) observations: Vec<RuntimeCensusObservation>,
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

/// Join the ten source adapters without treating any one adapter as global
/// runtime authority. Duplicate aliases converge to one candidate. Conflicting
/// profile identities remain one candidate with mismatch evidence, so the
/// later classification blocks activation rather than granting ownership.
pub(crate) fn adapt_runtime_census_readbacks(
    registry_revision: u64,
    readbacks: Vec<RuntimeCensusSourceReadback>,
) -> Result<RuntimeCensusRound, String> {
    let expected_sources = runtime_census_sources()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let actual_sources = readbacks
        .iter()
        .map(|readback| readback.source)
        .collect::<BTreeSet<_>>();
    if readbacks.len() != expected_sources.len() || actual_sources != expected_sources {
        return Err("runtime census source readback set is not closed".to_string());
    }

    let mut flattened = Vec::<(RuntimeCensusSource, RuntimeCensusObservation)>::new();
    let mut source_revisions = BTreeMap::new();
    for readback in readbacks {
        if readback.source_revision.trim().is_empty() {
            return Err("runtime census source revision is missing".to_string());
        }
        source_revisions.insert(readback.source, readback.source_revision);
        for mut observation in readback.observations {
            observation.aliases.sort();
            observation.aliases.dedup();
            if observation.aliases.is_empty()
                || observation
                    .aliases
                    .iter()
                    .any(|alias| alias.trim().is_empty())
                || observation
                    .profile_identity_digest
                    .as_deref()
                    .is_some_and(|digest| !is_sha256(digest))
                || observation
                    .logical_browser_id_hint
                    .as_deref()
                    .is_some_and(|hint| hint.trim().is_empty())
            {
                return Err("runtime census observation contains invalid join evidence".to_string());
            }
            flattened.push((readback.source, observation));
        }
    }

    let mut parent = (0..flattened.len()).collect::<Vec<_>>();
    let mut aliases = BTreeMap::<String, usize>::new();
    for (index, (_, observation)) in flattened.iter().enumerate() {
        let mut join_keys = observation.aliases.clone();
        if let Some(hint) = observation.logical_browser_id_hint.as_deref() {
            join_keys.push(format!("logical:{hint}"));
        }
        for alias in join_keys {
            if let Some(other) = aliases.insert(alias, index) {
                union_observations(&mut parent, index, other);
            }
        }
    }

    let mut grouped = BTreeMap::<usize, Vec<usize>>::new();
    for index in 0..flattened.len() {
        let root = find_observation_root(&mut parent, index);
        grouped.entry(root).or_default().push(index);
    }

    let mut candidates = Vec::with_capacity(grouped.len());
    let mut sources_by_runtime = BTreeMap::<String, BTreeSet<RuntimeCensusSource>>::new();
    for indexes in grouped.values() {
        let hints = indexes
            .iter()
            .filter_map(|index| flattened[*index].1.logical_browser_id_hint.clone())
            .collect::<BTreeSet<_>>();
        let group_aliases = indexes
            .iter()
            .flat_map(|index| flattened[*index].1.aliases.iter().cloned())
            .collect::<BTreeSet<_>>();
        let logical_browser_id = hints.iter().next().cloned().unwrap_or_else(|| {
            let payload = group_aliases.iter().cloned().collect::<Vec<_>>().join("\n");
            format!("observed-{}", &sha256_text(&payload)[..16])
        });
        let observed_sources = indexes
            .iter()
            .map(|index| flattened[*index].0)
            .collect::<BTreeSet<_>>();
        let profile_digests = indexes
            .iter()
            .filter_map(|index| flattened[*index].1.profile_identity_digest.clone())
            .collect::<BTreeSet<_>>();
        let profile_identity_digest = match profile_digests.len() {
            1 => profile_digests.iter().next().cloned().expect("one digest"),
            0 => sha256_text(&format!("unidentified-profile:{logical_browser_id}")),
            _ => sha256_text(
                &profile_digests
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        };
        let mut evidence =
            merge_evidence(indexes.iter().map(|index| &flattened[*index].1.evidence));
        if profile_digests.len() > 1 || hints.len() > 1 {
            evidence.profile_identity = EvidenceAgreement::Mismatch;
        } else if profile_digests.is_empty() && evidence.browser_live {
            evidence.profile_identity = EvidenceAgreement::Missing;
        }
        sources_by_runtime.insert(logical_browser_id.clone(), observed_sources.clone());
        candidates.push(RuntimeCensusCandidate {
            logical_browser_id,
            profile_identity_digest,
            observed_sources: observed_sources.into_iter().collect(),
            evidence,
        });
    }

    let source_snapshots = runtime_census_sources()
        .into_iter()
        .map(|source| RuntimeCensusSourceSnapshot {
            source,
            source_revision: source_revisions
                .remove(&source)
                .expect("closed source set was checked"),
            logical_browser_ids: sources_by_runtime
                .iter()
                .filter_map(|(runtime_id, sources)| {
                    sources.contains(&source).then_some(runtime_id.clone())
                })
                .collect(),
        })
        .collect();
    collect_runtime_census_round(registry_revision, source_snapshots, candidates)
}

fn find_observation_root(parent: &mut [usize], index: usize) -> usize {
    if parent[index] != index {
        parent[index] = find_observation_root(parent, parent[index]);
    }
    parent[index]
}

fn union_observations(parent: &mut [usize], left: usize, right: usize) {
    let left_root = find_observation_root(parent, left);
    let right_root = find_observation_root(parent, right);
    if left_root != right_root {
        parent[right_root] = left_root;
    }
}

fn merge_evidence<'a>(
    evidence: impl Iterator<Item = &'a RuntimeEvidenceSummary>,
) -> RuntimeEvidenceSummary {
    let mut merged = RuntimeEvidenceSummary {
        observation_rounds_agree: true,
        registry_revision_stable: true,
        ..RuntimeEvidenceSummary::default()
    };
    let mut live_daemon_observations = 0usize;
    let mut cooperative_daemon_observations = 0usize;
    for item in evidence {
        merged.browser_live |= item.browser_live;
        merged.daemon_live |= item.daemon_live;
        if item.daemon_live {
            live_daemon_observations += 1;
            cooperative_daemon_observations += usize::from(item.daemon_cooperative);
        }
        merged.manual_browser |= item.manual_browser;
        merged.externally_owned |= item.externally_owned;
        merged.metadata_present |= item.metadata_present;
        merged.observation_rounds_agree &= item.observation_rounds_agree;
        merged.registry_revision_stable &= item.registry_revision_stable;
        merged.owner_generations.extend(&item.owner_generations);
        merged.process_identity = merge_agreement(merged.process_identity, item.process_identity);
        merged.profile_identity = merge_agreement(merged.profile_identity, item.profile_identity);
        merged.browser_family = merge_agreement(merged.browser_family, item.browser_family);
        merged.cdp_endpoint = merge_agreement(merged.cdp_endpoint, item.cdp_endpoint);
        merged.target_set = merge_agreement(merged.target_set, item.target_set);
    }
    merged.owner_generations.sort_unstable();
    merged.owner_generations.dedup();
    merged.daemon_cooperative =
        live_daemon_observations > 0 && live_daemon_observations == cooperative_daemon_observations;
    merged
}

fn merge_agreement(left: EvidenceAgreement, right: EvidenceAgreement) -> EvidenceAgreement {
    use EvidenceAgreement::{Match, Mismatch, Missing, NotApplicable};
    match (left, right) {
        (Mismatch, _) | (_, Mismatch) => Mismatch,
        (Missing, _) | (_, Missing) => Missing,
        (Match, _) | (_, Match) => Match,
        (NotApplicable, NotApplicable) => NotApplicable,
    }
}

fn sha256_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

/// Collect one read-only host census round from the ten frozen Plan 0116
/// sources. The adapters retain only opaque join keys and digests. A caller
/// must collect a second round and pass both to `build_stable_runtime_census`
/// before admission drain or payload mutation.
pub(crate) fn collect_host_runtime_census_round() -> Result<RuntimeCensusRound, String> {
    use crate::native::service_store::{JsonServiceStateStore, ServiceStateStore};

    let service_path = JsonServiceStateStore::default_path()?;
    let service_state = JsonServiceStateStore::new(service_path).load()?;
    let runtime_profiles = crate::runtime_profile::list_runtime_profiles(&[], None)?;
    let supervisor_health = crate::session_supervisor::session_supervisor_health_json();
    let daemon_inventory = crate::install::active_runtime_inventory(None);

    let service_revision = source_revision(&service_state)?;
    let registry_revision = u64::from_str_radix(&service_revision[..16], 16)
        .map_err(|error| format!("invalid service registry revision: {error}"))?;
    let readbacks = vec![
        service_browser_readback(&service_state, &service_revision)?,
        runtime_profile_readback(&runtime_profiles)?,
        profile_owner_readback(&service_state, &service_revision)?,
        value_readback(
            RuntimeCensusSource::NamedSessionSupervisors,
            &supervisor_health,
            supervisor_observations(&supervisor_health),
        )?,
        value_readback(
            RuntimeCensusSource::DaemonMetadata,
            &daemon_inventory,
            daemon_observations(&daemon_inventory),
        )?,
        process_identity_readback(
            &service_state,
            &runtime_profiles,
            &supervisor_health,
            &daemon_inventory,
        )?,
        profile_lock_readback(&runtime_profiles)?,
        cdp_target_readback(&runtime_profiles)?,
        display_readback(&service_state, &service_revision),
        presentation_readback(&service_state, &service_revision),
    ];
    adapt_runtime_census_readbacks(registry_revision, readbacks)
}

fn service_browser_readback(
    state: &crate::native::service_model::ServiceState,
    service_revision: &str,
) -> Result<RuntimeCensusSourceReadback, String> {
    use crate::native::service_model::{BrowserHealth, BrowserHost};

    let observations = state
        .browsers
        .values()
        .map(|browser| -> Result<_, String> {
            let process = state.browser_process_identities.get(&browser.id);
            let profile = browser
                .profile_id
                .as_deref()
                .and_then(|profile_id| state.profiles.get(profile_id));
            let profile_path = process
                .and_then(|identity| identity.user_data_dir.as_deref())
                .or_else(|| profile.and_then(|profile| profile.user_data_dir.as_deref()));
            let profile_digest = profile_path.map(canonical_profile_digest).transpose()?;
            let mut aliases = vec![format!("browser:{}", browser.id)];
            aliases.extend(
                browser
                    .active_session_ids
                    .iter()
                    .map(|session| format!("session:{session}")),
            );
            if let Some(pid) = browser.pid {
                aliases.push(format!("pid:{pid}"));
            }
            if let Some(profile_id) = browser.profile_id.as_deref() {
                aliases.push(format!("service-profile:{profile_id}"));
            }
            if let Some(digest) = profile_digest.as_deref() {
                aliases.push(format!("profile-digest:{digest}"));
            }
            if let Some(runtime_profile) =
                process.and_then(|identity| identity.runtime_profile.as_deref())
            {
                aliases.push(format!("runtime-profile:{runtime_profile}"));
            }
            if let Some(endpoint) = browser.cdp_endpoint.as_deref() {
                aliases.push(format!("cdp-endpoint:{}", sha256_text(endpoint)));
            }
            let family_known = process
                .and_then(|identity| identity.process_identity.browser_family.as_deref())
                .is_some()
                || profile.and_then(|profile| profile.browser_build).is_some();
            let browser_live = matches!(
                browser.health,
                BrowserHealth::Ready
                    | BrowserHealth::Degraded
                    | BrowserHealth::Unreachable
                    | BrowserHealth::CdpDisconnected
                    | BrowserHealth::Reconnecting
                    | BrowserHealth::Closing
            );
            let mut evidence = base_fragment();
            evidence.browser_live = browser_live;
            evidence.manual_browser = browser_live
                && browser.cdp_endpoint.is_none()
                && matches!(
                    browser.host,
                    BrowserHost::LocalHeaded | BrowserHost::AttachedExisting
                );
            evidence.externally_owned = matches!(
                browser.host,
                BrowserHost::AttachedExisting | BrowserHost::CloudProvider
            );
            evidence.metadata_present = true;
            evidence.profile_identity = profile_digest
                .as_ref()
                .map_or(EvidenceAgreement::Missing, |_| EvidenceAgreement::Match);
            evidence.browser_family = if family_known {
                EvidenceAgreement::Match
            } else if browser_live && !evidence.manual_browser {
                EvidenceAgreement::Missing
            } else {
                EvidenceAgreement::NotApplicable
            };
            Ok(RuntimeCensusObservation {
                logical_browser_id_hint: Some(browser.id.clone()),
                aliases,
                profile_identity_digest: profile_digest,
                evidence,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(RuntimeCensusSourceReadback {
        source: RuntimeCensusSource::ServiceBrowserRecords,
        source_revision: service_revision.to_string(),
        observations,
    })
}

fn runtime_profile_readback(
    profiles: &[crate::runtime_profile::RuntimeProfileSummary],
) -> Result<RuntimeCensusSourceReadback, String> {
    let observations = profiles
        .iter()
        .map(|profile| -> Result<_, String> {
            let profile_digest = canonical_profile_digest(&profile.user_data_dir)?;
            let mut aliases = vec![
                format!("runtime-profile:{}", profile.runtime_profile),
                format!("profile-digest:{profile_digest}"),
            ];
            if let Some(pid) = profile.browser_pid {
                aliases.push(format!("pid:{pid}"));
            }
            if let Some(port) = profile.devtools_port {
                aliases.push(format!("cdp-port:{port}"));
            }
            let mut evidence = base_fragment();
            evidence.browser_live = profile.browser_alive;
            evidence.manual_browser = profile.browser_alive && profile.devtools_port.is_none();
            evidence.metadata_present = profile.configured
                || profile.browser_pid.is_some()
                || profile.launch_record.is_some();
            evidence.profile_identity = EvidenceAgreement::Match;
            evidence.browser_family = if profile
                .launch_record
                .as_ref()
                .and_then(|record| record.browser_family.as_ref())
                .is_some()
            {
                EvidenceAgreement::Match
            } else if profile.browser_alive && !evidence.manual_browser {
                EvidenceAgreement::Missing
            } else {
                EvidenceAgreement::NotApplicable
            };
            Ok(RuntimeCensusObservation {
                logical_browser_id_hint: None,
                aliases,
                profile_identity_digest: Some(profile_digest),
                evidence,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(RuntimeCensusSourceReadback {
        source: RuntimeCensusSource::RuntimeProfileState,
        source_revision: source_revision(&profiles)?,
        observations,
    })
}

fn profile_owner_readback(
    state: &crate::native::service_model::ServiceState,
    service_revision: &str,
) -> Result<RuntimeCensusSourceReadback, String> {
    let mut observations = state
        .runtime_owner_registry
        .owners
        .values()
        .map(|owner| {
            let mut evidence = base_fragment();
            evidence.metadata_present = true;
            evidence.profile_identity = EvidenceAgreement::Match;
            evidence.owner_generations.push(owner.owner_generation);
            RuntimeCensusObservation {
                logical_browser_id_hint: Some(owner.browser_id.clone()),
                aliases: vec![
                    format!("browser:{}", owner.browser_id),
                    format!("session:{}", owner.daemon_session_route),
                    format!("profile-digest:{}", owner.profile_identity_digest),
                ],
                profile_identity_digest: Some(owner.profile_identity_digest.clone()),
                evidence,
            }
        })
        .collect::<Vec<_>>();
    let authoritative_profiles = state
        .runtime_owner_registry
        .owners
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let legacy_observations = state
        .sessions
        .values()
        .filter(|session| session.profile_id.is_some() || !session.browser_ids.is_empty())
        .filter_map(|session| {
            let profile_path = session.profile_id.as_deref().and_then(|profile_id| {
                state
                    .profiles
                    .get(profile_id)
                    .and_then(|profile| profile.user_data_dir.as_deref())
            });
            let profile_digest = match profile_path.map(canonical_profile_digest).transpose() {
                Ok(digest) => digest,
                Err(error) => return Some(Err(error)),
            };
            if profile_digest
                .as_ref()
                .is_some_and(|digest| authoritative_profiles.contains(digest))
            {
                return None;
            }
            let mut aliases = vec![format!("session:{}", session.id)];
            aliases.extend(
                session
                    .browser_ids
                    .iter()
                    .map(|browser_id| format!("browser:{browser_id}")),
            );
            if let Some(profile_id) = session.profile_id.as_deref() {
                aliases.push(format!("service-profile:{profile_id}"));
            }
            if let Some(digest) = profile_digest.as_deref() {
                aliases.push(format!("profile-digest:{digest}"));
            }
            let mut evidence = base_fragment();
            evidence.metadata_present = true;
            evidence.profile_identity = profile_digest
                .as_ref()
                .map_or(EvidenceAgreement::Missing, |_| EvidenceAgreement::Match);
            Some(Ok(RuntimeCensusObservation {
                logical_browser_id_hint: (session.browser_ids.len() == 1)
                    .then(|| session.browser_ids[0].clone()),
                aliases,
                profile_identity_digest: profile_digest,
                evidence,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    observations.extend(legacy_observations);
    let source_revision = sha256_text(&format!(
        "{service_revision}:runtime-owner-registry:{}",
        state.runtime_owner_registry.revision
    ));
    Ok(RuntimeCensusSourceReadback {
        source: RuntimeCensusSource::ProfileOwnerReservations,
        source_revision,
        observations,
    })
}

fn supervisor_observations(value: &serde_json::Value) -> Vec<RuntimeCensusObservation> {
    value
        .get("sessions")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|session| {
            let name = session.get("session")?.as_str()?;
            let mut aliases = vec![format!("session:{name}")];
            if let Some(pid) = session.get("mainPid").and_then(serde_json::Value::as_u64) {
                aliases.push(format!("pid:{pid}"));
            }
            let runtime_profile = session
                .get("manifest")
                .and_then(|manifest| manifest.get("runtimeProfile"))
                .and_then(serde_json::Value::as_str);
            if let Some(profile) = runtime_profile {
                aliases.push(format!("runtime-profile:{profile}"));
            }
            let ready = session
                .get("ready")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let mut evidence = base_fragment();
            evidence.daemon_live = session
                .get("mainPid")
                .and_then(serde_json::Value::as_u64)
                .is_some();
            evidence.daemon_cooperative = ready;
            evidence.metadata_present = true;
            Some(RuntimeCensusObservation {
                logical_browser_id_hint: None,
                aliases,
                profile_identity_digest: None,
                evidence,
            })
        })
        .collect()
}

fn daemon_observations(value: &serde_json::Value) -> Vec<RuntimeCensusObservation> {
    value
        .get("runtimes")
        .or_else(|| value.get("sessions"))
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|runtime| {
            let session = runtime.get("session")?.as_str()?;
            let mut aliases = vec![format!("session:{session}")];
            if let Some(pid) = runtime.get("pid").and_then(serde_json::Value::as_u64) {
                aliases.push(format!("pid:{pid}"));
            }
            let live = runtime
                .get("pidRunning")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let mut evidence = base_fragment();
            evidence.daemon_live = live;
            evidence.daemon_cooperative = live
                && runtime.get("state").and_then(serde_json::Value::as_str) == Some("converged");
            evidence.metadata_present = true;
            Some(RuntimeCensusObservation {
                logical_browser_id_hint: None,
                aliases,
                profile_identity_digest: None,
                evidence,
            })
        })
        .collect()
}

#[derive(Default)]
struct ProcessCensusSeed {
    aliases: BTreeSet<String>,
    profile_digests: BTreeSet<String>,
    expected: Vec<crate::process_identity::RecordedProcessIdentity>,
    browser_pid: bool,
}

fn process_identity_readback(
    state: &crate::native::service_model::ServiceState,
    profiles: &[crate::runtime_profile::RuntimeProfileSummary],
    supervisors: &serde_json::Value,
    daemons: &serde_json::Value,
) -> Result<RuntimeCensusSourceReadback, String> {
    let mut seeds = BTreeMap::<u32, ProcessCensusSeed>::new();
    for browser in state.browsers.values() {
        let Some(pid) = browser.pid else {
            continue;
        };
        let seed = seeds.entry(pid).or_default();
        seed.browser_pid = true;
        seed.aliases.insert(format!("pid:{pid}"));
        seed.aliases.insert(format!("browser:{}", browser.id));
        seed.aliases.extend(
            browser
                .active_session_ids
                .iter()
                .map(|session| format!("session:{session}")),
        );
        if let Some(identity) = state.browser_process_identities.get(&browser.id) {
            seed.expected.push(identity.process_identity.clone());
            if let Some(runtime_profile) = identity.runtime_profile.as_deref() {
                seed.aliases
                    .insert(format!("runtime-profile:{runtime_profile}"));
            }
            if let Some(user_data_dir) = identity.user_data_dir.as_deref() {
                let digest = canonical_profile_digest(user_data_dir)?;
                seed.aliases.insert(format!("profile-digest:{digest}"));
                seed.profile_digests.insert(digest);
            }
        }
    }
    for profile in profiles {
        let Some(pid) = profile.browser_pid else {
            continue;
        };
        let seed = seeds.entry(pid).or_default();
        seed.browser_pid = true;
        seed.aliases.insert(format!("pid:{pid}"));
        seed.aliases
            .insert(format!("runtime-profile:{}", profile.runtime_profile));
        let digest = canonical_profile_digest(&profile.user_data_dir)?;
        seed.aliases.insert(format!("profile-digest:{digest}"));
        seed.profile_digests.insert(digest);
        if let Some(identity) =
            crate::runtime_profile::read_runtime_state(&profile.runtime_profile)?
                .and_then(|runtime| runtime.process_identity)
        {
            seed.expected.push(identity);
        }
    }
    add_value_process_aliases(supervisors.get("sessions"), &mut seeds);
    add_value_process_aliases(daemons.get("runtimes"), &mut seeds);

    let mut observations = Vec::with_capacity(seeds.len());
    for (pid, seed) in seeds {
        let observation = crate::process_identity::observe_process(pid);
        let mut evidence = base_fragment();
        evidence.metadata_present = true;
        let mut profile_identity_digest = match seed.profile_digests.len() {
            1 => seed.profile_digests.iter().next().cloned(),
            0 => None,
            _ => Some(sha256_text(
                &seed
                    .profile_digests
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n"),
            )),
        };
        if seed.profile_digests.len() > 1 {
            evidence.profile_identity = EvidenceAgreement::Mismatch;
        } else if profile_identity_digest.is_some() {
            evidence.profile_identity = EvidenceAgreement::Match;
        }
        if seed.browser_pid {
            match &observation {
                crate::process_identity::ProcessObservation::Observed(observed) => {
                    evidence.browser_live = true;
                    evidence.browser_family = if observed.browser_family.is_some() {
                        EvidenceAgreement::Match
                    } else {
                        EvidenceAgreement::Missing
                    };
                    evidence.process_identity = if seed.expected.is_empty() {
                        EvidenceAgreement::Missing
                    } else if seed.expected.iter().all(|expected| {
                        crate::process_identity::assess_process_ownership(
                            Some(expected),
                            observation.clone(),
                            crate::process_identity::LegacyProfileProof::Unproven,
                        )
                        .ownership
                            == crate::process_identity::RuntimeProcessOwnership::MatchingBrowser
                    }) {
                        EvidenceAgreement::Match
                    } else {
                        EvidenceAgreement::Mismatch
                    };
                }
                crate::process_identity::ProcessObservation::Missing
                | crate::process_identity::ProcessObservation::Failed { .. } => {
                    evidence.process_identity = EvidenceAgreement::Missing;
                    evidence.browser_family = EvidenceAgreement::Missing;
                }
            }
        }
        let logical_browser_id_hint = seed.aliases.iter().find_map(|alias| {
            alias
                .strip_prefix("browser:")
                .map(std::string::ToString::to_string)
        });
        if profile_identity_digest.is_none() && evidence.browser_live {
            evidence.profile_identity = EvidenceAgreement::Missing;
        }
        observations.push(RuntimeCensusObservation {
            logical_browser_id_hint,
            aliases: seed.aliases.into_iter().collect(),
            profile_identity_digest: profile_identity_digest.take(),
            evidence,
        });
    }
    Ok(RuntimeCensusSourceReadback {
        source: RuntimeCensusSource::OperatingSystemProcessIdentity,
        source_revision: source_revision(&observations)?,
        observations,
    })
}

fn add_value_process_aliases(
    rows: Option<&serde_json::Value>,
    seeds: &mut BTreeMap<u32, ProcessCensusSeed>,
) {
    for row in rows
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let pid = row
            .get("pid")
            .or_else(|| row.get("mainPid"))
            .and_then(serde_json::Value::as_u64)
            .and_then(|pid| u32::try_from(pid).ok());
        let Some(pid) = pid else {
            continue;
        };
        let seed = seeds.entry(pid).or_default();
        seed.aliases.insert(format!("pid:{pid}"));
        if let Some(session) = row.get("session").and_then(serde_json::Value::as_str) {
            seed.aliases.insert(format!("session:{session}"));
        }
    }
}

fn profile_lock_readback(
    profiles: &[crate::runtime_profile::RuntimeProfileSummary],
) -> Result<RuntimeCensusSourceReadback, String> {
    let observations = profiles
        .iter()
        .map(|profile| -> Result<_, String> {
            let digest = canonical_profile_digest(&profile.user_data_dir)?;
            let mut aliases = vec![
                format!("runtime-profile:{}", profile.runtime_profile),
                format!("profile-digest:{digest}"),
            ];
            if let Some(pid) = profile.browser_pid {
                aliases.push(format!("pid:{pid}"));
            }
            if let Some(port) = profile.devtools_port {
                aliases.push(format!("cdp-port:{port}"));
            }
            let mut evidence = base_fragment();
            evidence.metadata_present =
                profile.browser_pid.is_some() || profile.devtools_port.is_some();
            evidence.profile_identity = EvidenceAgreement::Match;
            evidence.manual_browser = profile.browser_alive && profile.devtools_port.is_none();
            evidence.cdp_endpoint = if profile.devtools_reachable {
                EvidenceAgreement::Match
            } else if profile.browser_alive && !evidence.manual_browser {
                EvidenceAgreement::Missing
            } else {
                EvidenceAgreement::NotApplicable
            };
            Ok(RuntimeCensusObservation {
                logical_browser_id_hint: None,
                aliases,
                profile_identity_digest: Some(digest),
                evidence,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(RuntimeCensusSourceReadback {
        source: RuntimeCensusSource::ProfileLockAndDevtools,
        source_revision: source_revision(&observations)?,
        observations,
    })
}

fn cdp_target_readback(
    profiles: &[crate::runtime_profile::RuntimeProfileSummary],
) -> Result<RuntimeCensusSourceReadback, String> {
    let mut observations = Vec::with_capacity(profiles.len());
    for profile in profiles {
        let status = crate::runtime_profile::runtime_status_with_user_data_dir(
            &profile.runtime_profile,
            Some(std::path::Path::new(&profile.user_data_dir)),
        )?;
        let digest = canonical_profile_digest(&profile.user_data_dir)?;
        let mut aliases = vec![
            format!("runtime-profile:{}", profile.runtime_profile),
            format!("profile-digest:{digest}"),
        ];
        if let Some(pid) = profile.browser_pid {
            aliases.push(format!("pid:{pid}"));
        }
        if let Some(port) = status.devtools_port {
            aliases.push(format!("cdp-port:{port}"));
        }
        let mut evidence = base_fragment();
        evidence.metadata_present = status.devtools_port.is_some();
        if let Some(port) = status.devtools_port.filter(|_| status.devtools_reachable) {
            match crate::runtime_profile::bounded_cdp_identity_and_target_digest(port) {
                Ok((browser_digest, target_digest)) => {
                    aliases.push(format!("cdp-browser:{browser_digest}"));
                    aliases.push(format!("target-set:{target_digest}"));
                    evidence.cdp_endpoint = EvidenceAgreement::Match;
                    evidence.target_set = EvidenceAgreement::Match;
                }
                Err(_) => {
                    evidence.cdp_endpoint = EvidenceAgreement::Missing;
                    evidence.target_set = EvidenceAgreement::Missing;
                }
            }
        } else if status.browser_alive && status.devtools_port.is_some() {
            evidence.cdp_endpoint = EvidenceAgreement::Missing;
            evidence.target_set = EvidenceAgreement::Missing;
        }
        observations.push(RuntimeCensusObservation {
            logical_browser_id_hint: None,
            aliases,
            profile_identity_digest: Some(digest),
            evidence,
        });
    }
    Ok(RuntimeCensusSourceReadback {
        source: RuntimeCensusSource::CdpBrowserAndTargets,
        source_revision: source_revision(&observations)?,
        observations,
    })
}

fn display_readback(
    state: &crate::native::service_model::ServiceState,
    service_revision: &str,
) -> RuntimeCensusSourceReadback {
    let observations = state
        .display_allocations
        .values()
        .map(|display| {
            let mut aliases = vec![format!("display:{}", display.id)];
            if let Some(browser_id) = display.owner_browser_id.as_deref() {
                aliases.push(format!("browser:{browser_id}"));
            }
            if let Some(session_id) = display.owner_session_id.as_deref() {
                aliases.push(format!("session:{session_id}"));
            }
            if let Some(profile_id) = display.profile_id.as_deref() {
                aliases.push(format!("service-profile:{profile_id}"));
            }
            let mut evidence = base_fragment();
            evidence.metadata_present = true;
            RuntimeCensusObservation {
                logical_browser_id_hint: display.owner_browser_id.clone(),
                aliases,
                profile_identity_digest: None,
                evidence,
            }
        })
        .collect();
    RuntimeCensusSourceReadback {
        source: RuntimeCensusSource::DisplayAllocationsAndVisibleWindowProof,
        source_revision: service_revision.to_string(),
        observations,
    }
}

fn presentation_readback(
    state: &crate::native::service_model::ServiceState,
    service_revision: &str,
) -> RuntimeCensusSourceReadback {
    let mut observations = Vec::new();
    for browser in state
        .browsers
        .values()
        .filter(|browser| !browser.view_streams.is_empty())
    {
        let mut aliases = vec![format!("browser:{}", browser.id)];
        for stream in &browser.view_streams {
            aliases.push(format!("view-stream:{}", stream.id));
            if let Some(route_id) = stream.route_id.as_deref() {
                aliases.push(format!("route:{route_id}"));
            }
            if let Some(display_id) = stream.display_allocation_id.as_deref() {
                aliases.push(format!("display:{display_id}"));
            }
        }
        let mut evidence = base_fragment();
        evidence.metadata_present = true;
        observations.push(RuntimeCensusObservation {
            logical_browser_id_hint: Some(browser.id.clone()),
            aliases,
            profile_identity_digest: None,
            evidence,
        });
    }
    for route in state.remote_view_routes.values() {
        let mut aliases = vec![format!("route:{}", route.id)];
        if let Some(browser_id) = route.browser_id.as_deref() {
            aliases.push(format!("browser:{browser_id}"));
        }
        if let Some(session_id) = route.session_id.as_deref() {
            aliases.push(format!("session:{session_id}"));
        }
        if let Some(display_id) = route.display_allocation_id.as_deref() {
            aliases.push(format!("display:{display_id}"));
        }
        let mut evidence = base_fragment();
        evidence.metadata_present = true;
        observations.push(RuntimeCensusObservation {
            logical_browser_id_hint: route.browser_id.clone(),
            aliases,
            profile_identity_digest: None,
            evidence,
        });
    }
    for handoff in state.remote_view_handoffs.values() {
        let mut aliases = vec![format!("handoff:{}", handoff.id)];
        if let Some(browser_id) = handoff.browser_id.as_deref() {
            aliases.push(format!("browser:{browser_id}"));
        }
        if let Some(session) = handoff.session_name.as_deref() {
            aliases.push(format!("session:{session}"));
        }
        if let Some(profile_id) = handoff.profile_id.as_deref() {
            aliases.push(format!("service-profile:{profile_id}"));
        }
        if let Some(route_id) = handoff.last_route_id.as_deref() {
            aliases.push(format!("route:{route_id}"));
        }
        if let Some(display_id) = handoff.last_display_allocation_id.as_deref() {
            aliases.push(format!("display:{display_id}"));
        }
        let mut evidence = base_fragment();
        evidence.metadata_present = true;
        observations.push(RuntimeCensusObservation {
            logical_browser_id_hint: handoff.browser_id.clone(),
            aliases,
            profile_identity_digest: None,
            evidence,
        });
    }
    for entry in state.route_pool.values() {
        let mut aliases = vec![
            format!("route-pool:{}", entry.id),
            format!("route:{}", entry.route_id),
        ];
        if let Some(browser_id) = entry
            .target
            .get("browserId")
            .and_then(serde_json::Value::as_str)
        {
            aliases.push(format!("browser:{browser_id}"));
        }
        if let Some(session_id) = entry
            .target
            .get("sessionId")
            .and_then(serde_json::Value::as_str)
        {
            aliases.push(format!("session:{session_id}"));
        }
        if let Some(display_id) = entry
            .target
            .get("displayAllocationId")
            .and_then(serde_json::Value::as_str)
        {
            aliases.push(format!("display:{display_id}"));
        }
        let mut evidence = base_fragment();
        evidence.metadata_present = true;
        observations.push(RuntimeCensusObservation {
            logical_browser_id_hint: entry
                .target
                .get("browserId")
                .and_then(serde_json::Value::as_str)
                .map(std::string::ToString::to_string),
            aliases,
            profile_identity_digest: None,
            evidence,
        });
    }
    RuntimeCensusSourceReadback {
        source: RuntimeCensusSource::ViewStreamsRoutePoolGuacamoleAndHandoffs,
        source_revision: service_revision.to_string(),
        observations,
    }
}

fn value_readback(
    source: RuntimeCensusSource,
    value: &serde_json::Value,
    observations: Vec<RuntimeCensusObservation>,
) -> Result<RuntimeCensusSourceReadback, String> {
    Ok(RuntimeCensusSourceReadback {
        source,
        source_revision: source_revision(value)?,
        observations,
    })
}

fn base_fragment() -> RuntimeEvidenceSummary {
    RuntimeEvidenceSummary {
        observation_rounds_agree: true,
        registry_revision_stable: true,
        ..RuntimeEvidenceSummary::default()
    }
}

fn canonical_profile_digest(value: &str) -> Result<String, String> {
    crate::runtime_profile::canonical_profile_identity_digest(std::path::Path::new(value))
}

fn source_revision(value: &(impl Serialize + ?Sized)) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| format!("could not serialize runtime census source: {error}"))
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

pub(crate) fn runtime_census_sources() -> [RuntimeCensusSource; 10] {
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
    if evidence.browser_live
        && evidence.daemon_live
        && evidence.daemon_cooperative
        && unique_owner_generations.len() == 1
    {
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
    fn upgrade_transaction_rejects_commit_until_runtime_preservation_is_receipted() {
        let samples: Value = serde_json::from_str(SCHEMA_SAMPLES).unwrap();
        let mut transaction: UpgradeTransaction =
            serde_json::from_value(samples["upgradeTransaction"].clone()).unwrap();
        transaction.runtime_migrations[0].adoption_receipt_id = None;

        let stale_revision = transaction.revision.saturating_sub(1);
        assert_eq!(
            transition_upgrade_transaction(
                &mut transaction,
                stale_revision,
                UpgradeTransactionState::AdmissionDraining,
                "admission_draining",
                "2026-08-16T15:00:00Z",
            )
            .unwrap_err(),
            "upgrade_transaction_revision_mismatch"
        );

        let revision = transaction.revision;
        transition_upgrade_transaction(
            &mut transaction,
            revision,
            UpgradeTransactionState::AdmissionDraining,
            "admission_draining",
            "2026-08-16T15:00:01Z",
        )
        .unwrap();
        let revision = transaction.revision;
        transition_upgrade_transaction(
            &mut transaction,
            revision,
            UpgradeTransactionState::RuntimesTransferring,
            "runtimes_transferring",
            "2026-08-16T15:00:02Z",
        )
        .unwrap();
        let revision = transaction.revision;
        transition_upgrade_transaction(
            &mut transaction,
            revision,
            UpgradeTransactionState::PresentationsRebinding,
            "presentations_rebinding",
            "2026-08-16T15:00:03Z",
        )
        .unwrap();
        let revision = transaction.revision;
        assert_eq!(
            transition_upgrade_transaction(
                &mut transaction,
                revision,
                UpgradeTransactionState::CandidateReady,
                "candidate_ready",
                "2026-08-16T15:00:04Z",
            )
            .unwrap_err(),
            "upgrade_runtime_preservation_unproven"
        );
        assert_eq!(
            transaction.state,
            UpgradeTransactionState::PresentationsRebinding
        );

        transaction.runtime_migrations[0].adoption_receipt_id =
            Some("adoption-receipt-1".to_string());
        transition_upgrade_transaction(
            &mut transaction,
            revision,
            UpgradeTransactionState::CandidateReady,
            "candidate_ready",
            "2026-08-16T15:00:05Z",
        )
        .unwrap();
        let revision = transaction.revision;
        transition_upgrade_transaction(
            &mut transaction,
            revision,
            UpgradeTransactionState::GenerationCommitted,
            "generation_committed",
            "2026-08-16T15:00:06Z",
        )
        .unwrap();
        assert_eq!(
            transaction.state,
            UpgradeTransactionState::GenerationCommitted
        );
        let revision = transaction.revision;
        transition_upgrade_transaction(
            &mut transaction,
            revision,
            UpgradeTransactionState::PostCommitValidating,
            "post_commit_validating",
            "2026-08-16T15:00:07Z",
        )
        .unwrap();
        let revision = transaction.revision;
        transition_upgrade_transaction(
            &mut transaction,
            revision,
            UpgradeTransactionState::Accepted,
            "accepted",
            "2026-08-16T15:00:08Z",
        )
        .unwrap();
        let revision = transaction.revision;
        transition_upgrade_transaction(
            &mut transaction,
            revision,
            UpgradeTransactionState::OperatorRecoveryRequired,
            "accepted_admission_recovery",
            "2026-08-16T15:00:09Z",
        )
        .unwrap();
        assert_eq!(
            transaction.state,
            UpgradeTransactionState::OperatorRecoveryRequired
        );
    }

    #[test]
    fn admission_drain_blocks_effects_but_keeps_transfer_and_observation_available() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-admission-drain-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("admission-drain.json");
        fs::write(
            &path,
            serde_json::to_vec(&RuntimeAdmissionDrain {
                schema_version: RUNTIME_ADOPTION_SCHEMA_VERSION.to_string(),
                transaction_id: "upgrade-test".to_string(),
                candidate_generation_id: "candidate-test".to_string(),
                transaction_revision: 4,
                recorded_at: "2026-08-16T15:05:00Z".to_string(),
            })
            .unwrap(),
        )
        .unwrap();

        assert!(require_runtime_admission(&path, "navigate")
            .unwrap_err()
            .contains("upgrade-test"));
        require_runtime_admission(&path, "runtime_handoff_prepare").unwrap();
        require_runtime_admission(&path, "runtime_handoff_resume").unwrap();
        require_runtime_admission(&path, "snapshot").unwrap();
        fs::remove_dir_all(root).unwrap();
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

    #[test]
    fn source_adapters_join_one_runtime_across_all_ten_readbacks() {
        let profile_digest = digest_text("profile-a");
        let aliases = vec!["browser:browser-a", "profile:profile-a", "pid:41"];
        let readbacks = runtime_census_sources()
            .into_iter()
            .map(|source| RuntimeCensusSourceReadback {
                source,
                source_revision: format!("{source:?}-revision"),
                observations: vec![RuntimeCensusObservation {
                    logical_browser_id_hint: Some("browser-a".to_string()),
                    aliases: aliases.iter().map(|value| (*value).to_string()).collect(),
                    profile_identity_digest: Some(profile_digest.clone()),
                    evidence: cooperative_fragment(source),
                }],
            })
            .collect();

        let round = adapt_runtime_census_readbacks(23, readbacks).unwrap();
        assert_eq!(round.candidates.len(), 1);
        let candidate = &round.candidates[0];
        assert_eq!(candidate.logical_browser_id, "browser-a");
        assert_eq!(candidate.profile_identity_digest, profile_digest);
        assert_eq!(candidate.observed_sources.len(), 10);
        assert_eq!(
            classify_runtime(&candidate.evidence).classification,
            RuntimeClassification::CooperativeLiveOwner
        );
    }

    #[test]
    fn source_adapters_merge_duplicate_pid_evidence_without_granting_authority() {
        let mut readbacks = empty_source_readbacks();
        readbacks[0].observations = vec![
            RuntimeCensusObservation {
                logical_browser_id_hint: Some("browser-a".to_string()),
                aliases: vec!["pid:41".to_string()],
                profile_identity_digest: Some(digest_text("profile-a")),
                evidence: live_identity_fragment(),
            },
            RuntimeCensusObservation {
                logical_browser_id_hint: Some("browser-b".to_string()),
                aliases: vec!["pid:41".to_string()],
                profile_identity_digest: Some(digest_text("profile-b")),
                evidence: live_identity_fragment(),
            },
        ];

        let round = adapt_runtime_census_readbacks(23, readbacks).unwrap();
        assert_eq!(round.candidates.len(), 1);
        assert_eq!(
            classify_runtime(&round.candidates[0].evidence).classification,
            RuntimeClassification::InsufficientEvidence
        );
        assert_eq!(
            round.candidates[0].evidence.profile_identity,
            EvidenceAgreement::Mismatch
        );
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

    fn empty_source_readbacks() -> Vec<RuntimeCensusSourceReadback> {
        runtime_census_sources()
            .into_iter()
            .map(|source| RuntimeCensusSourceReadback {
                source,
                source_revision: format!("{source:?}-revision"),
                observations: Vec::new(),
            })
            .collect()
    }

    fn cooperative_fragment(source: RuntimeCensusSource) -> RuntimeEvidenceSummary {
        let mut evidence = RuntimeEvidenceSummary::default();
        evidence.observation_rounds_agree = true;
        evidence.registry_revision_stable = true;
        match source {
            RuntimeCensusSource::ServiceBrowserRecords => {
                evidence.browser_live = true;
                evidence.profile_identity = EvidenceAgreement::Match;
                evidence.browser_family = EvidenceAgreement::Match;
            }
            RuntimeCensusSource::NamedSessionSupervisors | RuntimeCensusSource::DaemonMetadata => {
                evidence.daemon_live = true;
                evidence.daemon_cooperative = true;
            }
            RuntimeCensusSource::ProfileOwnerReservations => {
                evidence.owner_generations.push(7);
            }
            RuntimeCensusSource::OperatingSystemProcessIdentity => {
                evidence.process_identity = EvidenceAgreement::Match;
            }
            RuntimeCensusSource::ProfileLockAndDevtools => {
                evidence.cdp_endpoint = EvidenceAgreement::Match;
            }
            RuntimeCensusSource::CdpBrowserAndTargets => {
                evidence.target_set = EvidenceAgreement::Match;
            }
            _ => {}
        }
        evidence
    }

    fn live_identity_fragment() -> RuntimeEvidenceSummary {
        RuntimeEvidenceSummary {
            browser_live: true,
            observation_rounds_agree: true,
            registry_revision_stable: true,
            process_identity: EvidenceAgreement::Match,
            profile_identity: EvidenceAgreement::Match,
            browser_family: EvidenceAgreement::Match,
            cdp_endpoint: EvidenceAgreement::Match,
            target_set: EvidenceAgreement::Match,
            ..RuntimeEvidenceSummary::default()
        }
    }

    fn digest_text(value: &str) -> String {
        format!("{:x}", Sha256::digest(value.as_bytes()))
    }

    #[test]
    fn payload_commit_path_requires_runtime_preservation() {
        let corpus: UnsafeSeamCorpus = serde_json::from_str(UNSAFE_SEAMS).unwrap();
        assert_eq!(corpus.schema_version, RUNTIME_ADOPTION_SCHEMA_VERSION);
        assert_eq!(corpus.fixtures.len(), 1);
        for fixture in corpus.fixtures {
            assert!(!fixture.source_anchors.is_empty(), "{}", fixture.fixture_id);
            let decision = evaluate_unsafe_seam(&fixture);
            assert!(decision.passes_required_sequence, "{}", fixture.fixture_id);
            assert_eq!(decision.reason, None);
        }
    }

    #[test]
    fn transaction_sequences_remain_bound_to_current_source_ordering() {
        let workstation = include_str!("workstation_install.rs");
        assert_source_order(
            workstation,
            &[
                "prepare_payload_transaction(&root, &paths, &parsed, isolated_root)",
                "crate::install::install_remote_view_privileges(true, parsed.json)",
                "activate_prepared_payload_transaction(prepared, &paths, isolated_root)",
                "quiesce_existing_user_units(&paths)",
                "commit_prepared_payload_transaction(&paths, &parsed, prepared)",
                "begin_post_commit_validation(prepared)",
                "reconcile_workstation_locked_for_upgrade(",
                "validate_post_commit_transaction(&root, &paths, prepared)",
                "accept_prepared_payload_transaction(prepared, validation)",
            ],
        );

        let handoff = include_str!("native/action_runtime/runtime/navigation.rs");
        assert_source_order(
            handoff,
            &[
                "let descriptor = read_runtime_handoff(&source_session)?;",
                "BrowserManager::connect_cdp_for_handoff(",
                "commit_candidate_owner(&repository, attachment)",
            ],
        );
        assert_source_order(
            handoff,
            &[
                "begin_owner_transfer(",
                "let path = write_runtime_handoff(&descriptor)?;",
                "\"oldOwnerEffectCapable\": true",
                "handle_runtime_handoff_finalize(",
                "manager.relinquish_browser_for_handoff();",
                "state.browser = None;",
            ],
        );
        assert_source_order(
            handoff,
            &[
                "handle_runtime_handoff_orphan_adoption(",
                "runtime_handoff_process_assessment(&provisional, browser_pid)",
                "BrowserManager::connect_cdp_for_handoff(&cdp_url, None)",
                "BrowserAdoptionMode::OrphanAdoption",
                "write_runtime_handoff(&descriptor)?",
                "commit_candidate_owner(",
                "persist_adopted_logical_browser_health(",
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
