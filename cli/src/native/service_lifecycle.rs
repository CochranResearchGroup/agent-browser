//! Service-mode lifecycle helpers for launch-derived browser state.
//!
//! This module keeps profile and session mutations close to the durable service
//! model, while command parsing and browser process control remain in actions.

use super::service_model::{
    service_site_policy_id_for_url, BrowserBuild, BrowserProfile, BrowserSession, LeaseState,
    ProfileAllocationPolicy, ProfileKeyringPolicy, ProfileLeaseDisposition, ProfileSelectionReason,
    ServiceActor, ServiceState, SessionCleanupPolicy, ViewStream,
};

fn current_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ServiceLaunchMetadata {
    pub(crate) profile_id: Option<String>,
    pub(crate) profile_name: Option<String>,
    pub(crate) user_data_dir: Option<String>,
    pub(crate) persistent_profile: bool,
    pub(crate) keyring: ProfileKeyringPolicy,
    pub(crate) service_name: Option<String>,
    pub(crate) agent_name: Option<String>,
    pub(crate) task_name: Option<String>,
    pub(crate) cleanup: SessionCleanupPolicy,
    pub(crate) profile_selection_reason: Option<ProfileSelectionReason>,
    pub(crate) browser_stderr_log_path: Option<String>,
    pub(crate) browser_capability_launch: Option<serde_json::Value>,
    pub(crate) view_streams: Vec<ViewStream>,
    pub(crate) display_isolation: Option<String>,
    pub(crate) display_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProfileSelectionRequest {
    pub(crate) service_name: Option<String>,
    pub(crate) target_service_ids: Vec<String>,
    pub(crate) account_ids: Vec<String>,
    pub(crate) target_url: Option<String>,
    pub(crate) browser_build: Option<BrowserBuild>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileSelection {
    pub(crate) profile_id: String,
    pub(crate) reason: ProfileSelectionReason,
}

/// One deterministic profile-discovery candidate, ordered from best to worst.
///
/// Discovery callers use the complete ranked set so they can explain
/// alternatives without silently turning a generic browser-build default into
/// an identity match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileSelectionCandidate {
    pub(crate) profile_id: String,
    pub(crate) reason: ProfileSelectionReason,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProfileDiscoveryRequest {
    pub(crate) selection: ProfileSelectionRequest,
    pub(crate) profile_ids: Vec<String>,
    pub(crate) profile_names: Vec<String>,
    pub(crate) hostnames: Vec<String>,
    pub(crate) authentication_states: Vec<String>,
    pub(crate) freshness_states: Vec<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) free_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileDiscoveryCandidate {
    pub(crate) profile_id: String,
    pub(crate) reason: String,
    pub(crate) matched_field: String,
    pub(crate) matched_identity: String,
}

pub(crate) fn service_profile_id(
    profile: Option<&str>,
    runtime_profile: Option<&str>,
) -> Option<String> {
    if let Some(runtime_profile) = runtime_profile.filter(|value| !value.trim().is_empty()) {
        return Some(runtime_profile.to_string());
    }
    profile
        .filter(|value| !value.trim().is_empty())
        .map(|profile| format!("custom:{}", stable_short_hash(profile)))
}

pub(crate) fn select_service_profile_for_request(
    service_state: &ServiceState,
    request: &ProfileSelectionRequest,
) -> Option<ProfileSelection> {
    rank_service_profiles_for_request(service_state, request, true)
        .into_iter()
        .next()
        .map(|candidate| ProfileSelection {
            profile_id: candidate.profile_id,
            reason: candidate.reason,
        })
}

/// Rank profiles for an identity request.
///
/// `include_browser_build_default` is reserved for launch planning, where a
/// generic profile may safely host a new identity. Search and discovery
/// surfaces must pass `false` so an unknown identity produces `not_found`.
pub(crate) fn rank_service_profiles_for_request(
    service_state: &ServiceState,
    request: &ProfileSelectionRequest,
    include_browser_build_default: bool,
) -> Vec<ProfileSelectionCandidate> {
    let effective_request = effective_profile_selection_request(service_state, request);
    let normalized_target_service_ids =
        normalized_profile_request_targets(service_state, &effective_request);
    let mut candidates = service_state
        .profiles
        .iter()
        .filter(|(_, profile)| {
            profile_allows_service(profile, effective_request.service_name.as_deref())
        })
        .filter_map(|(id, profile)| {
            let rank =
                profile_selection_rank(profile, &effective_request, &normalized_target_service_ids);
            rank.reason()
                .filter(|reason| {
                    include_browser_build_default
                        || *reason != ProfileSelectionReason::BrowserBuildDefault
                })
                .map(|reason| (rank, id.clone(), reason))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates
        .into_iter()
        .map(|(_, profile_id, reason)| ProfileSelectionCandidate { profile_id, reason })
        .collect()
}

/// Rank catalog profiles without launching a browser or substituting a
/// generic browser-build default.
pub(crate) fn discover_service_profiles(
    service_state: &ServiceState,
    request: &ProfileDiscoveryRequest,
) -> Vec<ProfileDiscoveryCandidate> {
    let effective_request = effective_profile_selection_request(service_state, &request.selection);
    let normalized_targets = normalized_profile_request_targets(service_state, &effective_request);
    let mut candidates = service_state
        .profiles
        .iter()
        .filter(|(_, profile)| {
            profile_allows_service(profile, effective_request.service_name.as_deref())
        })
        .filter_map(|(id, profile)| {
            let selection_rank =
                profile_selection_rank(profile, &effective_request, &normalized_targets);
            let (catalog_rank, evidence) = profile_catalog_rank(
                id,
                profile,
                request,
                &effective_request,
                &normalized_targets,
                selection_rank,
            )?;
            Some((
                catalog_rank,
                id.clone(),
                ProfileDiscoveryCandidate {
                    profile_id: id.clone(),
                    reason: evidence.0.to_string(),
                    matched_field: evidence.1.to_string(),
                    matched_identity: evidence.2,
                },
            ))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates
        .into_iter()
        .map(|(_, _, candidate)| candidate)
        .collect()
}

pub(crate) fn upsert_service_profile_and_session(
    service_state: &mut ServiceState,
    session_id: &str,
    profile_id: Option<String>,
    metadata: &ServiceLaunchMetadata,
) {
    let lease_observed_at = current_timestamp();
    if let Some(profile_id) = profile_id.as_ref() {
        let profile = service_state
            .profiles
            .entry(profile_id.clone())
            .or_insert_with(|| BrowserProfile {
                id: profile_id.clone(),
                name: metadata
                    .profile_name
                    .clone()
                    .unwrap_or_else(|| profile_id.clone()),
                ..BrowserProfile::default()
            });
        if profile.name.is_empty() {
            profile.name = metadata
                .profile_name
                .clone()
                .unwrap_or_else(|| profile_id.clone());
        }
        if profile.user_data_dir.is_none() {
            profile.user_data_dir = metadata.user_data_dir.clone();
        }
        if metadata.service_name.is_some()
            && profile.allocation == ProfileAllocationPolicy::SharedService
        {
            profile.allocation = ProfileAllocationPolicy::PerService;
        }
        if profile.keyring == ProfileKeyringPolicy::BasicPasswordStore
            || metadata.keyring != ProfileKeyringPolicy::BasicPasswordStore
        {
            profile.keyring = metadata.keyring;
        }
        profile.persistent = profile.persistent || metadata.persistent_profile;
        if metadata.service_name.is_some() {
            profile.manual_login_preferred = profile.manual_login_preferred
                || metadata.keyring == ProfileKeyringPolicy::RealOsKeychain;
        }
        if let Some(service_name) = metadata.service_name.as_ref() {
            merge_unique(&mut profile.shared_service_ids, service_name.clone());
        }
        service_state.mark_runtime_observed_profile_source(profile_id);
    }

    let selected_profile_id = profile_id.clone().or_else(|| {
        service_state
            .sessions
            .get(session_id)
            .and_then(|session| session.profile_id.clone())
    });
    let lease_telemetry = selected_profile_id
        .as_deref()
        .map(|profile_id| profile_lease_telemetry(service_state, session_id, profile_id));

    let session = service_state
        .sessions
        .entry(session_id.to_string())
        .or_insert_with(|| BrowserSession {
            id: session_id.to_string(),
            boot_epoch: crate::process_identity::current_boot_epoch(),
            ..BrowserSession::default()
        });
    session.service_name = metadata
        .service_name
        .clone()
        .or(session.service_name.clone());
    session.agent_name = metadata.agent_name.clone().or(session.agent_name.clone());
    session.task_name = metadata.task_name.clone().or(session.task_name.clone());
    if session.owner.is_system() {
        session.owner = ServiceActor::from_caller_context(
            session.service_name.as_deref(),
            session.agent_name.as_deref(),
        );
    }
    session.profile_id = profile_id.or(session.profile_id.clone());
    session.profile_selection_reason = metadata
        .profile_selection_reason
        .or(session.profile_selection_reason);
    session.browser_capability_launch = metadata
        .browser_capability_launch
        .clone()
        .or(session.browser_capability_launch.clone());
    if let Some(lease_telemetry) = lease_telemetry {
        session.profile_lease_disposition = Some(lease_telemetry.disposition);
        session.profile_lease_conflict_session_ids = lease_telemetry.conflict_session_ids;
    }
    session.lease = if session.profile_id.is_some() {
        LeaseState::Exclusive
    } else {
        session.lease
    };
    session.last_lease_observed_at = Some(lease_observed_at);
    session.boot_epoch = crate::process_identity::current_boot_epoch();
    session.cleanup = metadata.cleanup;
    merge_unique(
        &mut session.browser_ids,
        service_browser_id_for_session(session_id),
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileLeaseTelemetry {
    pub(crate) disposition: ProfileLeaseDisposition,
    pub(crate) conflict_session_ids: Vec<String>,
}

pub(crate) fn profile_lease_telemetry(
    service_state: &ServiceState,
    session_id: &str,
    profile_id: &str,
) -> ProfileLeaseTelemetry {
    let current_browser_id = service_browser_id_for_session(session_id);
    let has_current_browser =
        service_state
            .browsers
            .get(&current_browser_id)
            .is_some_and(|browser| {
                browser.profile_id.as_deref() == Some(profile_id)
                    && browser
                        .active_session_ids
                        .iter()
                        .any(|active_session_id| active_session_id == session_id)
            });
    let mut conflict_session_ids = service_state
        .sessions
        .iter()
        .filter(|(candidate_id, session)| {
            candidate_id.as_str() != session_id
                && session.profile_id.as_deref() == Some(profile_id)
                && session.lease == LeaseState::Exclusive
        })
        .map(|(candidate_id, _)| candidate_id.clone())
        .collect::<Vec<_>>();
    conflict_session_ids.sort();
    conflict_session_ids.dedup();

    let disposition = if !conflict_session_ids.is_empty() {
        ProfileLeaseDisposition::ActiveLeaseConflict
    } else if has_current_browser {
        ProfileLeaseDisposition::ReusedBrowser
    } else {
        ProfileLeaseDisposition::NewBrowser
    };

    ProfileLeaseTelemetry {
        disposition,
        conflict_session_ids,
    }
}

fn profile_allows_service(profile: &BrowserProfile, service_name: Option<&str>) -> bool {
    let Some(service_name) = service_name else {
        return true;
    };
    if profile.shared_service_ids.is_empty() {
        return true;
    }
    profile
        .shared_service_ids
        .iter()
        .any(|allowed| allowed == service_name)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct ProfileSelectionRank {
    authenticated_target_matches: usize,
    account_matches: usize,
    target_matches: usize,
    caller_service_match: bool,
    browser_build_match: bool,
    browser_build_default: bool,
    persistent: bool,
}

impl ProfileSelectionRank {
    fn reason(self) -> Option<ProfileSelectionReason> {
        if self.authenticated_target_matches > 0 {
            Some(ProfileSelectionReason::AuthenticatedTarget)
        } else if self.account_matches > 0 {
            Some(ProfileSelectionReason::AccountMatch)
        } else if self.target_matches > 0 {
            Some(ProfileSelectionReason::TargetMatch)
        } else if self.caller_service_match {
            Some(ProfileSelectionReason::ServiceAllowList)
        } else if self.browser_build_default {
            Some(ProfileSelectionReason::BrowserBuildDefault)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct ProfileCatalogRank {
    authenticated_target_matches: usize,
    account_matches: usize,
    exact_profile_id_matches: usize,
    exact_profile_name_matches: usize,
    alias_matches: usize,
    login_matches: usize,
    origin_matches: usize,
    target_matches: usize,
    tag_matches: usize,
    authentication_state_matches: usize,
    freshness_state_matches: usize,
    caller_service_match: bool,
    free_text_match: bool,
    browser_build_match: bool,
    persistent: bool,
}

fn profile_catalog_rank(
    profile_id: &str,
    profile: &BrowserProfile,
    request: &ProfileDiscoveryRequest,
    selection_request: &ProfileSelectionRequest,
    target_service_ids: &[String],
    selection_rank: ProfileSelectionRank,
) -> Option<(ProfileCatalogRank, (&'static str, &'static str, String))> {
    let exact_profile_id = first_case_insensitive_match(&request.profile_ids, &[profile_id]);
    let exact_profile_name =
        first_case_insensitive_match(&request.profile_names, &[profile.name.as_str()]);
    let alias_match = first_case_insensitive_match(
        request
            .profile_names
            .iter()
            .chain(request.profile_ids.iter())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
        &profile
            .aliases
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    let login_match = first_case_insensitive_match(
        target_service_ids,
        &profile
            .login_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    let origin_match = first_origin_match(&request.hostnames, &profile.origins);
    let target_match = first_case_insensitive_match(
        target_service_ids,
        &profile
            .target_service_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    let account_match = first_case_insensitive_match(
        &selection_request.account_ids,
        &profile
            .account_ids
            .iter()
            .chain(profile.account_labels.iter())
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    let authenticated_match = first_case_insensitive_match(
        target_service_ids,
        &profile
            .authenticated_service_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    let tag_match = first_case_insensitive_match(
        &request.tags,
        &profile.tags.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let authentication_state_match =
        profile_authentication_state_match(profile, &request.authentication_states);
    let freshness_state_match = profile_freshness_state_match(profile, &request.freshness_states);
    let free_text_match = request
        .free_text
        .as_deref()
        .filter(|query| profile_matches_free_text(profile_id, profile, query))
        .map(str::to_string);
    let has_identity_query = !target_service_ids.is_empty()
        || !selection_request.account_ids.is_empty()
        || selection_request.service_name.is_some()
        || !request.profile_ids.is_empty()
        || !request.profile_names.is_empty()
        || !request.hostnames.is_empty()
        || !request.authentication_states.is_empty()
        || !request.freshness_states.is_empty()
        || !request.tags.is_empty()
        || request.free_text.is_some();
    let browser_build_catalog_match = selection_rank.browser_build_match && !has_identity_query;

    let evidence = if let Some(value) = authenticated_match.as_ref() {
        (
            "authenticated_target",
            "authenticatedServiceIds",
            value.clone(),
        )
    } else if let Some(value) = account_match.as_ref() {
        ("account_match", "accountLabels", value.clone())
    } else if let Some(value) = exact_profile_id.as_ref() {
        ("profile_id", "id", value.clone())
    } else if let Some(value) = exact_profile_name.as_ref() {
        ("profile_name", "name", value.clone())
    } else if let Some(value) = alias_match.as_ref() {
        ("alias", "aliases", value.clone())
    } else if let Some(value) = login_match.as_ref() {
        ("login_id", "loginIds", value.clone())
    } else if let Some(value) = origin_match.as_ref() {
        ("origin", "origins", value.clone())
    } else if let Some(value) = target_match.as_ref() {
        ("target_match", "targetServiceIds", value.clone())
    } else if let Some(value) = tag_match.as_ref() {
        ("tag", "tags", value.clone())
    } else if let Some(value) = authentication_state_match.as_ref() {
        ("authentication_state", "authenticationState", value.clone())
    } else if let Some(value) = freshness_state_match.as_ref() {
        ("freshness_state", "targetReadiness", value.clone())
    } else if selection_rank.caller_service_match {
        (
            "service_allow_list",
            "sharedServiceIds",
            selection_request.service_name.clone().unwrap_or_default(),
        )
    } else if let Some(value) = free_text_match.as_ref() {
        ("free_text", "safeMetadata", value.clone())
    } else if browser_build_catalog_match {
        (
            "browser_build",
            "browserBuild",
            selection_request
                .browser_build
                .map(|build| format!("{build:?}").to_lowercase())
                .unwrap_or_default(),
        )
    } else {
        return None;
    };

    Some((
        ProfileCatalogRank {
            authenticated_target_matches: selection_rank.authenticated_target_matches,
            account_matches: usize::from(account_match.is_some()),
            exact_profile_id_matches: usize::from(exact_profile_id.is_some()),
            exact_profile_name_matches: usize::from(exact_profile_name.is_some()),
            alias_matches: usize::from(alias_match.is_some()),
            login_matches: usize::from(login_match.is_some()),
            origin_matches: usize::from(origin_match.is_some()),
            target_matches: selection_rank.target_matches,
            tag_matches: usize::from(tag_match.is_some()),
            authentication_state_matches: usize::from(authentication_state_match.is_some()),
            freshness_state_matches: usize::from(freshness_state_match.is_some()),
            caller_service_match: selection_rank.caller_service_match,
            free_text_match: free_text_match.is_some(),
            browser_build_match: browser_build_catalog_match,
            persistent: profile.persistent,
        },
        evidence,
    ))
}

fn first_case_insensitive_match<T: AsRef<str>>(
    requested: &[T],
    available: &[&str],
) -> Option<String> {
    requested.iter().find_map(|requested| {
        let requested = requested.as_ref().trim();
        available
            .iter()
            .any(|available| requested.eq_ignore_ascii_case(available.trim()))
            .then(|| requested.to_string())
    })
}

fn first_origin_match(hostnames: &[String], origins: &[String]) -> Option<String> {
    hostnames.iter().find_map(|hostname| {
        let hostname = hostname.trim().to_ascii_lowercase();
        origins
            .iter()
            .any(|origin| {
                url::Url::parse(origin)
                    .ok()
                    .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
                    .is_some_and(|origin_host| origin_host == hostname)
            })
            .then_some(hostname)
    })
}

fn profile_authentication_state_match(
    profile: &BrowserProfile,
    requested: &[String],
) -> Option<String> {
    requested.iter().find_map(|state| {
        let matches = match state.trim().to_ascii_lowercase().as_str() {
            "authenticated" => !profile.authenticated_service_ids.is_empty(),
            "configured" => !profile.target_service_ids.is_empty(),
            "unknown" => {
                profile.authenticated_service_ids.is_empty() && profile.target_readiness.is_empty()
            }
            _ => false,
        };
        matches.then(|| state.clone())
    })
}

fn profile_freshness_state_match(profile: &BrowserProfile, requested: &[String]) -> Option<String> {
    requested.iter().find_map(|requested| {
        profile
            .target_readiness
            .iter()
            .filter_map(|row| serde_json::to_value(row.state).ok())
            .filter_map(|state| state.as_str().map(str::to_string))
            .any(|state| state.eq_ignore_ascii_case(requested.trim()))
            .then(|| requested.clone())
    })
}

fn profile_matches_free_text(profile_id: &str, profile: &BrowserProfile, query: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return false;
    }
    std::iter::once(profile_id)
        .chain(std::iter::once(profile.name.as_str()))
        .chain(profile.description.as_deref())
        .chain(profile.aliases.iter().map(String::as_str))
        .chain(profile.origins.iter().map(String::as_str))
        .chain(profile.login_ids.iter().map(String::as_str))
        .chain(profile.account_labels.iter().map(String::as_str))
        .chain(profile.target_service_ids.iter().map(String::as_str))
        .chain(profile.shared_service_ids.iter().map(String::as_str))
        .chain(profile.tags.iter().map(String::as_str))
        .any(|value| value.to_ascii_lowercase().contains(&query))
}

fn profile_selection_rank(
    profile: &BrowserProfile,
    request: &ProfileSelectionRequest,
    target_service_ids: &[String],
) -> ProfileSelectionRank {
    let authenticated_target_matches = target_service_ids
        .iter()
        .filter(|target_service_id| {
            profile
                .authenticated_service_ids
                .iter()
                .any(|candidate| candidate == *target_service_id)
        })
        .count();
    let target_matches = target_service_ids
        .iter()
        .filter(|target_service_id| {
            profile
                .target_service_ids
                .iter()
                .any(|candidate| candidate == *target_service_id)
        })
        .count();
    let account_matches = request
        .account_ids
        .iter()
        .filter(|account_id| {
            profile
                .account_ids
                .iter()
                .any(|candidate| candidate == *account_id)
        })
        .count();
    let caller_service_match = request.service_name.as_deref().is_some_and(|service_name| {
        profile
            .shared_service_ids
            .iter()
            .any(|allowed| allowed == service_name)
    });
    let browser_build_match = request
        .browser_build
        .is_some_and(|browser_build| profile.browser_build == Some(browser_build));
    let browser_build_default = browser_build_match
        && profile.target_service_ids.is_empty()
        && profile.authenticated_service_ids.is_empty()
        && profile.account_ids.is_empty()
        && profile.site_policy_ids.is_empty();

    ProfileSelectionRank {
        authenticated_target_matches,
        account_matches,
        target_matches,
        caller_service_match,
        browser_build_match,
        browser_build_default,
        persistent: profile.persistent,
    }
}

fn effective_profile_selection_request(
    service_state: &ServiceState,
    request: &ProfileSelectionRequest,
) -> ProfileSelectionRequest {
    let mut request = request.clone();
    if request.browser_build.is_none() {
        request.browser_build = browser_build_for_profile_request(service_state, &request);
    }
    request
}

fn browser_build_for_profile_request(
    service_state: &ServiceState,
    request: &ProfileSelectionRequest,
) -> Option<BrowserBuild> {
    if let Some(target_url) = request.target_url.as_deref() {
        if let Some(site_policy_id) = service_site_policy_id_for_url(service_state, target_url) {
            if let Some(browser_build) = service_state
                .site_policies
                .get(&site_policy_id)
                .and_then(|policy| policy.browser_build)
            {
                return Some(browser_build);
            }
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
    }
    service_state.default_browser_build
}

fn normalized_profile_request_targets(
    service_state: &ServiceState,
    request: &ProfileSelectionRequest,
) -> Vec<String> {
    let mut target_service_ids = request.target_service_ids.clone();
    if let Some(target_url) = request.target_url.as_deref() {
        if let Some(site_policy_id) = service_site_policy_id_for_url(service_state, target_url) {
            merge_unique(&mut target_service_ids, site_policy_id);
        }
    }
    target_service_ids.sort();
    target_service_ids.dedup();
    target_service_ids
}

fn merge_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn service_browser_id_for_session(session_id: &str) -> String {
    format!("session:{}", session_id)
}

fn stable_short_hash(value: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    value.as_bytes().iter().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::BrowserProcess;

    #[test]
    fn test_service_profile_id_prefers_runtime_profile() {
        assert_eq!(
            service_profile_id(Some("/tmp/browser-profile"), Some("work")),
            Some("work".to_string())
        );
    }

    #[test]
    fn test_service_profile_id_hashes_custom_profile_path() {
        let profile_id = service_profile_id(Some("/tmp/browser-profile"), None).unwrap();

        assert!(profile_id.starts_with("custom:"));
        assert_ne!(profile_id, "/tmp/browser-profile");
        assert_eq!(
            profile_id,
            service_profile_id(Some("/tmp/browser-profile"), None).unwrap()
        );
    }

    #[test]
    fn test_select_service_profile_prefers_authenticated_target_match() {
        let mut service_state = ServiceState::default();
        service_state.profiles.insert(
            "target-only".to_string(),
            BrowserProfile {
                id: "target-only".to_string(),
                name: "Target only".to_string(),
                target_service_ids: vec!["acs".to_string()],
                shared_service_ids: vec!["JournalDownloader".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );
        service_state.profiles.insert(
            "authenticated".to_string(),
            BrowserProfile {
                id: "authenticated".to_string(),
                name: "Authenticated".to_string(),
                target_service_ids: vec!["acs".to_string()],
                authenticated_service_ids: vec!["acs".to_string()],
                shared_service_ids: vec!["JournalDownloader".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let selected = select_service_profile_for_request(
            &service_state,
            &ProfileSelectionRequest {
                service_name: Some("JournalDownloader".to_string()),
                target_service_ids: vec!["acs".to_string()],
                account_ids: Vec::new(),
                target_url: None,
                browser_build: None,
            },
        );

        let selected = selected.expect("authenticated profile should be selected");
        assert_eq!(selected.profile_id, "authenticated");
        assert_eq!(selected.reason, ProfileSelectionReason::AuthenticatedTarget);
    }

    #[test]
    fn test_profile_discovery_considers_shared_profile_without_caller_service() {
        let mut service_state = ServiceState::default();
        service_state.profiles.insert(
            "shared-authenticated".to_string(),
            BrowserProfile {
                id: "shared-authenticated".to_string(),
                name: "Shared authenticated".to_string(),
                target_service_ids: vec!["x".to_string()],
                authenticated_service_ids: vec!["x".to_string()],
                shared_service_ids: vec!["last30days".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let selected = select_service_profile_for_request(
            &service_state,
            &ProfileSelectionRequest {
                service_name: None,
                target_service_ids: vec!["x".to_string()],
                account_ids: Vec::new(),
                target_url: None,
                browser_build: None,
            },
        )
        .expect("shared profile should remain discoverable without a caller service");

        assert_eq!(selected.profile_id, "shared-authenticated");
        assert_eq!(selected.reason, ProfileSelectionReason::AuthenticatedTarget);
    }

    #[test]
    fn test_profile_discovery_does_not_return_generic_default_for_unknown_identity() {
        let mut service_state = ServiceState {
            default_browser_build: Some(BrowserBuild::StealthcdpChromium),
            ..ServiceState::default()
        };
        service_state.profiles.insert(
            "stealth-default".to_string(),
            BrowserProfile {
                id: "stealth-default".to_string(),
                name: "Stealth default".to_string(),
                browser_build: Some(BrowserBuild::StealthcdpChromium),
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let candidates = rank_service_profiles_for_request(
            &service_state,
            &ProfileSelectionRequest {
                service_name: None,
                target_service_ids: vec!["unknown-site".to_string()],
                account_ids: Vec::new(),
                target_url: None,
                browser_build: None,
            },
            false,
        );

        assert!(
            candidates.is_empty(),
            "identity discovery must report not_found instead of a browser default"
        );
    }

    #[test]
    fn test_profile_discovery_free_text_miss_does_not_fall_back_to_browser_build() {
        let mut service_state = ServiceState {
            default_browser_build: Some(BrowserBuild::StealthcdpChromium),
            ..ServiceState::default()
        };
        service_state.profiles.insert(
            "social".to_string(),
            BrowserProfile {
                id: "social".to_string(),
                name: "Authenticated social profile".to_string(),
                browser_build: Some(BrowserBuild::StealthcdpChromium),
                tags: vec!["social".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let candidates = discover_service_profiles(
            &service_state,
            &ProfileDiscoveryRequest {
                selection: ProfileSelectionRequest {
                    browser_build: Some(BrowserBuild::StealthcdpChromium),
                    ..ProfileSelectionRequest::default()
                },
                free_text: Some("definitely-not-social".to_string()),
                ..ProfileDiscoveryRequest::default()
            },
        );

        assert!(
            candidates.is_empty(),
            "an unmatched free-text identity must report not_found"
        );
    }

    #[test]
    fn test_select_service_profile_authenticated_match_beats_many_target_matches() {
        let broad_target_service_ids = vec![
            "acs".to_string(),
            "google".to_string(),
            "microsoft".to_string(),
            "orcid".to_string(),
            "nih".to_string(),
            "pubmed".to_string(),
            "crossref".to_string(),
            "scopus".to_string(),
            "wos".to_string(),
            "canvas".to_string(),
            "github".to_string(),
            "gmail".to_string(),
            "outlook".to_string(),
        ];
        let mut service_state = ServiceState::default();
        service_state.profiles.insert(
            "many-targets".to_string(),
            BrowserProfile {
                id: "many-targets".to_string(),
                name: "Many target scopes".to_string(),
                target_service_ids: broad_target_service_ids.clone(),
                shared_service_ids: vec!["JournalDownloader".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );
        service_state.profiles.insert(
            "authenticated".to_string(),
            BrowserProfile {
                id: "authenticated".to_string(),
                name: "Authenticated".to_string(),
                target_service_ids: vec!["acs".to_string()],
                authenticated_service_ids: vec!["acs".to_string()],
                shared_service_ids: vec!["JournalDownloader".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let selected = select_service_profile_for_request(
            &service_state,
            &ProfileSelectionRequest {
                service_name: Some("JournalDownloader".to_string()),
                target_service_ids: broad_target_service_ids,
                account_ids: Vec::new(),
                target_url: None,
                browser_build: None,
            },
        );

        let selected = selected.expect("authenticated profile should be selected");
        assert_eq!(selected.profile_id, "authenticated");
        assert_eq!(selected.reason, ProfileSelectionReason::AuthenticatedTarget);
    }

    #[test]
    fn test_select_service_profile_respects_service_allow_list() {
        let mut service_state = ServiceState::default();
        service_state.profiles.insert(
            "restricted".to_string(),
            BrowserProfile {
                id: "restricted".to_string(),
                name: "Restricted".to_string(),
                target_service_ids: vec!["acs".to_string()],
                authenticated_service_ids: vec!["acs".to_string()],
                shared_service_ids: vec!["OtherService".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let selected = select_service_profile_for_request(
            &service_state,
            &ProfileSelectionRequest {
                service_name: Some("JournalDownloader".to_string()),
                target_service_ids: vec!["acs".to_string()],
                account_ids: Vec::new(),
                target_url: None,
                browser_build: None,
            },
        );

        assert!(selected.is_none());
    }

    #[test]
    fn test_select_service_profile_uses_service_match_as_fallback() {
        let mut service_state = ServiceState::default();
        service_state.profiles.insert(
            "service-profile".to_string(),
            BrowserProfile {
                id: "service-profile".to_string(),
                name: "Service profile".to_string(),
                shared_service_ids: vec!["JournalDownloader".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let selected = select_service_profile_for_request(
            &service_state,
            &ProfileSelectionRequest {
                service_name: Some("JournalDownloader".to_string()),
                target_service_ids: Vec::new(),
                account_ids: Vec::new(),
                target_url: None,
                browser_build: None,
            },
        );

        let selected = selected.expect("service allow-list fallback should be selected");
        assert_eq!(selected.profile_id, "service-profile");
        assert_eq!(selected.reason, ProfileSelectionReason::ServiceAllowList);
    }

    #[test]
    fn test_select_service_profile_uses_account_match_before_target_match() {
        let mut service_state = ServiceState::default();
        service_state.profiles.insert(
            "target-only".to_string(),
            BrowserProfile {
                id: "target-only".to_string(),
                name: "Target only".to_string(),
                target_service_ids: vec!["google".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );
        service_state.profiles.insert(
            "account".to_string(),
            BrowserProfile {
                id: "account".to_string(),
                name: "Account".to_string(),
                target_service_ids: vec!["google".to_string()],
                account_ids: vec!["eric@example.com".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let selected = select_service_profile_for_request(
            &service_state,
            &ProfileSelectionRequest {
                service_name: None,
                target_service_ids: vec!["google".to_string()],
                account_ids: vec!["eric@example.com".to_string()],
                target_url: None,
                browser_build: None,
            },
        )
        .expect("account profile should be selected");

        assert_eq!(selected.profile_id, "account");
        assert_eq!(selected.reason, ProfileSelectionReason::AccountMatch);
    }

    #[test]
    fn test_select_service_profile_derives_target_from_url_policy() {
        let mut service_state = ServiceState::default();
        service_state.profiles.insert(
            "canva".to_string(),
            BrowserProfile {
                id: "canva".to_string(),
                name: "Canva".to_string(),
                target_service_ids: vec!["canva".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let selected = select_service_profile_for_request(
            &service_state,
            &ProfileSelectionRequest {
                service_name: None,
                target_service_ids: Vec::new(),
                account_ids: Vec::new(),
                target_url: Some("https://www.canva.com/designs".to_string()),
                browser_build: None,
            },
        )
        .expect("URL should map through built-in Canva site policy");

        assert_eq!(selected.profile_id, "canva");
        assert_eq!(selected.reason, ProfileSelectionReason::TargetMatch);
    }

    #[test]
    fn test_select_service_profile_prefers_browser_build_with_same_identity_rank() {
        let mut service_state = ServiceState::default();
        service_state.profiles.insert(
            "chrome-native".to_string(),
            BrowserProfile {
                id: "chrome-native".to_string(),
                name: "Chrome native".to_string(),
                target_service_ids: vec!["only-works-on-chrome".to_string()],
                account_ids: vec!["myuser".to_string()],
                browser_build: Some(BrowserBuild::StockChrome),
                persistent: true,
                ..BrowserProfile::default()
            },
        );
        service_state.profiles.insert(
            "stealth-default".to_string(),
            BrowserProfile {
                id: "stealth-default".to_string(),
                name: "Stealth default".to_string(),
                target_service_ids: vec!["only-works-on-chrome".to_string()],
                account_ids: vec!["myuser".to_string()],
                browser_build: Some(BrowserBuild::StealthcdpChromium),
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let selected = select_service_profile_for_request(
            &service_state,
            &ProfileSelectionRequest {
                service_name: None,
                target_service_ids: vec!["only-works-on-chrome".to_string()],
                account_ids: vec!["myuser".to_string()],
                target_url: None,
                browser_build: Some(BrowserBuild::StockChrome),
            },
        )
        .expect("browser-build preference should break equal identity ties");

        assert_eq!(selected.profile_id, "chrome-native");
        assert_eq!(selected.reason, ProfileSelectionReason::AccountMatch);
    }

    #[test]
    fn test_select_service_profile_uses_generic_default_browser_build_profile() {
        let mut service_state = ServiceState {
            default_browser_build: Some(BrowserBuild::StealthcdpChromium),
            ..ServiceState::default()
        };
        service_state.profiles.insert(
            "stealth-default".to_string(),
            BrowserProfile {
                id: "stealth-default".to_string(),
                name: "Stealth default".to_string(),
                browser_build: Some(BrowserBuild::StealthcdpChromium),
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let selected = select_service_profile_for_request(
            &service_state,
            &ProfileSelectionRequest {
                service_name: Some("NewService".to_string()),
                target_service_ids: vec!["new-site".to_string()],
                account_ids: vec!["new-user".to_string()],
                target_url: None,
                browser_build: None,
            },
        )
        .expect("generic default browser-build profile should host new identities");

        assert_eq!(selected.profile_id, "stealth-default");
        assert_eq!(selected.reason, ProfileSelectionReason::BrowserBuildDefault);
    }

    #[test]
    fn test_upsert_service_profile_and_session_records_launch_relationships() {
        let mut service_state = ServiceState::default();
        let metadata = ServiceLaunchMetadata {
            profile_id: Some("work".to_string()),
            profile_name: Some("Work".to_string()),
            user_data_dir: Some("/tmp/agent-browser-work".to_string()),
            persistent_profile: true,
            keyring: ProfileKeyringPolicy::RealOsKeychain,
            service_name: Some("JournalDownloader".to_string()),
            agent_name: Some("codex".to_string()),
            task_name: Some("probe-acs-website".to_string()),
            cleanup: SessionCleanupPolicy::Detach,
            profile_selection_reason: Some(ProfileSelectionReason::ExplicitProfile),
            browser_stderr_log_path: None,
            browser_capability_launch: Some(serde_json::json!({
                "applied": false,
                "reason": "test"
            })),
            view_streams: Vec::new(),
            ..ServiceLaunchMetadata::default()
        };

        upsert_service_profile_and_session(
            &mut service_state,
            "persist-session",
            metadata.profile_id.clone(),
            &metadata,
        );

        let profile = &service_state.profiles["work"];
        assert_eq!(profile.name, "Work");
        assert_eq!(
            profile.user_data_dir.as_deref(),
            Some("/tmp/agent-browser-work")
        );
        assert_eq!(profile.allocation, ProfileAllocationPolicy::PerService);
        assert_eq!(profile.keyring, ProfileKeyringPolicy::RealOsKeychain);
        assert!(profile.persistent);
        assert!(profile.manual_login_preferred);
        assert_eq!(
            profile.shared_service_ids,
            vec!["JournalDownloader".to_string()]
        );

        let session = &service_state.sessions["persist-session"];
        assert_eq!(session.profile_id.as_deref(), Some("work"));
        assert_eq!(session.service_name.as_deref(), Some("JournalDownloader"));
        assert_eq!(session.agent_name.as_deref(), Some("codex"));
        assert_eq!(session.task_name.as_deref(), Some("probe-acs-website"));
        assert_eq!(session.owner, ServiceActor::Agent("codex".to_string()));
        assert_eq!(session.lease, LeaseState::Exclusive);
        assert_eq!(
            session.profile_selection_reason,
            Some(ProfileSelectionReason::ExplicitProfile)
        );
        assert_eq!(
            session.profile_lease_disposition,
            Some(ProfileLeaseDisposition::NewBrowser)
        );
        assert!(session.last_lease_observed_at.is_some());
        assert!(session.profile_lease_conflict_session_ids.is_empty());
        assert_eq!(session.cleanup, SessionCleanupPolicy::Detach);
        assert_eq!(session.browser_ids, vec!["session:persist-session"]);
        assert_eq!(
            session.browser_capability_launch.as_ref().unwrap()["reason"],
            "test"
        );
    }

    #[test]
    fn test_upsert_service_session_records_reused_browser_lease_disposition() {
        let mut service_state = ServiceState::default();
        service_state.browsers.insert(
            "session:persist-session".to_string(),
            BrowserProcess {
                id: "session:persist-session".to_string(),
                profile_id: Some("work".to_string()),
                active_session_ids: vec!["persist-session".to_string()],
                ..BrowserProcess::default()
            },
        );
        let metadata = ServiceLaunchMetadata {
            profile_id: Some("work".to_string()),
            profile_name: Some("Work".to_string()),
            persistent_profile: true,
            profile_selection_reason: Some(ProfileSelectionReason::AuthenticatedTarget),
            ..ServiceLaunchMetadata::default()
        };

        upsert_service_profile_and_session(
            &mut service_state,
            "persist-session",
            metadata.profile_id.clone(),
            &metadata,
        );

        let session = &service_state.sessions["persist-session"];
        assert_eq!(
            session.profile_lease_disposition,
            Some(ProfileLeaseDisposition::ReusedBrowser)
        );
        assert!(session.profile_lease_conflict_session_ids.is_empty());
    }

    #[test]
    fn test_upsert_service_session_records_active_lease_conflict() {
        let mut service_state = ServiceState::default();
        service_state.sessions.insert(
            "other-session".to_string(),
            BrowserSession {
                id: "other-session".to_string(),
                profile_id: Some("work".to_string()),
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        );
        let metadata = ServiceLaunchMetadata {
            profile_id: Some("work".to_string()),
            profile_name: Some("Work".to_string()),
            persistent_profile: true,
            profile_selection_reason: Some(ProfileSelectionReason::AuthenticatedTarget),
            ..ServiceLaunchMetadata::default()
        };

        upsert_service_profile_and_session(
            &mut service_state,
            "persist-session",
            metadata.profile_id.clone(),
            &metadata,
        );

        let session = &service_state.sessions["persist-session"];
        assert_eq!(
            session.profile_lease_disposition,
            Some(ProfileLeaseDisposition::ActiveLeaseConflict)
        );
        assert_eq!(
            session.profile_lease_conflict_session_ids,
            vec!["other-session".to_string()]
        );
    }
}
#[allow(dead_code, unused_imports)]
pub(crate) mod service_commands {
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::service_access::required_service_config_id;
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
    use serde_json::{json, Map, Value};
    use time::{format_description::well_known::Rfc3339, OffsetDateTime};
    pub(crate) async fn handle_service_session_upsert(cmd: &Value) -> Result<Value, String> {
        let session_id = required_service_config_id(cmd, "sessionId")?;
        let body = cmd.get("session").cloned().ok_or("Missing session")?;
        let session = upsert_persisted_session(session_id, body)?;
        Ok(json!({ "id" : session_id, "session" : session, "upserted" : true, }))
    }
    pub(crate) async fn handle_service_session_delete(cmd: &Value) -> Result<Value, String> {
        let session_id = required_service_config_id(cmd, "sessionId")?;
        let removed = delete_persisted_session(session_id)?;
        Ok(json!(
            { "id" : session_id, "deleted" : removed.is_some(), "session" : removed,
            }
        ))
    }
}
pub(crate) use service_commands::*;
