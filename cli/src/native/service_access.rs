//! No-launch access planning for service-owned browser/profile decisions.
//!
//! The access plan joins profile selection, site policy, provider, challenge,
//! and readiness state before a caller asks the service to launch or control a
//! browser. It is intentionally read-only so agents and software clients can
//! get the service recommendation without creating browser process pressure.

use serde_json::{json, Value};

use super::service_lifecycle::{select_service_profile_for_request, ProfileSelectionRequest};
use super::service_model::{
    BrowserProfile, Challenge, ChallengePolicy, ChallengeState, ProfileSelectionReason,
    ServiceState, SitePolicy,
};

/// Parsed access-plan selector shared by HTTP and MCP resources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ServiceAccessPlanRequest {
    pub(crate) service_name: Option<String>,
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
    let site_policy =
        select_site_policy(service_state, &request, selected_profile.as_ref()).cloned();
    let challenges = select_challenges(service_state, request.challenge_id.as_deref());
    let providers = select_providers(
        service_state,
        selected_profile.as_ref(),
        site_policy.as_ref(),
        &challenges,
    );
    let decision = access_plan_decision(
        selected_profile.as_ref(),
        site_policy.as_ref(),
        &challenges,
        &providers,
        readiness.as_ref(),
        &request.target_service_ids,
        &readiness_summary,
    );

    json!({
        "query": {
            "serviceName": request.service_name,
            "targetServiceIds": request.target_service_ids,
            "sitePolicyId": request.site_policy_id,
            "challengeId": request.challenge_id,
            "readinessProfileId": request.readiness_profile_id,
        },
        "selectedProfile": selected_profile.clone(),
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
        "sitePolicy": site_policy,
        "providers": providers,
        "challenges": challenges,
        "decision": decision,
    })
}

fn select_site_policy<'a>(
    service_state: &'a ServiceState,
    request: &ServiceAccessPlanRequest,
    selected_profile: Option<&BrowserProfile>,
) -> Option<&'a SitePolicy> {
    if let Some(site_policy_id) = request.site_policy_id.as_deref() {
        return service_state.site_policies.get(site_policy_id);
    }

    for target_service_id in &request.target_service_ids {
        if let Some(site_policy) = service_state.site_policies.get(target_service_id) {
            return Some(site_policy);
        }
    }

    selected_profile.and_then(|profile| {
        profile
            .site_policy_ids
            .iter()
            .find_map(|site_policy_id| service_state.site_policies.get(site_policy_id))
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
) -> Vec<Value> {
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
        .map(|provider| json!(provider))
        .collect()
}

fn access_plan_decision(
    selected_profile: Option<&BrowserProfile>,
    site_policy: Option<&SitePolicy>,
    challenges: &[Challenge],
    providers: &[Value],
    readiness: Option<&Value>,
    target_service_ids: &[String],
    readiness_summary: &Value,
) -> Value {
    let mut reasons = Vec::new();
    let manual_seeding_required =
        readiness_summary["manualSeedingRequired"].as_bool() == Some(true);
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
    } else if readiness_profile_needs_probe(readiness, target_service_ids) {
        "verify_or_seed_profile_before_authenticated_work"
    } else {
        "use_selected_profile"
    };

    json!({
        "recommendedAction": recommended_action,
        "browserHost": site_policy.and_then(|policy| policy.browser_host),
        "interactionMode": site_policy.map(|policy| policy.interaction_mode),
        "challengePolicy": site_policy.map(|policy| policy.challenge_policy),
        "profileId": selected_profile.map(|profile| profile.id.clone()),
        "manualActionRequired": manual_seeding_required || waiting_for_human || failed_challenge,
        "manualSeedingRequired": manual_seeding_required,
        "providerIds": providers.iter().filter_map(|provider| provider["id"].as_str()).collect::<Vec<_>>(),
        "challengeIds": challenges.iter().map(|challenge| challenge.id.clone()).collect::<Vec<_>>(),
        "reasons": reasons,
    })
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
        ProfileReadinessState, ProfileTargetReadiness, ProviderCapability, ProviderKind,
        ServiceProvider, SitePolicy,
    };

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
                target_service_ids: vec!["google".to_string()],
                ..ServiceAccessPlanRequest::default()
            },
        );

        assert_eq!(plan["selectedProfile"]["id"], "google-work");
        assert_eq!(plan["sitePolicy"]["id"], "google");
        assert_eq!(plan["providers"][0]["id"], "manual");
        assert_eq!(plan["challenges"][0]["id"], "challenge-1");
        assert_eq!(plan["readinessSummary"]["manualSeedingRequired"], true);
        assert_eq!(plan["decision"]["manualActionRequired"], true);
        assert_eq!(plan["decision"]["manualSeedingRequired"], true);
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
