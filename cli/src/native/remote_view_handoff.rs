use serde_json::{json, Map, Value};

use super::remote_view::{
    readiness_state, route_binding_readiness, RemoteViewAcquisitionPlan, RemoteViewOpenIntent,
    RemoteViewRouteBinding,
};
use super::remote_view_finalization::finalize_route_bound_acquisition;
use super::remote_view_lease::{RouteBoundLeaseLifecycle, RouteBoundLeaseState};
use super::remote_view_proof::{
    remote_view_operator_visible_state, remote_view_target_component_state,
};
use super::service_model::{
    ControlInputProvider, DisplayAllocation, RemoteViewAcquisitionLease, RemoteViewRoute,
    RoutePoolEntry, ViewStreamProvider,
};
use super::service_store::{
    JsonServiceStateStore, LockedServiceStateRepository, ServiceStateRepository,
};

pub struct RouteBoundHandoffPlannedResponseInput<'a> {
    pub intent: &'a RemoteViewOpenIntent,
    pub route_binding: &'a RemoteViewRouteBinding,
    pub acquisition_plan: &'a RemoteViewAcquisitionPlan,
    pub browser_id: &'a str,
    pub session_name: &'a str,
    pub managed_one_time_profile: &'a Value,
    pub one_time_profile_warning: &'a Value,
    pub operator_visible: &'a Value,
    pub launch_command: &'a Value,
    pub tab_command: &'a Value,
    pub checkout_command: &'a Value,
}

pub struct RouteBoundHandoffOpenedResponseInput<'a> {
    pub intent: &'a RemoteViewOpenIntent,
    pub planned_route_binding: &'a RemoteViewRouteBinding,
    pub final_route_binding: &'a RemoteViewRouteBinding,
    pub acquisition_plan: &'a RemoteViewAcquisitionPlan,
    pub browser_id: &'a str,
    pub session_name: &'a str,
    pub managed_one_time_profile: &'a Value,
    pub one_time_profile_warning: &'a Value,
    pub final_operator_visible: &'a Value,
    pub pre_checkout_operator_visible: &'a Value,
    pub browser_build_proof: &'a Value,
    pub launch: &'a Value,
    pub tab: &'a Value,
    pub focus: &'a Value,
    pub checkout: &'a Value,
    pub acquisition_lease: &'a Value,
    pub display_access_grant: &'a Value,
    pub reused_current_browser: bool,
    pub visible_window_proof: &'a Value,
}

pub struct CompleteRouteBoundHandoffOpenInput<'a> {
    pub intent: &'a RemoteViewOpenIntent,
    pub planned_route_binding: &'a RemoteViewRouteBinding,
    pub acquisition_plan: &'a RemoteViewAcquisitionPlan,
    pub repository: &'a LockedServiceStateRepository<JsonServiceStateStore>,
    pub lease: &'a RemoteViewAcquisitionLease,
    pub observed_at: &'a str,
    pub browser_id: &'a str,
    pub session_name: &'a str,
    pub managed_one_time_profile: &'a Value,
    pub one_time_profile_warning: &'a Value,
    pub final_operator_visible: &'a Value,
    pub pre_checkout_operator_visible: &'a Value,
    pub launch_command: &'a Value,
    pub launch: &'a Value,
    pub tab: &'a Value,
    pub focus: &'a Value,
    pub checkout: &'a Value,
    pub display_access_grant: &'a Value,
    pub reused_current_browser: bool,
    pub visible_window_proof: &'a Value,
}

pub struct RouteBoundHandoffRecordInput<'a> {
    pub state: &'a str,
    pub intent: &'a RemoteViewOpenIntent,
    pub route_binding: &'a RemoteViewRouteBinding,
    pub browser_id: &'a str,
    pub session_name: &'a str,
    pub operator_visible: &'a Value,
    pub tab: Option<&'a Value>,
    pub browser_build_proof: Option<&'a Value>,
}

pub struct RouteBoundHandoffSharedAcquisitionInput<'a> {
    pub state: &'a str,
    pub intent: &'a RemoteViewOpenIntent,
    pub route_binding: &'a RemoteViewRouteBinding,
    pub browser_id: &'a str,
    pub session_name: &'a str,
    pub tab: Option<&'a Value>,
    pub reused_current_browser: bool,
}

pub struct SharedProfileAcquisitionResultInput<'a> {
    pub state: Option<&'a str>,
    pub mode: &'a str,
    pub action: &'a str,
    pub recommended_action: Option<&'a str>,
    pub browser_reused: bool,
    pub tab_opened: bool,
    pub browser_id: &'a str,
    pub session_name: &'a str,
    pub profile_id: Option<&'a Value>,
    pub requested_profile: Option<&'a str>,
    pub planned_profile: Option<&'a str>,
    pub requested_browser_id: Option<&'a str>,
    pub requested_session_name: Option<&'a str>,
    pub route_hint_source: &'a str,
    pub route_hint_fields: &'a [&'a str],
    pub route_bound: bool,
    pub route_id: Option<&'a str>,
    pub display_allocation_id: Option<&'a str>,
    pub route_pool_entry_id: Option<&'a str>,
    pub provider: Option<Value>,
    pub provider_mode: Option<&'a str>,
    pub tab_acquisition_decision: Option<&'a str>,
}

pub struct BeginRouteBoundHandoffAcquisitionInput<'a> {
    pub inline_route_pool_entry: Option<&'a RoutePoolEntry>,
    pub acquisition_plan: &'a RemoteViewAcquisitionPlan,
    pub browser_id: &'a str,
    pub session_id: &'a str,
    pub observed_at: &'a str,
    pub default_control_input: Option<ControlInputProvider>,
}

pub struct RouteBoundHandoffFailureRollbackInput<'a> {
    pub lease: &'a RemoteViewAcquisitionLease,
    pub phase: &'a str,
    pub error: &'a str,
    pub cleanup: &'a Value,
    pub observed_at: &'a str,
}

pub struct RouteBoundHandoffFailureCleanupInput<'a> {
    pub lease_id: &'a str,
    pub rollback: &'a Value,
    pub cleanup: &'a Value,
    pub observed_at: &'a str,
}

pub struct RouteBoundHandoffFailureRecoveryInput<'a> {
    pub lease: &'a RemoteViewAcquisitionLease,
    pub phase: &'a str,
    pub error: &'a str,
    pub rollback_cleanup: &'a Value,
    pub launch: &'a Value,
    pub tab: Option<&'a Value>,
    pub observed_at: &'a str,
}

pub struct RouteBoundHandoffFailureRecovery {
    pub rollback: Value,
    pub cleanup_plan: RouteBoundHandoffFailureCleanupPlan,
    pub cleanup_task: RouteBoundHandoffFailureCleanupTask,
    pub skipped_cleanup: Option<Value>,
}

pub struct RouteBoundHandoffImmediateFailureInput<'a> {
    pub lease: &'a RemoteViewAcquisitionLease,
    pub phase: &'a str,
    pub error: &'a str,
    pub cleanup: &'a Value,
    pub observed_at: &'a str,
}

pub struct RouteBoundHandoffImmediateFailure {
    pub rollback: Value,
    pub summary: String,
}

pub struct RouteBoundHandoffFailureCleanupSummary {
    pub rollback: Value,
    pub summary: String,
}

pub struct RouteBoundHandoffProofFailure {
    pub error: String,
    pub cleanup: Value,
}

pub struct RouteBoundHandoffPostCheckoutProof {
    pub final_route_binding: RemoteViewRouteBinding,
    pub final_operator_visible: Value,
    pub failure: Option<RouteBoundHandoffProofFailure>,
}

pub struct RouteBoundHandoffRollbackFailure {
    pub phase: &'static str,
    pub cleanup: Value,
}

pub struct RouteBoundHandoffPostCheckoutProofInput<'a> {
    pub planned_route_binding: &'a RemoteViewRouteBinding,
    pub checkout: &'a Value,
    pub browser_id: &'a str,
    pub session_name: &'a str,
    pub pre_checkout_operator_visible: &'a Value,
    pub tab: Option<&'a Value>,
    pub expected_url: Option<&'a str>,
}

pub struct RouteBoundHandoffPlan {
    pub route_binding: RemoteViewRouteBinding,
    pub launch_command: Value,
    pub tab_command: Value,
    pub checkout_command: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteBoundHandoffFailureCleanupPlan {
    CloseOpenedTab { index: u64 },
    CloseNewBrowser,
    SkipExistingBrowserReused { reason: &'static str },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RouteBoundHandoffFailureCleanupTask {
    CloseOpenedTab { index: u64, command: Value },
    CloseNewBrowser { command: Value },
    Skipped { cleanup: Value },
}

pub fn route_bound_handoff_plan(
    cmd: &Value,
    acquisition_plan: &RemoteViewAcquisitionPlan,
    browser_id: &str,
    session_id: &str,
) -> RouteBoundHandoffPlan {
    let route_binding =
        normalize_route_bound_handoff_route_binding(acquisition_plan.route_binding.clone());
    let launch_command = route_bound_handoff_launch_command(cmd, &route_binding);
    let tab_command = route_bound_handoff_tab_command(cmd, browser_id, session_id);
    let checkout_command =
        route_bound_handoff_checkout_command(cmd, &route_binding, browser_id, session_id);

    RouteBoundHandoffPlan {
        route_binding,
        launch_command,
        tab_command,
        checkout_command,
    }
}

pub fn route_bound_handoff_operator_visible(
    route_binding: &RemoteViewRouteBinding,
    browser_id: &str,
    session_name: &str,
    visible_window_proof: Option<&Value>,
    tab: Option<&Value>,
    expected_url: Option<&str>,
) -> Value {
    let proof_state = visible_window_proof
        .and_then(|proof| proof.get("state"))
        .and_then(Value::as_str)
        .unwrap_or("not_checked");
    let target_id = tab
        .and_then(|tab| tab.get("targetId"))
        .and_then(Value::as_str);
    let target_url = tab.and_then(|tab| tab.get("url")).and_then(Value::as_str);
    let target_title = tab.and_then(|tab| tab.get("title")).and_then(Value::as_str);
    let profile_id = tab
        .and_then(|tab| tab.get("profileId").or_else(|| tab.get("runtimeProfile")))
        .and_then(Value::as_str);
    let url_readiness = route_bound_handoff_target_url_readiness(expected_url, target_url);
    let route_state = route_bound_handoff_route_state(route_binding);
    let route_readiness_state = route_binding
        .readiness
        .as_ref()
        .and_then(readiness_state)
        .filter(|state| !state.trim().is_empty());
    let route_reason = route_binding
        .readiness
        .as_ref()
        .and_then(|readiness| readiness.get("reason").or_else(|| readiness.get("message")))
        .and_then(Value::as_str);
    let target_state = remote_view_target_component_state(tab.is_some(), target_id, url_readiness);
    let guacamole_state = route_bound_handoff_guacamole_state(route_binding);
    let guacamole_readiness_state = route_binding
        .readiness
        .as_ref()
        .and_then(|readiness| readiness.get("state"))
        .and_then(Value::as_str);
    let guacamole_reason = route_binding
        .readiness
        .as_ref()
        .and_then(|readiness| readiness.get("reason").or_else(|| readiness.get("message")))
        .and_then(Value::as_str);
    let guacamole_has_route =
        route_binding.frame_url.is_some() || route_binding.external_url.is_some();
    let operator_state =
        remote_view_operator_visible_state(route_state, proof_state, target_state, guacamole_state);

    json!({
        "state": operator_state,
        "browserId": browser_id,
        "sessionName": session_name,
        "routeId": route_binding.route_id,
        "routePoolEntryId": route_binding.route_pool_entry_id,
        "displayAllocationId": route_binding.display_allocation_id,
        "displayName": route_binding.launch_display_name,
        "displayIsolation": route_binding.display_isolation,
        "provider": route_binding.provider,
        "providerMode": route_binding.provider_mode,
        "proof": visible_window_proof.cloned(),
        "target": {
            "state": target_state,
            "targetId": target_id,
            "url": target_url,
            "expectedUrl": expected_url,
            "title": target_title,
            "profileId": profile_id,
            "urlReadiness": url_readiness,
        },
        "components": {
            "route": {
                "state": route_state,
                "routeId": route_binding.route_id,
                "routePoolEntryId": route_binding.route_pool_entry_id,
                "routePoolEntryState": route_binding.route_pool_entry_state,
                "currentRouteAllocationId": route_binding.current_route_allocation_id,
                "readinessState": route_readiness_state,
                "reason": route_reason,
                "readiness": route_binding.readiness.clone(),
            },
            "display": {
                "state": proof_state,
                "displayAllocationId": route_binding.display_allocation_id,
                "displayName": route_binding.launch_display_name,
                "displayIsolation": route_binding.display_isolation,
                "contentState": visible_window_proof
                    .and_then(|proof| proof.pointer("/displayContent/state"))
                    .and_then(Value::as_str),
            },
            "browser": {
                "state": "ready",
                "browserId": browser_id,
                "sessionName": session_name,
                "profileId": profile_id,
            },
            "tab": {
                "state": target_state,
                "targetId": target_id,
                "url": target_url,
                "expectedUrl": expected_url,
                "urlReadiness": url_readiness,
                "title": target_title,
                "targetReadiness": tab
                    .and_then(|tab| tab.get("targetReadiness"))
                    .cloned(),
                "urlReadbackAttempts": tab
                    .and_then(|tab| tab.get("urlReadbackAttempts"))
                    .cloned(),
                "targetSwitch": tab
                    .and_then(|tab| tab.get("targetSwitch"))
                    .cloned(),
                "targetNavigation": tab
                    .and_then(|tab| tab.get("targetNavigation"))
                    .cloned(),
                "targetReselection": tab
                    .and_then(|tab| tab.get("targetReselection"))
                    .cloned(),
                "tabAcquisitionDecision": tab
                    .and_then(|tab| tab.get("tabAcquisitionDecision"))
                    .cloned(),
            },
            "stream": {
                "state": proof_state,
                "provider": route_binding.provider,
                "providerMode": route_binding.provider_mode,
            },
            "guacamole": {
                "state": guacamole_state,
                "readinessState": guacamole_readiness_state,
                "reason": guacamole_reason,
                "hasRouteUrl": guacamole_has_route,
                "frameUrl": route_binding.frame_url,
                "externalUrl": route_binding.external_url,
                "connectionId": route_binding.connection_id,
                "connectionName": route_binding.connection_name,
                "readiness": route_binding.readiness.clone(),
            }
        },
    })
}

fn route_bound_handoff_route_state(route_binding: &RemoteViewRouteBinding) -> &'static str {
    let readiness_state = route_binding.readiness.as_ref().and_then(readiness_state);
    if readiness_state.as_deref().is_some_and(|state| {
        matches!(
            state,
            "stale" | "stale_route_record" | "stale_route_pool_checkout"
        )
    }) {
        return "stale_route_record";
    }
    if route_binding
        .route_pool_entry_state
        .as_deref()
        .is_some_and(|state| state == "stale")
    {
        return "stale_route_record";
    }
    if let Some(current_route_allocation_id) = route_binding.current_route_allocation_id.as_deref()
    {
        if current_route_allocation_id != route_binding.route_id {
            return "stale_route_record";
        }
    }
    "ready"
}

fn route_bound_handoff_guacamole_state(route_binding: &RemoteViewRouteBinding) -> &'static str {
    let readiness_state = route_binding
        .readiness
        .as_ref()
        .and_then(|readiness| readiness.get("state"))
        .and_then(Value::as_str);
    if readiness_state.is_some_and(|state| state != "ready") {
        return "guacamole_route_unavailable";
    }
    if route_binding.frame_url.is_some()
        || route_binding.external_url.is_some()
        || readiness_state == Some("ready")
    {
        return "ready";
    }
    if matches!(&route_binding.provider, ViewStreamProvider::RdpGateway) {
        "guacamole_route_unavailable"
    } else {
        "not_checked"
    }
}

