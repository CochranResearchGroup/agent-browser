//! Serialized command routing, shared gates, timing, and response envelopes.

#[cfg(test)]
mod confirmation_tests;
#[cfg(test)]
mod dependent_batch_tests;
#[cfg(test)]
mod dispatch_tests;
#[cfg(test)]
mod remote_view_route_tests_one;
#[cfg(test)]
mod remote_view_route_tests_two;
#[cfg(test)]
mod runtime_route_host_tests;
#[cfg(test)]
mod service_activity_tests;
#[cfg(test)]
mod service_config_tests;
#[cfg(test)]
mod service_health_tests;
#[cfg(test)]
mod service_incident_mutation_tests;
#[cfg(test)]
mod service_incidents_tests;
#[cfg(test)]
mod service_inventory_tests;
#[cfg(test)]
mod service_jobs_tests;
#[cfg(test)]
mod service_reconcile_tests;
#[cfg(test)]
mod service_trace_tests;
#[cfg(test)]
mod state_tests;
use super::action_runtime::runtime::{
    active_browser_profile_mismatch, auto_launch, detect_browser_stale_state, handle_cdp_attach,
    handle_cdp_detach, handle_cdp_free_launch, handle_close, handle_external_byop_adopt,
    handle_launch, handle_navigate, handle_recovery_close, handle_runtime_handoff_abort,
    handle_runtime_handoff_finalize, handle_runtime_handoff_prepare, handle_runtime_handoff_resume,
    handle_runtime_handoff_rollback, handle_snapshot,
    persist_browser_recovery_started_from_persisted_state, persist_current_browser_stale_health,
    BackendType, CloseBehavior, DaemonState, PendingConfirmation,
};
use super::auth::{
    handle_auth_show, handle_credentials_delete, handle_credentials_get, handle_credentials_list,
    handle_credentials_set, handle_http_credentials,
};
use super::auth_workflow::{begin_confirmation, handle_auth_login, handle_auth_save, handle_deny};
use super::browser_context::{
    handle_bringtofront, handle_geolocation, handle_locale, handle_permissions, handle_timezone,
};
use super::browser_download::{handle_download, handle_waitfordownload};
use super::browser_emulation::{handle_device, handle_set_media};
use super::browser_emulation::{handle_user_agent, handle_viewport};
use super::browser_frame::{
    handle_frame, handle_mainframe, handle_waitforfunction, handle_waitforloadstate,
    handle_waitforurl,
};
use super::browser_input::{
    handle_input_keyboard, handle_input_mouse, handle_input_touch, handle_inserttext,
    handle_keyboard, handle_keydown, handle_keyup, handle_mouse, handle_mousedown,
    handle_mousemove, handle_mouseup, handle_wheel,
};
use super::browser_inspection::{
    handle_cdp_url, handle_console, handle_content, handle_errors, handle_evaluate, handle_inspect,
    handle_setcontent, handle_styles, handle_title, handle_url,
};
use super::browser_lifecycle::{
    handle_tab_close, handle_tab_handle_refresh, handle_tab_handle_release, handle_tab_switch,
    handle_view_focus, handle_view_takeover,
};
use super::browser_locator::{
    handle_drag, handle_evalhandle, handle_expose, handle_find, handle_getbyalttext,
    handle_getbylabel, handle_getbyplaceholder, handle_getbyrole, handle_getbytestid,
    handle_getbytext, handle_getbytitle, handle_multiselect, handle_nth, handle_pause,
};
use super::browser_navigation::{
    handle_back, handle_forward, handle_reload, take_response_warning,
};
use super::browser_tabs::{
    handle_browser_pid, handle_tab_list, handle_tab_new_with_cold_launch, handle_window_new,
};
use super::clipboard::handle_clipboard;
use super::cookies::{handle_cookies_clear, handle_cookies_get, handle_cookies_set};
use super::desktop_capture::{handle_desktop_capture, redact_desktop_capture_stream_result};
use super::desktop_evidence_action::{
    handle_desktop_evidence_observe, redact_desktop_evidence_stream_result,
};
use super::desktop_interaction::{
    handle_desktop_interact, redact_desktop_interaction_stream_result,
};
use super::desktop_locator::{handle_desktop_locate, redact_desktop_locate_stream_result};
use super::desktop_prompt_perception::{
    handle_desktop_prompt_observe, redact_desktop_prompt_stream_result,
};
use super::diff::{handle_diff_screenshot, handle_diff_snapshot, handle_diff_url};
use super::element::{
    handle_boundingbox, handle_count, handle_innerhtml, handle_innertext, handle_inputvalue,
    handle_setvalue,
};
use super::interaction::{
    handle_check, handle_clear, handle_click, handle_dblclick, handle_dialog, handle_dispatch,
    handle_fill, handle_focus, handle_getattribute, handle_gettext, handle_highlight, handle_hover,
    handle_ischecked, handle_isenabled, handle_isvisible, handle_press, handle_scroll,
    handle_scrollintoview, handle_select, handle_selectall, handle_tap, handle_type,
    handle_uncheck, handle_upload, handle_wait,
};
use super::network::{
    handle_headers, handle_offline, handle_responsebody, handle_route, handle_unroute,
};
use super::network_archive::handle_har_stop;
use super::network_requests::{handle_request_detail, handle_requests};
use super::page_capture::{handle_pdf, handle_screenshot};
use super::page_injection::{handle_addinitscript, handle_addscript, handle_addstyle};
use super::providers::{handle_service_provider_delete, handle_service_provider_upsert};
use super::recording::{
    handle_har_start, handle_recording_restart, handle_recording_start, handle_recording_stop,
    handle_video_start, handle_video_stop,
};
use super::remote_view::open::{
    handle_remote_view_open, handle_service_profile_manual_seeding_acquire,
    handle_service_profile_manual_seeding_close, handle_service_remote_view_browser_reattach,
    handle_service_remote_view_handoff_resolve, handle_service_remote_view_route_checkout,
    handle_service_remote_view_route_preflight, handle_service_remote_view_route_release,
    route_bound_open_attribution_from_authenticated_dispatch,
};
use super::remote_view::viewer_lease::{
    handle_service_controller_lease_takeover, handle_service_viewer_lease_heartbeat,
    handle_service_viewer_lease_release, handle_service_viewer_lease_request,
};
use super::service_access::{
    handle_service_browser_capability_preference_guide,
    handle_service_browser_capability_preflight, handle_service_browser_capability_registry_upsert,
    handle_service_profiles,
};
use super::service_activity::{handle_service_events, handle_service_incident_activity};
use super::service_browser_retirement::handle_service_browser_retirement_command;
use super::service_config::{
    handle_service_profile_delete, handle_service_profile_freshness_update,
    handle_service_profile_seeding_handoff_update, handle_service_profile_upsert,
    handle_service_site_policy_delete, handle_service_site_policy_upsert,
};
use super::service_diagnostics::handle_service_diagnostics;
use super::service_file_transfer::handle_service_file_transfer;
use super::service_health::{
    handle_service_browser_close, handle_service_browser_repair, handle_service_browser_retry,
    handle_service_reconcile,
};
use super::service_incidents::{
    handle_service_incident_acknowledge, handle_service_incident_resolve, handle_service_incidents,
    handle_service_remedies_apply,
};
use super::service_inventory::{
    handle_service_browsers, handle_service_challenges, handle_service_monitors,
    handle_service_profile_lookup, handle_service_profile_seeding_handoff,
    handle_service_providers, handle_service_sessions, handle_service_site_policies,
    handle_service_tabs,
};
use super::service_jobs::{handle_service_job_cancel, handle_service_jobs};
use super::service_lifecycle::{handle_service_session_delete, handle_service_session_upsert};
use super::service_model::MonitorState;
use super::service_monitors::{
    handle_service_monitor_delete, handle_service_monitor_reset_failures,
    handle_service_monitor_state_update, handle_service_monitor_triage,
    handle_service_monitor_upsert, handle_service_monitors_run_due,
};
use super::service_network_capture::handle_service_network_capture;
use super::service_probe::handle_service_probe;
use super::service_profile_lease::{
    handle_service_profile_lease_command, handle_service_profile_leases,
};
use super::service_profile_recovery::handle_service_profile_recovery_command;
use super::service_renderer_crash::{
    race_action_with_renderer_crash, renderer_crash_error_response, RendererCrashRace,
};
use super::service_resources::{
    handle_service_access_plan, handle_service_gc, handle_service_resources,
    handle_service_resources_monitor_summary, handle_service_resources_write_monitor_summary,
};
use super::service_retained_state::{
    handle_service_prune_retained, handle_service_repair_retained, handle_service_route_pool_repair,
};
use super::service_status_projection::handle_service_status;
use super::service_trace::handle_service_trace;
use super::service_ui_action::handle_service_ui_action;
use super::state::{handle_state_load, handle_state_save};
use super::storage::{handle_storage_clear, handle_storage_get, handle_storage_set};
use super::stream_runtime::{
    handle_screencast_start, handle_screencast_stop, handle_stream_disable, handle_stream_enable,
    handle_stream_status,
};
use super::tracing::{
    handle_profiler_start, handle_profiler_stop, handle_trace_start, handle_trace_stop,
};
use super::webdriver::mobile_gestures::{handle_device_list, handle_swipe};
use crate::native::action_runtime::cancellation::cancellation_error;
use crate::native::policy::PolicyResult;
use crate::native::service_health::BrowserRecoveryPersistence;
use crate::native::state;
use crate::native::webdriver::backend::WEBDRIVER_UNSUPPORTED_ACTIONS;
use serde_json::{json, Value};

