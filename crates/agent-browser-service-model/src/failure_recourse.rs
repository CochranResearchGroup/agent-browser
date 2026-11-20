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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrence_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Value>,
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
    /// Compiled exact-code classification complements authoritative owner recovery.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_registry: Option<Value>,
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

/// Classify current operation failures. Installer history has no admission role.
pub fn classify_service_failure(error: &str) -> ServiceFailureRecourse {
    // Preserve actual browser/CDP provenance before legacy job-word heuristics.
    // A page exception containing cancellation/timeout prose is never a job's
    // pre-dispatch proof, and no compatibility message attests no physical input.
    if let Some(failure) = classify_browser_compatibility_failure(error) {
        return failure;
    }
    // Emitted only by freshness input checks before the profile is changed.
    // Storage/commit errors keep their original uncertainty classification.
    if error.starts_with("profile_freshness_invalid:") {
        return ServiceFailureRecourse {
            schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
            code: "profile_freshness_invalid".to_string(),
            axis: ServiceFailureAxis::Request,
            phase: ServiceFailurePhase::IngressValidation,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::DoNotRetry,
            recommended_action: "correct_profile_freshness_request".to_string(),
            executable_next_action: Some(serde_json::json!({
                "action": "service_profiles", "executable": true,
                "request": {"action": "service_profiles"}
            })),
            safe_next_actions: vec!["inspect_service_profiles".to_string()],
            hard_stops: vec!["blind_retry".to_string()],
            ..ServiceFailureRecourse::default()
        };
    }
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
    let code = error.split(':').next().unwrap_or_default();
    if matches!(
        code,
        "operator_tab_close_authority_required"
            | "operator_tab_close_authority_unproven"
            | "operator_tab_close_binding_changed"
            | "operator_tab_close_profile_policy_denied"
            | "operator_tab_close_target_missing"
            | "operator_tab_close_target_ambiguous"
            | "operator_tab_close_process_changed"
    ) {
        return ServiceFailureRecourse {
            code: code.to_string(),
            axis: ServiceFailureAxis::ProfileAccess,
            phase: ServiceFailurePhase::ChildAdmission,
            effect_state: ServiceEffectState::NoEffect,
            retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
            recommended_action: "inspect_operator_tab_close_authority".into(),
            safe_next_actions: vec![
                "inspect_service_trace".into(),
                "inspect_operator_tab_close_authority".into(),
            ],
            missing_permission: (code == "operator_tab_close_profile_policy_denied")
                .then(|| "tab_close_any".into()),
            hard_stops: vec!["blind_retry".into(), "impersonate_profile_subject".into()],
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
    if matches!(
        error,
        "Control plane worker is stopped"
            | "Control plane worker is stopped while waiting for profile lease"
    ) {
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
    if matches!(
        error,
        "Service job was cancelled before dispatch" | "Cancelled by operator"
    ) {
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
    if owner_message(error, "Service job was cancelled while running") {
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
    if owner_message(error, "Service job timed out during route-bound open")
        || decimal_message(error, "Service job timed out after ", "ms")
    {
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

// These recognizers validate the producing owner's finite wire shapes. They
// never search page prose or infer effect certainty from error text.
fn owner_message(error: &str, exact: &str) -> bool {
    error == exact
        || error
            .strip_prefix(exact)
            .is_some_and(|rest| rest.starts_with("; "))
}

fn decimal_message(error: &str, prefix: &str, suffix: &str) -> bool {
    error
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix(suffix))
        .is_some_and(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
}

fn cdp_method_message(error: &str, prefix: &str, suffix: &str) -> bool {
    error
        .strip_prefix(prefix)
        .and_then(|rest| rest.split_once(suffix))
        .is_some_and(|(method, _)| {
            !method.is_empty()
                && method.contains('.')
                && method
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_'))
        })
}

/// Exact compatibility messages/prefixes emitted by browser, element, socket
/// and typed CDP owners. Website exception contents remain opaque; only their
/// actual `Evaluation error:` wrapper is classified. Raw error data is retained
/// by the existing response, never copied into a registry/journal subject.
fn classify_browser_compatibility_failure(error: &str) -> Option<ServiceFailureRecourse> {
    let mut error = error;
    for _ in 0..4 {
        let Some((wrapper, cause)) = error.split_once(':') else {
            break;
        };
        if matches!(
            wrapper,
            "workflow_transport_failed" | "workflow_transport_start_failed" | "workflow_cli_failed"
        ) {
            error = cause.trim_start();
        } else {
            break;
        }
    }
    let code = if error.starts_with("Evaluation error: ") {
        "browser_action_evaluation_exception"
    } else if cdp_method_message(error, "CDP error (", "): ") {
        "browser_action_cdp_protocol_failed"
    } else if error.starts_with("CDP command timed out: ") {
        let method = error.strip_prefix("CDP command timed out: ")?;
        if !method.contains('.') || !method.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_')) { return None; }
        "browser_action_cdp_timeout"
    } else if error.starts_with("Failed to serialize CDP command: ") {
        "browser_action_cdp_serialization_failed"
    } else if error.starts_with("Failed to send CDP command: ") || error == "CDP response channel closed" {
        "browser_action_cdp_transport_lost"
    } else if error.starts_with("Failed to deserialize CDP response for Runtime.evaluate: ") {
        "browser_action_cdp_response_invalid"
    } else if error.starts_with("CDP WebSocket connect failed: ") {
        "browser_action_cdp_connect_failed"
    } else if error.starts_with("Invalid WebSocket URL: ") || error.starts_with("Invalid CDP URL: ") || error.starts_with("No host in CDP URL: ") {
        "browser_action_cdp_endpoint_invalid"
    } else if error.starts_with("Navigation failed: ") {
        "browser_action_navigation_failed"
    } else if error.starts_with("Timeout waiting for load state: ") || matches!(error, "Timeout waiting for Page.loadEventFired" | "Timeout waiting for Page.domContentEventFired" | "Timeout waiting for networkidle") {
        "browser_action_navigation_wait_timeout"
    } else if decimal_message(error, "Wait timed out after ", "ms") || error == "Operation timed out. The page may still be loading or the element may not exist." {
        "browser_action_wait_timeout"
    } else if error == "Event stream closed" || error == "target inventory timed out" {
        "browser_action_target_observation_unavailable"
    } else if error.starts_with("Unknown ref: ") || error.starts_with("No objectId for ref ") {
        "browser_action_reference_stale"
    } else if error.starts_with("Element not found: ") || error.starts_with("Could not locate element with role=") || error == "Element not found. Verify the selector is correct and the element exists in the DOM." {
        "browser_action_selector_not_found"
    } else if error.starts_with("Element '") && error.ends_with("' does not have an href attribute. --new-tab only works on links.") {
        "browser_action_element_kind_invalid"
    } else if matches!(error, "Date input is not writable" | "Could not focus or fill the input") {
        "browser_action_fill_unverified"
    } else if error == "Invalid date value; expected YYYY-MM-DD" {
        "browser_action_value_invalid"
    } else if matches!(error, "Ref does not point to an iframe element" | "Could not resolve frame ID for iframe element" | "Frame not found") || error.starts_with("Ref ") && error.ends_with(" has no backend node id") {
        "browser_action_frame_unavailable"
    } else if error == "Element matched multiple results. Use a more specific selector." {
        "browser_action_selector_ambiguous"
    } else if error == "Element exists but is not visible. Wait for it to become visible or scroll it into view." {
        "browser_action_element_not_visible"
    } else if error == "Another element is covering the target element. Try scrolling or closing overlays." {
        "browser_action_element_intercepted"
    } else if error.starts_with("Could not get bounding box for: ") || error.starts_with("AX node has no backendDOMNodeId for role=") || error == "Could not get backendNodeId for file input" || error == "Could not find frame for selector" {
        "browser_action_element_geometry_unavailable"
    } else if matches!(error, "Browser not launched" | "No active page" | "Runtime handoff found no retained page targets" | "Cold-launch blank target disappeared during acquisition") {
        "browser_action_page_unavailable"
    } else if decimal_message(error, "Tab index ", " out of range") {
        "browser_action_tab_index_invalid"
    } else if error == "Cannot close the last tab" {
        "browser_action_last_tab_close_requires_intent"
    } else if error == "Can only leave open a launched managed runtime profile" {
        "browser_action_retention_context_invalid"
    } else if error.starts_with("Auto-launch failed: ") {
        "browser_action_launch_failed"
    } else if error.starts_with("Chrome not found. Checked:\n") {
        "browser_action_executable_unavailable"
    } else if matches!(error, "Cannot use storage_state with profile" | "Cannot use storage_state with extensions" | "Extensions are not supported with Lightpanda" | "Profiles are not supported with Lightpanda" | "Storage state is not supported with Lightpanda" | "File access is not supported with Lightpanda" | "Headed mode is not supported with Lightpanda (headless only)") {
        "browser_action_launch_options_invalid"
    } else if matches!(error, "Browser.getWindowForTarget did not return windowId" | "X11 browser property format changed" | "No managed X11 window found for browser close") || error.starts_with("No X11 window found for browser PID ") {
        "browser_action_native_window_unavailable"
    } else if ["Failed to create socket directory: ", "Failed to secure socket directory: ", "Failed to generate auth token: ", "Failed to write daemon auth token: ", "Failed to secure daemon auth token: ", "Failed to read daemon auth token: ", "Failed to encode daemon process identity: ", "Failed to write daemon process identity: ", "Failed to secure daemon process identity: ", "Failed to resolve current executable: ", "daemon process identity is unavailable: ", "daemon process identity is invalid: "].iter().any(|prefix| error.starts_with(prefix)) {
        "browser_action_transport_configuration_failed"
    } else if error.starts_with("Failed to start daemon: ") || error.starts_with("Daemon process exited during startup:") || error.starts_with("Daemon process exited during startup with no error output.") || error.starts_with("Daemon failed to start (") && error.ends_with(')') {
        "browser_action_transport_start_failed"
    } else if error == "Timed out waiting for runtime-host startup admission" {
        "browser_action_runtime_admission_wait"
    } else if error.starts_with("Failed to connect: ") {
        "browser_action_transport_connect_failed"
    } else if error.starts_with("Failed to send: ") || error.starts_with("Failed to read: ") {
        "browser_action_transport_outcome_uncertain"
    } else if error.starts_with("Invalid response: ") {
        "browser_action_transport_response_invalid"
    } else if error == "Command payload must be a JSON object" {
        "browser_action_argument_invalid"
    } else if error.starts_with("Unknown subaction: ") || error == "Missing 'value' for fill subaction" || error == "At least one of 'selector', 'name', or 'url' is required" {
        "browser_action_argument_invalid"
    } else if matches!(error, "evaluate requires serviceTabHandle" | "evaluate requires returnByValue=true so results can be capped" | "evaluate requires script or expression" | "evaluate requires positive timeoutMs" | "evaluate requires positive maxReturnBytes" | "evaluate requires serviceTabHandle.targetId") {
        "browser_action_evaluate_arguments_invalid"
    } else if let Some(field) = error.strip_prefix("Missing '").and_then(|rest| rest.strip_suffix("' parameter")) {
        if !matches!(field, "path" | "name" | "role" | "selector" | "index" | "script" | "source" | "target" | "html" | "url" | "expression" | "requestId" | "text" | "key" | "value" | "timezoneId" | "locale" | "latitude" | "longitude" | "userAgent" | "attribute" | "event") { return None; }
        "browser_action_argument_missing"
    } else { return None; };
    let request = matches!(
        code,
        "browser_action_cdp_endpoint_invalid"
            | "browser_action_tab_index_invalid"
            | "browser_action_launch_options_invalid"
            | "browser_action_evaluate_arguments_invalid"
            | "browser_action_argument_missing"
            | "browser_action_argument_invalid"
            | "browser_action_value_invalid"
    );
    Some(ServiceFailureRecourse {
        schema_version: SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION.to_string(),
        code: code.into(),
        axis: if request {
            ServiceFailureAxis::Request
        } else {
            ServiceFailureAxis::LifecycleOwner
        },
        phase: if request {
            ServiceFailurePhase::IngressValidation
        } else {
            ServiceFailurePhase::Unknown
        },
        effect_state: ServiceEffectState::EffectUncertain,
        retry_disposition: ServiceRetryDisposition::InspectBeforeRetry,
        recommended_action: "inspect_original_task_and_current_owner".into(),
        reuse_allowed: false,
        safe_next_actions: vec![
            "inspect_original_task_operation".into(),
            "observe_current_original_target".into(),
        ],
        hard_stops: vec!["blind_retry".into(), "infer_no_effect_from_message".into()],
        ..ServiceFailureRecourse::default()
    })
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

    #[test]
    fn browser_compatibility_recognition_preserves_original_uncertainty() {
        for (error, code) in [
            (
                "Evaluation error: Error: Service job was cancelled before dispatch",
                "browser_action_evaluation_exception",
            ),
            (
                "CDP error (Runtime.evaluate): Element not found: #button",
                "browser_action_cdp_protocol_failed",
            ),
            (
                "CDP command timed out: Input.dispatchMouseEvent",
                "browser_action_cdp_timeout",
            ),
            (
                "Failed to serialize CDP command: invalid payload",
                "browser_action_cdp_serialization_failed",
            ),
            (
                "Failed to send CDP command: closed",
                "browser_action_cdp_transport_lost",
            ),
            (
                "CDP response channel closed",
                "browser_action_cdp_transport_lost",
            ),
            (
                "Failed to deserialize CDP response for Runtime.evaluate: missing result",
                "browser_action_cdp_response_invalid",
            ),
            (
                "CDP WebSocket connect failed: refused",
                "browser_action_cdp_connect_failed",
            ),
            (
                "Invalid CDP URL: relative URL without a base",
                "browser_action_cdp_endpoint_invalid",
            ),
            (
                "Navigation failed: net::ERR_NAME_NOT_RESOLVED",
                "browser_action_navigation_failed",
            ),
            (
                "Timeout waiting for Page.loadEventFired",
                "browser_action_navigation_wait_timeout",
            ),
            ("Wait timed out after 1000ms", "browser_action_wait_timeout"),
            ("Unknown ref: e3", "browser_action_reference_stale"),
            (
                "Element not found: #button",
                "browser_action_selector_not_found",
            ),
            (
                "Element matched multiple results. Use a more specific selector.",
                "browser_action_selector_ambiguous",
            ),
            ("Element exists but is not visible. Wait for it to become visible or scroll it into view.", "browser_action_element_not_visible"),
            ("Another element is covering the target element. Try scrolling or closing overlays.", "browser_action_element_intercepted"),
            ("Could not get bounding box for: #input", "browser_action_element_geometry_unavailable"),
            ("Element '#link' does not have an href attribute. --new-tab only works on links.", "browser_action_element_kind_invalid"),
            ("Could not focus or fill the input", "browser_action_fill_unverified"),
            ("Invalid date value; expected YYYY-MM-DD", "browser_action_value_invalid"),
            ("Ref does not point to an iframe element", "browser_action_frame_unavailable"),
            ("Event stream closed", "browser_action_target_observation_unavailable"),
            ("Can only leave open a launched managed runtime profile", "browser_action_retention_context_invalid"),
            ("Auto-launch failed: original owner refused", "browser_action_launch_failed"),
            ("Chrome not found. Checked:\n  configured original cache", "browser_action_executable_unavailable"),
            ("Profiles are not supported with Lightpanda", "browser_action_launch_options_invalid"),
            ("No managed X11 window found for browser close", "browser_action_native_window_unavailable"),
            ("Failed to connect: connection refused", "browser_action_transport_connect_failed"),
            ("Invalid response: expected value", "browser_action_transport_response_invalid"),
            ("Unknown subaction: unreviewed", "browser_action_argument_invalid"),
            ("Failed to read daemon auth token: permission denied", "browser_action_transport_configuration_failed"),
            ("Failed to start daemon: missing executable", "browser_action_transport_start_failed"),
            ("Timed out waiting for runtime-host startup admission", "browser_action_runtime_admission_wait"),
            ("workflow_transport_failed: Failed to read: original reply lost", "browser_action_transport_outcome_uncertain"),
            ("workflow_cli_failed: Evaluation error: Error: cancelled before dispatch", "browser_action_evaluation_exception"),
            ("Browser not launched", "browser_action_page_unavailable"),
            (
                "Tab index 12 out of range",
                "browser_action_tab_index_invalid",
            ),
            (
                "Cannot close the last tab",
                "browser_action_last_tab_close_requires_intent",
            ),
            (
                "Failed to read: connection reset",
                "browser_action_transport_outcome_uncertain",
            ),
            (
                "Missing 'selector' parameter",
                "browser_action_argument_missing",
            ),
            (
                "evaluate requires positive timeoutMs",
                "browser_action_evaluate_arguments_invalid",
            ),
        ] {
            let failure = classify_service_failure(error);
            assert_eq!(failure.code, code, "{error}");
            assert_eq!(
                failure.effect_state,
                ServiceEffectState::EffectUncertain,
                "{error}"
            );
            assert_eq!(
                failure.retry_disposition,
                ServiceRetryDisposition::InspectBeforeRetry
            );
            assert!(!failure.reuse_allowed);
        }
    }

    #[test]
    fn browser_compatibility_recognition_rejects_website_and_unknown_patterns() {
        for error in [
            "page says Element not found: #button",
            "unknown_wrapper: Element not found: #button",
            "arbitrary page output {\"error\":\"Browser not launched\"}",
            "arbitrary provider timeout text",
            "arbitrary provider cancelled before dispatch",
            "fake Control plane worker is stopped message",
            "Missing 'unreviewedField' parameter",
            "Tab index x out of range",
            "Wait timed out after unknownms",
            "CDP error (not a method): timeout",
            "CDP command timed out: not-a-method",
            "Operation timed out but the source is unknown",
        ] {
            let failure = classify_service_failure(error);
            assert_eq!(failure.code, "service_operation_failed", "{error}");
            assert_eq!(
                failure.effect_state,
                ServiceEffectState::EffectUncertain,
                "{error}"
            );
        }
        for error in [
            "Service job was cancelled before dispatch",
            "Cancelled by operator",
        ] {
            assert_eq!(
                classify_service_failure(error).effect_state,
                ServiceEffectState::NoEffect
            );
        }
        assert_eq!(
            classify_service_failure("Service job timed out after 1000ms").code,
            "service_job_timed_out"
        );
        assert_eq!(classify_service_failure("Service job timed out during route-bound open; route_bound_compensation_state=unknown").code, "service_job_timed_out");
    }

    #[test]
    fn registry_classification_survives_typed_job_failure_round_trip() {
        let legacy = serde_json::to_value(classify_service_failure("fixture defect")).unwrap();
        assert!(legacy.get("recoveryRegistry").is_none());
        let mut source = legacy;
        source["recoveryRegistry"] =
            json!({"coverage":"registered","evidence":{"status":"not_observed_by_projection"}});
        let typed: ServiceFailureRecourse = serde_json::from_value(source.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(typed).unwrap()["recoveryRegistry"],
            source["recoveryRegistry"]
        );
    }

    #[test]
    fn retired_upgrade_error_does_not_recreate_transaction_wait_recourse() {
        let failure = classify_service_failure("runtime_admission_draining:legacy-upgrade");
        assert_eq!(failure.code, "service_operation_failed");
        assert!(!failure
            .safe_next_actions
            .iter()
            .any(|action| action == "wait_for_transaction_terminal"));
    }

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
    #[test]
    fn operator_close_denial_is_pre_effect_but_physical_uncertainty_is_not() {
        let denied =
            classify_service_failure("operator_tab_close_profile_policy_denied: diagnostic");
        assert_eq!(denied.effect_state, ServiceEffectState::NoEffect);
        assert_eq!(denied.axis, ServiceFailureAxis::ProfileAccess);
        assert_eq!(denied.missing_permission.as_deref(), Some("tab_close_any"));
        assert_eq!(
            denied.retry_disposition,
            ServiceRetryDisposition::InspectBeforeRetry
        );
        let uncertain =
            classify_service_failure("tab_close_effect_uncertain: target readback failed");
        assert_eq!(uncertain.effect_state, ServiceEffectState::EffectUncertain);
        assert_eq!(
            uncertain.retry_disposition,
            ServiceRetryDisposition::InspectBeforeRetry
        );
    }
}
