//! No-launch access planning for service-owned browser/profile decisions.
//!
//! The access plan joins profile selection, site policy, provider, challenge,
//! and readiness state before a caller asks the service to launch or control a
//! browser. It is intentionally read-only so agents and software clients can
//! get the service recommendation without creating browser process pressure.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};

use super::service_lifecycle::{select_service_profile_for_request, ProfileSelectionRequest};
use super::service_model::{
    builtin_site_policy, service_profile_seeding_handoff, service_site_policy_id_for_url,
    BrowserBuild, BrowserHost, BrowserProfile, Challenge, ChallengeKind, ChallengePolicy,
    ChallengeState, InteractionMode, ProfileSelectionReason, ProviderCapability,
    ServiceEntitySource, ServiceIncidentEscalation, ServiceIncidentState, ServiceProvider,
    ServiceState, SitePolicy, SERVICE_JOB_NAMING_WARNING_MISSING_AGENT_NAME,
    SERVICE_JOB_NAMING_WARNING_MISSING_SERVICE_NAME, SERVICE_JOB_NAMING_WARNING_MISSING_TASK_NAME,
};

/// Parsed access-plan selector shared by HTTP and MCP resources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ServiceAccessPlanRequest {
    pub(crate) service_name: Option<String>,
    pub(crate) agent_name: Option<String>,
    pub(crate) task_name: Option<String>,
    pub(crate) target_service_ids: Vec<String>,
    pub(crate) account_ids: Vec<String>,
    pub(crate) target_url: Option<String>,
    pub(crate) site_policy_id: Option<String>,
    pub(crate) challenge_id: Option<String>,
    pub(crate) readiness_profile_id: Option<String>,
    pub(crate) browser_build: Option<BrowserBuild>,
    pub(crate) browser_build_explicit: bool,
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
            "browserBuild" | "browser_build" | "browser-build" => {
                request.browser_build = parse_browser_build(&value)?;
                request.browser_build_explicit = request.browser_build.is_some();
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
    mut request: ServiceAccessPlanRequest,
) -> Value {
    let original_state = service_state;
    let mut effective_state = original_state.clone();
    effective_state.refresh_profile_readiness();
    let service_state = &effective_state;
    if let Some(site_policy_id) = request
        .target_url
        .as_deref()
        .and_then(|url| service_site_policy_id_for_url(service_state, url))
    {
        request.target_service_ids.push(site_policy_id);
        request.target_service_ids.sort();
        request.target_service_ids.dedup();
    }
    if request.browser_build.is_none() {
        request.browser_build = browser_build_for_access_request(service_state, &request);
    }
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
    let browser_capability_evidence = browser_capability_evidence_for_access_plan(
        service_state,
        &request,
        selected_profile.as_ref(),
        site_policy.as_ref(),
    );
    let naming_warnings = access_plan_naming_warnings(&request);
    let has_naming_warning = !naming_warnings.is_empty();
    let decision = access_plan_decision(AccessPlanDecisionInput {
        request: &request,
        selected_profile: selected_profile.as_ref(),
        service_state,
        site_policy: site_policy.as_ref(),
        challenges: &challenges,
        providers: &providers,
        readiness: readiness.as_ref(),
        target_service_ids: &request.target_service_ids,
        readiness_summary: &readiness_summary,
        monitor_findings: &monitor_findings,
        naming_warnings: &naming_warnings,
        browser_capability_evidence: &browser_capability_evidence,
    });

    json!({
        "query": {
            "serviceName": request.service_name,
            "agentName": request.agent_name,
            "taskName": request.task_name,
            "targetServiceIds": request.target_service_ids,
            "accountIds": request.account_ids,
            "url": request.target_url,
            "sitePolicyId": request.site_policy_id,
            "challengeId": request.challenge_id,
            "readinessProfileId": request.readiness_profile_id,
            "browserBuild": request.browser_build,
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
        "browserCapabilityEvidence": browser_capability_evidence,
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

    if let Some(site_policy_id) = request
        .target_url
        .as_deref()
        .and_then(|url| service_site_policy_id_for_url(service_state, url))
    {
        if let Some(site_policy) = service_state.site_policies.get(&site_policy_id) {
            return Some(selected_source_for_state_policy(
                service_state,
                site_policy.clone(),
                "target_url",
            ));
        }
        if let Some(site_policy) = builtin_site_policy(&site_policy_id) {
            return Some(SelectedSitePolicy {
                policy: site_policy,
                source: ServiceEntitySource::Builtin,
                matched_by: "target_url",
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

fn browser_capability_evidence_for_access_plan(
    service_state: &ServiceState,
    request: &ServiceAccessPlanRequest,
    selected_profile: Option<&BrowserProfile>,
    site_policy: Option<&SitePolicy>,
) -> Value {
    let registry = &service_state.browser_capability_registry;
    let browser_build =
        browser_build_for_evidence(service_state, request, selected_profile, site_policy);
    let browser_build_label = browser_build.map(browser_build_label);
    let selected_profile_id = selected_profile.map(|profile| profile.id.clone());
    let selected_preference_binding =
        preferred_registry_binding_for_access_request(registry, request, browser_build_label);
    let registry_routing_applied = selected_preference_binding.is_some()
        && !request.browser_build_explicit
        && site_policy
            .and_then(|policy| policy.browser_build)
            .is_none()
        && selected_profile
            .and_then(|profile| profile.browser_build)
            .is_none();

    let matching_preference_bindings = registry
        .browser_preference_bindings
        .iter()
        .filter(|binding| {
            preference_binding_matches_access_request(binding, request, browser_build_label)
        })
        .cloned()
        .collect::<Vec<_>>();
    let matching_executables = registry
        .browser_executables
        .iter()
        .filter(|executable| {
            browser_build_label.is_none_or(|label| {
                string_field(executable, "buildLabel").is_some_and(|build| build == label)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let executable_ids = matching_executables
        .iter()
        .filter_map(|executable| string_field(executable, "id"))
        .collect::<BTreeSet<_>>();
    let capability_ids_from_bindings = matching_preference_bindings
        .iter()
        .filter_map(|binding| string_field(binding, "preferredCapabilityId"))
        .collect::<BTreeSet<_>>();
    let executable_ids_from_bindings = matching_preference_bindings
        .iter()
        .filter_map(|binding| string_field(binding, "preferredExecutableId"))
        .collect::<BTreeSet<_>>();

    let matching_capabilities = registry
        .browser_capabilities
        .iter()
        .filter(|capability| {
            string_field(capability, "executableId").is_some_and(|id| {
                executable_ids.contains(&id) || executable_ids_from_bindings.contains(&id)
            }) || string_field(capability, "id")
                .is_some_and(|id| capability_ids_from_bindings.contains(&id))
        })
        .cloned()
        .collect::<Vec<_>>();
    let capability_ids = matching_capabilities
        .iter()
        .filter_map(|capability| string_field(capability, "id"))
        .collect::<BTreeSet<_>>();
    let host_ids = matching_executables
        .iter()
        .chain(matching_capabilities.iter())
        .chain(matching_preference_bindings.iter())
        .filter_map(|record| {
            string_field(record, "hostId").or_else(|| string_field(record, "preferredHostId"))
        })
        .collect::<BTreeSet<_>>();
    let matching_hosts = registry
        .browser_hosts
        .iter()
        .filter(|host| string_field(host, "id").is_some_and(|id| host_ids.contains(&id)))
        .cloned()
        .collect::<Vec<_>>();
    let matching_profile_compatibility = registry
        .profile_compatibility
        .iter()
        .filter(|compatibility| {
            selected_profile_id.as_ref().is_some_and(|profile_id| {
                string_field(compatibility, "profileId")
                    .is_some_and(|candidate| candidate == *profile_id)
            }) || string_field(compatibility, "hostId").is_some_and(|id| host_ids.contains(&id))
                || string_field(compatibility, "executableId").is_some_and(|id| {
                    executable_ids.contains(&id) || executable_ids_from_bindings.contains(&id)
                })
        })
        .cloned()
        .collect::<Vec<_>>();
    let matching_validation_evidence = registry
        .validation_evidence
        .iter()
        .filter(|evidence| {
            string_field(evidence, "hostId").is_some_and(|id| host_ids.contains(&id))
                || string_field(evidence, "executableId").is_some_and(|id| {
                    executable_ids.contains(&id) || executable_ids_from_bindings.contains(&id)
                })
                || string_field(evidence, "capabilityId")
                    .is_some_and(|id| capability_ids.contains(&id))
        })
        .cloned()
        .collect::<Vec<_>>();

    json!({
        "advisory": true,
        "routingApplied": registry_routing_applied,
        "routingScope": if registry_routing_applied {
            "access_plan_recommendation"
        } else {
            "none"
        },
        "source": "service.browserCapabilityRegistry",
        "browserBuild": browser_build,
        "browserBuildLabel": browser_build_label,
        "selectedProfileId": selected_profile_id,
        "selectedPreferenceBinding": selected_preference_binding,
        "targetServiceIds": request.target_service_ids.clone(),
        "accountIds": request.account_ids.clone(),
        "serviceName": request.service_name.clone(),
        "taskName": request.task_name.clone(),
        "generatedAt": registry.generated_at.clone(),
        "browserHosts": matching_hosts,
        "browserExecutables": matching_executables,
        "browserCapabilities": matching_capabilities,
        "profileCompatibility": matching_profile_compatibility,
        "browserPreferenceBindings": matching_preference_bindings,
        "validationEvidence": matching_validation_evidence,
        "counts": {
            "browserHosts": matching_hosts.len(),
            "browserExecutables": matching_executables.len(),
            "browserCapabilities": matching_capabilities.len(),
            "profileCompatibility": matching_profile_compatibility.len(),
            "browserPreferenceBindings": matching_preference_bindings.len(),
            "validationEvidence": matching_validation_evidence.len(),
        },
        "notes": [
            "Registry preference bindings can influence access-plan browser build recommendations when no explicit, site-policy, or profile browser build has already won.",
            "The scheduler and browser launch path still consume the copied access-plan request; this registry is not a direct launch router yet.",
        ],
    })
}

fn browser_build_for_evidence(
    service_state: &ServiceState,
    request: &ServiceAccessPlanRequest,
    selected_profile: Option<&BrowserProfile>,
    site_policy: Option<&SitePolicy>,
) -> Option<BrowserBuild> {
    site_policy
        .and_then(|policy| policy.browser_build)
        .or_else(|| selected_profile.and_then(|profile| profile.browser_build))
        .or(request.browser_build)
        .or(service_state.default_browser_build)
}

fn browser_build_label(browser_build: BrowserBuild) -> &'static str {
    match browser_build {
        BrowserBuild::StockChrome => "stock_chrome",
        BrowserBuild::StealthcdpChromium => "stealthcdp_chromium",
        BrowserBuild::CdpFreeHeaded => "cdp_free_headed",
    }
}

fn preference_binding_matches_access_request(
    binding: &Value,
    request: &ServiceAccessPlanRequest,
    browser_build_label: Option<&str>,
) -> bool {
    let browser_build_matches = browser_build_label.is_none_or(|label| {
        string_field(binding, "browserBuild")
            .as_deref()
            .is_none_or(|build| build == label)
    });
    let identity_matches = string_field(binding, "scope").as_deref() == Some("global")
        || array_field_intersects(binding, "targetServiceIds", &request.target_service_ids)
        || array_field_intersects(binding, "accountIds", &request.account_ids)
        || request.service_name.as_ref().is_some_and(|service_name| {
            array_field_contains(binding, "serviceNames", service_name)
        })
        || request
            .task_name
            .as_ref()
            .is_some_and(|task_name| array_field_contains(binding, "taskNames", task_name));
    browser_build_matches && identity_matches
}

fn preferred_registry_binding_for_access_request(
    registry: &super::service_model::BrowserCapabilityRegistry,
    request: &ServiceAccessPlanRequest,
    browser_build_label: Option<&str>,
) -> Option<Value> {
    registry
        .browser_preference_bindings
        .iter()
        .filter(|binding| {
            preference_binding_matches_access_request(binding, request, browser_build_label)
        })
        .max_by(|left, right| {
            preference_binding_rank(left, request).cmp(&preference_binding_rank(right, request))
        })
        .cloned()
}

fn preference_binding_rank(
    binding: &Value,
    request: &ServiceAccessPlanRequest,
) -> (i64, i64, String) {
    let priority = binding
        .get("priority")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let specificity = i64::from(array_field_intersects(
        binding,
        "accountIds",
        &request.account_ids,
    )) * 16
        + i64::from(array_field_intersects(
            binding,
            "targetServiceIds",
            &request.target_service_ids,
        )) * 8
        + i64::from(request.service_name.as_ref().is_some_and(|service_name| {
            array_field_contains(binding, "serviceNames", service_name)
        })) * 4
        + i64::from(
            request
                .task_name
                .as_ref()
                .is_some_and(|task_name| array_field_contains(binding, "taskNames", task_name)),
        ) * 2
        + i64::from(string_field(binding, "scope").as_deref() != Some("global"));
    let id = string_field(binding, "id").unwrap_or_default();
    (priority, specificity, id)
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn array_field_contains(value: &Value, field: &str, expected: &str) -> bool {
    value
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .any(|candidate| candidate == expected)
        })
}

fn array_field_intersects(value: &Value, field: &str, expected: &[String]) -> bool {
    !expected.is_empty()
        && value
            .get(field)
            .and_then(Value::as_array)
            .is_some_and(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|candidate| expected.iter().any(|item| item == candidate))
            })
}

struct AccessPlanDecisionInput<'a> {
    request: &'a ServiceAccessPlanRequest,
    selected_profile: Option<&'a BrowserProfile>,
    service_state: &'a ServiceState,
    site_policy: Option<&'a SitePolicy>,
    challenges: &'a [Challenge],
    providers: &'a [ServiceProvider],
    readiness: Option<&'a Value>,
    target_service_ids: &'a [String],
    readiness_summary: &'a Value,
    monitor_findings: &'a Value,
    naming_warnings: &'a [&'static str],
    browser_capability_evidence: &'a Value,
}

fn access_plan_decision(input: AccessPlanDecisionInput<'_>) -> Value {
    let request = input.request;
    let selected_profile = input.selected_profile;
    let service_state = input.service_state;
    let site_policy = input.site_policy;
    let challenges = input.challenges;
    let providers = input.providers;
    let readiness = input.readiness;
    let target_service_ids = input.target_service_ids;
    let readiness_summary = input.readiness_summary;
    let monitor_findings = input.monitor_findings;
    let naming_warnings = input.naming_warnings;
    let browser_capability_evidence = input.browser_capability_evidence;
    let mut reasons = Vec::new();
    let manual_seeding_required =
        readiness_summary["manualSeedingRequired"].as_bool() == Some(true);
    let profile_readiness_monitor_attention =
        monitor_findings["profileReadinessAttentionRequired"].as_bool() == Some(true);
    let profile_readiness_probe_due =
        monitor_findings["profileReadinessProbeDue"].as_bool() == Some(true);
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
    let launch_posture = launch_posture_decision(
        request,
        service_state,
        selected_profile,
        site_policy,
        manual_seeding_required,
        browser_capability_evidence,
    );
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
    if profile_readiness_probe_due {
        reasons.push("profile_readiness_probe_due");
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
    } else if profile_readiness_probe_due {
        "run_due_profile_readiness_monitor"
    } else if readiness_profile_needs_probe(readiness, target_service_ids) {
        "verify_or_seed_profile_before_authenticated_work"
    } else {
        "use_selected_profile"
    };
    let service_request = service_request_decision(
        input.request,
        selected_profile,
        policy_denies || denied_challenge,
        manual_seeding_required,
        manual_action_required,
        &launch_posture.value,
    );
    let post_seeding_probe = post_seeding_probe_decision(
        input.request,
        selected_profile,
        target_service_ids,
        manual_seeding_required || readiness_profile_needs_probe(readiness, target_service_ids),
    );
    let monitor_run_due =
        monitor_run_due_decision(input.request, monitor_findings, profile_readiness_probe_due);
    let attention = attention_decision(recommended_action);

    json!({
        "recommendedAction": recommended_action,
        "attention": attention,
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
        "monitorProbeDue": profile_readiness_probe_due,
        "providerIds": providers.iter().map(|provider| provider.id.clone()).collect::<Vec<_>>(),
        "authProviderIds": provider_decision.auth_provider_ids,
        "challengeProviderIds": provider_decision.challenge_provider_ids,
        "missingChallengeCapabilities": provider_decision.missing_challenge_capabilities,
        "challengeStrategy": provider_decision.challenge_strategy,
        "challengeIds": challenges.iter().map(|challenge| challenge.id.clone()).collect::<Vec<_>>(),
        "freshnessUpdate": freshness_update,
        "postSeedingProbe": post_seeding_probe,
        "monitorRunDue": monitor_run_due,
        "serviceRequest": service_request,
        "namingWarnings": naming_warnings,
        "hasNamingWarning": !naming_warnings.is_empty(),
        "reasons": reasons,
    })
}

/// Summarize who should act next without prescribing a UI presentation.
fn attention_decision(recommended_action: &str) -> Value {
    let (required, owner, severity, title, message, suggested_actions) = match recommended_action {
        "deny_request_by_site_policy" => (
            true,
            "operator",
            "blocking",
            "Request denied by site policy",
            "The selected site policy or retained challenge denies this browser request.",
            vec!["review_site_policy", "resolve_or_acknowledge_challenge"],
        ),
        "seed_profile_before_authenticated_work"
        | "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable" => (
            true,
            "operator",
            "blocking",
            "Profile needs detached seeding",
            "Launch the profile without CDP, complete sign-in or setup, close the browser, then run the post-seeding probe.",
            vec!["launch_detached_seeding", "close_seeded_browser", "run_post_seeding_probe"],
        ),
        "request_manual_challenge_approval" | "manual_intervention_required" => (
            true,
            "operator",
            "blocking",
            "Manual challenge intervention required",
            "A retained challenge requires human approval or manual recovery before browser work should continue.",
            vec!["inspect_challenge", "approve_or_resolve_challenge"],
        ),
        "wait_for_or_invoke_challenge_provider" => (
            true,
            "provider",
            "warning",
            "Challenge provider should act",
            "A retained challenge is waiting for an enabled provider or provider-backed workflow.",
            vec!["invoke_challenge_provider", "poll_challenge_state"],
        ),
        "register_or_seed_managed_profile" => (
            true,
            "operator",
            "blocking",
            "Managed profile required",
            "The selected site policy requires a managed profile, but no matching profile is registered.",
            vec!["register_managed_profile", "seed_profile_if_needed"],
        ),
        "register_managed_profile_or_request_throwaway_browser" => (
            true,
            "client",
            "warning",
            "No matching profile selected",
            "No matching managed profile was found; the caller should register one or explicitly request throwaway browser behavior.",
            vec!["register_managed_profile", "request_throwaway_browser"],
        ),
        "probe_target_auth_or_reseed_if_needed" | "verify_or_seed_profile_before_authenticated_work" => (
            true,
            "service",
            "warning",
            "Profile freshness needs verification",
            "Run a bounded auth probe for the selected target identity before relying on authenticated automation.",
            vec!["run_bounded_auth_probe", "update_profile_freshness", "seed_profile_if_probe_fails"],
        ),
        "run_due_profile_readiness_monitor" => (
            true,
            "service",
            "warning",
            "Profile-readiness monitor is due",
            "Run the due profile-readiness monitor before trusting retained profile freshness.",
            vec!["run_due_profile_readiness_monitor", "inspect_monitor_result"],
        ),
        _ => (
            false,
            "none",
            "info",
            "No intervention required",
            "The selected profile and policy are ready for the recommended service request path.",
            vec!["request_service_tab"],
        ),
    };

    json!({
        "required": required,
        "owner": owner,
        "severity": severity,
        "reason": recommended_action,
        "title": title,
        "message": message,
        "suggestedActions": suggested_actions,
        "presentation": "client_decides",
    })
}

/// Describe the queued service-owned monitor execution path for due monitors.
fn monitor_run_due_decision(
    request: &ServiceAccessPlanRequest,
    monitor_findings: &Value,
    recommended_before_use: bool,
) -> Value {
    let due_monitor_ids = string_array_from_value(
        monitor_findings
            .get("profileReadinessDueMonitorIds")
            .unwrap_or(&Value::Null),
    );
    let never_checked_monitor_ids = string_array_from_value(
        monitor_findings
            .get("profileReadinessNeverCheckedMonitorIds")
            .unwrap_or(&Value::Null),
    );
    let due_target_service_ids = string_array_from_value(
        monitor_findings
            .get("dueTargetServiceIds")
            .unwrap_or(&Value::Null),
    );
    let available = !due_monitor_ids.is_empty();

    json!({
        "available": available,
        "recommendedBeforeUse": recommended_before_use && available,
        "monitorIds": due_monitor_ids,
        "neverCheckedMonitorIds": never_checked_monitor_ids,
        "targetServiceIds": due_target_service_ids,
        "http": {
            "method": "POST",
            "route": "/api/service/monitors/run-due",
        },
        "mcp": {
            "tool": "service_monitors_run_due",
        },
        "client": {
            "package": "@agent-browser/client/service-observability",
            "helper": "runServiceAccessPlanMonitorRunDue",
        },
        "fallbackClient": {
            "package": "@agent-browser/client/service-observability",
            "helper": "runDueServiceMonitors",
        },
        "cli": {
            "command": "agent-browser service monitors run-due",
        },
        "requestFields": [],
        "notes": [
            "Runs all due active monitors through the service worker queue.",
            "Inspect monitorIds after completion to confirm the requested target freshness changed as expected.",
        ],
        "query": {
            "serviceName": request.service_name.as_ref(),
            "agentName": request.agent_name.as_ref(),
            "taskName": request.task_name.as_ref(),
        },
    })
}

fn string_array_from_value(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Describe the bounded post-close profile seeding verification recipe.
fn post_seeding_probe_decision(
    request: &ServiceAccessPlanRequest,
    selected_profile: Option<&BrowserProfile>,
    target_service_ids: &[String],
    recommended_after_close: bool,
) -> Value {
    let profile_id = selected_profile.map(|profile| profile.id.clone());
    let target_service_id = target_service_ids.first().cloned();
    let available = profile_id.is_some() && target_service_id.is_some();
    let helper = "verifyServiceProfileSeeding";
    let example_script = "examples/service-client/post-seeding-probe.mjs";
    let cli_command = match (profile_id.as_ref(), target_service_id.as_ref()) {
        (Some(profile_id), Some(target_service_id)) => Some(format!(
            "agent-browser service profiles {profile_id} verify-seeding {target_service_id} --state fresh --evidence <probe-evidence>"
        )),
        _ => None,
    };
    let example_command = match (profile_id.as_ref(), target_service_id.as_ref()) {
        (Some(profile_id), Some(target_service_id)) => Some(format!(
            "pnpm --filter agent-browser-service-client-example exec node {example_script} --base-url http://127.0.0.1:<stream-port> --profile-id {profile_id} --target-service-id {target_service_id}"
        )),
        _ => None,
    };

    json!({
        "available": available,
        "recommendedAfterClose": recommended_after_close && available,
        "profileId": profile_id,
        "targetServiceId": target_service_id,
        "targetServiceIds": target_service_ids,
        "boundedChecks": [
            "broker_selected_profile_matches_profile_id",
            "url_read",
            "title_read",
            "optional_expected_url_fragment",
            "optional_expected_title_fragment",
        ],
        "http": {
            "method": "POST",
            "route": profile_id
                .as_ref()
                .map(|profile_id| format!("/api/service/profiles/{profile_id}/freshness")),
            "routeTemplate": "/api/service/profiles/<id>/freshness",
        },
        "mcp": {
            "tool": "service_profile_freshness_update",
        },
        "client": {
            "package": "@agent-browser/client/service-observability",
            "helper": helper,
        },
        "serviceClientExample": {
            "package": "agent-browser-service-client-example",
            "script": example_script,
            "command": example_command,
        },
        "cli": {
            "command": cli_command,
        },
        "requestFields": [
            "profileId",
            "targetServiceId",
            "readinessState",
            "readinessEvidence",
            "lastVerifiedAt",
            "freshnessExpiresAt",
        ],
        "notes": [
            "Run only after detached CDP-free seeding has closed.",
            "The probe must verify the same broker-selected profile before recording freshness.",
        ],
        "query": {
            "serviceName": request.service_name.as_ref(),
            "agentName": request.agent_name.as_ref(),
            "taskName": request.task_name.as_ref(),
        },
    })
}

/// Describe the queued browser-control handoff clients should use after planning.
fn service_request_decision(
    request: &ServiceAccessPlanRequest,
    selected_profile: Option<&BrowserProfile>,
    denied: bool,
    manual_seeding_required: bool,
    manual_action_required: bool,
    launch_posture: &Value,
) -> Value {
    let selected_profile_id = selected_profile.map(|profile| profile.id.clone());
    let requires_cdp_free = launch_posture
        .get("requiresCdpFree")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let cdp_attachment_allowed = launch_posture
        .get("cdpAttachmentAllowed")
        .and_then(Value::as_bool)
        .unwrap_or(!requires_cdp_free);
    let blocked_by_cdp_free = requires_cdp_free && !cdp_attachment_allowed;
    let available =
        selected_profile_id.is_some() && !denied && !manual_action_required && !blocked_by_cdp_free;
    let recommended_after_manual_action =
        selected_profile_id.is_some() && !denied && manual_action_required && !blocked_by_cdp_free;
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
    if !request.account_ids.is_empty() {
        service_request.insert("accountIds".to_string(), json!(request.account_ids));
    }
    if let Some(target_url) = request.target_url.as_ref() {
        service_request.insert("url".to_string(), json!(target_url));
    }
    if let Some(browser_build) = launch_posture.get("browserBuild") {
        service_request.insert("browserBuild".to_string(), browser_build.clone());
    }
    if manual_action_required {
        service_request.insert("blockedByManualAction".to_string(), json!(true));
    }
    if manual_seeding_required {
        service_request.insert("manualSeedingRequired".to_string(), json!(true));
    }
    if requires_cdp_free {
        service_request.insert("requiresCdpFree".to_string(), json!(true));
    }
    service_request.insert(
        "cdpAttachmentAllowed".to_string(),
        json!(cdp_attachment_allowed),
    );
    service_request.insert("profileLeasePolicy".to_string(), json!("wait"));

    json!({
        "available": available,
        "recommendedAfterManualAction": recommended_after_manual_action,
        "blockedByManualAction": manual_action_required,
        "blockedByCdpFree": blocked_by_cdp_free,
        "blockedByPolicy": denied,
        "requiresCdpFree": requires_cdp_free,
        "cdpAttachmentAllowed": cdp_attachment_allowed,
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
            "accountIds",
            "browserBuild",
            "profileLeasePolicy",
            "requiresCdpFree",
            "cdpAttachmentAllowed",
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
    request: &ServiceAccessPlanRequest,
    service_state: &ServiceState,
    selected_profile: Option<&BrowserProfile>,
    site_policy: Option<&SitePolicy>,
    manual_seeding_required: bool,
    browser_capability_evidence: &Value,
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
    let requires_cdp_free = site_policy
        .map(|policy| policy.requires_cdp_free)
        .unwrap_or(false);
    let (browser_build, browser_build_source) = browser_build_decision(
        request,
        service_state,
        selected_profile,
        site_policy,
        requires_cdp_free,
        browser_capability_evidence,
    );
    let cdp_attachment_allowed = !requires_cdp_free && !manual_seeding_required;
    let attachable_after_seeding = cdp_attachment_allowed
        || (!requires_cdp_free && !matches!(browser_host, BrowserHost::AttachedExisting));
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
    if requires_cdp_free {
        rationale.push("site_policy_requires_cdp_free");
    } else if cdp_attachment_allowed {
        rationale.push("cdp_attachment_allowed");
    } else {
        rationale.push("cdp_attachment_blocked_until_manual_action_complete");
    }
    match browser_build {
        BrowserBuild::StockChrome => rationale.push("browser_build_stock_chrome"),
        BrowserBuild::StealthcdpChromium => rationale.push("browser_build_stealthcdp_chromium"),
        BrowserBuild::CdpFreeHeaded => rationale.push("browser_build_cdp_free_headed"),
    }
    if browser_build_source == "browser_preference_binding" {
        rationale.push("browser_build_from_browser_preference_binding");
    }
    if browser_build_source == "request" {
        rationale.push("browser_build_from_request");
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
            "browserBuild": browser_build,
            "browserBuildSource": browser_build_source,
            "source": source,
            "headed": headed,
            "remoteViewRecommended": remote_view_recommended,
            "requiresCdpFree": requires_cdp_free,
            "cdpAttachmentAllowed": cdp_attachment_allowed,
            "detachedFirstLoginRequired": manual_seeding_required,
            "attachableAfterSeeding": attachable_after_seeding,
            "rationale": rationale,
        }),
    }
}

fn browser_build_decision(
    request: &ServiceAccessPlanRequest,
    service_state: &ServiceState,
    selected_profile: Option<&BrowserProfile>,
    site_policy: Option<&SitePolicy>,
    requires_cdp_free: bool,
    browser_capability_evidence: &Value,
) -> (BrowserBuild, &'static str) {
    if requires_cdp_free {
        return (BrowserBuild::CdpFreeHeaded, "requires_cdp_free");
    }
    if request.browser_build_explicit {
        if let Some(browser_build) = request.browser_build {
            return (browser_build, "request");
        }
    }
    if let Some(browser_build) = site_policy.and_then(|policy| policy.browser_build) {
        return (browser_build, "site_policy");
    }
    if let Some(browser_build) = selected_profile.and_then(|profile| profile.browser_build) {
        return (browser_build, "profile_default");
    }
    if let Some(browser_build) =
        browser_build_from_selected_preference_binding(browser_capability_evidence)
    {
        return (browser_build, "browser_preference_binding");
    }
    if let Some(browser_build) = service_state.default_browser_build {
        return (browser_build, "service_default");
    }
    (BrowserBuild::StockChrome, "service_default")
}

fn browser_build_from_selected_preference_binding(
    browser_capability_evidence: &Value,
) -> Option<BrowserBuild> {
    browser_capability_evidence
        .get("selectedPreferenceBinding")
        .and_then(|binding| string_field(binding, "browserBuild"))
        .and_then(|label| BrowserBuild::parse_label(&label))
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
        ProfileSelectionReason::AccountMatch => (
            Some("accountIds"),
            first_matching_identity(&request.account_ids, &profile.account_ids),
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
        ProfileSelectionReason::BrowserBuildDefault => (
            Some("browserBuild"),
            request.browser_build.map(|browser_build| {
                serde_json::to_value(browser_build)
                    .ok()
                    .and_then(|value| value.as_str().map(ToString::to_string))
                    .unwrap_or_default()
            }),
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

fn parse_browser_build(value: &str) -> Result<Option<BrowserBuild>, String> {
    let Some(value) = non_empty(value.to_string()) else {
        return Ok(None);
    };
    BrowserBuild::parse_label(&value)
        .map(Some)
        .ok_or_else(|| format!("Unknown browserBuild value: {}", value))
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
    preferred_registry_binding_for_access_request(
        &service_state.browser_capability_registry,
        request,
        None,
    )
    .and_then(|binding| string_field(&binding, "browserBuild"))
    .and_then(|label| BrowserBuild::parse_label(&label))
    .or(service_state.default_browser_build)
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::native::service_model::{
        BrowserCapabilityRegistry, BrowserHost, BrowserProfile, Challenge, ChallengeKind,
        InteractionMode, MonitorState, MonitorTarget, ProfileKeyringPolicy, ProfileReadinessState,
        ProfileSeedingMode, ProfileTargetReadiness, ProviderCapability, ProviderKind,
        RateLimitPolicy, ServiceIncident, ServiceProvider, SiteMonitor, SitePolicy,
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
            ("login-id".to_string(), "acs".to_string()),
        ])
        .unwrap();

        assert_eq!(request.service_name.as_deref(), Some("JournalDownloader"));
        assert_eq!(request.agent_name.as_deref(), Some("codex"));
        assert_eq!(request.task_name.as_deref(), Some("probeACSwebsite"));
        assert_eq!(request.target_service_ids, vec!["acs".to_string()]);
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
            plan["browserCapabilityEvidence"]["counts"]["browserExecutables"],
            1
        );
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
            plan["decision"]["serviceRequest"]["request"]["browserBuild"],
            "stock_chrome"
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
            plan["decision"]["launchPosture"]["cdpAttachmentAllowed"],
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
        assert_eq!(plan["decision"]["launchPosture"]["requiresCdpFree"], false);
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
