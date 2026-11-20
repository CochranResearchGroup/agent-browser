//! Exact-code recovery contracts. Unknown producer codes remain visibly uncovered.
use serde_json::{json, Value};
use std::sync::OnceLock;

fn registry() -> &'static Value {
    static REGISTRY: OnceLock<Value> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry: Value = serde_json::from_str(include_str!(
            "../../../../docs/dev/contracts/public-error-recovery-registry.v1.json"
        ))
        .expect("validated recovery registry must parse");
        for source in [
            include_str!(
                "../../../../docs/dev/contracts/public-error-recovery-client-dashboard.v1.json"
            ),
            include_str!("../../../../docs/dev/contracts/public-error-recovery-service.v1.json"),
        ] {
            let fragment: Value =
                serde_json::from_str(source).expect("validated recovery fragment must parse");
            for key in ["recipes", "codes", "interfaces"] {
                if let Some(additions) = fragment[key].as_object() {
                    if registry[key].is_null() {
                        registry[key] = json!({});
                    }
                    let destination = registry[key].as_object_mut().expect("registry map");
                    for (name, value) in additions {
                        assert!(
                            destination.insert(name.clone(), value.clone()).is_none(),
                            "duplicate recovery contract"
                        );
                    }
                }
            }
        }
        registry
    })
}

pub(super) fn contains(code: &str) -> bool {
    registry()["codes"].get(code).is_some()
}

fn bind(value: &Value, command: &Value) -> (Value, bool) {
    match value {
        Value::String(token) if token.starts_with('$') => {
            let replacement = match token.as_str() {
                "$taskId" => command.get("taskId").or_else(|| command.get("sessionId")),
                "$requestId" => command.get("requestId").or_else(|| command.get("id")),
                "$occurrenceId" => command.get("occurrenceId"),
                "$attemptId" => command.pointer("/params/attemptId"),
                "$profileId" => command.get("profileId").or_else(|| {
                    command
                        .get("params")
                        .and_then(|params| params.get("profileId"))
                }),
                "$sourceProfileId" => command.get("sourceProfileId").or_else(|| {
                    command
                        .get("params")
                        .and_then(|params| params.get("sourceProfileId"))
                }),
                "$operationId" => command.get("operationId").or_else(|| {
                    command
                        .get("params")
                        .and_then(|params| params.get("operationId"))
                }),
                // Reviewed policy is deliberately never inferred from a failed request.
                _ => None,
            };
            match replacement.filter(|value| value.is_string() && value.as_str() != Some("")) {
                Some(value) => (value.clone(), true),
                None => (value.clone(), false),
            }
        }
        Value::Object(fields) => {
            let mut complete = true;
            let resolved = fields
                .iter()
                .map(|(key, value)| {
                    let (value, bound) = bind(value, command);
                    complete &= bound;
                    (key.clone(), value)
                })
                .collect();
            (Value::Object(resolved), complete)
        }
        Value::Array(values) => {
            let mut complete = true;
            let resolved = values
                .iter()
                .map(|value| {
                    let (value, bound) = bind(value, command);
                    complete &= bound;
                    value
                })
                .collect();
            (Value::Array(resolved), complete)
        }
        _ => (value.clone(), true),
    }
}

fn step(template: &Value, command: &Value) -> Value {
    let (mut call, complete) = bind(template, command);
    call["executable"] = json!(complete && call["kind"] != "source_reference");
    if !complete {
        call["precondition"] = json!("Resolve required task/original identity or reviewed policy before invoking this template; do not guess scope");
    }
    call
}

