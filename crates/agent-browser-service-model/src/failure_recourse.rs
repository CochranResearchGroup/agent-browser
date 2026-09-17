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

pub fn profile_child_denial_error(
    reason: &str,
    evidence: Option<&crate::ProfileChildAccessEvidence>,
) -> String {
    let message = format!("profile child access denied: {reason}");
    evidence
        .and_then(|value| serde_json::to_string(value).ok())
        .map(|encoded| format!("{message}{CHILD_EVIDENCE_SEPARATOR}{encoded}"))
        .unwrap_or(message)
}

/// Only the typed child decision evidence is eligible for journal projection.
/// Other recourse subjects may contain fields outside the journal privacy bound.
pub fn child_access_failure_evidence(failure: &ServiceFailureRecourse) -> Option<Value> {
    if !matches!(
        failure.code.as_str(),
        "profile_child_subject_mismatch"
            | "profile_child_permission_not_inherited"
            | "profile_child_owner_connection_still_active"
            | "profile_child_explicit_reconnect_required"
    ) {
        return None;
    }
    crate::ProfileChildAccessEvidence::decode(failure.subject.clone()?)
        .and_then(|evidence| serde_json::to_value(evidence).ok())
}

pub fn operator_focus_failure_code(error: &str) -> Option<&str> {
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
    // Desktop interaction validates controller authority before it can emit an
    // input event. Preserve that native certainty instead of projecting the
    // conservative generic operation-failure fallback.
    if error.split(':').next() == Some("desktop_interaction_authority_required") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "desktop_interaction_authority_required".to_string(),
            axis: ServiceFailureAxis::ProfileAccess,
            phase: ServiceFailurePhase::ChildAdmission,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "inspect_desktop_interaction_authority".to_string(),
            reuse_allowed: false,
            safe_next_actions: vec![
                "inspect_service_trace".to_string(),
                "compare_controller_viewer_and_interaction_agent".to_string(),
            ],
            hard_stops: vec![
                "blind_retry".to_string(),
                "impersonate_controller_viewer".to_string(),
            ],
            ..ServiceFailureRecourse::default()
        };
    }
    if error.starts_with("runtime_admission_draining:") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "runtime_admission_draining".to_string(),
            axis: ServiceFailureAxis::LifecycleOwner,
            phase: ServiceFailurePhase::LaunchAdmission,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "inspect_install_transaction".to_string(),
            safe_next_actions: vec![
                "inspect_install_transaction".to_string(),
                "wait_for_transaction_terminal".to_string(),
            ],
            hard_stops: vec!["blind_retry".to_string()],
            ..ServiceFailureRecourse::default()
        };
    }
    if error.starts_with("Invalid runtime profile '") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "invalid_runtime_profile".to_string(),
            axis: ServiceFailureAxis::Request,
            phase: ServiceFailurePhase::LaunchAdmission,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::DoNotRetry,
            recommended_action: "correct_runtime_profile_selector".to_string(),
            safe_next_actions: vec!["use_named_profile_or_exact_profile_path".to_string()],
            hard_stops: vec!["blind_retry".to_string()],
            ..ServiceFailureRecourse::default()
        };
    }
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
            let evidence = crate::ProfileChildAccessEvidence::decode(value)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProfileConnectionState, ProfileIdentityAssurance, ProfilePermission};
    use serde_json::json;

    fn child_evidence() -> crate::ProfileChildAccessEvidence {
        crate::ProfileChildAccessEvidence {
            source_function:
                "native/service_profile_access_policy.rs::evaluate_profile_child_access".to_string(),
            expected_subject_hash: Some("a".repeat(64)),
            observed_subject_hash: Some("b".repeat(64)),
            expected_connection_hash: Some("c".repeat(64)),
            observed_connection_hash: "d".repeat(64),
            owner_assurance: ProfileIdentityAssurance::RegisteredCapability,
            caller_assurance: ProfileIdentityAssurance::AuthenticatedIngress,
            connection_state: ProfileConnectionState::Active,
            permission: ProfilePermission::TabControlOwn,
            child_permission_present: true,
            current_permission_present: false,
            parent_policy_revision: 4,
            current_policy_revision: 5,
            reconnect_requested: false,
        }
    }

    #[test]
    fn default_and_wire_shape_remain_stable() {
        let default: ServiceFailureRecourse = serde_json::from_value(json!({})).unwrap();
        assert_eq!(default, ServiceFailureRecourse::default());
        assert_eq!(default.axis, ServiceFailureAxis::Unknown);
        assert_eq!(default.phase, ServiceFailurePhase::Unknown);
        assert_eq!(default.effect_state, ServiceEffectState::EffectUncertain);
        assert_eq!(
            default.retry_disposition,
            ServiceRetryDisposition::InspectBeforeRetry
        );

        let value = serde_json::to_value(default).unwrap();
        assert_eq!(
            value,
            json!({
                "schemaVersion": "",
                "code": "",
                "axis": "unknown",
                "phase": "unknown",
                "effectState": "effect_uncertain",
                "retryDisposition": "inspect_before_retry",
                "recommendedAction": "",
                "reuseAllowed": false,
                "subject": null,
                "missingPermission": null,
                "executableNextAction": null
            })
        );
    }

    #[test]
    fn known_no_effect_and_uncertain_failures_preserve_recourse() {
        let denied = classify_service_failure(
            "desktop_interaction_authority_required: controller authority was not proven",
        );
        assert_eq!(denied.code, "desktop_interaction_authority_required");
        assert_eq!(denied.axis, ServiceFailureAxis::ProfileAccess);
        assert_eq!(denied.phase, ServiceFailurePhase::ChildAdmission);
        assert_eq!(denied.effect_state, ServiceEffectState::NoEffect);
        assert_eq!(
            denied.retry_disposition,
            ServiceRetryDisposition::InspectBeforeRetry
        );
        assert_eq!(
            denied.recommended_action,
            "inspect_desktop_interaction_authority"
        );
        assert!(denied.hard_stops.iter().any(|stop| stop == "blind_retry"));

        let uncertain = classify_service_failure("download_click_uncertain: diagnostic");
        assert_eq!(uncertain.code, "download_click_uncertain");
        assert_eq!(uncertain.axis, ServiceFailureAxis::Unknown);
        assert_eq!(uncertain.effect_state, ServiceEffectState::EffectUncertain);
        assert_eq!(
            uncertain.retry_disposition,
            ServiceRetryDisposition::InspectBeforeRetry
        );
        assert_eq!(uncertain.recommended_action, "inspect_download_capture");
        assert!(uncertain
            .hard_stops
            .iter()
            .any(|stop| stop == "blind_retry"));
    }

    #[test]
    fn state_lock_metadata_and_stale_revision_are_deterministic() {
        let timeout = classify_service_failure(
            "service_state_lock_timeout: process mutation lock; waited_ms=1001; holder_operation=mutate",
        );
        assert_eq!(timeout.code, "service_state_lock_timeout");
        assert_eq!(timeout.axis, ServiceFailureAxis::ServiceState);
        assert_eq!(timeout.phase, ServiceFailurePhase::ProcessMutexWait);
        assert_eq!(timeout.effect_state, ServiceEffectState::EffectUncertain);
        assert_eq!(timeout.wait_ms, Some(1001));
        assert_eq!(timeout.holder_operation.as_deref(), Some("mutate"));

        let stale = classify_service_failure("service_state_stale_revision: expected=4; actual=5");
        assert_eq!(stale.phase, ServiceFailurePhase::Commit);
        assert_eq!(stale.effect_state, ServiceEffectState::NoEffect);
        assert_eq!(stale.recommended_action, "reload_state_and_replan");
        assert!(!stale.reuse_allowed);
    }

    #[test]
    fn child_denial_evidence_is_bounded_and_round_trips() {
        let evidence = child_evidence();
        let error = profile_child_denial_error("subject_mismatch", Some(&evidence));
        let failure = classify_service_failure(&error);

        assert_eq!(failure.code, "profile_child_subject_mismatch");
        assert_eq!(failure.axis, ServiceFailureAxis::ProfileAccess);
        assert_eq!(failure.phase, ServiceFailurePhase::ChildAdmission);
        assert_eq!(failure.effect_state, ServiceEffectState::NoEffect);
        assert_eq!(
            child_access_failure_evidence(&failure),
            Some(serde_json::to_value(&evidence).unwrap())
        );

        let unrelated = classify_service_failure("service_state_stale_revision");
        assert_eq!(child_access_failure_evidence(&unrelated), None);
    }

    #[test]
    fn malformed_child_evidence_is_not_projected() {
        let error = "profile child access denied: subject_mismatch; childAccessEvidence={\"sourceFunction\":\"forged\"}";
        let failure = classify_service_failure(error);

        assert_eq!(failure.code, "service_operation_failed");
        assert_eq!(failure.effect_state, ServiceEffectState::EffectUncertain);
        assert_eq!(failure.subject, None);
        assert_eq!(child_access_failure_evidence(&failure), None);
    }

    #[test]
    fn operator_focus_code_recognition_is_exact_and_bounded() {
        assert_eq!(
            operator_focus_failure_code(
                "operator_focus_binding_changed: expected exact controller binding"
            ),
            Some("operator_focus_binding_changed")
        );
        assert_eq!(
            operator_focus_failure_code(
                "wrapped operator_focus_binding_changed: expected exact controller binding"
            ),
            None
        );
        assert_eq!(operator_focus_failure_code("future_focus_error"), None);
    }
}
