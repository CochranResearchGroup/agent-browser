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
