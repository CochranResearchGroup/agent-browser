//! CLI transport adapters for provider-free Service failure recourse.

use serde_json::Value;

pub(crate) use agent_browser_service_model::{
    child_access_failure_evidence, operator_focus_failure_code, profile_child_denial_error,
};
pub use agent_browser_service_model::{
    classify_service_failure, ServiceEffectState, ServiceFailureAxis, ServiceFailurePhase,
    ServiceFailureRecourse, ServiceRetryDisposition,
};

/// Add machine-readable recourse to a failed Service response while preserving
/// the legacy error field. Successful responses and already-decorated failures
/// are left unchanged.
pub fn attach_service_failure_recourse(response: &mut Value) {
    if response.get("success").and_then(Value::as_bool) != Some(false)
        || response.get("failure").is_some()
    {
        return;
    }
    let Some(error) = response
        .get("error")
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return;
    };
    let mut recourse = classify_service_failure(&error);
    recourse.job_id = response
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string);
    if let Ok(value) = serde_json::to_value(recourse) {
        response["failure"] = value;
    }
}

/// Qualify pre-effect controller-takeover fence rejection using the action
/// boundary. The same fence error from a compound input action stays uncertain.
pub fn attach_service_failure_recourse_for_action(response: &mut Value, action: &str) {
    attach_service_failure_recourse(response);
    if action == "service_controller_lease_takeover"
        && response.get("success").and_then(Value::as_bool) == Some(false)
        && response.get("error").and_then(Value::as_str)
            == Some("desktop_input_route_fence_contended")
    {
        if let Some(failure) = response.get_mut("failure") {
            if let Ok(mut recourse) =
                serde_json::from_value::<ServiceFailureRecourse>(failure.clone())
            {
                recourse.code = "desktop_input_route_fence_contended".to_string();
                recourse.axis = ServiceFailureAxis::ServiceState;
                recourse.phase = ServiceFailurePhase::FileLockWait;
                recourse.effect_state = ServiceEffectState::NoEffect;
                recourse.retry_disposition = ServiceRetryDisposition::InspectBeforeRetry;
                recourse.recommended_action = "inspect_desktop_controller_contention".to_string();
                recourse.reuse_allowed = false;
                recourse.safe_next_actions = vec![
                    "inspect_service_status".to_string(),
                    "inspect_service_trace".to_string(),
                ];
                recourse.hard_stops = vec!["blind_retry".to_string()];
                if let Ok(value) = serde_json::to_value(recourse) {
                    *failure = value;
                }
            }
        }
        return;
    }
    if !matches!(
        action,
        "service_profiles" | "service_sessions" | "service_browsers"
    ) {
        return;
    }
    let Some(error) = response
        .get("error")
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return;
    };
    let Some(failure) = response.get_mut("failure") else {
        return;
    };
    let Ok(mut recourse) = serde_json::from_value::<ServiceFailureRecourse>(failure.clone()) else {
        return;
    };
    if let Some(code) = error.split(':').next().filter(|code| {
        !code.is_empty()
            && code
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    }) {
        recourse.code = code.to_string();
    }
    recourse.axis = ServiceFailureAxis::ServiceState;
    recourse.phase = ServiceFailurePhase::Finalize;
    recourse.effect_state = ServiceEffectState::NoEffect;
    recourse.retry_disposition = ServiceRetryDisposition::InspectBeforeRetry;
    recourse.recommended_action = "inspect_service_inventory_state".to_string();
    recourse.reuse_allowed = false;
    recourse.safe_next_actions = vec![
        "inspect_service_status".to_string(),
        "inspect_service_inventory_state".to_string(),
    ];
    recourse.hard_stops = Vec::new();
    if let Ok(value) = serde_json::to_value(recourse) {
        *failure = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn controller_takeover_fence_recourse_is_action_and_error_bound() {
        for action in [
            "service_controller_lease_takeover",
            "desktop_input",
            "view_focus",
        ] {
            for error in [
                "desktop_input_route_fence_contended",
                "wrapper: desktop_input_route_fence_contended",
                "desktop_input_effect_uncertain",
            ] {
                let mut response = json!({"success":false,"id":"fence-job","error":error});
                attach_service_failure_recourse_for_action(&mut response, action);
                let qualified = action == "service_controller_lease_takeover"
                    && error == "desktop_input_route_fence_contended";
                assert_eq!(
                    response["failure"]["effectState"],
                    if qualified {
                        "no_effect"
                    } else {
                        "effect_uncertain"
                    }
                );
                assert_eq!(
                    response["failure"]["retryDisposition"],
                    "inspect_before_retry"
                );
                assert_eq!(response["failure"]["reuseAllowed"], false);
                assert_eq!(response["failure"]["jobId"], "fence-job");
                if qualified {
                    assert_eq!(response["failure"]["code"], error);
                    assert_eq!(response["failure"]["phase"], "file_lock_wait");
                    assert_eq!(
                        response["failure"]["recommendedAction"],
                        "inspect_desktop_controller_contention"
                    );
                }
            }
        }
    }

    #[test]
    fn read_only_inventory_failure_reports_no_effect() {
        for action in ["service_profiles", "service_sessions", "service_browsers"] {
            let mut response = json!({
                "success": false,
                "id": format!("request-{action}"),
                "error": "protected_browser_owner_observation_invalid"
            });
            attach_service_failure_recourse_for_action(&mut response, action);
            assert_eq!(response["failure"]["effectState"], "no_effect");
            assert_eq!(
                response["failure"]["code"],
                "protected_browser_owner_observation_invalid"
            );
            assert_eq!(
                response["failure"]["recommendedAction"],
                "inspect_service_inventory_state"
            );
        }

        let mut malformed = json!({
            "success": false,
            "error": "Invalid serviceState: missing profiles"
        });
        attach_service_failure_recourse_for_action(&mut malformed, "service_profiles");
        assert_eq!(malformed["failure"]["code"], "service_operation_failed");
        assert_eq!(malformed["failure"]["effectState"], "no_effect");
    }

    #[test]
    fn viewport_lock_failure_response_keeps_error_and_adds_client_recourse() {
        let mut response = json!({
            "id": "viewport-job-1",
            "success": false,
            "error": "service_state_lock_timeout: process mutation lock"
        });

        attach_service_failure_recourse(&mut response);

        assert_eq!(
            response["error"],
            "service_state_lock_timeout: process mutation lock"
        );
        assert_eq!(response["failure"]["code"], "service_state_lock_timeout");
        assert_eq!(response["failure"]["phase"], "process_mutex_wait");
        assert_eq!(response["failure"]["effectState"], "effect_uncertain");
        assert_eq!(
            response["failure"]["retryDisposition"],
            "inspect_before_retry"
        );
        assert_eq!(response["failure"]["reuseAllowed"], false);
        assert_eq!(response["failure"]["jobId"], "viewport-job-1");
    }

    #[test]
    fn desktop_interaction_authority_failure_response_keeps_no_effect_recourse() {
        let mut response = json!({
            "id": "interaction-job-1",
            "success": false,
            "error": "desktop_interaction_authority_required: current machine controller authority was not proven"
        });

        attach_service_failure_recourse(&mut response);

        assert_eq!(
            response["error"],
            "desktop_interaction_authority_required: current machine controller authority was not proven"
        );
        assert_eq!(
            response["failure"]["code"],
            "desktop_interaction_authority_required"
        );
        assert_eq!(response["failure"]["axis"], "profile_access");
        assert_eq!(response["failure"]["phase"], "child_admission");
        assert_eq!(response["failure"]["effectState"], "no_effect");
        assert_eq!(
            response["failure"]["retryDisposition"],
            "inspect_before_retry"
        );
        assert_eq!(response["failure"]["reuseAllowed"], false);
        assert_eq!(response["failure"]["jobId"], "interaction-job-1");
    }
}
