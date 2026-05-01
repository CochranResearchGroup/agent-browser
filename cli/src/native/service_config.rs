//! Persistent service configuration mutations.
//!
//! These helpers keep HTTP and MCP mutation surfaces aligned while preserving
//! the JSON-backed service state as the current durable store.

use serde_json::Value;

use super::service_model::{
    BrowserProfile, BrowserSession, ProfileAllocationPolicy, ServiceActor, ServiceProvider,
    ServiceState, SitePolicy,
};
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};

fn validate_entity_id(id: &str, label: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err(format!("{label} id must not be empty"));
    }
    if id.contains('/') {
        return Err(format!("{label} id must not contain '/'"));
    }
    Ok(())
}

fn object_body_with_path_id(mut value: Value, id: &str, label: &str) -> Result<Value, String> {
    validate_entity_id(id, label)?;
    let Some(object) = value.as_object_mut() else {
        return Err(format!("{label} body must be a JSON object"));
    };
    if let Some(body_id) = object
        .get("id")
        .and_then(|value| value.as_str())
        .filter(|body_id| !body_id.is_empty())
    {
        if body_id != id {
            return Err(format!(
                "{label} body id '{body_id}' does not match path id '{id}'"
            ));
        }
    }
    object.insert("id".to_string(), Value::String(id.to_string()));
    Ok(value)
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn validate_profile_policy(profile: &BrowserProfile) -> Result<(), String> {
    if profile.allocation == ProfileAllocationPolicy::CallerSupplied
        && non_empty(profile.user_data_dir.as_deref()).is_none()
    {
        return Err("caller_supplied profile allocation requires userDataDir".to_string());
    }
    if profile.allocation == ProfileAllocationPolicy::PerService
        && profile.shared_service_ids.len() > 1
    {
        return Err(
            "per_service profile allocation may list at most one sharedServiceIds entry"
                .to_string(),
        );
    }
    Ok(())
}

fn normalize_session_owner(session: &mut BrowserSession) {
    if session.owner.is_system() {
        session.owner = ServiceActor::from_caller_context(
            session.service_name.as_deref(),
            session.agent_name.as_deref(),
        );
    }
}

fn validate_session_profile_policy(
    state: &ServiceState,
    session: &BrowserSession,
) -> Result<(), String> {
    let Some(profile_id) = non_empty(session.profile_id.as_deref()) else {
        return Ok(());
    };
    let Some(profile) = state.profiles.get(profile_id) else {
        return Err(format!(
            "session profileId '{profile_id}' does not match a persisted profile"
        ));
    };
    validate_profile_policy(profile)?;

    match profile.allocation {
        ProfileAllocationPolicy::SharedService | ProfileAllocationPolicy::PerService => {
            let Some(service_name) = non_empty(session.service_name.as_deref()) else {
                return Err(format!(
                    "session serviceName is required when using profile '{profile_id}' with {} allocation",
                    serde_json::to_string(&profile.allocation)
                        .unwrap_or_else(|_| "\"unknown\"".to_string())
                        .trim_matches('"')
                ));
            };
            if !profile.shared_service_ids.is_empty()
                && !profile
                    .shared_service_ids
                    .iter()
                    .any(|allowed| allowed == service_name)
            {
                return Err(format!(
                    "session serviceName '{service_name}' is not allowed to use profile '{profile_id}'"
                ));
            }
        }
        ProfileAllocationPolicy::PerSite
        | ProfileAllocationPolicy::PerIdentity
        | ProfileAllocationPolicy::CallerSupplied => {}
    }

    Ok(())
}

/// Upsert one service site-policy record into persisted service state.
pub fn upsert_site_policy(
    state: &mut ServiceState,
    id: &str,
    body: Value,
) -> Result<SitePolicy, String> {
    let body = object_body_with_path_id(body, id, "site policy")?;
    let policy = serde_json::from_value::<SitePolicy>(body)
        .map_err(|err| format!("Invalid site policy: {err}"))?;
    state.site_policies.insert(id.to_string(), policy.clone());
    Ok(policy)
}

/// Delete one service site-policy record from persisted service state.
pub fn delete_site_policy(
    state: &mut ServiceState,
    id: &str,
) -> Result<Option<SitePolicy>, String> {
    validate_entity_id(id, "site policy")?;
    Ok(state.site_policies.remove(id))
}

/// Upsert one service profile record into persisted service state.
pub fn upsert_profile(
    state: &mut ServiceState,
    id: &str,
    body: Value,
) -> Result<BrowserProfile, String> {
    let body = object_body_with_path_id(body, id, "profile")?;
    let profile = serde_json::from_value::<BrowserProfile>(body)
        .map_err(|err| format!("Invalid profile: {err}"))?;
    validate_profile_policy(&profile)?;
    state.profiles.insert(id.to_string(), profile.clone());
    Ok(profile)
}

/// Delete one service profile record from persisted service state.
pub fn delete_profile(
    state: &mut ServiceState,
    id: &str,
) -> Result<Option<BrowserProfile>, String> {
    validate_entity_id(id, "profile")?;
    Ok(state.profiles.remove(id))
}

/// Upsert one service session record into persisted service state.
pub fn upsert_session(
    state: &mut ServiceState,
    id: &str,
    body: Value,
) -> Result<BrowserSession, String> {
    let body = object_body_with_path_id(body, id, "session")?;
    let mut session = serde_json::from_value::<BrowserSession>(body)
        .map_err(|err| format!("Invalid session: {err}"))?;
    normalize_session_owner(&mut session);
    validate_session_profile_policy(state, &session)?;
    state.sessions.insert(id.to_string(), session.clone());
    Ok(session)
}

/// Delete one service session record from persisted service state.
pub fn delete_session(
    state: &mut ServiceState,
    id: &str,
) -> Result<Option<BrowserSession>, String> {
    validate_entity_id(id, "session")?;
    Ok(state.sessions.remove(id))
}

/// Upsert one service provider record into persisted service state.
pub fn upsert_provider(
    state: &mut ServiceState,
    id: &str,
    body: Value,
) -> Result<ServiceProvider, String> {
    let body = object_body_with_path_id(body, id, "provider")?;
    let provider = serde_json::from_value::<ServiceProvider>(body)
        .map_err(|err| format!("Invalid provider: {err}"))?;
    state.providers.insert(id.to_string(), provider.clone());
    Ok(provider)
}

/// Delete one service provider record from persisted service state.
pub fn delete_provider(
    state: &mut ServiceState,
    id: &str,
) -> Result<Option<ServiceProvider>, String> {
    validate_entity_id(id, "provider")?;
    Ok(state.providers.remove(id))
}

/// Upsert one persisted profile record under the serialized state mutator.
pub fn upsert_persisted_profile(id: &str, body: Value) -> Result<BrowserProfile, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    upsert_profile_in_repository(&repository, id, body)
}