/// Attach a bounded plan without broadening authority or asserting evidence that
/// the adapter has not observed. The journal can retain this same classification.
pub(super) fn enrich(failure: &mut Value, command: &Value) {
    let code = failure["code"]
        .as_str()
        .unwrap_or("service_operation_failed")
        .to_owned();
    let mut context = if command.is_object() {
        command.clone()
    } else {
        json!({})
    };
    // Recovery errors refer to the original journal occurrence, not the new
    // refusal occurrence. Other failures bind only their published actual ID.
    context["occurrenceId"] = if command["action"] == "failure_recovery" {
        command
            .pointer("/params/occurrenceId")
            .cloned()
            .unwrap_or_else(|| failure["occurrenceId"].clone())
    } else {
        failure["occurrenceId"].clone()
    };
    let command = &context;
    let source = registry();
    let Some(entry) = source["codes"].get(&code) else {
        let plan = json!({
            "schemaVersion":"agent-browser.public-error-recovery.v1",
            "coverage":"unregistered", "code":code, "failureCategory":"unclassified_runtime_failure",
            "requiredActor":"maintainer", "maximumAutomaticAttempts":0,
            "evidence":{"status":"unavailable","reason":"specific_check_contract_not_registered"},
            "escalation":{"operation":"retain_sanitized_failure_packet",
                "include":["code","occurrenceId","originalRequestId","taskId","effectState","logging"],
                "exclude":["credentials","cookies","pageContent","rawProviderErrors"]},
            "resumeCondition":"The specific defect or prerequisite is resolved, original effects are reconciled, and the same task is freshly observed; independent safe reads may continue"
        });
        publish_plan(failure, plan, command);
        return;
    };
    let recipe_name = command["action"]
        .as_str()
        .and_then(|action| entry["recipesByAction"][action].as_str())
        .or_else(|| entry["recipe"].as_str())
        .unwrap_or_default();
    let recipe = &source["recipes"][recipe_name];
    let inspect = step(&recipe["inspect"], command);
    let verification = step(&recipe["verification"], command);
    let mut remediation = recipe["remediation"].clone();
    if let Some(steps) = remediation.as_array_mut() {
        for remediation in steps {
            if let Some(call) = remediation.get("call") {
                remediation["call"] = step(call, command);
            }
        }
    }
    let diagnosis = &entry["evidenceClass"];
    // JSON-RPC compatibility codes are transport vocabulary, not diagnosis or
    // effect evidence. Categorize only the reviewed producer contract and keep
    // the actual owner's axis, phase, plan and effect state unchanged.
    let category = match diagnosis.as_str() {
        Some("invalid") => "invalid_argument_or_contract",
        Some("unavailable") => "runtime_dependency_unavailable",
        Some("stale") => "evidence_or_binding_changed",
        Some("denied") => "authority_required",
        Some("uncertain") => "owner_or_original_outcome_unqualified",
        Some("configured_budget") => "configured_operational_limit",
        Some("coordination_paused") => "coordination_pending",
        _ => "unclassified_runtime_failure",
    };
    let plan = json!({
        "schemaVersion":"agent-browser.public-error-recovery.v1",
        "coverage":"registered", "code":code, "diagnosis":diagnosis, "failureCategory":category,
        "rationale":recipe["rationale"], "requiredActor":recipe["requiredActor"],
        "policySource":{"kind":"owning_runtime_check","path":entry["policySource"],"effectiveConfiguration":"not_supplied"},
        "evidence":{"status":"not_observed_by_projection","required":recipe["evidenceRequirement"]},
        "inspect":inspect,"remediation":remediation,"verification":verification,
        "maximumAutomaticAttempts":recipe["maximumAutomaticAttempts"],"budgetScope":"single_recipe_cycle",
        "resumeCondition":recipe["resumeCondition"]
    });
    publish_plan(failure, plan, command);
    if failure["executableNextAction"].is_null() && inspect["executable"] == true {
        failure["executableNextAction"] = inspect;
    }
}

fn upgrade_compiled_bindings(value: &Value, command: &Value) -> Value {
    let (mut upgraded, _) = bind(value, command);
    fn refresh_steps(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                for value in fields.values_mut() {
                    refresh_steps(value);
                }
                if fields.contains_key("executable") {
                    let (_, complete) = bind(&Value::Object(fields.clone()), &Value::Null);
                    fields.insert(
                        "executable".into(),
                        json!(complete && fields.get("kind") != Some(&json!("source_reference"))),
                    );
                }
            }
            Value::Array(values) => values.iter_mut().for_each(refresh_steps),
            _ => {}
        }
    }
    refresh_steps(&mut upgraded);
    upgraded
}

