//! No-launch access planning for service-owned browser/profile decisions.
//!
//! The access plan joins profile selection, site policy, provider, challenge,
//! and readiness state before a caller asks the service to launch or control a
//! browser. It is intentionally read-only so agents and software clients can
//! get the service recommendation without creating browser process pressure.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use super::service_contracts::SERVICE_REQUEST_ACTIONS;
use super::service_lifecycle::{select_service_profile_for_request, ProfileSelectionRequest};
use super::service_model::{
    browser_profile_compatibility_matches, builtin_site_policy, service_profile_seeding_handoff,
    service_site_policy_id_for_url, BrowserBuild, BrowserHealth, BrowserHost, BrowserProcess,
    BrowserProfile, BrowserSession, Challenge, ChallengeKind, ChallengePolicy, ChallengeState,
    ControlInputProvider, InteractionMode, LeaseState, ProfileOrigin, ProfileSelectionReason,
    ProviderCapability, ServiceEntitySource, ServiceIncidentEscalation, ServiceIncidentState,
    ServiceProvider, ServiceState, SitePolicy, ViewStreamProvider,
    SERVICE_JOB_NAMING_WARNING_MISSING_AGENT_NAME, SERVICE_JOB_NAMING_WARNING_MISSING_SERVICE_NAME,
    SERVICE_JOB_NAMING_WARNING_MISSING_TASK_NAME,
};

