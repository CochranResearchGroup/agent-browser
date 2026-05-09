//! No-launch access planning for service-owned browser/profile decisions.
//!
//! The access plan joins profile selection, site policy, provider, challenge,
//! and readiness state before a caller asks the service to launch or control a
//! browser. It is intentionally read-only so agents and software clients can
//! get the service recommendation without creating browser process pressure.

use serde_json::{json, Map, Value};

use super::service_lifecycle::{select_service_profile_for_request, ProfileSelectionRequest};
use super::service_model::{
    builtin_site_policy, service_profile_seeding_handoff, BrowserHost, BrowserProfile, Challenge,
    ChallengeKind, ChallengePolicy, ChallengeState, InteractionMode, ProfileSelectionReason,
    ProviderCapability, ServiceEntitySource, ServiceIncidentEscalation, ServiceIncidentState,
    ServiceProvider, ServiceState, SitePolicy, SERVICE_JOB_NAMING_WARNING_MISSING_AGENT_NAME,
    SERVICE_JOB_NAMING_WARNING_MISSING_SERVICE_NAME, SERVICE_JOB_NAMING_WARNING_MISSING_TASK_NAME,
};

/// Parsed access-plan selector shared by HTTP and MCP resources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ServiceAccessPlanRequest {
    pub(crate) service_name: Option<String>,
    pub(crate) agent_name: Option<String>,
    pub(crate) task_name: Option<String>,
    pub(crate) target_service_ids: Vec<String>,
    pub(crate) site_policy_id: Option<String>,
    pub(crate) challenge_id: Option<String>,
    pub(crate) readiness_profile_id: Option<String>,
}