/// Delete one persisted profile record under the serialized state mutator.
pub fn delete_persisted_profile(id: &str) -> Result<Option<BrowserProfile>, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    delete_profile_in_repository(&repository, id)
}

/// Upsert one persisted session record under the serialized state mutator.
pub fn upsert_persisted_session(id: &str, body: Value) -> Result<BrowserSession, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    upsert_session_in_repository(&repository, id, body)
}

/// Delete one persisted session record under the serialized state mutator.
pub fn delete_persisted_session(id: &str) -> Result<Option<BrowserSession>, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    delete_session_in_repository(&repository, id)
}

/// Upsert one persisted site-policy record under the serialized state mutator.
pub fn upsert_persisted_site_policy(id: &str, body: Value) -> Result<SitePolicy, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    upsert_site_policy_in_repository(&repository, id, body)
}

/// Delete one persisted site-policy record under the serialized state mutator.
pub fn delete_persisted_site_policy(id: &str) -> Result<Option<SitePolicy>, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    delete_site_policy_in_repository(&repository, id)
}

/// Upsert one persisted provider record under the serialized state mutator.
pub fn upsert_persisted_provider(id: &str, body: Value) -> Result<ServiceProvider, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    upsert_provider_in_repository(&repository, id, body)
}

/// Delete one persisted provider record under the serialized state mutator.
pub fn delete_persisted_provider(id: &str) -> Result<Option<ServiceProvider>, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    delete_provider_in_repository(&repository, id)
}

