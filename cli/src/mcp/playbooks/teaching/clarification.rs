//! Clarification remains inert candidate knowledge, never operator approval.
use super::{identifier, identifiers, references, sha256, text, TeachingDraft};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Clarification {
    questions: Vec<Question>,
    answers: Vec<Answer>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Question {
    id: String,
    question: String,
    interpretation: String,
    category: String,
    evidence_refs: Vec<String>,
    step_refs: Vec<String>,
    depends_on: Vec<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Answer {
    question_id: String,
    question_sha256: String,
    answer_ref: String,
    answer: String,
    actor_ref: String,
    source_ref: String,
    provenance: String,
    task_id: String,
    root_guide_id: String,
    answered_guide_id: String,
    answered_sha256: String,
    content_sha256: String,
}
impl Clarification {
    pub(super) fn valid(&self, draft: &TeachingDraft) -> bool {
        let Some(ids) = identifiers(self.questions.iter().map(|q| q.id.as_str())) else {
            return false;
        };
        if ids.is_empty() || self.answers.len() > 64 {
            return false;
        }
        let evidence: BTreeSet<_> = draft.evidence.iter().map(|e| e.id.as_str()).collect();
        let mut prior = BTreeSet::new();
        for q in &self.questions {
            if !text(&q.question)
                || !text(&q.interpretation)
                || !matches!(
                    q.category.as_str(),
                    "ambiguity"
                        | "contradiction"
                        | "missing_intent"
                        | "variable_input"
                        | "outcome"
                        | "exception"
                        | "recovery"
                )
                || !references(&q.evidence_refs, &evidence, true)
                || !references(&q.depends_on, &prior, false)
                || q.step_refs.is_empty()
                || q.step_refs.len() > 64
                || q.step_refs.iter().any(|reference| {
                    !draft.routines.iter().any(|routine| {
                        routine.steps.iter().enumerate().any(|(index, step)| {
                            reference == &format!("{}:{index}", routine.id)
                                && step
                                    .evidence_refs
                                    .iter()
                                    .any(|id| q.evidence_refs.contains(id))
                        })
                    })
                })
            {
                return false;
            }
            prior.insert(q.id.as_str());
        }
        let mut answered = BTreeSet::new();
        let mut receipt_ids = BTreeSet::new();
        for answer in &self.answers {
            let Some(question) = self.questions.iter().find(|q| q.id == answer.question_id) else {
                return false;
            };
            if !answered.insert(answer.question_id.as_str())
                || !receipt_ids.insert(answer.answer_ref.as_str())
                || !question
                    .depends_on
                    .iter()
                    .all(|id| answered.contains(id.as_str()))
                || !identifier(&answer.answer_ref)
                || !text(&answer.answer)
                || !identifier(&answer.actor_ref)
                || !identifier(&answer.source_ref)
                || answer.provenance != "authenticated_operator"
                || answer.task_id != draft.source_task_id
                || !identifier(&answer.root_guide_id)
                || !identifier(&answer.answered_guide_id)
                || !sha256(&answer.question_sha256)
                || !sha256(&answer.answered_sha256)
                || !sha256(&answer.content_sha256)
            {
                return false;
            }
        }
        true
    }
}