impl ServiceAccessPlanRequest {
    fn profile_selection_request(&self) -> ProfileSelectionRequest {
        ProfileSelectionRequest {
            service_name: self.service_name.clone(),
            target_service_ids: self.target_service_ids.clone(),
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
            "sitePolicyId" | "site_policy_id" | "site-policy-id" => {
                request.site_policy_id = non_empty(value);
            }
            "challengeId" | "challenge_id" | "challenge-id" => {
                request.challenge_id = non_empty(value);
            }
            "readinessProfileId" | "readiness_profile_id" | "readiness-profile-id" => {
                request.readiness_profile_id = non_empty(value);
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
    Ok(request)
}

/// Build the read-only service access plan from already-loaded service state.
pub(crate) fn service_access_plan_for_state(
    service_state: &ServiceState,
    request: ServiceAccessPlanRequest,
) -> Value {
    let original_state = service_state;
    let mut effective_state = original_state.clone();
    effective_state.refresh_profile_readiness();
    let service_state = &effective_state;
    let profile_request = request.profile_selection_request();
    let selection = select_service_profile_for_request(service_state, &profile_request);
    let selected_profile = selection
        .as_ref()
        .and_then(|selection| service_state.profiles.get(&selection.profile_id))
        .cloned();
    let readiness_id = request.readiness_profile_id.clone().or_else(|| {
        selection
            .as_ref()
            .map(|selection| selection.profile_id.clone())
    });
    let readiness_profile = readiness_id
        .as_deref()
        .and_then(|profile_id| service_state.profiles.get(profile_id));
    let target_readiness = readiness_profile
        .map(|profile| profile.target_readiness.clone())
        .unwrap_or_default();
    let readiness = readiness_id.map(|profile_id| {
        let count = target_readiness.len();
        json!({
            "profileId": profile_id,
            "targetReadiness": target_readiness,
            "count": count,
        })
    });
    let readiness_summary = readiness_summary(readiness.as_ref(), &request.target_service_ids);
    let seeding_handoff =
        seeding_handoff_for_readiness(service_state, readiness.as_ref(), &readiness_summary);
    let monitor_findings = access_plan_monitor_findings(service_state, &request.target_service_ids);
    let selected_site_policy =
        select_site_policy(original_state, &request, selected_profile.as_ref());
    let site_policy = selected_site_policy
        .as_ref()
        .map(|selected| selected.policy.clone());
    let site_policy_source = selected_site_policy
        .as_ref()
        .map(|selected| selected.source_value())
        .unwrap_or(Value::Null);
    let challenges = select_challenges(service_state, request.challenge_id.as_deref());
    let providers = select_providers(
        service_state,
        selected_profile.as_ref(),
        site_policy.as_ref(),
        &challenges,
    );
    let naming_warnings = access_plan_naming_warnings(&request);
    let has_naming_warning = !naming_warnings.is_empty();
    let decision = access_plan_decision(AccessPlanDecisionInput {
        request: &request,
        selected_profile: selected_profile.as_ref(),
        site_policy: site_policy.as_ref(),
        challenges: &challenges,
        providers: &providers,
        readiness: readiness.as_ref(),
        target_service_ids: &request.target_service_ids,
        readiness_summary: &readiness_summary,
        monitor_findings: &monitor_findings,
        naming_warnings: &naming_warnings,
    });

    json!({
        "query": {
            "serviceName": request.service_name,
            "agentName": request.agent_name,
            "taskName": request.task_name,
            "targetServiceIds": request.target_service_ids,
            "sitePolicyId": request.site_policy_id,
            "challengeId": request.challenge_id,
            "readinessProfileId": request.readiness_profile_id,
            "namingWarnings": naming_warnings,
            "hasNamingWarning": has_naming_warning,
        },
        "selectedProfile": selected_profile.clone(),
        "selectedProfileSource": selection.as_ref().map(|selection| {
            profile_source_value(service_state, &selection.profile_id)
        }),
        "selectedProfileMatch": selection.as_ref().map(|selection| {
            let (matched_field, matched_identity) = selected_profile
                .as_ref()
                .map(|profile| service_profile_match_details(profile, &profile_request, selection.reason))
                .unwrap_or((None, None));
            json!({
                "profileId": selection.profile_id,
                "profile": selected_profile.clone(),
                "reason": selection.reason,
                "matchedField": matched_field,
                "matchedIdentity": matched_identity,
            })
        }),
        "readiness": readiness,
        "readinessSummary": readiness_summary,
        "seedingHandoff": seeding_handoff,
        "monitorFindings": monitor_findings,
        "sitePolicy": site_policy,
        "sitePolicySource": site_policy_source,
        "providers": providers,
        "challenges": challenges,
        "decision": decision,
    })
}

fn access_plan_naming_warnings(request: &ServiceAccessPlanRequest) -> Vec<&'static str> {
    [
        (
            request.service_name.is_none(),
            SERVICE_JOB_NAMING_WARNING_MISSING_SERVICE_NAME,
        ),
        (
            request.agent_name.is_none(),
            SERVICE_JOB_NAMING_WARNING_MISSING_AGENT_NAME,
        ),
        (
            request.task_name.is_none(),
            SERVICE_JOB_NAMING_WARNING_MISSING_TASK_NAME,
        ),
    ]
    .into_iter()
    .filter_map(|(missing, warning)| missing.then_some(warning))
    .collect()
}

fn profile_source_value(service_state: &ServiceState, profile_id: &str) -> Value {
    let source = service_state
        .profile_source(profile_id)
        .unwrap_or(ServiceEntitySource::PersistedState);
    json!({
        "id": profile_id,
        "source": source.as_str(),
        "overrideable": source.overrideable(),
        "precedence": ["config", "runtime_observed", "persisted_state"],
    })
}

#[derive(Debug, Clone)]
struct SelectedSitePolicy {
    policy: SitePolicy,
    source: ServiceEntitySource,
    matched_by: &'static str,
}

impl SelectedSitePolicy {
    fn source_value(&self) -> Value {
        json!({
            "id": self.policy.id.clone(),
            "source": self.source.as_str(),
            "matchedBy": self.matched_by,
            "overrideable": self.source.overrideable(),
            "precedence": ["config", "persisted_state", "builtin"],
        })
    }
}

fn selected_source_for_state_policy(
    service_state: &ServiceState,
    policy: SitePolicy,
    matched_by: &'static str,
) -> SelectedSitePolicy {
    let source = service_state
        .site_policy_source(&policy.id)
        .unwrap_or(ServiceEntitySource::PersistedState);
    SelectedSitePolicy {
        policy,
        source,
        matched_by,
    }
}

fn select_site_policy(
    service_state: &ServiceState,
    request: &ServiceAccessPlanRequest,
    selected_profile: Option<&BrowserProfile>,
) -> Option<SelectedSitePolicy> {
    if let Some(site_policy_id) = request.site_policy_id.as_deref() {
        if let Some(policy) = service_state.site_policies.get(site_policy_id) {
            return Some(selected_source_for_state_policy(
                service_state,
                policy.clone(),
                "explicit_site_policy_id",
            ));
        }
        return builtin_site_policy(site_policy_id).map(|policy| SelectedSitePolicy {
            policy,
            source: ServiceEntitySource::Builtin,
            matched_by: "explicit_site_policy_id",
        });
    }

    for target_service_id in &request.target_service_ids {
        if let Some(site_policy) = service_state.site_policies.get(target_service_id) {
            return Some(selected_source_for_state_policy(
                service_state,
                site_policy.clone(),
                "target_service_id",
            ));
        }
        if let Some(site_policy) = builtin_site_policy(target_service_id) {
            return Some(SelectedSitePolicy {
                policy: site_policy,
                source: ServiceEntitySource::Builtin,
                matched_by: "target_service_id",
            });
        }
    }

    selected_profile.and_then(|profile| {
        profile.site_policy_ids.iter().find_map(|site_policy_id| {
            if let Some(policy) = service_state.site_policies.get(site_policy_id) {
                return Some(selected_source_for_state_policy(
                    service_state,
                    policy.clone(),
                    "profile_site_policy_id",
                ));
            }
            builtin_site_policy(site_policy_id).map(|policy| SelectedSitePolicy {
                policy,
                source: ServiceEntitySource::Builtin,
                matched_by: "profile_site_policy_id",
            })
        })
    })
}

fn select_challenges(service_state: &ServiceState, challenge_id: Option<&str>) -> Vec<Challenge> {
    if let Some(challenge_id) = challenge_id {
        return service_state
            .challenges
            .get(challenge_id)
            .cloned()
            .into_iter()
            .collect();
    }

    service_state
        .challenges
        .values()
        .filter(|challenge| !matches!(challenge.state, ChallengeState::Resolved))
        .cloned()
        .collect()
}

fn select_providers(
    service_state: &ServiceState,
    selected_profile: Option<&BrowserProfile>,
    site_policy: Option<&SitePolicy>,
    challenges: &[Challenge],
) -> Vec<ServiceProvider> {
    let mut provider_ids = Vec::new();
    if let Some(profile) = selected_profile {
        provider_ids.extend(profile.credential_provider_ids.iter().cloned());
    }
    if let Some(site_policy) = site_policy {
        provider_ids.extend(site_policy.auth_providers.iter().cloned());
        provider_ids.extend(site_policy.allowed_challenge_providers.iter().cloned());
    }
    provider_ids.extend(
        challenges
            .iter()
            .filter_map(|challenge| challenge.provider_id.clone()),
    );
    provider_ids.sort();
    provider_ids.dedup();

    provider_ids
        .into_iter()
        .filter_map(|provider_id| service_state.providers.get(&provider_id))
        .filter(|provider| provider.enabled)
        .cloned()
        .collect()
}

struct AccessPlanDecisionInput<'a> {
    request: &'a ServiceAccessPlanRequest,
    selected_profile: Option<&'a BrowserProfile>,
    site_policy: Option<&'a SitePolicy>,
    challenges: &'a [Challenge],
    providers: &'a [ServiceProvider],
    readiness: Option<&'a Value>,
    target_service_ids: &'a [String],
    readiness_summary: &'a Value,
    monitor_findings: &'a Value,
    naming_warnings: &'a [&'static str],
}

