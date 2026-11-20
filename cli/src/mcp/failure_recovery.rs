//! Bounded recovery follows registered observations and existing authority.
//! It never submits the failed browser command or reconstructs its private input.
use super::{JsonRpcError, ServiceState};
use crate::native::service_failure::{self, recovery_state};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub(super) fn parameters_schema() -> Value {
    json!({"type":"object","additionalProperties":false,
        "properties":{
            "operation":{"enum":["inspect","packet","history","configure","run","reconcile","record_resume"]},
            "occurrenceId":{"type":"string","format":"uuid"},
            "attemptId":{"type":"string","format":"uuid"},
            "scope":{"enum":["task","defaults"]},"actor":{"type":"string","minLength":1},
            "reason":{"type":"string","minLength":1},
            "requestId":{"type":"string","minLength":1},
            "policy":{"type":"object","additionalProperties":false,"properties":{
                "maxAttempts":{"type":"integer","minimum":0},
                "maxNoProgress":{"type":"integer","minimum":1},
                "maxDurationMs":{"type":"integer","minimum":1}},"required":["maxAttempts","maxNoProgress"]},
            "remedy":{"type":"object","properties":{"tool":{"type":"string"},
                "arguments":{"type":"object"},"executable":{"type":"boolean"}},"required":["tool","arguments"]}},
        "required":["operation"],"oneOf":[
            {"properties":{"operation":{"const":"configure"}},"required":["actor","reason","policy"]},
            {"properties":{"operation":{"enum":["inspect","packet","history","run","reconcile"]}},"required":["occurrenceId"]},
            {"properties":{"operation":{"const":"record_resume"}},"required":["occurrenceId","attemptId","requestId","actor"]}]})
}

pub(super) fn cli(args: &[String], configured: &ServiceState) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("Usage: agent-browser --json mcp failure-recovery '<JSON request containing optional taskId and params>'".into());
    }
    let mut arguments: Value = serde_json::from_str(&args[2])
        .map_err(|_| "failure_recovery_cli_request_invalid".to_string())?;
    if !arguments.is_object() {
        return Err("failure_recovery_cli_request_invalid".into());
    }
    arguments["action"] = json!("failure_recovery");
    let call = json!({"name":"browser_command","arguments":arguments});
    match super::call_service_mcp_tool(Some(&call), "default", configured) {
        Ok(reply) => reply["content"]
            .as_array()
            .and_then(|blocks| blocks.iter().find_map(|block| block["text"].as_str()))
            .and_then(|text| serde_json::from_str(text).ok())
            .ok_or_else(|| "failure_recovery_cli_response_invalid".into()),
        Err(error) => Ok(
            json!({"success":false,"error":error.data.as_ref().and_then(|data|data["message"].as_str()).unwrap_or(error.message),
            "failure":error.data.as_ref().and_then(|data|data.get("failure"))}),
        ),
    }
}

fn issue(message: impl AsRef<str>) -> JsonRpcError {
    let message = message.as_ref();
    let code = message.split(':').next().unwrap_or_default();
    let invalid = matches!(
        code,
        "failure_recovery_argument_required"
            | "failure_recovery_scope_invalid"
            | "failure_recovery_policy_invalid"
            | "failure_recovery_attempt_id_invalid"
            | "failure_recovery_operation_invalid"
            | "failure_recovery_action_not_supported"
    );
    JsonRpcError {
        code: if invalid { -32602 } else { -32603 },
        message: if invalid {
            "Invalid recovery parameters"
        } else {
            "Recovery prerequisite unavailable"
        },
        data: Some(json!({"message":message,"failure":{"code":code,
            "effectState":if invalid {"no_effect"}else{"effect_uncertain"}}})),
    }
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, JsonRpcError> {
    value[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| issue(format!("failure_recovery_argument_required: {key}")))
}

fn occurrence(id: &str) -> Result<Value, JsonRpcError> {
    let read =
        crate::native::service_failure_journal::read_failure_occurrence(id).map_err(issue)?;
    serde_json::to_value(read).map_err(|_| issue("failure_recovery_record_invalid"))?["records"]
        .as_array().and_then(|records| records.first()).cloned()
        .ok_or_else(|| issue("failure_recovery_occurrence_missing: preserve the original occurrence ID and inspect its logging state"))
}

fn bound_task<'a>(record: &Value, arguments: &'a Value) -> Result<Option<&'a str>, JsonRpcError> {
    let Some(task) = arguments["taskId"].as_str() else {
        return Ok(None);
    };
    let expected = format!("{:x}", Sha256::digest(task.as_bytes()));
    if record
        .pointer("/details/taskIdSha256")
        .and_then(Value::as_str)
        != Some(expected.as_str())
    {
        return Err(issue("failure_recovery_scope_conflict: this occurrence has no verified binding to that task; use its original task or task-free diagnostics"));
    }
    Ok(Some(task))
}