macro_rules! race_renderer_crash {
    ($action:expr, $receiver:expr, $context:expr $(,)?) => {
        race_action_with_renderer_crash(Box::pin(async { $action }), $receiver, $context)
    };
}

pub(crate) fn action_skips_browser_launch(action: &str) -> bool {
    matches!(
        action,
        "" | "launch"
            | "runtime_handoff_prepare"
            | "runtime_handoff_resume"
            | "runtime_handoff_abort"
            | "runtime_handoff_rollback"
            | "runtime_handoff_finalize"
            | "cdp_free_launch"
            | "external_byop_adopt"
            | "cdp_attach"
            | "cdp_detach"
            | "diagnostics"
            | "desktop_capture"
            | "desktop_locate"
            | "desktop_evidence_observe"
            | "desktop_prompt_observe"
            | "desktop_interact"
            | "probe"
            | "close"
            | "confirm"
            | "deny"
            | "har_stop"
            | "credentials_set"
            | "credentials_get"
            | "credentials_delete"
            | "credentials_list"
            | "auth_save"
            | "auth_show"
            | "auth_delete"
            | "auth_list"
            | "state_list"
            | "state_show"
            | "state_clear"
            | "state_clean"
            | "state_rename"
            | "dependent_batch"
            | "device_list"
            | "stream_enable"
            | "stream_disable"
            | "stream_status"
            | "view_takeover"
            | "remote_view_open"
            | "service_profile_manual_seeding_acquire"
            | "service_profile_manual_seeding_close"
            | "service_remote_view_handoff_resolve"
            | "service_remote_view_route_preflight"
            | "service_remote_view_browser_reattach"
            | "service_remote_view_route_switch"
            | "service_remote_view_route_checkout"
            | "service_remote_view_route_release"
            | "service_route_pool_repair"
            | "service_viewer_lease_request"
            | "service_viewer_lease_heartbeat"
            | "service_viewer_lease_release"
            | "service_controller_lease_takeover"
            | "service_status"
            | "service_reconcile"
            | "service_browser_close"
            | "service_browser_repair"
            | "service_browser_contamination_report"
            | "service_browser_retirement_plan"
            | "service_browser_retirement_apply"
            | "service_resources"
            | "service_resources_monitor_summary"
            | "service_resources_write_monitor_summary"
            | "service_gc"
            | "service_prune_retained"
            | "service_repair_retained"
            | "service_access_plan"
            | "service_browser_capability_preflight"
            | "service_browser_capability_preference_guide"
            | "service_job_cancel"
            | "service_browser_retry"
            | "service_remedies_apply"
            | "service_profile_upsert"
            | "service_profile_freshness_update"
            | "service_profile_seeding_handoff_update"
            | "service_profile_delete"
            | "service_session_upsert"
            | "service_session_delete"
            | "service_site_policy_upsert"
            | "service_site_policy_delete"
            | "service_monitor_upsert"
            | "service_monitor_delete"
            | "service_monitor_pause"
            | "service_monitor_reset_failures"
            | "service_monitor_resume"
            | "service_monitor_triage"
            | "service_monitors_run_due"
            | "service_provider_upsert"
            | "service_provider_delete"
            | "service_browser_capability_registry_upsert"
            | "service_incident_acknowledge"
            | "service_incident_resolve"
            | "service_incident_activity"
            | "service_trace"
            | "service_profiles"
            | "service_profile_leases"
            | "service_profile_lease_inspect"
            | "service_profile_lease_explain"
            | "service_profile_lease_doctor"
            | "service_profile_capability_status"
            | "service_profile_lease_register"
            | "service_profile_capability_rotate"
            | "service_profile_lease_rejoin"
            | "service_profile_lease_renew"
            | "service_profile_lease_release"
            | "service_profile_lease_reconcile_plan"
            | "service_profile_lease_reconcile_apply"
            | "service_profile_lease_recover_plan"
            | "service_profile_lease_recover_apply"
            | "service_profile_acquire"
            | "service_profile_recovery_plan"
            | "service_profile_recovery_apply"
            | "service_profile_recovery_status"
            | "service_profile_lookup"
            | "service_profile_seeding_handoff"
            | "service_sessions"
            | "service_browsers"
            | "service_tabs"
            | "service_monitors"
            | "service_site_policies"
            | "service_providers"
            | "service_challenges"
            | "service_jobs"
            | "service_incidents"
            | "service_events"
            | "tab_handle_refresh"
            | "tab_handle_release"
            | "file_transfer"
    )
}

