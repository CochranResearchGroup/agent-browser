//! Generation-aware runtime adoption authority models.
//!
//! Plan 0116 Slice A intentionally keeps these models provider-free and
//! effect-free. They freeze the census decision vocabulary and durable receipt
//! shapes before installer, daemon, dashboard, or browser behavior changes.

use serde::{Deserialize, Serialize};

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