/// Parsed access-plan selector shared by HTTP and MCP resources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ServiceAccessPlanRequest {
    pub(crate) service_name: Option<String>,
    pub(crate) agent_name: Option<String>,
    pub(crate) task_name: Option<String>,
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
    let selected_profile = request
        .runtime_profile
        .as_deref()
        .and_then(|profile_id| service_state.profiles.get(profile_id))
        .cloned()
        .or_else(|| {
            selection
                .as_ref()
                .and_then(|selection| service_state.profiles.get(&selection.profile_id))
                .cloned()
        });
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
            "sessionName": request.session_name,
            "targetServiceIds": request.target_service_ids,
            "accountIds": request.account_ids,
            "url": request.target_url,
            "sitePolicyId": request.site_policy_id,
            "challengeId": request.challenge_id,
            "readinessProfileId": request.readiness_profile_id,
            "runtimeProfile": request.runtime_profile,
            "browserBuild": request.browser_build,
            "browserHost": request.browser_host,
            "viewStreamProvider": request.view_stream_provider,
            "controlInputProvider": request.control_input_provider,
            "displayIsolation": request.display_isolation,
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
            selected_profile_id.as_deref().is_some_and(|profile_id| {
                host_ids.iter().any(|host_id| {
                    executable_ids
                        .iter()
                        .chain(executable_ids_from_bindings.iter())
                        .any(|executable_id| {
                            browser_profile_compatibility_matches(
                                compatibility,
                                profile_id,
                                host_id,
                                executable_id,
                            )
                        })
                })
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
    let has_filters = array_field_has_items(binding, "targetServiceIds")
        || array_field_has_items(binding, "accountIds")
        || array_field_has_items(binding, "serviceNames")
        || array_field_has_items(binding, "taskNames");
    let identity_matches = string_field(binding, "scope").as_deref() == Some("global")
        && !has_filters
        || has_filters
            && binding_filter_matches(binding, "targetServiceIds", &request.target_service_ids)
            && binding_filter_matches(binding, "accountIds", &request.account_ids)
            && binding_optional_filter_matches(
                binding,
                "serviceNames",
                request.service_name.as_deref(),
            )
            && binding_optional_filter_matches(binding, "taskNames", request.task_name.as_deref());
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

fn array_field_has_items(value: &Value, field: &str) -> bool {
    value
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| item.as_str().is_some_and(|item| !item.is_empty()))
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

fn binding_filter_matches(value: &Value, field: &str, expected: &[String]) -> bool {
    !array_field_has_items(value, field) || array_field_intersects(value, field, expected)
}

fn binding_optional_filter_matches(value: &Value, field: &str, expected: Option<&str>) -> bool {
    !array_field_has_items(value, field)
        || expected.is_some_and(|expected| array_field_contains(value, field, expected))
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
    let lifecycle_replacement = lifecycle_replacement_decision(selected_profile, service_state);
    let mut profile_reuse = profile_reuse_decision(
        request,
        selected_profile,
        service_state,
        &launch_posture.value,
        manual_seeding_required,
        lifecycle_replacement
            .get("replacementSessionName")
            .and_then(Value::as_str),
    );
    let lifecycle_blocks_replacement = lifecycle_replacement["available"].as_bool() == Some(true)
        && lifecycle_replacement["replacementEligible"].as_bool() == Some(false);
    let acquisition_blocked_by_explicit_session =
        profile_reuse["recommendedAction"].as_str() == Some("blocked_by_explicit_session_route");
    let acquisition_blocked_by_lifecycle_owner = lifecycle_blocks_replacement
        && profile_reuse["recommendedAction"].as_str() == Some("launch_new_browser");
    if acquisition_blocked_by_lifecycle_owner {
        profile_reuse["recommendedAction"] = json!("blocked_by_lifecycle_owner");
        profile_reuse["defaultAcquisition"] = Value::Null;
        profile_reuse["sharedAcquisition"]["mode"] = Value::Null;
        profile_reuse["sharedAcquisition"]["requiresRouteHints"] = json!(false);
        profile_reuse["sharedAcquisition"]["routeHintFields"] = json!([]);
        if let Some(reasons) = profile_reuse["reasons"].as_array_mut() {
            reasons.push(json!("lifecycle_owner_blocks_replacement"));
            reasons.sort_by(|left, right| left.as_str().cmp(&right.as_str()));
            reasons.dedup();
        }
    }
    let acquisition_blocker = if acquisition_blocked_by_explicit_session {
        Some("explicit_session_route_invalid")
    } else if acquisition_blocked_by_lifecycle_owner {
        Some("lifecycle_owner_blocks_replacement")
    } else {
        None
    };
    let one_time_profile_recommendation =
        access_plan_one_time_profile_recommendation(request, selected_profile, service_state);
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
    if one_time_profile_recommendation
        .get("state")
        .and_then(Value::as_str)
        == Some("planned")
    {
        reasons.push("managed_one_time_profile_planned");
    }
    if one_time_profile_recommendation
        .get("state")
        .and_then(Value::as_str)
        == Some("warning")
    {
        reasons.push("operator_supplied_one_time_profile_warning");
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
    } else if acquisition_blocked_by_explicit_session {
        "resolve_explicit_session_route"
    } else if acquisition_blocked_by_lifecycle_owner {
        "reconcile_lifecycle_owner_for_tab_acquisition"
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
    let service_request = service_request_decision(ServiceRequestDecisionInput {
        request: input.request,
        selected_profile,
        denied: policy_denies || denied_challenge,
        manual_seeding_required,
        manual_action_required,
        launch_posture: &launch_posture.value,
        profile_reuse: &profile_reuse,
        lifecycle_replacement: &lifecycle_replacement,
        one_time_profile_recommendation: &one_time_profile_recommendation,
        acquisition_blocker,
    });
    let post_seeding_probe = post_seeding_probe_decision(
        input.request,
        selected_profile,
        target_service_ids,
        manual_seeding_required || readiness_profile_needs_probe(readiness, target_service_ids),
    );
    let monitor_run_due =
        monitor_run_due_decision(input.request, monitor_findings, profile_readiness_probe_due);
    let browser_capability_preflight = browser_capability_preflight_decision(
        input.request,
        selected_profile,
        &launch_posture.value,
        browser_capability_evidence,
    );
    let attention = attention_decision(recommended_action);

    json!({
        "recommendedAction": recommended_action,
        "attention": attention,
        "browserHost": launch_posture.browser_host,
        "launchPosture": launch_posture.value,
        "profileReuse": profile_reuse,
        "lifecycleReplacement": lifecycle_replacement,
        "oneTimeProfileRecommendation": one_time_profile_recommendation,
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
        "browserCapabilityPreflight": browser_capability_preflight,
        "serviceRequest": service_request,
        "namingWarnings": naming_warnings,
        "hasNamingWarning": !naming_warnings.is_empty(),
        "reasons": reasons,
    })
}

/// Project replacement authority and the exact collision-free daemon route
/// that can supersede one cleanup-satisfied terminal owner.
fn lifecycle_replacement_decision(
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
    let lifecycle = owner_lifecycle.or_else(|| records.last().copied());
    let terminal_cleanup_satisfied = lifecycle.is_some_and(|record| {
        record.lifecycle_state == crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Terminal
            && record.cleanup_obligation_state
                == crate::runtime_owner_transfer::CleanupObligationState::Satisfied
    });
    let replacement_route = owner.zip(owner_lifecycle).and_then(|(owner, record)| {
        let expected_browser_id = format!("session:{}", owner.daemon_session_route);
        (terminal_cleanup_satisfied
            && record.logical_browser_id == owner.browser_id
            && owner.browser_id == expected_browser_id)
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
        Some(_) if terminal_cleanup_satisfied => "terminal_replacement_route_inconsistent",
        Some(record)
            if record.lifecycle_state
                == crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Closing =>
        {
            "closing_lifecycle_requires_reconciliation"
        }
        Some(_) => "lifecycle_owner_blocks_replacement",
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
        "terminalEvidence": lifecycle.map(|record| record.terminal_evidence.clone()).unwrap_or_default(),
        "replacementEligible": replacement_eligible,
        "reason": reason,
        "requiredAction": required_action,
    })
}

/// Apply access-plan shared-profile route hints to tab-opening service requests.
///
/// HTTP and MCP adapters both call this before relay so the access planner stays
/// authoritative for selecting the compatible retained browser.
pub(crate) const SERVICE_REQUEST_ACCESS_PLAN_ROUTING_FIELDS: &[&str] = &[
    "serviceName",
    "agentName",
    "taskName",
    "sessionName",
    "targetServiceId",
    "targetService",
    "targetServiceIds",
    "targetServices",
    "siteId",
    "siteIds",
    "loginId",
    "loginIds",
    "accountId",
    "accountIds",
    "runtimeProfile",
    "profileId",
    "browserBuild",
    "browserHost",
    "viewStreamProvider",
    "controlInputProvider",
    "displayIsolation",
];

pub(crate) fn apply_shared_profile_route_hints_for_service_request(
    service_state: &ServiceState,
    command: &mut Value,
) -> Result<(), String> {
    if command.get("action").and_then(Value::as_str) != Some("tab_new") {
        return Ok(());
    }
    if service_request_has_complete_route_hints(command)
        || command
            .get("allowDuplicateProfileLane")
            .and_then(Value::as_bool)
            == Some(true)
    {
        return Ok(());
    }
    if service_request_has_browser_hint(command) && !service_request_has_session_hint(command) {
        return Err("service_access_plan_incomplete_route_hints".to_string());
    }

    let request = service_access_plan_request_from_service_command(command)?;
    let plan = service_access_plan_for_state(service_state, request);
    if plan["decision"]["serviceRequest"]["available"].as_bool() != Some(true) {
        let blocker = plan["decision"]["serviceRequest"]["acquisitionBlocker"]
            .as_str()
            .unwrap_or("service_request_unavailable");
        return Err(format!("service_access_plan_request_unavailable:{blocker}"));
    }
    let profile_reuse = &plan["decision"]["profileReuse"];
    if profile_reuse
        .get("recommendedAction")
        .and_then(Value::as_str)
        != Some("reuse_existing_browser")
    {
        if service_request_has_partial_route_hints(command) {
            let planned_terminal_session = plan["decision"]["lifecycleReplacement"]
                .get("replacementSessionName")
                .and_then(Value::as_str);
            let requested_session = command.get("sessionName").and_then(Value::as_str);
            let exact_terminal_replacement = !service_request_has_browser_hint(command)
                && plan["decision"]["lifecycleReplacement"]["replacementEligible"].as_bool()
                    == Some(true)
                && requested_session == planned_terminal_session;
            if !exact_terminal_replacement {
                return Err("service_access_plan_incomplete_route_hints".to_string());
            }
            return Ok(());
        }
        if !service_request_has_session_hint(command) {
            if let Some(session_name) = plan["decision"]["serviceRequest"]["request"]
                .get("sessionName")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
            {
                command["sessionName"] = json!(session_name);
            }
        }
        return Ok(());
    }

    let Some(browser_id) = profile_reuse
        .get("reusableBrowserId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return Err("service_access_plan_reuse_missing_browser_id".to_string());
    };
    let Some(session_name) = profile_reuse
        .get("reusableSessionName")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return Err("service_access_plan_reuse_missing_session_name".to_string());
    };

    command["browserId"] = json!(browser_id);
    command["sessionName"] = json!(session_name);
    Ok(())
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

fn service_request_has_complete_route_hints(command: &Value) -> bool {
    service_request_route_hint_count(command) == 2
}

fn service_request_has_partial_route_hints(command: &Value) -> bool {
    service_request_route_hint_count(command) == 1
}

fn service_access_plan_request_from_service_command(
    command: &Value,
) -> Result<ServiceAccessPlanRequest, String> {
    let mut params = Vec::new();
    for key in SERVICE_REQUEST_ACCESS_PLAN_ROUTING_FIELDS {
        append_service_command_access_param(&mut params, key, command.get(key));
    }
    let target_url = command.get("url").or_else(|| command.get("desiredUrl"));
    append_service_command_access_param(&mut params, "url", target_url);
    parse_service_access_plan_query(params)
}

fn append_service_command_access_param(
    params: &mut Vec<(String, String)>,
    key: &str,
    value: Option<&Value>,
) {
    let Some(value) = value else {
        return;
    };
    if let Some(value) = value.as_str().filter(|value| !value.trim().is_empty()) {
        params.push((key.to_string(), value.to_string()));
        return;
    }
    if let Some(items) = value.as_array() {
        let value = items
            .iter()
            .filter_map(Value::as_str)
            .filter(|item| !item.trim().is_empty())
            .collect::<Vec<_>>()
            .join(",");
        if !value.is_empty() {
            params.push((key.to_string(), value));
        }
    }
}

fn access_plan_one_time_profile_recommendation(
    request: &ServiceAccessPlanRequest,
    selected_profile: Option<&BrowserProfile>,
    service_state: &ServiceState,
) -> Value {
    if !access_plan_looks_like_one_time_operator_handoff(request) {
        return Value::Null;
    }
    if selected_profile.is_some() {
        return Value::Null;
    }
    let recommended_profile_id = access_plan_managed_one_time_profile_id(request);
    if let Some(runtime_profile) = request.runtime_profile.as_deref() {
        if service_state.profiles.contains_key(runtime_profile) {
            return Value::Null;
        }
        return json!({
            "state": "warning",
            "code": "arbitrary_runtime_profile_for_one_time_handoff",
            "requestedRuntimeProfile": runtime_profile,
            "profileClass": "operator_supplied",
            "recommendedProfileClass": "managed_one_time",
            "recommendedProfileId": recommended_profile_id,
            "runtimeProfile": runtime_profile,
            "message": "This access plan looks like a one-time operator handoff but it supplied an unknown runtime profile. Prefer the managed one-time task profile so retries reuse one lane and cleanup can remove abandoned task state safely.",
        });
    }
    json!({
        "state": "planned",
        "code": "managed_one_time_profile_planned",
        "profileClass": "managed_one_time",
        "profileOrigin": "agent_browser_owned",
        "recommendedProfileId": recommended_profile_id,
        "runtimeProfile": recommended_profile_id,
        "persistent": false,
        "message": "This access plan looks like a one-time operator handoff and no durable profile was selected, so the generated service request uses a deterministic managed one-time task profile.",
    })
}

fn access_plan_looks_like_one_time_operator_handoff(request: &ServiceAccessPlanRequest) -> bool {
    if request.view_stream_provider != Some(ViewStreamProvider::RdpGateway) {
        return false;
    }
    if request.control_input_provider != Some(ControlInputProvider::ManualAttachedDesktop) {
        return false;
    }
    if request.browser_host != Some(BrowserHost::RemoteHeaded) {
        return false;
    }
    let text = [
        request.service_name.as_deref(),
        request.agent_name.as_deref(),
        request.task_name.as_deref(),
        request.target_url.as_deref(),
    ]
    .into_iter()
    .flatten()
    .chain(request.target_service_ids.iter().map(String::as_str))
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase();
    [
        "temporary",
        "temp",
        "one-time",
        "one_time",
        "login",
        "payment",
        "challenge",
        "sosdirect",
        "templogin",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn access_plan_managed_one_time_profile_id(request: &ServiceAccessPlanRequest) -> String {
    let seed = [
        request.service_name.as_deref().unwrap_or("service"),
        request.agent_name.as_deref().unwrap_or("agent"),
        request.task_name.as_deref().unwrap_or("task"),
        request.target_url.as_deref().unwrap_or("url"),
    ]
    .join("|")
    .to_ascii_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();
    let suffix = digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("managed-one-time-{suffix}")
}

fn profile_reuse_decision(
    request: &ServiceAccessPlanRequest,
    selected_profile: Option<&BrowserProfile>,
    service_state: &ServiceState,
    launch_posture: &Value,
    manual_seeding_required: bool,
    terminal_replacement_session_name: Option<&str>,
) -> Value {
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

    let mut explicit_session_route_error = None;
    let mut explicit_session_route = None;
    let mut explicit_terminal_replacement_route = false;
    if let Some(session_name) = request.session_name.as_deref() {
        match service_state.sessions.get(session_name) {
            None if terminal_replacement_session_name == Some(session_name) => {
                explicit_terminal_replacement_route = true;
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
    }
    reasons.sort();
    reasons.dedup();

    let recommended_action = if manual_seeding_required {
        "seed_profile_before_reuse"
    } else if explicit_session_route_error.is_some() {
        "blocked_by_explicit_session_route"
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
        "activeLeaseCount": active_lease_session_ids.len(),
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
    )
}

fn session_blocks_profile_reuse(session: &BrowserSession, profile_id: &str) -> bool {
    session.profile_id.as_deref() == Some(profile_id)
        && matches!(
            session.lease,
            LeaseState::Exclusive | LeaseState::HumanTakeover
        )
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
        "reconcile_lifecycle_owner_for_tab_acquisition" => (
            true,
            "service",
            "blocking",
            "Existing lifecycle owner requires reconciliation",
            "The retained profile has a live lifecycle owner that cannot accept this tab request, so replacement launch is blocked.",
            vec!["inspect_lifecycle_owner", "reconcile_existing_browser_route"],
        ),
        "resolve_explicit_session_route" => (
            true,
            "client",
            "blocking",
            "Explicit session route is invalid",
            "The requested session does not map uniquely to one compatible retained browser.",
            vec!["inspect_service_sessions", "select_unique_session_route"],
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

/// Describe the no-launch browser capability preflight clients can run before browser work.
fn browser_capability_preflight_decision(
    request: &ServiceAccessPlanRequest,
    selected_profile: Option<&BrowserProfile>,
    launch_posture: &Value,
    browser_capability_evidence: &Value,
) -> Value {
    let browser_build = launch_posture
        .get("browserBuild")
        .cloned()
        .unwrap_or(Value::Null);
    let browser_build_label = browser_build.as_str().map(ToString::to_string);
    let selected_profile_id = selected_profile.map(|profile| profile.id.clone());
    let headed = launch_posture
        .get("headed")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let requires_cdp_free = launch_posture
        .get("requiresCdpFree")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let cdp_attachment_allowed = launch_posture
        .get("cdpAttachmentAllowed")
        .and_then(Value::as_bool)
        .unwrap_or(!requires_cdp_free);
    let evidence_available = browser_capability_evidence
        .get("counts")
        .and_then(Value::as_object)
        .map(|counts| {
            [
                "browserExecutables",
                "profileCompatibility",
                "validationEvidence",
            ]
            .iter()
            .any(|key| counts.get(*key).and_then(Value::as_u64).unwrap_or(0) > 0)
        })
        .unwrap_or(false)
        || browser_capability_evidence
            .get("selectedPreferenceBinding")
            .is_some_and(|value| value.is_object());
    let available = browser_build_label.is_some();
    let recommended_before_use = available && evidence_available;
    let mut preflight_request = Map::new();

    if let Some(browser_build_label) = browser_build_label.as_ref() {
        preflight_request.insert("browserBuild".to_string(), json!(browser_build_label));
    }
    if let Some(service_name) = request.service_name.as_ref() {
        preflight_request.insert("serviceName".to_string(), json!(service_name));
    }
    if let Some(agent_name) = request.agent_name.as_ref() {
        preflight_request.insert("agentName".to_string(), json!(agent_name));
    }
    if let Some(task_name) = request.task_name.as_ref() {
        preflight_request.insert("taskName".to_string(), json!(task_name));
    }
    if !request.target_service_ids.is_empty() {
        preflight_request.insert(
            "targetServiceIds".to_string(),
            json!(request.target_service_ids),
        );
    }
    if !request.account_ids.is_empty() {
        preflight_request.insert("accountIds".to_string(), json!(request.account_ids));
    }
    if let Some(target_url) = request.target_url.as_ref() {
        preflight_request.insert("url".to_string(), json!(target_url));
    }
    if let Some(profile_id) = selected_profile_id.as_ref() {
        preflight_request.insert("runtimeProfile".to_string(), json!(profile_id));
    }
    preflight_request.insert("headless".to_string(), json!(!headed && !requires_cdp_free));
    if headed {
        preflight_request.insert("headed".to_string(), json!(true));
    }
    if requires_cdp_free || !cdp_attachment_allowed {
        preflight_request.insert("cdpFree".to_string(), json!(true));
        preflight_request.insert("requiresCdpFree".to_string(), json!(true));
        preflight_request.insert("cdpAttachmentAllowed".to_string(), json!(false));
    } else {
        preflight_request.insert(
            "cdpAttachmentAllowed".to_string(),
            json!(cdp_attachment_allowed),
        );
    }

    json!({
        "available": available,
        "recommendedBeforeUse": recommended_before_use,
        "reason": if recommended_before_use {
            "browser_capability_evidence_available"
        } else if available {
            "browser_build_selected_without_registry_evidence"
        } else {
            "browser_build_unavailable"
        },
        "selectedProfileId": selected_profile_id,
        "browserBuild": browser_build,
        "request": Value::Object(preflight_request),
        "http": {
            "method": "GET",
            "route": "/api/service/browser-capability/preflight",
        },
        "mcp": {
            "tool": "service_browser_capability_preflight",
        },
        "client": {
            "package": "@agent-browser/client/service-observability",
            "helper": "runServiceAccessPlanBrowserCapabilityPreflight",
            "fallbackHelper": "getServiceBrowserCapabilityPreflight",
        },
        "cli": {
            "command": browser_capability_preflight_cli_command(
                browser_build_label.as_deref(),
                selected_profile_id.as_deref(),
                request,
                headed,
                requires_cdp_free,
            ),
        },
        "requestFields": [
            "browserBuild",
            "serviceName",
            "agentName",
            "taskName",
            "targetServiceIds",
            "accountIds",
            "url",
            "runtimeProfile",
            "headless",
            "headed",
            "cdpFree",
            "requiresCdpFree",
            "cdpAttachmentAllowed",
        ],
        "notes": [
            "This is a no-launch gate. It relays through the service worker and reports wouldLaunch false.",
            "Run before browser work when launch routing depends on browser capability registry evidence.",
        ],
    })
}

fn browser_capability_preflight_cli_command(
    browser_build: Option<&str>,
    runtime_profile: Option<&str>,
    request: &ServiceAccessPlanRequest,
    headed: bool,
    requires_cdp_free: bool,
) -> Option<String> {
    let browser_build = browser_build?;
    let mut parts = vec![
        "agent-browser".to_string(),
        "service".to_string(),
        "browser-capability".to_string(),
        "preflight".to_string(),
        "--browser-build".to_string(),
        shell_arg(browser_build),
    ];
    if let Some(runtime_profile) = runtime_profile {
        parts.push("--runtime-profile".to_string());
        parts.push(shell_arg(runtime_profile));
    }
    for target_service_id in &request.target_service_ids {
        parts.push("--target-service-id".to_string());
        parts.push(shell_arg(target_service_id));
    }
    for account_id in &request.account_ids {
        parts.push("--account-id".to_string());
        parts.push(shell_arg(account_id));
    }
    if let Some(url) = request.target_url.as_ref() {
        parts.push("--url".to_string());
        parts.push(shell_arg(url));
    }
    if let Some(service_name) = request.service_name.as_ref() {
        parts.push("--service-name".to_string());
        parts.push(shell_arg(service_name));
    }
    if let Some(agent_name) = request.agent_name.as_ref() {
        parts.push("--agent-name".to_string());
        parts.push(shell_arg(agent_name));
    }
    if let Some(task_name) = request.task_name.as_ref() {
        parts.push("--task-name".to_string());
        parts.push(shell_arg(task_name));
    }
    if headed {
        parts.push("--headed".to_string());
    }
    if requires_cdp_free {
        parts.push("--cdp-free".to_string());
    }
    Some(parts.join(" "))
}

fn shell_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '@'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
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
    if let Some(session_name) = request.session_name.as_deref().or(replacement_session_name) {
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
    let (browser_host, source) = if let Some(browser_host) = request.browser_host {
        (browser_host, "request")
    } else if let Some(browser_host) = site_policy.and_then(|policy| policy.browser_host) {
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
    let (view_stream_provider, view_stream_provider_source) =
        if let Some(provider) = request.view_stream_provider {
            (provider, "request")
        } else {
            launch_view_stream_provider_decision(browser_host, site_policy)
        };
    let (control_input_provider, control_input_provider_source) =
        if let Some(provider) = request.control_input_provider {
            (provider, "request")
        } else {
            launch_control_input_provider_decision(view_stream_provider, site_policy)
        };
    let display_isolation = request
        .display_isolation
        .as_deref()
        .or_else(|| launch_display_isolation_decision(browser_host));
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
    if display_isolation.is_some() {
        rationale.push("remote_headed_private_display_default");
    }
    match view_stream_provider_source {
        "request" => rationale.push("view_stream_from_request"),
        "site_policy" => rationale.push("view_stream_from_site_policy"),
        _ => rationale.push("view_stream_from_service_default"),
    }
    match control_input_provider_source {
        "request" => rationale.push("control_input_from_request"),
        "site_policy" => rationale.push("control_input_from_site_policy"),
        _ => rationale.push("control_input_from_view_stream"),
    }
    if request.display_isolation.is_some() {
        rationale.push("display_isolation_from_request");
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
    if browser_build_source == "site_policy" {
        rationale.push("browser_build_from_site_policy");
    }
    if browser_build_source == "profile_default" {
        rationale.push("browser_build_from_profile_default");
    }
    if browser_build_source == "request" {
        rationale.push("browser_build_from_request");
    }
    if browser_build_source == "service_default" {
        rationale.push("browser_build_from_service_default");
    }
    match source {
        "request" => rationale.push("browser_host_from_request"),
        "site_policy" => rationale.push("browser_host_from_site_policy"),
        "profile_default" => rationale.push("browser_host_from_profile_default"),
        _ => rationale.push("browser_host_from_service_default"),
    }
    let browser_build_selection = browser_build_selection_decision(
        browser_build,
        browser_build_source,
        selected_profile,
        requires_cdp_free,
        browser_capability_evidence,
    );

    LaunchPostureDecision {
        browser_host,
        value: json!({
            "browserHost": browser_host,
            "browserBuild": browser_build,
            "browserBuildSource": browser_build_source,
            "source": source,
            "viewStreamProvider": view_stream_provider,
            "viewStreamProviderSource": view_stream_provider_source,
            "controlInputProvider": control_input_provider,
            "controlInputProviderSource": control_input_provider_source,
            "displayIsolation": display_isolation,
            "headed": headed,
            "remoteViewRecommended": remote_view_recommended,
            "requiresCdpFree": requires_cdp_free,
            "cdpAttachmentAllowed": cdp_attachment_allowed,
            "detachedFirstLoginRequired": manual_seeding_required,
            "attachableAfterSeeding": attachable_after_seeding,
            "rationale": rationale,
            "browserBuildSelection": browser_build_selection,
        }),
    }
}

fn launch_view_stream_provider_decision(
    browser_host: BrowserHost,
    site_policy: Option<&SitePolicy>,
) -> (ViewStreamProvider, &'static str) {
    if let Some(view_stream) = site_policy.and_then(|policy| policy.view_stream) {
        return (view_stream, "site_policy");
    }

    let provider = match browser_host {
        BrowserHost::RemoteHeaded => ViewStreamProvider::RdpGateway,
        BrowserHost::DockerHeaded => ViewStreamProvider::Novnc,
        BrowserHost::CloudProvider => ViewStreamProvider::ExternalUrl,
        BrowserHost::LocalHeadless | BrowserHost::LocalHeaded | BrowserHost::AttachedExisting => {
            ViewStreamProvider::CdpScreencast
        }
    };
    (provider, "service_default")
}

fn launch_control_input_provider_decision(
    view_stream_provider: ViewStreamProvider,
    site_policy: Option<&SitePolicy>,
) -> (ControlInputProvider, &'static str) {
    if let Some(control_input) = site_policy.and_then(|policy| policy.control_input) {
        return (control_input, "site_policy");
    }

    let provider = match view_stream_provider {
        ViewStreamProvider::CdpScreencast => ControlInputProvider::CdpInput,
        ViewStreamProvider::ChromeTabWebrtc | ViewStreamProvider::VirtualDisplayWebrtc => {
            ControlInputProvider::WebrtcInput
        }
        ViewStreamProvider::Novnc => ControlInputProvider::VncInput,
        ViewStreamProvider::RdpGateway | ViewStreamProvider::ExternalUrl => {
            ControlInputProvider::ManualAttachedDesktop
        }
    };
    (provider, "view_stream")
}

fn launch_display_isolation_decision(browser_host: BrowserHost) -> Option<&'static str> {
    match browser_host {
        BrowserHost::RemoteHeaded => Some("private_virtual_display"),
        _ => None,
    }
}

fn browser_build_selection_decision(
    browser_build: BrowserBuild,
    browser_build_source: &'static str,
    selected_profile: Option<&BrowserProfile>,
    requires_cdp_free: bool,
    browser_capability_evidence: &Value,
) -> Value {
    let selected_preference_binding = browser_capability_evidence.get("selectedPreferenceBinding");
    let selected_preference_binding_id =
        selected_preference_binding.and_then(|binding| string_field(binding, "id"));
    let selected_preference_binding_reason =
        selected_preference_binding.and_then(|binding| string_field(binding, "reason"));
    let profile_compatibility_rows = browser_capability_evidence
        .get("profileCompatibility")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let validation_rows = browser_capability_evidence
        .get("validationEvidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let profile_compatibility =
        browser_build_profile_compatibility_summary(selected_profile, &profile_compatibility_rows);
    let validation_evidence = browser_build_validation_evidence_summary(&validation_rows);
    let evidence_source = match browser_build_source {
        "request" => "operator_request",
        "site_policy" => "site_policy",
        "profile_default" => "selected_profile",
        "browser_preference_binding" => "service.browserCapabilityRegistry",
        "requires_cdp_free" => "site_policy_requires_cdp_free",
        _ => "service_default",
    };
    let summary = match browser_build_source {
        "request" => "Explicit browserBuild request selected this build.",
        "site_policy" => "Selected site policy chose this build.",
        "profile_default" => "Selected managed profile prefers this build.",
        "browser_preference_binding" => {
            "Browser capability registry preference binding selected this build."
        }
        "requires_cdp_free" => "Site policy requires CDP-free operation.",
        _ => "Service default browser build selected this build.",
    };

    json!({
        "browserBuild": browser_build,
        "source": browser_build_source,
        "evidenceSource": evidence_source,
        "summary": summary,
        "operatorOverride": browser_build_source == "request",
        "requiresCdpFree": requires_cdp_free,
        "selectedProfileId": selected_profile.map(|profile| profile.id.clone()),
        "selectedProfileBrowserBuild": selected_profile.and_then(|profile| profile.browser_build),
        "selectedPreferenceBindingId": selected_preference_binding_id,
        "selectedPreferenceBindingReason": selected_preference_binding_reason,
        "profileCompatibility": profile_compatibility,
        "validationEvidence": validation_evidence,
    })
}

fn browser_build_profile_compatibility_summary(
    selected_profile: Option<&BrowserProfile>,
    profile_compatibility_rows: &[Value],
) -> Value {
    let matching_ids = profile_compatibility_rows
        .iter()
        .filter_map(|row| string_field(row, "id"))
        .collect::<Vec<_>>();
    let compatible_ids = profile_compatibility_rows
        .iter()
        .filter(|row| row.get("compatible").and_then(Value::as_bool) == Some(true))
        .filter_map(|row| string_field(row, "id"))
        .collect::<Vec<_>>();
    let incompatible_ids = profile_compatibility_rows
        .iter()
        .filter(|row| row.get("compatible").and_then(Value::as_bool) == Some(false))
        .filter_map(|row| string_field(row, "id"))
        .collect::<Vec<_>>();
    let selected_profile_id = selected_profile.map(|profile| profile.id.clone());
    let status = if selected_profile.is_none() {
        "no_selected_profile"
    } else if profile_compatibility_rows.is_empty() {
        "not_declared"
    } else if !incompatible_ids.is_empty() {
        "incompatible_or_mixed"
    } else if !compatible_ids.is_empty() {
        "compatible"
    } else {
        "present_without_boolean_result"
    };
    let reason = match status {
        "no_selected_profile" => "No managed profile was selected for this access plan.",
        "not_declared" => {
            "No browser capability registry profile-compatibility row matched the selected profile."
        }
        "incompatible_or_mixed" => {
            "At least one matched profile-compatibility row reports incompatible."
        }
        "compatible" => {
            "Matched profile-compatibility rows report the selected profile as compatible."
        }
        _ => "Profile-compatibility rows matched but did not declare a boolean result.",
    };

    json!({
        "status": status,
        "reason": reason,
        "selectedProfileId": selected_profile_id,
        "matchingIds": matching_ids,
        "compatibleIds": compatible_ids,
        "incompatibleIds": incompatible_ids,
        "count": profile_compatibility_rows.len(),
    })
}

fn browser_build_validation_evidence_summary(validation_rows: &[Value]) -> Value {
    let matching_ids = validation_rows
        .iter()
        .filter_map(|row| string_field(row, "id"))
        .collect::<Vec<_>>();
    let passed_ids = validation_rows
        .iter()
        .filter(|row| string_field(row, "state").as_deref() == Some("passed"))
        .filter_map(|row| string_field(row, "id"))
        .collect::<Vec<_>>();
    let failed_ids = validation_rows
        .iter()
        .filter(|row| {
            matches!(
                string_field(row, "state").as_deref(),
                Some("failed") | Some("error")
            )
        })
        .filter_map(|row| string_field(row, "id"))
        .collect::<Vec<_>>();
    let status = if validation_rows.is_empty() {
        "missing"
    } else if !failed_ids.is_empty() {
        "failed_or_mixed"
    } else if !passed_ids.is_empty() {
        "passed"
    } else {
        "present"
    };
    let reason = match status {
        "missing" => "No matching browser validation evidence was found.",
        "failed_or_mixed" => "At least one matching browser validation evidence row failed.",
        "passed" => "Matching browser validation evidence includes a passed row.",
        _ => "Matching browser validation evidence exists without a passed or failed state.",
    };

    json!({
        "status": status,
        "reason": reason,
        "matchingIds": matching_ids,
        "passedIds": passed_ids,
        "failedIds": failed_ids,
        "count": validation_rows.len(),
    })
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

    use crate::native::service_model::ProfileOrigin;

    use super::*;
    use crate::native::service_model::{
        BrowserCapabilityRegistry, BrowserHealth, BrowserHost, BrowserProcess, BrowserProfile,
        BrowserSession, Challenge, ChallengeKind, InteractionMode, LeaseState, MonitorState,
        MonitorTarget, ProfileKeyringPolicy, ProfileReadinessState, ProfileSeedingMode,
        ProfileTargetReadiness, ProviderCapability, ProviderKind, RateLimitPolicy, ServiceIncident,
        ServiceProvider, SiteMonitor, SitePolicy, ViewStream,
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
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["runtimeProfile"],
            "known-temp"
        );
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["profile"],
            "/tmp/known-temp-profile"
        );
        assert!(plan["decision"]["oneTimeProfileRecommendation"].is_null());
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
    fn service_access_plan_reuses_ready_transferred_owner_for_tab_acquisition() {
        use crate::runtime_owner_transfer::{
            CleanupObligationState, ProfileOwner, ProfileOwnerState, RuntimeLaneLifecycleState,
            RuntimeLifecycleRecord, RuntimeOwnerRegistry,
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
            runtime_owner_registry: RuntimeOwnerRegistry {
                revision: 659,
                owners: BTreeMap::from([(profile_identity_digest.clone(), owner)]),
                lifecycle_records: BTreeMap::from([(
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
            },
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
    fn service_access_plan_blocks_replacement_when_ready_owner_is_not_reusable() {
        use crate::runtime_owner_transfer::{
            CleanupObligationState, ProfileOwner, ProfileOwnerState, RuntimeLaneLifecycleState,
            RuntimeLifecycleRecord, RuntimeOwnerRegistry,
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
            runtime_owner_registry: RuntimeOwnerRegistry {
                revision: 21,
                owners: BTreeMap::from([(profile_identity_digest.clone(), owner)]),
                lifecycle_records: BTreeMap::from([(
                    browser_id.to_string(),
                    RuntimeLifecycleRecord {
                        logical_browser_id: browser_id.to_string(),
                        profile_identity_digest,
                        owner_generation: 3,
                        lifecycle_state: RuntimeLaneLifecycleState::Ready,
                        cleanup_obligation_state: CleanupObligationState::Owned,
                        ..RuntimeLifecycleRecord::default()
                    },
                )]),
            },
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["acs".to_string()],
                browser_host: Some(BrowserHost::RemoteHeaded),
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(
            plan["decision"]["recommendedAction"],
            "reconcile_lifecycle_owner_for_tab_acquisition"
        );
        assert_eq!(
            plan["decision"]["profileReuse"]["recommendedAction"],
            "blocked_by_lifecycle_owner"
        );
        assert_eq!(
            plan["decision"]["lifecycleReplacement"]["replacementEligible"],
            false
        );
        assert_eq!(plan["decision"]["attention"]["required"], true);
        assert_eq!(plan["decision"]["serviceRequest"]["available"], false);
        assert_eq!(plan["decision"]["serviceRequest"]["request"], Value::Null);
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

        apply_shared_profile_route_hints_for_service_request(&state, &mut command).unwrap();

        assert_eq!(command["browserId"], "browser-x");
        assert_eq!(command["sessionName"], "operator-x");

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
    fn service_access_plan_exposes_terminal_lifecycle_replacement_eligibility() {
        use crate::runtime_owner_transfer::{
            CleanupObligationState, ProfileOwner, ProfileOwnerState, RuntimeLaneLifecycleState,
            RuntimeLifecycleRecord, RuntimeOwnerRegistry,
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
        let state = ServiceState {
            profiles: BTreeMap::from([(
                "bill-soylei".to_string(),
                BrowserProfile {
                    id: "bill-soylei".to_string(),
                    name: "BILL".to_string(),
                    user_data_dir: Some(profile_path.to_string()),
                    target_service_ids: vec!["bill".to_string()],
                    authenticated_service_ids: vec!["bill".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            runtime_owner_registry: RuntimeOwnerRegistry {
                revision: 11,
                owners: BTreeMap::from([(profile_identity_digest.clone(), owner)]),
                lifecycle_records: BTreeMap::from([(
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
            },
            ..ServiceState::default()
        };

        let plan = service_access_plan_for_state(
            &state,
            ServiceAccessPlanRequest {
                target_service_ids: vec!["bill".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
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
        assert_eq!(
            plan["decision"]["serviceRequest"]["request"]["sessionName"],
            "terminal-lane"
        );
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
            true
        );
        assert_eq!(
            explicit_plan["decision"]["serviceRequest"]["request"]["sessionName"],
            "terminal-lane"
        );

        let mut copied_request = json!({
            "action": "tab_new",
            "runtimeProfile": "bill-soylei",
            "targetServiceIds": ["bill"],
        });
        apply_shared_profile_route_hints_for_service_request(&state, &mut copied_request).unwrap();
        assert_eq!(copied_request["sessionName"], "terminal-lane");

        let mut exact_explicit_request = json!({
            "action": "tab_new",
            "runtimeProfile": "bill-soylei",
            "targetServiceIds": ["bill"],
            "sessionName": "terminal-lane",
        });
        apply_shared_profile_route_hints_for_service_request(&state, &mut exact_explicit_request)
            .unwrap();
        assert!(exact_explicit_request.get("browserId").is_none());
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
        ViewStream, ViewStreamProvider, ViewerLease,
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