fn active_manual_seeding_cdp_blocker(cmd: &Value, state: &DaemonState) -> Option<String> {
    let repository =
        crate::native::service_store::LockedServiceStateRepository::default_json().ok()?;
    let service_state =
        crate::native::service_store::ServiceStateRepository::load_snapshot(&repository).ok()?;
    let active_browser_id = super::action_runtime::runtime::service_browser_id(&state.session_id);
    let profile_id = cmd
        .get("profileId")
        .or_else(|| cmd.get("runtimeProfile"))
        .or_else(|| cmd.get("profile"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            service_state
                .browsers
                .get(&active_browser_id)
                .and_then(|browser| browser.profile_id.clone())
        })?;
    service_state
        .profile_seeding_handoffs
        .values()
        .find(|handoff| {
            handoff.profile_id == profile_id && handoff.state.blocks_profile_lease()
        })
        .map(|handoff| {
            format!(
                "manual_seeding_cdp_action_denied: action requires DevTools while profile '{}' is in '{}' lifecycle state; close PID {} through the exact manual-seeding handoff first",
                profile_id,
                serde_json::to_value(handoff.state)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_string))
                    .unwrap_or_else(|| "manual_seeding".to_string()),
                handoff
                    .pid
                    .map(|pid| pid.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            )
        })
}

pub(crate) fn active_target_binding(state: &DaemonState) -> Option<String> {
    state
        .browser
        .as_ref()
        .and_then(|manager| manager.active_session_id().ok())
        .map(str::to_string)
}

