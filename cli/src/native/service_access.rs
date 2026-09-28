//! No-launch access planning for service-owned browser/profile decisions.
//!
//! The access plan joins profile selection, site policy, provider, challenge,
//! and readiness state before a caller asks the service to launch or control a
//! browser. It is intentionally read-only so agents and software clients can
//! get the service recommendation without creating browser process pressure.

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use super::service_lifecycle::{rank_service_profiles_for_request, ProfileSelectionRequest};
use super::service_model::{
    builtin_site_policy, service_profile_seeding_handoff, BrowserBuild, BrowserHost,
    ControlInputProvider, ServiceIncidentEscalation, ServiceIncidentState, ServiceState,
    ViewStreamProvider,
};
use super::service_request::AuthenticatedServicePrincipal;

/// Parsed access-plan selector shared by HTTP and MCP resources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ServiceAccessPlanRequest {
    pub(crate) service_name: Option<String>,
    pub(crate) agent_name: Option<String>,
    pub(crate) task_name: Option<String>,
    pub(crate) client_subject_id: Option<String>,
    pub(crate) identity_assurance: Option<String>,
    /// Explicit daemon lane that must survive planning into the executable request.
    pub(crate) session_name: Option<String>,
    pub(crate) target_service_ids: Vec<String>,
    pub(crate) account_ids: Vec<String>,
    pub(crate) target_url: Option<String>,
    pub(crate) site_policy_id: Option<String>,
    pub(crate) challenge_id: Option<String>,
    pub(crate) readiness_profile_id: Option<String>,
    pub(crate) runtime_profile: Option<String>,
    pub(crate) browser_build: Option<BrowserBuild>,
    pub(crate) browser_build_explicit: bool,
    pub(crate) browser_host: Option<BrowserHost>,
    pub(crate) view_stream_provider: Option<ViewStreamProvider>,
    pub(crate) control_input_provider: Option<ControlInputProvider>,
    pub(crate) display_isolation: Option<String>,
}

impl ServiceAccessPlanRequest {
    fn profile_selection_request(&self) -> ProfileSelectionRequest {
        ProfileSelectionRequest {
            service_name: self.service_name.clone(),
            target_service_ids: self.target_service_ids.clone(),
            account_ids: self.account_ids.clone(),
            target_url: self.target_url.clone(),
            browser_build: self.browser_build,
        }
    }
}

/// Parse query parameters accepted by the access-plan HTTP and MCP surfaces.
pub(crate) fn parse_service_access_plan_query(
    params: Vec<(String, String)>,
) -> Result<ServiceAccessPlanRequest, String> {
    let mut request = ServiceAccessPlanRequest::default();

    for (key, value) in params {
        match key.as_str() {
            "serviceName" | "service_name" | "service-name" => {
                request.service_name = non_empty(value)
            }
            "agentName" | "agent_name" | "agent-name" => request.agent_name = non_empty(value),
            "taskName" | "task_name" | "task-name" => request.task_name = non_empty(value),
            "clientSubjectId" | "client_subject_id" | "client-subject-id" => {
                request.client_subject_id = non_empty(value)
            }
            "identityAssurance" | "identity_assurance" | "identity-assurance" => {
                request.identity_assurance = non_empty(value)
            }
            "sessionName" | "session_name" | "session-name" => {
                request.session_name = non_empty(value)
            }
            "targetServiceId" | "target_service_id" | "target-service-id" | "targetService"
            | "target_service" | "target-service" | "siteId" | "site_id" | "site-id"
            | "loginId" | "login_id" | "login-id" => {
                append_identity_values(&mut request.target_service_ids, &value);
            }
            "targetServiceIds" | "target_service_ids" | "target-service-ids" | "targetServices"
            | "target_services" | "target-services" | "siteIds" | "site_ids" | "site-ids"
            | "loginIds" | "login_ids" | "login-ids" => {
                append_identity_values(&mut request.target_service_ids, &value);
            }
            "accountId" | "account_id" | "account-id" | "account" => {
                append_identity_values(&mut request.account_ids, &value);
            }
            "accountIds" | "account_ids" | "account-ids" | "accounts" => {
                append_identity_values(&mut request.account_ids, &value);
            }
            "url" | "targetUrl" | "target_url" | "target-url" => {
                request.target_url = non_empty(value);
            }
            "sitePolicyId" | "site_policy_id" | "site-policy-id" => {
                request.site_policy_id = non_empty(value);
            }
            "challengeId" | "challenge_id" | "challenge-id" => {
                request.challenge_id = non_empty(value);
            }
            "readinessProfileId" | "readiness_profile_id" | "readiness-profile-id" => {
                request.readiness_profile_id = non_empty(value);
            }
            "runtimeProfile" | "runtime_profile" | "runtime-profile" | "profileId"
            | "profile_id" | "profile-id" => {
                request.runtime_profile = non_empty(value);
            }
            "browserBuild" | "browser_build" | "browser-build" => {
                request.browser_build = parse_browser_build(&value)?;
                request.browser_build_explicit = request.browser_build.is_some();
            }
            "browserHost" | "browser_host" | "browser-host" => {
                request.browser_host = parse_browser_host(&value)?;
            }
            "viewStreamProvider"
            | "view_stream_provider"
            | "view-stream-provider"
            | "viewStream"
            | "view_stream"
            | "view-stream" => {
                request.view_stream_provider = parse_view_stream_provider(&value)?;
            }
            "controlInputProvider"
            | "control_input_provider"
            | "control-input-provider"
            | "controlInput"
            | "control_input"
            | "control-input" => {
                request.control_input_provider = parse_control_input_provider(&value)?;
            }
            "displayIsolation" | "display_isolation" | "display-isolation" => {
                request.display_isolation = parse_display_isolation(&value)?;
            }
            "" => {}
            _ => {
                return Err(format!(
                    "Unknown service access plan query parameter: {}",
                    key
                ))
            }
        }
    }

    request.target_service_ids.sort();
    request.target_service_ids.dedup();
    request.account_ids.sort();
    request.account_ids.dedup();
    Ok(request)
}

/// Build the read-only service access plan from already-loaded service state.
pub(crate) fn service_access_plan_for_state(
    service_state: &ServiceState,
    request: ServiceAccessPlanRequest,
) -> Value {
    service_access_plan_for_state_with_principal(service_state, request, None)
}

pub(crate) fn service_access_plan_for_state_with_principal(
    service_state: &ServiceState,
    request: ServiceAccessPlanRequest,
    _authenticated_principal: Option<&AuthenticatedServicePrincipal>,
) -> Value {
    let selected = rank_service_profiles_for_request(
        service_state,
        &request.profile_selection_request(),
        true,
    )
    .into_iter()
    .next();
    let profile_id = request.runtime_profile.clone().or_else(|| {
        selected
            .as_ref()
            .map(|candidate| candidate.profile_id.clone())
    });
    let reason = selected
        .as_ref()
        .map(|candidate| format!("{:?}", candidate.reason).to_lowercase())
        .unwrap_or_else(|| "availability_first_open".to_string());
    let session_name = request
        .session_name
        .clone()
        .or_else(|| request.agent_name.clone())
        .unwrap_or_else(|| "default".to_string());
    json!({
        "schemaVersion": "agent-browser.service-access-plan.v2",
        "selectedProfile": profile_id.as_ref().map(|id| json!({
            "profileId": id,
            "reason": reason,
        })),
        "decision": {
            "profileId": profile_id,
            "profileReuse": {
                "recommendedAction": "open_or_reuse_browser_session",
                "reasons": ["browser_session_manager_authority"],
            },
            "serviceRequest": {
                "available": true,
                "request": {
                    "sessionName": session_name,
                    "runtimeProfile": request.runtime_profile,
                    "targetUrl": request.target_url,
                },
            },
        },
    })
}

/// Admission failure with the planner-owned denied decision, when applicable.
#[derive(Debug)]
pub(crate) struct ProfileRouteHintFailure {
    pub(crate) message: String,
    pub(crate) access_decision:
        Option<Box<super::service_profile_access_policy::ServiceProfileAccessDecision>>,
}

impl From<String> for ProfileRouteHintFailure {
    fn from(message: String) -> Self {
        Self {
            message,
            access_decision: None,
        }
    }
}

pub(crate) fn apply_shared_profile_route_hints_for_service_request(
    service_state: &ServiceState,
    command: &mut Value,
) -> Result<(), String> {
    apply_shared_profile_route_hints_for_service_request_with_principal(
        service_state,
        command,
        None,
    )
}

pub(crate) fn apply_shared_profile_route_hints_for_service_request_with_principal(
    service_state: &ServiceState,
    command: &mut Value,
    authenticated_principal: Option<&AuthenticatedServicePrincipal>,
) -> Result<(), String> {
    apply_shared_profile_route_hints_with_decision(service_state, command, authenticated_principal)
        .map_err(|failure| failure.message)
}

pub(crate) fn apply_shared_profile_route_hints_with_decision(
    service_state: &ServiceState,
    command: &mut Value,
    _authenticated_principal: Option<&AuthenticatedServicePrincipal>,
) -> Result<(), ProfileRouteHintFailure> {
    if !matches!(
        command.get("action").and_then(Value::as_str),
        Some("tab_new" | "remote_view_open")
    ) {
        return Ok(());
    }
    let requested_browser = command
        .get("browserId")
        .and_then(Value::as_str)
        .map(str::to_string);
    let requested_session = command
        .get("sessionName")
        .and_then(Value::as_str)
        .map(str::to_string);
    if requested_browser.is_some() != requested_session.is_some() {
        return Err("service_access_plan_incomplete_route_hints"
            .to_string()
            .into());
    }
    if requested_browser.is_some() {
        return Ok(());
    }
    let session_name = command
        .get("sessionName")
        .and_then(Value::as_str)
        .or_else(|| command.get("agentName").and_then(Value::as_str))
        .unwrap_or("default")
        .to_string();
    let session = service_state.sessions.get(&session_name);
    if let Some(session) = session {
        if let Some(browser_id) = session.browser_ids.first() {
            command["browserId"] = json!(browser_id);
            command["sessionName"] = json!(session_name);
        }
        if command.get("runtimeProfile").is_none() {
            if let Some(profile_id) = session.profile_id.as_deref() {
                command["runtimeProfile"] = json!(profile_id);
            }
        }
    }
    Ok(())
}

pub(crate) fn parse_browser_host(value: &str) -> Result<Option<BrowserHost>, String> {
    let Some(value) = non_empty(value.to_string()) else {
        return Ok(None);
    };
    let host = match value.as_str() {
        "local_headless" | "local-headless" => BrowserHost::LocalHeadless,
        "local_headed" | "local-headed" => BrowserHost::LocalHeaded,
        "docker_headed" | "docker-headed" => BrowserHost::DockerHeaded,
        "remote_headed" | "remote-headed" => BrowserHost::RemoteHeaded,
        "cloud_provider" | "cloud-provider" => BrowserHost::CloudProvider,
        "attached_existing" | "attached-existing" => BrowserHost::AttachedExisting,
        _ => return Err(format!("Unknown browserHost value: {}", value)),
    };
    Ok(Some(host))
}

pub(crate) fn parse_view_stream_provider(
    value: &str,
) -> Result<Option<ViewStreamProvider>, String> {
    let Some(value) = non_empty(value.to_string()) else {
        return Ok(None);
    };
    let provider = match value.as_str() {
        "cdp_screencast" | "cdp-screencast" => ViewStreamProvider::CdpScreencast,
        "chrome_tab_webrtc" | "chrome-tab-webrtc" => ViewStreamProvider::ChromeTabWebrtc,
        "virtual_display_webrtc" | "virtual-display-webrtc" => {
            ViewStreamProvider::VirtualDisplayWebrtc
        }
        "novnc" => ViewStreamProvider::Novnc,
        "rdp_gateway" | "rdp-gateway" | "rdp" => ViewStreamProvider::RdpGateway,
        "external_url" | "external-url" => ViewStreamProvider::ExternalUrl,
        _ => return Err(format!("Unknown viewStreamProvider value: {}", value)),
    };
    Ok(Some(provider))
}

pub(crate) fn parse_control_input_provider(
    value: &str,
) -> Result<Option<ControlInputProvider>, String> {
    let Some(value) = non_empty(value.to_string()) else {
        return Ok(None);
    };
    let provider = match value.as_str() {
        "cdp_input" | "cdp-input" | "cdp" => ControlInputProvider::CdpInput,
        "webrtc_input" | "webrtc-input" | "webrtc" => ControlInputProvider::WebrtcInput,
        "vnc_input" | "vnc-input" | "vnc" => ControlInputProvider::VncInput,
        "manual_attached_desktop"
        | "manual-attached-desktop"
        | "manual_desktop"
        | "manual-desktop"
        | "manual" => ControlInputProvider::ManualAttachedDesktop,
        _ => return Err(format!("Unknown controlInputProvider value: {}", value)),
    };
    Ok(Some(provider))
}

pub(crate) fn parse_display_isolation(value: &str) -> Result<Option<String>, String> {
    let Some(value) = non_empty(value.to_string()) else {
        return Ok(None);
    };
    let display = match value.as_str() {
        "private_virtual_display" | "private-virtual-display" | "private" => {
            "private_virtual_display"
        }
        "shared_display" | "shared-display" | "shared" => "shared_display",
        "ambient_display" | "ambient-display" | "ambient" => "ambient_display",
        _ => return Err(format!("Unknown displayIsolation value: {}", value)),
    };
    Ok(Some(display.to_string()))
}

fn browser_build_for_access_request(
    service_state: &ServiceState,
    request: &ServiceAccessPlanRequest,
) -> Option<BrowserBuild> {
    if let Some(site_policy_id) = request.site_policy_id.as_deref() {
        if let Some(browser_build) = service_state
            .site_policies
            .get(site_policy_id)
            .and_then(|policy| policy.browser_build)
        {
            return Some(browser_build);
        }
        if let Some(browser_build) =
            builtin_site_policy(site_policy_id).and_then(|policy| policy.browser_build)
        {
            return Some(browser_build);
        }
    }
    for target_service_id in &request.target_service_ids {
        if let Some(browser_build) = service_state
            .site_policies
            .get(target_service_id)
            .and_then(|policy| policy.browser_build)
        {
            return Some(browser_build);
        }
        if let Some(browser_build) =
            builtin_site_policy(target_service_id).and_then(|policy| policy.browser_build)
        {
            return Some(browser_build);
        }
    }
    service_state.default_browser_build
}

fn append_identity_values(values: &mut Vec<String>, raw: &str) {
    values.extend(
        raw.split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
    );
}

