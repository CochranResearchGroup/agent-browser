use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{
    ArtifactClass, BuildProfileConfiguration, CandidateManifest, ExecutableInputClosure,
    SourceTreeState, CANDIDATE_MANIFEST_SCHEMA_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidationReceipt {
    pub locator: String,
    pub binary_sha256: String,
    pub terminal_success: bool,
    pub scope_current: bool,
    pub residue_clear: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromotionEvidence {
    pub source_commit_is_ancestor: bool,
    pub observed_binary_sha256: String,
    pub observed_support_manifest_sha256: String,
    pub required_receipt_locators: Vec<String>,
    pub receipts: Vec<ValidationReceipt>,
    pub development_doctor_ready: bool,
    pub task_residue_clear: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionOutcome {
    Eligible,
    RebuildRequired,
    IntegrityPreconditionFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionReason {
    ArtifactClassNotPromotable,
    BinaryDigestMismatch,
    CandidateManifestInconsistent,
    DevelopmentDoctorNotReady,
    ExecutableInputChanged,
    ProductionProfileMismatch,
    RequiredReceiptInvalid,
    RequiredReceiptMissing,
    SourceCommitNotIntegrated,
    SourceTreeNotClean,
    SupportManifestDigestMismatch,
    TaskResiduePresent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromotionDecision {
    pub outcome: PromotionOutcome,
    pub reasons: Vec<PromotionReason>,
    pub reusable_binary_sha256: Option<String>,
}

pub fn evaluate_promotion(
    manifest: &CandidateManifest,
    current_closure: &ExecutableInputClosure,
    evidence: &PromotionEvidence,
) -> PromotionDecision {
    let mut rebuild = Vec::new();
    let mut integrity = Vec::new();

    if manifest.artifact_class != ArtifactClass::ProductionShaped {
        rebuild.push(PromotionReason::ArtifactClassNotPromotable);
    }
    if manifest.source.state != SourceTreeState::Clean {
        rebuild.push(PromotionReason::SourceTreeNotClean);
    }
    if manifest.cargo_profile != "release"
        || manifest.resolved_build_profile != BuildProfileConfiguration::production_release()
    {
        rebuild.push(PromotionReason::ProductionProfileMismatch);
    }
    if manifest.executable_input_sha256 != current_closure.digest() {
        rebuild.push(PromotionReason::ExecutableInputChanged);
    }

    if manifest.validate_internal().is_err()
        || manifest.schema_version != CANDIDATE_MANIFEST_SCHEMA_VERSION
    {
        integrity.push(PromotionReason::CandidateManifestInconsistent);
    }
    if !evidence.source_commit_is_ancestor {
        integrity.push(PromotionReason::SourceCommitNotIntegrated);
    }
    if evidence.observed_binary_sha256 != manifest.binary_sha256 {
        integrity.push(PromotionReason::BinaryDigestMismatch);
    }
    if evidence.observed_support_manifest_sha256 != manifest.support_manifest_sha256 {
        integrity.push(PromotionReason::SupportManifestDigestMismatch);
    }
    if !evidence.development_doctor_ready {
        integrity.push(PromotionReason::DevelopmentDoctorNotReady);
    }
    if !evidence.task_residue_clear {
        integrity.push(PromotionReason::TaskResiduePresent);
    }

    let receipts = evidence
        .receipts
        .iter()
        .map(|receipt| (receipt.locator.as_str(), receipt))
        .collect::<BTreeMap<_, _>>();
    for required in &evidence.required_receipt_locators {
        if !manifest
            .validation_receipts
            .iter()
            .any(|locator| locator == required)
        {
            integrity.push(PromotionReason::RequiredReceiptMissing);
            continue;
        }
        let Some(receipt) = receipts.get(required.as_str()) else {
            integrity.push(PromotionReason::RequiredReceiptMissing);
            continue;
        };
        if receipt.binary_sha256 != manifest.binary_sha256
            || !receipt.terminal_success
            || !receipt.scope_current
            || !receipt.residue_clear
        {
            integrity.push(PromotionReason::RequiredReceiptInvalid);
        }
    }

    integrity.sort();
    integrity.dedup();
    rebuild.sort();
    rebuild.dedup();
    if !integrity.is_empty() {
        return PromotionDecision {
            outcome: PromotionOutcome::IntegrityPreconditionFailed,
            reasons: integrity,
            reusable_binary_sha256: None,
        };
    }
    if !rebuild.is_empty() {
        return PromotionDecision {
            outcome: PromotionOutcome::RebuildRequired,
            reasons: rebuild,
            reusable_binary_sha256: None,
        };
    }
    PromotionDecision {
        outcome: PromotionOutcome::Eligible,
        reasons: Vec::new(),
        reusable_binary_sha256: Some(manifest.binary_sha256.clone()),
    }
}