pub fn upsert_profile_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
    body: Value,
) -> Result<BrowserProfile, String> {
    repository.mutate(|state| upsert_profile(state, id, body))
}

pub fn delete_profile_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
) -> Result<Option<BrowserProfile>, String> {
    repository.mutate(|state| delete_profile(state, id))
}

pub fn upsert_session_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
    body: Value,
) -> Result<BrowserSession, String> {
    repository.mutate(|state| upsert_session(state, id, body))
}

pub fn delete_session_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
) -> Result<Option<BrowserSession>, String> {
    repository.mutate(|state| delete_session(state, id))
}

pub fn upsert_site_policy_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
    body: Value,
) -> Result<SitePolicy, String> {
    repository.mutate(|state| upsert_site_policy(state, id, body))
}

pub fn delete_site_policy_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
) -> Result<Option<SitePolicy>, String> {
    repository.mutate(|state| delete_site_policy(state, id))
}

pub fn upsert_provider_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
    body: Value,
) -> Result<ServiceProvider, String> {
    repository.mutate(|state| upsert_provider(state, id, body))
}

pub fn delete_provider_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
) -> Result<Option<ServiceProvider>, String> {
    repository.mutate(|state| delete_provider(state, id))
}

#[cfg(test)]
mod tests {
    use super::super::service_store::{JsonServiceStateStore, ServiceStateStore};
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_state_path(label: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "agent-browser-{label}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("state.json")
    }

    #[test]
    fn upsert_site_policy_sets_path_id_and_defaults() {
        let mut state = ServiceState::default();

        let policy = upsert_site_policy(
            &mut state,
            "google",
            json!({
                "originPattern": "https://accounts.google.com",
                "interactionMode": "human_like_input"
            }),
        )
        .unwrap();

        assert_eq!(policy.id, "google");
        assert_eq!(policy.origin_pattern, "https://accounts.google.com");
        assert!(state.site_policies.contains_key("google"));
    }

    #[test]
    fn upsert_profile_sets_path_id_and_defaults() {
        let mut state = ServiceState::default();

        let profile = upsert_profile(
            &mut state,
            "journal-downloader",
            json!({
                "name": "Journal Downloader",
                "allocation": "per_service",
                "keyring": "basic_password_store",
                "persistent": true
            }),
        )
        .unwrap();

        assert_eq!(profile.id, "journal-downloader");
        assert_eq!(profile.name, "Journal Downloader");
        assert!(profile.persistent);
        assert!(state.profiles.contains_key("journal-downloader"));
    }

    #[test]
    fn upsert_profile_rejects_caller_supplied_without_user_data_dir() {
        let mut state = ServiceState::default();

        let err = upsert_profile(
            &mut state,
            "custom-profile",
            json!({
                "name": "Custom",
                "allocation": "caller_supplied"
            }),
        )
        .unwrap_err();

        assert!(err.contains("requires userDataDir"));
    }

    #[test]
    fn upsert_session_rejects_body_id_mismatch() {
        let mut state = ServiceState::default();

        let err = upsert_session(
            &mut state,
            "journal-run",
            json!({
                "id": "other",
                "serviceName": "JournalDownloader"
            }),
        )
        .unwrap_err();

        assert!(err.contains("does not match path id"));
    }

    #[test]
    fn upsert_session_infers_owner_from_agent_label() {
        let mut state = ServiceState::default();
        upsert_profile(
            &mut state,
            "journal-downloader",
            json!({
                "name": "Journal Downloader",
                "allocation": "per_service",
                "sharedServiceIds": ["JournalDownloader"]
            }),
        )
        .unwrap();

        let session = upsert_session(
            &mut state,
            "journal-run",
            json!({
                "serviceName": "JournalDownloader",
                "agentName": "codex",
                "taskName": "probeACSwebsite",
                "profileId": "journal-downloader",
                "lease": "exclusive"
            }),
        )
        .unwrap();

        assert_eq!(session.owner, ServiceActor::Agent("codex".to_string()));
    }