fn access_plan_decision(input: AccessPlanDecisionInput<'_>) -> Value {
    let selected_profile = input.selected_profile;
    let site_policy = input.site_policy;
    let challenges = input.challenges;
    let providers = input.providers;
    let readiness = input.readiness;
    let target_service_ids = input.target_service_ids;
    let readiness_summary = input.readiness_summary;
    let monitor_findings = input.monitor_findings;
    let naming_warnings = input.naming_warnings;
    let mut reasons = Vec::new();
    let manual_seeding_required =
        readiness_summary["manualSeedingRequired"].as_bool() == Some(true);
    let profile_readiness_monitor_attention =
        monitor_findings["profileReadinessAttentionRequired"].as_bool() == Some(true);
    let denied_challenge = challenges
        .iter()
        .any(|challenge| matches!(challenge.state, ChallengeState::Denied));
    let failed_challenge = challenges
        .iter()
        .any(|challenge| matches!(challenge.state, ChallengeState::Failed));
    let waiting_for_human = challenges
        .iter()
        .any(|challenge| matches!(challenge.state, ChallengeState::WaitingForHuman));
    let waiting_for_provider = challenges.iter().any(|challenge| {
        matches!(
            challenge.state,
            ChallengeState::Detected | ChallengeState::WaitingForProvider
        )
    });
    let policy_denies = site_policy
        .is_some_and(|site_policy| matches!(site_policy.challenge_policy, ChallengePolicy::Deny));
    let profile_required = site_policy.is_some_and(|site_policy| site_policy.profile_required);
    let provider_decision = provider_decision(selected_profile, site_policy, challenges, providers);
    let interaction_decision = interaction_decision(site_policy);
    let launch_posture =
        launch_posture_decision(selected_profile, site_policy, manual_seeding_required);
    let manual_action_required = manual_seeding_required || waiting_for_human || failed_challenge;
    let freshness_update = freshness_update_decision(
        selected_profile,
        target_service_ids,
        manual_seeding_required || readiness_profile_needs_probe(readiness, target_service_ids),
    );

    if let Some(profile) = selected_profile {
        if readiness_profile_is_fresh_or_seeded(readiness, &profile.id, target_service_ids) {
            reasons.push("selected_profile_has_readiness_evidence");
        }
    } else {
        reasons.push("no_matching_profile");
    }

    if providers.is_empty() {
        reasons.push("no_enabled_provider_selected");
    } else {
        reasons.push("provider_available");
    }
    if manual_seeding_required {
        reasons.push("manual_seeding_required");
    }
    if profile_readiness_monitor_attention {
        reasons.push("profile_readiness_monitor_attention");
    }
    if let Some(site_policy) = site_policy {
        reasons.push("site_policy_selected");
        if site_policy.manual_login_preferred {
            reasons.push("site_policy_manual_login_preferred");
        }
        if site_policy.profile_required {
            reasons.push("site_policy_profile_required");
        }
    }
    if policy_denies {
        reasons.push("challenge_policy_deny");
    }
    if denied_challenge {
        reasons.push("challenge_denied");
    }
    if failed_challenge {
        reasons.push("challenge_failed");
    }
    if waiting_for_human {
        reasons.push("challenge_waiting_for_human");
    }
    if waiting_for_provider {
        reasons.push("challenge_waiting_for_provider");
    }
    reasons.sort();
    reasons.dedup();

    let recommended_action = if policy_denies || denied_challenge {
        "deny_request_by_site_policy"
    } else if manual_seeding_required {
        readiness_recommended_action(readiness, target_service_ids)
            .unwrap_or("seed_profile_before_authenticated_work")
    } else if waiting_for_human {
        "request_manual_challenge_approval"
    } else if waiting_for_provider {
        "wait_for_or_invoke_challenge_provider"
    } else if failed_challenge {
        "manual_intervention_required"
    } else if selected_profile.is_none() && profile_required {
        "register_or_seed_managed_profile"
    } else if selected_profile.is_none() {
        "register_managed_profile_or_request_throwaway_browser"
    } else if profile_readiness_monitor_attention
        && readiness_profile_needs_probe(readiness, target_service_ids)
    {
        "probe_target_auth_or_reseed_if_needed"
    } else if readiness_profile_needs_probe(readiness, target_service_ids) {
        "verify_or_seed_profile_before_authenticated_work"
    } else {
        "use_selected_profile"
    };
    let service_request = service_request_decision(
        input.request,
        selected_profile,
        policy_denies || denied_challenge,
        manual_action_required,
    );

    json!({
        "recommendedAction": recommended_action,
        "browserHost": launch_posture.browser_host,
        "launchPosture": launch_posture.value,
        "interactionMode": site_policy.map(|policy| policy.interaction_mode),
        "interactionRisk": interaction_decision.interaction_risk,
        "pacing": interaction_decision.pacing,
        "challengePolicy": site_policy.map(|policy| policy.challenge_policy),
        "profileId": selected_profile.map(|profile| profile.id.clone()),
        "manualActionRequired": manual_action_required,
        "manualSeedingRequired": manual_seeding_required,
        "monitorAttentionRequired": profile_readiness_monitor_attention,
        "providerIds": providers.iter().map(|provider| provider.id.clone()).collect::<Vec<_>>(),
        "authProviderIds": provider_decision.auth_provider_ids,
        "challengeProviderIds": provider_decision.challenge_provider_ids,
        "missingChallengeCapabilities": provider_decision.missing_challenge_capabilities,
        "challengeStrategy": provider_decision.challenge_strategy,
        "challengeIds": challenges.iter().map(|challenge| challenge.id.clone()).collect::<Vec<_>>(),
        "freshnessUpdate": freshness_update,
        "serviceRequest": service_request,
        "namingWarnings": naming_warnings,
        "hasNamingWarning": !naming_warnings.is_empty(),
        "reasons": reasons,
    })
}