pub fn route_bound_handoff_target_url_readiness(
    expected_url: Option<&str>,
    actual_url: Option<&str>,
) -> &'static str {
    let Some(expected_url) = route_bound_handoff_normalized_url(expected_url) else {
        return "not_checked";
    };
    let Some(actual_url) = route_bound_handoff_normalized_url(actual_url) else {
        return "target_url_missing";
    };
    if expected_url == actual_url
        || actual_url.starts_with(&(expected_url.clone() + "/"))
        || expected_url.starts_with(&(actual_url.clone() + "/"))
    {
        "ready"
    } else {
        "wrong_tab"
    }
}

fn route_bound_handoff_normalized_url(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.trim_end_matches('/').to_ascii_lowercase())
}

pub fn begin_route_bound_handoff_plan_acquisition(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    inline_route_pool_entry: Option<&RoutePoolEntry>,
    acquisition_plan: &RemoteViewAcquisitionPlan,
    browser_id: &str,
    session_id: &str,
    observed_at: &str,
) -> Result<RemoteViewAcquisitionLease, String> {
    begin_route_bound_handoff_acquisition(
        repository,
        BeginRouteBoundHandoffAcquisitionInput {
            inline_route_pool_entry,
            acquisition_plan,
            browser_id,
            session_id,
            observed_at,
            default_control_input: route_bound_handoff_default_control_input_provider(
                acquisition_plan.requested_view_stream_provider,
            ),
        },
    )
}

pub fn complete_route_bound_handoff_plan_acquisition(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    lease: &RemoteViewAcquisitionLease,
    checkout: &Value,
    observed_at: &str,
) -> Result<RemoteViewAcquisitionLease, String> {
    restore_route_bound_handoff_lease_if_missing(repository, lease)?;
    complete_route_bound_handoff_acquisition(repository, &lease.id, checkout, observed_at)
}

fn route_bound_handoff_default_control_input_provider(
    provider: super::service_model::ViewStreamProvider,
) -> Option<ControlInputProvider> {
    let input = match provider {
        super::service_model::ViewStreamProvider::CdpScreencast => ControlInputProvider::CdpInput,
        super::service_model::ViewStreamProvider::ChromeTabWebrtc
        | super::service_model::ViewStreamProvider::VirtualDisplayWebrtc => {
            ControlInputProvider::WebrtcInput
        }
        super::service_model::ViewStreamProvider::Novnc => ControlInputProvider::VncInput,
        super::service_model::ViewStreamProvider::RdpGateway
        | super::service_model::ViewStreamProvider::ExternalUrl => {
            ControlInputProvider::ManualAttachedDesktop
        }
    };
    Some(input)
}

fn normalize_route_bound_handoff_route_binding(
    mut route_binding: RemoteViewRouteBinding,
) -> RemoteViewRouteBinding {
    let stale_acquisition_pending = route_binding
        .readiness
        .as_ref()
        .and_then(|readiness| readiness.get("state"))
        .and_then(Value::as_str)
        .is_some_and(|state| state == "pending")
        && route_binding
            .readiness
            .as_ref()
            .and_then(|readiness| readiness.get("component"))
            .and_then(Value::as_str)
            .is_some_and(|component| component == "remote_view_open_acquisition")
        && route_binding.current_route_allocation_id.is_none()
        && route_binding
            .route_pool_entry_state
            .as_deref()
            .is_some_and(|state| state == "available");
    if stale_acquisition_pending {
        route_binding.readiness = Some(route_binding_readiness(&route_binding));
    }
    route_binding
}

pub fn route_bound_handoff_launch_command(
    cmd: &Value,
    route_binding: &RemoteViewRouteBinding,
) -> Value {
    let mut command = command_object_with_action(cmd, "launch");
    command.remove("provider");
    command.insert("headless".to_string(), Value::Bool(false));
    command.insert(
        "browserHost".to_string(),
        Value::String("remote_headed".to_string()),
    );
    command.insert(
        "displayIsolation".to_string(),
        Value::String(route_binding.display_isolation.clone()),
    );
    command.insert(
        "viewStreamProvider".to_string(),
        json!(route_binding.provider),
    );
    command.insert(
        "controlInput".to_string(),
        Value::String("manual_attached_desktop".to_string()),
    );
    command.insert(
        "displayAllocationId".to_string(),
        Value::String(route_binding.display_allocation_id.clone()),
    );
    command.insert(
        "routeId".to_string(),
        Value::String(route_binding.route_id.clone()),
    );
    command.insert(
        "providerMode".to_string(),
        Value::String(route_binding.provider_mode.clone()),
    );
    if let Some(value) = route_binding.launch_display_name.clone() {
        command.insert("remoteHeadedDisplay".to_string(), Value::String(value));
    }
    insert_route_bound_handoff_route_fields(&mut command, route_binding);
    Value::Object(command)
}

pub fn route_bound_handoff_reused_browser_launch_result(
    route_binding: &RemoteViewRouteBinding,
    browser_id: &str,
    session_id: &str,
) -> Value {
    json!({
        "status": "reused",
        "reused": true,
        "browserId": browser_id,
        "sessionName": session_id,
        "routeId": route_binding.route_id,
        "displayAllocationId": route_binding.display_allocation_id,
        "reason": "same_owner_checked_out_route",
    })
}

pub fn route_bound_handoff_tab_command(cmd: &Value, browser_id: &str, session_id: &str) -> Value {
    let mut command = command_object_with_action(cmd, "tab_new");
    command.insert(
        "browserId".to_string(),
        Value::String(browser_id.to_string()),
    );
    command.insert(
        "sessionName".to_string(),
        Value::String(session_id.to_string()),
    );
    if !command.contains_key("url") {
        command.insert("url".to_string(), Value::String("about:blank".to_string()));
    }
    Value::Object(command)
}

pub fn route_bound_handoff_focus_command(cmd: &Value, tab: &Value, session_id: &str) -> Value {
    let mut command = command_object_with_action(cmd, "view_focus");
    command.insert(
        "sessionName".to_string(),
        Value::String(session_id.to_string()),
    );
    command.insert("maximize".to_string(), Value::Bool(true));
    let has_target_id = if let Some(target_id) = tab.get("targetId").and_then(Value::as_str) {
        command.insert("targetId".to_string(), Value::String(target_id.to_string()));
        true
    } else {
        false
    };
    if !has_target_id {
        if let Some(index) = tab.get("index").and_then(Value::as_u64) {
            command.insert("index".to_string(), Value::Number(index.into()));
        }
    }
    Value::Object(command)
}

pub fn route_bound_handoff_checkout_command(
    cmd: &Value,
    route_binding: &RemoteViewRouteBinding,
    browser_id: &str,
    session_id: &str,
) -> Value {
    let mut command = command_object_with_action(cmd, "service_remote_view_route_checkout");
    command.insert(
        "browserId".to_string(),
        Value::String(browser_id.to_string()),
    );
    command.insert(
        "sessionName".to_string(),
        Value::String(session_id.to_string()),
    );
    command.insert(
        "displayAllocationId".to_string(),
        Value::String(route_binding.display_allocation_id.clone()),
    );
    command.insert(
        "routeId".to_string(),
        Value::String(route_binding.route_id.clone()),
    );
    command.insert("provider".to_string(), json!(route_binding.provider));
    command.insert(
        "providerMode".to_string(),
        Value::String(route_binding.provider_mode.clone()),
    );
    command.insert(
        "streamId".to_string(),
        Value::String("remote-headed-view".to_string()),
    );
    insert_route_bound_handoff_route_fields(&mut command, route_binding);
    Value::Object(command)
}

pub fn route_bound_handoff_checkout_command_with_visible_window_proof(
    checkout_command: &Value,
    visible_window_proof: &Value,
) -> Value {
    let mut command = checkout_command.clone();
    if let Some(checkout) = command.as_object_mut() {
        let display_content = visible_window_proof
            .get("displayContent")
            .cloned()
            .unwrap_or(Value::Null);
        checkout.insert(
            "readiness".to_string(),
            json!({
                "state": "ready",
                "component": "remote_view_open_visible_window",
                "displayContent": display_content,
            }),
        );
        if let Some(display_content) = visible_window_proof.get("displayContent").cloned() {
            checkout.insert("displayContent".to_string(), display_content);
        }
    }
    command
}

fn command_object_with_action(cmd: &Value, action: &str) -> Map<String, Value> {
    let mut command = cmd.as_object().cloned().unwrap_or_default();
    command.insert("action".to_string(), Value::String(action.to_string()));
    command.remove("dryRun");
    command
}

fn insert_route_bound_handoff_route_fields(
    command: &mut Map<String, Value>,
    route_binding: &RemoteViewRouteBinding,
) {
    if let Some(value) = route_binding.route_pool_entry_id.clone() {
        command.insert("routePoolEntryId".to_string(), Value::String(value));
    }
    if let Some(value) = route_binding.frame_url.clone() {
        command.insert("frameUrl".to_string(), Value::String(value));
    }
    if let Some(value) = route_binding.external_url.clone() {
        command.insert("externalUrl".to_string(), Value::String(value));
    }
    if let Some(value) = route_binding.connection_id.clone() {
        command.insert("connectionId".to_string(), Value::String(value));
    }
    if let Some(value) = route_binding.connection_name.clone() {
        command.insert("connectionName".to_string(), Value::String(value));
    }
    if let Some(value) = route_binding.route_descriptor.clone() {
        command.insert("routeDescriptor".to_string(), value);
    }
}

fn route_bound_handoff_lease_id(route_id: &str, session_id: &str, observed_at: &str) -> String {
    let compact = |value: &str| {
        value
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    };
    format!(
        "remote-view-open:{}:{}:{}",
        compact(session_id),
        compact(route_id),
        observed_at.replace([':', '.'], "-")
    )
}

pub fn route_bound_handoff_record(input: RouteBoundHandoffRecordInput<'_>) -> Value {
    let tab_profile_id = input
        .tab
        .and_then(|tab| tab.get("profileId").or_else(|| tab.get("runtimeProfile")))
        .and_then(Value::as_str);
    let profile_id = tab_profile_id
        .or(input.intent.runtime_profile.as_deref())
        .or(input.intent.profile.as_deref());
    let profile_source = if tab_profile_id.is_some() {
        "tab"
    } else if input.intent.runtime_profile.is_some() {
        "runtime_profile"
    } else if input.intent.profile.is_some() {
        "profile"
    } else {
        "unspecified"
    };

    json!({
        "state": input.state,
        "profile": {
            "id": profile_id,
            "source": profile_source,
            "runtimeProfile": input.intent.runtime_profile.as_deref(),
            "profile": input.intent.profile.as_deref(),
            "tabProfileId": tab_profile_id,
        },
        "browser": {
            "browserId": input.browser_id,
            "sessionName": input.session_name,
            "requestedBrowserBuild": input.intent.browser_build.as_deref(),
            "browserHost": input.intent.browser_host.as_str(),
            "browserBuildProof": input.browser_build_proof.cloned(),
        },
        "route": {
            "routeId": input.route_binding.route_id.as_str(),
            "routePoolEntryId": input.route_binding.route_pool_entry_id.as_deref(),
            "routePoolEntryState": input.route_binding.route_pool_entry_state.as_deref(),
            "currentRouteAllocationId": input.route_binding.current_route_allocation_id.as_deref(),
            "provider": &input.route_binding.provider,
            "providerMode": input.route_binding.provider_mode.as_str(),
            "connectionId": input.route_binding.connection_id.as_deref(),
            "connectionName": input.route_binding.connection_name.as_deref(),
            "frameUrl": input.route_binding.frame_url.as_deref(),
            "externalUrl": input.route_binding.external_url.as_deref(),
            "routeDescriptor": input.route_binding.route_descriptor.as_ref(),
            "readiness": input.route_binding.readiness.as_ref(),
        },
        "display": {
            "displayAllocationId": input.route_binding.display_allocation_id.as_str(),
            "displayName": input.route_binding.launch_display_name.as_deref(),
            "displayIsolation": input.route_binding.display_isolation.as_str(),
            "displayAccess": input.route_binding.display_access.as_ref(),
        },
        "tab": input.tab.cloned(),
        "operatorVisible": input.operator_visible.clone(),
    })
}

pub fn shared_profile_acquisition_result(input: SharedProfileAcquisitionResultInput<'_>) -> Value {
    let profile_id = input.profile_id.cloned().unwrap_or(Value::Null);
    json!({
        "state": input.state,
        "policy": "shared_browser_tabs",
        "mode": input.mode,
        "action": input.action,
        "recommendedAction": input.recommended_action,
        "browserReused": input.browser_reused,
        "tabOpened": input.tab_opened,
        "waitedForProfileLease": false,
        "rejectedDuplicateProcess": false,
        "duplicateProcessAllowed": false,
        "duplicateProcessPolicy": "reject_duplicate_process",
        "browserId": input.browser_id,
        "sessionName": input.session_name,
        "profileId": profile_id,
        "requestedProfile": input.requested_profile,
        "plannedProfile": input.planned_profile,
        "requestedBrowserId": input.requested_browser_id,
        "requestedSessionName": input.requested_session_name,
        "requiresRouteHints": !input.route_hint_fields.is_empty(),
        "routeHintFields": input.route_hint_fields,
        "routeHintSource": input.route_hint_source,
        "controlSerialization": "service_queue",
        "cleanupPolicy": "client_tab",
        "routeBound": input.route_bound,
        "routeId": input.route_id,
        "displayAllocationId": input.display_allocation_id,
        "routePoolEntryId": input.route_pool_entry_id,
        "provider": input.provider,
        "providerMode": input.provider_mode,
        "tabAcquisitionDecision": input.tab_acquisition_decision,
    })
}