fn parse_browser_build(value: &str) -> Result<Option<BrowserBuild>, String> {
    let Some(value) = non_empty(value.to_string()) else {
        return Ok(None);
    };
    BrowserBuild::parse_label(&value)
        .map(Some)
        .ok_or_else(|| format!("Unknown browserBuild value: {value}"))
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn access_plan_monitor_findings(
    service_state: &ServiceState,
    target_service_ids: &[String],
) -> Value {
    let mut incident_ids = Vec::new();
    let mut monitor_ids = Vec::new();
    let mut monitor_results = Vec::new();
    let mut matched_target_service_ids = Vec::new();
    let mut due_monitor_ids = Vec::new();
    let mut never_checked_monitor_ids = Vec::new();
    let mut due_target_service_ids = Vec::new();
    let now = Utc::now();

    for monitor in service_state.monitors.values() {
        if monitor.state != super::service_model::MonitorState::Active {
            continue;
        }
        let super::service_model::MonitorTarget::ProfileReadiness(target_service_id) =
            &monitor.target
        else {
            continue;
        };
        if !target_matches_request(target_service_id, target_service_ids) {
            continue;
        }
        if profile_readiness_monitor_due_for_access_plan(monitor, now) {
            due_monitor_ids.push(monitor.id.clone());
            due_target_service_ids.push(target_service_id.clone());
            if monitor.last_checked_at.is_none() {
                never_checked_monitor_ids.push(monitor.id.clone());
            }
        }
    }

    for incident in &service_state.incidents {
        if incident.state != ServiceIncidentState::Active
            || incident.escalation != ServiceIncidentEscalation::MonitorAttention
        {
            continue;
        }
        let Some(target_service_id) = incident
            .monitor_target
            .as_ref()
            .and_then(|target| target.get("profile_readiness"))
            .and_then(|target| target.as_str())
        else {
            continue;
        };
        if !target_service_ids.is_empty()
            && !target_service_ids
                .iter()
                .any(|requested| requested == target_service_id)
        {
            continue;
        }
        incident_ids.push(incident.id.clone());
        if let Some(monitor_id) = incident.monitor_id.as_ref() {
            monitor_ids.push(monitor_id.clone());
        }
        if let Some(monitor_result) = incident.monitor_result.as_ref() {
            monitor_results.push(monitor_result.clone());
        }
        matched_target_service_ids.push(target_service_id.to_string());
    }

    incident_ids.sort();
    incident_ids.dedup();
    monitor_ids.sort();
    monitor_ids.dedup();
    monitor_results.sort();
    monitor_results.dedup();
    matched_target_service_ids.sort();
    matched_target_service_ids.dedup();
    due_monitor_ids.sort();
    due_monitor_ids.dedup();
    never_checked_monitor_ids.sort();
    never_checked_monitor_ids.dedup();
    due_target_service_ids.sort();
    due_target_service_ids.dedup();

    json!({
        "profileReadinessAttentionRequired": !incident_ids.is_empty(),
        "profileReadinessProbeDue": !due_monitor_ids.is_empty(),
        "profileReadinessIncidentIds": incident_ids,
        "profileReadinessMonitorIds": monitor_ids,
        "profileReadinessDueMonitorIds": due_monitor_ids,
        "profileReadinessNeverCheckedMonitorIds": never_checked_monitor_ids,
        "profileReadinessResults": monitor_results,
        "targetServiceIds": matched_target_service_ids,
        "dueTargetServiceIds": due_target_service_ids,
    })
}

fn target_matches_request(target_service_id: &str, target_service_ids: &[String]) -> bool {
    target_service_ids.is_empty()
        || target_service_ids
            .iter()
            .any(|requested| requested == target_service_id)
}

fn profile_readiness_monitor_due_for_access_plan(
    monitor: &super::service_model::SiteMonitor,
    now: DateTime<Utc>,
) -> bool {
    let Some(last_checked_at) = monitor.last_checked_at.as_deref() else {
        return true;
    };
    let Ok(last_checked_at) = DateTime::parse_from_rfc3339(last_checked_at) else {
        return true;
    };
    let elapsed_ms = now
        .signed_duration_since(last_checked_at.with_timezone(&Utc))
        .num_milliseconds();
    elapsed_ms >= 0 && elapsed_ms as u64 >= monitor.interval_ms
}

fn readiness_summary(readiness: Option<&Value>, target_service_ids: &[String]) -> Value {
    let manual_rows = readiness
        .and_then(|readiness| readiness["targetReadiness"].as_array())
        .map(|rows| {
            rows.iter()
                .filter(|row| readiness_row_matches_target(row, target_service_ids))
                .filter(|row| {
                    row["state"] == "needs_manual_seeding"
                        || row["manualSeedingRequired"].as_bool() == Some(true)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let target_service_ids = manual_rows
        .iter()
        .filter_map(|row| row["targetServiceId"].as_str())
        .collect::<Vec<_>>();
    let mut recommended_actions = manual_rows
        .iter()
        .filter_map(|row| row["recommendedAction"].as_str())
        .filter(|action| !action.is_empty())
        .collect::<Vec<_>>();
    recommended_actions.sort();
    recommended_actions.dedup();

    json!({
        "needsManualSeeding": manual_rows.iter().any(|row| row["state"] == "needs_manual_seeding"),
        "manualSeedingRequired": !manual_rows.is_empty(),
        "targetServiceIds": target_service_ids,
        "recommendedActions": recommended_actions,
    })
}

fn seeding_handoff_for_readiness(
    service_state: &ServiceState,
    readiness: Option<&Value>,
    readiness_summary: &Value,
) -> Value {
    if readiness_summary["manualSeedingRequired"].as_bool() != Some(true) {
        return Value::Null;
    }
    let Some(profile_id) = readiness.and_then(|readiness| readiness["profileId"].as_str()) else {
        return Value::Null;
    };
    let target_service_id = readiness_summary["targetServiceIds"]
        .as_array()
        .and_then(|targets| targets.iter().find_map(|target| target.as_str()));

    service_profile_seeding_handoff(service_state, profile_id, target_service_id)
        .unwrap_or(Value::Null)
}

fn readiness_recommended_action<'a>(
    readiness: Option<&'a Value>,
    target_service_ids: &[String],
) -> Option<&'a str> {
    readiness
        .and_then(|readiness| readiness["targetReadiness"].as_array())
        .and_then(|rows| {
            rows.iter().find_map(|row| {
                readiness_row_matches_target(row, target_service_ids)
                    .then(|| {
                        row["recommendedAction"]
                            .as_str()
                            .filter(|action| !action.is_empty())
                    })
                    .flatten()
            })
        })
}

fn readiness_profile_is_fresh_or_seeded(
    readiness: Option<&Value>,
    profile_id: &str,
    target_service_ids: &[String],
) -> bool {
    readiness
        .filter(|readiness| readiness["profileId"].as_str() == Some(profile_id))
        .and_then(|readiness| readiness["targetReadiness"].as_array())
        .is_some_and(|rows| {
            rows.iter().any(|row| {
                readiness_row_matches_target(row, target_service_ids)
                    && matches!(
                        row["state"].as_str(),
                        Some("fresh" | "seeded_unknown_freshness")
                    )
            })
        })
}

fn readiness_profile_needs_probe(readiness: Option<&Value>, target_service_ids: &[String]) -> bool {
    let Some(rows) = readiness.and_then(|readiness| readiness["targetReadiness"].as_array()) else {
        return true;
    };
    let matching_rows = rows
        .iter()
        .filter(|row| readiness_row_matches_target(row, target_service_ids))
        .collect::<Vec<_>>();
    matching_rows.is_empty()
        || matching_rows.iter().any(|row| {
            matches!(
                row["state"].as_str(),
                Some("unknown" | "stale" | "blocked_by_attached_devtools")
            )
        })
}

fn readiness_row_matches_target(row: &Value, target_service_ids: &[String]) -> bool {
    target_service_ids.is_empty()
        || row["targetServiceId"]
            .as_str()
            .is_some_and(|target_service_id| {
                target_service_ids
                    .iter()
                    .any(|requested| requested == target_service_id)
            })
}

#[cfg(any())]
mod tests {
    use std::collections::BTreeMap;

    use crate::native::service_model::ProfileOrigin;

    use super::*;
    use crate::native::service_model::{
        BrowserCapabilityRegistry, BrowserHealth, BrowserHost, BrowserProcess, BrowserProfile,
        BrowserSession, Challenge, ChallengeKind, InteractionMode, LeaseState, MonitorState,
        MonitorTarget, ProfileKeyringPolicy, ProfileReadinessState, ProfileSeedingMode,
        ProfileTargetReadiness, ProviderCapability, ProviderKind, RateLimitPolicy, ServiceIncident,
        ServiceProvider, SiteMonitor, SitePolicy, ViewStream,
    };
    use crate::native::service_profile_access_policy::{
        ProfileAccessMode, ProfilePermission, ServiceProfileAccessPolicy,
    };
    use serde_json::json;

    fn restricted_profile_policy(profile_id: &str) -> ServiceProfileAccessPolicy {
        ServiceProfileAccessPolicy {
            profile_id: profile_id.to_string(),
            mode: ProfileAccessMode::Restricted,
            default_permissions: vec![ProfilePermission::TabCreate],
            ..ServiceProfileAccessPolicy::default()
        }
    }

    #[test]
    fn service_access_plan_recommends_google_manual_seeding_before_attachable_work() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "google-work".to_string(),
                BrowserProfile {
                    id: "google-work".to_string(),
                    name: "Google work".to_string(),
                    user_data_dir: Some("/tmp/google-work-profile".to_string()),
                    site_policy_ids: vec!["google".to_string()],
                    target_service_ids: vec!["google".to_string()],
                    credential_provider_ids: vec!["manual".to_string()],
                    target_readiness: vec![ProfileTargetReadiness {
                        target_service_id: "google".to_string(),
                        state: ProfileReadinessState::NeedsManualSeeding,
                        manual_seeding_required: true,
                        evidence: "manual_seed_required_without_authenticated_hint".to_string(),
                        recommended_action:
                            "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
                                .to_string(),
                        seeding_mode: ProfileSeedingMode::DetachedHeadedNoCdp,
                        cdp_attachment_allowed_during_seeding: false,
                        preferred_keyring: Some(ProfileKeyringPolicy::BasicPasswordStore),
                        ..ProfileTargetReadiness::default()
                    }],
                    ..BrowserProfile::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "https://accounts.google.com".to_string(),
                    browser_host: Some(BrowserHost::LocalHeaded),
                    interaction_mode: InteractionMode::HumanLikeInput,
                    manual_login_preferred: true,
                    profile_required: true,
                    auth_providers: vec!["manual".to_string()],
                    challenge_policy: ChallengePolicy::ManualOnly,
                    allowed_challenge_providers: vec!["manual".to_string()],
                    ..SitePolicy::default()
                },
            )]),
            providers: BTreeMap::from([(
                "manual".to_string(),
                ServiceProvider {
                    id: "manual".to_string(),
                    kind: ProviderKind::ManualApproval,
                    display_name: "Manual approval".to_string(),
                    capabilities: vec![ProviderCapability::HumanApproval],
                    ..ServiceProvider::default()
                },
            )]),
            challenges: BTreeMap::from([(
                "challenge-1".to_string(),
                Challenge {
                    id: "challenge-1".to_string(),
                    kind: ChallengeKind::TwoFactor,
                    state: ChallengeState::WaitingForHuman,
                    provider_id: Some("manual".to_string()),
                    ..Challenge::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("JournalDownloader".to_string()),
                agent_name: Some("codex".to_string()),
                task_name: Some("probeGoogleLogin".to_string()),
                target_service_ids: vec!["google".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["query"]["serviceName"], "JournalDownloader");
        assert_eq!(plan["query"]["agentName"], "codex");
        assert_eq!(plan["query"]["taskName"], "probeGoogleLogin");
        assert_eq!(plan["query"]["namingWarnings"], json!([]));
        assert_eq!(plan["decision"]["hasNamingWarning"], false);
        assert_eq!(plan["selectedProfile"]["id"], "google-work");
        assert_eq!(plan["sitePolicy"]["id"], "google");
        assert_eq!(plan["providers"][0]["id"], "manual");
        assert_eq!(plan["challenges"][0]["id"], "challenge-1");
        assert_eq!(plan["readinessSummary"]["manualSeedingRequired"], true);
        assert_eq!(plan["seedingHandoff"]["profileId"], "google-work");
        assert_eq!(plan["seedingHandoff"]["targetServiceId"], "google");
        assert_eq!(
            plan["seedingHandoff"]["seedingMode"],
            "detached_headed_no_cdp"
        );
        assert_eq!(
            plan["seedingHandoff"]["command"],
            "agent-browser --runtime-profile google-work runtime login https://accounts.google.com"
        );
        assert_eq!(plan["decision"]["authProviderIds"][0], "manual");
        assert_eq!(plan["decision"]["challengeProviderIds"][0], "manual");
        assert_eq!(plan["decision"]["challengeStrategy"], "manual_only");
        assert_eq!(plan["decision"]["browserHost"], "local_headed");
        assert_eq!(plan["decision"]["launchPosture"]["source"], "site_policy");
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuild"],
            "stock_chrome"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSource"],
            "service_default"
        );
        assert_eq!(plan["decision"]["launchPosture"]["headed"], true);
        assert_eq!(plan["decision"]["launchPosture"]["requiresCdpFree"], false);
        assert_eq!(
            plan["decision"]["launchPosture"]["cdpAttachmentAllowed"],
            false
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["detachedFirstLoginRequired"],
            true
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["attachableAfterSeeding"],
            true
        );
        assert_eq!(plan["decision"]["interactionRisk"], "manual");
        assert_eq!(plan["decision"]["pacing"]["rateLimited"], false);
        assert_eq!(
            plan["decision"]["missingChallengeCapabilities"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert_eq!(plan["decision"]["manualActionRequired"], true);
        assert_eq!(plan["decision"]["manualSeedingRequired"], true);
        assert_eq!(
            plan["decision"]["freshnessUpdate"]["profileId"],
            "google-work"
        );
        assert_eq!(
            plan["decision"]["freshnessUpdate"]["recommendedAfterProbe"],
            true
        );
        assert_eq!(
            plan["decision"]["freshnessUpdate"]["http"]["route"],
            "/api/service/profiles/google-work/freshness"
        );
        assert_eq!(
            plan["decision"]["freshnessUpdate"]["mcp"]["tool"],
            "service_profile_freshness_update"
        );
        assert_eq!(
            plan["decision"]["freshnessUpdate"]["client"]["helper"],
            "updateServiceProfileFreshness"
        );
        assert_eq!(plan["decision"]["postSeedingProbe"]["available"], true);
        assert_eq!(
            plan["decision"]["postSeedingProbe"]["recommendedAfterClose"],
            true
        );
        assert_eq!(
            plan["decision"]["postSeedingProbe"]["profileId"],
            "google-work"
        );
        assert_eq!(
            plan["decision"]["postSeedingProbe"]["targetServiceId"],
            "google"
        );
        assert_eq!(
            plan["decision"]["postSeedingProbe"]["http"]["route"],
            "/api/service/profiles/google-work/freshness"
        );
        assert_eq!(
            plan["decision"]["postSeedingProbe"]["mcp"]["tool"],
            "service_profile_freshness_update"
        );
        assert_eq!(
            plan["decision"]["postSeedingProbe"]["client"]["helper"],
            "verifyServiceProfileSeeding"
        );
        assert_eq!(
            plan["decision"]["postSeedingProbe"]["serviceClientExample"]["script"],
            "examples/service-client/post-seeding-probe.mjs"
        );
        assert_eq!(
            plan["decision"]["postSeedingProbe"]["cli"]["command"],
            "agent-browser service profiles google-work verify-seeding google --state fresh --evidence <probe-evidence>"
        );
        assert_eq!(plan["decision"]["monitorRunDue"]["available"], false);
        assert_eq!(
            plan["decision"]["monitorRunDue"]["recommendedBeforeUse"],
            false
        );
        assert_eq!(
            plan["decision"]["monitorRunDue"]["http"]["route"],
            "/api/service/monitors/run-due"
        );
        assert_eq!(
            plan["decision"]["monitorRunDue"]["mcp"]["tool"],
            "service_monitors_run_due"
        );
        assert_eq!(
            plan["decision"]["monitorRunDue"]["client"]["helper"],
            "runServiceAccessPlanMonitorRunDue"
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);
        assert_eq!(
            plan["decision"]["serviceRequest"]["recommendedAfterManualAction"],
            true
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["blockedByManualAction"],
            true
        );
        assert_eq!(plan["decision"]["serviceRequest"]["action"], "tab_new");
        assert_eq!(
            plan["decision"]["serviceRequest"]["selectedProfileId"],
            "google-work"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["profileLeasePolicy"],
            "wait"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["serviceName"],
            "JournalDownloader"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["agentName"],
            "codex"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["taskName"],
            "probeGoogleLogin"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["targetServiceIds"][0],
            "google"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["profileLeasePolicy"],
            "wait"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["runtimeProfile"],
            "google-work"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["profile"],
            "/tmp/google-work-profile"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["blockedByManualAction"],
            true
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["manualSeedingRequired"],
            true
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["http"]["route"],
            "/api/service/request"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["mcp"]["tool"],
            "service_request"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["client"]["helper"],
            "requestServiceTab"
        );
        assert_eq!(
            plan["decision"]["recommendedAction"],
            "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
        );
        assert_eq!(plan["decision"]["attention"]["required"], true);
        assert_eq!(plan["decision"]["attention"]["owner"], "operator");
        assert_eq!(plan["decision"]["attention"]["severity"], "blocking");
        assert_eq!(
            plan["decision"]["attention"]["reason"],
            plan["decision"]["recommendedAction"]
        );
        assert!(plan["decision"]["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "site_policy_manual_login_preferred"));
    }

    #[test]
    fn service_access_plan_plans_managed_one_time_profile_for_operator_handoff() {
        let plan = service_access_plan_for_state(
            &ServiceState::default(),
            ServiceAccessPlanRequest {
                service_name: Some("sosdirect".to_string()),
                agent_name: Some("codex".to_string()),
                task_name: Some("temporary-login-payment".to_string()),
                target_url: Some(
                    "https://direct.sos.state.tx.us/acct/acct-templogin.asp".to_string(),
                ),
                browser_build: Some(BrowserBuild::StockChrome),
                browser_build_explicit: true,
                browser_host: Some(BrowserHost::RemoteHeaded),
                view_stream_provider: Some(ViewStreamProvider::RdpGateway),
                control_input_provider: Some(ControlInputProvider::ManualAttachedDesktop),
                display_isolation: Some("private_virtual_display".to_string()),
                ..ServiceAccessPlanRequest::default()
            },
        );
        let runtime_profile = plan["decision"]["oneTimeProfileRecommendation"]["runtimeProfile"]
            .as_str()
            .expect("runtime profile");

        assert_eq!(
            plan["decision"]["oneTimeProfileRecommendation"]["state"],
            "planned"
        );
        assert_eq!(
            plan["decision"]["oneTimeProfileRecommendation"]["profileClass"],
            "managed_one_time"
        );
        assert!(runtime_profile.starts_with("managed-one-time-"));
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["runtimeProfile"],
            runtime_profile
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["profileClass"],
            "managed_one_time"
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], true);
        assert_eq!(plan["decision"]["profileId"], Value::Null);
    }

    #[test]
    fn self_declared_ephemeral_client_receives_an_executable_disposable_profile() {
        let request = ServiceAccessPlanRequest {
            service_name: Some("browser-debugger".to_string()),
            agent_name: Some("codex".to_string()),
            task_name: Some("inspect-page".to_string()),
            client_subject_id: Some("client:debugger".to_string()),
            target_service_ids: vec!["example-site".to_string()],
            ..ServiceAccessPlanRequest::default()
        };
        let first = service_access_plan_for_state(&ServiceState::default(), request.clone());
        let second = service_access_plan_for_state(&ServiceState::default(), request);
        let runtime_profile = first["decision"]["serviceRequest"]["request"]["runtimeProfile"]
            .as_str()
            .expect("managed ephemeral profile");

        assert!(runtime_profile.starts_with("managed-ephemeral-"));
        assert_eq!(
            first["decision"]["oneTimeProfileRecommendation"]["code"],
            "managed_ephemeral_profile_planned"
        );
        assert_eq!(first["decision"]["serviceRequest"]["available"], true);
        assert_eq!(
            first["decision"]["profileReuse"]["recommendedAction"],
            "launch_new_browser"
        );
        assert_eq!(
            second["decision"]["serviceRequest"]["request"]["runtimeProfile"],
            runtime_profile
        );
    }

    #[test]
    fn service_access_plan_warns_on_arbitrary_one_time_runtime_profile() {
        let plan = service_access_plan_for_state(
            &ServiceState::default(),
            ServiceAccessPlanRequest {
                service_name: Some("sosdirect".to_string()),
                agent_name: Some("codex".to_string()),
                task_name: Some("temporary-login-payment".to_string()),
                target_url: Some(
                    "https://direct.sos.state.tx.us/acct/acct-templogin.asp".to_string(),
                ),
                runtime_profile: Some("tx-sos-temp-stock-b".to_string()),
                browser_build: Some(BrowserBuild::StockChrome),
                browser_build_explicit: true,
                browser_host: Some(BrowserHost::RemoteHeaded),
                view_stream_provider: Some(ViewStreamProvider::RdpGateway),
                control_input_provider: Some(ControlInputProvider::ManualAttachedDesktop),
                display_isolation: Some("private_virtual_display".to_string()),
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["oneTimeProfileRecommendation"]["state"],
            "warning"
        );
        assert_eq!(
            plan["decision"]["oneTimeProfileRecommendation"]["requestedRuntimeProfile"],
            "tx-sos-temp-stock-b"
        );
        assert_eq!(
            plan["decision"]["oneTimeProfileRecommendation"]["recommendedProfileClass"],
            "managed_one_time"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["runtimeProfile"],
            "tx-sos-temp-stock-b"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["profileClass"],
            "operator_supplied"
        );
    }

    #[test]
    fn service_access_plan_selects_explicit_known_runtime_profile() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "known-temp".to_string(),
                BrowserProfile {
                    id: "known-temp".to_string(),
                    name: "Known temp".to_string(),
                    target_service_ids: vec!["fixture-site".to_string()],
                    user_data_dir: Some("/tmp/known-temp-profile".to_string()),
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };
        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("sosdirect".to_string()),
                agent_name: Some("codex".to_string()),
                task_name: Some("temporary-login-payment".to_string()),
                target_url: Some(
                    "https://direct.sos.state.tx.us/acct/acct-templogin.asp".to_string(),
                ),
                runtime_profile: Some("known-temp".to_string()),
                browser_host: Some(BrowserHost::RemoteHeaded),
                view_stream_provider: Some(ViewStreamProvider::RdpGateway),
                control_input_provider: Some(ControlInputProvider::ManualAttachedDesktop),
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["query"]["runtimeProfile"], "known-temp");
        assert_eq!(plan["selectedProfile"]["id"], "known-temp");
        assert_eq!(plan["selectedProfileMatch"]["reason"], "explicit_profile");
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["runtimeProfile"],
            "known-temp"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["profile"],
            "/tmp/known-temp-profile"
        );
        assert!(plan["decision"]["oneTimeProfileRecommendation"].is_null());
        let unknown = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                runtime_profile: Some("requested-new-profile".to_string()),
                target_service_ids: vec!["fixture-site".to_string()],
                target_url: Some("about:blank".to_string()),
                ..ServiceAccessPlanRequest::default()
            },
        );
        assert!(
            unknown["selectedProfile"].is_null(),
            "an explicit unknown profile must not select another catalog profile"
        );
        assert_eq!(
            unknown["decision"]["serviceRequest"]["request"]["runtimeProfile"],
            "requested-new-profile"
        );
        assert!(unknown["decision"]["serviceRequest"]["request"]["profile"].is_null());
        assert!(unknown["selectedProfileSource"].is_null());
        assert!(unknown["selectedProfileMatch"].is_null());
    }

    #[test]
    fn service_access_plan_reports_missing_caller_labels() {
        let plan = service_access_plan_for_state(
            &ServiceState::default(),
            ServiceAccessPlanRequest {
                target_service_ids: vec!["acs".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["query"]["serviceName"], Value::Null);
        assert_eq!(plan["query"]["agentName"], Value::Null);
        assert_eq!(plan["query"]["taskName"], Value::Null);
        assert_eq!(
            plan["query"]["namingWarnings"],
            json!([
                "missing_service_name",
                "missing_agent_name",
                "missing_task_name"
            ])
        );
        assert_eq!(plan["query"]["hasNamingWarning"], true);
        assert_eq!(
            plan["decision"]["namingWarnings"],
            plan["query"]["namingWarnings"]
        );
        assert_eq!(plan["decision"]["hasNamingWarning"], true);
    }

    #[test]
    fn service_access_plan_reports_profile_readiness_monitor_attention() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "journal-acs".to_string(),
                BrowserProfile {
                    id: "journal-acs".to_string(),
                    name: "Journal ACS".to_string(),
                    target_service_ids: vec!["acs".to_string()],
                    shared_service_ids: vec!["JournalDownloader".to_string()],
                    target_readiness: vec![ProfileTargetReadiness {
                        target_service_id: "acs".to_string(),
                        state: ProfileReadinessState::Stale,
                        evidence: "freshness_expired_by_monitor:acs-freshness".to_string(),
                        recommended_action: "probe_target_auth_or_reseed_if_needed".to_string(),
                        ..ProfileTargetReadiness::default()
                    }],
                    ..BrowserProfile::default()
                },
            )]),
            incidents: vec![ServiceIncident {
                id: "monitor:acs-freshness".to_string(),
                monitor_id: Some("acs-freshness".to_string()),
                monitor_target: Some(json!({"profile_readiness": "acs"})),
                monitor_result: Some("profile_readiness_expired".to_string()),
                state: ServiceIncidentState::Active,
                escalation: ServiceIncidentEscalation::MonitorAttention,
                latest_timestamp: "2026-05-09T00:00:00Z".to_string(),
                latest_kind: "reconciliation_error".to_string(),
                ..ServiceIncident::default()
            }],
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("JournalDownloader".to_string()),
                target_service_ids: vec!["acs".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["monitorFindings"]["profileReadinessAttentionRequired"],
            true
        );
        assert_eq!(
            plan["monitorFindings"]["profileReadinessIncidentIds"],
            json!(["monitor:acs-freshness"])
        );
        assert_eq!(
            plan["monitorFindings"]["profileReadinessMonitorIds"],
            json!(["acs-freshness"])
        );
        assert_eq!(
            plan["monitorFindings"]["profileReadinessResults"],
            json!(["profile_readiness_expired"])
        );
        assert_eq!(
            plan["decision"]["recommendedAction"],
            "probe_target_auth_or_reseed_if_needed"
        );
        assert_eq!(plan["decision"]["attention"]["required"], true);
        assert_eq!(plan["decision"]["attention"]["owner"], "service");
        assert_eq!(plan["decision"]["attention"]["severity"], "warning");
        assert_eq!(plan["decision"]["monitorAttentionRequired"], true);
        assert!(plan["decision"]["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "profile_readiness_monitor_attention"));
    }

    #[test]
    fn service_access_plan_reports_due_profile_readiness_monitor_before_tab_request() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "journal-acs".to_string(),
                BrowserProfile {
                    id: "journal-acs".to_string(),
                    name: "Journal ACS".to_string(),
                    target_service_ids: vec!["acs".to_string()],
                    authenticated_service_ids: vec!["acs".to_string()],
                    shared_service_ids: vec!["JournalDownloader".to_string()],
                    target_readiness: vec![ProfileTargetReadiness {
                        target_service_id: "acs".to_string(),
                        login_id: Some("acs".to_string()),
                        state: ProfileReadinessState::Fresh,
                        evidence: "auth_probe_cookie_present".to_string(),
                        recommended_action: "use_profile".to_string(),
                        freshness_expires_at: Some("2999-05-01T00:00:01Z".to_string()),
                        ..ProfileTargetReadiness::default()
                    }],
                    ..BrowserProfile::default()
                },
            )]),
            monitors: BTreeMap::from([(
                "acs-freshness".to_string(),
                SiteMonitor {
                    id: "acs-freshness".to_string(),
                    name: "ACS freshness".to_string(),
                    target: MonitorTarget::ProfileReadiness("acs".to_string()),
                    state: MonitorState::Active,
                    last_checked_at: None,
                    interval_ms: 60_000,
                    ..SiteMonitor::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("JournalDownloader".to_string()),
                agent_name: Some("codex".to_string()),
                task_name: Some("probeACSwebsite".to_string()),
                target_service_ids: vec!["acs".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["monitorFindings"]["profileReadinessAttentionRequired"],
            false
        );
        assert_eq!(plan["monitorFindings"]["profileReadinessProbeDue"], true);
        assert_eq!(
            plan["monitorFindings"]["profileReadinessDueMonitorIds"],
            json!(["acs-freshness"])
        );
        assert_eq!(
            plan["monitorFindings"]["profileReadinessNeverCheckedMonitorIds"],
            json!(["acs-freshness"])
        );
        assert_eq!(
            plan["monitorFindings"]["dueTargetServiceIds"],
            json!(["acs"])
        );
        assert_eq!(plan["decision"]["monitorProbeDue"], true);
        assert_eq!(
            plan["decision"]["recommendedAction"],
            "run_due_profile_readiness_monitor"
        );
        assert_eq!(plan["decision"]["monitorRunDue"]["available"], true);
        assert_eq!(
            plan["decision"]["monitorRunDue"]["recommendedBeforeUse"],
            true
        );
        assert_eq!(
            plan["decision"]["monitorRunDue"]["monitorIds"],
            json!(["acs-freshness"])
        );
        assert_eq!(
            plan["decision"]["monitorRunDue"]["neverCheckedMonitorIds"],
            json!(["acs-freshness"])
        );
        assert_eq!(
            plan["decision"]["monitorRunDue"]["targetServiceIds"],
            json!(["acs"])
        );
        assert_eq!(
            plan["decision"]["monitorRunDue"]["http"]["route"],
            "/api/service/monitors/run-due"
        );
        assert_eq!(
            plan["decision"]["monitorRunDue"]["mcp"]["tool"],
            "service_monitors_run_due"
        );
        assert_eq!(
            plan["decision"]["monitorRunDue"]["client"]["helper"],
            "runServiceAccessPlanMonitorRunDue"
        );
        assert_eq!(
            plan["decision"]["monitorRunDue"]["fallbackClient"]["helper"],
            "runDueServiceMonitors"
        );
        assert_eq!(
            plan["decision"]["monitorRunDue"]["cli"]["command"],
            "agent-browser service monitors run-due"
        );
        assert!(plan["decision"]["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "profile_readiness_probe_due"));
        assert_eq!(plan["decision"]["serviceRequest"]["available"], true);
    }

    #[test]
    fn parse_service_access_plan_query_accepts_caller_labels() {
        let request = parse_service_access_plan_query(vec![
            ("service-name".to_string(), "JournalDownloader".to_string()),
            ("agentName".to_string(), "codex".to_string()),
            ("task_name".to_string(), "probeACSwebsite".to_string()),
            ("session-name".to_string(), "bill-soylei".to_string()),
            ("login-id".to_string(), "acs".to_string()),
        ])
        .unwrap();

        assert_eq!(request.service_name.as_deref(), Some("JournalDownloader"));
        assert_eq!(request.agent_name.as_deref(), Some("codex"));
        assert_eq!(request.task_name.as_deref(), Some("probeACSwebsite"));
        assert_eq!(request.session_name.as_deref(), Some("bill-soylei"));
        assert_eq!(request.target_service_ids, vec!["acs".to_string()]);
    }

    #[test]
    fn fresh_access_plan_rejects_missing_explicit_session_lane() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "bill-profile".to_string(),
                BrowserProfile {
                    id: "bill-profile".to_string(),
                    target_service_ids: vec!["bill".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("BooksReceipts".to_string()),
                agent_name: Some("codex".to_string()),
                task_name: Some("inspect-bill".to_string()),
                session_name: Some("bill-soylei".to_string()),
                target_service_ids: vec!["bill".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "blocked_by_explicit_session_route"
        );
        assert_eq!(plan["query"]["sessionName"], "bill-soylei");
        assert_eq!(
            plan["decision"]["recommendedAction"],
            "resolve_explicit_session_route"
        );
        assert_eq!(plan["decision"]["attention"]["required"], true);
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);
        assert_eq!(plan["decision"]["serviceRequest"]["request"], Value::Null);
    }

    #[test]
    fn fresh_access_plan_expands_explicit_session_to_unique_browser_route() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "bill-profile".to_string(),
                BrowserProfile {
                    id: "bill-profile".to_string(),
                    target_service_ids: vec!["bill".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-bill".to_string(),
                BrowserProcess {
                    id: "browser-bill".to_string(),
                    profile_id: Some("bill-profile".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["bill-soylei".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "bill-soylei".to_string(),
                BrowserSession {
                    id: "bill-soylei".to_string(),
                    profile_id: Some("bill-profile".to_string()),
                    browser_ids: vec!["browser-bill".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                session_name: Some("bill-soylei".to_string()),
                target_service_ids: vec!["bill".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "reuse_existing_browser"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["browserId"],
            "browser-bill"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["sessionName"],
            "bill-soylei"
        );
    }

    #[test]
    fn parse_service_access_plan_query_accepts_account_and_url_hints() {
        let request = parse_service_access_plan_query(vec![
            ("serviceName".to_string(), "CanvaCLI".to_string()),
            ("accountId".to_string(), "eric@example.com".to_string()),
            (
                "url".to_string(),
                "https://www.canva.com/designs".to_string(),
            ),
        ])
        .unwrap();

        assert_eq!(request.service_name.as_deref(), Some("CanvaCLI"));
        assert_eq!(request.account_ids, vec!["eric@example.com".to_string()]);
        assert_eq!(
            request.target_url.as_deref(),
            Some("https://www.canva.com/designs")
        );
    }

    #[test]
    fn parse_service_access_plan_query_accepts_remote_view_hints() {
        let request = parse_service_access_plan_query(vec![
            ("browserHost".to_string(), "remote_headed".to_string()),
            ("viewStreamProvider".to_string(), "rdp_gateway".to_string()),
            (
                "controlInputProvider".to_string(),
                "manual_attached_desktop".to_string(),
            ),
            (
                "displayIsolation".to_string(),
                "private_virtual_display".to_string(),
            ),
        ])
        .unwrap();

        assert_eq!(request.browser_host, Some(BrowserHost::RemoteHeaded));
        assert_eq!(
            request.view_stream_provider,
            Some(ViewStreamProvider::RdpGateway)
        );
        assert_eq!(
            request.control_input_provider,
            Some(ControlInputProvider::ManualAttachedDesktop)
        );
        assert_eq!(
            request.display_isolation.as_deref(),
            Some("private_virtual_display")
        );
    }

    #[test]
    fn service_access_plan_uses_url_derived_target_and_account_match() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "canva-work".to_string(),
                BrowserProfile {
                    id: "canva-work".to_string(),
                    name: "Canva work".to_string(),
                    target_service_ids: vec!["canva".to_string()],
                    account_ids: vec!["eric@example.com".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("CanvaCLI".to_string()),
                account_ids: vec!["eric@example.com".to_string()],
                target_url: Some("https://www.canva.com/designs".to_string()),
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["query"]["targetServiceIds"], json!(["canva"]));
        assert_eq!(plan["selectedProfile"]["id"], "canva-work");
        assert_eq!(plan["selectedProfileMatch"]["reason"], "account_match");
        assert_eq!(plan["selectedProfileMatch"]["matchedField"], "accountIds");
        assert_eq!(plan["sitePolicy"]["id"], "canva");
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["url"],
            "https://www.canva.com/designs"
        );
    }

    #[test]
    fn service_access_plan_recommends_selected_authenticated_profile() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "acs".to_string(),
                BrowserProfile {
                    id: "acs".to_string(),
                    name: "ACS".to_string(),
                    profile_origin: ProfileOrigin::ExternalByop,
                    target_service_ids: vec!["acs".to_string()],
                    authenticated_service_ids: vec!["acs".to_string()],
                    shared_service_ids: vec!["JournalDownloader".to_string()],
                    target_readiness: vec![ProfileTargetReadiness {
                        target_service_id: "acs".to_string(),
                        state: ProfileReadinessState::Fresh,
                        evidence: "authenticated_hint_present".to_string(),
                        recommended_action: "use_profile".to_string(),
                        ..ProfileTargetReadiness::default()
                    }],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("JournalDownloader".to_string()),
                target_service_ids: vec!["acs".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["selectedProfile"]["id"], "acs");
        assert_eq!(plan["selectedProfile"]["profileOrigin"], "external_byop");
        assert_eq!(
            plan["selectedProfileMatch"]["reason"],
            "authenticated_target"
        );
        assert_eq!(
            plan["decision"]["recommendedAction"],
            "use_selected_profile"
        );
        assert_eq!(plan["decision"]["manualActionRequired"], false);
        assert_eq!(plan["decision"]["freshnessUpdate"]["profileId"], "acs");
        assert_eq!(
            plan["decision"]["freshnessUpdate"]["recommendedAfterProbe"],
            false
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], true);
        assert_eq!(
            plan["decision"]["serviceRequest"]["recommendedAfterManualAction"],
            false
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["blockedByManualAction"],
            false
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["selectedProfileId"],
            "acs"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["serviceName"],
            "JournalDownloader"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["targetServiceIds"][0],
            "acs"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["client"]["package"],
            "@agent-browser/client/service-request"
        );
    }

    #[test]
    fn service_access_plan_recommends_reusing_compatible_live_browser() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "acs".to_string(),
                BrowserProfile {
                    id: "acs".to_string(),
                    name: "ACS".to_string(),
                    target_service_ids: vec!["acs".to_string()],
                    authenticated_service_ids: vec!["acs".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([
                (
                    "browser-primary".to_string(),
                    BrowserProcess {
                        id: "browser-primary".to_string(),
                        profile_id: Some("acs".to_string()),
                        host: BrowserHost::RemoteHeaded,
                        health: BrowserHealth::Ready,
                        display_isolation: Some("private_virtual_display".to_string()),
                        view_streams: vec![ViewStream {
                            provider: ViewStreamProvider::RdpGateway,
                            control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                            ..ViewStream::default()
                        }],
                        active_session_ids: vec!["session-primary".to_string()],
                        ..BrowserProcess::default()
                    },
                ),
                (
                    "browser-duplicate".to_string(),
                    BrowserProcess {
                        id: "browser-duplicate".to_string(),
                        profile_id: Some("acs".to_string()),
                        host: BrowserHost::RemoteHeaded,
                        health: BrowserHealth::Ready,
                        display_isolation: Some("private_virtual_display".to_string()),
                        view_streams: vec![ViewStream {
                            provider: ViewStreamProvider::RdpGateway,
                            control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                            ..ViewStream::default()
                        }],
                        active_session_ids: vec!["session-duplicate".to_string()],
                        ..BrowserProcess::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["acs".to_string()],
                browser_host: Some(BrowserHost::RemoteHeaded),
                view_stream_provider: Some(ViewStreamProvider::RdpGateway),
                control_input_provider: Some(ControlInputProvider::ManualAttachedDesktop),
                display_isolation: Some("private_virtual_display".to_string()),
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "reuse_existing_browser"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["reusableBrowserId"],
            "browser-duplicate"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["reusableSessionName"],
            "session-duplicate"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["browserId"],
            "browser-duplicate"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["sessionName"],
            "session-duplicate"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["compatibleLiveBrowserCount"],
            2
        );
        assert_eq!(plan["decision"]["profileReuse"]["duplicatePressure"], true);
        assert!(plan["decision"]["profileReuse"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("duplicate_live_browsers_for_profile")));
    }

    #[test]
    fn service_access_plan_requires_rejoin_for_expired_retained_session() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "bill-soylei".to_string(),
                BrowserProfile {
                    id: "bill-soylei".to_string(),
                    name: "BILL SoyLei".to_string(),
                    target_service_ids: vec!["bill".to_string()],
                    authenticated_service_ids: vec!["bill".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:retained-bill".to_string(),
                BrowserProcess {
                    id: "session:retained-bill".to_string(),
                    profile_id: Some("bill-soylei".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["retained-bill".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "retained-bill".to_string(),
                BrowserSession {
                    id: "retained-bill".to_string(),
                    profile_id: Some("bill-soylei".to_string()),
                    browser_ids: vec!["session:retained-bill".to_string()],
                    lease: LeaseState::Expired,
                    expires_at: Some("2026-09-12T04:43:45Z".to_string()),
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("BooksReceipts".to_string()),
                agent_name: Some("receipt-agent".to_string()),
                task_name: Some("review-bill".to_string()),
                target_service_ids: vec!["bill".to_string()],
                runtime_profile: Some("bill-soylei".to_string()),
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "rejoin_profile_lease"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["reusableSessionName"],
            Value::Null
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);
        assert_eq!(
            plan["decision"]["serviceRequest"]["acquisitionBlocker"],
            "expired_session_recovery_required"
        );
        assert!(plan["decision"]["profileReuse"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("expired_session_recovery_required")));
    }

    #[test]
    fn service_access_plan_reuses_ready_transferred_owner_for_tab_acquisition() {
        use crate::runtime_owner_transfer::{
            CleanupObligationState, ProfileOwner, ProfileOwnerState, RuntimeLaneLifecycleState,
            RuntimeLifecycleRecord,
        };

        let profile_path = "/tmp/agent-browser-access-plan-transferred-owner";
        let profile_identity_digest = crate::runtime_profile::canonical_profile_identity_digest(
            std::path::Path::new(profile_path),
        )
        .unwrap();
        let browser_id = "session:last30days-facebook--last30days-facebook";
        let session_name = "handoff-17959ea3e226ee61";
        let owner = ProfileOwner {
            owner_id: "owner-transferred".to_string(),
            profile_identity_digest: profile_identity_digest.clone(),
            state: ProfileOwnerState::Ready,
            owner_generation: 14,
            browser_id: browser_id.to_string(),
            daemon_session_route: session_name.to_string(),
            process_instance_digest: "process-digest".to_string(),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: "cdp-digest".to_string(),
            target_set_digest: "target-digest".to_string(),
            pending_transfer: None,
            last_transition: None,
        };
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "last30days-facebook".to_string(),
                BrowserProfile {
                    id: "last30days-facebook".to_string(),
                    name: "Last30Days social profile".to_string(),
                    user_data_dir: Some(profile_path.to_string()),
                    target_service_ids: vec!["x".to_string()],
                    authenticated_service_ids: vec!["x".to_string()],
                    shared_service_ids: vec!["last30days".to_string()],
                    default_browser_host: Some(BrowserHost::RemoteHeaded),
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    profile_id: Some("last30days-facebook".to_string()),
                    host: BrowserHost::AttachedExisting,
                    health: BrowserHealth::Ready,
                    display_isolation: Some("shared_display".to_string()),
                    view_streams: vec![ViewStream {
                        provider: ViewStreamProvider::RdpGateway,
                        control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                        ..ViewStream::default()
                    }],
                    active_session_ids: vec![session_name.to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                session_name.to_string(),
                BrowserSession {
                    id: session_name.to_string(),
                    profile_id: Some("last30days-facebook".to_string()),
                    browser_ids: vec![browser_id.to_string()],
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            runtime_owner_registry: crate::runtime_owner_transfer::RuntimeOwnerRegistryFixture {
                registry_revision: 659,
                owner_records: BTreeMap::from([(profile_identity_digest.clone(), owner)]),
                principal_records: BTreeMap::new(),
                lifecycle_rows: BTreeMap::from([(
                    browser_id.to_string(),
                    RuntimeLifecycleRecord {
                        logical_browser_id: browser_id.to_string(),
                        profile_identity_digest,
                        owner_generation: 14,
                        lifecycle_state: RuntimeLaneLifecycleState::Ready,
                        cleanup_obligation_state: CleanupObligationState::Owned,
                        ..RuntimeLifecycleRecord::default()
                    },
                )]),
            }
            .into_registry(),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("last30days".to_string()),
                agent_name: Some("x-scraper".to_string()),
                task_name: Some("x-feed".to_string()),
                target_service_ids: vec!["x".to_string()],
                runtime_profile: Some("last30days-facebook".to_string()),
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "reuse_existing_browser"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["reusableBrowserId"],
            browser_id
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["reusableSessionName"],
            session_name
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["compatibleLiveBrowserCount"],
            1
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["sameProfileLiveBrowserCount"],
            1
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["browserId"],
            browser_id
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["sessionName"],
            session_name
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], true);
        assert_eq!(plan["decision"]["attention"]["required"], false);
    }

    #[test]
    fn stale_transferring_owner_without_live_authority_does_not_block_cold_launch() {
        use crate::runtime_owner_transfer::{
            CleanupObligationState, ProfileOwner, ProfileOwnerState, RuntimeLaneLifecycleState,
            RuntimeLifecycleRecord,
        };

        let profile_path = "/tmp/agent-browser-access-plan-incompatible-ready-owner";
        let profile_identity_digest = crate::runtime_profile::canonical_profile_identity_digest(
            std::path::Path::new(profile_path),
        )
        .unwrap();
        let browser_id = "session:retained-owner";
        let owner = ProfileOwner {
            owner_id: "owner-ready".to_string(),
            profile_identity_digest: profile_identity_digest.clone(),
            state: ProfileOwnerState::Ready,
            owner_generation: 3,
            browser_id: browser_id.to_string(),
            daemon_session_route: "retained-owner".to_string(),
            process_instance_digest: "process-digest".to_string(),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: "cdp-digest".to_string(),
            target_set_digest: "target-digest".to_string(),
            pending_transfer: None,
            last_transition: None,
        };
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "acs".to_string(),
                BrowserProfile {
                    id: "acs".to_string(),
                    name: "ACS".to_string(),
                    user_data_dir: Some(profile_path.to_string()),
                    target_service_ids: vec!["acs".to_string()],
                    authenticated_service_ids: vec!["acs".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    profile_id: Some("acs".to_string()),
                    host: BrowserHost::LocalHeaded,
                    health: BrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            )]),
            runtime_owner_registry: crate::runtime_owner_transfer::RuntimeOwnerRegistryFixture {
                registry_revision: 21,
                owner_records: BTreeMap::from([(profile_identity_digest.clone(), owner)]),
                principal_records: BTreeMap::new(),
                lifecycle_rows: BTreeMap::from([(
                    browser_id.to_string(),
                    RuntimeLifecycleRecord {
                        logical_browser_id: browser_id.to_string(),
                        profile_identity_digest,
                        owner_generation: 3,
                        lifecycle_state: RuntimeLaneLifecycleState::Transferring,
                        cleanup_obligation_state: CleanupObligationState::Transferring,
                        ..RuntimeLifecycleRecord::default()
                    },
                )]),
            }
            .into_registry(),
            ..ServiceState::default()
        };

        let request = ServiceAccessPlanRequest {
            target_service_ids: vec!["acs".to_string()],
            browser_host: Some(BrowserHost::RemoteHeaded),
            ..ServiceAccessPlanRequest::default()
        };
        let plan = service_access_plan_for_state(&state, request.clone());
        let mut without_history = state.clone();
        without_history.runtime_owner_registry =
            crate::runtime_owner_transfer::RuntimeOwnerRegistry::default();
        let clean_plan = service_access_plan_for_state(&without_history, request);

        assert_eq!(
            plan["decision"]["recommendedAction"],
            "use_selected_profile"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "launch_new_browser"
        );
        assert_eq!(
            plan["decision"]["lifecycleReplacement"]["replacementEligible"],
            false
        );
        assert_eq!(plan["decision"]["attention"]["required"], false);
        assert_eq!(plan["decision"]["serviceRequest"]["available"], true);
        assert!(plan["decision"]["serviceRequest"]["request"].is_object());
        assert_eq!(
            plan["decision"]["serviceRequest"]["acquisitionBlocker"],
            Value::Null
        );
        assert_eq!(
            plan["decision"]["recommendedAction"],
            clean_plan["decision"]["recommendedAction"]
        );
        assert_eq!(
            plan["decision"]["profileReuse"],
            clean_plan["decision"]["profileReuse"]
        );
        assert_eq!(
            plan["decision"]["serviceRequest"],
            clean_plan["decision"]["serviceRequest"]
        );
    }

    #[test]
    fn service_request_route_hints_reuse_compatible_live_browser() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "x-social".to_string(),
                BrowserProfile {
                    id: "x-social".to_string(),
                    name: "X social".to_string(),
                    target_service_ids: vec!["x".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-x".to_string(),
                BrowserProcess {
                    id: "browser-x".to_string(),
                    profile_id: Some("x-social".to_string()),
                    host: BrowserHost::RemoteHeaded,
                    health: BrowserHealth::Ready,
                    display_isolation: Some("private_virtual_display".to_string()),
                    view_streams: vec![ViewStream {
                        provider: ViewStreamProvider::RdpGateway,
                        control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                        ..ViewStream::default()
                    }],
                    active_session_ids: vec!["operator-x".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "operator-x".to_string(),
                BrowserSession {
                    id: "operator-x".to_string(),
                    profile_id: Some("x-social".to_string()),
                    browser_ids: vec!["browser-x".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };
        let mut command = json!({
            "action": "tab_new",
            "runtimeProfile": "x-social",
            "siteId": "x",
            "browserHost": "remote_headed",
            "viewStreamProvider": "rdp_gateway",
            "controlInputProvider": "manual_attached_desktop",
            "displayIsolation": "private_virtual_display",
            "sessionName": "operator-x",
        });

        let request = service_access_plan_request_from_service_command(&command).unwrap();
        let artifact = service_access_plan_artifact_for_state_with_principal(&state, request, None);
        assert_eq!(
            artifact.acquisition.disposition(),
            ProfileAcquisitionDisposition::ReuseExistingBrowser
        );
        assert_eq!(artifact.acquisition.browser_id(), Some("browser-x"));
        assert_eq!(artifact.acquisition.session_name(), Some("operator-x"));
        assert_eq!(
            artifact.public_plan["decision"]["serviceRequest"]["request"]["browserId"],
            "browser-x"
        );
        assert_eq!(
            artifact.public_plan["decision"]["serviceRequest"]["request"]["sessionName"],
            "operator-x"
        );

        apply_shared_profile_route_hints_for_service_request(&state, &mut command).unwrap();

        assert_eq!(command["browserId"], "browser-x");
        assert_eq!(command["sessionName"], "operator-x");
        apply_shared_profile_route_hints_for_service_request(&state, &mut command)
            .expect("an exact planned route should remain idempotently valid");

        let mut retained = state.clone();
        retained.browsers.get_mut("browser-x").unwrap().host = BrowserHost::AttachedExisting;
        retained
            .profiles
            .get_mut("x-social")
            .unwrap()
            .default_browser_host = Some(BrowserHost::RemoteHeaded);
        let request = ServiceAccessPlanRequest {
            runtime_profile: Some("x-social".to_string()),
            service_name: Some("reuse-service".to_string()),
            agent_name: Some("reuse-agent".to_string()),
            task_name: Some("reuse-task".to_string()),
            ..ServiceAccessPlanRequest::default()
        };
        let planned = service_access_plan_for_state(&retained, request);
        let mut generated = planned["decision"]["serviceRequest"]["request"].clone();
        assert_eq!(generated["browserId"], "browser-x");
        assert!(generated["params"]["browserHost"].is_null());
        assert!(generated["params"]["displayIsolation"].is_null());
        apply_shared_profile_route_hints_for_service_request(&retained, &mut generated)
            .expect("replacement defaults must not contradict the selected reusable browser");

        let mut contradictory_complete_route = json!({
            "action": "tab_new",
            "runtimeProfile": "x-social",
            "siteId": "x",
            "browserId": "browser-other",
            "sessionName": "operator-x",
        });
        let error = apply_shared_profile_route_hints_for_service_request(
            &state,
            &mut contradictory_complete_route,
        )
        .unwrap_err();
        assert_eq!(error, "service_access_plan_route_browser_conflict");

        let mut contradictory_session_route = json!({
            "action": "tab_new",
            "runtimeProfile": "x-social",
            "siteId": "x",
            "browserId": "browser-x",
            "sessionName": "operator-other",
        });
        let error = apply_shared_profile_route_hints_for_service_request(
            &state,
            &mut contradictory_session_route,
        )
        .unwrap_err();
        assert_eq!(error, "service_access_plan_route_session_conflict");

        let mut invalid_command = json!({
            "action": "tab_new",
            "runtimeProfile": "x-social",
            "siteId": "x",
            "sessionName": "missing-session",
        });
        let error =
            apply_shared_profile_route_hints_for_service_request(&state, &mut invalid_command)
                .unwrap_err();
        assert_eq!(
            error,
            "service_access_plan_request_unavailable:explicit_session_route_invalid"
        );
        assert!(invalid_command.get("browserId").is_none());
    }

    #[test]
    fn unsafe_claim_any_route_adopts_explicit_foreign_session_without_principal_check() {
        let state = ServiceState {
            browsers: BTreeMap::from([(
                "browser-foreign".to_string(),
                BrowserProcess {
                    id: "browser-foreign".to_string(),
                    profile_id: Some("foreign-profile".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["foreign-session".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "foreign-session".to_string(),
                BrowserSession {
                    id: "foreign-session".to_string(),
                    profile_id: Some("foreign-profile".to_string()),
                    principal_id: Some("principal:foreign".to_string()),
                    browser_ids: vec!["browser-foreign".to_string()],
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };
        let mut command = json!({
            "action": "tab_new",
            "runtimeProfile": "foreign-profile",
            "sessionName": "foreign-session"
        });

        apply_unsafe_claim_any_route(&state, &mut command).unwrap();

        assert_eq!(command["browserId"], "browser-foreign");
        assert_eq!(command["sessionName"], "foreign-session");
        assert_eq!(command["runtimeProfile"], "foreign-profile");
        assert_eq!(command["profileLeaseUnsafeClaim"]["applied"], true);
        assert_eq!(
            command["profileLeaseUnsafeClaim"]["principalContinuityBypassed"],
            true
        );
    }

    #[test]
    fn service_access_plan_does_not_reuse_external_observed_browser() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "observed".to_string(),
                BrowserProfile {
                    id: "observed".to_string(),
                    name: "Observed Chrome".to_string(),
                    target_service_ids: vec!["auracall".to_string()],
                    authenticated_service_ids: vec!["auracall".to_string()],
                    profile_origin: ProfileOrigin::ExternalObserved,
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-observed".to_string(),
                BrowserProcess {
                    id: "browser-observed".to_string(),
                    profile_id: Some("observed".to_string()),
                    host: BrowserHost::AttachedExisting,
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-observed".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["auracall".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "launch_new_browser"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["reusableBrowserIds"],
            json!([])
        );
        assert!(plan["decision"]["profileReuse"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("external_observed_not_reusable")));
    }

    #[test]
    fn service_access_plan_reuses_external_byop_attached_browser_without_host_request() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "byop".to_string(),
                BrowserProfile {
                    id: "byop".to_string(),
                    name: "BYOP Chrome".to_string(),
                    target_service_ids: vec!["auracall".to_string()],
                    authenticated_service_ids: vec!["auracall".to_string()],
                    profile_origin: ProfileOrigin::ExternalByop,
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-byop".to_string(),
                BrowserProcess {
                    id: "browser-byop".to_string(),
                    profile_id: Some("byop".to_string()),
                    host: BrowserHost::AttachedExisting,
                    health: BrowserHealth::Ready,
                    view_streams: vec![ViewStream {
                        provider: ViewStreamProvider::CdpScreencast,
                        control_input: Some(ControlInputProvider::CdpInput),
                        ..ViewStream::default()
                    }],
                    active_session_ids: vec!["session-byop".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["auracall".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "reuse_existing_browser"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["profileProcessPolicy"],
            "exclusive_process"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["clientSharingPolicy"],
            "shared_browser_tabs"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["defaultAcquisition"],
            "tab_new"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["sharedAcquisition"]["policy"],
            "shared_browser_tabs"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["sharedAcquisition"]["mode"],
            "tab_new"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["sharedAcquisition"]["browserId"],
            "browser-byop"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["sharedAcquisition"]["sessionName"],
            "session-byop"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["sharedAcquisition"]["requiresRouteHints"],
            true
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["sharedAcquisition"]["routeHintFields"],
            json!(["browserId", "sessionName"])
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["reusableBrowserId"],
            "browser-byop"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["reusableSessionName"],
            "session-byop"
        );
        assert!(plan["decision"]["profileReuse"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("external_byop_browser_host_unconstrained")));
    }

    #[test]
    fn authenticated_cold_profile_acquisition_uses_a_principal_owned_session_lane() {
        use crate::native::service_principal::{
            AuthenticatedServicePrincipal, ServicePrincipalProvenance,
        };

        let authority = AuthenticatedServicePrincipal {
            principal_id: "principal:odollo-fulfillment".to_string(),
            profile_id: "odollo-fedex".to_string(),
            capability_id: "profile-capability-v1:odollo-fedex".to_string(),
            capability_revision: 1,
            provenance: ServicePrincipalProvenance::RegisteredCapability,
        };
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "odollo-fedex".to_string(),
                BrowserProfile {
                    id: "odollo-fedex".to_string(),
                    target_service_ids: vec!["fedex".to_string()],
                    authenticated_service_ids: vec!["fedex".to_string()],
                    access_policy: Some(restricted_profile_policy("odollo-fedex")),
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state_with_principal(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["fedex".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
            Some(&authority),
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "launch_new_browser"
        );
        let session_name = plan["decision"]["serviceRequest"]["request"]["sessionName"]
            .as_str()
            .expect("authenticated cold acquisition should carry a session route");
        assert_ne!(session_name, "default");
        assert!(session_name.starts_with("principal-profile-"));
        assert!(crate::validation::is_valid_session_name(session_name));
    }

    #[test]
    fn authenticated_cold_profile_session_round_trips_through_request_admission() {
        use crate::native::service_principal::{
            AuthenticatedServicePrincipal, ServicePrincipalProvenance,
        };

        let authority = AuthenticatedServicePrincipal {
            principal_id: "principal:odollo-fulfillment".to_string(),
            profile_id: "odollo-fedex".to_string(),
            capability_id: "profile-capability-v1:odollo-fedex".to_string(),
            capability_revision: 1,
            provenance: ServicePrincipalProvenance::RegisteredCapability,
        };
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "odollo-fedex".to_string(),
                BrowserProfile {
                    id: "odollo-fedex".to_string(),
                    target_service_ids: vec!["fedex".to_string()],
                    authenticated_service_ids: vec!["fedex".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };
        let first_plan = service_access_plan_for_state_with_principal(
            &state,
            ServiceAccessPlanRequest {
                runtime_profile: Some("odollo-fedex".to_string()),
                service_name: Some("OdolloFulfillment".to_string()),
                target_service_ids: vec!["fedex".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
            Some(&authority),
        );
        let mut command = first_plan["decision"]["serviceRequest"]["request"].clone();
        let expected_session = command["sessionName"].as_str().unwrap().to_string();

        apply_shared_profile_route_hints_for_service_request_with_principal(
            &state,
            &mut command,
            Some(&authority),
        )
        .unwrap();

        assert_eq!(command["sessionName"], expected_session);
        assert!(command.get("browserId").is_none());
        assert_eq!(
            command["serviceProfileRouteAuthorization"]["kind"],
            "authenticated_cold"
        );
        assert_eq!(
            command["serviceProfileRouteAuthorization"]["sessionName"],
            expected_session
        );
    }

    #[test]
    fn shared_local_cold_profile_session_round_trips_without_owner_proof() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "p158-shared".to_string(),
                BrowserProfile {
                    id: "p158-shared".to_string(),
                    access_policy: Some(ServiceProfileAccessPolicy::shared_local_default(
                        "p158-shared",
                    )),
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };
        let request = ServiceAccessPlanRequest {
            runtime_profile: Some("p158-shared".to_string()),
            service_name: Some("Last30Days".to_string()),
            agent_name: Some("x-scraper".to_string()),
            task_name: Some("x-feed".to_string()),
            ..ServiceAccessPlanRequest::default()
        };
        let first_plan = service_access_plan_for_state(&state, request.clone());
        let second_plan = service_access_plan_for_state(&state, request);
        let mut command = first_plan["decision"]["serviceRequest"]["request"].clone();
        let expected_session = command["sessionName"]
            .as_str()
            .expect("shared-local cold acquisition should carry a session route")
            .to_string();

        assert_ne!(expected_session, "default");
        assert!(expected_session.starts_with("shared-profile-"));
        assert_eq!(
            second_plan["decision"]["serviceRequest"]["request"]["sessionName"],
            expected_session
        );
        apply_shared_profile_route_hints_for_service_request(&state, &mut command).unwrap();

        assert_eq!(command["sessionName"], expected_session);
        assert!(command.get("browserId").is_none());
        assert!(command.get("serviceProfileRouteAuthorization").is_none());
    }

    #[test]
    fn authenticated_same_principal_reuses_its_coherent_retained_browser() {
        use crate::native::service_principal::{
            AuthenticatedServicePrincipal, ServicePrincipalProvenance,
        };

        let authority = AuthenticatedServicePrincipal {
            principal_id: "principal:odollo-fulfillment".to_string(),
            profile_id: "odollo-fedex".to_string(),
            capability_id: "profile-capability-v1:odollo-fedex".to_string(),
            capability_revision: 1,
            provenance: ServicePrincipalProvenance::RegisteredCapability,
        };
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "odollo-fedex".to_string(),
                BrowserProfile {
                    id: "odollo-fedex".to_string(),
                    target_service_ids: vec!["fedex".to_string()],
                    authenticated_service_ids: vec!["fedex".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-fedex".to_string(),
                BrowserProcess {
                    id: "browser-fedex".to_string(),
                    profile_id: Some("odollo-fedex".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-fedex-old".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-fedex-old".to_string(),
                BrowserSession {
                    id: "session-fedex-old".to_string(),
                    principal_id: Some(authority.principal_id.clone()),
                    principal_provenance: Some(authority.provenance),
                    profile_id: Some("odollo-fedex".to_string()),
                    browser_ids: vec!["browser-fedex".to_string()],
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state_with_principal(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["fedex".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
            Some(&authority),
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "reuse_existing_browser"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["reusableBrowserId"],
            "browser-fedex"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["sessionName"],
            "session-fedex-old"
        );
    }

    #[test]
    fn authenticated_foreign_principal_cannot_reuse_retained_browser() {
        use crate::native::service_principal::{
            AuthenticatedServicePrincipal, ServicePrincipalProvenance,
        };

        let owner_principal = "principal:odollo-fulfillment";
        let requester = AuthenticatedServicePrincipal {
            principal_id: "principal:foreign-service".to_string(),
            profile_id: "odollo-fedex".to_string(),
            capability_id: "profile-capability-v1:foreign-fedex".to_string(),
            capability_revision: 1,
            provenance: ServicePrincipalProvenance::RegisteredCapability,
        };
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "odollo-fedex".to_string(),
                BrowserProfile {
                    id: "odollo-fedex".to_string(),
                    target_service_ids: vec!["fedex".to_string()],
                    authenticated_service_ids: vec!["fedex".to_string()],
                    access_policy: Some(restricted_profile_policy("odollo-fedex")),
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-fedex".to_string(),
                BrowserProcess {
                    id: "browser-fedex".to_string(),
                    profile_id: Some("odollo-fedex".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-fedex-owner".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-fedex-owner".to_string(),
                BrowserSession {
                    id: "session-fedex-owner".to_string(),
                    principal_id: Some(owner_principal.to_string()),
                    principal_provenance: Some(ServicePrincipalProvenance::RegisteredCapability),
                    profile_id: Some("odollo-fedex".to_string()),
                    browser_ids: vec!["browser-fedex".to_string()],
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state_with_principal(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["fedex".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
            Some(&requester),
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "wait_for_foreign_principal"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["reusableBrowserId"],
            Value::Null
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);
    }

    #[test]
    fn capability_for_another_profile_cannot_authorize_a_cold_launch() {
        use crate::native::service_principal::{
            AuthenticatedServicePrincipal, ServicePrincipalProvenance,
        };

        let authority = AuthenticatedServicePrincipal {
            principal_id: "principal:foreign-service".to_string(),
            profile_id: "foreign-profile".to_string(),
            capability_id: "profile-capability-v1:foreign-profile".to_string(),
            capability_revision: 1,
            provenance: ServicePrincipalProvenance::RegisteredCapability,
        };
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "odollo-fedex".to_string(),
                BrowserProfile {
                    id: "odollo-fedex".to_string(),
                    access_policy: Some(restricted_profile_policy("odollo-fedex")),
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state_with_principal(
            &state,
            ServiceAccessPlanRequest {
                runtime_profile: Some("odollo-fedex".to_string()),
                ..ServiceAccessPlanRequest::default()
            },
            Some(&authority),
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "lifecycle_profile_identity_inconsistent"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["acquisitionBlocker"],
            "lifecycle_profile_identity_inconsistent"
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);
        assert_eq!(plan["decision"]["serviceRequest"]["request"], Value::Null);
    }

    #[test]
    fn capability_for_another_profile_cannot_reuse_same_principal_lane() {
        use crate::native::service_principal::{
            AuthenticatedServicePrincipal, ServicePrincipalProvenance,
        };

        let authority = AuthenticatedServicePrincipal {
            principal_id: "principal:odollo-fulfillment".to_string(),
            profile_id: "odollo-ups".to_string(),
            capability_id: "profile-capability-v1:odollo-ups".to_string(),
            capability_revision: 1,
            provenance: ServicePrincipalProvenance::RegisteredCapability,
        };
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "odollo-fedex".to_string(),
                BrowserProfile {
                    id: "odollo-fedex".to_string(),
                    target_service_ids: vec!["fedex".to_string()],
                    authenticated_service_ids: vec!["fedex".to_string()],
                    access_policy: Some(restricted_profile_policy("odollo-fedex")),
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-fedex".to_string(),
                BrowserProcess {
                    id: "browser-fedex".to_string(),
                    profile_id: Some("odollo-fedex".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-fedex".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-fedex".to_string(),
                BrowserSession {
                    id: "session-fedex".to_string(),
                    principal_id: Some(authority.principal_id.clone()),
                    principal_provenance: Some(authority.provenance),
                    profile_id: Some("odollo-fedex".to_string()),
                    browser_ids: vec!["browser-fedex".to_string()],
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state_with_principal(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["fedex".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
            Some(&authority),
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "lifecycle_profile_identity_inconsistent"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["blockingIdentityAxes"],
            json!(["profile"])
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);
    }

    #[test]
    fn unauthenticated_caller_cannot_reuse_principal_bound_browser() {
        use crate::native::service_principal::ServicePrincipalProvenance;

        let state = ServiceState {
            profiles: BTreeMap::from([(
                "books-bank".to_string(),
                BrowserProfile {
                    id: "books-bank".to_string(),
                    target_service_ids: vec!["bank".to_string()],
                    authenticated_service_ids: vec!["bank".to_string()],
                    access_policy: Some(restricted_profile_policy("books-bank")),
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-books-bank".to_string(),
                BrowserProcess {
                    id: "browser-books-bank".to_string(),
                    profile_id: Some("books-bank".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-books-bank".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-books-bank".to_string(),
                BrowserSession {
                    id: "session-books-bank".to_string(),
                    principal_id: Some("principal:books-receipts".to_string()),
                    principal_provenance: Some(ServicePrincipalProvenance::RegisteredCapability),
                    profile_id: Some("books-bank".to_string()),
                    browser_ids: vec!["browser-books-bank".to_string()],
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["bank".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "authenticate_for_profile_reuse"
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);
    }

    #[test]
    fn shared_local_self_declared_client_reuses_principal_bound_browser() {
        use crate::native::service_principal::ServicePrincipalProvenance;

        let state = ServiceState {
            profiles: BTreeMap::from([(
                "research-gov".to_string(),
                BrowserProfile {
                    id: "research-gov".to_string(),
                    target_service_ids: vec!["research-gov".to_string()],
                    authenticated_service_ids: vec!["research-gov".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-research-gov".to_string(),
                BrowserProcess {
                    id: "browser-research-gov".to_string(),
                    profile_id: Some("research-gov".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-research-gov".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-research-gov".to_string(),
                BrowserSession {
                    id: "session-research-gov".to_string(),
                    principal_id: Some("principal:prior-client".to_string()),
                    principal_provenance: Some(ServicePrincipalProvenance::RegisteredCapability),
                    profile_id: Some("research-gov".to_string()),
                    browser_ids: vec!["browser-research-gov".to_string()],
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                client_subject_id: Some("client:fieldwork".to_string()),
                identity_assurance: Some("self-declared".to_string()),
                target_service_ids: vec!["research-gov".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileAccess"]["policy"]["mode"],
            "shared-local"
        );
        assert_eq!(
            plan["decision"]["profileAccess"]["decision"]["allowed"],
            true
        );
        assert_eq!(
            plan["decision"]["profileAccess"]["decision"]["subject"]["assurance"],
            "self-declared"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "reuse_existing_browser"
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], true);
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["clientSubjectId"],
            "client:fieldwork"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["policyRevision"],
            1
        );
        assert!(
            plan["decision"]["serviceRequest"]["request"]["accessDecisionId"]
                .as_str()
                .is_some_and(|value| value.starts_with("profile-access-decision:"))
        );
    }

    #[test]
    fn caller_cannot_self_promote_identity_assurance_for_restricted_profile() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "research-gov".to_string(),
                BrowserProfile {
                    id: "research-gov".to_string(),
                    target_service_ids: vec!["research-gov".to_string()],
                    access_policy: Some(restricted_profile_policy("research-gov")),
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                client_subject_id: Some("client:fieldwork".to_string()),
                identity_assurance: Some("operator".to_string()),
                target_service_ids: vec!["research-gov".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileAccess"]["decision"]["subject"]["assurance"],
            "self-declared"
        );
        assert_eq!(
            plan["decision"]["profileAccess"]["decision"]["allowed"],
            false
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["acquisitionBlocker"],
            "profile_access_denied"
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);
        assert_eq!(
            plan["decision"]["recommendedAction"],
            plan["decision"]["profileAccess"]["decision"]["nextAction"]["action"]
        );
        assert_eq!(plan["decision"]["attention"]["severity"], "blocking");
        assert_eq!(
            plan["decision"]["attention"]["suggestedActions"],
            json!(["inspect_profile_access_policy"])
        );
    }

    #[test]
    fn same_principal_profile_contradiction_is_not_reported_as_lease_wait() {
        use crate::native::service_principal::{
            AuthenticatedServicePrincipal, ServicePrincipalProvenance,
        };

        let authority = AuthenticatedServicePrincipal {
            principal_id: "principal:last30days".to_string(),
            profile_id: "last30days-social".to_string(),
            capability_id: "profile-capability-v1:last30days-social".to_string(),
            capability_revision: 1,
            provenance: ServicePrincipalProvenance::RegisteredCapability,
        };
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "last30days-social".to_string(),
                BrowserProfile {
                    id: "last30days-social".to_string(),
                    target_service_ids: vec!["social".to_string()],
                    authenticated_service_ids: vec!["social".to_string()],
                    access_policy: Some(restricted_profile_policy("last30days-social")),
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-social".to_string(),
                BrowserProcess {
                    id: "browser-social".to_string(),
                    profile_id: Some("default".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-social-old".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-social-old".to_string(),
                BrowserSession {
                    id: "session-social-old".to_string(),
                    principal_id: Some(authority.principal_id.clone()),
                    principal_provenance: Some(authority.provenance),
                    profile_id: Some("last30days-social".to_string()),
                    browser_ids: vec!["browser-social".to_string()],
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state_with_principal(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["social".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
            Some(&authority),
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "lifecycle_profile_identity_inconsistent"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["blockingIdentityAxes"],
            json!(["profile"])
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);
    }

    #[test]
    fn service_access_plan_recommends_waiting_for_profile_lease() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "acs".to_string(),
                BrowserProfile {
                    id: "acs".to_string(),
                    name: "ACS".to_string(),
                    target_service_ids: vec!["acs".to_string()],
                    authenticated_service_ids: vec!["acs".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "holder-session".to_string(),
                BrowserSession {
                    id: "holder-session".to_string(),
                    profile_id: Some("acs".to_string()),
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["acs".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "wait_for_profile_lease"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["profileProcessPolicy"],
            "exclusive_process"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["clientSharingPolicy"],
            "shared_browser_tabs"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["defaultAcquisition"],
            "launch_new_browser"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["sharedAcquisition"]["policy"],
            "shared_browser_tabs"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["sharedAcquisition"]["mode"],
            Value::Null
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["sharedAcquisition"]["requiresRouteHints"],
            false
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["activeLeaseSessionIds"][0],
            "holder-session"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["profileLeasePolicy"],
            "wait"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["profileLeasePolicy"],
            "wait"
        );
    }

    #[test]
    fn terminal_session_history_cannot_change_access_plan_decision() {
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "last30days-social".to_string(),
                BrowserProfile {
                    id: "last30days-social".to_string(),
                    target_service_ids: vec!["social".to_string()],
                    authenticated_service_ids: vec!["social".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };
        let request = || ServiceAccessPlanRequest {
            target_service_ids: vec!["social".to_string()],
            ..ServiceAccessPlanRequest::default()
        };

        let before = service_access_plan_for_state(&state, request());
        assert_eq!(
            before["decision"]["profileReuse"]["recommendedAction"],
            "launch_new_browser"
        );

        state.sessions.extend([
            (
                "released-history".to_string(),
                BrowserSession {
                    id: "released-history".to_string(),
                    profile_id: Some("last30days-social".to_string()),
                    lease: LeaseState::Released,
                    ..BrowserSession::default()
                },
            ),
            (
                "expired-history".to_string(),
                BrowserSession {
                    id: "expired-history".to_string(),
                    profile_id: Some("last30days-social".to_string()),
                    lease: LeaseState::Expired,
                    ..BrowserSession::default()
                },
            ),
        ]);

        let after = service_access_plan_for_state(&state, request());
        assert_eq!(after["decision"], before["decision"]);
    }

    #[test]
    fn canonical_active_claim_controls_access_without_session_projection() {
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "last30days-social".to_string(),
                BrowserProfile {
                    id: "last30days-social".to_string(),
                    target_service_ids: vec!["social".to_string()],
                    authenticated_service_ids: vec!["social".to_string()],
                    access_policy: Some(restricted_profile_policy("last30days-social")),
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };
        let claim_now = Utc::now();
        let claim = state
            .acquire_lease_claim(AcquireLeaseClaimRequest {
                resource: LeaseResourceKey::profile("last30days-social"),
                parent_claim_id: None,
                principal_id: "principal:last30days".to_string(),
                capability_id: "capability:last30days".to_string(),
                capability_revision: 1,
                mode: LeaseClaimMode::Ephemeral,
                expected_claim_revision: 0,
                idempotency_key: "access-plan-claim".to_string(),
                now: claim_now.to_rfc3339(),
                expires_at: (claim_now + chrono::Duration::minutes(5)).to_rfc3339(),
                transition_deadline: None,
                recovery_controller_id: None,
                boot_epoch: None,
                owner_generation: None,
            })
            .unwrap();

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["social".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "authenticate_for_profile_reuse"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["activeClaimId"],
            claim.claim_id()
        );
        assert_eq!(plan["decision"]["profileReuse"]["activeLeaseCount"], 1);
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);

        let same_principal = AuthenticatedServicePrincipal {
            principal_id: "principal:last30days".to_string(),
            profile_id: "last30days-social".to_string(),
            capability_id: "capability:last30days".to_string(),
            capability_revision: 1,
            provenance:
                crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability,
        };
        let same = service_access_plan_for_state_with_principal(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["social".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
            Some(&same_principal),
        );
        assert_eq!(
            same["decision"]["profileReuse"]["recommendedAction"],
            "launch_new_browser"
        );

        let foreign_principal = AuthenticatedServicePrincipal {
            principal_id: "principal:foreign".to_string(),
            capability_id: "capability:foreign".to_string(),
            ..same_principal
        };
        let foreign = service_access_plan_artifact_for_state_with_principal(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["social".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
            Some(&foreign_principal),
        );
        assert_eq!(
            foreign.public_plan["decision"]["profileReuse"]["recommendedAction"],
            "wait_for_foreign_principal"
        );
        assert_eq!(
            foreign.acquisition.disposition(),
            ProfileAcquisitionDisposition::Blocked
        );
        assert_eq!(
            foreign.acquisition.acquisition_blocker(),
            Some("foreign_principal_profile_lease")
        );
    }

    #[test]
    fn service_access_plan_recommends_new_browser_when_no_reusable_lane_exists() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "acs".to_string(),
                BrowserProfile {
                    id: "acs".to_string(),
                    name: "ACS".to_string(),
                    target_service_ids: vec!["acs".to_string()],
                    authenticated_service_ids: vec!["acs".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "wrong-host".to_string(),
                BrowserProcess {
                    id: "wrong-host".to_string(),
                    profile_id: Some("acs".to_string()),
                    host: BrowserHost::LocalHeaded,
                    health: BrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };

        let artifact = service_access_plan_artifact_for_state_with_principal(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["acs".to_string()],
                browser_host: Some(BrowserHost::RemoteHeaded),
                view_stream_provider: Some(ViewStreamProvider::RdpGateway),
                control_input_provider: Some(ControlInputProvider::ManualAttachedDesktop),
                display_isolation: Some("private_virtual_display".to_string()),
                ..ServiceAccessPlanRequest::default()
            },
            None,
        );
        assert_eq!(
            artifact.acquisition.disposition(),
            ProfileAcquisitionDisposition::LaunchNewBrowser
        );
        assert_eq!(artifact.acquisition.browser_id(), None);
        let plan = artifact.public_plan;

        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "launch_new_browser"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["compatibleLiveBrowserCount"],
            0
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["sameProfileLiveBrowserCount"],
            1
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["reusableBrowserIds"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert!(plan["decision"]["profileReuse"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("no_compatible_live_browser")));
    }

    #[test]
    fn occupied_profile_explains_unavailable_browser_identity_cause() {
        let mut state = ServiceState::default();
        state.profiles.insert(
            "synthetic-profile".into(),
            BrowserProfile {
                id: "synthetic-profile".into(),
                ..BrowserProfile::default()
            },
        );
        let mut browser = BrowserProcess {
            id: "synthetic-browser".into(),
            profile_id: Some("synthetic-profile".into()),
            health: BrowserHealth::Degraded,
            ..BrowserProcess::default()
        };
        crate::native::service_health::apply_browser_health_observation(
            &mut browser,
            Some(&json!({
                "failureClass": "browser_process_identity_ambiguous",
                "processIdentityAssessmentReason": "process_observation_failed",
                "processObservationFailure": "linux_proc_exe_permission_denied; process_observer_timeout"
            })),
        );
        state.browsers.insert(browser.id.clone(), browser);
        state.sessions.insert(
            "synthetic-session".into(),
            BrowserSession {
                id: "synthetic-session".into(),
                profile_id: Some("synthetic-profile".into()),
                browser_ids: vec!["synthetic-browser".into()],
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        );
        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                runtime_profile: Some("synthetic-profile".into()),
                ..ServiceAccessPlanRequest::default()
            },
        );
        let reuse = &plan["decision"]["profileReuse"];
        assert_eq!(reuse["recommendedAction"], "wait_for_profile_lease");
        assert_eq!(
            reuse["unavailableBrowsers"][0]["browserId"],
            "synthetic-browser"
        );
        assert_eq!(
            reuse["unavailableBrowsers"][0]["processIdentityAssessmentReason"],
            "process_observation_failed"
        );
        assert_eq!(
            reuse["unavailableBrowsers"][0]["processObservationFailure"],
            "linux_proc_exe_permission_denied; process_observer_timeout"
        );
        assert!(reuse["reusableBrowserId"].is_null());
    }

    #[test]
    fn service_access_plan_exposes_terminal_lifecycle_replacement_eligibility() {
        use crate::native::service_principal::{
            AuthenticatedServicePrincipal, ServicePrincipalProvenance,
        };
        use crate::runtime_owner_transfer::{
            CleanupObligationState, ProfileOwner, ProfileOwnerState, RuntimeLaneLifecycleState,
            RuntimeLifecycleRecord,
        };

        let profile_path = "/tmp/agent-browser-access-plan-terminal-profile";
        let profile_identity_digest = crate::runtime_profile::canonical_profile_identity_digest(
            std::path::Path::new(profile_path),
        )
        .unwrap();
        let owner = ProfileOwner {
            owner_id: "owner-terminal".to_string(),
            profile_identity_digest: profile_identity_digest.clone(),
            state: ProfileOwnerState::Ready,
            owner_generation: 7,
            browser_id: "session:terminal-lane".to_string(),
            daemon_session_route: "terminal-lane".to_string(),
            process_instance_digest: "process-digest".to_string(),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: "cdp-digest".to_string(),
            target_set_digest: "target-digest".to_string(),
            pending_transfer: None,
            last_transition: None,
        };
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "bill-soylei".to_string(),
                BrowserProfile {
                    id: "bill-soylei".to_string(),
                    name: "BILL".to_string(),
                    user_data_dir: Some(profile_path.to_string()),
                    target_service_ids: vec!["bill".to_string()],
                    authenticated_service_ids: vec!["bill".to_string()],
                    access_policy: Some(ServiceProfileAccessPolicy {
                        mode: ProfileAccessMode::Restricted,
                        ..ServiceProfileAccessPolicy::shared_local_default("bill-soylei")
                    }),
                    ..BrowserProfile::default()
                },
            )]),
            runtime_owner_registry: crate::runtime_owner_transfer::RuntimeOwnerRegistryFixture {
                registry_revision: 11,
                owner_records: BTreeMap::from([(profile_identity_digest.clone(), owner)]),
                principal_records: BTreeMap::new(),
                lifecycle_rows: BTreeMap::from([(
                    "session:terminal-lane".to_string(),
                    RuntimeLifecycleRecord {
                        logical_browser_id: "session:terminal-lane".to_string(),
                        profile_identity_digest,
                        owner_generation: 7,
                        lifecycle_state: RuntimeLaneLifecycleState::Terminal,
                        cleanup_obligation_state: CleanupObligationState::Satisfied,
                        terminal_evidence: vec![
                            "exact_process_exited".to_string(),
                            "profile_lock_released".to_string(),
                        ],
                        ..RuntimeLifecycleRecord::default()
                    },
                )]),
            }
            .into_registry(),
            ..ServiceState::default()
        };
        let authority = AuthenticatedServicePrincipal {
            principal_id: "principal:bill-soylei".to_string(),
            profile_id: "bill-soylei".to_string(),
            capability_id: "profile-capability-v1:bill-soylei".to_string(),
            capability_revision: 1,
            provenance: ServicePrincipalProvenance::RegisteredCapability,
        };

        let plan = service_access_plan_for_state_with_principal(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["bill".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
            Some(&authority),
        );

        assert_eq!(
            plan["decision"]["lifecycleReplacement"]["logicalBrowserId"],
            "session:terminal-lane"
        );
        assert_eq!(
            plan["decision"]["lifecycleReplacement"]["replacementEligible"],
            true
        );
        assert_eq!(
            plan["decision"]["lifecycleReplacement"]["reason"],
            "terminal_cleanup_satisfied"
        );
        assert_eq!(
            plan["decision"]["lifecycleReplacement"]["replacementSessionName"],
            "terminal-lane"
        );
        let launch_session = plan["decision"]["serviceRequest"]["request"]["sessionName"]
            .as_str()
            .expect("terminal replacement should expose a fresh launch session");
        assert_ne!(launch_session, "terminal-lane");
        assert!(launch_session.starts_with("terminal-profile-"));
        assert_eq!(
            plan["decision"]["lifecycleReplacement"]["terminalEvidence"],
            json!(["exact_process_exited", "profile_lock_released"])
        );

        let explicit_plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                session_name: Some("terminal-lane".to_string()),
                target_service_ids: vec!["bill".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );
        assert_eq!(
            explicit_plan["decision"]["profileReuse"]["recommendedAction"],
            "launch_new_browser"
        );
        assert_eq!(
            explicit_plan["decision"]["serviceRequest"]["available"],
            false
        );
        assert_eq!(
            explicit_plan["decision"]["serviceRequest"]["acquisitionBlocker"],
            "profile_access_denied"
        );
        assert!(explicit_plan["decision"]["serviceRequest"]["request"].is_null());

        let mut copied_request = json!({
            "action": "tab_new",
            "runtimeProfile": "bill-soylei",
            "targetServiceIds": ["bill"],
        });
        apply_shared_profile_route_hints_for_service_request_with_principal(
            &state,
            &mut copied_request,
            Some(&authority),
        )
        .unwrap();
        assert_eq!(copied_request["sessionName"], launch_session);
        assert_eq!(
            copied_request["serviceProfileRouteAuthorization"]["kind"],
            "authenticated_cold"
        );

        let unattributed_plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["bill".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );
        assert_eq!(
            unattributed_plan["decision"]["serviceRequest"]["available"],
            false
        );
        assert_eq!(
            unattributed_plan["decision"]["serviceRequest"]["acquisitionBlocker"],
            "profile_access_denied"
        );
        assert!(unattributed_plan["decision"]["serviceRequest"]["request"].is_null());

        let mut copied_unattributed_request = json!({
            "action": "tab_new",
            "runtimeProfile": "bill-soylei",
            "targetServiceIds": ["bill"],
            "sessionName": launch_session,
        });
        let error = apply_shared_profile_route_hints_for_service_request(
            &state,
            &mut copied_unattributed_request,
        )
        .expect_err("the unauthenticated terminal launch must fail before daemon relay");
        assert_eq!(
            error,
            "service_access_plan_request_unavailable:profile_access_denied"
        );

        let mut exact_explicit_request = json!({
            "action": "tab_new",
            "runtimeProfile": "bill-soylei",
            "targetServiceIds": ["bill"],
            "sessionName": "terminal-lane",
        });
        let error = apply_shared_profile_route_hints_for_service_request(
            &state,
            &mut exact_explicit_request,
        )
        .expect_err("the historical terminal route cannot bypass principal authentication");
        assert_eq!(
            error,
            "service_access_plan_request_unavailable:profile_access_denied"
        );
        assert!(exact_explicit_request.get("browserId").is_none());

        state.profiles.get_mut("bill-soylei").unwrap().access_policy = Some(
            ServiceProfileAccessPolicy::shared_local_default("bill-soylei"),
        );
        let shared_local_plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("Last30Days".to_string()),
                agent_name: Some("x-scraper".to_string()),
                task_name: Some("x-feed".to_string()),
                target_service_ids: vec!["bill".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );
        assert_eq!(
            shared_local_plan["decision"]["profileAccess"]["decision"]["allowed"],
            true
        );
        assert_eq!(
            shared_local_plan["decision"]["serviceRequest"]["available"],
            true
        );
        assert!(shared_local_plan["decision"]["serviceRequest"]["acquisitionBlocker"].is_null());
        assert!(shared_local_plan["decision"]["serviceRequest"]["request"].is_object());
    }

    #[test]
    fn service_access_plan_does_not_require_manual_seeding_for_authenticated_google_profile() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "google-seeded".to_string(),
                BrowserProfile {
                    id: "google-seeded".to_string(),
                    name: "Google Seeded".to_string(),
                    target_service_ids: vec!["google".to_string()],
                    authenticated_service_ids: vec!["google".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("JournalDownloader".to_string()),
                agent_name: Some("codex".to_string()),
                task_name: Some("probeGoogleLogin".to_string()),
                target_service_ids: vec!["google".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["selectedProfile"]["id"], "google-seeded");
        assert_eq!(plan["readinessSummary"]["manualSeedingRequired"], false);
        assert_eq!(plan["readinessSummary"]["needsManualSeeding"], false);
        assert_eq!(
            plan["decision"]["recommendedAction"],
            "use_selected_profile"
        );
        assert_eq!(plan["decision"]["manualActionRequired"], false);
        assert_eq!(plan["decision"]["manualSeedingRequired"], false);
    }

    #[test]
    fn service_access_plan_uses_explicit_freshness_evidence() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "google-fresh".to_string(),
                BrowserProfile {
                    id: "google-fresh".to_string(),
                    name: "Google Fresh".to_string(),
                    target_service_ids: vec!["google".to_string()],
                    authenticated_service_ids: vec!["google".to_string()],
                    target_readiness: vec![ProfileTargetReadiness {
                        target_service_id: "google".to_string(),
                        state: ProfileReadinessState::Fresh,
                        evidence: "auth_probe_cookie_present".to_string(),
                        recommended_action: "use_profile".to_string(),
                        last_verified_at: Some("2026-05-06T12:00:00Z".to_string()),
                        freshness_expires_at: Some("2026-05-06T13:00:00Z".to_string()),
                        ..ProfileTargetReadiness::default()
                    }],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("JournalDownloader".to_string()),
                agent_name: Some("codex".to_string()),
                task_name: Some("probeGoogleLogin".to_string()),
                target_service_ids: vec!["google".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["readiness"]["targetReadiness"][0]["state"], "fresh");
        assert_eq!(
            plan["readiness"]["targetReadiness"][0]["evidence"],
            "auth_probe_cookie_present"
        );
        assert_eq!(
            plan["readiness"]["targetReadiness"][0]["lastVerifiedAt"],
            "2026-05-06T12:00:00Z"
        );
        assert_eq!(
            plan["decision"]["recommendedAction"],
            "use_selected_profile"
        );
        assert_eq!(plan["decision"]["manualActionRequired"], false);
    }

    #[test]
    fn service_access_plan_includes_advisory_browser_capability_evidence() {
        let state = ServiceState {
            default_browser_build: Some(BrowserBuild::StealthcdpChromium),
            profiles: BTreeMap::from([(
                "canva-work".to_string(),
                BrowserProfile {
                    id: "canva-work".to_string(),
                    name: "Canva work".to_string(),
                    target_service_ids: vec!["design".to_string()],
                    authenticated_service_ids: vec!["design".to_string()],
                    browser_build: Some(BrowserBuild::StealthcdpChromium),
                    ..BrowserProfile::default()
                },
            )]),
            browser_capability_registry: BrowserCapabilityRegistry {
                browser_hosts: vec![
                    json!({"id": "local-linux", "name": "Local Linux"}),
                    json!({"id": "other-host", "name": "Other Host"}),
                ],
                browser_executables: vec![
                    json!({
                        "id": "stealth-current",
                        "hostId": "local-linux",
                        "buildLabel": "stealthcdp_chromium"
                    }),
                    json!({
                        "id": "stock-current",
                        "hostId": "other-host",
                        "buildLabel": "stock_chrome"
                    }),
                ],
                browser_capabilities: vec![json!({
                    "id": "stealth-capability",
                    "hostId": "local-linux",
                    "executableId": "stealth-current",
                    "cdpSupported": true,
                    "cdpFreeLaunchSupported": true
                })],
                profile_compatibility: vec![json!({
                    "id": "canva-work-stealth",
                    "profileId": "canva-work",
                    "hostId": "local-linux",
                    "executableId": "stealth-current",
                    "compatible": true
                })],
                browser_preference_bindings: vec![json!({
                    "id": "canva-prefers-stealth",
                    "scope": "site",
                    "targetServiceIds": ["design"],
                    "accountIds": [],
                    "serviceNames": ["CanvaCLI"],
                    "taskNames": [],
                    "preferredHostId": "local-linux",
                    "preferredExecutableId": "stealth-current",
                    "preferredCapabilityId": "stealth-capability",
                    "browserBuild": "stealthcdp_chromium",
                    "priority": 100,
                    "reason": "canva_bot_sensitive"
                })],
                validation_evidence: vec![json!({
                    "id": "stealth-smoke",
                    "hostId": "local-linux",
                    "executableId": "stealth-current",
                    "capabilityId": "stealth-capability",
                    "kind": "cdp_attach",
                    "state": "passed",
                    "evidence": "navigator.webdriver=false"
                })],
                generated_at: Some("2026-05-13T00:00:00Z".to_string()),
            },
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("CanvaCLI".to_string()),
                target_service_ids: vec!["design".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["selectedProfile"]["id"], "canva-work");
        assert_eq!(plan["browserCapabilityEvidence"]["advisory"], true);
        assert_eq!(plan["browserCapabilityEvidence"]["routingApplied"], false);
        assert_eq!(
            plan["browserCapabilityEvidence"]["browserBuildLabel"],
            "stealthcdp_chromium"
        );
        assert_eq!(
            plan["browserCapabilityEvidence"]["browserExecutables"][0]["id"],
            "stealth-current"
        );
        assert_eq!(
            plan["browserCapabilityEvidence"]["browserHosts"][0]["id"],
            "local-linux"
        );
        assert_eq!(
            plan["browserCapabilityEvidence"]["browserPreferenceBindings"][0]["id"],
            "canva-prefers-stealth"
        );
        assert_eq!(
            plan["browserCapabilityEvidence"]["validationEvidence"][0]["id"],
            "stealth-smoke"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSelection"]["source"],
            "profile_default"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSelection"]["profileCompatibility"]
                ["status"],
            "compatible"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSelection"]["validationEvidence"]
                ["status"],
            "passed"
        );
        assert_eq!(
            plan["browserCapabilityEvidence"]["counts"]["browserExecutables"],
            1
        );
    }

    #[test]
    fn service_access_plan_does_not_borrow_profile_compatibility_from_another_profile() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "bill-soylei".to_string(),
                BrowserProfile {
                    id: "bill-soylei".to_string(),
                    name: "BILL SoyLei".to_string(),
                    target_service_ids: vec!["bill".to_string()],
                    account_ids: vec!["soylei".to_string()],
                    authenticated_service_ids: vec!["bill".to_string()],
                    browser_build: Some(BrowserBuild::StockChrome),
                    ..BrowserProfile::default()
                },
            )]),
            browser_capability_registry: BrowserCapabilityRegistry {
                browser_hosts: vec![json!({"id": "tenant-desktop"})],
                browser_executables: vec![json!({
                    "id": "tenant-chrome",
                    "hostId": "tenant-desktop",
                    "buildLabel": "stock_chrome"
                })],
                browser_capabilities: vec![json!({
                    "id": "tenant-chrome-capability",
                    "hostId": "tenant-desktop",
                    "executableId": "tenant-chrome"
                })],
                profile_compatibility: vec![json!({
                    "id": "other-tenant-profile-compatible",
                    "profileId": "bill-other-tenant",
                    "hostId": "tenant-desktop",
                    "executableId": "tenant-chrome",
                    "compatible": true
                })],
                ..BrowserCapabilityRegistry::default()
            },
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("BillCLI".to_string()),
                target_service_ids: vec!["bill".to_string()],
                account_ids: vec!["soylei".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["selectedProfile"]["id"], "bill-soylei");
        assert_eq!(
            plan["browserCapabilityEvidence"]["counts"]["profileCompatibility"],
            0
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSelection"]["profileCompatibility"]
                ["status"],
            "not_declared"
        );
    }

    #[test]
    fn service_access_plan_selects_compatible_profile_before_incompatible_retained_profile() {
        let profiles = ["a-incompatible-retained", "z-compatible-retained"]
            .into_iter()
            .map(|profile_id| {
                (
                    profile_id.to_string(),
                    BrowserProfile {
                        id: profile_id.to_string(),
                        name: profile_id.to_string(),
                        target_service_ids: vec!["public-registry".to_string()],
                        browser_build: Some(BrowserBuild::StockChrome),
                        persistent: true,
                        ..BrowserProfile::default()
                    },
                )
            })
            .collect();
        let state = ServiceState {
            profiles,
            browser_capability_registry: BrowserCapabilityRegistry {
                browser_hosts: vec![json!({
                    "id": "local-desktop",
                    "hostKind": "local",
                    "reachable": true,
                    "lifecycleOwner": "agent_browser"
                })],
                browser_executables: vec![json!({
                    "id": "stock-current",
                    "hostId": "local-desktop",
                    "buildLabel": "stock_chrome"
                })],
                browser_capabilities: vec![json!({
                    "id": "stock-headed",
                    "hostId": "local-desktop",
                    "executableId": "stock-current",
                    "headedSupported": true,
                    "cdpSupported": true
                })],
                profile_compatibility: vec![
                    json!({
                        "id": "incompatible-retained-stock",
                        "profileId": "a-incompatible-retained",
                        "hostId": "local-desktop",
                        "executableId": "stock-current",
                        "compatible": false
                    }),
                    json!({
                        "id": "compatible-retained-stock",
                        "profileId": "z-compatible-retained",
                        "hostId": "local-desktop",
                        "executableId": "stock-current",
                        "compatible": true
                    }),
                ],
                browser_preference_bindings: vec![json!({
                    "id": "public-registry-stock",
                    "scope": "site",
                    "targetServiceIds": ["public-registry"],
                    "preferredHostId": "local-desktop",
                    "preferredExecutableId": "stock-current",
                    "preferredCapabilityId": "stock-headed",
                    "browserBuild": "stock_chrome",
                    "priority": 100
                })],
                ..BrowserCapabilityRegistry::default()
            },
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["public-registry".to_string()],
                browser_build: Some(BrowserBuild::StockChrome),
                browser_build_explicit: true,
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["selectedProfile"]["id"], "z-compatible-retained");
        assert_eq!(plan["profileSelection"]["status"], "selected");
        assert_eq!(
            plan["profileSelection"]["compatibilityIds"],
            json!(["compatible-retained-stock"])
        );
        assert_eq!(plan["selectedProfileMatch"]["reason"], "target_match");
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSelection"]["profileCompatibility"]
                ["status"],
            "compatible"
        );
    }

    #[test]
    fn service_access_plan_rejects_explicit_incompatible_retained_profile_without_effect() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "retained-incompatible".to_string(),
                BrowserProfile {
                    id: "retained-incompatible".to_string(),
                    name: "Retained incompatible".to_string(),
                    target_service_ids: vec!["public-registry".to_string()],
                    browser_build: Some(BrowserBuild::StockChrome),
                    persistent: true,
                    ..BrowserProfile::default()
                },
            )]),
            browser_capability_registry: BrowserCapabilityRegistry {
                browser_hosts: vec![json!({"id": "local-desktop"})],
                browser_executables: vec![json!({
                    "id": "stock-current",
                    "hostId": "local-desktop",
                    "buildLabel": "stock_chrome"
                })],
                profile_compatibility: vec![json!({
                    "id": "retained-incompatible-stock",
                    "profileId": "retained-incompatible",
                    "hostId": "local-desktop",
                    "executableId": "stock-current",
                    "compatible": false
                })],
                browser_preference_bindings: vec![json!({
                    "id": "public-registry-stock",
                    "scope": "site",
                    "targetServiceIds": ["public-registry"],
                    "preferredHostId": "local-desktop",
                    "preferredExecutableId": "stock-current",
                    "browserBuild": "stock_chrome",
                    "priority": 100
                })],
                ..BrowserCapabilityRegistry::default()
            },
            ..ServiceState::default()
        };
        let state_before = serde_json::to_value(&state).unwrap();

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["public-registry".to_string()],
                runtime_profile: Some("retained-incompatible".to_string()),
                browser_build: Some(BrowserBuild::StockChrome),
                browser_build_explicit: true,
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert!(plan["selectedProfile"].is_null());
        assert_eq!(plan["profileSelection"]["status"], "rejected");
        assert_eq!(
            plan["profileSelection"]["reason"],
            "profile_compatibility_missing_or_blocked"
        );
        assert_eq!(plan["profileSelection"]["effect"], "no_effect");
        assert_eq!(
            plan["profileSelection"]["rejectedProfileId"],
            "retained-incompatible"
        );
        assert_eq!(
            plan["profileSelection"]["compatibilityIds"],
            json!(["retained-incompatible-stock"])
        );
        assert_eq!(
            plan["decision"]["recommendedAction"],
            "select_compatible_profile_or_request_throwaway_browser"
        );
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);
        assert!(plan["decision"]["serviceRequest"]["request"].is_null());
        let mut command = json!({
            "action": "tab_new",
            "runtimeProfile": "retained-incompatible",
            "browserBuild": "stock_chrome",
            "targetServiceIds": ["public-registry"]
        });
        assert_eq!(
            apply_shared_profile_route_hints_for_service_request_with_principal(
                &state,
                &mut command,
                None,
            )
            .unwrap_err(),
            "service_access_plan_request_unavailable:profile_compatibility_missing_or_blocked"
        );
        assert_eq!(serde_json::to_value(&state).unwrap(), state_before);
    }

    #[test]
    fn service_access_plan_applies_browser_preference_binding_to_recommendation() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "only-works-profile".to_string(),
                BrowserProfile {
                    id: "only-works-profile".to_string(),
                    name: "Only works profile".to_string(),
                    target_service_ids: vec!["only-works-on-chrome".to_string()],
                    account_ids: vec!["myuser".to_string()],
                    authenticated_service_ids: vec!["only-works-on-chrome".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            browser_capability_registry: BrowserCapabilityRegistry {
                browser_hosts: vec![json!({"id": "windows-desktop-1", "name": "Windows desktop"})],
                browser_executables: vec![json!({
                    "id": "windows-chrome-stable",
                    "hostId": "windows-desktop-1",
                    "buildLabel": "stock_chrome"
                })],
                browser_capabilities: vec![json!({
                    "id": "windows-chrome-capability",
                    "hostId": "windows-desktop-1",
                    "executableId": "windows-chrome-stable",
                    "cdpSupported": true
                })],
                browser_preference_bindings: vec![
                    json!({
                        "id": "default-new-identities-use-stealthcdp",
                        "scope": "global",
                        "browserBuild": "stealthcdp_chromium",
                        "priority": 10
                    }),
                    json!({
                        "id": "only-works-on-chrome-myuser-primary",
                        "scope": "account",
                        "targetServiceIds": ["only-works-on-chrome"],
                        "accountIds": ["myuser"],
                        "preferredHostId": "windows-desktop-1",
                        "preferredExecutableId": "windows-chrome-stable",
                        "preferredCapabilityId": "windows-chrome-capability",
                        "browserBuild": "stock_chrome",
                        "priority": 100,
                        "reason": "site_requires_stock_chrome"
                    }),
                ],
                validation_evidence: vec![json!({
                    "id": "windows-chrome-smoke",
                    "hostId": "windows-desktop-1",
                    "executableId": "windows-chrome-stable",
                    "capabilityId": "windows-chrome-capability",
                    "state": "passed"
                })],
                ..BrowserCapabilityRegistry::default()
            },
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("Downloader".to_string()),
                target_service_ids: vec!["only-works-on-chrome".to_string()],
                account_ids: vec!["myuser".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["selectedProfile"]["id"], "only-works-profile");
        assert_eq!(plan["query"]["browserBuild"], "stock_chrome");
        assert_eq!(plan["browserCapabilityEvidence"]["routingApplied"], true);
        assert_eq!(
            plan["browserCapabilityEvidence"]["routingScope"],
            "access_plan_recommendation"
        );
        assert_eq!(
            plan["browserCapabilityEvidence"]["selectedPreferenceBinding"]["id"],
            "only-works-on-chrome-myuser-primary"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuild"],
            "stock_chrome"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSource"],
            "browser_preference_binding"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSelection"]["evidenceSource"],
            "service.browserCapabilityRegistry"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSelection"]
                ["selectedPreferenceBindingId"],
            "only-works-on-chrome-myuser-primary"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSelection"]["validationEvidence"]
                ["status"],
            "passed"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["browserBuild"],
            "stock_chrome"
        );
        assert_eq!(
            plan["decision"]["browserCapabilityPreflight"]["available"],
            true
        );
        assert_eq!(
            plan["decision"]["browserCapabilityPreflight"]["recommendedBeforeUse"],
            true
        );
        assert_eq!(
            plan["decision"]["browserCapabilityPreflight"]["request"]["browserBuild"],
            "stock_chrome"
        );
        assert_eq!(
            plan["decision"]["browserCapabilityPreflight"]["request"]["runtimeProfile"],
            "only-works-profile"
        );
        assert_eq!(
            plan["decision"]["browserCapabilityPreflight"]["http"]["route"],
            "/api/service/browser-capability/preflight"
        );
        assert_eq!(
            plan["decision"]["browserCapabilityPreflight"]["mcp"]["tool"],
            "service_browser_capability_preflight"
        );
        assert_eq!(
            plan["decision"]["browserCapabilityPreflight"]["client"]["helper"],
            "runServiceAccessPlanBrowserCapabilityPreflight"
        );

        let target_only_plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("Downloader".to_string()),
                target_service_ids: vec!["only-works-on-chrome".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            target_only_plan["query"]["browserBuild"],
            "stealthcdp_chromium"
        );
        assert_eq!(
            target_only_plan["browserCapabilityEvidence"]["selectedPreferenceBinding"]["id"],
            "default-new-identities-use-stealthcdp"
        );
        assert_eq!(
            target_only_plan["decision"]["launchPosture"]["browserBuildSource"],
            "browser_preference_binding"
        );
    }

    #[test]
    fn service_access_plan_explicit_browser_build_wins_over_preference_binding() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "only-works-profile".to_string(),
                BrowserProfile {
                    id: "only-works-profile".to_string(),
                    name: "Only works profile".to_string(),
                    target_service_ids: vec!["only-works-on-chrome".to_string()],
                    account_ids: vec!["myuser".to_string()],
                    authenticated_service_ids: vec!["only-works-on-chrome".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            browser_capability_registry: BrowserCapabilityRegistry {
                browser_preference_bindings: vec![json!({
                    "id": "only-works-on-chrome-myuser-primary",
                    "scope": "account",
                    "targetServiceIds": ["only-works-on-chrome"],
                    "accountIds": ["myuser"],
                    "browserBuild": "stealthcdp_chromium",
                    "priority": 100
                })],
                ..BrowserCapabilityRegistry::default()
            },
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["only-works-on-chrome".to_string()],
                account_ids: vec!["myuser".to_string()],
                browser_build: Some(BrowserBuild::StockChrome),
                browser_build_explicit: true,
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["query"]["browserBuild"], "stock_chrome");
        assert_eq!(plan["browserCapabilityEvidence"]["routingApplied"], false);
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuild"],
            "stock_chrome"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSource"],
            "request"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSelection"]["operatorOverride"],
            true
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSelection"]["evidenceSource"],
            "operator_request"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["browserBuild"],
            "stock_chrome"
        );
    }

    #[test]
    fn service_access_plan_explains_challenge_provider_fit() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "canva".to_string(),
                BrowserProfile {
                    id: "canva".to_string(),
                    name: "Canva".to_string(),
                    target_service_ids: vec!["canva".to_string()],
                    authenticated_service_ids: vec!["canva".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "canva".to_string(),
                SitePolicy {
                    id: "canva".to_string(),
                    origin_pattern: "https://www.canva.com".to_string(),
                    challenge_policy: ChallengePolicy::ProviderAllowed,
                    allowed_challenge_providers: vec!["captcha".to_string()],
                    ..SitePolicy::default()
                },
            )]),
            providers: BTreeMap::from([(
                "captcha".to_string(),
                ServiceProvider {
                    id: "captcha".to_string(),
                    kind: ProviderKind::Captcha,
                    display_name: "Captcha solver".to_string(),
                    capabilities: vec![ProviderCapability::CaptchaSolve],
                    ..ServiceProvider::default()
                },
            )]),
            challenges: BTreeMap::from([(
                "captcha-1".to_string(),
                Challenge {
                    id: "captcha-1".to_string(),
                    kind: ChallengeKind::Captcha,
                    state: ChallengeState::Detected,
                    ..Challenge::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["canva".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["decision"]["challengeProviderIds"][0], "captcha");
        assert_eq!(plan["decision"]["challengeStrategy"], "provider_allowed");
        assert_eq!(plan["decision"]["missingChallengeCapabilities"], json!([]));
        assert_eq!(
            plan["decision"]["recommendedAction"],
            "wait_for_or_invoke_challenge_provider"
        );
        assert_eq!(plan["decision"]["attention"]["required"], true);
        assert_eq!(plan["decision"]["attention"]["owner"], "provider");
        assert_eq!(plan["decision"]["attention"]["severity"], "warning");
    }

    #[test]
    fn service_access_plan_reports_missing_challenge_provider_capability() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "secure".to_string(),
                BrowserProfile {
                    id: "secure".to_string(),
                    name: "Secure app".to_string(),
                    target_service_ids: vec!["secure".to_string()],
                    authenticated_service_ids: vec!["secure".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "secure".to_string(),
                SitePolicy {
                    id: "secure".to_string(),
                    origin_pattern: "https://secure.example".to_string(),
                    challenge_policy: ChallengePolicy::ProviderAllowed,
                    allowed_challenge_providers: vec!["sms".to_string()],
                    ..SitePolicy::default()
                },
            )]),
            providers: BTreeMap::new(),
            challenges: BTreeMap::from([(
                "two-factor-1".to_string(),
                Challenge {
                    id: "two-factor-1".to_string(),
                    kind: ChallengeKind::TwoFactor,
                    state: ChallengeState::Detected,
                    ..Challenge::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["secure".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["decision"]["challengeProviderIds"], json!([]));
        assert_eq!(plan["decision"]["challengeStrategy"], "missing_provider");
        assert_eq!(
            plan["decision"]["missingChallengeCapabilities"],
            json!(["email_code", "human_approval", "sms_code", "totp_code"])
        );
    }

    #[test]
    fn service_access_plan_explains_pacing_and_interaction_risk() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "microsoft".to_string(),
                BrowserProfile {
                    id: "microsoft".to_string(),
                    name: "Microsoft".to_string(),
                    target_service_ids: vec!["microsoft".to_string()],
                    authenticated_service_ids: vec!["microsoft".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "microsoft".to_string(),
                SitePolicy {
                    id: "microsoft".to_string(),
                    origin_pattern: "https://login.microsoftonline.com".to_string(),
                    interaction_mode: InteractionMode::HumanLikeInput,
                    rate_limit: RateLimitPolicy {
                        min_action_delay_ms: Some(450),
                        jitter_ms: Some(250),
                        cooldown_ms: Some(2_000),
                        max_parallel_sessions: Some(1),
                        retry_budget: Some(2),
                    },
                    ..SitePolicy::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["microsoft".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["decision"]["interactionRisk"], "hardened");
        assert_eq!(plan["decision"]["pacing"]["minActionDelayMs"], 450);
        assert_eq!(plan["decision"]["pacing"]["jitterMs"], 250);
        assert_eq!(plan["decision"]["pacing"]["cooldownMs"], 2_000);
        assert_eq!(plan["decision"]["pacing"]["maxParallelSessions"], 1);
        assert_eq!(plan["decision"]["pacing"]["retryBudget"], 2);
        assert_eq!(plan["decision"]["pacing"]["rateLimited"], true);
        assert_eq!(plan["decision"]["pacing"]["jittered"], true);
        assert_eq!(plan["decision"]["pacing"]["singleSessionRecommended"], true);
    }

    #[test]
    fn service_access_plan_explains_profile_default_launch_posture() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "remote".to_string(),
                BrowserProfile {
                    id: "remote".to_string(),
                    name: "Remote profile".to_string(),
                    target_service_ids: vec!["remote-app".to_string()],
                    authenticated_service_ids: vec!["remote-app".to_string()],
                    default_browser_host: Some(BrowserHost::RemoteHeaded),
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["remote-app".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["decision"]["browserHost"], "remote_headed");
        assert_eq!(
            plan["decision"]["launchPosture"]["browserHost"],
            "remote_headed"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["source"],
            "profile_default"
        );
        assert_eq!(plan["decision"]["launchPosture"]["headed"], true);
        assert_eq!(
            plan["decision"]["launchPosture"]["remoteViewRecommended"],
            true
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["viewStreamProvider"],
            "rdp_gateway"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["viewStreamProviderSource"],
            "service_default"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["controlInputProvider"],
            "manual_attached_desktop"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["controlInputProviderSource"],
            "view_stream"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["displayIsolation"],
            "private_virtual_display"
        );
        assert!(plan["decision"]["launchPosture"]["rationale"]
            .as_array()
            .unwrap()
            .contains(&json!("remote_headed_private_display_default")));
        assert_eq!(
            plan["decision"]["launchPosture"]["cdpAttachmentAllowed"],
            true
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["detachedFirstLoginRequired"],
            false
        );
    }

    #[test]
    fn service_access_plan_uses_requested_remote_view_posture() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "default".to_string(),
                BrowserProfile {
                    id: "default".to_string(),
                    name: "Default profile".to_string(),
                    target_service_ids: vec!["example".to_string()],
                    authenticated_service_ids: vec!["example".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["example".to_string()],
                browser_host: Some(BrowserHost::RemoteHeaded),
                view_stream_provider: Some(ViewStreamProvider::RdpGateway),
                control_input_provider: Some(ControlInputProvider::ManualAttachedDesktop),
                display_isolation: Some("private_virtual_display".to_string()),
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["decision"]["browserHost"], "remote_headed");
        assert_eq!(plan["decision"]["launchPosture"]["source"], "request");
        assert_eq!(
            plan["decision"]["launchPosture"]["viewStreamProvider"],
            "rdp_gateway"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["viewStreamProviderSource"],
            "request"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["controlInputProvider"],
            "manual_attached_desktop"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["controlInputProviderSource"],
            "request"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["displayIsolation"],
            "private_virtual_display"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["params"]["headless"],
            false
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["params"]["browserHost"],
            "remote_headed"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["params"]["viewStreamProvider"],
            "rdp_gateway"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["params"]["controlInputProvider"],
            "manual_attached_desktop"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["params"]["displayIsolation"],
            "private_virtual_display"
        );
    }

    #[test]
    fn service_access_plan_uses_builtin_identity_provider_policy() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "google-work".to_string(),
                BrowserProfile {
                    id: "google-work".to_string(),
                    name: "Google Work".to_string(),
                    target_service_ids: vec!["google".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["google".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["sitePolicy"]["id"], "google");
        assert_eq!(plan["sitePolicySource"]["id"], "google");
        assert_eq!(plan["sitePolicySource"]["source"], "builtin");
        assert_eq!(plan["sitePolicySource"]["matchedBy"], "target_service_id");
        assert_eq!(plan["sitePolicySource"]["overrideable"], true);
        assert_eq!(
            plan["sitePolicy"]["originPattern"],
            "https://accounts.google.com"
        );
        assert_eq!(plan["decision"]["browserHost"], "local_headed");
        assert_eq!(plan["decision"]["interactionRisk"], "hardened");
        assert_eq!(plan["decision"]["pacing"]["singleSessionRecommended"], true);
        assert_eq!(plan["decision"]["launchPosture"]["requiresCdpFree"], false);
        assert_eq!(
            plan["decision"]["launchPosture"]["detachedFirstLoginRequired"],
            false
        );
        assert_eq!(
            plan["decision"]["recommendedAction"],
            "verify_or_seed_profile_before_authenticated_work"
        );
    }

    #[test]
    fn service_access_plan_uses_builtin_cdp_free_canva_policy() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "canva-work".to_string(),
                BrowserProfile {
                    id: "canva-work".to_string(),
                    name: "Canva Work".to_string(),
                    target_service_ids: vec!["canva".to_string()],
                    authenticated_service_ids: vec!["canva".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["canva".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["sitePolicy"]["id"], "canva");
        assert_eq!(plan["sitePolicySource"]["source"], "builtin");
        assert_eq!(plan["sitePolicy"]["requiresCdpFree"], true);
        assert_eq!(plan["sitePolicy"]["browserBuild"], "cdp_free_headed");
        assert_eq!(plan["decision"]["browserHost"], "local_headed");
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);
        assert_eq!(plan["decision"]["serviceRequest"]["blockedByCdpFree"], true);
        assert_eq!(plan["decision"]["serviceRequest"]["requiresCdpFree"], true);
        assert_eq!(
            plan["decision"]["serviceRequest"]["cdpAttachmentAllowed"],
            false
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["requiresCdpFree"],
            true
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["cdpAttachmentAllowed"],
            false
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["cdpFreeAvailability"]["applies"],
            true
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["cdpFreeAvailability"]["availableCommands"][0],
            "cdp_free_launch"
        );
        assert!(
            plan["decision"]["serviceRequest"]["cdpFreeAvailability"]["unsupportedCommands"]
                .as_array()
                .unwrap()
                .iter()
                .any(|command| command == "snapshot")
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["cdpFreeAvailability"]["client"]["summaryHelper"],
            "summarizeServiceCdpFreeLaunchAvailability"
        );
        assert_eq!(plan["decision"]["launchPosture"]["requiresCdpFree"], true);
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuild"],
            "cdp_free_headed"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSource"],
            "requires_cdp_free"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["cdpAttachmentAllowed"],
            false
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["attachableAfterSeeding"],
            false
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["rationale"]
                .as_array()
                .unwrap()
                .iter()
                .any(|reason| reason == "site_policy_requires_cdp_free"),
            true
        );
    }

    #[test]
    fn service_access_plan_uses_builtin_ups_remote_view_headed_policy() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "ups-work".to_string(),
                BrowserProfile {
                    id: "ups-work".to_string(),
                    name: "UPS Work".to_string(),
                    target_service_ids: vec!["ups".to_string()],
                    authenticated_service_ids: vec!["ups".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_url: Some(
                    "https://www.ups.com/track?tracknum=1Z035CX1YW53854301".to_string(),
                ),
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["query"]["targetServiceIds"], json!(["ups"]));
        assert_eq!(plan["sitePolicy"]["id"], "ups");
        assert_eq!(plan["sitePolicySource"]["source"], "builtin");
        assert_eq!(plan["sitePolicySource"]["matchedBy"], "target_service_id");
        assert_eq!(plan["sitePolicy"]["browserHost"], "remote_headed");
        assert_eq!(plan["sitePolicy"]["browserBuild"], "stealthcdp_chromium");
        assert_eq!(plan["sitePolicy"]["viewStream"], "rdp_gateway");
        assert_eq!(
            plan["sitePolicy"]["controlInput"],
            "manual_attached_desktop"
        );
        assert_eq!(plan["decision"]["browserHost"], "remote_headed");
        assert_eq!(plan["decision"]["launchPosture"]["headed"], true);
        assert_eq!(
            plan["decision"]["launchPosture"]["remoteViewRecommended"],
            true
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuild"],
            "stealthcdp_chromium"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["browserBuildSource"],
            "site_policy"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["viewStreamProvider"],
            "rdp_gateway"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["viewStreamProviderSource"],
            "site_policy"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["controlInputProvider"],
            "manual_attached_desktop"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["controlInputProviderSource"],
            "site_policy"
        );
        assert_eq!(
            plan["decision"]["launchPosture"]["displayIsolation"],
            "private_virtual_display"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["params"]["headless"],
            false
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["params"]["browserHost"],
            "remote_headed"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["params"]["viewStreamProvider"],
            "rdp_gateway"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["params"]["controlInputProvider"],
            "manual_attached_desktop"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["params"]["displayIsolation"],
            "private_virtual_display"
        );
    }

    #[test]
    fn service_access_plan_reports_local_policy_override_source() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "google-work".to_string(),
                BrowserProfile {
                    id: "google-work".to_string(),
                    name: "Google Work".to_string(),
                    target_service_ids: vec!["google".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "local-google".to_string(),
                    browser_host: Some(BrowserHost::RemoteHeaded),
                    ..SitePolicy::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["google".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["sitePolicy"]["originPattern"], "local-google");
        assert_eq!(plan["sitePolicySource"]["source"], "persisted_state");
        assert_eq!(plan["sitePolicySource"]["matchedBy"], "target_service_id");
        assert_eq!(plan["sitePolicySource"]["overrideable"], false);
        assert_eq!(plan["decision"]["browserHost"], "remote_headed");
    }

    #[test]
    fn service_access_plan_reports_config_policy_override_source() {
        let mut state = ServiceState {
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "configured-google".to_string(),
                    ..SitePolicy::default()
                },
            )]),
            ..ServiceState::default()
        };
        state.mark_config_entity_sources();

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["google".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["sitePolicy"]["originPattern"], "configured-google");
        assert_eq!(plan["sitePolicySource"]["source"], "config");
        assert_eq!(
            plan["sitePolicySource"]["precedence"],
            json!(["config", "persisted_state", "builtin"])
        );
    }

    #[test]
    fn service_access_plan_scopes_readiness_to_requested_target_identity() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "mixed".to_string(),
                BrowserProfile {
                    id: "mixed".to_string(),
                    name: "Mixed target profile".to_string(),
                    target_service_ids: vec!["acs".to_string(), "google".to_string()],
                    authenticated_service_ids: vec!["acs".to_string()],
                    target_readiness: vec![
                        ProfileTargetReadiness {
                            target_service_id: "google".to_string(),
                            state: ProfileReadinessState::NeedsManualSeeding,
                            manual_seeding_required: true,
                            evidence: "manual_seed_required_without_authenticated_hint"
                                .to_string(),
                            recommended_action:
                                "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
                                    .to_string(),
                            ..ProfileTargetReadiness::default()
                        },
                        ProfileTargetReadiness {
                            target_service_id: "acs".to_string(),
                            state: ProfileReadinessState::Fresh,
                            evidence: "authenticated_hint_present".to_string(),
                            recommended_action: "use_profile".to_string(),
                            ..ProfileTargetReadiness::default()
                        },
                    ],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                service_name: Some("JournalDownloader".to_string()),
                target_service_ids: vec!["acs".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["selectedProfile"]["id"], "mixed");
        assert_eq!(plan["readinessSummary"]["manualSeedingRequired"], false);
        assert_eq!(plan["readinessSummary"]["needsManualSeeding"], false);
        assert_eq!(
            plan["decision"]["recommendedAction"],
            "use_selected_profile"
        );
        assert_eq!(plan["decision"]["manualActionRequired"], false);
        assert!(plan["decision"]["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "selected_profile_has_readiness_evidence"));
    }
}
#[allow(dead_code, unused_imports)]
pub(crate) mod service_commands {
    use crate::native::action_runtime::runtime::{
        account_ids_from_command, apply_service_browser_capability_selection,
        browser_build_from_command, browser_build_label, is_stale_page_session_error,
        launch_profile_from_sources, optional_command_string, recover_browser_command_channel,
        registry_string_field, relaunch_and_restore_page, runtime_profile_from_sources,
        service_browser_id, target_service_ids_from_command, target_url_from_command,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::auth;
    use crate::native::cdp::chrome::{launch_chrome_detached, LaunchOptions, ManualChromeLaunch};
    use crate::native::providers;
    use crate::native::service_access::{service_access_plan_for_state, ServiceAccessPlanRequest};
    use crate::native::service_config::{
        delete_persisted_monitor, delete_persisted_profile, delete_persisted_provider,
        delete_persisted_session, delete_persisted_site_policy, reset_persisted_monitor_failures,
        update_persisted_monitor_state, update_persisted_profile_freshness,
        update_persisted_profile_seeding_handoff,
        upsert_persisted_browser_capability_registry_record, upsert_persisted_monitor,
        upsert_persisted_profile, upsert_persisted_provider, upsert_persisted_session,
        upsert_persisted_site_policy,
    };
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::service_lifecycle::{
        profile_lease_telemetry, select_service_profile_for_request, service_profile_id,
        ProfileSelectionRequest, ServiceLaunchMetadata,
    };
    use crate::native::service_model::{
        retained_display_allocation_candidates, service_profile_allocations,
        service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
        BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
        BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession,
        BrowserTab, ControlInputProvider, DisplayAllocation, JobState as ServiceJobState,
        LeaseState, MonitorState, ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy,
        ProfileLeaseDisposition, ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease,
        RemoteViewHandoff, RemoteViewRoute, RoutePoolEntry, ServiceEntitySource, ServiceEvent,
        ServiceEventKind, ServiceState, ServiceTabHandle, SessionCleanupPolicy, TabLifecycle,
        ViewStream, ViewStreamProvider,
    };
    use crate::native::state;
    use chrono::{DateTime, FixedOffset};
    use serde_json::{json, Map, Value};
    use sha2::{Digest, Sha256};
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
    use std::env;
    pub(crate) fn access_plan_browser_build_selection_summary(plan: &Value) -> Value {
        let selection = plan
            .pointer("/decision/launchPosture/browserBuildSelection")
            .unwrap_or(&Value::Null);
        let profile_compatibility = selection
            .get("profileCompatibility")
            .unwrap_or(&Value::Null);
        let validation_evidence = selection.get("validationEvidence").unwrap_or(&Value::Null);
        let browser_build = optional_command_string(selection, "browserBuild");
        let source = optional_command_string(selection, "source");
        let evidence_source = optional_command_string(selection, "evidenceSource");
        let profile_compatibility_status = optional_command_string(profile_compatibility, "status");
        let validation_evidence_status = optional_command_string(validation_evidence, "status");
        let selected_preference_binding_id =
            optional_command_string(selection, "selectedPreferenceBindingId");
        let mut compact_parts = vec![
            format!("build={}", browser_build.as_deref().unwrap_or("unknown")),
            format!("source={}", source.as_deref().unwrap_or("unknown")),
            format!(
                "evidence={}",
                evidence_source.as_deref().unwrap_or("unknown")
            ),
            format!(
                "override={}",
                if selection
                    .get("operatorOverride")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "yes"
                } else {
                    "no"
                }
            ),
            format!(
                "profileCompatibility={}",
                profile_compatibility_status.as_deref().unwrap_or("unknown")
            ),
            format!(
                "validation={}",
                validation_evidence_status.as_deref().unwrap_or("unknown")
            ),
        ];
        if let Some(binding_id) = selected_preference_binding_id.as_deref() {
            compact_parts.push(format!("preferenceBinding={binding_id}"));
        }
        let mut audit_flags = Vec::new();
        if source.as_deref() == Some("browser_preference_binding") {
            audit_flags.push("preference_binding_selected");
        }
        if selection
            .get("operatorOverride")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            audit_flags.push("operator_override");
        }
        if selection
            .get("requiresCdpFree")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            audit_flags.push("requires_cdp_free");
        }
        if profile_compatibility_status.as_deref() == Some("incompatible_or_mixed") {
            audit_flags.push("profile_compatibility_attention");
        }
        if matches!(
            validation_evidence_status.as_deref(),
            Some("failed_or_mixed" | "missing")
        ) {
            audit_flags.push("validation_evidence_attention");
        }
        let attention_required = audit_flags.iter().any(|flag| flag.ends_with("_attention"));
        json!(
            { "browserBuild" : browser_build, "source" : source, "evidenceSource" :
            evidence_source, "summary" : optional_command_string(selection, "summary"),
            "operatorOverride" : selection.get("operatorOverride")
            .and_then(Value::as_bool).unwrap_or(false), "requiresCdpFree" : selection
            .get("requiresCdpFree").and_then(Value::as_bool).unwrap_or(false),
            "selectedProfileId" : optional_command_string(selection,
            "selectedProfileId"), "selectedProfileBrowserBuild" :
            optional_command_string(selection, "selectedProfileBrowserBuild"),
            "selectedPreferenceBindingId" : selected_preference_binding_id,
            "selectedPreferenceBindingReason" : optional_command_string(selection,
            "selectedPreferenceBindingReason"), "profileCompatibilityStatus" :
            profile_compatibility_status, "profileCompatibilityReason" :
            optional_command_string(profile_compatibility, "reason"),
            "profileCompatibilityIds" : string_array_field(profile_compatibility,
            "matchingIds"), "validationEvidenceStatus" : validation_evidence_status,
            "validationEvidenceReason" : optional_command_string(validation_evidence,
            "reason"), "validationEvidenceIds" : string_array_field(validation_evidence,
            "matchingIds"), "auditFlags" : audit_flags, "attentionRequired" :
            attention_required, "compact" : compact_parts.join(" "), }
        )
    }
    pub(crate) fn string_array_field(value: &Value, key: &str) -> Vec<String> {
        value
            .get(key)
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }
    /// Evaluate browser capability launch gates without starting Chrome.
    pub(crate) async fn handle_service_browser_capability_preflight(
        cmd: &Value,
    ) -> Result<Value, String> {
        let requested_build = browser_build_from_command(cmd);
        let cdp_free = cmd
            .get("requiresCdpFree")
            .or_else(|| cmd.get("cdpFree"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || requested_build == Some(BrowserBuild::CdpFreeHeaded)
            || cmd
                .get("cdpAttachmentAllowed")
                .and_then(Value::as_bool)
                .is_some_and(|allowed| !allowed);
        let headless = if cdp_free {
            false
        } else {
            cmd.get("headless").and_then(Value::as_bool).unwrap_or(true)
        };
        let mut launch_options = LaunchOptions {
            headless,
            executable_path: cmd
                .get("executablePath")
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .or_else(|| env::var("AGENT_BROWSER_EXECUTABLE_PATH").ok()),
            profile: launch_profile_from_sources(cmd, true),
            runtime_profile: runtime_profile_from_sources(cmd, true),
            manual_login: cdp_free,
            attachable: !cdp_free,
            ..LaunchOptions::default()
        };
        let browser_capability_launch =
            apply_service_browser_capability_selection(&mut launch_options, cmd);
        Ok(json!(
            { "preflight" : true, "wouldLaunch" : false, "wouldApplyExecutable" :
            browser_capability_launch.applied, "browserCapabilityLaunch" :
            browser_capability_launch.to_value(), "request" : { "browserBuild" :
            requested_build.map(browser_build_label), "profileId" :
            service_profile_id(launch_options.profile.as_deref(), launch_options
            .runtime_profile.as_deref()), "headless" : launch_options.headless,
            "cdpFree" : cdp_free, "serviceName" : optional_command_string(cmd,
            "serviceName"), "agentName" : optional_command_string(cmd, "agentName"),
            "taskName" : optional_command_string(cmd, "taskName"), "targetServiceIds"
            : target_service_ids_from_command(cmd), "accountIds" :
            account_ids_from_command(cmd), "url" : target_url_from_command(cmd), },
            "selectedExecutablePath" : launch_options.executable_path, }
        ))
    }
    /// Generate operator-facing commands for binding a site/account to a known browser executable.
    pub(crate) async fn handle_service_browser_capability_preference_guide(
        cmd: &Value,
    ) -> Result<Value, String> {
        let service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        Ok(browser_capability_preference_guide(&service_state, cmd))
    }
    pub(crate) fn browser_capability_preference_guide(
        service_state: &ServiceState,
        cmd: &Value,
    ) -> Value {
        let registry = &service_state.browser_capability_registry;
        let requested_build = optional_command_string(cmd, "browserBuild");
        let target_service_ids = target_service_ids_from_command(cmd);
        let account_ids = account_ids_from_command(cmd);
        let service_name = optional_command_string(cmd, "serviceName");
        let task_name = optional_command_string(cmd, "taskName");
        let reason = optional_command_string(cmd, "reason")
            .unwrap_or_else(|| "operator_primary_browser_preference".to_string());
        let has_filter = !target_service_ids.is_empty()
            || !account_ids.is_empty()
            || service_name.is_some()
            || task_name.is_some();
        let mut suggestions = registry
            .browser_executables
            .iter()
            .filter(|executable| {
                requested_build.as_deref().is_none_or(|build| {
                    registry_string_field(executable, "buildLabel").as_deref() == Some(build)
                })
            })
            .filter_map(|executable| {
                let executable_id = registry_string_field(executable, "id")?;
                let browser_build = registry_string_field(executable, "buildLabel")
                    .or_else(|| requested_build.clone())
                    .unwrap_or_else(|| "stock_chrome".to_string());
                let host_id = registry_string_field(executable, "hostId");
                let capability_id =
                    matching_capability_id(registry, host_id.as_deref(), &executable_id);
                let command = browser_preference_command(BrowserPreferenceCommandInput {
                    browser_build: &browser_build,
                    executable_id: &executable_id,
                    host_id: host_id.as_deref(),
                    capability_id: capability_id.as_deref(),
                    target_service_ids: &target_service_ids,
                    account_ids: &account_ids,
                    service_name: service_name.as_deref(),
                    task_name: task_name.as_deref(),
                    reason: &reason,
                });
                let existing_binding_ids = registry
                    .browser_preference_bindings
                    .iter()
                    .filter(|binding| {
                        registry_string_field(binding, "preferredExecutableId").as_deref()
                            == Some(executable_id.as_str())
                    })
                    .filter_map(|binding| registry_string_field(binding, "id"))
                    .collect::<Vec<_>>();
                Some(json!(
                    { "executableId" : executable_id, "browserBuild" : browser_build,
                    "hostId" : host_id, "capabilityId" : capability_id, "source" :
                    registry_string_field(executable, "source"), "executablePath" :
                    registry_string_field(executable, "executablePath"), "fresh" :
                    executable.get("fresh").cloned().unwrap_or(Value::Null), "tags" :
                    executable.get("tags").cloned().unwrap_or_else(|| json!([])),
                    "existingBindingIds" : existing_binding_ids, "copyable" :
                    has_filter, "command" : command, }
                ))
            })
            .collect::<Vec<_>>();
        suggestions.sort_by(|left, right| {
            json_string_field(left, "browserBuild")
                .cmp(&json_string_field(right, "browserBuild"))
                .then_with(|| {
                    json_string_field(left, "executableId")
                        .cmp(&json_string_field(right, "executableId"))
                })
        });
        json!(
            { "guide" : true, "advisory" : true, "copyable" : has_filter, "requested" : {
            "browserBuild" : requested_build, "targetServiceIds" : target_service_ids,
            "accountIds" : account_ids, "serviceName" : service_name, "taskName" :
            task_name, "reason" : reason, }, "counts" : { "browserExecutables" : registry
            .browser_executables.len(), "matchingExecutables" : suggestions.len(),
            "browserPreferenceBindings" : registry.browser_preference_bindings.len(), },
            "suggestions" : suggestions, "recommendedNextStep" : if has_filter {
            "Copy the preferred command, run it, then run service browser-capability preflight for the same site/account before requesting browser work."
            } else {
            "Rerun with --target-service-id and --account-id to produce exact copyable prefer commands."
            }, }
        )
    }
    pub(crate) fn json_string_field(value: &Value, field: &str) -> String {
        value
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }
    pub(crate) fn matching_capability_id(
        registry: &BrowserCapabilityRegistry,
        host_id: Option<&str>,
        executable_id: &str,
    ) -> Option<String> {
        registry.browser_capabilities.iter().find_map(|capability| {
            let executable_matches =
                registry_string_field(capability, "executableId").as_deref() == Some(executable_id);
            let host_matches = host_id.is_none_or(|host_id| {
                registry_string_field(capability, "hostId").as_deref() == Some(host_id)
            });
            (executable_matches && host_matches).then(|| registry_string_field(capability, "id"))?
        })
    }
    pub(crate) struct BrowserPreferenceCommandInput<'a> {
        pub(crate) browser_build: &'a str,
        pub(crate) executable_id: &'a str,
        pub(crate) host_id: Option<&'a str>,
        pub(crate) capability_id: Option<&'a str>,
        pub(crate) target_service_ids: &'a [String],
        pub(crate) account_ids: &'a [String],
        pub(crate) service_name: Option<&'a str>,
        pub(crate) task_name: Option<&'a str>,
        pub(crate) reason: &'a str,
    }
    pub(crate) fn browser_preference_command(input: BrowserPreferenceCommandInput<'_>) -> String {
        let mut args = vec![
            "agent-browser".to_string(),
            "service".to_string(),
            "browser-capability".to_string(),
            "prefer".to_string(),
            "--browser-build".to_string(),
            input.browser_build.to_string(),
            "--preferred-executable-id".to_string(),
            input.executable_id.to_string(),
        ];
        if let Some(host_id) = input.host_id {
            args.push("--preferred-host-id".to_string());
            args.push(host_id.to_string());
        }
        if let Some(capability_id) = input.capability_id {
            args.push("--preferred-capability-id".to_string());
            args.push(capability_id.to_string());
        }
        if input.target_service_ids.is_empty()
            && input.account_ids.is_empty()
            && input.service_name.is_none()
            && input.task_name.is_none()
        {
            args.push("--target-service-id".to_string());
            args.push("<site>".to_string());
            args.push("--account-id".to_string());
            args.push("<account>".to_string());
        } else {
            for target in input.target_service_ids {
                args.push("--target-service-id".to_string());
                args.push(target.clone());
            }
            for account in input.account_ids {
                args.push("--account-id".to_string());
                args.push(account.clone());
            }
            if let Some(service_name) = input.service_name {
                args.push("--service-name".to_string());
                args.push(service_name.to_string());
            }
            if let Some(task_name) = input.task_name {
                args.push("--task-name".to_string());
                args.push(task_name.to_string());
            }
        }
        args.push("--reason".to_string());
        args.push(input.reason.to_string());
        args.into_iter()
            .map(|arg| shell_quote_command_arg(&arg))
            .collect::<Vec<_>>()
            .join(" ")
    }
    pub(crate) fn shell_quote_command_arg(arg: &str) -> String {
        if arg.starts_with('<') && arg.ends_with('>') {
            return arg.to_string();
        }
        if arg
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':' | '='))
        {
            arg.to_string()
        } else {
            format!("'{}'", arg.replace('\'', "'\\''"))
        }
    }
    /// Return the service-owned profile collection without the full status payload.
    pub(crate) async fn handle_service_profiles(cmd: &Value) -> Result<Value, String> {
        let mut service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        service_state.refresh_profile_readiness();
        let profile_allocations = service_profile_allocations(&service_state);
        let profile_sources = service_profile_sources(&service_state);
        let mut profiles = service_state.profiles.into_values().collect::<Vec<_>>();
        profiles.sort_by(|left, right| left.id.cmp(&right.id));
        let count = profiles.len();
        Ok(json!(
            { "profiles" : profiles, "profileSources" : profile_sources,
            "profileAllocations" : profile_allocations, "count" : count, }
        ))
    }
    pub(crate) async fn handle_service_browser_capability_registry_upsert(
        cmd: &Value,
    ) -> Result<Value, String> {
        let collection = required_service_config_id(cmd, "collection")?;
        let record_id = required_service_config_id(cmd, "recordId")?;
        let body = cmd.get("record").cloned().ok_or("Missing record")?;
        let (record, registry, counts) =
            upsert_persisted_browser_capability_registry_record(collection, record_id, body)?;
        Ok(json!(
            { "id" : record_id, "collection" : collection, "record" : record,
            "browserCapabilityRegistry" : registry, "counts" : counts, "upserted" :
            true, "advisory" : true, "routingApplied" : false, }
        ))
    }
    pub(crate) fn required_service_config_id<'a>(
        cmd: &'a Value,
        field: &str,
    ) -> Result<&'a str, String> {
        cmd.get(field)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("Missing {field}"))
    }
}
pub(crate) use service_commands::*;

#[cfg(any())]
#[path = "service_access/plan_0134_repro_tests.rs"]
mod plan_0134_repro_tests;

#[cfg(any())]
#[path = "service_access/plan_0137_repro_tests.rs"]
mod plan_0137_repro_tests;
