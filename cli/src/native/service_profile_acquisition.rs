//! Typed profile-acquisition decision shared by planning and execution.
//!
//! Public access-plan JSON is a projection of this decision, not an interface
//! that execution callers must parse. This module starts the behavior-preserving
//! extraction by giving callers one typed representation of the selected
//! browser and daemon route. The owner also contains recovery coordination so
//! access planning, retry, and execution share one dependency direction.
//! Revisioned profile-access policy is evaluated here before coordination and
//! lifecycle evidence are considered.

use chrono::Utc;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use super::service_access::ServiceAccessPlanRequest;
use super::service_contracts::SERVICE_REQUEST_ACTIONS;
use super::service_lease_authority::{ActiveLeaseClaim, LeaseResourceKey};
use super::service_model::{
    BrowserHealth, BrowserHost, BrowserProcess, BrowserProfile, BrowserSession,
    ControlInputProvider, LeaseState, ProfileOrigin, ServiceState, ViewStreamProvider,
};
use super::service_principal::AuthenticatedServicePrincipal;
use super::service_profile_access_policy::{
    evaluate_profile_access, ProfileAccessEvaluation, ProfileAccessMode, ProfileChildAccess,
    ProfileIdentityAssurance, ProfilePermission, ServiceProfileAccessDecision,
    ServiceProfileAccessPolicy,
};
use super::{
    action_runtime, service_lease_authority, service_model, service_principal, service_resources,
    service_store, service_trace,
};

#[path = "service_profile_recovery.rs"]
mod recovery;

pub(crate) use recovery::*;

/// Inputs required to choose one profile owner and executable browser route.
pub(crate) struct ProfileAcquisitionInput<'a> {
    pub(crate) request: &'a ServiceAccessPlanRequest,
    pub(crate) selected_profile: Option<&'a BrowserProfile>,
    pub(crate) service_state: &'a ServiceState,
    pub(crate) denied: bool,
    pub(crate) manual_seeding_required: bool,
    pub(crate) manual_action_required: bool,
    pub(crate) launch_posture: &'a Value,
    pub(crate) one_time_profile_recommendation: &'a Value,
    pub(crate) authenticated_principal: Option<&'a AuthenticatedServicePrincipal>,
}

/// Typed acquisition result plus its stable public compatibility projections.
pub(crate) struct ProfileAcquisitionPlan {
    pub(crate) access_policy: ServiceProfileAccessPolicy,
    pub(crate) access_decision: ServiceProfileAccessDecision,
    pub(crate) profile_reuse: Value,
    pub(crate) lifecycle_replacement: Value,
    pub(crate) service_request: Value,
    pub(crate) decision: ProfileAcquisitionDecision,
}

/// Select the profile owner, dominant blocker, and executable daemon route once.
pub(crate) fn decide_profile_acquisition(
    input: ProfileAcquisitionInput<'_>,
) -> ProfileAcquisitionPlan {
    let lifecycle_replacement =
        lifecycle_replacement_decision(input.selected_profile, input.service_state);
    let (access_policy, access_decision) = profile_access_decision(&input);
    let strict_identity_required = access_policy.mode != ProfileAccessMode::SharedLocal;
    let profile_reuse = profile_reuse_decision(ProfileReuseInput {
        request: input.request,
        selected_profile: input.selected_profile,
        service_state: input.service_state,
        launch_posture: input.launch_posture,
        manual_seeding_required: input.manual_seeding_required,
        lifecycle_replacement: &lifecycle_replacement,
        authenticated_principal: input.authenticated_principal,
        strict_identity_required,
    });
    let reuse_action = profile_reuse
        .get("recommendedAction")
        .and_then(Value::as_str);
    let terminal_replacement_requires_capability = input.authenticated_principal.is_none()
        && reuse_action == Some("launch_new_browser")
        && lifecycle_replacement["replacementEligible"].as_bool() == Some(true)
        && lifecycle_replacement["reason"].as_str() == Some("terminal_cleanup_satisfied");
    let acquisition_blocker = if !access_decision.allowed {
        Some("profile_access_denied")
    } else {
        match reuse_action {
            Some("blocked_by_explicit_session_route") => Some("explicit_session_route_invalid"),
            Some("wait_for_foreign_principal") => Some("foreign_principal_profile_lease"),
            Some("authenticate_for_profile_reuse") => Some("profile_capability_required"),
            Some("lifecycle_profile_identity_inconsistent") => {
                Some("lifecycle_profile_identity_inconsistent")
            }
            _ if terminal_replacement_requires_capability => Some("profile_capability_required"),
            _ => None,
        }
    };
    let service_request = service_request_decision(ServiceRequestDecisionInput {
        request: input.request,
        selected_profile: input.selected_profile,
        denied: input.denied,
        manual_seeding_required: input.manual_seeding_required,
        manual_action_required: input.manual_action_required,
        launch_posture: input.launch_posture,
        profile_reuse: &profile_reuse,
        lifecycle_replacement: &lifecycle_replacement,
        one_time_profile_recommendation: input.one_time_profile_recommendation,
        acquisition_blocker,
        authenticated_principal: input.authenticated_principal,
        access_policy: &access_policy,
        access_decision: &access_decision,
    });
    let decision = ProfileAcquisitionDecision::from_components(
        input.selected_profile,
        &access_policy,
        &access_decision,
        &profile_reuse,
        &lifecycle_replacement,
        &service_request,
    )
    .expect("profile acquisition owner must produce a coherent decision");

    ProfileAcquisitionPlan {
        access_policy,
        access_decision,
        profile_reuse,
        lifecycle_replacement,
        service_request,
        decision,
    }
}

/// Describe the queued browser-control handoff clients should use after planning.
struct ServiceRequestDecisionInput<'a> {
    request: &'a ServiceAccessPlanRequest,
    selected_profile: Option<&'a BrowserProfile>,
    denied: bool,
    manual_seeding_required: bool,
    manual_action_required: bool,
    launch_posture: &'a Value,
    profile_reuse: &'a Value,
    lifecycle_replacement: &'a Value,
    one_time_profile_recommendation: &'a Value,
    acquisition_blocker: Option<&'a str>,
    authenticated_principal: Option<&'a AuthenticatedServicePrincipal>,
    access_policy: &'a ServiceProfileAccessPolicy,
    access_decision: &'a ServiceProfileAccessDecision,
}