/// Describe the queued browser-control handoff clients should use after planning.
fn service_request_decision(
    request: &ServiceAccessPlanRequest,
    selected_profile: Option<&BrowserProfile>,
    denied: bool,
    manual_action_required: bool,
) -> Value {
    let selected_profile_id = selected_profile.map(|profile| profile.id.clone());
    let available = selected_profile_id.is_some() && !denied && !manual_action_required;
    let recommended_after_manual_action =
        selected_profile_id.is_some() && !denied && manual_action_required;
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
    if !request.target_service_ids.is_empty() {
        service_request.insert(
            "targetServiceIds".to_string(),
            json!(request.target_service_ids),
        );
    }
    service_request.insert("profileLeasePolicy".to_string(), json!("wait"));

    json!({
        "available": available,
        "recommendedAfterManualAction": recommended_after_manual_action,
        "blockedByManualAction": manual_action_required,
        "blockedByPolicy": denied,
        "action": "tab_new",
        "selectedProfileId": selected_profile_id,
        "profileLeasePolicy": "wait",
        "request": Value::Object(service_request),
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
            "targetServiceIds",
            "profileLeasePolicy",
            "url",
            "params",
        ],
    })
}

/// Describe the serialized service-owned write path for bounded auth probes.
fn freshness_update_decision(
    selected_profile: Option<&BrowserProfile>,
    target_service_ids: &[String],
    recommended_after_probe: bool,
) -> Value {
    let profile_id = selected_profile.map(|profile| profile.id.clone());
    let http_route = profile_id
        .as_ref()
        .map(|profile_id| format!("/api/service/profiles/{}/freshness", profile_id));

    json!({
        "available": profile_id.is_some(),
        "recommendedAfterProbe": recommended_after_probe && profile_id.is_some(),
        "profileId": profile_id,
        "targetServiceIds": target_service_ids,
        "http": {
            "method": "POST",
            "route": http_route,
            "routeTemplate": "/api/service/profiles/<id>/freshness",
        },
        "mcp": {
            "tool": "service_profile_freshness_update",
        },
        "client": {
            "package": "@agent-browser/client/service-observability",
            "helper": "updateServiceProfileFreshness",
        },
        "requestFields": [
            "loginId",
            "targetServiceId",
            "targetServiceIds",
            "readinessState",
            "readinessEvidence",
            "lastVerifiedAt",
            "freshnessExpiresAt",
        ],
    })
}

