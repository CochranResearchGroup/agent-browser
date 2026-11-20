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

/// Job readback is not a new refusal. Enrich retained failure contracts without
/// appending journal occurrences, changing outcomes, or guessing task identity
/// from provenance.sessionId (legacy producers also stored a session name there).
fn project_job_recovery(job: &mut Value) {
    if job["state"] == "running" {
        job["deadlineEvidence"] = json!({"state":"unqualified_legacy_start","automaticTerminalization":false,
            "nextAction":{"interface":"cli","command":"agent-browser","arguments":["--json","service","jobs","--id",job["id"]]},
            "resumeCondition":"Inspect current owner and original outcome; active-owner cancellation and positive process exit remain separate supported proofs"});
    }
    let mut command = json!({"action":job["action"],"jobId":job["id"]});
    if let Some(request) = job
        .pointer("/provenance/requestId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && *value != "unknown")
    {
        command["requestId"] = json!(request);
    }
    if !job["failure"].is_object()
        && matches!(
            job["state"].as_str(),
            Some("failed" | "cancelled" | "timed_out")
        )
    {
        if let Some(error) = job["error"].as_str() {
            let mut failure = serde_json::to_value(classify_service_failure(error))
                .expect("failure recourse serializes");
            if failure["code"] == "service_operation_failed" {
                if let Some(code) = code_token(error.split(':').next().unwrap_or_default()) {
                    failure["code"] = json!(code);
                }
            }
            job["failure"] = failure;
        }
    }
    for path in ["/failure", "/terminalOutcome/failure", "/result/failure"] {
        if let Some(failure) = job.pointer_mut(path).filter(|value| value.is_object()) {
            super::recovery_registry::enrich(failure, &command);
            if failure["executableNextAction"].is_null() && command["jobId"].is_string() {
                failure["executableNextAction"] = json!({"interface":"cli","command":"agent-browser",
                    "arguments":["--json","service","jobs","--id",command["jobId"]],"executable":true,
                    "purpose":"Inspect retained owner/original job; no task identity is inferred from a legacy session name"});
            }
        }
    }
}

fn project_job_readback(response: &mut Value, command: &Value) {
    if !matches!(
        command["action"].as_str(),
        Some("service_jobs" | "service_job_cancel" | "service_trace")
    ) {
        return;
    }
    if let Some(job) = response
        .pointer_mut("/data/job")
        .filter(|value| value.is_object())
    {
        project_job_recovery(job);
    }
    if let Some(jobs) = response
        .pointer_mut("/data/jobs")
        .and_then(Value::as_array_mut)
    {
        for job in jobs {
            project_job_recovery(job);
        }
    }
}

fn project_capture_time_recourse(response: &mut Value, command: &Value) {
    if command["action"] != "native_capture" || response["success"] != true {
        return;
    }
    let Some(error) = response
        .pointer("/data/timeQualificationFailure")
        .and_then(Value::as_str)
    else {
        return;
    };
    let code = error.split(':').next().unwrap_or_default();
    if !super::recovery_registry::contains(code) {
        return;
    }
    // The read remains acknowledged. This refusal applies only to the requested
    // freshness qualification; it never changes the original capture outcome.
    let mut qualification = json!({"success":false,"error":error,"failure":{"code":code,
        "scope":"native_observation_time_qualification","effectState":"no_effect",
        "readCustody":"acknowledged","inputEligibility":"not_granted"}});
    attach_public_failure(&mut qualification, command, "native_time_qualification");
    response["data"]["timeQualificationRecourse"] = qualification["failure"].clone();
}

/// Preserve existing typed evidence, add executable exact-task inspection and
/// publish sanitized refusal metadata. Request payloads never enter the journal.
pub(crate) fn attach_public_failure(response: &mut Value, command: &Value, source: &str) {
    project_job_readback(response, command);
    project_capture_time_recourse(response, command);
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
    fn running_job_exposes_unqualified_deadline_without_fabricating_failure() {
        let mut job =
            json!({"id":"retained-job","state":"running","startedAt":"2099-01-01T00:00:00Z"});
        project_job_recovery(&mut job);
        assert_eq!(job["state"], "running");
        assert_eq!(job["deadlineEvidence"]["state"], "unqualified_legacy_start");
        assert!(job.get("failure").is_none());
        let mut completed = json!({"id":"complete","state":"succeeded"});
        let original = completed.clone();
        project_job_recovery(&mut completed);
        assert_eq!(completed, original);
    }

    #[test]
    fn successful_job_readback_preserves_outcome_without_guessing_task_or_logging() {
        let mut response = json!({"success":true,"data":{"jobs":[{"id":"original-job","action":"native_input","state":"failed","provenance":{"requestId":"original-request","sessionId":"a-human-readable-lane"},"error":"browser_task_operation_capacity_reached","failure":{"code":"browser_task_operation_capacity_reached","effectState":"no_effect"}}]}});
        project_job_readback(&mut response, &json!({"action":"service_jobs"}));
        let job = &response["data"]["jobs"][0];
        assert_eq!(response["success"], true);
        assert_eq!(job["state"], "failed");
        assert_eq!(job["failure"]["recoveryRegistry"]["coverage"], "registered");
        assert_eq!(
            job["failure"]["recoveryRegistry"]["inspect"]["executable"],
            false
        );
        assert_eq!(
            job["failure"]["executableNextAction"]["arguments"][4],
            "original-job"
        );
        assert!(job["failure"].get("occurrenceId").is_none());
        assert!(!job["failure"].to_string().contains("a-human-readable-lane"));
        let before = response.clone();
        project_job_readback(&mut response, &json!({"action":"evaluate"}));
        assert_eq!(response, before);
    }

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
