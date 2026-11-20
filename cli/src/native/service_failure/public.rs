//! Public failure projection and private refusal custody share one occurrence.
use super::*;
use crate::native::service_failure_journal::{
    append_service_failure, ServiceFailureCategory, ServiceFailureRecord, ServiceFailureReferences,
};
use serde_json::json;
use sha2::{Digest, Sha256};

fn code_token(value: &str) -> Option<&str> {
    (!value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'))
    .then_some(value)
}

/// Known transparent transport wrappers must not mask an exact registered
/// producer code. Unknown payloads keep the wrapper and its uncovered contract;
/// arbitrary provider prose is never promoted into a journal classification.
fn registered_producer_code(error: &str) -> Option<&str> {
    let mut remaining = error;
    for _ in 0..4 {
        let (wrapper, rest) = remaining.split_once(':')?;
        if !matches!(
            wrapper,
            "workflow_transport_failed" | "workflow_transport_start_failed" | "workflow_cli_failed"
        ) {
            return None;
        }
        let candidate = rest.split(':').next().unwrap_or_default();
        if super::recovery_registry::contains(candidate) {
            return Some(candidate);
        }
        remaining = rest;
    }
    None
}

/// Preserve existing typed evidence, add executable exact-task inspection and
/// publish sanitized refusal metadata. Request payloads never enter the journal.
pub(crate) fn attach_public_failure(response: &mut Value, command: &Value, source: &str) {
    if response["success"] != false {
        // Successful transport does not establish a successful operation. Only
        // the action envelope is inspected, never arbitrary evaluate result data.
        if response["success"] == true && response.pointer("/data/ok") == Some(&json!(false)) {
            let cause = response
                .pointer("/data/error")
                .and_then(Value::as_str)
                .or_else(|| response.pointer("/data/errorKind").and_then(Value::as_str))
                .unwrap_or("service_operation_failed");
            let mut operation = json!({"success":false,"error":cause});
            if response["failure"].is_object() {
                operation["failure"] = response["failure"].clone();
            }
            attach_public_failure(&mut operation, command, source);
            response["failure"] = operation["failure"].clone();
        }
        return;
    }
    let had_failure = response["failure"].is_object();
    attach_service_failure_recourse_for_action(
        response,
        command["action"].as_str().unwrap_or("unknown"),
    );
    let error = response["error"]
        .as_str()
        .unwrap_or("operation_failed")
        .to_owned();
    let candidate = registered_producer_code(&error)
        .unwrap_or_else(|| error.split(':').next().unwrap_or_default());
    let code = code_token(candidate)
        .unwrap_or("service_operation_failed")
        .to_owned();
    if had_failure {
        if let Ok(Value::Object(defaults)) = serde_json::to_value(classify_service_failure(&error))
        {
            let fields = response["failure"]
                .as_object_mut()
                .expect("existing failure is an object");
            for (key, value) in defaults {
                fields.entry(key).or_insert(value);
            }
        }
    }
    let failure = &mut response["failure"];
    if !had_failure && command["jobId"].is_null() && command["serviceJobId"].is_null() {
        failure
            .as_object_mut()
            .expect("classified failure is an object")
            .remove("jobId");
    }
    if failure["code"] == "service_operation_failed" {
        failure["code"] = json!(code);
    }
    let task = command["taskId"]
        .as_str()
        .or_else(|| command["sessionId"].as_str());
    let request = command["requestId"]
        .as_str()
        .or_else(|| command["id"].as_str());
    if command["action"] == "failure_log" {
        failure["effectState"] = json!("no_effect");
    }
    if code == "browser_task_operation_capacity_reached" {
        failure["effectState"] = json!("no_effect");
        failure["phase"] = json!("child_admission");
        failure["recommendedAction"] = json!("inspect_task_operation_capacity");
        if let Some(task) = task.filter(|_| failure["executableNextAction"].is_null()) {
            failure["executableNextAction"] = json!({"tool":"browser_command","executable":true,
                "arguments":{"taskId":task,"action":"task_operation","params":{"operation":"capacity"}},
                "purpose":"Inspect capacity and unresolved originals before adjusting configured limits"});
        }
    }
    super::recovery_registry::enrich(failure, command);
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
    let journal_code = failure["code"]
        .as_str()
        .and_then(code_token)
        .unwrap_or(&code)
        .to_owned();
    let record = ServiceFailureRecord::new(
        ServiceFailureCategory::ServiceAction,
        source,
        "public_refusal",
        &journal_code,
        &journal_code,
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
            "recommendedAction":failure["recommendedAction"],
            "blockingCheck":code,
            "taskIdSha256":task.map(|id| format!("{:x}", Sha256::digest(id.as_bytes()))),
        "evidenceFreshness":failure["recoveryPlan"]["evidence"],
        "policySource":failure["recoveryPlan"]["policySource"],
        "recoveryCoverage":failure["recoveryRegistry"]["coverage"]
    }));
    failure["occurrenceId"] = json!(record.occurrence_id);
    failure["logging"] = match append_service_failure(&record) {
        Ok(_) => json!({"state":"durable_pending","projection":"asynchronous"}),
        Err(_) => json!({"state":"failed","code":"failure_journal_custody_failed"}),
    };
}

#[cfg(test)]
mod registry_projection_tests {
    use super::*;
    #[test]
    fn transparent_wrapper_retains_registered_capacity_cause_only() {
        assert_eq!(
            registered_producer_code(
                "workflow_transport_failed:browser_task_operation_capacity_reached:private details"
            ),
            Some("browser_task_operation_capacity_reached")
        );
        assert_eq!(registered_producer_code("workflow_transport_failed:workflow_transport_start_failed:browser_control_lock_unavailable:private details"), Some("browser_control_lock_unavailable"));
        assert_eq!(
            registered_producer_code(
                "workflow_transport_failed:SELECT private_value FROM private_table"
            ),
            None
        );
        assert_eq!(
            registered_producer_code(
                "unregistered_wrapper:browser_task_operation_capacity_reached"
            ),
            None
        );
    }
}