/// Executes parsed commands under the outer control-plane request. Stable
/// steps must preserve the active target identity; target-changing steps make
/// the next step bind to the new active target.
#[rustfmt::skip]
pub(crate) async fn handle_dependent_batch(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let commands = cmd
        .get("commands")
        .and_then(Value::as_array)
        .ok_or("Missing 'commands' array for dependent batch")?;
    if commands.is_empty() {
        return Ok(json!(
            { "results" : [], "completed" : 0, "hadError" : false, "bail" : cmd
            .get("bail").and_then(Value::as_bool).unwrap_or(false),
            "initialTargetBinding" : active_target_binding(state),
            "finalTargetBinding" : active_target_binding(state), }
        ));
    }
    let bail = cmd.get("bail").and_then(Value::as_bool).unwrap_or(false);
    let initial_binding = active_target_binding(state);
    let mut expected_binding = initial_binding.clone();
    let mut results = Vec::with_capacity(commands.len());
    let mut had_error = false;
    for (index, command) in commands.iter().enumerate() {
        let action = command.get("action").and_then(Value::as_str).unwrap_or("");
        if !super::dependent_batch::nested_batch_allowed(action) {
            had_error = true;
            results.push(json!(
                { "index" : index, "action" : action, "success" : false, "error"
                :
                format!("Action '{action}' cannot run inside a dependent batch"),
                "targetBindingBefore" : active_target_binding(state),
                "targetBindingAfter" : active_target_binding(state), }
            ));
            if bail {
                break;
            }
            continue;
        }
        let effect = super::dependent_batch::target_effect(action);
        let binding_before = active_target_binding(state);
        if effect == super::dependent_batch::TargetEffect::Stable
            && expected_binding.is_some()
            && binding_before != expected_binding
        {
            had_error = true;
            results.push(json!(
                { "index" : index, "action" : action, "success" : false, "error"
                : "Active target changed before a target-stable dependent step",
                "expectedTargetBinding" : expected_binding, "targetBindingBefore"
                : binding_before, "targetBindingAfter" :
                active_target_binding(state), }
            ));
            if bail {
                break;
            }
            expected_binding = active_target_binding(state);
            continue;
        }
        let step_started = std::time::Instant::now();
        let response = Box::pin(execute_command(command, state)).await;
        let action_execution_ms =
            u64::try_from(step_started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let binding_after = active_target_binding(state);
        let mut success = response
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut error = response.get("error").cloned().unwrap_or(Value::Null);
        if effect == super::dependent_batch::TargetEffect::Stable && binding_before != binding_after
        {
            success = false;
            error = json!("Target identity changed during a target-stable dependent step");
        }
        if !success {
            had_error = true;
        }
        results.push(json!(
            { "index" : index, "action" : action, "success" : success, "result" :
            response.get("data").cloned().unwrap_or(Value::Null), "error" :
            error, "daemonTimings" : response.get("timings").cloned()
            .unwrap_or(Value::Null), "targetBindingBefore" : binding_before,
            "targetBindingAfter" : binding_after, "targetRebound" : effect ==
            super::dependent_batch::TargetEffect::Rebind, "timings" : {
            "actionExecutionMs" : action_execution_ms, } }
        ));
        expected_binding = binding_after;
        if had_error && bail {
            break;
        }
    }
    let completed = results.len();
    Ok(json!(
        { "results" : results, "completed" : completed, "requested" : commands.len(),
        "hadError" : had_error, "bail" : bail, "initialTargetBinding" :
        initial_binding, "finalTargetBinding" : active_target_binding(state), }
    ))
}

pub(crate) async fn execute_command(cmd: &Value, state: &mut DaemonState) -> Value {
    let _service_state_lock_timeout_override =
        crate::native::service_store::service_state_lock_timeout_override(
            cmd.get("serviceStateLockTimeoutMs").and_then(Value::as_u64),
        );
    let action = cmd.get("action").and_then(|v| v.as_str()).unwrap_or("");
    let id = cmd
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let cmd_start = std::time::Instant::now();
    if action != "confirm"
        && action != "deny"
        && !matches!(
            action,
            "desktop_evidence_observe" | "desktop_interact" | "desktop_prompt_observe"
        )
    {
        if let Some(ref ca) = state.confirm_actions {
            if ca.requires_confirmation(action) {
                state.pending_confirmation = Some(PendingConfirmation {
                    action: action.to_string(),
                    cmd: cmd.clone(),
                });
                return json!(
                    { "id" : id, "success" : true, "data" : { "confirmation_required" :
                    true, "confirmation_id" : id, "action" : action, }, }
                );
            }
        }
    }
    if crate::runtime_owner_transfer::action_requires_owner_effect_authority(action) {
        let admission_drain = match crate::runtime_adoption::runtime_admission_drain_path() {
            Ok(path) => path,
            Err(error) => return error_response(&id, &error),
        };
        if let Err(error) =
            crate::runtime_adoption::require_runtime_admission(&admission_drain, action, cmd)
        {
            return error_response(&id, &error);
        }
        if let Err(error) = crate::native::runtime_lifecycle::admit_default_action_effect(
            &mut state.runtime_owner_binding,
            action,
            &state.session_id,
        ) {
            return error_response(&id, &error);
        }
    }
    // Desktop evidence is service-owned and does not require a daemon-local
    // browser manager. Resolve it before CDP event drain, policy reload,
    // confirmation mutation, or browser recovery.
    if action == "desktop_evidence_observe" {
        return match handle_desktop_evidence_observe(cmd).await {
            Ok(data) => success_response(&id, data),
            Err(error) => error_response(&id, &error),
        };
    }
    // PoC 4 and PoC 5 have no configured production providers. Resolve that
    // static availability posture before any other effect.
    if action == "desktop_prompt_observe" {
        return match handle_desktop_prompt_observe(cmd).await {
            Ok(data) => success_response(&id, data),
            Err(error) => error_response(&id, &error),
        };
    }
    if action == "desktop_interact" {
        return match handle_desktop_interact(cmd).await {
            Ok(data) => success_response(&id, data),
            Err(error) => error_response(&id, &error),
        };
    }
    #[cfg(test)]
    if action == "__test_sleep" {
        let ms = cmd.get("ms").and_then(|value| value.as_u64()).unwrap_or(1);
        tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
        return success_response(&id, json!({ "sleptMs" : ms }));
    }
    if let Some(ref server) = state.stream_server {
        server.broadcast_command(action, &id, cmd);
    }
    state.drain_cdp_events_background().await;
    if let Some(ref mut policy) = state.policy {
        let _ = policy.reload();
        match policy.check(action) {
            PolicyResult::Allow => {}
            PolicyResult::Deny(reason) => {
                return error_response(
                    &id,
                    &format!("Action '{}' denied by policy: {}", action, reason),
                );
            }
            PolicyResult::RequiresConfirmation => {
                state.pending_confirmation = Some(PendingConfirmation {
                    action: action.to_string(),
                    cmd: cmd.clone(),
                });
                return json!(
                    { "id" : id, "success" : true, "data" : { "confirmation_required" :
                    true, "action" : action }, }
                );
            }
        }
    }
    if action != "confirm" && action != "deny" {
        if let Some(ref ca) = state.confirm_actions {
            if ca.requires_confirmation(action) {
                state.pending_confirmation = Some(PendingConfirmation {
                    action: action.to_string(),
                    cmd: cmd.clone(),
                });
                return json!(
                    { "id" : id, "success" : true, "data" : { "confirmation_required" :
                    true, "confirmation_id" : id, "action" : action, }, }
                );
            }
        }
    }
    if action == "dependent_batch" {
        let action_started = std::time::Instant::now();
        let mut response = match handle_dependent_batch(cmd, state).await {
            Ok(data) => success_response(&id, data),
            Err(error) => error_response(&id, &error),
        };
        if cmd
            .get("includeTimings")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let action_execution_ms =
                u64::try_from(action_started.elapsed().as_millis()).unwrap_or(u64::MAX);
            let serialization_started = std::time::Instant::now();
            let _ = serde_json::to_vec(&response);
            let response_serialization_ms =
                u64::try_from(serialization_started.elapsed().as_millis()).unwrap_or(u64::MAX);
            let daemon_total_ms =
                u64::try_from(cmd_start.elapsed().as_millis()).unwrap_or(u64::MAX);
            response["timings"] = json!(
                { "commandPreparationMs" : daemon_total_ms
                .saturating_sub(action_execution_ms), "actionExecutionMs" :
                action_execution_ms, "responseSerializationMs" :
                response_serialization_ms, "daemonTotalMs" : daemon_total_ms, }
            );
        }
        return response;
    }
    let skip_launch = action_skips_browser_launch(action)
        || (action == "evaluate" && cmd.get("serviceTabHandle").is_some());
    if !skip_launch {
        if let Some(blocker) = active_manual_seeding_cdp_blocker(cmd, state) {
            return error_response(&id, &blocker);
        }
    }
    let mut cold_owned_launch = false;
    if !skip_launch {
        let stale_state = detect_browser_stale_state(state).await;
        let mut needs_launch = stale_state.needs_launch;
        if needs_launch
            && state.browser.is_some()
            && state
                .try_recover_browser_connection()
                .await
                .unwrap_or(false)
        {
            needs_launch = false;
        }
        if needs_launch {
            let mut recovery_persistence = BrowserRecoveryPersistence::NotRecorded;
            if state.browser.is_some() {
                if let (Some(health), Some(reason_kind), Some(message)) = (
                    stale_state.health,
                    stale_state.recovery_reason_kind,
                    stale_state.message,
                ) {
                    recovery_persistence = persist_current_browser_stale_health(
                        state,
                        health,
                        reason_kind,
                        message,
                        stale_state.event_details,
                    );
                }
                state.close_behavior = CloseBehavior::CloseBrowser;
                if let Err(error) = handle_recovery_close(state).await {
                    return error_response(&id, &error);
                }
            }
            if !recovery_persistence.recorded() {
                recovery_persistence = persist_browser_recovery_started_from_persisted_state(
                    state,
                    "Browser relaunch requested from persisted unhealthy state",
                );
            }
            if let BrowserRecoveryPersistence::Blocked(reason) = recovery_persistence {
                return error_response(&id, &reason);
            }
            if let Err(e) = auto_launch(state, cmd).await {
                return error_response(&id, &format!("Auto-launch failed: {}", e));
            }
            cold_owned_launch = state
                .browser
                .as_ref()
                .is_some_and(|browser| browser.owns_launched_browser_process());
        }
        if let Some(ref mut mgr) = state.browser {
            if mgr.page_count() == 0 {
                let _ = mgr.ensure_page().await;
            }
        }
        if let Some(mismatch) = active_browser_profile_mismatch(cmd, state) {
            return error_response(&id, &mismatch);
        }
    }
    if matches!(state.backend_type, BackendType::WebDriver)
        && WEBDRIVER_UNSUPPORTED_ACTIONS.contains(&action)
    {
        return error_response(
            &id,
            &format!(
                "Action '{}' is not supported on the WebDriver backend",
                action
            ),
        );
    }
    let renderer_crash_context = state.renderer_crash_command_context(cmd);
    let mut renderer_crash_rx = state
        .browser
        .as_ref()
        .map(|browser| browser.client.subscribe());
    let command_preparation_ms = u64::try_from(cmd_start.elapsed().as_millis()).unwrap_or(u64::MAX);
    let action_started = std::time::Instant::now();
    let action_race = race_renderer_crash!(
        match action {
            "launch" => handle_launch(cmd, state).await,
            "cdp_free_launch" => handle_cdp_free_launch(cmd, state).await,
            "external_byop_adopt" => handle_external_byop_adopt(cmd, state).await,
            "cdp_attach" => handle_cdp_attach(cmd, state).await,
            "cdp_detach" => handle_cdp_detach(cmd, state).await,
            "diagnostics" => handle_service_diagnostics(cmd, state).await,
            "desktop_capture" => handle_desktop_capture(cmd).await,
            "desktop_locate" => handle_desktop_locate(cmd).await,
            "desktop_interact" => handle_desktop_interact(cmd).await,
            "probe" => handle_service_probe(cmd, state).await,
            "ui_action" => handle_service_ui_action(cmd, state).await,
            "network_capture" => handle_service_network_capture(cmd, state).await,
            "file_transfer" => handle_service_file_transfer(cmd, state).await,
            "navigate" => handle_navigate(cmd, state).await,
            "url" => handle_url(state).await,
            "browser_pid" => handle_browser_pid(state),
            "cdp_url" => handle_cdp_url(state),
            "inspect" => handle_inspect(state).await,
            "title" => handle_title(state).await,
            "content" => handle_content(state).await,
            "evaluate" => handle_evaluate(cmd, state).await,
            "runtime_handoff_prepare" => handle_runtime_handoff_prepare(state).await,
            "runtime_handoff_abort" => handle_runtime_handoff_abort(state),
            "runtime_handoff_resume" => handle_runtime_handoff_resume(cmd, state).await,
            "runtime_handoff_rollback" => handle_runtime_handoff_rollback(cmd, state).await,
            "runtime_handoff_finalize" => handle_runtime_handoff_finalize(state).await,
            "close" => handle_close(state).await,
            "snapshot" => handle_snapshot(cmd, state).await,
            "screenshot" => handle_screenshot(cmd, state).await,
            "click" => handle_click(cmd, state).await,
            "dblclick" => handle_dblclick(cmd, state).await,
            "fill" => handle_fill(cmd, state).await,
            "type" => handle_type(cmd, state).await,
            "press" => handle_press(cmd, state).await,
            "hover" => handle_hover(cmd, state).await,
            "scroll" => handle_scroll(cmd, state).await,
            "select" => handle_select(cmd, state).await,
            "check" => handle_check(cmd, state).await,
            "uncheck" => handle_uncheck(cmd, state).await,
            "wait" => handle_wait(cmd, state).await,
            "gettext" => handle_gettext(cmd, state).await,
            "getattribute" => handle_getattribute(cmd, state).await,
            "isvisible" => handle_isvisible(cmd, state).await,
            "isenabled" => handle_isenabled(cmd, state).await,
            "ischecked" => handle_ischecked(cmd, state).await,
            "back" => handle_back(state).await,
            "forward" => handle_forward(state).await,
            "reload" => handle_reload(state).await,
            "cookies_get" => handle_cookies_get(cmd, state).await,
            "cookies_set" => handle_cookies_set(cmd, state).await,
            "cookies_clear" => handle_cookies_clear(state).await,
            "storage_get" => handle_storage_get(cmd, state).await,
            "storage_set" => handle_storage_set(cmd, state).await,
            "storage_clear" => handle_storage_clear(cmd, state).await,
            "setcontent" => handle_setcontent(cmd, state).await,
            "headers" => handle_headers(cmd, state).await,
            "offline" => handle_offline(cmd, state).await,
            "console" => handle_console(cmd, state).await,
            "errors" => handle_errors(state).await,
            "state_save" => handle_state_save(cmd, state).await,
            "state_load" => handle_state_load(cmd, state).await,
            "state_list" | "state_show" | "state_clear" | "state_clean" | "state_rename" => {
                state::dispatch_state_command(cmd)
                    .expect("dispatch_state_command must handle all state_* actions matched here")
            }
            "trace_start" => handle_trace_start(state).await,
            "trace_stop" => handle_trace_stop(cmd, state).await,
            "profiler_start" => handle_profiler_start(cmd, state).await,
            "profiler_stop" => handle_profiler_stop(cmd, state).await,
            "recording_start" => handle_recording_start(cmd, state).await,
            "recording_stop" => handle_recording_stop(state).await,
            "recording_restart" => handle_recording_restart(cmd, state).await,
            "pdf" => handle_pdf(cmd, state).await,
            "tab_list" => handle_tab_list(cmd, state).await,
            "tab_new" => handle_tab_new_with_cold_launch(cmd, state, cold_owned_launch).await,
            "tab_switch" => handle_tab_switch(cmd, state).await,
            "tab_close" => handle_tab_close(cmd, state).await,
            "tab_handle_refresh" => handle_tab_handle_refresh(cmd, state).await,
            "tab_handle_release" => handle_tab_handle_release(cmd, state).await,
            "view_focus" => handle_view_focus(cmd, state).await,
            "view_takeover" => handle_view_takeover(cmd, state).await,
            "remote_view_open" => {
                let attribution = route_bound_open_attribution_from_authenticated_dispatch(cmd);
                handle_remote_view_open(cmd, state, attribution).await
            }
            "service_profile_manual_seeding_acquire" => {
                let attribution = route_bound_open_attribution_from_authenticated_dispatch(cmd);
                handle_service_profile_manual_seeding_acquire(cmd, state, attribution).await
            }
            "service_profile_manual_seeding_close" => {
                handle_service_profile_manual_seeding_close(cmd, state).await
            }
            "service_remote_view_handoff_resolve" => {
                let attribution = route_bound_open_attribution_from_authenticated_dispatch(cmd);
                handle_service_remote_view_handoff_resolve(cmd, state, attribution).await
            }
            "service_remote_view_route_preflight" => {
                handle_service_remote_view_route_preflight(cmd, state).await
            }
            "service_remote_view_browser_reattach" => {
                handle_service_remote_view_browser_reattach(cmd, state, false).await
            }
            "service_remote_view_route_switch" => {
                handle_service_remote_view_browser_reattach(cmd, state, true).await
            }
            "service_remote_view_route_checkout" => {
                handle_service_remote_view_route_checkout(cmd, state).await
            }
            "service_remote_view_route_release" => {
                handle_service_remote_view_route_release(cmd, state).await
            }
            "service_route_pool_repair" => handle_service_route_pool_repair(cmd).await,
            "service_viewer_lease_request" => handle_service_viewer_lease_request(cmd, state).await,
            "service_viewer_lease_heartbeat" =>
                handle_service_viewer_lease_heartbeat(cmd, state).await,
            "service_viewer_lease_release" => handle_service_viewer_lease_release(cmd, state).await,
            "service_controller_lease_takeover" => {
                handle_service_controller_lease_takeover(cmd, state).await
            }
            "viewport" => handle_viewport(cmd, state).await,
            "useragent" | "user_agent" => handle_user_agent(cmd, state).await,
            "set_media" => handle_set_media(cmd, state).await,
            "download" => handle_download(cmd, state).await,
            "diff_snapshot" => handle_diff_snapshot(cmd, state).await,
            "diff_url" => handle_diff_url(cmd, state).await,
            "credentials_set" => handle_credentials_set(cmd).await,
            "credentials_get" => handle_credentials_get(cmd).await,
            "credentials_delete" => handle_credentials_delete(cmd).await,
            "credentials_list" => handle_credentials_list().await,
            "mouse" => handle_mouse(cmd, state).await,
            "keyboard" => handle_keyboard(cmd, state).await,
            "focus" => handle_focus(cmd, state).await,
            "clear" => handle_clear(cmd, state).await,
            "selectall" => handle_selectall(cmd, state).await,
            "scrollintoview" => handle_scrollintoview(cmd, state).await,
            "dispatch" => handle_dispatch(cmd, state).await,
            "highlight" => handle_highlight(cmd, state).await,
            "tap" => handle_tap(cmd, state).await,
            "boundingbox" => handle_boundingbox(cmd, state).await,
            "innertext" => handle_innertext(cmd, state).await,
            "innerhtml" => handle_innerhtml(cmd, state).await,
            "inputvalue" => handle_inputvalue(cmd, state).await,
            "setvalue" => handle_setvalue(cmd, state).await,
            "count" => handle_count(cmd, state).await,
            "styles" => handle_styles(cmd, state).await,
            "bringtofront" => handle_bringtofront(state).await,
            "timezone" => handle_timezone(cmd, state).await,
            "locale" => handle_locale(cmd, state).await,
            "geolocation" => handle_geolocation(cmd, state).await,
            "permissions" => handle_permissions(cmd, state).await,
            "dialog" => handle_dialog(cmd, state).await,
            "upload" => handle_upload(cmd, state).await,
            "addscript" => handle_addscript(cmd, state).await,
            "addinitscript" => handle_addinitscript(cmd, state).await,
            "addstyle" => handle_addstyle(cmd, state).await,
            "clipboard" => handle_clipboard(cmd, state).await,
            "wheel" => handle_wheel(cmd, state).await,
            "device" => handle_device(cmd, state).await,
            "screencast_start" => handle_screencast_start(cmd, state).await,
            "screencast_stop" => handle_screencast_stop(state).await,
            "stream_enable" => handle_stream_enable(cmd, state).await,
            "stream_disable" => handle_stream_disable(state).await,
            "stream_status" => handle_stream_status(state).await,
            "service_status" => handle_service_status(cmd).await,
            "service_reconcile" => handle_service_reconcile(cmd).await,
            "service_browser_close" => handle_service_browser_close(cmd, state).await,
            "service_browser_repair" => handle_service_browser_repair(cmd).await,
            "service_browser_contamination_report" =>
                handle_service_browser_retirement_command(cmd),
            "service_browser_retirement_plan" => handle_service_browser_retirement_command(cmd),
            "service_browser_retirement_apply" => handle_service_browser_retirement_command(cmd),
            "service_resources" => handle_service_resources(cmd).await,
            "service_resources_monitor_summary" => handle_service_resources_monitor_summary().await,
            "service_resources_write_monitor_summary" => {
                handle_service_resources_write_monitor_summary(cmd).await
            }
            "service_gc" => handle_service_gc(cmd).await,
            "service_prune_retained" => handle_service_prune_retained(cmd).await,
            "service_repair_retained" => handle_service_repair_retained(cmd).await,
            "service_access_plan" => handle_service_access_plan(cmd).await,
            "service_browser_capability_preflight" => {
                handle_service_browser_capability_preflight(cmd).await
            }
            "service_browser_capability_preference_guide" => {
                handle_service_browser_capability_preference_guide(cmd).await
            }
            "service_job_cancel" => handle_service_job_cancel(cmd).await,
            "service_browser_retry" => handle_service_browser_retry(cmd).await,
            "service_remedies_apply" => handle_service_remedies_apply(cmd).await,
            "service_profile_upsert" => handle_service_profile_upsert(cmd).await,
            "service_profile_freshness_update" =>
                handle_service_profile_freshness_update(cmd).await,
            "service_profile_seeding_handoff_update" => {
                handle_service_profile_seeding_handoff_update(cmd).await
            }
            "service_profile_delete" => handle_service_profile_delete(cmd).await,
            "service_session_upsert" => handle_service_session_upsert(cmd).await,
            "service_session_delete" => handle_service_session_delete(cmd).await,
            "service_site_policy_upsert" => handle_service_site_policy_upsert(cmd).await,
            "service_site_policy_delete" => handle_service_site_policy_delete(cmd).await,
            "service_monitor_upsert" => handle_service_monitor_upsert(cmd).await,
            "service_monitor_delete" => handle_service_monitor_delete(cmd).await,
            "service_monitor_pause" => {
                handle_service_monitor_state_update(cmd, MonitorState::Paused).await
            }
            "service_monitor_reset_failures" => handle_service_monitor_reset_failures(cmd).await,
            "service_monitor_resume" => {
                handle_service_monitor_state_update(cmd, MonitorState::Active).await
            }
            "service_monitor_triage" => handle_service_monitor_triage(cmd).await,
            "service_monitors_run_due" => handle_service_monitors_run_due(cmd).await,
            "service_provider_upsert" => handle_service_provider_upsert(cmd).await,
            "service_provider_delete" => handle_service_provider_delete(cmd).await,
            "service_browser_capability_registry_upsert" => {
                handle_service_browser_capability_registry_upsert(cmd).await
            }
            "service_incident_acknowledge" => handle_service_incident_acknowledge(cmd).await,
            "service_incident_resolve" => handle_service_incident_resolve(cmd).await,
            "service_incident_activity" => handle_service_incident_activity(cmd).await,
            "service_trace" => handle_service_trace(cmd).await,
            "service_profiles" => handle_service_profiles(cmd).await,
            "service_profile_leases" => handle_service_profile_leases(cmd).await,
            "service_profile_lease_inspect" => handle_service_profile_lease_command(cmd).await,
            "service_profile_lease_explain" => handle_service_profile_lease_command(cmd).await,
            "service_profile_lease_doctor" => handle_service_profile_lease_command(cmd).await,
            "service_profile_capability_status" => handle_service_profile_lease_command(cmd).await,
            "service_profile_lease_register" => handle_service_profile_lease_command(cmd).await,
            "service_profile_capability_rotate" => handle_service_profile_lease_command(cmd).await,
            "service_profile_lease_rejoin" => handle_service_profile_lease_command(cmd).await,
            "service_profile_lease_renew" => handle_service_profile_lease_command(cmd).await,
            "service_profile_lease_release" => handle_service_profile_lease_command(cmd).await,
            "service_profile_lease_reconcile_plan" => {
                handle_service_profile_lease_command(cmd).await
            }
            "service_profile_lease_reconcile_apply" => {
                handle_service_profile_lease_command(cmd).await
            }
            "service_profile_lease_recover_plan" => {
                handle_service_profile_lease_command(cmd).await
            }
            "service_profile_lease_recover_apply" => {
                handle_service_profile_lease_command(cmd).await
            }
            "service_profile_acquire" => {
                handle_service_profile_recovery_command(cmd, state).await
            }
            "service_profile_recovery_plan" => {
                handle_service_profile_recovery_command(cmd, state).await
            }
            "service_profile_recovery_apply" => {
                handle_service_profile_recovery_command(cmd, state).await
            }
            "service_profile_recovery_status" => {
                handle_service_profile_recovery_command(cmd, state).await
            }
            "service_profile_lookup" => handle_service_profile_lookup(cmd).await,
            "service_profile_seeding_handoff" => handle_service_profile_seeding_handoff(cmd).await,
            "service_sessions" => handle_service_sessions(cmd).await,
            "service_browsers" => handle_service_browsers(cmd).await,
            "service_tabs" => handle_service_tabs(cmd).await,
            "service_monitors" => handle_service_monitors(cmd).await,
            "service_site_policies" => handle_service_site_policies(cmd).await,
            "service_providers" => handle_service_providers(cmd).await,
            "service_challenges" => handle_service_challenges(cmd).await,
            "service_jobs" => handle_service_jobs(cmd).await,
            "service_incidents" => handle_service_incidents(cmd).await,
            "service_events" => handle_service_events(cmd).await,
            "waitforurl" => handle_waitforurl(cmd, state).await,
            "waitforloadstate" => handle_waitforloadstate(cmd, state).await,
            "waitforfunction" => handle_waitforfunction(cmd, state).await,
            "frame" => handle_frame(cmd, state).await,
            "mainframe" => handle_mainframe(state).await,
            "getbyrole" => handle_getbyrole(cmd, state).await,
            "getbytext" => handle_getbytext(cmd, state).await,
            "getbylabel" => handle_getbylabel(cmd, state).await,
            "getbyplaceholder" => handle_getbyplaceholder(cmd, state).await,
            "getbyalttext" => handle_getbyalttext(cmd, state).await,
            "getbytitle" => handle_getbytitle(cmd, state).await,
            "getbytestid" => handle_getbytestid(cmd, state).await,
            "nth" => handle_nth(cmd, state).await,
            "find" => handle_find(cmd, state).await,
            "evalhandle" => handle_evalhandle(cmd, state).await,
            "drag" => handle_drag(cmd, state).await,
            "expose" => handle_expose(cmd, state).await,
            "pause" => handle_pause(state).await,
            "multiselect" => handle_multiselect(cmd, state).await,
            "responsebody" => handle_responsebody(cmd, state).await,
            "waitfordownload" => handle_waitfordownload(cmd, state).await,
            "window_new" => handle_window_new(cmd, state).await,
            "diff_screenshot" => handle_diff_screenshot(cmd, state).await,
            "video_start" => handle_video_start(cmd, state).await,
            "video_stop" => handle_video_stop(state).await,
            "har_start" => handle_har_start(state).await,
            "har_stop" => handle_har_stop(cmd, state).await,
            "route" => handle_route(cmd, state).await,
            "unroute" => handle_unroute(cmd, state).await,
            "requests" => handle_requests(cmd, state).await,
            "request_detail" => handle_request_detail(cmd, state).await,
            "credentials" => handle_http_credentials(cmd, state).await,
            "emulatemedia" => handle_set_media(cmd, state).await,
            "auth_save" => handle_auth_save(cmd).await,
            "auth_login" => handle_auth_login(cmd, state).await,
            "auth_list" => handle_credentials_list().await,
            "auth_delete" => handle_credentials_delete(cmd).await,
            "auth_show" => handle_auth_show(cmd).await,
            "confirm" => match begin_confirmation(state) {
                Ok(confirmation) => {
                    let command = confirmation.command().clone();
                    let result = Box::pin(execute_command(&command, state)).await;
                    Ok(confirmation.complete(state, result))
                }
                Err(error) => Err(error),
            },
            "deny" => handle_deny(cmd, state).await,
            "swipe" => handle_swipe(cmd, state).await,
            "device_list" => handle_device_list().await,
            "input_mouse" => handle_input_mouse(cmd, state).await,
            "input_keyboard" => handle_input_keyboard(cmd, state).await,
            "input_touch" => handle_input_touch(cmd, state).await,
            "keydown" => handle_keydown(cmd, state).await,
            "keyup" => handle_keyup(cmd, state).await,
            "inserttext" => handle_inserttext(cmd, state).await,
            "mousemove" => handle_mousemove(cmd, state).await,
            "mousedown" => handle_mousedown(cmd, state).await,
            "mouseup" => handle_mouseup(cmd, state).await,
            _ => Err(format!("Not yet implemented: {}", action)),
        },
        renderer_crash_rx.as_mut(),
        &renderer_crash_context,
    )
    .await;
    let (result, observed_renderer_crash) = match action_race {
        RendererCrashRace::Action(result) => (result, None),
        RendererCrashRace::Crash(observation) => (
            Err("The active renderer target crashed while the command was running".to_string()),
            Some(*observation),
        ),
    };
    let renderer_crash = state
        .drain_cdp_events_for_command(&renderer_crash_context)
        .await
        .or_else(|| {
            observed_renderer_crash.map(|observation| {
                let persistence = state.persist_renderer_crash_observation(&observation);
                (observation, persistence)
            })
        });
    let mut resp = if let Some((observation, persistence)) = renderer_crash {
        renderer_crash_error_response(&id, observation, persistence)
    } else {
        match result {
            Ok(mut data) => {
                let warning = take_response_warning(&mut data);
                let mut resp = success_response(&id, data);
                if let Some(warning) = warning {
                    if let Some(obj) = resp.as_object_mut() {
                        obj.insert("warning".to_string(), json!(warning));
                    }
                }
                resp
            }
            Err(e) if e == cancellation_error() => {
                json!(
                    { "id" : id, "success" : false, "error" : e, "data" : { "cancelled" :
                    true, }, }
                )
            }
            Err(e) => error_response(&id, &super::browser::to_ai_friendly_error(&e)),
        }
    };
    let action_execution_ms =
        u64::try_from(action_started.elapsed().as_millis()).unwrap_or(u64::MAX);
    if action != "dialog" {
        if let Some(ref dialog) = state.pending_dialog {
            if let Some(obj) = resp.as_object_mut() {
                obj.insert(
                    "warning".to_string(),
                    json!(
                        format!("A JavaScript {} dialog is blocking the page: \"{}\" — use `dialog accept` or `dialog dismiss` to resolve it",
                        dialog.dialog_type, dialog.message)
                    ),
                );
            }
        }
    }
    if let Some(ref server) = state.stream_server {
        let duration_ms = cmd_start.elapsed().as_millis() as u64;
        let success = resp
            .get("status")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s == "success");
        let response_data = resp.get("data").unwrap_or(&Value::Null);
        let data = match action {
            "desktop_capture" => redact_desktop_capture_stream_result(response_data),
            "desktop_locate" => redact_desktop_locate_stream_result(response_data),
            "desktop_evidence_observe" => redact_desktop_evidence_stream_result(response_data),
            "desktop_prompt_observe" => redact_desktop_prompt_stream_result(response_data),
            "desktop_interact" => redact_desktop_interaction_stream_result(response_data),
            _ => response_data.clone(),
        };
        server.broadcast_result(&id, action, success, &data, duration_ms);
        if let Some(ref mgr) = state.browser {
            server.broadcast_tabs(&mgr.tab_list(false)).await;
            if matches!(
                action,
                "tab_new" | "tab_switch" | "tab_close" | "open" | "navigate" | "view_focus"
            ) {
                let session_id = mgr.active_session_id().ok().map(|s| s.to_string());
                server.set_cdp_session_id(session_id).await;
                server.notify_client_changed();
            }
        }
    }
    if cmd
        .get("includeTimings")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let serialization_started = std::time::Instant::now();
        let _ = serde_json::to_vec(&resp);
        let response_serialization_ms =
            u64::try_from(serialization_started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let daemon_total_ms = u64::try_from(cmd_start.elapsed().as_millis()).unwrap_or(u64::MAX);
        if let Some(response) = resp.as_object_mut() {
            response.insert(
                "timings".to_string(),
                json!(
                    { "commandPreparationMs" : command_preparation_ms,
                    "actionExecutionMs" : action_execution_ms,
                    "responseSerializationMs" : response_serialization_ms,
                    "daemonTotalMs" : daemon_total_ms, }
                ),
            );
        }
    }
    resp
}

pub(crate) fn success_response(id: &str, data: Value) -> Value {
    json!({ "id" : id, "success" : true, "data" : data, })
}

pub(crate) fn error_response(id: &str, error: &str) -> Value {
    json!({ "id" : id, "success" : false, "error" : error, })
}