fn contract(record: &Value, task: Option<&str>) -> Value {
    let mut failure = json!({"code":record["code"],"occurrenceId":record["occurrenceId"],
        "effectState":record.pointer("/details/effectState").cloned().unwrap_or(json!("effect_uncertain"))});
    let command = json!({"taskId":task,"requestId":record.pointer("/references/requestId"),
        "jobId":record.pointer("/references/jobId"),"action":record["action"]});
    service_failure::enrich_recovery_contract(&mut failure, &command);
    failure
}

fn plan(failure: &Value) -> &Value {
    if failure["recoveryRegistry"].is_object() {
        &failure["recoveryRegistry"]
    } else {
        &failure["recoveryPlan"]
    }
}

/// Persist and return only diagnostic shape, never a page URL, body, credential,
/// raw provider response, artifact bytes, or caller-supplied command parameters.
fn diagnostic(value: &Value) -> Value {
    let mut result = serde_json::Map::new();
    for key in [
        "status",
        "mode",
        "currentContext",
        "urlObserved",
        "state",
        "effectStatus",
        "activeCount",
        "unresolvedCount",
        "archivedCount",
        "capacity",
        "policy",
        "artifactReferenceCount",
        "artifactReferenceLimit",
        "timeQualificationFailure",
        "policySource",
        "effectivePolicySource",
        "pauseReads",
        "owner",
        "awaitingObservation",
        "epoch",
    ] {
        if let Some(value) = value.get(key) {
            if value.is_boolean()
                || value.is_number()
                || value.is_null()
                || (value.is_string()
                    && value.as_str().is_some_and(|s| {
                        s.len() <= 128
                            && s.bytes()
                                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
                    }))
                || (key == "policy"
                    && value.is_object()
                    && value
                        .as_object()
                        .is_some_and(|p| p.values().all(Value::is_u64)))
            {
                result.insert(key.into(), value.clone());
            }
        }
    }
    for key in ["operation", "control"] {
        if value[key].is_object() {
            result.insert(key.into(), diagnostic(&value[key]));
        }
    }
    if let Some(failure) = value["timeQualificationFailure"].as_str() {
        result.insert("clockQualification".into(), json!("unavailable"));
        let code = failure.split(':').next().unwrap_or_default();
        if code.len() <= 128 && code.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            result.insert("clockFailureCode".into(), json!(code));
        }
    } else if [
        "/serviceTime",
        "/observation/serviceTime",
        "/nativeLoginControls/serviceTime",
    ]
    .iter()
    .any(|path| value.pointer(path).is_some_and(Value::is_object))
    {
        result.insert(
            "clockQualification".into(),
            json!("qualified_owner_observation"),
        );
    }
    Value::Object(result)
}

fn supported(step: &Value, task: Option<&str>) -> bool {
    if step["executable"] == false {
        return false;
    }
    let tool = step["tool"].as_str().unwrap_or_default();
    let args = &step["arguments"];
    if tool != "browser_discover" && (task.is_none() || args["taskId"].as_str() != task) {
        return false;
    }
    match tool {
        "browser_discover" | "browser_continue" => true,
        "browser_control" => matches!(args["operation"].as_str(), Some("status" | "view")),
        "browser_reconnect" => matches!(args["operation"].as_str(), Some("status" | "reconnect")),
        "browser_policy" => matches!(
            args["operation"].as_str(),
            Some("status" | "override_lookup")
        ),
        "browser_command" => match args["action"].as_str() {
            Some("mode" | "task_briefing" | "failure_log" | "native_capture") => true,
            Some("task_operation") => {
                args["params"]["scope"] != "defaults"
                    && matches!(
                        args["params"]["operation"].as_str(),
                        None | Some("capacity" | "reconcile")
                    )
            }
            Some("profile_duplicate") => matches!(
                args["params"]["choice"].as_str(),
                Some("inspect" | "status")
            ),
            _ => false,
        },
        _ => false,
    }
}

