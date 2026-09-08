//! Structured client recourse for Service operation failures.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION: &str =
    "agent-browser.service-failure-recourse.v1";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceFailureAxis {
    Request,
    ServiceState,
    LifecycleOwner,
    ProfileLease,
    ProfileAccess,
    Presentation,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceFailurePhase {
    IngressValidation,
    ProcessMutexWait,
    FileLockWait,
    LaunchAdmission,
    ChildAdmission,
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
    pub subject: Option<Value>,
    pub missing_permission: Option<String>,
    pub executable_next_action: Option<Value>,
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

// The native command transport carries errors as strings. Keep the legacy
// message and attach only strictly decoded, privacy-bounded decision evidence.
const CHILD_EVIDENCE_SEPARATOR: &str = "; childAccessEvidence=";

pub(crate) fn profile_child_denial_error(
    reason: &str,
    evidence: Option<&super::service_profile_access_policy::ProfileChildAccessEvidence>,
) -> String {
    let message = format!("profile child access denied: {reason}");
    evidence
        .and_then(|value| serde_json::to_string(value).ok())
        .map(|encoded| format!("{message}{CHILD_EVIDENCE_SEPARATOR}{encoded}"))
        .unwrap_or(message)
}

/// Only the typed child decision evidence is eligible for journal projection.
/// Other recourse subjects may contain fields outside the journal privacy bound.
pub(crate) fn child_access_failure_evidence(failure: &ServiceFailureRecourse) -> Option<Value> {
    if !matches!(
        failure.code.as_str(),
        "profile_child_subject_mismatch"
            | "profile_child_permission_not_inherited"
            | "profile_child_owner_connection_still_active"
            | "profile_child_explicit_reconnect_required"
    ) {
        return None;
    }
    super::service_profile_access_policy::ProfileChildAccessEvidence::decode(
        failure.subject.clone()?,
    )
    .and_then(|evidence| serde_json::to_value(evidence).ok())
}

pub(crate) fn operator_focus_failure_code(error: &str) -> Option<&str> {
    let code = error.split(':').next()?;
    matches!(
        code,
        "operator_focus_authority_required"
            | "operator_focus_authority_unproven"
            | "operator_focus_binding_required"
            | "operator_focus_binding_changed"
            | "operator_focus_profile_policy_denied"
            | "operator_focus_controller_required"
            | "operator_focus_action_mismatch"
            | "operator_focus_browser_missing"
            | "operator_focus_profile_missing"
            | "operator_focus_route_missing"
            | "operator_focus_process_missing"
            | "operator_focus_proof_encoding_failed"
            | "operator_controller_authority_required"
    )
    .then_some(code)
}

pub fn classify_service_failure(error: &str) -> ServiceFailureRecourse {
    let download_code = match error {
        "Download was canceled" => Some("download_canceled"),
        "Downloaded file not found at captured path" => Some("download_completion_path_missing"),
        "file_transfer download click timed out" => Some("download_click_uncertain"),
        _ => error.split_once(':').map(|(code, _)| code).filter(|code| {
            matches!(
                *code,
                "download_target_unproven"
                    | "download_event_unproven"
                    | "download_event_ambiguous"
                    | "download_canceled"
                    | "download_completion_path_missing"
                    | "download_source_identity_unproven"
                    | "download_artifact_path_unsafe"
                    | "download_artifact_limit_exceeded"
                    | "download_destination_exists"
                    | "download_artifact_delivery_failed"
                    | "download_events_unavailable"
                    | "download_click_uncertain"
                    | "download_subscription_failed"
                    | "download_subscription_cleanup_failed"
                    | "download_launch_policy_unavailable"
            )
        }),
    };
    if let Some(code) = download_code {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.into(),
            code: code.into(),
            // Missing/canceled download events do not establish an owner failure.
            // Retain the specific download code without inventing a lease cause.
            axis: match code {
                "download_target_unproven" | "download_source_identity_unproven" => {
                    ServiceFailureAxis::LifecycleOwner
                }
                _ => ServiceFailureAxis::Unknown,
            },
            phase: ServiceFailurePhase::Finalize,
            // A combined transfer may already have uploaded or clicked. Never
            // advertise a safe blind replay after missing completion evidence.
            effect_state: ServiceEffectState::EffectUncertain,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "inspect_download_capture".into(),
            safe_next_actions: vec![
                "inspect_service_trace".into(),
                "verify_browser_download_policy_and_artifact_identity".into(),
            ],
            hard_stops: vec![
                "blind_retry".into(),
                "overwrite_unknown_context_download_policy".into(),
            ],
            ..ServiceFailureRecourse::default()
        };
    }
    if let Some(code) = operator_focus_failure_code(error) {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: code.to_string(),
            axis: ServiceFailureAxis::ProfileAccess,
            phase: ServiceFailurePhase::ChildAdmission,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "inspect_operator_focus_authority".to_string(),
            safe_next_actions: vec![
                "inspect_service_trace".into(),
                "inspect_operator_focus_authority".into(),
            ],
            hard_stops: vec!["blind_retry".into(), "impersonate_profile_subject".into()],
            ..ServiceFailureRecourse::default()
        };
    }
    for code in [
        "display_access_grant_failed",
        "display_access_grant_timeout",
    ] {
        if error.split(':').next() == Some(code) {
            return ServiceFailureRecourse {
                schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
                code: code.to_string(),
                axis: ServiceFailureAxis::Presentation,
                phase: ServiceFailurePhase::LaunchAdmission,
                // A helper timeout may follow a completed X access grant even
                // when browser launch and reservation rollback never progressed.
                effect_state: ServiceEffectState::EffectUncertain,
                retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
                recommended_action: "inspect_privileged_display_grant".to_string(),
                safe_next_actions: vec![
                    "inspect_service_trace".to_string(),
                    "inspect_privileged_display_grant".to_string(),
                ],
                hard_stops: vec![
                    "blind_retry".to_string(),
                    "disable_runtime_isolation".to_string(),
                ],
                ..ServiceFailureRecourse::default()
            };
        }
    }
    for code in [
        "route_display_owner_unproven",
        "route_display_owner_mismatch",
        "route_display_server_unavailable",
        "route_display_server_ambiguous",
    ] {
        if error.split(':').next() == Some(code) {
            return ServiceFailureRecourse {
                schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
                code: code.to_string(),
                axis: ServiceFailureAxis::Presentation,
                phase: ServiceFailurePhase::LaunchAdmission,
                effect_state: ServiceEffectState::EffectUncertain,
                retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
                recommended_action: "repair_route_display_binding".to_string(),
                safe_next_actions: vec![
                    "inspect_service_trace".to_string(),
                    "inspect_route_display_owner".to_string(),
                ],
                hard_stops: vec![
                    "blind_retry".to_string(),
                    "grant_access_to_foreign_display".to_string(),
                ],
                ..ServiceFailureRecourse::default()
            };
        }
    }
    for code in [
        "retained_browser_close_identity_unproven",
        "browser_terminal_close_unproven",
    ] {
        if error.starts_with(code) {
            let pre_effect = code == "retained_browser_close_identity_unproven";
            return ServiceFailureRecourse {
                schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
                code: code.to_string(),
                axis: ServiceFailureAxis::LifecycleOwner,
                phase: if pre_effect {
                    ServiceFailurePhase::ChildAdmission
                } else {
                    ServiceFailurePhase::Finalize
                },
                effect_state: if pre_effect {
                    ServiceEffectState::NoEffect
                } else {
                    ServiceEffectState::EffectUncertain
                },
                retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
                recommended_action: "inspect_lifecycle_owner".to_string(),
                safe_next_actions: vec![
                    "inspect_service_trace".to_string(),
                    "inspect_exact_process_and_profile_lock".to_string(),
                ],
                hard_stops: vec![
                    "blind_retry".to_string(),
                    "launch_duplicate_profile_lane".to_string(),
                ],
                ..ServiceFailureRecourse::default()
            };
        }
    }
    if error.contains("service_browser_close_authority_denied") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "service_browser_close_authority_denied".to_string(),
            axis: ServiceFailureAxis::ProfileAccess,
            phase: ServiceFailurePhase::ChildAdmission,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "inspect_profile_access_policy".to_string(),
            missing_permission: Some("full_shutdown".to_string()),
            hard_stops: vec!["blind_retry".to_string()],
            ..ServiceFailureRecourse::default()
        };
    }
    if error.split(':').next() == Some("explicit_profile_conflicts_with_current_owner") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "explicit_profile_conflicts_with_current_owner".to_string(),
            axis: ServiceFailureAxis::ProfileLease,
            phase: ServiceFailurePhase::LaunchAdmission,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "compare_requested_profile_to_current_owner".to_string(),
            safe_next_actions: vec![
                "inspect_service_trace".to_string(),
                "compare_requested_profile_to_current_owner".to_string(),
            ],
            hard_stops: vec![
                "blind_retry".to_string(),
                "launch_duplicate_profile_lane".to_string(),
            ],
            ..ServiceFailureRecourse::default()
        };
    }
    if let Some((code, _)) = error.split_once(':') {
        if matches!(
            code,
            "tab_close_invalid_selector"
                | "service_tab_profile_selector_conflict"
                | "service_tab_route_mismatch"
                | "service_tab_target_selector_conflict"
                | "service_tab_target_unproven"
                | "tab_close_selector_conflict"
                | "tab_close_target_unproven"
                | "tab_close_target_missing"
                | "tab_close_preflight_failed"
                | "tab_close_effect_uncertain"
                | "tab_cleanup_pending"
        ) {
            return ServiceFailureRecourse {
                schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
                code: code.to_string(),
                axis: ServiceFailureAxis::LifecycleOwner,
                phase: if matches!(code, "tab_close_effect_uncertain" | "tab_cleanup_pending") {
                    ServiceFailurePhase::Finalize
                } else {
                    ServiceFailurePhase::ChildAdmission
                },
                effect_state: if matches!(
                    code,
                    "tab_close_effect_uncertain" | "tab_cleanup_pending"
                ) {
                    ServiceEffectState::EffectUncertain
                } else {
                    ServiceEffectState::NoEffect
                },
                retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
                recommended_action: "inspect_service_trace".to_string(),
                safe_next_actions: vec![
                    "inspect_service_trace".to_string(),
                    if code == "service_tab_profile_selector_conflict" {
                        "compare_requested_profile_to_current_owner"
                    } else if code == "service_tab_route_mismatch" {
                        "compare_requested_session_to_current_handle"
                    } else if matches!(
                        code,
                        "service_tab_target_selector_conflict" | "service_tab_target_unproven"
                    ) {
                        "compare_requested_target_to_current_handle"
                    } else {
                        "inspect_exact_target_cleanup"
                    }
                    .to_string(),
                ],
                hard_stops: vec![
                    "blind_retry".to_string(),
                    "close_active_or_peer_tab_as_fallback".to_string(),
                ],
                ..ServiceFailureRecourse::default()
            };
        }
    }
    // These recovery guards run after child authorization and before target
    // attachment. Transport/bootstrap failures may already have attached CDP;
    // keep them uncertain rather than declaring the whole recovery effect-free.
    if let Some((code, _)) = error.split_once(':') {
        if matches!(
            code,
            "service_tab_recovery_owner_missing"
                | "service_tab_recovery_identity_mismatch"
                | "service_tab_recovery_process_unproven"
                | "service_tab_recovery_endpoint_unproven"
                | "service_tab_recovery_target_missing"
                | "service_tab_recovery_attach_failed"
        ) {
            return ServiceFailureRecourse {
                schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
                code: code.to_string(),
                axis: ServiceFailureAxis::LifecycleOwner,
                phase: ServiceFailurePhase::ChildAdmission,
                effect_state: if code == "service_tab_recovery_attach_failed" {
                    ServiceEffectState::EffectUncertain
                } else {
                    ServiceEffectState::NoEffect
                },
                retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
                recommended_action: "inspect_service_trace".to_string(),
                executable_next_action: Some(serde_json::json!({
                    "action": "service_trace", "executable": true,
                    "request": {"action": "service_trace"}
                })),
                safe_next_actions: vec![
                    "inspect_service_trace".to_string(),
                    "inspect_profile_recovery_plan".to_string(),
                ],
                hard_stops: vec![
                    "blind_retry".to_string(),
                    "replace_original_service_tab".to_string(),
                ],
                ..ServiceFailureRecourse::default()
            };
        }
    }
    // These exact compatibility messages originate from the child authority
    // guard before child reconnect or the guarded browser operation. Never
    // infer no-effect from a substring or an unrecognized policy reason.
    let (child_message, child_evidence) = error
        .split_once(CHILD_EVIDENCE_SEPARATOR)
        .and_then(|(message, encoded)| {
            let value = serde_json::from_str(encoded).ok()?;
            let evidence =
                super::service_profile_access_policy::ProfileChildAccessEvidence::decode(value)?;
            Some((message, serde_json::to_value(evidence).ok()))
        })
        .unwrap_or((error, None));
    let child_denial = match child_message {
        "profile child access record is missing" => Some((
            "profile_child_access_record_missing",
            "inspect_service_trace",
        )),
        "profile child access denied: subject_mismatch" => Some((
            "profile_child_subject_mismatch",
            "use_own_service_tab_handle",
        )),
        "profile child access denied: permission_not_inherited" => Some((
            "profile_child_permission_not_inherited",
            "inspect_profile_access_policy",
        )),
        "profile child access denied: owner_connection_still_active" => Some((
            "profile_child_owner_connection_still_active",
            "use_owner_connection_or_wait",
        )),
        "profile child access denied: explicit_reconnect_required" => Some((
            "profile_child_explicit_reconnect_required",
            "reconnect_owned_service_tab_handle",
        )),
        _ => None,
    };
    if let Some((code, action)) = child_denial {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: code.to_string(),
            axis: ServiceFailureAxis::ProfileAccess,
            phase: ServiceFailurePhase::ChildAdmission,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::DoNotRetry,
            recommended_action: action.to_string(),
            subject: child_evidence,
            reuse_allowed: false,
            safe_next_actions: if action == "inspect_service_trace" {
                vec![action.to_string()]
            } else {
                vec![action.to_string(), "inspect_service_trace".to_string()]
            },
            hard_stops: vec![
                "blind_retry".to_string(),
                "impersonate_child_owner".to_string(),
            ],
            ..ServiceFailureRecourse::default()
        };
    }
    // Hint mismatch precedes attachment; an existing-owner refusal can follow
    // CDP attachment. Both require inspection, while effect certainty differs.
    let orphan_guard = error.split_once(':').map(|(code, _)| code).filter(|code| {
        matches!(
            *code,
            "runtime_handoff_orphan_browser_hint_mismatch" | "runtime_handoff_orphan_owner_present"
        )
    });
    if let Some(code) = orphan_guard {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: code.to_string(),
            axis: ServiceFailureAxis::LifecycleOwner,
            phase: ServiceFailurePhase::LaunchAdmission,
            effect_state: if code == "runtime_handoff_orphan_browser_hint_mismatch" {
                ServiceEffectState::NoEffect
            } else {
                ServiceEffectState::EffectUncertain
            },
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "inspect_profile_recovery_plan".to_string(),
            reuse_allowed: false,
            safe_next_actions: vec![
                "inspect_profile_recovery_plan".to_string(),
                "inspect_service_job".to_string(),
                "inspect_service_trace".to_string(),
            ],
            hard_stops: vec![
                "blind_retry".to_string(),
                "replace_handoff_identity".to_string(),
                "reopen_without_operator_confirmation".to_string(),
            ],
            ..ServiceFailureRecourse::default()
        };
    }
    let wait_ms = failure_metadata_value(error, "waited_ms").and_then(|value| value.parse().ok());
    let holder_operation = failure_metadata_value(error, "holder_operation").map(str::to_string);
    let route_bound_blocker_code = failure_metadata_value(error, "route_bound_blocker_code");
    let route_bound_compensation_state =
        failure_metadata_value(error, "route_bound_compensation_state");
    if let (Some(code), Some(compensation_state)) =
        (route_bound_blocker_code, route_bound_compensation_state)
    {
        let rollback_complete = compensation_state == "rolled_back";
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: code.to_string(),
            axis: ServiceFailureAxis::Presentation,
            phase: ServiceFailurePhase::Finalize,
            effect_state: if rollback_complete {
                ServiceEffectState::NoEffect
            } else {
                ServiceEffectState::EffectUncertain
            },
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: if rollback_complete {
                "address_blocker_then_retry".to_string()
            } else {
                "inspect_job_and_route_cleanup".to_string()
            },
            reuse_allowed: false,
            safe_next_actions: vec![
                "inspect_service_job".to_string(),
                "inspect_service_trace".to_string(),
            ],
            hard_stops: vec!["blind_retry".to_string()],
            ..ServiceFailureRecourse::default()
        };
    }
    if error == "Control queue is full" {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "control_queue_full".to_string(),
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::RetrySameRequest,
            recommended_action: "retry_after_queue_capacity".to_string(),
            reuse_allowed: true,
            safe_next_actions: vec![
                "inspect_service_status".to_string(),
                "retry_same_request".to_string(),
            ],
            ..ServiceFailureRecourse::default()
        };
    }
    if error.contains("Control plane worker is stopped") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "control_plane_worker_stopped".to_string(),
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "inspect_service_status".to_string(),
            reuse_allowed: false,
            safe_next_actions: vec!["inspect_service_status".to_string()],
            hard_stops: vec!["blind_retry".to_string()],
            ..ServiceFailureRecourse::default()
        };
    }
    if error.contains("cancelled before dispatch") || error == "Cancelled by operator" {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "service_job_cancelled".to_string(),
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::DoNotRetry,
            recommended_action: "inspect_service_job".to_string(),
            reuse_allowed: false,
            safe_next_actions: vec!["inspect_service_job".to_string()],
            ..ServiceFailureRecourse::default()
        };
    }
    if error.contains("cancelled while running") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "service_job_cancelled".to_string(),
            effect_state: ServiceEffectState::EffectUncertain,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "inspect_service_job_and_trace".to_string(),
            reuse_allowed: false,
            safe_next_actions: vec![
                "inspect_service_job".to_string(),
                "inspect_service_trace".to_string(),
            ],
            hard_stops: vec!["blind_retry".to_string()],
            ..ServiceFailureRecourse::default()
        };
    }
    if error.contains("timed out") || error.contains("exceeded its persisted") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "service_job_timed_out".to_string(),
            effect_state: ServiceEffectState::EffectUncertain,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "inspect_service_job_and_trace".to_string(),
            reuse_allowed: false,
            safe_next_actions: vec![
                "inspect_service_job".to_string(),
                "inspect_service_trace".to_string(),
            ],
            hard_stops: vec!["blind_retry".to_string()],
            ..ServiceFailureRecourse::default()
        };
    }
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
        "service_access_plan_route_browser_conflict",
        "service_access_plan_route_session_conflict",
    ] {
        if error.starts_with(code) {
            return ServiceFailureRecourse {
                schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
                code: code.to_string(),
                axis: ServiceFailureAxis::ProfileLease,
                phase: ServiceFailurePhase::LaunchAdmission,
                effect_state: ServiceEffectState::NoEffect,
                retry_disposition: ServiceRetryDisposition::RefreshAccessPlan,
                recommended_action: "refresh_access_plan_and_use_exact_route".to_string(),
                reuse_allowed: false,
                executable_next_action: Some(serde_json::json!({
                    "action": "service_access_plan",
                    "executable": true,
                    "request": { "action": "service_access_plan" },
                })),
                safe_next_actions: vec![
                    "refresh_access_plan".to_string(),
                    "submit_exact_planned_route".to_string(),
                ],
                hard_stops: vec![
                    "blind_retry".to_string(),
                    "launch_duplicate_profile_lane".to_string(),
                ],
                ..ServiceFailureRecourse::default()
            };
        }
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
                recommended_action: "inspect_profile_recovery_plan".to_string(),
                reuse_allowed: false,
                subject: Some(serde_json::json!({
                    "subjectId": null,
                    "assurance": "unknown",
                })),
                missing_permission: Some("lifecycle_manage".to_string()),
                executable_next_action: Some(serde_json::json!({
                    "action": "service_profile_recovery_plan",
                    "executable": true,
                    "request": { "action": "service_profile_recovery_plan" },
                })),
                safe_next_actions: vec![
                    "inspect_profile_lease".to_string(),
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
    fn download_observation_failure_does_not_invent_an_ownership_denial() {
        for code in [
            "download_events_unavailable",
            "download_event_unproven",
            "download_event_ambiguous",
            "download_canceled",
            "download_completion_path_missing",
            "download_artifact_path_unsafe",
            "download_destination_exists",
            "download_artifact_delivery_failed",
            "download_click_uncertain",
            "download_subscription_failed",
            "download_subscription_cleanup_failed",
        ] {
            let failure = classify_service_failure(&format!("{code}: bounded diagnostic"));
            assert_eq!(failure.axis, ServiceFailureAxis::Unknown);
            assert_eq!(serde_json::to_value(&failure).unwrap()["axis"], "unknown");
            assert_eq!(failure.code, code);
            assert_eq!(failure.effect_state, ServiceEffectState::EffectUncertain);
            assert_eq!(
                failure.retry_disposition,
                ServiceRetryDisposition::InspectBeforeRetry
            );
            assert_eq!(failure.recommended_action, "inspect_download_capture");
            assert!(failure.hard_stops.iter().any(|stop| stop == "blind_retry"));
        }
        for code in [
            "download_target_unproven",
            "download_source_identity_unproven",
        ] {
            let failure = classify_service_failure(&format!("{code}: exact proof missing"));
            assert_eq!(failure.axis, ServiceFailureAxis::LifecycleOwner);
        }
    }

    #[test]
    fn display_grant_cause_survives_browser_reservation_rollback_metadata() {
        for code in [
            "display_access_grant_failed",
            "display_access_grant_timeout",
        ] {
            let result = classify_service_failure(&format!("{code}: helper stderr; route_bound_blocker_code=display_access_failed; route_bound_compensation_state=rolled_back"));
            assert_eq!(result.code, code);
            assert_eq!(result.phase, ServiceFailurePhase::LaunchAdmission);
            assert_eq!(result.effect_state, ServiceEffectState::EffectUncertain);
            assert_eq!(
                result.recommended_action,
                "inspect_privileged_display_grant"
            );
        }
    }

    #[test]
    fn retained_recovery_failures_keep_guard_and_attachment_certainty_distinct() {
        for code in [
            "service_tab_recovery_owner_missing",
            "service_tab_recovery_identity_mismatch",
            "service_tab_recovery_process_unproven",
            "service_tab_recovery_endpoint_unproven",
            "service_tab_recovery_target_missing",
            "service_tab_recovery_attach_failed",
        ] {
            let failure = classify_service_failure(&format!("{code}: recorded cause"));
            assert_eq!(failure.code, code);
            assert_eq!(failure.axis, ServiceFailureAxis::LifecycleOwner);
            assert_eq!(failure.phase, ServiceFailurePhase::ChildAdmission);
            assert_eq!(
                failure.effect_state,
                if code.ends_with("attach_failed") {
                    ServiceEffectState::EffectUncertain
                } else {
                    ServiceEffectState::NoEffect
                }
            );
            assert!(failure.executable_next_action.is_some());
            assert_eq!(
                failure.retry_disposition,
                ServiceRetryDisposition::InspectBeforeRetry
            );
        }
        assert_eq!(
            classify_service_failure("wrapped service_tab_recovery_owner_missing: unknown")
                .effect_state,
            ServiceEffectState::EffectUncertain
        );
    }

    #[test]
    fn child_authority_denials_preserve_cause_without_claiming_unknown_errors_are_effect_free() {
        let missing = classify_service_failure("profile child access record is missing");
        assert_eq!(missing.code, "profile_child_access_record_missing");
        assert_eq!(missing.axis, ServiceFailureAxis::ProfileAccess);
        assert_eq!(missing.phase, ServiceFailurePhase::ChildAdmission);
        assert_eq!(missing.effect_state, ServiceEffectState::NoEffect);
        assert_eq!(missing.recommended_action, "inspect_service_trace");
        for (reason, code, action) in [
            (
                "subject_mismatch",
                "profile_child_subject_mismatch",
                "use_own_service_tab_handle",
            ),
            (
                "permission_not_inherited",
                "profile_child_permission_not_inherited",
                "inspect_profile_access_policy",
            ),
            (
                "owner_connection_still_active",
                "profile_child_owner_connection_still_active",
                "use_owner_connection_or_wait",
            ),
            (
                "explicit_reconnect_required",
                "profile_child_explicit_reconnect_required",
                "reconnect_owned_service_tab_handle",
            ),
        ] {
            let failure =
                classify_service_failure(&format!("profile child access denied: {reason}"));
            assert_eq!(failure.code, code);
            assert_eq!(failure.axis, ServiceFailureAxis::ProfileAccess);
            assert_eq!(failure.phase, ServiceFailurePhase::ChildAdmission);
            assert_eq!(failure.effect_state, ServiceEffectState::NoEffect);
            assert_eq!(
                failure.retry_disposition,
                ServiceRetryDisposition::DoNotRetry
            );
            assert_eq!(failure.recommended_action, action);
        }
        for error in [
            "operation failed after profile child access record is missing",
            "profile child access denied: future_reason",
            "operation failed after profile child access denied: subject_mismatch",
        ] {
            assert_eq!(
                classify_service_failure(error).effect_state,
                ServiceEffectState::EffectUncertain
            );
        }
    }

    #[test]
    fn control_queue_rejection_is_safe_to_retry_as_the_same_request() {
        let recourse = classify_service_failure("Control queue is full");

        assert_eq!(recourse.code, "control_queue_full");
        assert_eq!(recourse.effect_state, ServiceEffectState::NoEffect);
        assert_eq!(
            recourse.retry_disposition,
            ServiceRetryDisposition::RetrySameRequest
        );
        assert!(recourse.reuse_allowed);
    }

    #[test]
    fn running_timeout_requires_inspection_before_retry() {
        let recourse = classify_service_failure("Service job timed out after 1000ms");

        assert_eq!(recourse.code, "service_job_timed_out");
        assert_eq!(recourse.effect_state, ServiceEffectState::EffectUncertain);
        assert_eq!(
            recourse.retry_disposition,
            ServiceRetryDisposition::InspectBeforeRetry
        );
    }

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
    fn profile_identity_admission_failure_has_non_circular_lifecycle_recourse() {
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
            assert_eq!(recourse.recommended_action, "inspect_profile_recovery_plan");
            assert_eq!(
                recourse.missing_permission.as_deref(),
                Some("lifecycle_manage")
            );
            assert_eq!(
                recourse.executable_next_action.as_ref().unwrap()["action"],
                "service_profile_recovery_plan"
            );
            assert!(!recourse
                .safe_next_actions
                .contains(&"acquire_profile".to_string()));
            assert!(recourse.hard_stops.contains(&"blind_retry".to_string()));
        }
    }

    #[test]
    fn access_plan_route_conflict_refreshes_before_exact_reuse() {
        for code in [
            "service_access_plan_route_browser_conflict",
            "service_access_plan_route_session_conflict",
        ] {
            let recourse = classify_service_failure(code);

            assert_eq!(recourse.code, code);
            assert_eq!(recourse.axis, ServiceFailureAxis::ProfileLease);
            assert_eq!(recourse.phase, ServiceFailurePhase::LaunchAdmission);
            assert_eq!(recourse.effect_state, ServiceEffectState::NoEffect);
            assert_eq!(
                recourse.retry_disposition,
                ServiceRetryDisposition::RefreshAccessPlan
            );
            assert_eq!(
                recourse.recommended_action,
                "refresh_access_plan_and_use_exact_route"
            );
            assert_eq!(
                recourse.executable_next_action.as_ref().unwrap()["action"],
                "service_access_plan"
            );
            assert!(recourse
                .safe_next_actions
                .contains(&"submit_exact_planned_route".to_string()));
            assert!(recourse.hard_stops.contains(&"blind_retry".to_string()));
            assert!(recourse
                .hard_stops
                .contains(&"launch_duplicate_profile_lane".to_string()));
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