fn publish_plan(failure: &mut Value, plan: Value, command: &Value) {
    let existing = failure["recoveryRegistry"].clone();
    if existing["coverage"] == "registered" && existing["code"] == plan["code"] {
        // Only fill unresolved tokens; already bound scope and owner evidence
        // survive a facade which has missing or different current context.
        let upgraded = upgrade_compiled_bindings(&existing, command);
        if failure["recoveryPlan"] == existing {
            failure["recoveryPlan"] = upgraded.clone();
        }
        failure["recoveryRegistry"] = upgraded;
    } else {
        failure["recoveryRegistry"] = plan.clone();
    }
    if !failure["recoveryPlan"].is_object() {
        failure["recoveryPlan"] = plan;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_occurrence_binds_after_publication_and_preserves_original_recovery_scope() {
        let mut failure = json!({"code":"failure_recovery_budget_exhausted"});
        enrich(&mut failure, &json!({"taskId":"original-task"}));
        assert_eq!(failure["recoveryRegistry"]["inspect"]["executable"], false);
        failure["occurrenceId"] = json!("published-occurrence");
        enrich(&mut failure, &json!({"taskId":"different-facade-task"}));
        assert_eq!(
            failure["recoveryRegistry"]["inspect"]["arguments"]["params"]["occurrenceId"],
            "published-occurrence"
        );
        assert_eq!(
            failure["recoveryRegistry"]["inspect"]["arguments"]["taskId"],
            "original-task"
        );
        assert_eq!(failure["recoveryRegistry"]["inspect"]["executable"], true);
        let bound = failure.clone();
        enrich(&mut failure, &Value::Null);
        assert_eq!(failure, bound);
        let mut recovery_error =
            json!({"code":"failure_recovery_budget_exhausted", "occurrenceId":"new-refusal"});
        enrich(
            &mut recovery_error,
            &json!({"action":"failure_recovery", "taskId":"task", "params":{"occurrenceId":"original-occurrence"}}),
        );
        assert_eq!(
            recovery_error["recoveryRegistry"]["inspect"]["arguments"]["params"]["occurrenceId"],
            "original-occurrence"
        );
    }
    #[test]
    fn transport_invalid_params_does_not_classify_runtime_failure_or_settle_original() {
        let mut runtime = json!({"code":"browser_runtime_database_open_failed", "effectState":"effect_uncertain", "rpcCompatibilityCode":-32602});
        enrich(&mut runtime, &Value::Null);
        assert_eq!(
            runtime["recoveryRegistry"]["failureCategory"],
            "runtime_dependency_unavailable"
        );
        assert_eq!(runtime["effectState"], "effect_uncertain");
        assert_eq!(runtime["rpcCompatibilityCode"], -32602);
        let mut caller = json!({"code":"browser_build_absolute_path_required", "effectState":"effect_uncertain"});
        enrich(&mut caller, &Value::Null);
        assert_eq!(
            caller["recoveryRegistry"]["failureCategory"],
            "invalid_argument_or_contract"
        );
        assert_eq!(caller["effectState"], "effect_uncertain");
    }
    #[test]
    fn capacity_recovers_exact_task_without_inventing_policy_or_freshness() {
        let mut failure =
            json!({"code":"browser_task_operation_capacity_reached","effectState":"no_effect"});
        enrich(
            &mut failure,
            &json!({"taskId":"task-one","requestId":"request-one"}),
        );
        assert_eq!(
            failure["executableNextAction"]["arguments"]["taskId"],
            "task-one"
        );
        assert_eq!(
            failure["recoveryPlan"]["evidence"]["status"],
            "not_observed_by_projection"
        );
        assert_eq!(
            failure["recoveryPlan"]["remediation"][0]["call"]["executable"],
            false
        );
        assert_eq!(failure["effectState"], "no_effect");
    }
    #[test]
    fn registered_contract_preserves_authoritative_owner_plan_action_and_evidence() {
        let owner_plan = json!({"requiredActor":"actual-owner","evidence":{"status":"fresh","observationId":"original"},"remediation":[{"call":{"tool":"owner-specific-recovery"}}]});
        let owner_action =
            json!({"tool":"owner-specific-inspection","arguments":{"original":"retained"}});
        let mut failure = json!({"code":"browser_task_operation_capacity_reached","recoveryPlan":owner_plan,"executableNextAction":owner_action});
        enrich(&mut failure, &json!({"taskId":"task"}));
        assert_eq!(failure["recoveryPlan"], owner_plan);
        assert_eq!(failure["executableNextAction"], owner_action);
        assert_eq!(failure["recoveryRegistry"]["coverage"], "registered");
    }
    #[test]
    fn facade_without_scope_does_not_replace_bound_owner_registry() {
        let mut failure = json!({"code":"browser_task_operation_capacity_reached"});
        enrich(
            &mut failure,
            &json!({"taskId":"original-task","requestId":"original-request"}),
        );
        let original = failure.clone();
        enrich(&mut failure, &Value::Null);
        assert_eq!(failure, original);
    }
    #[test]
    fn unbound_and_unknown_contracts_never_claim_executable_coverage() {
        let mut failure = json!({"code":"browser_task_operation_capacity_reached"});
        enrich(&mut failure, &Value::Null);
        assert_eq!(failure["recoveryPlan"]["inspect"]["executable"], false);
        let mut unknown = json!({"code":"new_unknown_runtime_defect"});
        enrich(&mut unknown, &Value::Null);
        assert_eq!(unknown["recoveryPlan"]["coverage"], "unregistered");
        assert_eq!(unknown["recoveryPlan"]["maximumAutomaticAttempts"], 0);
    }
}