fn profile_access_decision(
    input: &ProfileAcquisitionInput<'_>,
) -> (ServiceProfileAccessPolicy, ServiceProfileAccessDecision) {
    let profile_id = input
        .selected_profile
        .map(|profile| profile.id.as_str())
        .or(input.request.runtime_profile.as_deref())
        .unwrap_or("unselected");
    let subject_id = input
        .authenticated_principal
        .map(|authority| authority.principal_id.clone())
        .or_else(|| input.request.client_subject_id.clone())
        .or_else(|| self_declared_subject(input.request));
    let assurance = if input.authenticated_principal.is_some() {
        ProfileIdentityAssurance::RegisteredCapability
    } else if subject_id.is_some() {
        // Identity assurance is derived from trusted ingress state. A caller may
        // describe its own subject, but it cannot promote that assertion by
        // supplying a stronger assurance label in the request.
        ProfileIdentityAssurance::SelfDeclared
    } else {
        ProfileIdentityAssurance::Unknown
    };
    let incompatible_occupancy = input
        .service_state
        .sessions
        .iter()
        .filter(|(_id, session)| {
            session_blocks_profile_reuse(session, profile_id)
                && session.principal_id.as_deref() != subject_id.as_deref()
        })
        .map(|(id, _session)| id.clone())
        .collect();
    evaluate_profile_access(ProfileAccessEvaluation {
        profile_id,
        explicit_policy: input
            .selected_profile
            .and_then(|profile| profile.access_policy.as_ref()),
        subject_id,
        assurance,
        connection_instance_id: None,
        permission: ProfilePermission::TabCreate,
        operation: "tab_create",
        incompatible_occupancy,
    })
}