#[derive(Debug)]
struct LaunchPostureDecision {
    browser_host: BrowserHost,
    value: Value,
}

fn launch_posture_decision(
    selected_profile: Option<&BrowserProfile>,
    site_policy: Option<&SitePolicy>,
    manual_seeding_required: bool,
) -> LaunchPostureDecision {
    let (browser_host, source) =
        if let Some(browser_host) = site_policy.and_then(|policy| policy.browser_host) {
            (browser_host, "site_policy")
        } else if let Some(browser_host) =
            selected_profile.and_then(|profile| profile.default_browser_host)
        {
            (browser_host, "profile_default")
        } else {
            (BrowserHost::LocalHeaded, "service_default")
        };
    let headed = !matches!(browser_host, BrowserHost::LocalHeadless);
    let remote_view_recommended = matches!(
        browser_host,
        BrowserHost::DockerHeaded | BrowserHost::RemoteHeaded | BrowserHost::CloudProvider
    );
    let attachable_after_seeding =
        !manual_seeding_required || !matches!(browser_host, BrowserHost::AttachedExisting);
    let mut rationale = Vec::new();

    if headed {
        rationale.push("headed_browser_host");
    } else {
        rationale.push("headless_browser_host");
    }
    if remote_view_recommended {
        rationale.push("remote_view_capable_host");
    }
    if manual_seeding_required {
        rationale.push("detached_first_login_required");
    }
    match source {
        "site_policy" => rationale.push("browser_host_from_site_policy"),
        "profile_default" => rationale.push("browser_host_from_profile_default"),
        _ => rationale.push("browser_host_from_service_default"),
    }

    LaunchPostureDecision {
        browser_host,
        value: json!({
            "browserHost": browser_host,
            "source": source,
            "headed": headed,
            "remoteViewRecommended": remote_view_recommended,
            "detachedFirstLoginRequired": manual_seeding_required,
            "attachableAfterSeeding": attachable_after_seeding,
            "rationale": rationale,
        }),
    }
}