struct Observation {
    data: Value,
    summary: Value,
    success: bool,
}
fn invoke(
    step: &Value,
    task: Option<&str>,
    request: &str,
    session: &str,
    configured: &ServiceState,
) -> Observation {
    if !supported(step, task) {
        return Observation {
            data: Value::Null,
            success: false,
            summary: json!({"status":"external_action_required","nextAction":step}),
        };
    }
    let mut call = json!({"name":step["tool"],"arguments":step["arguments"]});
    if call["arguments"]["requestId"].is_null() {
        call["arguments"]["requestId"] = json!(request);
    }
    let reply = super::call_service_mcp_tool(Some(&call), session, configured);
    match reply {
        Ok(reply) => {
            let payload = reply["content"]
                .as_array()
                .and_then(|blocks| blocks.iter().find_map(|b| b["text"].as_str()))
                .and_then(|text| serde_json::from_str::<Value>(text).ok())
                .unwrap_or(Value::Null);
            if payload.is_null() {
                return Observation {
                    data: Value::Null,
                    success: false,
                    summary: json!({"status":"response_unavailable","code":"failure_recovery_tool_response_invalid","nextAction":step}),
                };
            }
            let success =
                payload["success"] == true && payload.pointer("/data/ok") != Some(&json!(false));
            let data = payload["data"].clone();
            Observation {
                summary: json!({"status":if success {"observed"} else {"refused"},
                "tool":step["tool"],"observation":diagnostic(&data),"code":payload.pointer("/failure/code"),
                "occurrenceId":payload.pointer("/failure/occurrenceId")}),
                data,
                success,
            }
        }
        Err(error) => Observation {
            data: Value::Null,
            success: false,
            summary: json!({"status":"refused","tool":step["tool"],
                "code":error.data.as_ref().and_then(|data| data.pointer("/failure/code")),
                "occurrenceId":error.data.as_ref().and_then(|data| data.pointer("/failure/occurrenceId"))}),
        },
    }
}

fn prerequisite(code: &str, data: &Value) -> bool {
    match code {
        "browser_task_operation_capacity_reached" => data["activeCount"]
            .as_u64()
            .map(|active| data["unresolvedCount"].as_u64().unwrap_or(active))
            .zip(
                data.pointer("/policy/maximumCapacity")
                    .and_then(Value::as_u64),
            )
            .is_some_and(|(a, b)| a < b),
        "browser_artifact_reference_capacity_reached" => data["artifactReferenceCount"]
            .as_u64()
            .zip(data["artifactReferenceLimit"].as_u64())
            .is_some_and(|(a, b)| a < b),
        "browser_control_human_active" | "browser_control_paused" => {
            data["owner"] == "agent" && data["awaitingObservation"] == false
        }
        _ => false,
    }
}

fn packet(record: &Value, failure: &Value, task: Option<&str>) -> Value {
    json!({"schemaVersion":"agent-browser.recovery-packet.v1","occurrenceId":record["occurrenceId"],
        "code":record["code"],"originalRequestId":record.pointer("/references/requestId"),
        "taskIdSha256":record.pointer("/details/taskIdSha256"),"effectState":failure["effectState"],
        "recoveryPlan":plan(failure),"originalOperationReplayed":false,"taskCompletion":"unverified",
        "resume":{"tool":"browser_command","arguments":{"taskId":task,"action":"failure_recovery",
            "params":{"operation":"inspect","occurrenceId":record["occurrenceId"]}}},
        "excludes":["credentials","cookies","pageContent","rawProviderResponses","privateCommandParameters"]})
}