fn self_declared_subject(request: &ServiceAccessPlanRequest) -> Option<String> {
    let parts = [
        request
            .service_name
            .as_deref()
            .map(|value| format!("service:{value}")),
        request
            .agent_name
            .as_deref()
            .map(|value| format!("agent:{value}")),
        request
            .task_name
            .as_deref()
            .map(|value| format!("task:{value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Return the stable daemon route for a registered principal's first browser.
///
/// Cold authenticated requests must not inherit the transport's ambient
/// `default` session because that route can retain another profile's identity.
/// The route uses only public principal/profile identity and never includes
/// raw capability material.
pub(crate) fn authenticated_cold_session_name(
    authority: &AuthenticatedServicePrincipal,
    selected_profile: &BrowserProfile,
) -> Option<String> {
    if authority.profile_id != selected_profile.id {
        return None;
    }
    let mut hasher = Sha256::new();
    hasher.update(authority.principal_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(selected_profile.id.as_bytes());
    let suffix = hasher
        .finalize()
        .iter()
        .take(12)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Some(format!("principal-profile-{suffix}"))
}

/// Return a fresh daemon route for replacing one exact terminal owner.
///
/// The retained daemon route remains lifecycle evidence for the owner being
/// superseded. Reusing it for the next launch would make the executor classify
/// the request as existing-session work before the replacement can register
/// its new owner generation.
pub(crate) fn terminal_replacement_launch_session_name(
    selected_profile: &BrowserProfile,
    lifecycle_replacement: &Value,
) -> Option<String> {
    if lifecycle_replacement
        .get("replacementEligible")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return None;
    }
    let logical_browser_id = lifecycle_replacement
        .get("logicalBrowserId")
        .and_then(Value::as_str)?;
    let owner_generation = lifecycle_replacement
        .get("ownerGeneration")
        .and_then(Value::as_u64)?;
    let mut hasher = Sha256::new();
    hasher.update(selected_profile.id.as_bytes());
    hasher.update(b"\0");
    hasher.update(logical_browser_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(owner_generation.to_le_bytes());
    let suffix = hasher
        .finalize()
        .iter()
        .take(12)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Some(format!("terminal-profile-{suffix}"))
}

fn service_request_decision(input: ServiceRequestDecisionInput<'_>) -> Value {
    let request = input.request;
    let selected_profile = input.selected_profile;
    let launch_posture = input.launch_posture;
    let profile_reuse = input.profile_reuse;
    let lifecycle_replacement = input.lifecycle_replacement;
    let one_time_profile_recommendation = input.one_time_profile_recommendation;
    let selected_profile_id = selected_profile.map(|profile| profile.id.clone());
    let recommended_runtime_profile = one_time_profile_recommendation
        .get("runtimeProfile")
        .and_then(Value::as_str)
        .map(str::to_string);
    let effective_runtime_profile = selected_profile_id
        .clone()
        .or_else(|| request.runtime_profile.clone())
        .or(recommended_runtime_profile);
    let effective_profile_class = selected_profile
        .map(|profile| json!(profile.profile_class))
        .or_else(|| one_time_profile_recommendation.get("profileClass").cloned())
        .or_else(|| {
            request
                .runtime_profile
                .as_ref()
                .map(|_| json!("operator_supplied"))
        });
    let requires_cdp_free = launch_posture
        .get("requiresCdpFree")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let cdp_attachment_allowed = launch_posture
        .get("cdpAttachmentAllowed")
        .and_then(Value::as_bool)
        .unwrap_or(!requires_cdp_free);
    let blocked_by_cdp_free = requires_cdp_free && !cdp_attachment_allowed;
    let has_profile_lane = effective_runtime_profile.is_some();
    let available = has_profile_lane
        && !input.denied
        && !input.manual_action_required
        && !blocked_by_cdp_free
        && input.acquisition_blocker.is_none();
    let recommended_after_manual_action =
        has_profile_lane && !input.denied && input.manual_action_required && !blocked_by_cdp_free;
    let mut service_request = Map::new();
    service_request.insert("action".to_string(), json!("tab_new"));
    if let Some(service_name) = request.service_name.as_ref() {
        service_request.insert("serviceName".to_string(), json!(service_name));
    }
    if let Some(agent_name) = request.agent_name.as_ref() {
        service_request.insert("agentName".to_string(), json!(agent_name));
    }
    if let Some(task_name) = request.task_name.as_ref() {
        service_request.insert("taskName".to_string(), json!(task_name));
    }
    if let Some(subject_id) = input.access_decision.subject.subject_id.as_ref() {
        service_request.insert("clientSubjectId".to_string(), json!(subject_id));
    }
    service_request.insert(
        "identityAssurance".to_string(),
        json!(input.access_decision.subject.assurance),
    );
    service_request.insert(
        "policyRevision".to_string(),
        json!(input.access_policy.revision),
    );
    service_request.insert(
        "accessDecisionId".to_string(),
        json!(input.access_decision.decision_id),
    );
    let replacement_session_name = (profile_reuse
        .get("recommendedAction")
        .and_then(Value::as_str)
        == Some("launch_new_browser")
        && lifecycle_replacement
            .get("replacementEligible")
            .and_then(Value::as_bool)
            == Some(true))
    .then(|| {
        lifecycle_replacement
            .get("replacementSessionName")
            .and_then(Value::as_str)
    })
    .flatten();
    let authenticated_cold_session_name = (available
        && profile_reuse
            .get("recommendedAction")
            .and_then(Value::as_str)
            == Some("launch_new_browser"))
    .then(|| {
        input
            .authenticated_principal
            .zip(selected_profile)
            .and_then(|(authority, profile)| authenticated_cold_session_name(authority, profile))
    })
    .flatten();
    let terminal_replacement_launch_session_name = (available
        && profile_reuse
            .get("recommendedAction")
            .and_then(Value::as_str)
            == Some("launch_new_browser"))
    .then(|| {
        selected_profile.and_then(|profile| {
            terminal_replacement_launch_session_name(profile, lifecycle_replacement)
        })
    })
    .flatten();
    if let Some(session_name) = request
        .session_name
        .as_deref()
        .or(terminal_replacement_launch_session_name.as_deref())
        .or(authenticated_cold_session_name.as_deref())
        .or(replacement_session_name)
    {
        service_request.insert("sessionName".to_string(), json!(session_name));
    }
    if !request.target_service_ids.is_empty() {
        service_request.insert(
            "targetServiceIds".to_string(),
            json!(request.target_service_ids),
        );
    }
    if !request.account_ids.is_empty() {
        service_request.insert("accountIds".to_string(), json!(request.account_ids));
    }
    if let Some(target_url) = request.target_url.as_ref() {
        service_request.insert("url".to_string(), json!(target_url));
    }
    if let Some(browser_build) = launch_posture.get("browserBuild") {
        service_request.insert("browserBuild".to_string(), browser_build.clone());
    }
    if let Some(runtime_profile) = effective_runtime_profile.as_deref() {
        service_request.insert("runtimeProfile".to_string(), json!(runtime_profile));
    }
    if let Some(profile_class) = effective_profile_class.clone() {
        service_request.insert("profileClass".to_string(), profile_class);
    }
    if let Some(selected_profile) = selected_profile {
        if let Some(user_data_dir) = selected_profile
            .user_data_dir
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            service_request.insert("profile".to_string(), json!(user_data_dir));
        }
    }
    if profile_reuse
        .get("recommendedAction")
        .and_then(Value::as_str)
        == Some("reuse_existing_browser")
    {
        if let Some(browser_id) = profile_reuse
            .get("reusableBrowserId")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            service_request.insert("browserId".to_string(), json!(browser_id));
        }
        if let Some(session_name) = profile_reuse
            .get("reusableSessionName")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            service_request.insert("sessionName".to_string(), json!(session_name));
        }
    }
    if input.manual_action_required {
        service_request.insert("blockedByManualAction".to_string(), json!(true));
    }
    if input.manual_seeding_required {
        service_request.insert("manualSeedingRequired".to_string(), json!(true));
    }
    if requires_cdp_free {
        service_request.insert("requiresCdpFree".to_string(), json!(true));
    }
    let headed = launch_posture
        .get("headed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let browser_host = launch_posture.get("browserHost").cloned();
    let view_stream_provider = launch_posture.get("viewStreamProvider").cloned();
    let control_input_provider = launch_posture.get("controlInputProvider").cloned();
    let display_isolation = launch_posture
        .get("displayIsolation")
        .and_then(Value::as_str)
        .map(|value| json!(value));
    let mut request_params = Map::new();
    if headed {
        request_params.insert("headless".to_string(), json!(false));
    }
    if let Some(browser_host) = browser_host {
        request_params.insert("browserHost".to_string(), browser_host);
    }
    if let Some(view_stream_provider) = view_stream_provider {
        request_params.insert("viewStreamProvider".to_string(), view_stream_provider);
    }
    if let Some(control_input_provider) = control_input_provider {
        request_params.insert("controlInputProvider".to_string(), control_input_provider);
    }
    if let Some(display_isolation) = display_isolation {
        request_params.insert("displayIsolation".to_string(), display_isolation);
    }
    if !request_params.is_empty() {
        service_request.insert("params".to_string(), Value::Object(request_params));
    }
    service_request.insert(
        "cdpAttachmentAllowed".to_string(),
        json!(cdp_attachment_allowed),
    );
    service_request.insert("profileLeasePolicy".to_string(), json!("wait"));

    json!({
        "available": available,
        "recommendedAfterManualAction": recommended_after_manual_action,
        "blockedByManualAction": input.manual_action_required,
        "blockedByCdpFree": blocked_by_cdp_free,
        "blockedByPolicy": input.denied,
        "blockedByAcquisition": input.acquisition_blocker.is_some(),
        "blockedByLifecycleOwner": input.acquisition_blocker == Some("lifecycle_owner_blocks_replacement"),
        "acquisitionBlocker": input.acquisition_blocker,
        "requiresCdpFree": requires_cdp_free,
        "cdpAttachmentAllowed": cdp_attachment_allowed,
        "action": "tab_new",
        "selectedProfileId": selected_profile_id,
        "runtimeProfile": effective_runtime_profile,
        "profileClass": effective_profile_class,
        "profileLeasePolicy": "wait",
        "oneTimeProfileRecommendation": one_time_profile_recommendation,
        "cdpFreeAvailability": cdp_free_command_availability(blocked_by_cdp_free),
        "request": if input.acquisition_blocker.is_some() {
            Value::Null
        } else {
            Value::Object(service_request)
        },
        "http": {
            "method": "POST",
            "route": "/api/service/request",
        },
        "mcp": {
            "tool": "service_request",
        },
        "client": {
            "package": "@agent-browser/client/service-request",
            "helper": "requestServiceTab",
        },
        "requestFields": [
            "serviceName",
            "agentName",
            "taskName",
            "clientSubjectId",
            "identityAssurance",
            "policyRevision",
            "accessDecisionId",
            "targetServiceIds",
            "accountIds",
            "browserBuild",
            "runtimeProfile",
            "browserId",
            "sessionName",
            "profile",
            "displayIsolation",
            "profileLeasePolicy",
            "requiresCdpFree",
            "cdpAttachmentAllowed",
            "url",
            "params",
        ],
    })
}

/// No-launch command availability for clients preparing CDP-free lifecycle-only work.
fn cdp_free_command_availability(applies: bool) -> Value {
    let unsupported_commands: Vec<&str> = if applies {
        SERVICE_REQUEST_ACTIONS
            .iter()
            .copied()
            .filter(|action| *action != "cdp_free_launch")
            .collect()
    } else {
        Vec::new()
    };
    let available_commands: Vec<&str> = if applies {
        vec!["cdp_free_launch"]
    } else {
        Vec::new()
    };

    json!({
        "applies": applies,
        "controlPlaneMode": "cdp_free",
        "lifecycleOnly": applies,
        "cdpAttachmentAllowed": !applies,
        "supportedOperations": if applies {
            vec!["process_lifecycle", "profile_lease", "service_state"]
        } else {
            Vec::<&str>::new()
        },
        "unsupportedOperations": if applies {
            vec!["cdp_commands", "snapshot", "screenshot", "dom_interaction"]
        } else {
            Vec::<&str>::new()
        },
        "unsupportedCommands": unsupported_commands,
        "availableCommands": available_commands,
        "hasUnsupportedCommandList": applies,
        "client": {
            "package": "@agent-browser/client/service-request",
            "summaryHelper": "summarizeServiceCdpFreeLaunchAvailability",
            "predicateHelper": "isServiceCdpFreeActionAvailable",
        },
    })
}

struct ProfileReuseInput<'a> {
    request: &'a ServiceAccessPlanRequest,
    selected_profile: Option<&'a BrowserProfile>,
    service_state: &'a ServiceState,
    launch_posture: &'a Value,
    manual_seeding_required: bool,
    lifecycle_replacement: &'a Value,
    authenticated_principal: Option<&'a AuthenticatedServicePrincipal>,
    strict_identity_required: bool,
}

fn profile_reuse_decision(input: ProfileReuseInput<'_>) -> Value {
    let request = input.request;
    let selected_profile = input.selected_profile;
    let service_state = input.service_state;
    let launch_posture = input.launch_posture;
    let manual_seeding_required = input.manual_seeding_required;
    let lifecycle_replacement = input.lifecycle_replacement;
    let authenticated_principal = input.authenticated_principal;
    let strict_identity_required = input.strict_identity_required;
    let Some(profile) = selected_profile else {
        return json!({
            "recommendedAction": "register_or_select_profile",
            "selectedProfileId": null,
            "reusableBrowserId": null,
            "reusableSessionName": null,
            "reusableBrowserIds": [],
            "compatibleLiveBrowserCount": 0,
            "sameProfileLiveBrowserCount": 0,
            "sameProfileLiveBrowserIds": [],
            "activeLeaseSessionIds": [],
            "activeLeaseCount": 0,
            "duplicatePressure": false,
            "profileLeasePolicy": "wait",
            "reasons": ["no_selected_profile"],
        });
    };
    let terminal_replacement_session_name = lifecycle_replacement
        .get("replacementSessionName")
        .and_then(Value::as_str);
    let terminal_replacement_launch_session_name =
        terminal_replacement_launch_session_name(profile, lifecycle_replacement);

    if profile.profile_origin == ProfileOrigin::ExternalObserved {
        let mut same_profile_live_browser_ids = service_state
            .browsers
            .iter()
            .filter(|(_id, browser)| {
                browser.profile_id.as_deref() == Some(profile.id.as_str())
                    && browser_has_live_health(browser)
            })
            .map(|(id, _browser)| id.clone())
            .collect::<Vec<_>>();
        same_profile_live_browser_ids.sort();
        same_profile_live_browser_ids.dedup();

        return json!({
            "recommendedAction": "launch_new_browser",
            "selectedProfileId": profile.id,
            "reusableBrowserId": null,
            "reusableSessionName": null,
            "reusableBrowserIds": [],
            "compatibleLiveBrowserCount": 0,
            "sameProfileLiveBrowserCount": same_profile_live_browser_ids.len(),
            "sameProfileLiveBrowserIds": same_profile_live_browser_ids,
            "activeLeaseSessionIds": [],
            "activeLeaseCount": 0,
            "duplicatePressure": false,
            "profileLeasePolicy": "wait",
            "reasons": ["external_observed_not_reusable"],
        });
    }

    let observed_at = Utc::now().to_rfc3339();
    let active_claim = service_state
        .lease_authority()
        .current_claim(&LeaseResourceKey::profile(&profile.id), &observed_at);
    let active_claim_requires_authentication =
        active_claim.is_some() && authenticated_principal.is_none();
    let active_claim_is_foreign = active_claim
        .zip(authenticated_principal)
        .is_some_and(|(claim, authority)| claim.principal_id() != authority.principal_id);

    let browser_host = launch_posture
        .get("browserHost")
        .and_then(|value| serde_json::from_value::<BrowserHost>(value.clone()).ok());
    let view_stream_provider: Option<ViewStreamProvider> = launch_posture
        .get("viewStreamProvider")
        .and_then(|value| serde_json::from_value(value.clone()).ok());
    let control_input_provider: Option<ControlInputProvider> = launch_posture
        .get("controlInputProvider")
        .and_then(|value| serde_json::from_value(value.clone()).ok());
    let display_isolation = launch_posture
        .get("displayIsolation")
        .and_then(Value::as_str);
    // Launch posture defaults describe how to create a replacement browser. They
    // must not make an already-running browser ineligible for tab acquisition.
    // Only caller-supplied constraints narrow reuse of an existing owner.
    let reusable_browser_host = request.browser_host;
    let reusable_view_stream_provider = request.view_stream_provider;
    let reusable_control_input_provider = request.control_input_provider;
    let reusable_display_isolation = request.display_isolation.as_deref();

    let mut reusable_browser_ids = service_state
        .browsers
        .iter()
        .filter(|(_id, browser)| {
            browser.profile_id.as_deref() == Some(profile.id.as_str())
                && browser_is_reusable_for_posture(
                    browser,
                    reusable_browser_host,
                    reusable_view_stream_provider,
                    reusable_control_input_provider,
                    reusable_display_isolation,
                )
        })
        .map(|(id, _browser)| id.clone())
        .collect::<Vec<_>>();
    reusable_browser_ids.sort();
    reusable_browser_ids.dedup();

    let mut foreign_principal_session_ids = Vec::new();
    let mut principal_bound_session_ids = service_state
        .sessions
        .iter()
        .filter(|(_id, session)| {
            session_blocks_profile_reuse(session, &profile.id) && session.principal_id.is_some()
        })
        .map(|(id, _session)| id.clone())
        .collect::<Vec<_>>();
    principal_bound_session_ids.sort();
    principal_bound_session_ids.dedup();
    let mut same_principal_profile_mismatch_browser_ids = Vec::new();
    let mut capability_profile_mismatch = false;
    if strict_identity_required {
        if let Some(authority) = authenticated_principal {
            foreign_principal_session_ids = service_state
                .sessions
                .iter()
                .filter(|(_id, session)| {
                    session_blocks_profile_reuse(session, &profile.id)
                        && session
                            .principal_id
                            .as_deref()
                            .is_some_and(|principal_id| principal_id != authority.principal_id)
                })
                .map(|(id, _session)| id.clone())
                .collect();
            foreign_principal_session_ids.sort();
            foreign_principal_session_ids.dedup();
            same_principal_profile_mismatch_browser_ids = service_state
                .sessions
                .values()
                .filter(|session| {
                    session_blocks_profile_reuse(session, &profile.id)
                        && session.principal_id.as_deref() == Some(authority.principal_id.as_str())
                })
                .flat_map(|session| session.browser_ids.iter())
                .filter(|browser_id| {
                    service_state
                        .browsers
                        .get(*browser_id)
                        .is_some_and(|browser| {
                            browser.profile_id.as_deref() != Some(profile.id.as_str())
                        })
                })
                .cloned()
                .collect();
            same_principal_profile_mismatch_browser_ids.sort();
            same_principal_profile_mismatch_browser_ids.dedup();
            capability_profile_mismatch = authority.profile_id != profile.id;
            if !foreign_principal_session_ids.is_empty()
                || capability_profile_mismatch
                || active_claim_is_foreign
            {
                reusable_browser_ids.clear();
            }
        } else if !principal_bound_session_ids.is_empty() || active_claim_requires_authentication {
            reusable_browser_ids.clear();
        }
    } else {
        principal_bound_session_ids.clear();
    }

    let mut explicit_session_route_error = None;
    let mut explicit_session_route = None;
    let mut explicit_terminal_replacement_route = false;
    let mut explicit_terminal_launch_route = false;
    let mut explicit_authenticated_cold_route = false;
    if let Some(session_name) = request.session_name.as_deref() {
        match service_state.sessions.get(session_name) {
            None if terminal_replacement_session_name == Some(session_name) => {
                explicit_terminal_replacement_route = true;
            }
            None if terminal_replacement_launch_session_name.as_deref() == Some(session_name) => {
                explicit_terminal_launch_route = true;
            }
            None if authenticated_principal
                .and_then(|authority| authenticated_cold_session_name(authority, profile))
                .as_deref()
                == Some(session_name) =>
            {
                explicit_authenticated_cold_route = true;
            }
            None => explicit_session_route_error = Some("explicit_session_not_found"),
            Some(session) if session.browser_ids.len() != 1 => {
                explicit_session_route_error = Some("explicit_session_browser_mapping_ambiguous");
            }
            Some(session) => {
                let browser_id = &session.browser_ids[0];
                if reusable_browser_ids.iter().any(|id| id == browser_id) {
                    reusable_browser_ids.retain(|id| id == browser_id);
                    explicit_session_route = Some((browser_id.clone(), session_name.to_string()));
                } else {
                    explicit_session_route_error = Some("explicit_session_browser_not_compatible");
                    reusable_browser_ids.clear();
                }
            }
        }
    }

    let mut same_profile_live_browser_ids = service_state
        .browsers
        .iter()
        .filter(|(_id, browser)| {
            browser.profile_id.as_deref() == Some(profile.id.as_str())
                && browser_has_live_health(browser)
        })
        .map(|(id, _browser)| id.clone())
        .collect::<Vec<_>>();
    same_profile_live_browser_ids.sort();
    same_profile_live_browser_ids.dedup();

    let mut active_lease_session_ids = service_state
        .sessions
        .iter()
        .filter(|(_id, session)| {
            session_blocks_profile_reuse(session, &profile.id)
                && !session
                    .browser_ids
                    .iter()
                    .any(|browser_id| reusable_browser_ids.contains(browser_id))
        })
        .map(|(id, _session)| id.clone())
        .collect::<Vec<_>>();
    active_lease_session_ids.extend(foreign_principal_session_ids.iter().cloned());
    active_lease_session_ids.sort();
    active_lease_session_ids.dedup();

    let mut reasons = Vec::new();
    if manual_seeding_required {
        reasons.push("manual_seeding_required");
    }
    if reusable_browser_ids.is_empty() {
        reasons.push("no_compatible_live_browser");
    } else {
        reasons.push("compatible_live_browser_available");
    }
    if active_lease_session_ids.is_empty() {
        reasons.push("no_active_profile_lease_conflict");
    } else {
        reasons.push("active_profile_lease_conflict");
    }
    if !foreign_principal_session_ids.is_empty() {
        reasons.push("foreign_principal_profile_lease");
    }
    if active_claim.is_some() {
        reasons.push("canonical_active_profile_claim");
    }
    if active_claim_is_foreign {
        reasons.push("foreign_principal_active_claim");
    }
    if strict_identity_required
        && authenticated_principal.is_none()
        && (!principal_bound_session_ids.is_empty() || active_claim_requires_authentication)
    {
        reasons.push("profile_capability_required");
    }
    let profile_identity_inconsistent =
        capability_profile_mismatch || !same_principal_profile_mismatch_browser_ids.is_empty();
    if profile_identity_inconsistent {
        reasons.push("lifecycle_profile_identity_inconsistent");
    }
    if capability_profile_mismatch {
        reasons.push("profile_capability_profile_mismatch");
    }
    if same_profile_live_browser_ids.len() > 1 {
        reasons.push("duplicate_live_browsers_for_profile");
    }
    if active_lease_session_ids.len() > 1 {
        reasons.push("duplicate_active_leases_for_profile");
    }
    if request.browser_host.is_some() {
        reasons.push("browser_host_constrained_by_request");
    } else if profile.profile_origin == ProfileOrigin::ExternalByop && browser_host.is_some() {
        reasons.push("external_byop_browser_host_unconstrained");
    }
    if request.view_stream_provider.is_some() {
        reasons.push("view_stream_constrained_by_request");
    }
    if request.control_input_provider.is_some() {
        reasons.push("control_input_constrained_by_request");
    }
    if request.display_isolation.is_some() {
        reasons.push("display_isolation_constrained_by_request");
    }
    if let Some(reason) = explicit_session_route_error {
        reasons.push(reason);
    } else if explicit_session_route.is_some() {
        reasons.push("explicit_session_route_selected");
    } else if explicit_terminal_replacement_route {
        reasons.push("explicit_session_terminal_replacement_selected");
    } else if explicit_terminal_launch_route {
        reasons.push("explicit_session_terminal_launch_selected");
    } else if explicit_authenticated_cold_route {
        reasons.push("explicit_authenticated_cold_route_selected");
    }
    reasons.sort();
    reasons.dedup();

    let recommended_action = if manual_seeding_required {
        "seed_profile_before_reuse"
    } else if explicit_session_route_error.is_some() {
        "blocked_by_explicit_session_route"
    } else if strict_identity_required
        && (!foreign_principal_session_ids.is_empty() || active_claim_is_foreign)
    {
        "wait_for_foreign_principal"
    } else if strict_identity_required
        && authenticated_principal.is_none()
        && (!principal_bound_session_ids.is_empty() || active_claim_requires_authentication)
    {
        "authenticate_for_profile_reuse"
    } else if profile_identity_inconsistent {
        "lifecycle_profile_identity_inconsistent"
    } else if !reusable_browser_ids.is_empty() {
        "reuse_existing_browser"
    } else if !active_lease_session_ids.is_empty() {
        "wait_for_profile_lease"
    } else {
        "launch_new_browser"
    };
    let reusable_browser_id = reusable_browser_ids.first().cloned();
    let reusable_session_name = explicit_session_route
        .map(|(_browser_id, session_name)| session_name)
        .or_else(|| {
            reusable_browser_id
                .as_deref()
                .and_then(|browser_id| reusable_session_name_for_browser(service_state, browser_id))
        });

    let active_lease_count = if active_claim.is_some() {
        1
    } else {
        active_lease_session_ids.len()
    };
    json!({
        "recommendedAction": recommended_action,
        "selectedProfileId": profile.id,
        "profileProcessPolicy": "exclusive_process",
        "clientSharingPolicy": "shared_browser_tabs",
        "defaultAcquisition": if recommended_action == "reuse_existing_browser" { "tab_new" } else { "launch_new_browser" },
        "sharedAcquisition": {
            "policy": "shared_browser_tabs",
            "mode": if recommended_action == "reuse_existing_browser" { json!("tab_new") } else { Value::Null },
            "browserId": reusable_browser_id.clone(),
            "sessionName": reusable_session_name.clone(),
            "requiresRouteHints": recommended_action == "reuse_existing_browser",
            "routeHintFields": if recommended_action == "reuse_existing_browser" { json!(["browserId", "sessionName"]) } else { json!([]) },
            "controlSerialization": "service_queue",
            "cleanupPolicy": "close_tabs",
            "duplicateProcessAllowed": false,
        },
        "maxConcurrentTabs": Value::Null,
        "maxConcurrentWindows": Value::Null,
        "reusableBrowserId": reusable_browser_id,
        "reusableSessionName": reusable_session_name,
        "reusableBrowserIds": reusable_browser_ids,
        "compatibleLiveBrowserCount": reusable_browser_ids.len(),
        "sameProfileLiveBrowserCount": same_profile_live_browser_ids.len(),
        "sameProfileLiveBrowserIds": same_profile_live_browser_ids,
        "activeLeaseSessionIds": active_lease_session_ids,
        "activeLeaseCount": active_lease_count,
        "activeClaimId": active_claim.map(ActiveLeaseClaim::claim_id),
        "activeClaimRevision": active_claim.map(ActiveLeaseClaim::revision),
        "activeClaimFencingToken": active_claim.map(ActiveLeaseClaim::fencing_token),
        "activeClaimPrincipalId": active_claim.map(ActiveLeaseClaim::principal_id),
        "foreignPrincipalSessionIds": foreign_principal_session_ids,
        "principalBoundSessionIds": principal_bound_session_ids,
        "profileMismatchBrowserIds": same_principal_profile_mismatch_browser_ids,
        "blockingIdentityAxes": if profile_identity_inconsistent { json!(["profile"]) } else { json!([]) },
        "duplicatePressure": same_profile_live_browser_ids.len() > 1 || active_lease_session_ids.len() > 1,
        "profileLeasePolicy": "wait",
        "browserHost": browser_host,
        "viewStreamProvider": view_stream_provider,
        "controlInputProvider": control_input_provider,
        "displayIsolation": display_isolation,
        "reasons": reasons,
    })
}

fn reusable_session_name_for_browser(
    service_state: &ServiceState,
    browser_id: &str,
) -> Option<String> {
    service_state
        .browsers
        .get(browser_id)
        .and_then(|browser| browser.active_session_ids.first().cloned())
        .or_else(|| {
            service_state
                .sessions
                .iter()
                .find_map(|(session_id, session)| {
                    session
                        .browser_ids
                        .iter()
                        .any(|id| id == browser_id)
                        .then_some(session_id.clone())
                })
        })
        .or_else(|| browser_id.strip_prefix("session:").map(str::to_string))
}

fn browser_is_reusable_for_posture(
    browser: &BrowserProcess,
    browser_host: Option<BrowserHost>,
    view_stream_provider: Option<ViewStreamProvider>,
    control_input_provider: Option<ControlInputProvider>,
    display_isolation: Option<&str>,
) -> bool {
    if !browser_has_live_health(browser) {
        return false;
    }
    if browser_host.is_some_and(|expected| browser.host != expected) {
        return false;
    }
    if display_isolation.is_some() && browser.display_isolation.as_deref() != display_isolation {
        return false;
    }
    if let Some(expected_provider) = view_stream_provider {
        if !browser
            .view_streams
            .iter()
            .any(|stream| stream.provider == expected_provider)
        {
            return false;
        }
    }
    if let Some(expected_input) = control_input_provider {
        if !browser.view_streams.iter().any(|stream| {
            stream
                .control_input
                .is_some_and(|control_input| control_input == expected_input)
        }) {
            return false;
        }
    }
    true
}

fn browser_has_live_health(browser: &BrowserProcess) -> bool {
    matches!(
        browser.health,
        BrowserHealth::Ready | BrowserHealth::Launching | BrowserHealth::Reconnecting
    ) && boot_epoch_is_not_prior(browser.boot_epoch.as_deref())
}

fn session_blocks_profile_reuse(session: &BrowserSession, profile_id: &str) -> bool {
    session.profile_id.as_deref() == Some(profile_id)
        && boot_epoch_is_not_prior(session.boot_epoch.as_deref())
        && matches!(
            session.lease,
            LeaseState::Exclusive | LeaseState::HumanTakeover
        )
}

fn boot_epoch_is_not_prior(recorded_boot_epoch: Option<&str>) -> bool {
    crate::process_identity::boot_epoch_status(
        recorded_boot_epoch,
        crate::process_identity::current_boot_epoch().as_deref(),
    ) != crate::process_identity::BootEpochStatus::Prior
}

/// Project replacement authority and the exact collision-free daemon route
/// that can supersede one cleanup-satisfied terminal owner.
pub(crate) fn lifecycle_replacement_decision(
    selected_profile: Option<&BrowserProfile>,
    service_state: &ServiceState,
) -> Value {
    let Some(profile) = selected_profile else {
        return json!({
            "available": false,
            "replacementEligible": false,
            "reason": "no_selected_profile",
        });
    };
    let profile_path = profile
        .user_data_dir
        .as_deref()
        .map(std::path::PathBuf::from)
        .or_else(|| crate::runtime_profile::runtime_profile_user_data_dir(&profile.id).ok());
    let Some(profile_identity_digest) = profile_path
        .as_deref()
        .and_then(|path| crate::runtime_profile::canonical_profile_identity_digest(path).ok())
    else {
        return json!({
            "available": false,
            "profileId": profile.id,
            "replacementEligible": false,
            "reason": "profile_identity_unavailable",
        });
    };
    let owner = service_state
        .runtime_owner_registry
        .owners
        .get(&profile_identity_digest);
    let mut records = service_state
        .runtime_owner_registry
        .lifecycle_records
        .values()
        .filter(|record| record.profile_identity_digest == profile_identity_digest)
        .collect::<Vec<_>>();
    records.sort_by_key(|record| record.owner_generation);
    let owner_lifecycle = owner.and_then(|owner| {
        records.iter().copied().find(|record| {
            record.logical_browser_id == owner.browser_id
                && record.owner_generation == owner.owner_generation
        })
    });
    // Lifecycle history is observational. Only the record joined to the exact
    // current owner generation may participate in an operational decision.
    let lifecycle = owner_lifecycle;
    let terminal_cleanup_satisfied = lifecycle.is_some_and(|record| {
        record.lifecycle_state == crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Terminal
            && record.cleanup_obligation_state
                == crate::runtime_owner_transfer::CleanupObligationState::Satisfied
    });
    let terminal_process_exit_recorded = lifecycle.is_some_and(|record| {
        record.terminal_evidence.iter().any(|evidence| {
            evidence == "exact_process_exited"
                || evidence.starts_with("service_reconcile_process_group_absent:")
        })
    });
    let terminal_profile_lock_release_recorded = lifecycle.is_some_and(|record| {
        record.terminal_evidence.iter().any(|evidence| {
            evidence == "profile_lock_released"
                || evidence == "service_reconcile_profile_lock_absent"
                || evidence.starts_with("service_reconcile_profile_lock_stale_pid_absent:")
        })
    });
    let current_process_proven = owner.is_some_and(|owner| {
        service_state
            .browsers
            .get(&owner.browser_id)
            .is_some_and(|browser| browser.pid.is_some())
    });
    let terminal_process_absence_proven = terminal_process_exit_recorded
        && terminal_profile_lock_release_recorded
        && !current_process_proven;
    let active_profile_lease_session_ids = service_state
        .sessions
        .values()
        .filter(|session| {
            session.profile_id.as_deref() == Some(profile.id.as_str())
                && matches!(
                    session.lease,
                    LeaseState::Shared | LeaseState::Exclusive | LeaseState::HumanTakeover
                )
        })
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    let replacement_route = owner.zip(owner_lifecycle).and_then(|(owner, record)| {
        (terminal_cleanup_satisfied
            && terminal_process_absence_proven
            && active_profile_lease_session_ids.is_empty()
            && record.logical_browser_id == owner.browser_id
            && record.owner_generation == owner.owner_generation)
            .then(|| (owner.browser_id.clone(), owner.daemon_session_route.clone()))
    });
    let replacement_eligible = match (owner, lifecycle) {
        (None, None) => true,
        (Some(_), Some(_)) => replacement_route.is_some(),
        _ => false,
    };
    let reason = match lifecycle {
        None if owner.is_none() => "no_lifecycle_owner",
        None => "lifecycle_owner_record_missing",
        Some(_) if replacement_route.is_some() => "terminal_cleanup_satisfied",
        Some(_) if current_process_proven => "terminal_process_still_live",
        Some(_) if !active_profile_lease_session_ids.is_empty() => "terminal_profile_lease_active",
        Some(_) if terminal_cleanup_satisfied => "terminal_replacement_route_inconsistent",
        Some(record)
            if record.lifecycle_state
                == crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Closing =>
        {
            "closing_lifecycle_requires_reconciliation"
        }
        Some(_) => "lifecycle_observation_not_replacement_eligible",
    };
    let required_action = match reason {
        "no_lifecycle_owner" => "launch_new_browser",
        "terminal_cleanup_satisfied" => "supersede_terminal_owner",
        "closing_lifecycle_requires_reconciliation" => "reconcile_lifecycle_owner",
        _ => "inspect_lifecycle_owner",
    };

    json!({
        "available": true,
        "profileId": profile.id,
        "registryRevision": service_state.runtime_owner_registry.revision,
        "ownerId": owner.map(|owner| owner.owner_id.clone()),
        "ownerState": owner.map(|owner| owner.state),
        "replacementBrowserId": replacement_route.as_ref().map(|(browser_id, _)| browser_id.clone()),
        "replacementSessionName": replacement_route.as_ref().map(|(_, session_name)| session_name.clone()),
        "logicalBrowserId": lifecycle.map(|record| record.logical_browser_id.clone()),
        "ownerGeneration": lifecycle.map(|record| record.owner_generation),
        "lifecycleState": lifecycle.map(|record| record.lifecycle_state),
        "cleanupObligationState": lifecycle.map(|record| record.cleanup_obligation_state),
        "processAbsenceProven": terminal_process_absence_proven,
        "activeProfileLeaseSessionIds": active_profile_lease_session_ids,
        "terminalEvidence": lifecycle.map(|record| record.terminal_evidence.clone()).unwrap_or_default(),
        "replacementEligible": replacement_eligible,
        "reason": reason,
        "requiredAction": required_action,
    })
}

/// Executable posture selected for one profile-acquisition request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileAcquisitionDisposition {
    ReuseExistingBrowser,
    LaunchNewBrowser,
    Blocked,
}

/// Joined acquisition result consumed by access-plan and action-runtime code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileAcquisitionDecision {
    disposition: ProfileAcquisitionDisposition,
    selected_profile_id: Option<String>,
    browser_id: Option<String>,
    session_name: Option<String>,
    acquisition_blocker: Option<String>,
    policy_revision: u64,
    access_decision_id: String,
    client_subject_id: Option<String>,
    identity_assurance: ProfileIdentityAssurance,
    connection_instance_id: Option<String>,
    profile_child_access: ProfileChildAccess,
    service_request_available: bool,
    profile_reuse_action: String,
    profile_reuse_reasons: Vec<String>,
    replacement_eligible: bool,
    replacement_session_name: Option<String>,
    runtime_owner_registry_revision: Option<u64>,
    owner_id: Option<String>,
    owner_generation: Option<u64>,
}