pub fn route_bound_handoff_shared_acquisition(
    input: RouteBoundHandoffSharedAcquisitionInput<'_>,
) -> Value {
    let tab_profile_id = input
        .tab
        .and_then(|tab| tab.get("profileId").or_else(|| tab.get("runtimeProfile")))
        .and_then(Value::as_str);
    let profile_id = tab_profile_id
        .or(input.intent.runtime_profile.as_deref())
        .or(input.intent.profile.as_deref());
    let profile_id_value = profile_id.map(|value| Value::String(value.to_string()));
    let tab_opened = input.tab.is_some();

    shared_profile_acquisition_result(SharedProfileAcquisitionResultInput {
        state: Some(input.state),
        mode: "remote_view_open",
        action: if tab_opened {
            "opened_route_bound_handoff"
        } else {
            "planned_route_bound_handoff"
        },
        recommended_action: Some("open_shared_profile_tab"),
        browser_reused: input.reused_current_browser,
        tab_opened,
        browser_id: input.browser_id,
        session_name: input.session_name,
        profile_id: profile_id_value.as_ref(),
        requested_profile: input
            .intent
            .runtime_profile
            .as_deref()
            .or(input.intent.profile.as_deref()),
        planned_profile: profile_id,
        requested_browser_id: None,
        requested_session_name: None,
        route_hint_source: "route_bound_handoff",
        route_hint_fields: &["browserId", "sessionName", "routeId", "displayAllocationId"],
        route_bound: true,
        route_id: Some(input.route_binding.route_id.as_str()),
        display_allocation_id: Some(input.route_binding.display_allocation_id.as_str()),
        route_pool_entry_id: input.route_binding.route_pool_entry_id.as_deref(),
        provider: Some(json!(input.route_binding.provider)),
        provider_mode: Some(input.route_binding.provider_mode.as_str()),
        tab_acquisition_decision: input
            .tab
            .and_then(|tab| tab.get("tabAcquisitionDecision"))
            .and_then(Value::as_str),
    })
}

pub fn begin_route_bound_handoff_acquisition(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    input: BeginRouteBoundHandoffAcquisitionInput<'_>,
) -> Result<RemoteViewAcquisitionLease, String> {
    let lease_id = route_bound_handoff_lease_id(
        &input.acquisition_plan.selected_route_id,
        input.session_id,
        input.observed_at,
    );
    let mut lifecycle = RouteBoundLeaseLifecycle::new();
    lifecycle
        .transition_to(RouteBoundLeaseState::Planned)
        .map_err(|error| error.to_string())?;
    lifecycle
        .transition_to(RouteBoundLeaseState::Reserved)
        .map_err(|error| error.to_string())?;
    let (lease_state, lease_phase) = lifecycle.state_phase();
    repository.mutate(|state| {
        if let Some(entry) = input.inline_route_pool_entry {
            state.route_pool.insert(entry.id.clone(), entry.clone());
        }
        let previous_route_pool_entry = input
            .acquisition_plan
            .selected_route_pool_entry_id
            .as_ref()
            .and_then(|id| state.route_pool.get(id).cloned());
        let previous_display_allocation = state
            .display_allocations
            .get(&input.acquisition_plan.display_allocation_id)
            .cloned();
        let previous_remote_view_route = state
            .remote_view_routes
            .get(&input.acquisition_plan.selected_route_id)
            .cloned();
        let previous_browser_display_allocation_id = state
            .browsers
            .get(input.browser_id)
            .and_then(|browser| browser.display_allocation_id.clone());

        if let Some(route_pool_entry_id) =
            input.acquisition_plan.selected_route_pool_entry_id.as_ref()
        {
            if let Some(entry) = state.route_pool.get_mut(route_pool_entry_id) {
                entry.state = "pending".to_string();
                entry.current_route_allocation_id =
                    Some(input.acquisition_plan.selected_route_id.clone());
                entry.readiness = Some(json!({
                    "state": "pending",
                    "component": "remote_view_open_acquisition",
                    "leaseId": lease_id,
                    "updatedAt": input.observed_at,
                }));
            }
        }

        let display_allocation = state
            .display_allocations
            .entry(input.acquisition_plan.display_allocation_id.clone())
            .or_insert_with(|| DisplayAllocation {
                id: input.acquisition_plan.display_allocation_id.clone(),
                display_name: input.acquisition_plan.display_name.clone(),
                display_isolation: input
                    .acquisition_plan
                    .route_binding
                    .display_isolation
                    .clone(),
                owner_browser_id: Some(input.browser_id.to_string()),
                owner_session_id: Some(input.session_id.to_string()),
                state: "pending".to_string(),
                created_at: Some(input.observed_at.to_string()),
                updated_at: Some(input.observed_at.to_string()),
                ..DisplayAllocation::default()
            });
        display_allocation.display_name = input.acquisition_plan.display_name.clone();
        display_allocation.display_isolation = input
            .acquisition_plan
            .route_binding
            .display_isolation
            .clone();
        display_allocation.owner_browser_id = Some(input.browser_id.to_string());
        display_allocation.owner_session_id = Some(input.session_id.to_string());
        display_allocation.state = "pending".to_string();
        display_allocation.updated_at = Some(input.observed_at.to_string());
        display_allocation.readiness = Some(json!({
            "state": "pending",
            "component": "remote_view_open_acquisition",
            "leaseId": lease_id,
            "updatedAt": input.observed_at,
        }));
        if !display_allocation
            .route_ids
            .contains(&input.acquisition_plan.selected_route_id)
        {
            display_allocation
                .route_ids
                .push(input.acquisition_plan.selected_route_id.clone());
        }

        state.remote_view_routes.insert(
            input.acquisition_plan.selected_route_id.clone(),
            RemoteViewRoute {
                id: input.acquisition_plan.selected_route_id.clone(),
                provider: input.acquisition_plan.requested_view_stream_provider,
                display_allocation_id: Some(input.acquisition_plan.display_allocation_id.clone()),
                browser_id: Some(input.browser_id.to_string()),
                session_id: Some(input.session_id.to_string()),
                route_source: if input
                    .acquisition_plan
                    .selected_route_pool_entry_id
                    .is_some()
                {
                    "pool".to_string()
                } else {
                    "retained_state".to_string()
                },
                connection_id: input.acquisition_plan.route_binding.connection_id.clone(),
                connection_name: input.acquisition_plan.route_binding.connection_name.clone(),
                frame_url: input.acquisition_plan.route_binding.frame_url.clone(),
                external_url: input.acquisition_plan.route_binding.external_url.clone(),
                route_descriptor: input
                    .acquisition_plan
                    .route_binding
                    .route_descriptor
                    .clone(),
                control_input: input.default_control_input,
                provider_mode: input.acquisition_plan.route_binding.provider_mode.clone(),
                state: "pending".to_string(),
                last_provider_event: Some("remote_view_open_acquisition_pending".to_string()),
                readiness: Some(json!({
                    "state": "pending",
                    "component": "remote_view_open_acquisition",
                    "leaseId": lease_id,
                    "updatedAt": input.observed_at,
                })),
                ..previous_remote_view_route.clone().unwrap_or_default()
            },
        );

        let lease = RemoteViewAcquisitionLease {
            id: lease_id.clone(),
            browser_id: input.browser_id.to_string(),
            session_id: input.session_id.to_string(),
            route_id: input.acquisition_plan.selected_route_id.clone(),
            display_allocation_id: input.acquisition_plan.display_allocation_id.clone(),
            route_pool_entry_id: input.acquisition_plan.selected_route_pool_entry_id.clone(),
            state: lease_state.to_string(),
            phase: lease_phase.to_string(),
            created_at: Some(input.observed_at.to_string()),
            updated_at: Some(input.observed_at.to_string()),
            previous_route_pool_entry,
            previous_display_allocation,
            previous_remote_view_route,
            previous_browser_display_allocation_id,
            ..RemoteViewAcquisitionLease::default()
        };
        state
            .remote_view_acquisition_leases
            .insert(lease_id, lease.clone());
        Ok(lease)
    })
}

pub fn planned_route_bound_handoff_response(
    input: RouteBoundHandoffPlannedResponseInput<'_>,
) -> Value {
    let route_bound_handoff = route_bound_handoff_record(RouteBoundHandoffRecordInput {
        state: "planned",
        intent: input.intent,
        route_binding: input.route_binding,
        browser_id: input.browser_id,
        session_name: input.session_name,
        operator_visible: input.operator_visible,
        tab: None,
        browser_build_proof: None,
    });
    let shared_acquisition =
        route_bound_handoff_shared_acquisition(RouteBoundHandoffSharedAcquisitionInput {
            state: "planned",
            intent: input.intent,
            route_binding: input.route_binding,
            browser_id: input.browser_id,
            session_name: input.session_name,
            tab: None,
            reused_current_browser: false,
        });

    json!({
        "status": "planned",
        "dryRun": true,
        "intent": input.intent,
        "browserId": input.browser_id,
        "sessionName": input.session_name,
        "routeId": input.route_binding.route_id,
        "displayAllocationId": input.route_binding.display_allocation_id,
        "routePoolEntryId": input.route_binding.route_pool_entry_id,
        "frameUrl": input.route_binding.frame_url,
        "externalUrl": input.route_binding.external_url,
        "routeDescriptor": input.route_binding.route_descriptor,
        "routeBinding": input.route_binding.clone(),
        "acquisitionPlan": input.acquisition_plan,
        "managedOneTimeProfile": input.managed_one_time_profile,
        "oneTimeProfileWarning": input.one_time_profile_warning,
        "operatorVisible": input.operator_visible,
        "sharedAcquisition": shared_acquisition,
        "routeBoundHandoff": route_bound_handoff,
        "launchCommand": input.launch_command,
        "tabCommand": input.tab_command,
        "checkoutCommand": input.checkout_command,
        "verification": {
            "routeBindingPlanned": true,
            "launchDisplayName": input.route_binding.launch_display_name,
            "displayIsolation": input.route_binding.display_isolation,
            "displayAccessGrant": "not_checked",
            "browserLaunchRequested": false,
            "tabOpenRequested": false,
            "routeCheckoutRequested": false,
            "visibleWindowProof": "not_checked"
        }
    })
}

pub fn opened_route_bound_handoff_response(
    input: RouteBoundHandoffOpenedResponseInput<'_>,
) -> Value {
    let route_bound_handoff = route_bound_handoff_record(RouteBoundHandoffRecordInput {
        state: "opened",
        intent: input.intent,
        route_binding: input.final_route_binding,
        browser_id: input.browser_id,
        session_name: input.session_name,
        operator_visible: input.final_operator_visible,
        tab: Some(input.tab),
        browser_build_proof: Some(input.browser_build_proof),
    });
    let shared_acquisition =
        route_bound_handoff_shared_acquisition(RouteBoundHandoffSharedAcquisitionInput {
            state: "opened",
            intent: input.intent,
            route_binding: input.final_route_binding,
            browser_id: input.browser_id,
            session_name: input.session_name,
            tab: Some(input.tab),
            reused_current_browser: input.reused_current_browser,
        });

    json!({
        "status": "opened",
        "dryRun": false,
        "intent": input.intent,
        "browserId": input.browser_id,
        "sessionName": input.session_name,
        "routeId": input.planned_route_binding.route_id,
        "displayAllocationId": input.planned_route_binding.display_allocation_id,
        "routePoolEntryId": input.planned_route_binding.route_pool_entry_id,
        "frameUrl": input.planned_route_binding.frame_url,
        "externalUrl": input.planned_route_binding.external_url,
        "routeDescriptor": input.planned_route_binding.route_descriptor,
        "routeBinding": input.final_route_binding.clone(),
        "acquisitionPlan": input.acquisition_plan,
        "managedOneTimeProfile": input.managed_one_time_profile,
        "oneTimeProfileWarning": input.one_time_profile_warning,
        "operatorVisible": input.final_operator_visible,
        "preCheckoutOperatorVisible": input.pre_checkout_operator_visible,
        "finalOperatorVisible": input.final_operator_visible,
        "sharedAcquisition": shared_acquisition,
        "routeBoundHandoff": route_bound_handoff,
        "browserBuildProof": input.browser_build_proof,
        "launch": input.launch,
        "tab": input.tab,
        "focus": input.focus,
        "checkout": input.checkout,
        "acquisitionLease": input.acquisition_lease,
        "verification": {
            "routeBindingPlanned": true,
            "launchDisplayName": input.planned_route_binding.launch_display_name,
            "displayIsolation": input.planned_route_binding.display_isolation,
            "displayAccessGrant": input.display_access_grant,
            "browserLaunchRequested": !input.reused_current_browser,
            "tabOpenRequested": true,
            "routeCheckoutRequested": true,
            "visibleWindowProof": input.visible_window_proof
        }
    })
}

pub fn complete_route_bound_handoff_open(
    input: CompleteRouteBoundHandoffOpenInput<'_>,
) -> Result<Value, String> {
    let final_route_binding =
        final_route_bound_handoff_route_binding(input.planned_route_binding, input.checkout);
    let acquisition_lease = complete_route_bound_handoff_plan_acquisition(
        input.repository,
        input.lease,
        input.checkout,
        input.observed_at,
    )?;
    let browser_build_proof =
        route_bound_handoff_browser_build_proof(input.intent, input.launch_command, input.launch);
    let acquisition_lease = serde_json::to_value(acquisition_lease)
        .map_err(|err| format!("route_bound_handoff_lease_serialize_failed: {err}"))?;

    Ok(opened_route_bound_handoff_response(
        RouteBoundHandoffOpenedResponseInput {
            intent: input.intent,
            planned_route_binding: input.planned_route_binding,
            final_route_binding: &final_route_binding,
            acquisition_plan: input.acquisition_plan,
            browser_id: input.browser_id,
            session_name: input.session_name,
            managed_one_time_profile: input.managed_one_time_profile,
            one_time_profile_warning: input.one_time_profile_warning,
            final_operator_visible: input.final_operator_visible,
            pre_checkout_operator_visible: input.pre_checkout_operator_visible,
            browser_build_proof: &browser_build_proof,
            launch: input.launch,
            tab: input.tab,
            focus: input.focus,
            checkout: input.checkout,
            acquisition_lease: &acquisition_lease,
            display_access_grant: input.display_access_grant,
            reused_current_browser: input.reused_current_browser,
            visible_window_proof: input.visible_window_proof,
        },
    ))
}