fn journal(
    record: &Value,
    task: Option<&str>,
    attempt: &str,
    outcome: &str,
    verified_request: Option<&str>,
) -> Value {
    use crate::native::service_failure_journal::{
        append_service_failure, ServiceFailureCategory, ServiceFailureRecord,
    };
    let event = ServiceFailureRecord::new(
        ServiceFailureCategory::ServiceAction,
        "bounded_failure_recovery",
        if outcome=="in_flight" {"recovery_attempt"}else{"recovery_outcome"},
        "failure_recovery_outcome",
        "Bounded recovery outcome; original browser command was not replayed",
    )
    .with_action("failure_recovery")
    .with_details(
        json!({"recordKind":if outcome=="in_flight" {"recovery_attempt"}else{"recovery_outcome"},"parentOccurrenceId":record["occurrenceId"],
            "originalCode":record["code"],"attemptId":attempt,"outcome":outcome,"verifiedRequestId":verified_request,
            "taskIdSha256":task.map(|id|format!("{:x}",Sha256::digest(id.as_bytes())))}),
    );
    match append_service_failure(&event) {
        Ok(_) => json!({"state":"durable_pending","occurrenceId":event.occurrence_id}),
        Err(_) => json!({"state":"failed","code":"failure_journal_custody_failed"}),
    }
}

