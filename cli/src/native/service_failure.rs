//! Structured client recourse for Service operation failures.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION: &str =
    "agent-browser.service-failure-recourse.v1";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceFailureAxis {
    ServiceState,
    LifecycleOwner,
    ProfileLease,
    Presentation,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceFailurePhase {
    ProcessMutexWait,
    FileLockWait,
    LaunchAdmission,
    Commit,
    Finalize,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceEffectState {
    NoEffect,
    #[default]
    EffectUncertain,
    VerifiedEffect,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceRetryDisposition {
    DoNotRetry,
    #[default]
    InspectBeforeRetry,
    RetrySameRequest,
    RefreshAccessPlan,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceFailureRecourse {
    pub schema_version: String,
    pub code: String,
    pub axis: ServiceFailureAxis,
    pub phase: ServiceFailurePhase,
    pub effect_state: ServiceEffectState,
    pub retry_disposition: ServiceRetryDisposition,
    pub recommended_action: String,
    pub reuse_allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder_operation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_plan: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub safe_next_actions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hard_stops: Vec<String>,
}

pub fn classify_service_failure(error: &str) -> ServiceFailureRecourse {
    let wait_ms = failure_metadata_value(error, "waited_ms").and_then(|value| value.parse().ok());
    let holder_operation = failure_metadata_value(error, "holder_operation").map(str::to_string);
    if error.starts_with("service_state_stale_revision:") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "service_state_stale_revision".to_string(),
            axis: ServiceFailureAxis::ServiceState,
            phase: ServiceFailurePhase::Commit,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "reload_state_and_replan".to_string(),
            reuse_allowed: false,
            safe_next_actions: vec![
                "inspect_service_job".to_string(),
                "reload_service_state".to_string(),
                "replan_same_intent".to_string(),
            ],
            hard_stops: vec!["blind_retry".to_string()],
            ..ServiceFailureRecourse::default()
        };
    }
    if error.starts_with("service_state_lock_timeout: process mutation lock") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "service_state_lock_timeout".to_string(),
            axis: ServiceFailureAxis::ServiceState,
            phase: ServiceFailurePhase::ProcessMutexWait,
            effect_state: ServiceEffectState::EffectUncertain,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "inspect_job_and_refresh_plan".to_string(),
            reuse_allowed: false,
            wait_ms,
            holder_operation,
            safe_next_actions: vec![
                "inspect_service_job".to_string(),
                "inspect_service_trace".to_string(),
                "refresh_access_plan".to_string(),
            ],
            hard_stops: vec![
                "blind_retry".to_string(),
                "launch_duplicate_profile_lane".to_string(),
            ],
            ..ServiceFailureRecourse::default()
        };
    }

    if error.starts_with("service_state_lock_timeout:") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "service_state_lock_timeout".to_string(),
            axis: ServiceFailureAxis::ServiceState,
            phase: ServiceFailurePhase::FileLockWait,
            effect_state: ServiceEffectState::EffectUncertain,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "inspect_job_and_refresh_plan".to_string(),
            reuse_allowed: false,
            wait_ms,
            holder_operation,
            safe_next_actions: vec![
                "inspect_service_job".to_string(),
                "inspect_service_trace".to_string(),
                "refresh_access_plan".to_string(),
            ],
            hard_stops: vec!["blind_retry".to_string()],
            ..ServiceFailureRecourse::default()
        };
    }

    if error.contains("runtime_lifecycle_existing_owner_requires_explicit_transition") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "runtime_lifecycle_existing_owner_requires_explicit_transition".to_string(),
            axis: ServiceFailureAxis::LifecycleOwner,
            phase: ServiceFailurePhase::LaunchAdmission,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::RefreshAccessPlan,
            recommended_action: "refresh_access_plan".to_string(),
            reuse_allowed: false,
            safe_next_actions: vec![
                "refresh_access_plan".to_string(),
                "inspect_profile_recovery_plan".to_string(),
            ],
            hard_stops: vec![
                "retry_direct_launch".to_string(),
                "launch_duplicate_profile_lane".to_string(),
            ],
            ..ServiceFailureRecourse::default()
        };
    }

    if error
        .starts_with("service_access_plan_request_unavailable:lifecycle_owner_blocks_replacement")
    {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "lifecycle_owner_blocks_replacement".to_string(),
            axis: ServiceFailureAxis::LifecycleOwner,
            phase: ServiceFailurePhase::LaunchAdmission,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::DoNotRetry,
            recommended_action: "inspect_lifecycle_owner".to_string(),
            reuse_allowed: false,
            safe_next_actions: vec![
                "inspect_profile_allocation".to_string(),
                "inspect_lifecycle_owner".to_string(),
                "inspect_profile_recovery_plan".to_string(),
            ],
            hard_stops: vec![
                "retry_direct_launch".to_string(),
                "launch_duplicate_profile_lane".to_string(),
                "force_unlock_or_process_cleanup".to_string(),
            ],
            ..ServiceFailureRecourse::default()
        };
    }

    if error.starts_with("service_access_plan_request_unavailable:foreign_principal_profile_lease")
    {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "foreign_principal_profile_lease".to_string(),
            axis: ServiceFailureAxis::ProfileLease,
            phase: ServiceFailurePhase::LaunchAdmission,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::DoNotRetry,
            recommended_action: "coordinate_with_profile_lease_holder".to_string(),
            reuse_allowed: false,
            safe_next_actions: vec![
                "inspect_profile_lease".to_string(),
                "wait_for_profile_lease_holder".to_string(),
            ],
            hard_stops: vec![
                "borrow_foreign_principal".to_string(),
                "launch_duplicate_profile_lane".to_string(),
            ],
            ..ServiceFailureRecourse::default()
        };
    }

    for code in [
        "existing_session_profile_identity_unproven",
        "existing_session_profile_identity_inconsistent",
    ] {
        if error.contains(code) {
            return ServiceFailureRecourse {
                schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
                code: code.to_string(),
                axis: ServiceFailureAxis::ProfileLease,
                phase: ServiceFailurePhase::LaunchAdmission,
                effect_state: ServiceEffectState::NoEffect,
                retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
                recommended_action: "acquire_profile".to_string(),
                reuse_allowed: false,
                safe_next_actions: vec![
                    "inspect_profile_lease".to_string(),
                    "acquire_profile".to_string(),
                    "inspect_profile_recovery_plan".to_string(),
                ],
                hard_stops: vec![
                    "blind_retry".to_string(),
                    "launch_duplicate_profile_lane".to_string(),
                ],
                ..ServiceFailureRecourse::default()
            };
        }
    }

    ServiceFailureRecourse {
        schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
        code: "service_operation_failed".to_string(),
        recommended_action: "inspect_failure".to_string(),
        hard_stops: vec!["blind_retry".to_string()],
        ..ServiceFailureRecourse::default()
    }
}