pub fn final_route_bound_handoff_route_binding(
    original: &RemoteViewRouteBinding,
    checkout: &Value,
) -> RemoteViewRouteBinding {
    let mut binding = checkout
        .get("routeBinding")
        .cloned()
        .and_then(|value| serde_json::from_value::<RemoteViewRouteBinding>(value).ok())
        .unwrap_or_else(|| original.clone());

    if let Some(route) = checkout.get("remoteViewRoute").and_then(Value::as_object) {
        if let Some(value) = route.get("id").and_then(Value::as_str) {
            binding.route_id = value.to_string();
        }
        if let Some(value) = route.get("displayAllocationId").and_then(Value::as_str) {
            binding.display_allocation_id = value.to_string();
        }
        if let Some(value) = route.get("connectionId").and_then(Value::as_str) {
            binding.connection_id = Some(value.to_string());
        }
        if let Some(value) = route.get("connectionName").and_then(Value::as_str) {
            binding.connection_name = Some(value.to_string());
        }
        if let Some(value) = route.get("frameUrl").and_then(Value::as_str) {
            binding.frame_url = Some(value.to_string());
        }
        if let Some(value) = route.get("externalUrl").and_then(Value::as_str) {
            binding.external_url = Some(value.to_string());
        }
        if let Some(value) = route.get("routeDescriptor") {
            binding.route_descriptor = Some(value.clone());
        }
        if let Some(value) = route.get("providerMode").and_then(Value::as_str) {
            binding.provider_mode = value.to_string();
        }
        if let Some(value) = route.get("readiness") {
            binding.readiness = Some(value.clone());
        }
    }

    if let Some(entry) = checkout.get("routePoolEntry").and_then(Value::as_object) {
        if let Some(value) = entry.get("id").and_then(Value::as_str) {
            binding.route_pool_entry_id = Some(value.to_string());
        }
        if let Some(value) = entry.get("state").and_then(Value::as_str) {
            binding.route_pool_entry_state = Some(value.to_string());
        }
        binding.current_route_allocation_id = entry
            .get("currentRouteAllocationId")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some(value) = entry.get("readiness") {
            binding.readiness = Some(value.clone());
        }
    }

    binding
}

pub fn route_bound_handoff_post_checkout_proof<F>(
    input: RouteBoundHandoffPostCheckoutProofInput<'_>,
    operator_visible: F,
) -> RouteBoundHandoffPostCheckoutProof
where
    F: FnOnce(&RemoteViewRouteBinding) -> Value,
{
    let final_route_binding =
        final_route_bound_handoff_route_binding(input.planned_route_binding, input.checkout);
    let final_operator_visible = operator_visible(&final_route_binding);
    let failure = route_bound_handoff_final_operator_visible_failure_if_not_ready(
        &final_route_binding,
        input.browser_id,
        input.session_name,
        &final_operator_visible,
        input.pre_checkout_operator_visible,
        input.tab,
        input.expected_url,
    );
    RouteBoundHandoffPostCheckoutProof {
        final_route_binding,
        final_operator_visible,
        failure,
    }
}

pub fn route_bound_handoff_failure_record(
    reason: &str,
    route_binding: &RemoteViewRouteBinding,
    browser_id: &str,
    session_name: &str,
    operator_visible: &Value,
    tab: Option<&Value>,
    expected_url: Option<&str>,
) -> Value {
    json!({
        "state": "failed",
        "reason": reason,
        "browser": {
            "browserId": browser_id,
            "sessionName": session_name,
        },
        "route": {
            "routeId": route_binding.route_id.as_str(),
            "routePoolEntryId": route_binding.route_pool_entry_id.as_deref(),
            "displayAllocationId": route_binding.display_allocation_id.as_str(),
            "displayName": route_binding.launch_display_name.as_deref(),
        },
        "target": {
            "expectedUrl": expected_url,
            "tab": tab.cloned(),
        },
        "operatorVisible": operator_visible.clone(),
    })
}

pub fn route_bound_handoff_cleanup_summary(cleanup: &Value, rollback: Option<&Value>) -> String {
    let summary = json!({
        "state": cleanup
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        "cleanup": cleanup,
        "leaseRollback": rollback,
    });
    serde_json::to_string(&summary).unwrap_or_else(|_| {
        cleanup
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string()
    })
}

pub fn route_bound_handoff_pending_rollback_cleanup(reason: &str) -> Value {
    json!({
        "state": "pending_after_rollback",
        "reason": reason,
    })
}

pub fn route_bound_handoff_pre_launch_failure_cleanup(reason: &str) -> Value {
    json!({
        "state": "skipped_before_browser_launch",
        "reason": reason,
    })
}

pub fn route_bound_handoff_launch_failure_cleanup(reason: &str) -> Value {
    json!({
        "state": "skipped_after_launch_failure",
        "reason": reason,
    })
}

pub fn route_bound_handoff_operator_visible_failure_cleanup(
    operator_visible: &Value,
    route_bound_handoff: &Value,
) -> Value {
    json!({
        "state": "pending_after_rollback",
        "reason": "operator_visible_proof_failed",
        "operatorVisible": operator_visible,
        "routeBoundHandoff": route_bound_handoff,
    })
}

pub fn route_bound_handoff_final_operator_visible_failure_cleanup(
    final_operator_visible: &Value,
    pre_checkout_operator_visible: &Value,
    route_bound_handoff: &Value,
) -> Value {
    json!({
        "state": "pending_after_rollback",
        "reason": "final_operator_visible_proof_failed",
        "finalOperatorVisible": final_operator_visible,
        "preCheckoutOperatorVisible": pre_checkout_operator_visible,
        "routeBoundHandoff": route_bound_handoff,
    })
}

pub fn route_bound_handoff_operator_visible_failure(
    route_binding: &RemoteViewRouteBinding,
    browser_id: &str,
    session_name: &str,
    operator_visible: &Value,
    tab: Option<&Value>,
    expected_url: Option<&str>,
) -> RouteBoundHandoffProofFailure {
    let route_bound_handoff = route_bound_handoff_failure_record(
        "operator_visible_proof_failed",
        route_binding,
        browser_id,
        session_name,
        operator_visible,
        tab,
        expected_url,
    );
    let failure_state = operator_visible
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let error = format!(
        "{}: route '{}' display '{}' operator-visible proof is not ready; routeBoundHandoff={}",
        failure_state,
        route_binding.route_id,
        route_binding
            .launch_display_name
            .as_deref()
            .unwrap_or("unknown"),
        route_bound_handoff,
    );
    let cleanup = route_bound_handoff_operator_visible_failure_cleanup(
        operator_visible,
        &route_bound_handoff,
    );
    RouteBoundHandoffProofFailure { error, cleanup }
}

pub fn route_bound_handoff_operator_visible_failure_if_not_ready(
    route_binding: &RemoteViewRouteBinding,
    browser_id: &str,
    session_name: &str,
    operator_visible: &Value,
    tab: Option<&Value>,
    expected_url: Option<&str>,
) -> Option<RouteBoundHandoffProofFailure> {
    operator_visible
        .get("state")
        .and_then(Value::as_str)
        .filter(|state| *state != "ready")
        .map(|_| {
            route_bound_handoff_operator_visible_failure(
                route_binding,
                browser_id,
                session_name,
                operator_visible,
                tab,
                expected_url,
            )
        })
}

pub fn route_bound_handoff_final_operator_visible_failure(
    route_binding: &RemoteViewRouteBinding,
    browser_id: &str,
    session_name: &str,
    final_operator_visible: &Value,
    pre_checkout_operator_visible: &Value,
    tab: Option<&Value>,
    expected_url: Option<&str>,
) -> RouteBoundHandoffProofFailure {
    let route_bound_handoff = route_bound_handoff_failure_record(
        "final_operator_visible_proof_failed",
        route_binding,
        browser_id,
        session_name,
        final_operator_visible,
        tab,
        expected_url,
    );
    let failure_state = final_operator_visible
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let error = format!(
        "{}: route '{}' display '{}' final operator-visible proof is not ready after route checkout; routeBoundHandoff={}; preCheckoutOperatorVisible={}",
        failure_state,
        route_binding.route_id,
        route_binding
            .launch_display_name
            .as_deref()
            .unwrap_or("unknown"),
        route_bound_handoff,
        pre_checkout_operator_visible,
    );
    let cleanup = route_bound_handoff_final_operator_visible_failure_cleanup(
        final_operator_visible,
        pre_checkout_operator_visible,
        &route_bound_handoff,
    );
    RouteBoundHandoffProofFailure { error, cleanup }
}

pub fn route_bound_handoff_final_operator_visible_failure_if_not_ready(
    route_binding: &RemoteViewRouteBinding,
    browser_id: &str,
    session_name: &str,
    final_operator_visible: &Value,
    pre_checkout_operator_visible: &Value,
    tab: Option<&Value>,
    expected_url: Option<&str>,
) -> Option<RouteBoundHandoffProofFailure> {
    final_operator_visible
        .get("state")
        .and_then(Value::as_str)
        .filter(|state| *state != "ready")
        .map(|_| {
            route_bound_handoff_final_operator_visible_failure(
                route_binding,
                browser_id,
                session_name,
                final_operator_visible,
                pre_checkout_operator_visible,
                tab,
                expected_url,
            )
        })
}

pub fn route_bound_handoff_checkout_failure() -> RouteBoundHandoffRollbackFailure {
    RouteBoundHandoffRollbackFailure {
        phase: "checkout_failed",
        cleanup: route_bound_handoff_pending_rollback_cleanup("checkout_failed"),
    }
}

pub fn route_bound_handoff_tab_open_failure() -> RouteBoundHandoffRollbackFailure {
    RouteBoundHandoffRollbackFailure {
        phase: "tab_open_failed",
        cleanup: route_bound_handoff_pending_rollback_cleanup("tab_open_failed"),
    }
}

pub fn route_bound_handoff_focus_failure() -> RouteBoundHandoffRollbackFailure {
    RouteBoundHandoffRollbackFailure {
        phase: "focus_failed",
        cleanup: route_bound_handoff_pending_rollback_cleanup("focus_failed"),
    }
}

pub fn route_bound_handoff_visible_window_proof_failure() -> RouteBoundHandoffRollbackFailure {
    RouteBoundHandoffRollbackFailure {
        phase: "proof_failed",
        cleanup: route_bound_handoff_pending_rollback_cleanup("proof_failed"),
    }
}

pub fn route_bound_handoff_failure_cleanup_plan(
    launch: &Value,
    tab: Option<&Value>,
) -> RouteBoundHandoffFailureCleanupPlan {
    if launch
        .get("reused")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return tab
            .and_then(|tab| tab.get("index"))
            .and_then(Value::as_u64)
            .map(|index| RouteBoundHandoffFailureCleanupPlan::CloseOpenedTab { index })
            .unwrap_or(
                RouteBoundHandoffFailureCleanupPlan::SkipExistingBrowserReused {
                    reason: "opened tab index unavailable",
                },
            );
    }

    RouteBoundHandoffFailureCleanupPlan::CloseNewBrowser
}

pub fn route_bound_handoff_skipped_failure_cleanup(
    plan: &RouteBoundHandoffFailureCleanupPlan,
) -> Option<Value> {
    match plan {
        RouteBoundHandoffFailureCleanupPlan::SkipExistingBrowserReused { reason } => Some(json!({
            "state": "skipped_existing_browser_reused",
            "reason": reason,
        })),
        _ => None,
    }
}

pub fn route_bound_handoff_failure_cleanup_task(
    plan: &RouteBoundHandoffFailureCleanupPlan,
) -> RouteBoundHandoffFailureCleanupTask {
    match plan {
        RouteBoundHandoffFailureCleanupPlan::CloseOpenedTab { index } => {
            RouteBoundHandoffFailureCleanupTask::CloseOpenedTab {
                index: *index,
                command: json!({ "index": index }),
            }
        }
        RouteBoundHandoffFailureCleanupPlan::CloseNewBrowser => {
            RouteBoundHandoffFailureCleanupTask::CloseNewBrowser {
                command: json!({ "action": "close" }),
            }
        }
        RouteBoundHandoffFailureCleanupPlan::SkipExistingBrowserReused { .. } => {
            RouteBoundHandoffFailureCleanupTask::Skipped {
                cleanup: route_bound_handoff_skipped_failure_cleanup(plan)
                    .unwrap_or_else(|| json!({ "state": "skipped_existing_browser_reused" })),
            }
        }
    }
}

pub fn route_bound_handoff_failure_cleanup_task_result(
    task: &RouteBoundHandoffFailureCleanupTask,
    result: Result<Value, String>,
) -> Value {
    match task {
        RouteBoundHandoffFailureCleanupTask::CloseOpenedTab { index, .. } => {
            route_bound_handoff_failure_cleanup_result(
                &RouteBoundHandoffFailureCleanupPlan::CloseOpenedTab { index: *index },
                result,
            )
        }
        RouteBoundHandoffFailureCleanupTask::CloseNewBrowser { .. } => {
            route_bound_handoff_failure_cleanup_result(
                &RouteBoundHandoffFailureCleanupPlan::CloseNewBrowser,
                result,
            )
        }
        RouteBoundHandoffFailureCleanupTask::Skipped { cleanup } => cleanup.clone(),
    }
}

pub fn route_bound_handoff_failure_cleanup_result(
    plan: &RouteBoundHandoffFailureCleanupPlan,
    result: Result<Value, String>,
) -> Value {
    match (plan, result) {
        (RouteBoundHandoffFailureCleanupPlan::CloseOpenedTab { index }, Ok(result)) => json!({
            "state": "closed_opened_tab",
            "index": index,
            "result": result,
        }),
        (RouteBoundHandoffFailureCleanupPlan::CloseOpenedTab { index }, Err(error)) => json!({
            "state": "failed_opened_tab_close",
            "index": index,
            "error": error,
        }),
        (RouteBoundHandoffFailureCleanupPlan::CloseNewBrowser, Ok(result)) => json!({
            "state": "closed_new_browser",
            "result": result,
        }),
        (RouteBoundHandoffFailureCleanupPlan::CloseNewBrowser, Err(error)) => json!({
            "state": "failed_new_browser_close",
            "error": error,
        }),
        (RouteBoundHandoffFailureCleanupPlan::SkipExistingBrowserReused { reason }, _) => json!({
            "state": "skipped_existing_browser_reused",
            "reason": reason,
        }),
    }
}