pub(super) fn call(
    arguments: &Value,
    session: &str,
    configured: &ServiceState,
) -> Result<Value, JsonRpcError> {
    let params = &arguments["params"];
    let operation = params["operation"].as_str().unwrap_or("inspect");
    if operation == "configure" {
        text(params, "actor")?;
        text(params, "reason")?;
        let task = match params["scope"].as_str().unwrap_or("task") {
            "defaults" => None,
            "task" => Some(text(arguments, "taskId")?),
            _ => {
                return Err(issue(
                    "failure_recovery_scope_invalid: use task or defaults",
                ))
            }
        };
        if let Some(task) = task {
            super::workflow::inspect_briefing(task).map_err(issue)?;
        }
        let policy: recovery_state::Policy = serde_json::from_value(params["policy"].clone())
            .map_err(|_| issue("failure_recovery_policy_invalid: provide maxAttempts and positive maxNoProgress"))?;
        recovery_state::configure(
            task,
            &policy,
            text(params, "actor")?,
            text(params, "reason")?,
        )
        .map_err(issue)?;
        return Ok(super::tool_response_from_payload(
            "browser_command",
            session,
            json!({}),
            json!({"status":"configured","policy":policy,"scope":if task.is_some(){"task"}else{"defaults"},
                "requiresRestart":false,"permissionsExpanded":false}),
            false,
        ));
    }
    let id = text(params, "occurrenceId")?;
    let mut intent = arguments.clone();
    if let Some(params) = intent["params"].as_object_mut() {
        params.remove("attemptId");
    }
    let intent_tag = hex::encode(Sha256::digest(
        serde_json::to_vec(&intent).map_err(|_| issue("failure_recovery_request_invalid"))?,
    ));
    if operation == "history" {
        let history = recovery_state::inspect(id, params["attemptId"].as_str()).map_err(issue)?;
        if history
            .pointer("/summary/occurrenceId")
            .and_then(Value::as_str)
            .is_some_and(|id| !id.is_empty())
            && history.pointer("/summary/taskId").and_then(Value::as_str)
                != arguments["taskId"].as_str()
        {
            return Err(issue("failure_recovery_scope_conflict: use the original task for retained recovery history"));
        }
        return Ok(super::tool_response_from_payload(
            "browser_command",
            session,
            json!({}),
            json!({"status":"inspectable","history":history,"occurrenceId":id,
                "journalReadRequired":false,"originalOperationReplayed":false}),
            false,
        ));
    }
    if operation == "run" {
        if let Some(attempt) = params["attemptId"].as_str() {
            if let Some(mut prior) =
                recovery_state::replay(id, attempt, arguments["taskId"].as_str(), &intent_tag)
                    .map_err(issue)?
            {
                prior["replayedRecoveryResult"] = json!(true);
                return Ok(super::tool_response_from_payload(
                    "browser_command",
                    session,
                    json!({}),
                    prior,
                    false,
                ));
            }
        }
    }
    let record = occurrence(id)?;
    let task = bound_task(&record, arguments)?;
    let failure = contract(&record, task);
    let policy = recovery_state::policy(task);
    let history = recovery_state::inspect(id, params["attemptId"].as_str());
    let mut result = json!({"occurrenceId":id,"code":record["code"],"taskId":task,
        "failure":failure,"packet":packet(&record,&failure,task),"requiresRestart":false,
        "originalOperationReplayed":false,"physicalInputDispatched":false,"resumed":false});
    match &policy {
        Ok((policy,source,provenance)) => { result["policy"]=json!(policy); result["policySource"]=json!(source); result["policyProvenance"]=provenance.clone(); }
        Err(_) => result["policyAvailability"]=json!("runtime_store_unavailable; task-free packet and declared CLI diagnostics remain available"),
    }
    if let Ok(history) = &history {
        result["history"] = history.clone();
    }
    match operation {
        "record_resume" => {
            let task = task.ok_or_else(||issue("failure_recovery_scope_conflict: a resume report needs the original task binding"))?;
            let request = text(params,"requestId")?;
            let observed = super::task_operations::lookup(task,request).map_err(issue)?;
            if observed.pointer("/operation/taskId").and_then(Value::as_str) != Some(task)
                || observed.pointer("/operation/state") != Some(&json!("owner_acknowledged")) {
                return Err(issue("failure_recovery_resume_unverified: the exact task operation must be owner acknowledged; uncertain or refused input is not resumed"));
            }
            let attempt = text(params,"attemptId")?;
            result = recovery_state::record_resume(id,attempt,task,request,text(params,"actor")?).map_err(issue)?;
            if result["replayedResumeReport"] != true {
                result["logging"] = journal(&record,Some(task),attempt,"resume_reported_owner_acknowledgement_verified",Some(request));
            }
        }
        "inspect" | "packet" | "history" => result["status"]=json!("inspectable"),
        "run" | "reconcile" => {
            if plan(&failure)["coverage"] != "registered" {
                result["status"] = json!("unregistered_recovery_requires_sanitized_escalation");
                return Ok(super::tool_response_from_payload("browser_command",session,json!({}),result,false));
            }
            if let Some(step) = params.get("remedy") {
                if !supported(step,task) {
                    return Err(issue("failure_recovery_action_not_supported: retain the declared actor/interface and task; no physical input, permission override, or original command replay is supported"));
                }
            }
            let (policy,_,_) = policy.map_err(issue)?;
            let attempt = params["attemptId"].as_str().map(str::to_owned)
                .unwrap_or_else(||uuid::Uuid::new_v4().to_string());
            let budget_start = if policy.max_duration_ms.is_some() {
                Some(crate::connection::stable_time::native_now().map_err(|cause| issue(format!("failure_recovery_time_unqualified: {cause}; inspect/history remain available")))?)
            } else { None };
            let within_budget = || -> Result<(), String> {
                if let (Some(start),Some(limit)) = (&budget_start,policy.max_duration_ms) {
                    let now=crate::connection::stable_time::native_now()?;
                    start.fresh_at(&now,limit).map_err(|cause|format!("failure_recovery_time_budget_unqualified: {cause}; inspect the retained attempt and update the optional live budget before a new recovery intent"))?;
                }
                Ok(())
            };
            let current_plan = plan(&failure);
            let observed = invoke(&current_plan["inspect"],task,&format!("recovery-{attempt}-inspect"),session,configured);
            let before = diagnostic(&observed.data);
            let diagnosis = hex::encode(Sha256::digest(serde_json::to_vec(&json!({"code":record["code"],"evidence":before})).expect("diagnosis serializes")));
            if operation == "run" {
                if let Some(mut prior) = recovery_state::claim(id,task,&attempt,&intent_tag,&diagnosis,&policy).map_err(issue)? {
                    prior["replayedRecoveryResult"]=json!(true);
                    return Ok(super::tool_response_from_payload("browser_command",session,json!({}),prior,false));
                }
            } else {
                let history = history.map_err(issue)?;
                if history["summary"]["inFlight"].as_str() != Some(attempt.as_str()) {
                    return Err(issue("failure_recovery_attempt_conflict: reconcile only the retained in-flight attempt ID"));
                }
            }
            result["attemptId"]=json!(attempt);
            if operation=="run" { result["attemptLogging"]=journal(&record,task,&attempt,"in_flight",None); }
            result["inspection"]=observed.summary;
            let mut last_data = observed.data;
            let time_admission = within_budget();
            if operation == "run" && observed.success && time_admission.is_ok() {
                let supplied = params.get("remedy");
                let derived = current_plan["remediation"].as_array().and_then(|steps|
                    steps.iter().filter_map(|step|step.get("call")).find(|step|supported(step,task)));
                if let Some(step) = supplied.or(derived) {
                    let remedy = invoke(step,task,&format!("recovery-{attempt}-remedy"),session,configured);
                    result["remedy"]=remedy.summary;
                    if remedy.success { last_data=remedy.data; }
                } else { result["nextAction"]=current_plan["remediation"].clone(); }
            }
            let verification_admission = within_budget();
            let verified = if verification_admission.is_ok() {
                invoke(&current_plan["verification"],task,&format!("recovery-{attempt}-verify"),session,configured)
            } else {
                Observation{data:Value::Null,success:false,summary:json!({"status":"time_budget_unqualified","cause":verification_admission.err(),"inspectionRemainsAvailable":true})}
            };
            if let Err(cause)=time_admission { result["timeBudget"]=json!({"status":"unqualified","cause":cause,"inspectionRemainsAvailable":true}); }
            result["verification"]=verified.summary;
            let ready = prerequisite(record["code"].as_str().unwrap_or_default(),&last_data)
                && verified.success;
            let settled_no_effect = last_data.pointer("/operation/state") == Some(&json!("owner_refused_no_effect"));
            result["status"]=json!(if settled_no_effect {"original_no_effect_settled"}else if ready {"ready_for_explicit_resume"}else if operation=="reconcile"{"interrupted_outcome_not_reconstructed"}else{"unresolved"});
            if settled_no_effect {
                result["freshObservationRequiredBeforeNewIntent"] = json!(true);
                result["originalRequestIdReusable"] = json!(false);
            }
            result["resumeCondition"]=current_plan["resumeCondition"].clone();
            let after = hex::encode(Sha256::digest(serde_json::to_vec(&json!({"code":record["code"],"evidence":diagnostic(&last_data)})).expect("diagnosis serializes")));
            result["logging"]=journal(&record,task,&attempt,result["status"].as_str().unwrap_or("unresolved"),None);
            recovery_state::complete(id,&attempt,&after,ready,&result).map_err(issue)?;
        }
        _ => return Err(issue("failure_recovery_operation_invalid: use inspect, packet, history, configure, run, reconcile or record_resume")),
    }
    Ok(super::tool_response_from_payload(
        "browser_command",
        session,
        json!({}),
        result,
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_primitives_never_dispatch_original_input_or_broaden_scope() {
        for action in ["native_input", "evaluate", "navigate", "mode_convert"] {
            assert!(!supported(
                &json!({"tool":"browser_command","arguments":{"taskId":"one","action":action}}),
                Some("one")
            ));
        }
        assert!(!supported(
            &json!({"tool":"browser_control","arguments":{"taskId":"one","operation":"override"}}),
            Some("one")
        ));
        assert!(!supported(
            &json!({"tool":"browser_command","arguments":{"taskId":"peer","action":"task_operation","params":{"operation":"reconcile","requestId":"original"}}}),
            Some("one")
        ));
        assert!(!supported(
            &json!({"tool":"browser_command","arguments":{"taskId":"one","action":"task_operation","params":{"operation":"capacity","scope":"defaults"}}}),
            Some("one")
        ));
        assert!(supported(
            &json!({"tool":"browser_command","arguments":{"taskId":"one","action":"task_operation","params":{"operation":"reconcile","requestId":"original"}}}),
            Some("one")
        ));
        let raw = json!({"url":"https://private.invalid/?token=PRIVATE","body":"PRIVATE", "password":"PRIVATE","state":"uncertain","capacity":3});
        assert!(!diagnostic(&raw).to_string().contains("PRIVATE"));
        let record = json!({"occurrenceId":"retained","details":{"taskIdSha256":hex::encode(Sha256::digest("one"))}});
        assert_eq!(
            bound_task(&record, &json!({"taskId":"one"})).unwrap(),
            Some("one")
        );
        assert!(bound_task(&record, &json!({"taskId":"peer"})).is_err());
    }
}
