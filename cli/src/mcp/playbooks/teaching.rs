//! Inert teaching knowledge extends the existing SiteGuide candidate format.
//! Validation checks attribution/reference integrity, not the truth of model claims.
mod clarification;
mod fresh_repeat;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{fault, JsonRpcError};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Attribution {
    Observed,
    Inferred,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TeachingDraft {
    schema_version: String,
    state: String,
    prepared_by: String,
    source_task_id: String,
    recording_id: String,
    video_sha256: String,
    evidence: Vec<Evidence>,
    claims: Vec<Claim>,
    parameters: Vec<Parameter>,
    states: Vec<PageKnowledge>,
    controls: Vec<Control>,
    routines: Vec<Routine>,
    playbook: Playbook,
    unresolved: Vec<Question>,
    limitations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lineage: Option<Lineage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    clarification: Option<clarification::Clarification>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    validation: Option<fresh_repeat::TeachingValidation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    review: Option<OperatorReview>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Evidence {
    id: String,
    kind: String,
    artifact: String,
    start_ms: u64,
    end_ms: u64,
    sha256: String,
    summary: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Claim {
    id: String,
    attribution: Attribution,
    statement: String,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Parameter {
    id: String,
    kind: String,
    example: Value,
    meaning: String,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PageKnowledge {
    id: String,
    description: String,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Control {
    id: String,
    role: String,
    name: String,
    attribution: Attribution,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Routine {
    id: String,
    goal: String,
    inputs: Vec<String>,
    preconditions: Vec<String>,
    steps: Vec<Step>,
    branches: Vec<Branch>,
    outcomes: Vec<Outcome>,
    recovery: Vec<Branch>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Step {
    action: String,
    target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parameter: Option<String>,
    description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expected_result: Option<String>,
    evidence_refs: Vec<String>,
    attribution: Attribution,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Branch {
    condition: String,
    next: String,
    evidence_refs: Vec<String>,
    attribution: Attribution,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Outcome {
    description: String,
    verification: String,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Playbook {
    id: String,
    goal: String,
    routine_ids: Vec<String>,
    parameters: Vec<String>,
    completion_verification: String,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Question {
    id: String,
    question: String,
    interpretation: String,
    evidence_refs: Vec<String>,
}

/// Immutable version provenance within the existing SiteGuide directory.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Lineage {
    root_guide_id: String,
    revision: u16,
    parent_guide_id: String,
    parent_sha256: String,
    operation_id: String,
    operation_kind: String,
    intent_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    accepted_version: Option<RetainedVersion>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RetainedVersion {
    guide_id: String,
    sha256: String,
}
/// A stored claim is not qualification: the receiving application must verify
/// the original operator receipt through its authenticated authority port.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OperatorReview {
    decision_id: String,
    decision: String,
    actor_ref: String,
    source_ref: String,
    provenance: String,
    task_id: String,
    root_guide_id: String,
    reviewed_guide_id: String,
    reviewed_sha256: String,
    content_sha256: String,
    source_unavailable_acknowledged: bool,
}

fn text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 4000 && !value.chars().any(|c| c.is_control())
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(&byte))
}
fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn identifiers<'a>(items: impl Iterator<Item = &'a str>) -> Option<BTreeSet<&'a str>> {
    let values: Vec<_> = items.collect();
    let set: BTreeSet<_> = values.iter().copied().collect();
    (values.len() <= 64 && values.len() == set.len() && values.iter().all(|id| identifier(id)))
        .then_some(set)
}
fn references(items: &[String], known: &BTreeSet<&str>, required: bool) -> bool {
    items.len() <= 64
        && (!required || !items.is_empty())
        && items.iter().all(|id| known.contains(id.as_str()))
}

impl TeachingDraft {
    pub(super) fn validate(&self) -> Result<(), JsonRpcError> {
        let bad = || fault("teaching_draft_contract_invalid");
        if self.schema_version != "agent-browser.teaching-draft.v1"
            || self.state != "draft"
            || !text(&self.prepared_by)
            || !text(&self.source_task_id)
            || !identifier(&self.recording_id)
            || !sha256(&self.video_sha256)
            || self.evidence.is_empty()
            || self.claims.is_empty()
            || self.states.is_empty()
            || self.controls.is_empty()
            || self.routines.is_empty()
            || self.limitations.len() > 32
            || self.limitations.iter().any(|item| !text(item))
        {
            return Err(bad());
        }
        if self.lineage.as_ref().is_some_and(|lineage| {
            !super::valid_guide_id(&lineage.root_guide_id)
                || !super::valid_guide_id(&lineage.parent_guide_id)
                || !(2..=256).contains(&lineage.revision)
                || !sha256(&lineage.parent_sha256)
                || !identifier(&lineage.operation_id)
                || !matches!(
                    lineage.operation_kind.as_str(),
                    "edit" | "review" | "repeat" | "promote"
                )
                || !sha256(&lineage.intent_sha256)
                || lineage.accepted_version.as_ref().is_some_and(|retained| {
                    !super::valid_guide_id(&retained.guide_id) || !sha256(&retained.sha256)
                })
        }) {
            return Err(bad());
        }
        if let Some(review) = &self.review {
            let Some(lineage) = &self.lineage else {
                return Err(bad());
            };
            if !identifier(&review.decision_id)
                || !identifier(&review.actor_ref)
                || !identifier(&review.source_ref)
                || !matches!(review.decision.as_str(), "approved" | "rejected")
                || review.provenance != "authenticated_operator"
                || !text(&review.task_id)
                || !super::valid_guide_id(&review.reviewed_guide_id)
                || !sha256(&review.reviewed_sha256)
                || review.root_guide_id != lineage.root_guide_id
                || !sha256(&review.content_sha256)
                || match lineage.operation_kind.as_str() {
                    "review" => {
                        review.reviewed_guide_id != lineage.parent_guide_id
                            || review.reviewed_sha256 != lineage.parent_sha256
                    }
                    "repeat" | "promote" => self.validation.as_ref().is_none_or(|validation| {
                        validation.review_decision_id != review.decision_id
                            || validation.content_sha256 != review.content_sha256
                    }),
                    _ => true,
                }
            {
                return Err(bad());
            }
        }
        if self.lineage.as_ref().is_some_and(|lineage| {
            matches!(lineage.operation_kind.as_str(), "repeat" | "promote")
                && (self.review.is_none() || self.validation.is_none())
        }) {
            return Err(bad());
        }
        if self
            .clarification
            .as_ref()
            .is_some_and(|value| !value.valid(self))
            || self
                .validation
                .as_ref()
                .is_some_and(|value| !value.valid(self))
        {
            return Err(bad());
        }
        let evidence =
            identifiers(self.evidence.iter().map(|item| item.id.as_str())).ok_or_else(bad)?;
        identifiers(self.claims.iter().map(|item| item.id.as_str())).ok_or_else(bad)?;
        let parameters =
            identifiers(self.parameters.iter().map(|item| item.id.as_str())).ok_or_else(bad)?;
        let states =
            identifiers(self.states.iter().map(|item| item.id.as_str())).ok_or_else(bad)?;
        let controls =
            identifiers(self.controls.iter().map(|item| item.id.as_str())).ok_or_else(bad)?;
        let routines =
            identifiers(self.routines.iter().map(|item| item.id.as_str())).ok_or_else(bad)?;
        identifiers(self.unresolved.iter().map(|item| item.id.as_str())).ok_or_else(bad)?;
        let refs = |items: &[String]| references(items, &evidence, true);
        if self.evidence.iter().any(|item| {
            !matches!(
                item.kind.as_str(),
                "video" | "narration" | "input" | "result"
            ) || !identifier(&item.artifact)
                || item.start_ms >= item.end_ms
                || item.end_ms > 1_800_000
                || !sha256(&item.sha256)
                || !text(&item.summary)
        }) || self.claims.iter().any(|item| {
            !text(&item.statement)
                || !references(
                    &item.evidence_refs,
                    &evidence,
                    !matches!(item.attribution, Attribution::Unknown),
                )
        }) || self.parameters.iter().any(|item| {
            !text(&item.meaning)
                || !refs(&item.evidence_refs)
                || match item.kind.as_str() {
                    "date" => !item.example.as_str().is_some_and(|date| {
                        date.len() == 10
                            && chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok()
                    }),
                    "boolean" => !item.example.is_boolean(),
                    "text" => !item.example.as_str().is_some_and(text),
                    _ => true,
                }
        }) || self
            .states
            .iter()
            .any(|item| !text(&item.description) || !refs(&item.evidence_refs))
            || self.controls.iter().any(|item| {
                !identifier(&item.role) || !text(&item.name) || !refs(&item.evidence_refs)
            })
        {
            return Err(bad());
        }
        for routine in &self.routines {
            if !text(&routine.goal)
                || !references(&routine.inputs, &parameters, false)
                || routine.preconditions.is_empty()
                || routine.preconditions.len() > 32
                || routine.preconditions.iter().any(|item| !text(item))
                || routine.steps.is_empty()
                || routine.steps.len() > 64
                || routine.outcomes.is_empty()
                || routine.outcomes.len() > 32
                || routine.branches.len() > 32
                || routine.recovery.is_empty()
                || routine.recovery.len() > 32
            {
                return Err(bad());
            }
            for step in &routine.steps {
                if !matches!(
                    step.action.as_str(),
                    "observe"
                        | "select"
                        | "activate"
                        | "fill"
                        | "set_checked"
                        | "verify"
                        | "request_assistance"
                ) || !(states.contains(step.target.as_str())
                    || controls.contains(step.target.as_str())
                    || step.target == "downloaded-artifact")
                    || !text(&step.description)
                    || step
                        .expected_result
                        .as_deref()
                        .is_some_and(|value| !text(value))
                    || !refs(&step.evidence_refs)
                    || step
                        .parameter
                        .as_ref()
                        .is_some_and(|id| !routine.inputs.contains(id))
                {
                    return Err(bad());
                }
            }
            if routine.outcomes.iter().any(|item| {
                !text(&item.description) || !text(&item.verification) || !refs(&item.evidence_refs)
            }) || routine
                .branches
                .iter()
                .chain(&routine.recovery)
                .any(|item| {
                    !text(&item.condition) || !text(&item.next) || !refs(&item.evidence_refs)
                })
            {
                return Err(bad());
            }
        }
        if !identifier(&self.playbook.id)
            || !text(&self.playbook.goal)
            || self.playbook.routine_ids.is_empty()
            || !references(&self.playbook.routine_ids, &routines, true)
            || !references(&self.playbook.parameters, &parameters, false)
            || !text(&self.playbook.completion_verification)
            || !refs(&self.playbook.evidence_refs)
            || self.unresolved.iter().any(|item| {
                !text(&item.question) || !text(&item.interpretation) || !refs(&item.evidence_refs)
            })
        {
            return Err(bad());
        }
        Ok(())
    }
}