pub fn complete_route_bound_handoff_acquisition(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    lease_id: &str,
    checkout: &Value,
    observed_at: &str,
) -> Result<RemoteViewAcquisitionLease, String> {
    repository
        .mutate(|state| finalize_route_bound_acquisition(state, lease_id, checkout, observed_at))
}

pub fn restore_route_bound_handoff_lease_if_missing(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    lease: &RemoteViewAcquisitionLease,
) -> Result<(), String> {
    repository.mutate(|state| {
        state
            .remote_view_acquisition_leases
            .entry(lease.id.clone())
            .or_insert_with(|| lease.clone());
        Ok(())
    })
}

pub fn rollback_route_bound_handoff_acquisition(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    lease_id: &str,
    phase: &str,
    error: &str,
    cleanup: &Value,
    observed_at: &str,
) -> Result<Value, String> {
    repository.mutate(|state| {
        let lease_snapshot = state
            .remote_view_acquisition_leases
            .get(lease_id)
            .cloned()
            .ok_or_else(|| format!("remote_view_acquisition_lease_missing: {lease_id}"))?;

        match lease_snapshot.previous_route_pool_entry.clone() {
            Some(entry) => {
                state.route_pool.insert(entry.id.clone(), entry);
            }
            None => {
                if let Some(id) = lease_snapshot.route_pool_entry_id.as_ref() {
                    state.route_pool.remove(id);
                }
            }
        }
        match lease_snapshot.previous_display_allocation.clone() {
            Some(allocation) => {
                state
                    .display_allocations
                    .insert(allocation.id.clone(), allocation);
            }
            None => {
                state
                    .display_allocations
                    .remove(&lease_snapshot.display_allocation_id);
            }
        }
        match lease_snapshot.previous_remote_view_route.clone() {
            Some(route) => {
                state.remote_view_routes.insert(route.id.clone(), route);
            }
            None => {
                state.remote_view_routes.remove(&lease_snapshot.route_id);
            }
        }
        if let Some(browser) = state.browsers.get_mut(&lease_snapshot.browser_id) {
            browser.display_allocation_id =
                lease_snapshot.previous_browser_display_allocation_id.clone();
        }

        let rollback = json!({
            "state": "rolled_back",
            "leaseId": lease_id,
            "phase": phase,
            "routeId": lease_snapshot.route_id,
            "displayAllocationId": lease_snapshot.display_allocation_id,
            "routePoolEntryId": lease_snapshot.route_pool_entry_id,
            "restoredRoutePoolEntry": lease_snapshot.previous_route_pool_entry.is_some(),
            "restoredDisplayAllocation": lease_snapshot.previous_display_allocation.is_some(),
            "restoredRemoteViewRoute": lease_snapshot.previous_remote_view_route.is_some(),
            "restoredBrowserDisplayAllocation": lease_snapshot.previous_browser_display_allocation_id,
            "cleanup": cleanup,
            "updatedAt": observed_at,
        });
        if let Some(lease) = state.remote_view_acquisition_leases.get_mut(lease_id) {
            let mut lifecycle =
                RouteBoundLeaseLifecycle::from_state_phase(&lease.state, &lease.phase)
                    .unwrap_or_default();
            lifecycle
                .transition_to(RouteBoundLeaseState::RolledBack)
                .map_err(|error| error.to_string())?;
            let (lease_state, lease_phase) = lifecycle.state_phase();
            lease.state = lease_state.to_string();
            lease.phase = lease_phase.to_string();
            lease.updated_at = Some(observed_at.to_string());
            lease.failed_at = Some(observed_at.to_string());
            lease.failure_reason = Some(format!("{phase}: {error}"));
            lease.cleanup = Some(rollback.clone());
        }
        Ok(rollback)
    })
}

pub fn update_route_bound_handoff_acquisition_cleanup(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    lease_id: &str,
    rollback: &Value,
    cleanup: &Value,
    observed_at: &str,
) -> Result<Value, String> {
    let mut updated_rollback = rollback.clone();
    if let Some(object) = updated_rollback.as_object_mut() {
        object.insert("cleanup".to_string(), cleanup.clone());
        object.insert(
            "updatedAt".to_string(),
            Value::String(observed_at.to_string()),
        );
    }
    repository.mutate(|state| {
        if let Some(lease) = state.remote_view_acquisition_leases.get_mut(lease_id) {
            lease.updated_at = Some(observed_at.to_string());
            lease.cleanup = Some(updated_rollback.clone());
        }
        Ok(updated_rollback)
    })
}

pub fn rollback_route_bound_handoff_failure(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    input: RouteBoundHandoffFailureRollbackInput<'_>,
) -> Result<Value, String> {
    restore_route_bound_handoff_lease_if_missing(repository, input.lease)?;
    rollback_route_bound_handoff_acquisition(
        repository,
        &input.lease.id,
        input.phase,
        input.error,
        input.cleanup,
        input.observed_at,
    )
}

pub fn begin_route_bound_handoff_failure_recovery(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    input: RouteBoundHandoffFailureRecoveryInput<'_>,
) -> Result<RouteBoundHandoffFailureRecovery, String> {
    let rollback = rollback_route_bound_handoff_failure(
        repository,
        RouteBoundHandoffFailureRollbackInput {
            lease: input.lease,
            phase: input.phase,
            error: input.error,
            cleanup: input.rollback_cleanup,
            observed_at: input.observed_at,
        },
    )?;
    let cleanup_plan = route_bound_handoff_failure_cleanup_plan(input.launch, input.tab);
    let cleanup_task = route_bound_handoff_failure_cleanup_task(&cleanup_plan);
    let skipped_cleanup = route_bound_handoff_skipped_failure_cleanup(&cleanup_plan);
    Ok(RouteBoundHandoffFailureRecovery {
        rollback,
        cleanup_plan,
        cleanup_task,
        skipped_cleanup,
    })
}

pub fn route_bound_handoff_immediate_failure(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    input: RouteBoundHandoffImmediateFailureInput<'_>,
) -> Result<RouteBoundHandoffImmediateFailure, String> {
    let rollback = rollback_route_bound_handoff_failure(
        repository,
        RouteBoundHandoffFailureRollbackInput {
            lease: input.lease,
            phase: input.phase,
            error: input.error,
            cleanup: input.cleanup,
            observed_at: input.observed_at,
        },
    )?;
    let summary = route_bound_handoff_cleanup_summary(input.cleanup, Some(&rollback));
    Ok(RouteBoundHandoffImmediateFailure { rollback, summary })
}

pub fn complete_route_bound_handoff_failure_cleanup(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    input: RouteBoundHandoffFailureCleanupInput<'_>,
) -> Result<RouteBoundHandoffFailureCleanupSummary, String> {
    let rollback = update_route_bound_handoff_acquisition_cleanup(
        repository,
        input.lease_id,
        input.rollback,
        input.cleanup,
        input.observed_at,
    )?;
    let summary = route_bound_handoff_cleanup_summary(input.cleanup, Some(&rollback));
    Ok(RouteBoundHandoffFailureCleanupSummary { rollback, summary })
}