    #[test]
    fn upsert_session_rejects_unknown_profile() {
        let mut state = ServiceState::default();

        let err = upsert_session(
            &mut state,
            "journal-run",
            json!({
                "serviceName": "JournalDownloader",
                "profileId": "missing-profile"
            }),
        )
        .unwrap_err();

        assert!(err.contains("does not match a persisted profile"));
    }

    #[test]
    fn upsert_session_rejects_service_not_allowed_by_profile() {
        let mut state = ServiceState::default();
        upsert_profile(
            &mut state,
            "journal-downloader",
            json!({
                "name": "Journal Downloader",
                "allocation": "per_service",
                "sharedServiceIds": ["JournalDownloader"]
            }),
        )
        .unwrap();

        let err = upsert_session(
            &mut state,
            "other-run",
            json!({
                "serviceName": "OtherService",
                "profileId": "journal-downloader"
            }),
        )
        .unwrap_err();

        assert!(err.contains("is not allowed to use profile"));
    }

    #[test]
    fn upsert_provider_rejects_body_id_mismatch() {
        let mut state = ServiceState::default();

        let err = upsert_provider(
            &mut state,
            "manual",
            json!({
                "id": "other",
                "kind": "manual_approval"
            }),
        )
        .unwrap_err();

        assert!(err.contains("does not match path id"));
    }

    #[test]
    fn delete_provider_reports_removed_record() {
        let mut state = ServiceState::default();
        upsert_provider(
            &mut state,
            "manual",
            json!({
                "displayName": "Dashboard approval",
                "kind": "manual_approval"
            }),
        )
        .unwrap();

        let removed = delete_provider(&mut state, "manual").unwrap();

        assert!(removed.is_some());
        assert!(!state.providers.contains_key("manual"));
    }

    #[test]
    fn repository_helpers_mutate_explicit_repository() {
        let path = unique_state_path("service-config-repository");
        let store = JsonServiceStateStore::new(&path);
        let repository = LockedServiceStateRepository::new(store.clone());

        let profile = upsert_profile_in_repository(
            &repository,
            "journal-downloader",
            json!({
                "name": "Journal Downloader",
                "allocation": "per_service",
                "keyring": "basic_password_store",
                "persistent": true
            }),
        )
        .unwrap();
        let session = upsert_session_in_repository(
            &repository,
            "journal-run",
            json!({
                "serviceName": "JournalDownloader",
                "agentName": "codex",
                "taskName": "probeACSwebsite",
                "profileId": "journal-downloader",
                "lease": "exclusive",
                "cleanup": "close_browser"
            }),
        )
        .unwrap();
        let policy = upsert_site_policy_in_repository(
            &repository,
            "google",
            json!({
                "originPattern": "https://accounts.google.com",
                "interactionMode": "human_like_input"
            }),
        )
        .unwrap();
        let provider = upsert_provider_in_repository(
            &repository,
            "manual",
            json!({
                "displayName": "Dashboard approval",
                "kind": "manual_approval"
            }),
        )
        .unwrap();
        let removed_session = delete_session_in_repository(&repository, "journal-run").unwrap();
        let removed = delete_provider_in_repository(&repository, "manual").unwrap();

        let persisted = store.load().unwrap();
        assert_eq!(profile.id, "journal-downloader");
        assert_eq!(session.id, "journal-run");
        assert_eq!(policy.id, "google");
        assert_eq!(provider.id, "manual");
        assert!(removed_session.is_some());
        assert!(persisted.profiles.contains_key("journal-downloader"));
        assert!(!persisted.sessions.contains_key("journal-run"));
        assert!(removed.is_some());
        assert!(persisted.site_policies.contains_key("google"));
        assert!(!persisted.providers.contains_key("manual"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