#[derive(Debug, Default)]
struct InteractionDecision {
    interaction_risk: &'static str,
    pacing: Value,
}

fn interaction_decision(site_policy: Option<&SitePolicy>) -> InteractionDecision {
    let Some(site_policy) = site_policy else {
        return InteractionDecision {
            interaction_risk: "standard",
            pacing: json!({
                "minActionDelayMs": 0,
                "jitterMs": 0,
                "cooldownMs": null,
                "maxParallelSessions": null,
                "retryBudget": null,
                "rateLimited": false,
                "jittered": false,
                "singleSessionRecommended": false,
            }),
        };
    };
    let min_action_delay_ms = site_policy.rate_limit.min_action_delay_ms.unwrap_or(0);
    let jitter_ms = site_policy.rate_limit.jitter_ms.unwrap_or(0);
    let cooldown_ms = site_policy.rate_limit.cooldown_ms;
    let max_parallel_sessions = site_policy.rate_limit.max_parallel_sessions;
    let retry_budget = site_policy.rate_limit.retry_budget;
    let rate_limited = min_action_delay_ms > 0 || cooldown_ms.unwrap_or(0) > 0;
    let jittered = jitter_ms > 0;
    let single_session_recommended = max_parallel_sessions == Some(1);
    let interaction_risk = if site_policy.manual_login_preferred
        || matches!(site_policy.interaction_mode, InteractionMode::Manual)
    {
        "manual"
    } else if matches!(
        site_policy.interaction_mode,
        InteractionMode::HumanLikeInput
    ) || rate_limited
        || jittered
        || single_session_recommended
    {
        "hardened"
    } else {
        "standard"
    };

    InteractionDecision {
        interaction_risk,
        pacing: json!({
            "minActionDelayMs": min_action_delay_ms,
            "jitterMs": jitter_ms,
            "cooldownMs": cooldown_ms,
            "maxParallelSessions": max_parallel_sessions,
            "retryBudget": retry_budget,
            "rateLimited": rate_limited,
            "jittered": jittered,
            "singleSessionRecommended": single_session_recommended,
        }),
    }
}

#[derive(Debug, Default)]
struct ProviderDecision {
    auth_provider_ids: Vec<String>,
    challenge_provider_ids: Vec<String>,
    missing_challenge_capabilities: Vec<&'static str>,
    challenge_strategy: &'static str,
}

