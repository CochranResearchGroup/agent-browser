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
        let fragment: Value = serde_json::from_str(include_str!(
            "../../../../docs/dev/contracts/public-error-recovery-client-dashboard.v1.json"
        ))
        .expect("validated client recovery fragment must parse");
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
    let source = registry();
    let Some(entry) = source["codes"].get(&code) else {
        let plan = json!({
            "schemaVersion":"agent-browser.public-error-recovery.v1",
            "coverage":"unregistered", "code":code,
            "requiredActor":"maintainer", "maximumAutomaticAttempts":0,
            "evidence":{"status":"unavailable","reason":"specific_check_contract_not_registered"},
            "escalation":{"operation":"retain_sanitized_failure_packet",
                "include":["code","occurrenceId","originalRequestId","taskId","effectState","logging"],
                "exclude":["credentials","cookies","pageContent","rawProviderErrors"]},
            "resumeCondition":"The specific defect or prerequisite is resolved, original effects are reconciled, and the same task is freshly observed; independent safe reads may continue"
        });
        publish_plan(failure, plan);
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
    let plan = json!({
        "schemaVersion":"agent-browser.public-error-recovery.v1",
        "coverage":"registered", "code":code, "diagnosis":diagnosis,
        "rationale":recipe["rationale"], "requiredActor":recipe["requiredActor"],
        "policySource":{"kind":"owning_runtime_check","path":entry["policySource"],"effectiveConfiguration":"not_supplied"},
        "evidence":{"status":"not_observed_by_projection","required":recipe["evidenceRequirement"]},
        "inspect":inspect,"remediation":remediation,"verification":verification,
        "maximumAutomaticAttempts":recipe["maximumAutomaticAttempts"],"budgetScope":"single_recipe_cycle",
        "resumeCondition":recipe["resumeCondition"]
    });
    publish_plan(failure, plan);
    if failure["executableNextAction"].is_null() && inspect["executable"] == true {
        failure["executableNextAction"] = inspect;
    }
}

fn publish_plan(failure: &mut Value, plan: Value) {
    let existing = &failure["recoveryRegistry"];
    if !(existing["coverage"] == "registered" && existing["code"] == plan["code"]) {
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