fn failure_metadata_value<'a>(error: &'a str, key: &str) -> Option<&'a str> {
    error
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(&format!("{key}=")))
        .filter(|value| !value.is_empty())
}

/// Add machine-readable recourse to a failed Service response while preserving
/// the legacy error field. Successful responses and already-decorated failures
/// are left unchanged.
pub fn attach_service_failure_recourse(response: &mut Value) {
    if response.get("success").and_then(Value::as_bool) != Some(false)
        || response.get("failure").is_some()
    {
        return;
    }
    let Some(error) = response.get("error").and_then(Value::as_str) else {
        return;
    };
    let mut recourse = classify_service_failure(error);
    recourse.job_id = response
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string);
    if let Ok(value) = serde_json::to_value(recourse) {
        response["failure"] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn process_mutation_lock_timeout_requires_inspection_before_retry() {
        let recourse =
            classify_service_failure("service_state_lock_timeout: process mutation lock");

        assert_eq!(recourse.code, "service_state_lock_timeout");
        assert_eq!(recourse.axis, ServiceFailureAxis::ServiceState);
        assert_eq!(recourse.phase, ServiceFailurePhase::ProcessMutexWait);
        assert_eq!(recourse.effect_state, ServiceEffectState::EffectUncertain);
        assert_eq!(
            recourse.retry_disposition,
            ServiceRetryDisposition::InspectBeforeRetry
        );
        assert_eq!(recourse.recommended_action, "inspect_job_and_refresh_plan");
        assert!(!recourse.reuse_allowed);
        assert!(recourse.hard_stops.contains(&"blind_retry".to_string()));
        assert!(recourse
            .hard_stops
            .contains(&"launch_duplicate_profile_lane".to_string()));
    }

    #[test]
    fn stale_revision_fails_before_effect_and_requires_replanning() {
        let recourse =
            classify_service_failure("service_state_stale_revision: expected=4; actual=5");

        assert_eq!(recourse.code, "service_state_stale_revision");
        assert_eq!(recourse.phase, ServiceFailurePhase::Commit);
        assert_eq!(recourse.effect_state, ServiceEffectState::NoEffect);
        assert_eq!(recourse.recommended_action, "reload_state_and_replan");
        assert!(!recourse.reuse_allowed);
    }

    #[test]
    fn file_lock_timeout_requires_inspection_before_retry() {
        let recourse =
            classify_service_failure("service_state_lock_timeout: file lock; waited_ms=21");

        assert_eq!(recourse.code, "service_state_lock_timeout");
        assert_eq!(recourse.axis, ServiceFailureAxis::ServiceState);
        assert_eq!(recourse.phase, ServiceFailurePhase::FileLockWait);
        assert_eq!(recourse.effect_state, ServiceEffectState::EffectUncertain);
        assert_eq!(
            recourse.retry_disposition,
            ServiceRetryDisposition::InspectBeforeRetry
        );
        assert_eq!(recourse.recommended_action, "inspect_job_and_refresh_plan");
        assert!(!recourse.reuse_allowed);
        assert_eq!(recourse.wait_ms, Some(21));
        assert!(recourse.hard_stops.contains(&"blind_retry".to_string()));
    }

    #[test]
    fn process_lock_timeout_reports_safe_holder_metadata() {
        let recourse = classify_service_failure(
            "service_state_lock_timeout: process mutation lock; waited_ms=1001; holder_operation=mutate",
        );

        assert_eq!(recourse.wait_ms, Some(1001));
        assert_eq!(recourse.holder_operation.as_deref(), Some("mutate"));
    }

    #[test]
    fn lifecycle_owner_blocker_refreshes_access_plan_without_inventing_reuse() {
        let recourse = classify_service_failure(
            "runtime_lifecycle_existing_owner_requires_explicit_transition",
        );

        assert_eq!(
            recourse.code,
            "runtime_lifecycle_existing_owner_requires_explicit_transition"
        );
        assert_eq!(recourse.axis, ServiceFailureAxis::LifecycleOwner);
        assert_eq!(recourse.phase, ServiceFailurePhase::LaunchAdmission);
        assert_eq!(recourse.effect_state, ServiceEffectState::NoEffect);
        assert_eq!(
            recourse.retry_disposition,
            ServiceRetryDisposition::RefreshAccessPlan
        );
        assert_eq!(recourse.recommended_action, "refresh_access_plan");
        assert!(!recourse.reuse_allowed);
        assert!(recourse.recovery_plan.is_none());
        assert!(recourse
            .hard_stops
            .contains(&"launch_duplicate_profile_lane".to_string()));
    }

    #[test]
    fn access_plan_lifecycle_blocker_is_terminal_without_duplicate_launch() {
        let recourse = classify_service_failure(
            "service_access_plan_request_unavailable:lifecycle_owner_blocks_replacement",
        );

        assert_eq!(recourse.code, "lifecycle_owner_blocks_replacement");
        assert_eq!(recourse.axis, ServiceFailureAxis::LifecycleOwner);
        assert_eq!(recourse.effect_state, ServiceEffectState::NoEffect);
        assert_eq!(
            recourse.retry_disposition,
            ServiceRetryDisposition::DoNotRetry
        );
        assert!(!recourse.reuse_allowed);
        assert!(recourse
            .hard_stops
            .contains(&"launch_duplicate_profile_lane".to_string()));
    }

    #[test]
    fn foreign_profile_lease_never_becomes_reuse_authority() {
        let recourse = classify_service_failure(
            "service_access_plan_request_unavailable:foreign_principal_profile_lease",
        );

        assert_eq!(recourse.axis, ServiceFailureAxis::ProfileLease);
        assert_eq!(recourse.effect_state, ServiceEffectState::NoEffect);
        assert_eq!(
            recourse.retry_disposition,
            ServiceRetryDisposition::DoNotRetry
        );
        assert!(!recourse.reuse_allowed);
        assert!(recourse
            .hard_stops
            .contains(&"borrow_foreign_principal".to_string()));
    }

    #[test]
    fn profile_identity_admission_failure_routes_to_profile_acquisition() {
        for error in [
            "existing_session_profile_identity_unproven",
            "existing_session_profile_identity_inconsistent",
        ] {
            let recourse = classify_service_failure(error);

            assert_eq!(recourse.code, error);
            assert_eq!(recourse.axis, ServiceFailureAxis::ProfileLease);
            assert_eq!(recourse.phase, ServiceFailurePhase::LaunchAdmission);
            assert_eq!(recourse.effect_state, ServiceEffectState::NoEffect);
            assert_eq!(
                recourse.retry_disposition,
                ServiceRetryDisposition::InspectBeforeRetry
            );
            assert_eq!(recourse.recommended_action, "acquire_profile");
            assert!(recourse
                .safe_next_actions
                .contains(&"acquire_profile".to_string()));
            assert!(recourse.hard_stops.contains(&"blind_retry".to_string()));
        }
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
}
