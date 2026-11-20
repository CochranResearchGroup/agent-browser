//! Public failure projection and private refusal custody share one occurrence.
use super::*;
use crate::native::service_failure_journal::{
    append_service_failure, ServiceFailureCategory, ServiceFailureRecord, ServiceFailureReferences,
};
use serde_json::json;

/// Preserve existing typed evidence, add executable exact-task inspection and
/// publish sanitized refusal metadata. Request payloads never enter the journal.
pub(crate) fn attach_public_failure(response: &mut Value, command: &Value, source: &str) {
    if response["success"] != false {
        return;
    }
    attach_service_failure_recourse_for_action(
        response,
        command["action"].as_str().unwrap_or("unknown"),
    );
    let error = response["error"].as_str().unwrap_or("operation_failed");
    let candidate = error.split(':').next().unwrap_or_default();
    let code = if !candidate.is_empty()
        && candidate.len() <= 128
        && candidate
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        candidate
    } else {
        "service_operation_failed"
    }
    .to_owned();
    let failure = &mut response["failure"];
    if failure["code"] == "service_operation_failed" {
        failure["code"] = json!(code);
    }
    let task = command["taskId"]
        .as_str()
        .or_else(|| command["sessionId"].as_str());
    let request = command["requestId"]
        .as_str()
        .or_else(|| command["id"].as_str());
    if failure["executableNextAction"].is_null() {
        failure["executableNextAction"] = if let Some(task) = task {
            json!({"tool":"browser_command","executable":true,
                "arguments":{"taskId":task,"action":"task_briefing"},
                "purpose":"Inspect current task and original uncertain operations before choosing recovery"})
        } else {
            json!({"tool":"browser_discover","executable":true,
                "arguments":{"category":"diagnostics"},
                "purpose":"Discover available profiles and supported workflow operations"})
        };
    }
    if failure["occurrenceId"].is_string() {
        return;
    }
    let record = ServiceFailureRecord::new(
        ServiceFailureCategory::ServiceAction,
        source,
        "public_refusal",
        &code,
        &code,
    )
    .with_action(command["action"].as_str().unwrap_or("unknown"))
    .with_references(ServiceFailureReferences {
        request_id: request.map(str::to_owned),
        session_id: task.map(str::to_owned),
        ..Default::default()
    })
    .with_details(json!({
        "effectState":failure["effectState"],
        "retryDisposition":failure["retryDisposition"],
        "evidenceFreshness":"not_supplied",
        "policySource":"not_supplied"
    }));
    failure["occurrenceId"] = json!(record.occurrence_id);
    failure["logging"] = match append_service_failure(&record) {
        Ok(_) => json!({"state":"durable_pending","projection":"asynchronous"}),
        Err(_) => json!({"state":"failed","code":"failure_journal_custody_failed"}),
    };
}