impl ProfileAcquisitionDecision {
    /// Join the owner's compatibility projections into its executable result.
    fn from_components(
        selected_profile: Option<&BrowserProfile>,
        access_policy: &ServiceProfileAccessPolicy,
        access_decision: &ServiceProfileAccessDecision,
        profile_reuse: &Value,
        lifecycle_replacement: &Value,
        service_request: &Value,
    ) -> Result<Self, String> {
        let recommended_action = profile_reuse
            .get("recommendedAction")
            .and_then(Value::as_str)
            .ok_or_else(|| "profile_acquisition_recommended_action_missing".to_string())?;
        let available = service_request
            .get("available")
            .and_then(Value::as_bool)
            .ok_or_else(|| "profile_acquisition_availability_missing".to_string())?;
        let acquisition_blocker = service_request
            .get("acquisitionBlocker")
            .and_then(Value::as_str)
            .map(str::to_string);
        let disposition = if recommended_action == "reuse_existing_browser" && available {
            ProfileAcquisitionDisposition::ReuseExistingBrowser
        } else if recommended_action == "launch_new_browser" && available {
            ProfileAcquisitionDisposition::LaunchNewBrowser
        } else {
            ProfileAcquisitionDisposition::Blocked
        };
        let browser_id = profile_reuse
            .get("reusableBrowserId")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string);
        let session_name = service_request
            .get("request")
            .and_then(Value::as_object)
            .and_then(|request| request.get("sessionName"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                profile_reuse
                    .get("reusableSessionName")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
            })
            .map(str::to_string);