pub fn route_bound_handoff_browser_build_proof(
    intent: &RemoteViewOpenIntent,
    launch_command: &Value,
    launch: &Value,
) -> Value {
    let requested_build = intent.browser_build.as_deref();
    let launch_selection = launch_command.get("browserCapabilityLaunch");
    let selected_build = launch_selection
        .and_then(|value| value.get("browserBuild"))
        .and_then(Value::as_str)
        .or(requested_build);
    let actual_executable_path = launch_selection
        .and_then(|value| value.get("executablePath"))
        .and_then(Value::as_str)
        .or_else(|| launch.get("executablePath").and_then(Value::as_str));
    let applied = launch_selection
        .and_then(|value| value.get("applied"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mismatch_reason = route_bound_handoff_browser_build_mismatch_reason(
        requested_build,
        selected_build,
        actual_executable_path,
    );
    let state = if mismatch_reason.is_some() {
        "mismatch"
    } else if requested_build.is_none()
        && selected_build.is_none()
        && actual_executable_path.is_none()
    {
        "not_checked"
    } else {
        "matched"
    };

    json!({
        "state": state,
        "requestedBrowserBuild": requested_build,
        "selectedBrowserBuild": selected_build,
        "actualExecutablePath": actual_executable_path,
        "browserCapabilityApplied": applied,
        "browserCapabilityLaunch": launch_selection.cloned(),
        "mismatchReason": mismatch_reason,
    })
}

fn route_bound_handoff_browser_build_mismatch_reason(
    requested_build: Option<&str>,
    selected_build: Option<&str>,
    actual_executable_path: Option<&str>,
) -> Option<&'static str> {
    if let (Some(requested), Some(selected)) = (requested_build, selected_build) {
        if requested != selected {
            return Some("selected_build_mismatch");
        }
    }
    if requested_build == Some("stock_chrome")
        && actual_executable_path
            .map(|path| path.to_ascii_lowercase().contains("stealth"))
            .unwrap_or(false)
    {
        return Some("stock_chrome_resolved_to_stealth_executable");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserProcess, DisplayAllocation, RemoteViewRoute, ServiceState, ViewStreamProvider,
    };
    use crate::native::service_store::ServiceStateStore;
    use std::collections::BTreeMap;

    fn command_test_route_binding() -> RemoteViewRouteBinding {
        RemoteViewRouteBinding {
            route_id: "route-a".to_string(),
            route_pool_entry_id: Some("pool-a".to_string()),
            display_allocation_id: "display-a".to_string(),
            route_pool_entry_state: Some("available".to_string()),
            current_route_allocation_id: None,
            display_name: Some(":31".to_string()),
            launch_display_name: Some(":31".to_string()),
            display_isolation: "private_virtual_display".to_string(),
            route_user: None,
            display_access: None,
            provider: ViewStreamProvider::RdpGateway,
            provider_mode: "single_controller".to_string(),
            connection_id: Some("conn-a".to_string()),
            connection_name: Some("Route A".to_string()),
            frame_url: Some("https://dashboard.example/guacamole/#/client/route-a".to_string()),
            external_url: Some("https://guac.example/#/client/route-a".to_string()),
            route_descriptor: Some(json!({ "kind": "guacamole", "id": "route-a" })),
            readiness: None,
        }
    }

    fn command_test_acquisition_plan(
        route_binding: RemoteViewRouteBinding,
    ) -> RemoteViewAcquisitionPlan {
        RemoteViewAcquisitionPlan {
            mode: "strict_operator_open".to_string(),
            reuse_policy: "test".to_string(),
            tab_policy: "open_new".to_string(),
            requested_profile: Some("shared-social".to_string()),
            requested_browser_build: Some("stock_chrome".to_string()),
            requested_browser_host: "remote_headed".to_string(),
            requested_view_stream_provider: ViewStreamProvider::RdpGateway,
            requested_control_input: "manual_attached_desktop".to_string(),
            requested_display_isolation: Some(route_binding.display_isolation.clone()),
            requested_route_pool_entry_id: route_binding.route_pool_entry_id.clone(),
            requested_route_id: Some(route_binding.route_id.clone()),
            selected_route_pool_entry_id: route_binding.route_pool_entry_id.clone(),
            selected_route_id: route_binding.route_id.clone(),
            display_allocation_id: route_binding.display_allocation_id.clone(),
            display_name: route_binding.launch_display_name.clone(),
            route_binding,
            decisions: Vec::new(),
            blockers: Vec::new(),
            proof_required: Vec::new(),
            cleanup_on_failure: Vec::new(),
            suggested_commands: Vec::new(),
        }
    }

    fn command_test_intent() -> RemoteViewOpenIntent {
        RemoteViewOpenIntent {
            url: Some("https://x.com/home".to_string()),
            runtime_profile: Some("shared-social".to_string()),
            profile: None,
            browser_id: None,
            session_name: None,
            service_name: None,
            agent_name: None,
            task_name: None,
            browser_build: Some("stealth_chrome".to_string()),
            browser_host: "remote_headed".to_string(),
            view_stream_provider: ViewStreamProvider::RdpGateway,
            control_input: "manual_attached_desktop".to_string(),
            route_pool_entry_id: Some("pool-a".to_string()),
            route_id: Some("route-a".to_string()),
            display_allocation_id: Some("display-a".to_string()),
            remote_headed_display: Some(":31".to_string()),
            display_isolation: Some("private_virtual_display".to_string()),
            dry_run: false,
        }
    }

    fn rollback_test_repository() -> (
        std::path::PathBuf,
        JsonServiceStateStore,
        LockedServiceStateRepository<JsonServiceStateStore>,
        RemoteViewAcquisitionLease,
    ) {
        let path = std::env::temp_dir().join(format!(
            "agent-browser-handoff-rollback-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = JsonServiceStateStore::new(path.clone());
        store
            .save(&ServiceState {
                route_pool: BTreeMap::from([(
                    "pool-a".to_string(),
                    RoutePoolEntry {
                        id: "pool-a".to_string(),
                        route_id: "route-a".to_string(),
                        state: "pending".to_string(),
                        current_route_allocation_id: Some("route-a".to_string()),
                        ..RoutePoolEntry::default()
                    },
                )]),
                display_allocations: BTreeMap::from([(
                    "display-a".to_string(),
                    DisplayAllocation {
                        id: "display-a".to_string(),
                        state: "pending".to_string(),
                        owner_browser_id: Some("session:a".to_string()),
                        ..DisplayAllocation::default()
                    },
                )]),
                remote_view_routes: BTreeMap::from([(
                    "route-a".to_string(),
                    RemoteViewRoute {
                        id: "route-a".to_string(),
                        state: "pending".to_string(),
                        browser_id: Some("session:a".to_string()),
                        ..RemoteViewRoute::default()
                    },
                )]),
                browsers: BTreeMap::from([(
                    "session:a".to_string(),
                    BrowserProcess {
                        id: "session:a".to_string(),
                        display_allocation_id: Some("display-a".to_string()),
                        ..BrowserProcess::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();
        let repository = LockedServiceStateRepository::new(store.clone());
        let previous_entry = RoutePoolEntry {
            id: "pool-a".to_string(),
            route_id: "route-a".to_string(),
            state: "available".to_string(),
            current_route_allocation_id: None,
            ..RoutePoolEntry::default()
        };
        let previous_display = DisplayAllocation {
            id: "display-a".to_string(),
            state: "ready".to_string(),
            owner_browser_id: Some("session:previous".to_string()),
            ..DisplayAllocation::default()
        };
        let previous_route = RemoteViewRoute {
            id: "route-a".to_string(),
            state: "ready".to_string(),
            browser_id: Some("session:previous".to_string()),
            ..RemoteViewRoute::default()
        };
        let lease = RemoteViewAcquisitionLease {
            id: "lease-a".to_string(),
            browser_id: "session:a".to_string(),
            session_id: "a".to_string(),
            route_id: "route-a".to_string(),
            display_allocation_id: "display-a".to_string(),
            route_pool_entry_id: Some("pool-a".to_string()),
            state: "reserved".to_string(),
            phase: "route_reserved".to_string(),
            previous_route_pool_entry: Some(previous_entry),
            previous_display_allocation: Some(previous_display),
            previous_remote_view_route: Some(previous_route),
            previous_browser_display_allocation_id: Some("display-previous".to_string()),
            ..RemoteViewAcquisitionLease::default()
        };

        (path, store, repository, lease)
    }

    #[test]
    fn handoff_plan_normalizes_route_binding_and_groups_commands() {
        let mut route_binding = command_test_route_binding();
        route_binding.readiness = Some(json!({
            "state": "pending",
            "component": "remote_view_open_acquisition",
            "leaseId": "remote-view-open:session-a:route-a:stale"
        }));
        let acquisition_plan = command_test_acquisition_plan(route_binding);

        let plan = route_bound_handoff_plan(
            &json!({
                "action": "remote_view_open",
                "url": "https://example.com/",
                "dryRun": true
            }),
            &acquisition_plan,
            "browser-a",
            "session-a",
        );

        assert_eq!(
            plan.route_binding
                .readiness
                .as_ref()
                .and_then(|readiness| readiness.get("state"))
                .and_then(Value::as_str),
            Some("ready")
        );
        assert_eq!(plan.launch_command["action"], "launch");
        assert_eq!(plan.tab_command["action"], "tab_new");
        assert_eq!(plan.tab_command["browserId"], "browser-a");
        assert_eq!(
            plan.checkout_command["action"],
            "service_remote_view_route_checkout"
        );
        assert_eq!(plan.checkout_command["routeId"], "route-a");
        assert!(plan.launch_command.get("dryRun").is_none());
    }

    #[test]
    fn operator_visible_record_reports_route_display_tab_and_guacamole_readiness() {
        let mut route_binding = command_test_route_binding();
        route_binding.readiness = Some(json!({
            "state": "ready",
            "reason": "route ready"
        }));
        let visible_window_proof = json!({
            "state": "ready",
            "displayName": ":31",
            "displayContent": {
                "state": "browser_window_visible"
            }
        });
        let tab = json!({
            "targetId": "target-a",
            "url": "https://x.com/home",
            "title": "X",
            "profileId": "shared-social",
            "targetReadiness": "ready",
            "tabAcquisitionDecision": "opened_new_tab"
        });

        let record = route_bound_handoff_operator_visible(
            &route_binding,
            "browser-a",
            "session-a",
            Some(&visible_window_proof),
            Some(&tab),
            Some("https://x.com/home"),
        );

        assert_eq!(record["state"], "ready");
        assert_eq!(record["browserId"], "browser-a");
        assert_eq!(record["target"]["urlReadiness"], "ready");
        assert_eq!(record["components"]["route"]["state"], "ready");
        assert_eq!(record["components"]["display"]["state"], "ready");
        assert_eq!(
            record["components"]["display"]["contentState"],
            "browser_window_visible"
        );
        assert_eq!(record["components"]["tab"]["state"], "ready");
        assert_eq!(record["components"]["tab"]["profileId"], Value::Null);
        assert_eq!(record["components"]["guacamole"]["state"], "ready");
        assert_eq!(record["components"]["guacamole"]["hasRouteUrl"], true);
    }

    #[test]
    fn command_builders_apply_route_bound_launch_and_checkout_fields() {
        let route_binding = command_test_route_binding();
        let cmd = json!({
            "action": "remote_view_open",
            "provider": "should-not-leak-to-launch",
            "dryRun": true,
            "url": "https://example.com/"
        });

        let launch = route_bound_handoff_launch_command(&cmd, &route_binding);
        assert_eq!(launch["action"], "launch");
        assert_eq!(launch["headless"], false);
        assert_eq!(launch["browserHost"], "remote_headed");
        assert_eq!(launch["displayIsolation"], "private_virtual_display");
        assert_eq!(launch["viewStreamProvider"], "rdp_gateway");
        assert_eq!(launch["controlInput"], "manual_attached_desktop");
        assert_eq!(launch["remoteHeadedDisplay"], ":31");
        assert_eq!(launch["routeId"], "route-a");
        assert_eq!(launch["displayAllocationId"], "display-a");
        assert_eq!(launch["routePoolEntryId"], "pool-a");
        assert_eq!(launch["connectionId"], "conn-a");
        assert!(launch.get("provider").is_none());
        assert!(launch.get("dryRun").is_none());

        let checkout =
            route_bound_handoff_checkout_command(&cmd, &route_binding, "browser-a", "session-a");
        assert_eq!(checkout["action"], "service_remote_view_route_checkout");
        assert_eq!(checkout["browserId"], "browser-a");
        assert_eq!(checkout["sessionName"], "session-a");
        assert_eq!(checkout["provider"], "rdp_gateway");
        assert_eq!(checkout["providerMode"], "single_controller");
        assert_eq!(checkout["streamId"], "remote-headed-view");
        assert_eq!(checkout["routeId"], "route-a");
        assert_eq!(checkout["displayAllocationId"], "display-a");
        assert_eq!(checkout["frameUrl"], route_binding.frame_url.unwrap());
        assert_eq!(checkout["routeDescriptor"]["kind"], "guacamole");
        assert!(checkout.get("dryRun").is_none());
    }

    #[test]
    fn reused_browser_launch_result_records_route_owner_evidence() {
        let route_binding = command_test_route_binding();

        let launch = route_bound_handoff_reused_browser_launch_result(
            &route_binding,
            "browser-a",
            "session-a",
        );

        assert_eq!(launch["status"], "reused");
        assert_eq!(launch["reused"], true);
        assert_eq!(launch["browserId"], "browser-a");
        assert_eq!(launch["sessionName"], "session-a");
        assert_eq!(launch["routeId"], "route-a");
        assert_eq!(launch["displayAllocationId"], "display-a");
        assert_eq!(launch["reason"], "same_owner_checked_out_route");
    }

    #[test]
    fn checkout_command_with_visible_window_proof_attaches_readiness_and_display_content() {
        let checkout = json!({
            "action": "service_remote_view_route_checkout",
            "routeId": "route-a"
        });
        let proof = json!({
            "state": "ready",
            "displayContent": {
                "state": "browser_window_visible",
                "displayName": ":31"
            }
        });

        let command =
            route_bound_handoff_checkout_command_with_visible_window_proof(&checkout, &proof);

        assert_eq!(command["action"], "service_remote_view_route_checkout");
        assert_eq!(command["readiness"]["state"], "ready");
        assert_eq!(
            command["readiness"]["component"],
            "remote_view_open_visible_window"
        );
        assert_eq!(
            command["readiness"]["displayContent"]["state"],
            "browser_window_visible"
        );
        assert_eq!(command["displayContent"]["displayName"], ":31");
    }

    #[test]
    fn checkout_command_with_visible_window_proof_records_null_display_content_when_missing() {
        let checkout = json!({
            "action": "service_remote_view_route_checkout",
            "routeId": "route-a"
        });
        let proof = json!({
            "state": "ready"
        });

        let command =
            route_bound_handoff_checkout_command_with_visible_window_proof(&checkout, &proof);

        assert_eq!(command["readiness"]["displayContent"], Value::Null);
        assert!(command.get("displayContent").is_none());
    }

    #[test]
    fn tab_command_defaults_blank_url_for_missing_target_url() {
        let command = route_bound_handoff_tab_command(
            &json!({
                "action": "remote_view_open",
                "dryRun": true
            }),
            "browser-a",
            "session-a",
        );

        assert_eq!(command["action"], "tab_new");
        assert_eq!(command["browserId"], "browser-a");
        assert_eq!(command["sessionName"], "session-a");
        assert_eq!(command["url"], "about:blank");
        assert!(command.get("dryRun").is_none());
    }

    #[test]
    fn focus_command_prefers_target_id_over_stale_index() {
        let command = route_bound_handoff_focus_command(
            &json!({
                "action": "remote_view_open",
                "url": "https://example.com/"
            }),
            &json!({
                "targetId": "selected-target",
                "index": 1
            }),
            "remote-session",
        );

        assert_eq!(command["action"], "view_focus");
        assert_eq!(command["sessionName"], "remote-session");
        assert_eq!(command["targetId"], "selected-target");
        assert!(command.get("index").is_none());
    }

    #[test]
    fn focus_command_uses_index_without_target_id() {
        let command = route_bound_handoff_focus_command(
            &json!({
                "action": "remote_view_open",
                "url": "https://example.com/"
            }),
            &json!({
                "index": 0
            }),
            "remote-session",
        );

        assert_eq!(command["action"], "view_focus");
        assert_eq!(command["sessionName"], "remote-session");
        assert_eq!(command["index"], 0);
        assert!(command.get("targetId").is_none());
    }

    #[test]
    fn handoff_record_prefers_tab_profile_as_authoritative_profile() {
        let intent = RemoteViewOpenIntent {
            url: Some("https://www.facebook.com/".to_string()),
            runtime_profile: Some("shared-social".to_string()),
            profile: None,
            browser_id: None,
            session_name: None,
            service_name: None,
            agent_name: None,
            task_name: None,
            browser_build: Some("stealth_chrome".to_string()),
            browser_host: "remote_headed".to_string(),
            view_stream_provider: ViewStreamProvider::RdpGateway,
            control_input: "cdp".to_string(),
            route_pool_entry_id: Some("pool-a".to_string()),
            route_id: Some("route-a".to_string()),
            display_allocation_id: Some("display-a".to_string()),
            remote_headed_display: Some(":31".to_string()),
            display_isolation: Some("shared_display".to_string()),
            dry_run: false,
        };
        let route_binding = RemoteViewRouteBinding {
            route_id: "route-a".to_string(),
            route_pool_entry_id: Some("pool-a".to_string()),
            display_allocation_id: "display-a".to_string(),
            route_pool_entry_state: Some("checked_out".to_string()),
            current_route_allocation_id: Some("route-a".to_string()),
            display_name: Some(":31".to_string()),
            launch_display_name: Some(":31".to_string()),
            display_isolation: "shared_display".to_string(),
            route_user: None,
            display_access: None,
            provider: ViewStreamProvider::RdpGateway,
            provider_mode: "single_controller".to_string(),
            connection_id: Some("conn-a".to_string()),
            connection_name: Some("Route A".to_string()),
            frame_url: Some("https://dashboard.example/guacamole/#/client/route-a".to_string()),
            external_url: Some("https://guac.example/#/client/route-a".to_string()),
            route_descriptor: None,
            readiness: None,
        };
        let operator_visible = json!({
            "state": "ready",
            "routeId": "route-a",
            "displayAllocationId": "display-a",
        });
        let tab = json!({
            "targetId": "target-1",
            "profileId": "shared-social",
            "url": "https://www.facebook.com/",
        });

        let record = route_bound_handoff_record(RouteBoundHandoffRecordInput {
            state: "opened",
            intent: &intent,
            route_binding: &route_binding,
            browser_id: "session:rdp-a",
            session_name: "rdp-a",
            operator_visible: &operator_visible,
            tab: Some(&tab),
            browser_build_proof: None,
        });

        assert_eq!(record["profile"]["id"], "shared-social");
        assert_eq!(record["profile"]["source"], "tab");
        assert_eq!(record["browser"]["browserId"], "session:rdp-a");
        assert_eq!(record["browser"]["sessionName"], "rdp-a");
        assert_eq!(record["route"]["routeId"], "route-a");
        assert_eq!(record["display"]["displayAllocationId"], "display-a");
        assert_eq!(record["tab"]["targetId"], "target-1");
        assert_eq!(record["operatorVisible"]["state"], "ready");
    }

    #[test]
    fn cleanup_summary_includes_cleanup_and_lease_rollback() {
        let cleanup = json!({
            "state": "closed_opened_tab",
            "index": 2,
        });
        let rollback = json!({
            "state": "rolled_back",
            "leaseId": "lease-a",
        });

        let summary: Value = serde_json::from_str(&route_bound_handoff_cleanup_summary(
            &cleanup,
            Some(&rollback),
        ))
        .unwrap();

        assert_eq!(summary["state"], "closed_opened_tab");
        assert_eq!(summary["cleanup"]["index"], 2);
        assert_eq!(summary["leaseRollback"]["leaseId"], "lease-a");
    }

    #[test]
    fn failure_recovery_begins_with_rollback_and_cleanup_plan() {
        let (path, store, repository, lease) = rollback_test_repository();
        let rollback_cleanup = route_bound_handoff_pending_rollback_cleanup("proof_failed");
        let launch = json!({
            "reused": true,
            "browserId": "session:a"
        });

        let recovery = begin_route_bound_handoff_failure_recovery(
            &repository,
            RouteBoundHandoffFailureRecoveryInput {
                lease: &lease,
                phase: "proof_failed",
                error: "not visible",
                rollback_cleanup: &rollback_cleanup,
                launch: &launch,
                tab: None,
                observed_at: "2026-07-06T12:00:00Z",
            },
        )
        .unwrap();

        assert_eq!(recovery.rollback["state"], "rolled_back");
        assert_eq!(
            recovery.cleanup_plan,
            RouteBoundHandoffFailureCleanupPlan::SkipExistingBrowserReused {
                reason: "opened tab index unavailable"
            }
        );
        assert_eq!(
            recovery.skipped_cleanup.as_ref().unwrap()["state"],
            "skipped_existing_browser_reused"
        );
        match recovery.cleanup_task {
            RouteBoundHandoffFailureCleanupTask::Skipped { cleanup } => {
                assert_eq!(cleanup["state"], "skipped_existing_browser_reused");
                assert_eq!(cleanup["reason"], "opened tab index unavailable");
            }
            other => panic!("unexpected cleanup task: {other:?}"),
        }
        let state = store.load().unwrap();
        assert_eq!(state.route_pool["pool-a"].state, "available");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn immediate_failure_rolls_back_and_returns_summary() {
        let (path, store, repository, lease) = rollback_test_repository();
        let cleanup = route_bound_handoff_launch_failure_cleanup("browser_launch_failed");

        let failure = route_bound_handoff_immediate_failure(
            &repository,
            RouteBoundHandoffImmediateFailureInput {
                lease: &lease,
                phase: "browser_launch_failed",
                error: "profile lock",
                cleanup: &cleanup,
                observed_at: "2026-07-06T12:00:00Z",
            },
        )
        .unwrap();

        assert_eq!(failure.rollback["state"], "rolled_back");
        let summary: Value = serde_json::from_str(&failure.summary).unwrap();
        assert_eq!(summary["cleanup"]["state"], "skipped_after_launch_failure");
        assert_eq!(summary["leaseRollback"]["phase"], "browser_launch_failed");
        let state = store.load().unwrap();
        assert_eq!(state.display_allocations["display-a"].state, "ready");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn complete_open_finalizes_lease_and_returns_opened_response() {
        let (path, store, repository, mut lease) = rollback_test_repository();
        lease.state = "pending".to_string();
        lease.phase = "reserved".to_string();
        let planned_route_binding = command_test_route_binding();
        let acquisition_plan = command_test_acquisition_plan(planned_route_binding.clone());
        let intent = command_test_intent();
        let operator_visible = json!({
            "state": "ready"
        });
        let checkout = json!({
            "routeId": "route-final",
            "remoteViewRouteId": "route-final",
            "displayAllocationId": "display-final",
            "routePoolEntryId": "pool-a",
            "browserId": "session:a",
            "sessionName": "a",
            "remoteViewRoute": {
                "id": "route-final",
                "displayAllocationId": "display-final",
                "readiness": {
                    "state": "ready",
                    "component": "route_bound_finalization"
                }
            }
        });
        let launch_command = json!({
            "browserCapabilityLaunch": {
                "applied": true,
                "browserBuild": "stealth_chrome",
                "executablePath": "/opt/chromium-stealthcdp/chrome"
            }
        });
        let tab = json!({
            "index": 2,
            "targetId": "target-2",
            "url": "https://x.com/home",
            "profileId": "shared-social"
        });

        let response = complete_route_bound_handoff_open(CompleteRouteBoundHandoffOpenInput {
            intent: &intent,
            planned_route_binding: &planned_route_binding,
            acquisition_plan: &acquisition_plan,
            repository: &repository,
            lease: &lease,
            observed_at: "2026-07-06T12:00:00Z",
            browser_id: "session:a",
            session_name: "a",
            managed_one_time_profile: &json!({ "state": "not_used" }),
            one_time_profile_warning: &json!({ "state": "none" }),
            final_operator_visible: &operator_visible,
            pre_checkout_operator_visible: &operator_visible,
            launch_command: &launch_command,
            launch: &json!({ "status": "reused" }),
            tab: &tab,
            focus: &json!({ "focused": true }),
            checkout: &checkout,
            display_access_grant: &json!({ "state": "already_ready" }),
            reused_current_browser: true,
            visible_window_proof: &json!({ "state": "ready" }),
        })
        .unwrap();

        assert_eq!(response["status"], "opened");
        assert_eq!(response["routeBoundHandoff"]["state"], "opened");
        assert_eq!(
            response["routeBoundHandoff"]["route"]["routeId"],
            "route-final"
        );
        assert_eq!(response["routeBinding"]["routeId"], "route-final");
        assert_eq!(response["browserBuildProof"]["state"], "matched");
        assert_eq!(response["acquisitionLease"]["state"], "completed");
        assert_eq!(response["acquisitionLease"]["phase"], "checked_out");
        let state = store.load().unwrap();
        assert_eq!(
            state.remote_view_acquisition_leases["lease-a"].state,
            "completed"
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rollback_failure_restores_lease_and_summarizes_cleanup() {
        let path = std::env::temp_dir().join(format!(
            "agent-browser-handoff-rollback-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = JsonServiceStateStore::new(path.clone());
        store
            .save(&ServiceState {
                route_pool: BTreeMap::from([(
                    "pool-a".to_string(),
                    RoutePoolEntry {
                        id: "pool-a".to_string(),
                        route_id: "route-a".to_string(),
                        state: "pending".to_string(),
                        current_route_allocation_id: Some("route-a".to_string()),
                        ..RoutePoolEntry::default()
                    },
                )]),
                display_allocations: BTreeMap::from([(
                    "display-a".to_string(),
                    DisplayAllocation {
                        id: "display-a".to_string(),
                        state: "pending".to_string(),
                        owner_browser_id: Some("session:a".to_string()),
                        ..DisplayAllocation::default()
                    },
                )]),
                remote_view_routes: BTreeMap::from([(
                    "route-a".to_string(),
                    RemoteViewRoute {
                        id: "route-a".to_string(),
                        state: "pending".to_string(),
                        browser_id: Some("session:a".to_string()),
                        ..RemoteViewRoute::default()
                    },
                )]),
                browsers: BTreeMap::from([(
                    "session:a".to_string(),
                    BrowserProcess {
                        id: "session:a".to_string(),
                        display_allocation_id: Some("display-a".to_string()),
                        ..BrowserProcess::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();
        let repository = LockedServiceStateRepository::new(store.clone());
        let previous_entry = RoutePoolEntry {
            id: "pool-a".to_string(),
            route_id: "route-a".to_string(),
            state: "available".to_string(),
            current_route_allocation_id: None,
            ..RoutePoolEntry::default()
        };
        let previous_display = DisplayAllocation {
            id: "display-a".to_string(),
            state: "ready".to_string(),
            owner_browser_id: Some("session:previous".to_string()),
            ..DisplayAllocation::default()
        };
        let previous_route = RemoteViewRoute {
            id: "route-a".to_string(),
            state: "ready".to_string(),
            browser_id: Some("session:previous".to_string()),
            ..RemoteViewRoute::default()
        };
        let lease = RemoteViewAcquisitionLease {
            id: "lease-a".to_string(),
            browser_id: "session:a".to_string(),
            session_id: "a".to_string(),
            route_id: "route-a".to_string(),
            display_allocation_id: "display-a".to_string(),
            route_pool_entry_id: Some("pool-a".to_string()),
            state: "reserved".to_string(),
            phase: "route_reserved".to_string(),
            previous_route_pool_entry: Some(previous_entry),
            previous_display_allocation: Some(previous_display),
            previous_remote_view_route: Some(previous_route),
            previous_browser_display_allocation_id: Some("display-previous".to_string()),
            ..RemoteViewAcquisitionLease::default()
        };
        let rollback_cleanup = json!({
            "state": "pending_after_rollback",
            "reason": "proof_failed"
        });

        let rollback = rollback_route_bound_handoff_failure(
            &repository,
            RouteBoundHandoffFailureRollbackInput {
                lease: &lease,
                phase: "proof_failed",
                error: "not visible",
                cleanup: &rollback_cleanup,
                observed_at: "2026-07-06T12:00:00Z",
            },
        )
        .unwrap();
        let browser_cleanup = json!({
            "state": "closed_opened_tab",
            "index": 2,
        });
        let failure = complete_route_bound_handoff_failure_cleanup(
            &repository,
            RouteBoundHandoffFailureCleanupInput {
                lease_id: "lease-a",
                rollback: &rollback,
                cleanup: &browser_cleanup,
                observed_at: "2026-07-06T12:00:01Z",
            },
        )
        .unwrap();

        let state = store.load().unwrap();
        assert_eq!(state.route_pool["pool-a"].state, "available");
        assert_eq!(state.route_pool["pool-a"].current_route_allocation_id, None);
        assert_eq!(state.display_allocations["display-a"].state, "ready");
        assert_eq!(state.remote_view_routes["route-a"].state, "ready");
        assert_eq!(
            state.browsers["session:a"].display_allocation_id.as_deref(),
            Some("display-previous")
        );
        let persisted_lease = &state.remote_view_acquisition_leases["lease-a"];
        assert_eq!(persisted_lease.state, "failed");
        assert_eq!(persisted_lease.phase, "rollback_complete");
        assert_eq!(
            persisted_lease.cleanup.as_ref().unwrap()["cleanup"]["state"],
            "closed_opened_tab"
        );
        assert_eq!(failure.rollback["cleanup"]["index"], 2);
        let summary: Value = serde_json::from_str(&failure.summary).unwrap();
        assert_eq!(summary["cleanup"]["state"], "closed_opened_tab");
        assert_eq!(summary["leaseRollback"]["leaseId"], "lease-a");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn final_route_binding_applies_checkout_route_and_pool_readback() {
        let binding = RemoteViewRouteBinding {
            route_id: "route-planned".to_string(),
            route_pool_entry_id: Some("pool-planned".to_string()),
            display_allocation_id: "display-planned".to_string(),
            route_pool_entry_state: Some("available".to_string()),
            current_route_allocation_id: None,
            display_name: Some(":31".to_string()),
            launch_display_name: Some(":31".to_string()),
            display_isolation: "shared_display".to_string(),
            route_user: Some("agent-browser-rdp-a".to_string()),
            display_access: None,
            provider: ViewStreamProvider::RdpGateway,
            provider_mode: "single_controller".to_string(),
            connection_id: Some("conn-planned".to_string()),
            connection_name: Some("Planned Route".to_string()),
            frame_url: Some("https://dashboard.example/guac/#/client/planned".to_string()),
            external_url: Some("https://guac.example/#/client/planned".to_string()),
            route_descriptor: None,
            readiness: Some(json!({
                "state": "ready",
                "reason": "pre_checkout_ready"
            })),
        };
        let checkout = json!({
            "remoteViewRoute": {
                "id": "route-final",
                "displayAllocationId": "display-final",
                "connectionId": "conn-final",
                "connectionName": "Final Route",
                "frameUrl": "https://dashboard.example/guac/#/client/final",
                "externalUrl": "https://guac.example/#/client/final",
                "providerMode": "multi_viewer",
                "readiness": {
                    "state": "ready",
                    "component": "route_bound_finalization"
                }
            },
            "routePoolEntry": {
                "id": "pool-final",
                "state": "checked_out",
                "currentRouteAllocationId": "route-final",
                "readiness": {
                    "state": "ready",
                    "component": "route_bound_finalization"
                }
            }
        });

        let final_binding = final_route_bound_handoff_route_binding(&binding, &checkout);

        assert_eq!(final_binding.route_id, "route-final");
        assert_eq!(final_binding.display_allocation_id, "display-final");
        assert_eq!(
            final_binding.route_pool_entry_id.as_deref(),
            Some("pool-final")
        );
        assert_eq!(
            final_binding.route_pool_entry_state.as_deref(),
            Some("checked_out")
        );
        assert_eq!(
            final_binding.current_route_allocation_id.as_deref(),
            Some("route-final")
        );
        assert_eq!(final_binding.connection_id.as_deref(), Some("conn-final"));
        assert_eq!(
            final_binding.connection_name.as_deref(),
            Some("Final Route")
        );
        assert_eq!(final_binding.provider_mode, "multi_viewer");
        assert_eq!(
            final_binding.readiness.as_ref().unwrap()["component"],
            "route_bound_finalization"
        );
    }

    #[test]
    fn post_checkout_proof_derives_final_binding_and_passes_ready_proof() {
        let binding = command_test_route_binding();
        let checkout = json!({
            "remoteViewRoute": {
                "id": "route-final",
                "displayAllocationId": "display-final"
            }
        });
        let tab = json!({
            "index": 2,
            "url": "https://x.com/home"
        });

        let proof = route_bound_handoff_post_checkout_proof(
            RouteBoundHandoffPostCheckoutProofInput {
                planned_route_binding: &binding,
                checkout: &checkout,
                browser_id: "session:route-a",
                session_name: "route-a",
                pre_checkout_operator_visible: &json!({ "state": "ready" }),
                tab: Some(&tab),
                expected_url: Some("https://x.com/home"),
            },
            |final_binding| {
                json!({
                    "state": "ready",
                    "routeId": final_binding.route_id,
                    "displayAllocationId": final_binding.display_allocation_id
                })
            },
        );

        assert_eq!(proof.final_route_binding.route_id, "route-final");
        assert_eq!(proof.final_operator_visible["state"], "ready");
        assert!(proof.failure.is_none());
    }

    #[test]
    fn post_checkout_proof_returns_final_failure_when_not_ready() {
        let binding = command_test_route_binding();
        let checkout = json!({
            "remoteViewRoute": {
                "id": "route-final",
                "displayAllocationId": "display-final"
            }
        });

        let proof = route_bound_handoff_post_checkout_proof(
            RouteBoundHandoffPostCheckoutProofInput {
                planned_route_binding: &binding,
                checkout: &checkout,
                browser_id: "session:route-a",
                session_name: "route-a",
                pre_checkout_operator_visible: &json!({ "state": "ready" }),
                tab: None,
                expected_url: Some("https://x.com/home"),
            },
            |_| json!({ "state": "stale_route_record" }),
        );

        let failure = proof.failure.unwrap();
        assert!(failure.error.contains("route 'route-final'"));
        assert_eq!(
            failure.cleanup["reason"],
            "final_operator_visible_proof_failed"
        );
    }

    #[test]
    fn shared_acquisition_records_route_bound_handoff_result() {
        let intent = RemoteViewOpenIntent {
            url: Some("https://x.com/home".to_string()),
            runtime_profile: Some("last30days-facebook".to_string()),
            profile: None,
            browser_id: None,
            session_name: None,
            service_name: None,
            agent_name: None,
            task_name: None,
            browser_build: Some("stealthcdp_chromium".to_string()),
            browser_host: "remote_headed".to_string(),
            view_stream_provider: ViewStreamProvider::RdpGateway,
            control_input: "manual_attached_desktop".to_string(),
            route_pool_entry_id: Some("pool-a".to_string()),
            route_id: Some("route-a".to_string()),
            display_allocation_id: Some("display-a".to_string()),
            remote_headed_display: Some(":31".to_string()),
            display_isolation: Some("shared_display".to_string()),
            dry_run: false,
        };
        let route_binding = RemoteViewRouteBinding {
            route_id: "route-a".to_string(),
            route_pool_entry_id: Some("pool-a".to_string()),
            display_allocation_id: "display-a".to_string(),
            route_pool_entry_state: Some("checked_out".to_string()),
            current_route_allocation_id: Some("route-a".to_string()),
            display_name: Some(":31".to_string()),
            launch_display_name: Some(":31".to_string()),
            display_isolation: "shared_display".to_string(),
            route_user: None,
            display_access: None,
            provider: ViewStreamProvider::RdpGateway,
            provider_mode: "single_controller".to_string(),
            connection_id: Some("conn-a".to_string()),
            connection_name: Some("Route A".to_string()),
            frame_url: Some("https://dashboard.example/guacamole/#/client/route-a".to_string()),
            external_url: Some("https://guac.example/#/client/route-a".to_string()),
            route_descriptor: None,
            readiness: None,
        };
        let tab = json!({
            "targetId": "target-x",
            "profileId": "last30days-facebook",
            "tabAcquisitionDecision": "opened_new_target",
        });

        let acquisition =
            route_bound_handoff_shared_acquisition(RouteBoundHandoffSharedAcquisitionInput {
                state: "opened",
                intent: &intent,
                route_binding: &route_binding,
                browser_id: "session:social",
                session_name: "social",
                tab: Some(&tab),
                reused_current_browser: true,
            });

        assert_eq!(acquisition["policy"], "shared_browser_tabs");
        assert_eq!(acquisition["mode"], "remote_view_open");
        assert_eq!(acquisition["action"], "opened_route_bound_handoff");
        assert_eq!(acquisition["recommendedAction"], "open_shared_profile_tab");
        assert_eq!(acquisition["browserReused"], true);
        assert_eq!(acquisition["tabOpened"], true);
        assert_eq!(
            acquisition["duplicateProcessPolicy"],
            "reject_duplicate_process"
        );
        assert_eq!(acquisition["browserId"], "session:social");
        assert_eq!(acquisition["sessionName"], "social");
        assert_eq!(acquisition["profileId"], "last30days-facebook");
        assert_eq!(acquisition["requestedProfile"], "last30days-facebook");
        assert_eq!(acquisition["plannedProfile"], "last30days-facebook");
        assert_eq!(acquisition["routeBound"], true);
        assert_eq!(acquisition["routeId"], "route-a");
        assert_eq!(acquisition["displayAllocationId"], "display-a");
        assert_eq!(acquisition["routeHintFields"][2], "routeId");
        assert_eq!(acquisition["tabAcquisitionDecision"], "opened_new_target");
    }

    #[test]
    fn failure_cleanup_plan_closes_only_opened_tab_for_reused_browser() {
        let launch = json!({
            "status": "reused",
            "reused": true,
        });
        let tab = json!({
            "index": 3,
            "targetId": "target-a",
        });

        let plan = route_bound_handoff_failure_cleanup_plan(&launch, Some(&tab));

        assert_eq!(
            plan,
            RouteBoundHandoffFailureCleanupPlan::CloseOpenedTab { index: 3 }
        );
        let cleanup =
            route_bound_handoff_failure_cleanup_result(&plan, Ok(json!({ "closed": true })));
        assert_eq!(cleanup["state"], "closed_opened_tab");
        assert_eq!(cleanup["index"], 3);
        assert_eq!(cleanup["result"]["closed"], true);
    }

    #[test]
    fn failure_cleanup_task_builds_opened_tab_close_command() {
        let plan = RouteBoundHandoffFailureCleanupPlan::CloseOpenedTab { index: 3 };
        let task = route_bound_handoff_failure_cleanup_task(&plan);

        match task {
            RouteBoundHandoffFailureCleanupTask::CloseOpenedTab { index, command } => {
                assert_eq!(index, 3);
                assert_eq!(command["index"], 3);
            }
            other => panic!("unexpected cleanup task: {other:?}"),
        }
    }

    #[test]
    fn failure_cleanup_task_builds_new_browser_close_command() {
        let plan = RouteBoundHandoffFailureCleanupPlan::CloseNewBrowser;
        let task = route_bound_handoff_failure_cleanup_task(&plan);

        match task {
            RouteBoundHandoffFailureCleanupTask::CloseNewBrowser { command } => {
                assert_eq!(command["action"], "close");
            }
            other => panic!("unexpected cleanup task: {other:?}"),
        }
    }

    #[test]
    fn failure_cleanup_task_preserves_skipped_cleanup_payload() {
        let plan = RouteBoundHandoffFailureCleanupPlan::SkipExistingBrowserReused {
            reason: "opened tab index unavailable",
        };
        let task = route_bound_handoff_failure_cleanup_task(&plan);

        match task {
            RouteBoundHandoffFailureCleanupTask::Skipped { cleanup } => {
                assert_eq!(cleanup["state"], "skipped_existing_browser_reused");
                assert_eq!(cleanup["reason"], "opened tab index unavailable");
            }
            other => panic!("unexpected cleanup task: {other:?}"),
        }
    }

    #[test]
    fn failure_cleanup_plan_skips_reused_browser_without_tab_index() {
        let launch = json!({
            "status": "reused",
            "reused": true,
        });
        let tab = json!({
            "targetId": "target-a",
        });

        let plan = route_bound_handoff_failure_cleanup_plan(&launch, Some(&tab));
        let cleanup = route_bound_handoff_skipped_failure_cleanup(&plan).unwrap();

        assert_eq!(
            plan,
            RouteBoundHandoffFailureCleanupPlan::SkipExistingBrowserReused {
                reason: "opened tab index unavailable"
            }
        );
        assert_eq!(cleanup["state"], "skipped_existing_browser_reused");
        assert_eq!(cleanup["reason"], "opened tab index unavailable");
    }

    #[test]
    fn failure_cleanup_plan_closes_new_browser_for_launch_result() {
        let launch = json!({
            "status": "launched",
            "browserId": "session:route-a",
        });

        let plan = route_bound_handoff_failure_cleanup_plan(&launch, None);

        assert_eq!(plan, RouteBoundHandoffFailureCleanupPlan::CloseNewBrowser);
        let cleanup =
            route_bound_handoff_failure_cleanup_result(&plan, Err("close failed".to_string()));
        assert_eq!(cleanup["state"], "failed_new_browser_close");
        assert_eq!(cleanup["error"], "close failed");
    }

    #[test]
    fn pending_rollback_cleanup_records_simple_reason() {
        let cleanup = route_bound_handoff_pending_rollback_cleanup("tab_open_failed");

        assert_eq!(cleanup["state"], "pending_after_rollback");
        assert_eq!(cleanup["reason"], "tab_open_failed");
    }

    #[test]
    fn checkout_failure_records_phase_and_cleanup_payload() {
        let failure = route_bound_handoff_checkout_failure();

        assert_eq!(failure.phase, "checkout_failed");
        assert_eq!(failure.cleanup["state"], "pending_after_rollback");
        assert_eq!(failure.cleanup["reason"], "checkout_failed");
    }

    #[test]
    fn simple_rollback_failures_record_phase_and_cleanup_payload() {
        for (failure, phase) in [
            (route_bound_handoff_tab_open_failure(), "tab_open_failed"),
            (route_bound_handoff_focus_failure(), "focus_failed"),
            (
                route_bound_handoff_visible_window_proof_failure(),
                "proof_failed",
            ),
        ] {
            assert_eq!(failure.phase, phase);
            assert_eq!(failure.cleanup["state"], "pending_after_rollback");
            assert_eq!(failure.cleanup["reason"], phase);
        }
    }

    #[test]
    fn pre_launch_failure_cleanup_records_display_access_reason() {
        let cleanup = route_bound_handoff_pre_launch_failure_cleanup("display_access_failed");

        assert_eq!(cleanup["state"], "skipped_before_browser_launch");
        assert_eq!(cleanup["reason"], "display_access_failed");
    }

    #[test]
    fn launch_failure_cleanup_records_browser_launch_reason() {
        let cleanup = route_bound_handoff_launch_failure_cleanup("browser_launch_failed");

        assert_eq!(cleanup["state"], "skipped_after_launch_failure");
        assert_eq!(cleanup["reason"], "browser_launch_failed");
    }

    #[test]
    fn operator_visible_failure_cleanup_preserves_proof_surfaces() {
        let operator_visible = json!({
            "state": "wrong_tab",
            "components": {
                "tab": {
                    "state": "wrong_url"
                }
            }
        });
        let handoff = json!({
            "state": "operator_visible_proof_failed",
            "routeId": "route-a"
        });

        let cleanup =
            route_bound_handoff_operator_visible_failure_cleanup(&operator_visible, &handoff);

        assert_eq!(cleanup["state"], "pending_after_rollback");
        assert_eq!(cleanup["reason"], "operator_visible_proof_failed");
        assert_eq!(cleanup["operatorVisible"]["state"], "wrong_tab");
        assert_eq!(cleanup["routeBoundHandoff"]["routeId"], "route-a");
    }

    #[test]
    fn operator_visible_failure_builds_error_and_cleanup_together() {
        let route_binding = command_test_route_binding();
        let operator_visible = json!({
            "state": "wrong_tab",
            "components": {
                "tab": {
                    "state": "wrong_url"
                }
            }
        });
        let tab = json!({
            "index": 2,
            "url": "https://x.com/home"
        });

        let failure = route_bound_handoff_operator_visible_failure(
            &route_binding,
            "session:route-a",
            "route-a",
            &operator_visible,
            Some(&tab),
            Some("https://x.com/home"),
        );

        assert!(failure.error.contains("wrong_tab: route 'route-a'"));
        assert!(failure.error.contains("routeBoundHandoff="));
        assert_eq!(failure.cleanup["reason"], "operator_visible_proof_failed");
        assert_eq!(failure.cleanup["operatorVisible"]["state"], "wrong_tab");
        assert_eq!(
            failure.cleanup["routeBoundHandoff"]["target"]["tab"]["index"],
            2
        );
    }

    #[test]
    fn operator_visible_failure_gate_returns_none_when_ready() {
        let route_binding = command_test_route_binding();

        let failure = route_bound_handoff_operator_visible_failure_if_not_ready(
            &route_binding,
            "session:route-a",
            "route-a",
            &json!({ "state": "ready" }),
            None,
            Some("https://x.com/home"),
        );

        assert!(failure.is_none());
    }

    #[test]
    fn operator_visible_failure_gate_builds_failure_when_not_ready() {
        let route_binding = command_test_route_binding();

        let failure = route_bound_handoff_operator_visible_failure_if_not_ready(
            &route_binding,
            "session:route-a",
            "route-a",
            &json!({ "state": "wrong_tab" }),
            None,
            Some("https://x.com/home"),
        )
        .unwrap();

        assert!(failure.error.contains("wrong_tab: route 'route-a'"));
        assert_eq!(failure.cleanup["reason"], "operator_visible_proof_failed");
    }

    #[test]
    fn final_operator_visible_failure_cleanup_preserves_pre_checkout_proof() {
        let final_operator_visible = json!({
            "state": "stale_route_record"
        });
        let pre_checkout_operator_visible = json!({
            "state": "ready"
        });
        let handoff = json!({
            "state": "final_operator_visible_proof_failed",
            "routeId": "route-a"
        });

        let cleanup = route_bound_handoff_final_operator_visible_failure_cleanup(
            &final_operator_visible,
            &pre_checkout_operator_visible,
            &handoff,
        );

        assert_eq!(cleanup["reason"], "final_operator_visible_proof_failed");
        assert_eq!(
            cleanup["finalOperatorVisible"]["state"],
            "stale_route_record"
        );
        assert_eq!(cleanup["preCheckoutOperatorVisible"]["state"], "ready");
        assert_eq!(
            cleanup["routeBoundHandoff"]["state"],
            "final_operator_visible_proof_failed"
        );
    }

    #[test]
    fn final_operator_visible_failure_builds_error_and_preserves_pre_checkout_label() {
        let route_binding = command_test_route_binding();
        let final_operator_visible = json!({
            "state": "stale_route_record"
        });
        let pre_checkout_operator_visible = json!({
            "state": "ready"
        });
        let tab = json!({
            "index": 2,
            "url": "https://x.com/home"
        });

        let failure = route_bound_handoff_final_operator_visible_failure(
            &route_binding,
            "session:route-a",
            "route-a",
            &final_operator_visible,
            &pre_checkout_operator_visible,
            Some(&tab),
            Some("https://x.com/home"),
        );

        assert!(failure
            .error
            .contains("stale_route_record: route 'route-a'"));
        assert!(failure.error.contains("preCheckoutOperatorVisible="));
        assert_eq!(
            failure.cleanup["reason"],
            "final_operator_visible_proof_failed"
        );
        assert_eq!(
            failure.cleanup["finalOperatorVisible"]["state"],
            "stale_route_record"
        );
        assert_eq!(
            failure.cleanup["preCheckoutOperatorVisible"]["state"],
            "ready"
        );
    }

    #[test]
    fn final_operator_visible_failure_gate_preserves_pre_checkout_context() {
        let route_binding = command_test_route_binding();

        let failure = route_bound_handoff_final_operator_visible_failure_if_not_ready(
            &route_binding,
            "session:route-a",
            "route-a",
            &json!({ "state": "stale_route_record" }),
            &json!({ "state": "ready" }),
            None,
            Some("https://x.com/home"),
        )
        .unwrap();

        assert!(failure.error.contains("preCheckoutOperatorVisible="));
        assert_eq!(
            failure.cleanup["reason"],
            "final_operator_visible_proof_failed"
        );
        assert_eq!(
            failure.cleanup["preCheckoutOperatorVisible"]["state"],
            "ready"
        );
    }

    #[test]
    fn browser_build_proof_flags_stock_chrome_stealth_mismatch() {
        let intent = RemoteViewOpenIntent {
            url: Some("https://direct.sos.state.tx.us/".to_string()),
            runtime_profile: None,
            profile: None,
            browser_id: None,
            session_name: None,
            service_name: Some("sosdirect".to_string()),
            agent_name: Some("codex".to_string()),
            task_name: Some("temporary-login".to_string()),
            browser_build: Some("stock_chrome".to_string()),
            browser_host: "remote_headed".to_string(),
            view_stream_provider: ViewStreamProvider::RdpGateway,
            control_input: "manual_attached_desktop".to_string(),
            route_pool_entry_id: None,
            route_id: None,
            display_allocation_id: None,
            remote_headed_display: None,
            display_isolation: Some("private_virtual_display".to_string()),
            dry_run: false,
        };
        let launch_command = json!({
            "browserBuild": "stock_chrome",
            "browserCapabilityLaunch": {
                "applied": true,
                "browserBuild": "stock_chrome",
                "executablePath": "/opt/chromium-stealthcdp/chrome"
            }
        });

        let proof = route_bound_handoff_browser_build_proof(
            &intent,
            &launch_command,
            &json!({ "launched": true }),
        );

        assert_eq!(proof["state"], "mismatch");
        assert_eq!(proof["requestedBrowserBuild"], "stock_chrome");
        assert_eq!(
            proof["mismatchReason"],
            "stock_chrome_resolved_to_stealth_executable"
        );
        assert_eq!(
            proof["actualExecutablePath"],
            "/opt/chromium-stealthcdp/chrome"
        );
    }
}
