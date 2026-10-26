//! A stored verifier claim is not execution, approval or artifact qualification.
use super::{identifier, sha256, text, TeachingDraft};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TeachingValidation {
    schema_version: String,
    validation_id: String,
    provenance: String,
    task_id: String,
    source_task_id: String,
    root_guide_id: String,
    executed_guide_id: String,
    executed_sha256: String,
    pub(super) content_sha256: String,
    pub(super) review_decision_id: String,
    playbook_id: String,
    requested_start_date: String,
    requested_end_date: String,
    report_id: String,
    artifact_id: String,
    artifact_sha256: String,
    artifact_media_type: String,
    original_operation_id: String,
    verification: String,
}
impl TeachingValidation {
    pub(super) fn valid(&self, draft: &TeachingDraft) -> bool {
        self.schema_version == "agent-browser.teaching-fresh-repeat.v1"
            && self.provenance == "receiving_application_verifier"
            && identifier(&self.validation_id)
            && text(&self.task_id)
            && self.task_id != self.source_task_id
            && self.source_task_id == draft.source_task_id
            && draft
                .lineage
                .as_ref()
                .is_some_and(|lineage| lineage.root_guide_id == self.root_guide_id)
            && super::super::valid_guide_id(&self.executed_guide_id)
            && sha256(&self.executed_sha256)
            && sha256(&self.content_sha256)
            && identifier(&self.review_decision_id)
            && self.playbook_id == draft.playbook.id
            && chrono::NaiveDate::parse_from_str(&self.requested_start_date, "%Y-%m-%d").is_ok()
            && chrono::NaiveDate::parse_from_str(&self.requested_end_date, "%Y-%m-%d").is_ok()
            && self.requested_start_date.len() == 10
            && self.requested_end_date.len() == 10
            && self.requested_start_date <= self.requested_end_date
            && identifier(&self.report_id)
            && identifier(&self.artifact_id)
            && sha256(&self.artifact_sha256)
            && self.artifact_media_type == "text/csv"
            && identifier(&self.original_operation_id)
            && self.verification == "report_identity_requested_dates_and_content"
    }
}