        if disposition == ProfileAcquisitionDisposition::ReuseExistingBrowser
            && (browser_id.is_none() || session_name.is_none())
        {
            return Err("profile_acquisition_reuse_route_incomplete".to_string());
        }

        Ok(Self {
            disposition,
            selected_profile_id: selected_profile.map(|profile| profile.id.clone()),
            browser_id,
            session_name,
            acquisition_blocker,
            policy_revision: access_decision.policy_revision,
            access_decision_id: access_decision.decision_id.clone(),
            client_subject_id: access_decision.subject.subject_id.clone(),
            identity_assurance: access_decision.subject.assurance,
            connection_instance_id: access_decision.subject.connection_instance_id.clone(),
            profile_child_access: ProfileChildAccess::from_admission(
                access_policy,
                access_decision,
                access_decision.subject.connection_instance_id.clone(),
            ),
            service_request_available: available,
            profile_reuse_action: recommended_action.to_string(),
            profile_reuse_reasons: profile_reuse
                .get("reasons")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
            replacement_eligible: lifecycle_replacement
                .get("replacementEligible")
                .and_then(Value::as_bool)
                == Some(true),
            replacement_session_name: lifecycle_replacement
                .get("replacementSessionName")
                .and_then(Value::as_str)
                .map(str::to_string),
            runtime_owner_registry_revision: lifecycle_replacement
                .get("registryRevision")
                .and_then(Value::as_u64),
            owner_id: lifecycle_replacement
                .get("ownerId")
                .and_then(Value::as_str)
                .map(str::to_string),
            owner_generation: lifecycle_replacement
                .get("ownerGeneration")
                .and_then(Value::as_u64),
        })
    }

    pub(crate) fn disposition(&self) -> ProfileAcquisitionDisposition {
        self.disposition
    }

    pub(crate) fn selected_profile_id(&self) -> Option<&str> {
        self.selected_profile_id.as_deref()
    }

    pub(crate) fn browser_id(&self) -> Option<&str> {
        self.browser_id.as_deref()
    }

    pub(crate) fn session_name(&self) -> Option<&str> {
        self.session_name.as_deref()
    }

    pub(crate) fn acquisition_blocker(&self) -> Option<&str> {
        self.acquisition_blocker.as_deref()
    }

    /// Canonical semantic oracle for the typed owner and its public projection.
    #[cfg(test)]
    pub(crate) fn assert_public_projection(&self, public_decision: &Value) {
        let profile_reuse = &public_decision["profileReuse"];
        let service_request = &public_decision["serviceRequest"];
        assert_eq!(
            public_decision["profileId"].as_str(),
            self.selected_profile_id(),
            "selected profile projection diverged from acquisition owner"
        );
        assert_eq!(
            profile_reuse["recommendedAction"].as_str(),
            Some(self.profile_reuse_action.as_str()),
            "reuse action projection diverged from acquisition owner"
        );
        assert_eq!(
            service_request["available"].as_bool(),
            Some(self.service_request_available),
            "request availability projection diverged from acquisition owner"
        );
        assert_eq!(
            service_request["acquisitionBlocker"].as_str(),
            self.acquisition_blocker(),
            "dominant blocker projection diverged from acquisition owner"
        );
        assert_eq!(
            profile_reuse["reusableBrowserId"].as_str(),
            self.browser_id(),
            "browser route projection diverged from acquisition owner"
        );
        let projected_session = service_request["request"]["sessionName"]
            .as_str()
            .or_else(|| profile_reuse["reusableSessionName"].as_str());
        assert_eq!(
            projected_session,
            self.session_name(),
            "session route projection diverged from acquisition owner"
        );
        let projected_disposition = match (
            profile_reuse["recommendedAction"].as_str(),
            service_request["available"].as_bool(),
        ) {
            (Some("reuse_existing_browser"), Some(true)) => {
                ProfileAcquisitionDisposition::ReuseExistingBrowser
            }
            (Some("launch_new_browser"), Some(true)) => {
                ProfileAcquisitionDisposition::LaunchNewBrowser
            }
            _ => ProfileAcquisitionDisposition::Blocked,
        };
        assert_eq!(
            projected_disposition,
            self.disposition(),
            "disposition projection diverged from acquisition owner"
        );
    }

    /// Apply the already-joined route decision without reparsing public plan
    /// JSON in the action-runtime caller.
    pub(crate) fn apply_to_service_command(
        &self,
        command: &mut Value,
        authenticated_principal: Option<&AuthenticatedServicePrincipal>,
    ) -> Result<(), String> {
        command["policyRevision"] = json!(self.policy_revision);
        command["accessDecisionId"] = json!(self.access_decision_id);
        command["identityAssurance"] = json!(self.identity_assurance);
        if let Some(subject_id) = self.client_subject_id.as_ref() {
            command["clientSubjectId"] = json!(subject_id);
        }
        if let Some(connection_instance_id) = self.connection_instance_id.as_ref() {
            command["connectionInstanceId"] = json!(connection_instance_id);
        }
        command["profileChildAccess"] = serde_json::to_value(&self.profile_child_access)
            .expect("profile child access must serialize");
        if !self.service_request_available {
            return Err(format!(
                "service_access_plan_request_unavailable:{}",
                self.acquisition_blocker()
                    .unwrap_or("service_request_unavailable")
            ));
        }

        if self.disposition != ProfileAcquisitionDisposition::ReuseExistingBrowser {
            if service_request_has_partial_route_hints(command) {
                let requested_session = command.get("sessionName").and_then(Value::as_str);
                let exact_terminal_replacement = !service_request_has_browser_hint(command)
                    && self.replacement_eligible
                    && requested_session == self.replacement_session_name.as_deref();
                let exact_authenticated_cold_route = !service_request_has_browser_hint(command)
                    && self.profile_reuse_action == "launch_new_browser"
                    && self
                        .profile_reuse_reasons
                        .iter()
                        .any(|reason| reason == "explicit_authenticated_cold_route_selected")
                    && requested_session == self.session_name();
                let exact_terminal_launch_route = !service_request_has_browser_hint(command)
                    && self.replacement_eligible
                    && self
                        .profile_reuse_reasons
                        .iter()
                        .any(|reason| reason == "explicit_session_terminal_launch_selected")
                    && requested_session == self.session_name()
                    && requested_session != self.replacement_session_name.as_deref();
                if !exact_terminal_replacement
                    && !exact_authenticated_cold_route
                    && !exact_terminal_launch_route
                {
                    return Err("service_access_plan_incomplete_route_hints".to_string());
                }
                if let Some(authority) = authenticated_principal {
                    self.attach_launch_route_authorization(
                        command,
                        authority,
                        if exact_terminal_replacement {
                            "terminal_replacement"
                        } else {
                            "authenticated_cold"
                        },
                    );
                }
                return Ok(());
            }
            if !service_request_has_session_hint(command) {
                if let Some(session_name) = self.session_name() {
                    command["sessionName"] = json!(session_name);
                }
            }
            if let Some(authority) = authenticated_principal {
                let planned_session = command.get("sessionName").and_then(Value::as_str);
                self.attach_launch_route_authorization(
                    command,
                    authority,
                    if self.replacement_eligible
                        && planned_session == self.replacement_session_name.as_deref()
                        && self.replacement_session_name.is_some()
                    {
                        "terminal_replacement"
                    } else {
                        "authenticated_cold"
                    },
                );
            }
            return Ok(());
        }

        let browser_id = self
            .browser_id()
            .ok_or_else(|| "service_access_plan_reuse_missing_browser_id".to_string())?;
        let session_name = self
            .session_name()
            .ok_or_else(|| "service_access_plan_reuse_missing_session_name".to_string())?;
        command["browserId"] = json!(browser_id);
        command["sessionName"] = json!(session_name);
        Ok(())
    }

    fn attach_launch_route_authorization(
        &self,
        command: &mut Value,
        authority: &AuthenticatedServicePrincipal,
        route_kind: &str,
    ) {
        let session_name = command.get("sessionName").and_then(Value::as_str);
        if session_name.is_none()
            || self.selected_profile_id() != Some(authority.profile_id.as_str())
        {
            return;
        }
        command["serviceProfileRouteAuthorization"] = json!({
            "schemaVersion": "agent-browser.profile-launch-route-authorization.v1",
            "kind": route_kind,
            "sessionName": session_name,
            "profileId": self.selected_profile_id,
            "principalId": authority.principal_id,
            "capabilityId": authority.capability_id,
            "capabilityRevision": authority.capability_revision,
            "runtimeOwnerRegistryRevision": self.runtime_owner_registry_revision,
            "ownerId": self.owner_id,
            "ownerGeneration": self.owner_generation,
        });
    }
}

fn service_request_route_hint_count(command: &Value) -> usize {
    ["browserId", "sessionName"]
        .iter()
        .filter(|key| {
            command
                .get(**key)
                .and_then(Value::as_str)
                .is_some_and(|value| !value.trim().is_empty())
        })
        .count()
}

fn service_request_has_browser_hint(command: &Value) -> bool {
    service_request_has_route_hint_field(command, "browserId")
}

fn service_request_has_session_hint(command: &Value) -> bool {
    service_request_has_route_hint_field(command, "sessionName")
}

fn service_request_has_route_hint_field(command: &Value, field: &str) -> bool {
    command
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

fn service_request_has_partial_route_hints(command: &Value) -> bool {
    service_request_route_hint_count(command) == 1
}