fn provider_decision(
    selected_profile: Option<&BrowserProfile>,
    site_policy: Option<&SitePolicy>,
    challenges: &[Challenge],
    providers: &[ServiceProvider],
) -> ProviderDecision {
    let mut auth_provider_ids = providers
        .iter()
        .filter(|provider| {
            selected_profile
                .is_some_and(|profile| profile.credential_provider_ids.contains(&provider.id))
                || site_policy.is_some_and(|policy| policy.auth_providers.contains(&provider.id))
        })
        .map(|provider| provider.id.clone())
        .collect::<Vec<_>>();
    let active_challenges = challenges
        .iter()
        .filter(|challenge| !matches!(challenge.state, ChallengeState::Resolved))
        .collect::<Vec<_>>();
    let required_capabilities = active_challenges
        .iter()
        .flat_map(|challenge| challenge_required_capabilities(challenge.kind))
        .collect::<Vec<_>>();
    let mut challenge_provider_ids = providers
        .iter()
        .filter(|provider| {
            required_capabilities
                .iter()
                .any(|capability| provider.capabilities.contains(capability))
        })
        .filter(|provider| {
            site_policy
                .filter(|policy| !policy.allowed_challenge_providers.is_empty())
                .is_none_or(|policy| policy.allowed_challenge_providers.contains(&provider.id))
        })
        .map(|provider| provider.id.clone())
        .collect::<Vec<_>>();
    let mut missing_challenge_capabilities = active_challenges
        .iter()
        .filter(|challenge| {
            let capabilities = challenge_required_capabilities(challenge.kind);
            !providers.iter().any(|provider| {
                provider_allowed_for_challenge(provider, site_policy)
                    && capabilities
                        .iter()
                        .any(|capability| provider.capabilities.contains(capability))
            })
        })
        .flat_map(|challenge| {
            challenge_required_capabilities(challenge.kind)
                .into_iter()
                .map(provider_capability_wire_name)
        })
        .collect::<Vec<_>>();

    auth_provider_ids.sort();
    auth_provider_ids.dedup();
    challenge_provider_ids.sort();
    challenge_provider_ids.dedup();
    missing_challenge_capabilities.sort();
    missing_challenge_capabilities.dedup();

    let challenge_strategy = match site_policy.map(|policy| policy.challenge_policy) {
        Some(ChallengePolicy::Deny) => "deny",
        _ if active_challenges.is_empty() => "none",
        Some(ChallengePolicy::ManualOnly) => "manual_only",
        Some(ChallengePolicy::ProviderPreferred) if !challenge_provider_ids.is_empty() => {
            "provider_preferred"
        }
        Some(ChallengePolicy::ProviderAllowed) if !challenge_provider_ids.is_empty() => {
            "provider_allowed"
        }
        Some(ChallengePolicy::AvoidFirst) => "avoid_first",
        _ if !missing_challenge_capabilities.is_empty() => "missing_provider",
        _ => "manual_review",
    };

    ProviderDecision {
        auth_provider_ids,
        challenge_provider_ids,
        missing_challenge_capabilities,
        challenge_strategy,
    }
}

fn provider_allowed_for_challenge(
    provider: &ServiceProvider,
    site_policy: Option<&SitePolicy>,
) -> bool {
    site_policy
        .filter(|policy| !policy.allowed_challenge_providers.is_empty())
        .is_none_or(|policy| policy.allowed_challenge_providers.contains(&provider.id))
}

fn challenge_required_capabilities(kind: ChallengeKind) -> Vec<ProviderCapability> {
    match kind {
        ChallengeKind::Captcha => vec![
            ProviderCapability::CaptchaSolve,
            ProviderCapability::VisualReasoning,
            ProviderCapability::HumanApproval,
        ],
        ChallengeKind::TwoFactor => vec![
            ProviderCapability::TotpCode,
            ProviderCapability::SmsCode,
            ProviderCapability::EmailCode,
            ProviderCapability::HumanApproval,
        ],
        ChallengeKind::Passkey => {
            vec![
                ProviderCapability::Passkey,
                ProviderCapability::HumanApproval,
            ]
        }
        ChallengeKind::SuspiciousLogin | ChallengeKind::BlockedFlow | ChallengeKind::Unknown => {
            vec![
                ProviderCapability::VisualReasoning,
                ProviderCapability::HumanApproval,
            ]
        }
    }
}

fn provider_capability_wire_name(capability: ProviderCapability) -> &'static str {
    match capability {
        ProviderCapability::PasswordFill => "password_fill",
        ProviderCapability::Passkey => "passkey",
        ProviderCapability::TotpCode => "totp_code",
        ProviderCapability::SmsCode => "sms_code",
        ProviderCapability::EmailCode => "email_code",
        ProviderCapability::VisualReasoning => "visual_reasoning",
        ProviderCapability::CaptchaSolve => "captcha_solve",
        ProviderCapability::HumanApproval => "human_approval",
    }
}

