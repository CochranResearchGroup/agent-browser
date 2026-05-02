//! Service-mode lifecycle helpers for launch-derived browser state.
//!
//! This module keeps profile and session mutations close to the durable service
//! model, while command parsing and browser process control remain in actions.

use super::service_model::{
    BrowserProfile, BrowserSession, LeaseState, ProfileAllocationPolicy, ProfileKeyringPolicy,
    ServiceActor, ServiceState, SessionCleanupPolicy,
};

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
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProfileSelectionRequest {
    pub(crate) service_name: Option<String>,
    pub(crate) target_service_ids: Vec<String>,
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
) -> Option<String> {
    service_state
        .profiles
        .iter()
        .filter(|(_, profile)| profile_allows_service(profile, request.service_name.as_deref()))
        .filter_map(|(id, profile)| {
            let rank = profile_selection_rank(profile, request);
            rank.is_match().then(|| (rank, id.clone()))
        })
        .max_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(&left.1)))
        .map(|(_, id)| id)
}

pub(crate) fn upsert_service_profile_and_session(
    service_state: &mut ServiceState,
    session_id: &str,
    profile_id: Option<String>,
    metadata: &ServiceLaunchMetadata,
) {
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
    }

    let session = service_state
        .sessions
        .entry(session_id.to_string())
        .or_insert_with(|| BrowserSession {
            id: session_id.to_string(),
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
    session.lease = if session.profile_id.is_some() {
        LeaseState::Exclusive
    } else {
        session.lease
    };
    session.cleanup = metadata.cleanup;
    merge_unique(
        &mut session.browser_ids,
        service_browser_id_for_session(session_id),
    );
}

fn profile_allows_service(profile: &BrowserProfile, service_name: Option<&str>) -> bool {
    if profile.shared_service_ids.is_empty() {
        return true;
    }
    service_name.is_some_and(|service_name| {
        profile
            .shared_service_ids
            .iter()
            .any(|allowed| allowed == service_name)
    })
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct ProfileSelectionRank {
    authenticated_target_matches: usize,
    target_matches: usize,
    caller_service_match: bool,
    persistent: bool,
}

impl ProfileSelectionRank {
    fn is_match(self) -> bool {
        self.authenticated_target_matches > 0
            || self.target_matches > 0
            || self.caller_service_match
    }
}

fn profile_selection_rank(
    profile: &BrowserProfile,
    request: &ProfileSelectionRequest,
) -> ProfileSelectionRank {
    let authenticated_target_matches = request
        .target_service_ids
        .iter()
        .filter(|target_service_id| {
            profile
                .authenticated_service_ids
                .iter()
                .any(|candidate| candidate == *target_service_id)
        })
        .count();
    let target_matches = request
        .target_service_ids
        .iter()
        .filter(|target_service_id| {
            profile
                .target_service_ids
                .iter()
                .any(|candidate| candidate == *target_service_id)
        })
        .count();
    let caller_service_match = request.service_name.as_deref().is_some_and(|service_name| {
        profile
            .shared_service_ids
            .iter()
            .any(|allowed| allowed == service_name)
    });

    ProfileSelectionRank {
        authenticated_target_matches,
        target_matches,
        caller_service_match,
        persistent: profile.persistent,
    }
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
            },
        );

        assert_eq!(selected.as_deref(), Some("authenticated"));
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
            },
        );

        assert_eq!(selected.as_deref(), Some("authenticated"));
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
            },
        );

        assert_eq!(selected.as_deref(), Some("service-profile"));
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
        assert_eq!(session.cleanup, SessionCleanupPolicy::Detach);
        assert_eq!(session.browser_ids, vec!["session:persist-session"]);
    }
}