fn service_profile_match_details(
    profile: &BrowserProfile,
    request: &ProfileSelectionRequest,
    reason: ProfileSelectionReason,
) -> (Option<&'static str>, Option<String>) {
    match reason {
        ProfileSelectionReason::AuthenticatedTarget => (
            Some("authenticatedServiceIds"),
            first_matching_identity(
                &request.target_service_ids,
                &profile.authenticated_service_ids,
            ),
        ),
        ProfileSelectionReason::TargetMatch => (
            Some("targetServiceIds"),
            first_matching_identity(&request.target_service_ids, &profile.target_service_ids),
        ),
        ProfileSelectionReason::ServiceAllowList => (
            Some("sharedServiceIds"),
            request
                .service_name
                .as_ref()
                .filter(|service_name| {
                    profile
                        .shared_service_ids
                        .iter()
                        .any(|allowed| allowed == *service_name)
                })
                .cloned(),
        ),
        ProfileSelectionReason::ExplicitProfile => (None, None),
    }
}

fn first_matching_identity(requested: &[String], candidates: &[String]) -> Option<String> {
    requested
        .iter()
        .find(|requested| candidates.iter().any(|candidate| candidate == *requested))
        .cloned()
}

fn append_identity_values(target_service_ids: &mut Vec<String>, value: &str) {
    for item in value.split(',') {
        if let Some(item) = non_empty(item.to_string()) {
            target_service_ids.push(item);
        }
    }
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

    json!({
        "profileReadinessAttentionRequired": !incident_ids.is_empty(),
        "profileReadinessIncidentIds": incident_ids,
        "profileReadinessMonitorIds": monitor_ids,
        "profileReadinessResults": monitor_results,
        "targetServiceIds": matched_target_service_ids,
    })
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::native::service_model::{
        BrowserHost, BrowserProfile, Challenge, ChallengeKind, InteractionMode,
        ProfileKeyringPolicy, ProfileReadinessState, ProfileSeedingMode, ProfileTargetReadiness,
        ProviderCapability, ProviderKind, RateLimitPolicy, ServiceIncident, ServiceProvider,
        SitePolicy,
    };
    use serde_json::json;

    #[test]
    fn service_access_plan_recommends_google_manual_seeding_before_attachable_work() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "google-work".to_string(),
                BrowserProfile {
                    id: "google-work".to_string(),
                    name: "Google work".to_string(),
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
        assert_eq!(plan["decision"]["launchPosture"]["headed"], true);
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
        assert!(plan["decision"]["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "site_policy_manual_login_preferred"));
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
        assert_eq!(plan["decision"]["monitorAttentionRequired"], true);
        assert!(plan["decision"]["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "profile_readiness_monitor_attention"));
    }

    #[test]
    fn parse_service_access_plan_query_accepts_caller_labels() {
        let request = parse_service_access_plan_query(vec![
            ("service-name".to_string(), "JournalDownloader".to_string()),
            ("agentName".to_string(), "codex".to_string()),
            ("task_name".to_string(), "probeACSwebsite".to_string()),
            ("login-id".to_string(), "acs".to_string()),
        ])
        .unwrap();

        assert_eq!(request.service_name.as_deref(), Some("JournalDownloader"));
        assert_eq!(request.agent_name.as_deref(), Some("codex"));
        assert_eq!(request.task_name.as_deref(), Some("probeACSwebsite"));
        assert_eq!(request.target_service_ids, vec!["acs".to_string()]);
    }

    #[test]
    fn service_access_plan_recommends_selected_authenticated_profile() {
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "acs".to_string(),
                BrowserProfile {
                    id: "acs".to_string(),
                    name: "ACS".to_string(),
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
            plan["decision"]["launchPosture"]["detachedFirstLoginRequired"],
            false
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
        assert_eq!(plan["decision"]["interactionRisk"], "manual");
        assert_eq!(plan["decision"]["pacing"]["singleSessionRecommended"], true);
        assert_eq!(
            plan["decision"]["launchPosture"]["detachedFirstLoginRequired"],
            true
        );
        assert_eq!(
            plan["decision"]["recommendedAction"],
            "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
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
