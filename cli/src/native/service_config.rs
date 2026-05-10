//! Persistent service configuration mutations.
//!
//! These helpers keep HTTP and MCP mutation surfaces aligned while preserving
//! the JSON-backed service state as the current durable store.

use serde::Deserialize;
use serde_json::Value;

use super::service_model::{
    profile_seeding_handoff_id, BrowserProfile, BrowserSession, MonitorState,
    ProfileAllocationPolicy, ProfileKeyringPolicy, ProfileReadinessState,
    ProfileSeedingHandoffRecord, ProfileSeedingHandoffState, ProfileSeedingMode,
    ProfileTargetReadiness, ServiceActor, ServiceEntitySource, ServiceProvider, ServiceState,
    SiteMonitor, SitePolicy,
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProfileFreshnessUpdate {
    pub login_id: Option<String>,
    pub site_id: Option<String>,
    pub target_service_id: Option<String>,
    pub target_service_ids: Vec<String>,
    #[serde(default = "default_freshness_readiness_state")]
    pub readiness_state: ProfileReadinessState,
    pub readiness_evidence: Option<String>,
    pub readiness_recommended_action: Option<String>,
    pub last_verified_at: Option<String>,
    pub freshness_expires_at: Option<String>,
    pub update_authenticated_service_ids: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProfileSeedingHandoffUpdate {
    pub target_service_id: Option<String>,
    pub state: Option<ProfileSeedingHandoffState>,
    pub pid: Option<u32>,
    pub started_at: Option<String>,
    pub expires_at: Option<String>,
    pub last_prompted_at: Option<String>,
    pub declared_complete_at: Option<String>,
    pub closed_at: Option<String>,
    pub actor: Option<String>,
    pub note: Option<String>,
}

fn default_freshness_readiness_state() -> ProfileReadinessState {
    ProfileReadinessState::Fresh
}

impl ProfileFreshnessUpdate {
    fn target_ids(&self) -> Vec<String> {
        let mut targets = self.target_service_ids.clone();
        if let Some(target) = self
            .login_id
            .as_deref()
            .or(self.site_id.as_deref())
            .or(self.target_service_id.as_deref())
        {
            targets.push(target.to_string());
        }
        unique_non_empty_strings(targets)
    }

    fn login_id_for_row(&self) -> Option<String> {
        self.login_id
            .clone()
            .or_else(|| self.site_id.clone())
            .or_else(|| self.target_service_id.clone())
    }

    fn should_update_authenticated_service_ids(&self) -> bool {
        self.update_authenticated_service_ids.unwrap_or(true)
    }
}

fn unique_non_empty_strings(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() && !out.iter().any(|existing| existing == trimmed) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn readiness_recommended_action(state: ProfileReadinessState) -> &'static str {
    match state {
        ProfileReadinessState::Fresh => "use_profile",
        ProfileReadinessState::Stale => "probe_target_auth_or_reseed_if_needed",
        ProfileReadinessState::BlockedByAttachedDevtools => {
            "close_attached_devtools_then_verify_profile"
        }
        ProfileReadinessState::NeedsManualSeeding => {
            "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
        }
        ProfileReadinessState::SeededUnknownFreshness => "probe_target_auth_or_reuse_if_acceptable",
        ProfileReadinessState::Unknown => "verify_or_seed_profile_before_authenticated_work",
    }
}

fn profile_seeding_setup_scopes(target_service_id: &str) -> Vec<String> {
    let normalized = target_service_id.to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "google" | "gmail" | "google-login" | "google_signin" | "google-signin"
    ) {
        vec![
            "signin".to_string(),
            "chrome_sync".to_string(),
            "passkeys".to_string(),
            "browser_plugins".to_string(),
        ]
    } else {
        vec!["signin".to_string()]
    }
}

fn readiness_evidence(state: ProfileReadinessState) -> &'static str {
    match state {
        ProfileReadinessState::Fresh => "client_reported_authenticated",
        ProfileReadinessState::Stale => "client_reported_stale",
        ProfileReadinessState::BlockedByAttachedDevtools => "client_reported_attached_devtools",
        ProfileReadinessState::NeedsManualSeeding => "client_reported_manual_seeding_needed",
        ProfileReadinessState::SeededUnknownFreshness => "client_reported_seeded_unknown_freshness",
        ProfileReadinessState::Unknown => "client_reported_unknown",
    }
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
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
    state
        .entity_sources
        .site_policies
        .insert(id.to_string(), ServiceEntitySource::PersistedState);
    Ok(policy)
}

/// Delete one service site-policy record from persisted service state.
pub fn delete_site_policy(
    state: &mut ServiceState,
    id: &str,
) -> Result<Option<SitePolicy>, String> {
    validate_entity_id(id, "site policy")?;
    state.entity_sources.site_policies.remove(id);
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
    state
        .entity_sources
        .profiles
        .insert(id.to_string(), ServiceEntitySource::PersistedState);
    Ok(profile)
}

/// Merge target-readiness freshness evidence into one persisted profile record.
pub fn update_profile_freshness(
    state: &mut ServiceState,
    id: &str,
    update: ProfileFreshnessUpdate,
) -> Result<BrowserProfile, String> {
    validate_entity_id(id, "profile")?;
    let targets = update.target_ids();
    if targets.is_empty() {
        return Err(
            "profile freshness update requires loginId, siteId, targetServiceId, or targetServiceIds"
                .to_string(),
        );
    }
    let Some(profile) = state.profiles.get_mut(id) else {
        return Err(format!("profile '{id}' does not exist"));
    };

    let login_id = update.login_id_for_row();
    let last_verified_at = update.last_verified_at.clone().unwrap_or_else(now_rfc3339);
    for target in &targets {
        if !profile
            .target_service_ids
            .iter()
            .any(|existing| existing == target)
        {
            profile.target_service_ids.push(target.clone());
        }
        let manual_seeding_required =
            update.readiness_state == ProfileReadinessState::NeedsManualSeeding;
        let row =
            ProfileTargetReadiness {
                target_service_id: target.clone(),
                login_id: login_id.clone(),
                state: update.readiness_state,
                manual_seeding_required,
                evidence: update
                    .readiness_evidence
                    .clone()
                    .unwrap_or_else(|| readiness_evidence(update.readiness_state).to_string()),
                recommended_action: update.readiness_recommended_action.clone().unwrap_or_else(
                    || readiness_recommended_action(update.readiness_state).to_string(),
                ),
                seeding_mode: if manual_seeding_required {
                    ProfileSeedingMode::DetachedHeadedNoCdp
                } else {
                    ProfileSeedingMode::NotRequired
                },
                cdp_attachment_allowed_during_seeding: false,
                preferred_keyring: manual_seeding_required
                    .then_some(ProfileKeyringPolicy::BasicPasswordStore),
                setup_scopes: if manual_seeding_required {
                    profile_seeding_setup_scopes(target)
                } else {
                    Vec::new()
                },
                last_verified_at: Some(last_verified_at.clone()),
                freshness_expires_at: update.freshness_expires_at.clone(),
            };
        if let Some(existing) = profile
            .target_readiness
            .iter_mut()
            .find(|row| row.target_service_id == *target)
        {
            *existing = row;
        } else {
            profile.target_readiness.push(row);
        }
    }

    if update.should_update_authenticated_service_ids() {
        for target in &targets {
            if matches!(
                update.readiness_state,
                ProfileReadinessState::Fresh | ProfileReadinessState::SeededUnknownFreshness
            ) {
                if !profile
                    .authenticated_service_ids
                    .iter()
                    .any(|existing| existing == target)
                {
                    profile.authenticated_service_ids.push(target.clone());
                }
            } else {
                profile
                    .authenticated_service_ids
                    .retain(|existing| existing != target);
            }
        }
    }

    validate_profile_policy(profile)?;
    state
        .entity_sources
        .profiles
        .insert(id.to_string(), ServiceEntitySource::PersistedState);
    Ok(profile.clone())
}

/// Upsert one persisted CDP-free profile seeding handoff lifecycle record.
pub fn update_profile_seeding_handoff(
    state: &mut ServiceState,
    profile_id: &str,
    update: ProfileSeedingHandoffUpdate,
) -> Result<ProfileSeedingHandoffRecord, String> {
    validate_entity_id(profile_id, "profile")?;
    if !state.profiles.contains_key(profile_id) {
        return Err(format!("profile '{profile_id}' does not exist"));
    }
    let target_service_id = update
        .target_service_id
        .as_deref()
        .and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then_some(trimmed.to_string())
        })
        .ok_or_else(|| "profile seeding handoff update requires targetServiceId".to_string())?;
    let id = profile_seeding_handoff_id(profile_id, &target_service_id);
    let updated_at = now_rfc3339();
    let record = state
        .profile_seeding_handoffs
        .entry(id.clone())
        .or_insert_with(|| ProfileSeedingHandoffRecord {
            id: id.clone(),
            profile_id: profile_id.to_string(),
            target_service_id: target_service_id.clone(),
            ..ProfileSeedingHandoffRecord::default()
        });
    record.profile_id = profile_id.to_string();
    record.target_service_id = target_service_id;
    if let Some(lifecycle_state) = update.state {
        record.state = lifecycle_state;
    }
    if update.pid.is_some() {
        record.pid = update.pid;
    }
    if update.started_at.is_some() {
        record.started_at = update.started_at;
    }
    if update.expires_at.is_some() {
        record.expires_at = update.expires_at;
    }
    if update.last_prompted_at.is_some() {
        record.last_prompted_at = update.last_prompted_at;
    }
    if update.declared_complete_at.is_some() {
        record.declared_complete_at = update.declared_complete_at;
    }
    if update.closed_at.is_some() {
        record.closed_at = update.closed_at;
    }
    if update.actor.is_some() {
        record.actor = update.actor;
    }
    if update.note.is_some() {
        record.note = update.note;
    }
    record.updated_at = Some(updated_at);
    Ok(record.clone())
}

/// Delete one service profile record from persisted service state.
pub fn delete_profile(
    state: &mut ServiceState,
    id: &str,
) -> Result<Option<BrowserProfile>, String> {
    validate_entity_id(id, "profile")?;
    state.entity_sources.profiles.remove(id);
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

/// Upsert one service monitor record into persisted service state.
pub fn upsert_monitor(
    state: &mut ServiceState,
    id: &str,
    body: Value,
) -> Result<SiteMonitor, String> {
    let body = object_body_with_path_id(body, id, "monitor")?;
    let monitor = serde_json::from_value::<SiteMonitor>(body)
        .map_err(|err| format!("Invalid monitor: {err}"))?;
    state.monitors.insert(id.to_string(), monitor.clone());
    Ok(monitor)
}

/// Delete one service monitor record from persisted service state.
pub fn delete_monitor(state: &mut ServiceState, id: &str) -> Result<Option<SiteMonitor>, String> {
    validate_entity_id(id, "monitor")?;
    Ok(state.monitors.remove(id))
}

/// Update one service monitor's execution state while retaining health history.
pub fn update_monitor_state(
    state: &mut ServiceState,
    id: &str,
    monitor_state: MonitorState,
) -> Result<SiteMonitor, String> {
    validate_entity_id(id, "monitor")?;
    let monitor = state
        .monitors
        .get_mut(id)
        .ok_or_else(|| format!("Unknown monitor id: {id}"))?;
    monitor.state = monitor_state;
    Ok(monitor.clone())
}

/// Clear reviewed monitor failure counts while preserving last failure evidence.
pub fn reset_monitor_failures(state: &mut ServiceState, id: &str) -> Result<SiteMonitor, String> {
    validate_entity_id(id, "monitor")?;
    let monitor = state
        .monitors
        .get_mut(id)
        .ok_or_else(|| format!("Unknown monitor id: {id}"))?;
    monitor.consecutive_failures = 0;
    if monitor.state == MonitorState::Faulted {
        monitor.state = MonitorState::Active;
    }
    Ok(monitor.clone())
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

/// Update one persisted profile's freshness rows under the serialized state mutator.
pub fn update_persisted_profile_freshness(id: &str, body: Value) -> Result<BrowserProfile, String> {
    let update = serde_json::from_value::<ProfileFreshnessUpdate>(body)
        .map_err(|err| format!("Invalid profile freshness update: {err}"))?;
    let repository = LockedServiceStateRepository::default_json()?;
    update_profile_freshness_in_repository(&repository, id, update)
}

/// Update one persisted profile seeding handoff under the serialized state mutator.
pub fn update_persisted_profile_seeding_handoff(
    id: &str,
    body: Value,
) -> Result<ProfileSeedingHandoffRecord, String> {
    let update = serde_json::from_value::<ProfileSeedingHandoffUpdate>(body)
        .map_err(|err| format!("Invalid profile seeding handoff update: {err}"))?;
    let repository = LockedServiceStateRepository::default_json()?;
    update_profile_seeding_handoff_in_repository(&repository, id, update)
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

/// Upsert one persisted monitor record under the serialized state mutator.
pub fn upsert_persisted_monitor(id: &str, body: Value) -> Result<SiteMonitor, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    upsert_monitor_in_repository(&repository, id, body)
}

/// Delete one persisted monitor record under the serialized state mutator.
pub fn delete_persisted_monitor(id: &str) -> Result<Option<SiteMonitor>, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    delete_monitor_in_repository(&repository, id)
}

/// Update one persisted monitor's execution state under the serialized mutator.
pub fn update_persisted_monitor_state(
    id: &str,
    monitor_state: MonitorState,
) -> Result<SiteMonitor, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    update_monitor_state_in_repository(&repository, id, monitor_state)
}

/// Clear one persisted monitor's reviewed failure count under the serialized mutator.
pub fn reset_persisted_monitor_failures(id: &str) -> Result<SiteMonitor, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    reset_monitor_failures_in_repository(&repository, id)
}

pub fn upsert_profile_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
    body: Value,
) -> Result<BrowserProfile, String> {
    repository.mutate(|state| upsert_profile(state, id, body))
}

pub fn update_profile_freshness_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
    update: ProfileFreshnessUpdate,
) -> Result<BrowserProfile, String> {
    repository.mutate(|state| update_profile_freshness(state, id, update))
}

pub fn update_profile_seeding_handoff_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
    update: ProfileSeedingHandoffUpdate,
) -> Result<ProfileSeedingHandoffRecord, String> {
    repository.mutate(|state| update_profile_seeding_handoff(state, id, update))
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

pub fn upsert_monitor_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
    body: Value,
) -> Result<SiteMonitor, String> {
    repository.mutate(|state| upsert_monitor(state, id, body))
}

pub fn delete_monitor_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
) -> Result<Option<SiteMonitor>, String> {
    repository.mutate(|state| delete_monitor(state, id))
}

pub fn update_monitor_state_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
    monitor_state: MonitorState,
) -> Result<SiteMonitor, String> {
    repository.mutate(|state| update_monitor_state(state, id, monitor_state))
}

pub fn reset_monitor_failures_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
) -> Result<SiteMonitor, String> {
    repository.mutate(|state| reset_monitor_failures(state, id))
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
    fn update_profile_freshness_merges_readiness_and_auth_targets() {
        let mut state = ServiceState::default();
        upsert_profile(
            &mut state,
            "journal-google",
            json!({
                "name": "Journal Google",
                "allocation": "per_service",
                "keyring": "basic_password_store",
                "persistent": true,
                "targetServiceIds": ["google"],
                "authenticatedServiceIds": ["google"],
                "sharedServiceIds": ["JournalDownloader"],
                "targetReadiness": [{
                    "targetServiceId": "google",
                    "loginId": "google",
                    "state": "fresh",
                    "manualSeedingRequired": false,
                    "evidence": "auth_probe_cookie_present",
                    "recommendedAction": "use_profile",
                    "lastVerifiedAt": "2026-05-06T12:00:00Z",
                    "freshnessExpiresAt": "2026-05-06T13:00:00Z"
                }]
            }),
        )
        .unwrap();

        let profile = update_profile_freshness(
            &mut state,
            "journal-google",
            serde_json::from_value(json!({
                "loginId": "google",
                "readinessState": "stale",
                "readinessEvidence": "auth_probe_cookie_missing",
                "lastVerifiedAt": "2026-05-06T14:00:00Z"
            }))
            .unwrap(),
        )
        .unwrap();

        assert_eq!(profile.authenticated_service_ids, Vec::<String>::new());
        assert_eq!(profile.target_readiness.len(), 1);
        assert_eq!(
            profile.target_readiness[0].state,
            ProfileReadinessState::Stale
        );
        assert_eq!(
            profile.target_readiness[0].evidence,
            "auth_probe_cookie_missing"
        );
        assert_eq!(
            profile.target_readiness[0].last_verified_at.as_deref(),
            Some("2026-05-06T14:00:00Z")
        );
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
    fn upsert_monitor_sets_path_id_and_defaults() {
        let mut state = ServiceState::default();

        let monitor = upsert_monitor(
            &mut state,
            "google-login-freshness",
            json!({
                "name": "Google login freshness",
                "target": {"site_policy": "google"},
                "intervalMs": 60000,
                "state": "paused"
            }),
        )
        .unwrap();

        assert_eq!(monitor.id, "google-login-freshness");
        assert_eq!(monitor.name, "Google login freshness");
        assert!(state.monitors.contains_key("google-login-freshness"));
    }

    #[test]
    fn delete_monitor_reports_removed_record() {
        let mut state = ServiceState::default();
        upsert_monitor(
            &mut state,
            "google-login-freshness",
            json!({
                "name": "Google login freshness",
                "target": {"site_policy": "google"},
                "intervalMs": 60000,
                "state": "paused"
            }),
        )
        .unwrap();

        let removed = delete_monitor(&mut state, "google-login-freshness").unwrap();

        assert!(removed.is_some());
        assert!(!state.monitors.contains_key("google-login-freshness"));
    }

    #[test]
    fn update_monitor_state_preserves_health_history() {
        let mut state = ServiceState::default();
        upsert_monitor(
            &mut state,
            "google-login-freshness",
            json!({
                "name": "Google login freshness",
                "target": {"site_policy": "google"},
                "intervalMs": 60000,
                "state": "faulted",
                "lastFailedAt": "2026-05-07T00:00:00Z",
                "lastResult": "login_stale",
                "consecutiveFailures": 3
            }),
        )
        .unwrap();

        let monitor =
            update_monitor_state(&mut state, "google-login-freshness", MonitorState::Paused)
                .unwrap();

        assert_eq!(monitor.state, MonitorState::Paused);
        assert_eq!(
            monitor.last_failed_at.as_deref(),
            Some("2026-05-07T00:00:00Z")
        );
        assert_eq!(monitor.consecutive_failures, 3);
    }

    #[test]
    fn update_monitor_state_rejects_unknown_monitor() {
        let mut state = ServiceState::default();

        let err = update_monitor_state(&mut state, "missing", MonitorState::Active).unwrap_err();

        assert!(err.contains("Unknown monitor id"));
    }

    #[test]
    fn reset_monitor_failures_clears_counter_and_preserves_evidence() {
        let mut state = ServiceState::default();
        upsert_monitor(
            &mut state,
            "google-login-freshness",
            json!({
                "name": "Google login freshness",
                "target": {"site_policy": "google"},
                "intervalMs": 60000,
                "state": "faulted",
                "lastFailedAt": "2026-05-07T00:00:00Z",
                "lastResult": "login_stale",
                "consecutiveFailures": 3
            }),
        )
        .unwrap();

        let monitor = reset_monitor_failures(&mut state, "google-login-freshness").unwrap();

        assert_eq!(monitor.state, MonitorState::Active);
        assert_eq!(monitor.consecutive_failures, 0);
        assert_eq!(
            monitor.last_failed_at.as_deref(),
            Some("2026-05-07T00:00:00Z")
        );
        assert_eq!(monitor.last_result.as_deref(), Some("login_stale"));
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
        let monitor = upsert_monitor_in_repository(
            &repository,
            "google-login-freshness",
            json!({
                "name": "Google login freshness",
                "target": {"site_policy": "google"},
                "intervalMs": 60000,
                "state": "paused"
            }),
        )
        .unwrap();
        let resumed_monitor = update_monitor_state_in_repository(
            &repository,
            "google-login-freshness",
            MonitorState::Active,
        )
        .unwrap();
        let reset_monitor =
            reset_monitor_failures_in_repository(&repository, "google-login-freshness").unwrap();
        let removed_session = delete_session_in_repository(&repository, "journal-run").unwrap();
        let removed = delete_provider_in_repository(&repository, "manual").unwrap();
        let removed_monitor =
            delete_monitor_in_repository(&repository, "google-login-freshness").unwrap();

        let persisted = store.load().unwrap();
        assert_eq!(profile.id, "journal-downloader");
        assert_eq!(session.id, "journal-run");
        assert_eq!(policy.id, "google");
        assert_eq!(provider.id, "manual");
        assert_eq!(monitor.id, "google-login-freshness");
        assert_eq!(resumed_monitor.state, MonitorState::Active);
        assert_eq!(reset_monitor.consecutive_failures, 0);
        assert!(removed_session.is_some());
        assert!(persisted.profiles.contains_key("journal-downloader"));
        assert!(!persisted.sessions.contains_key("journal-run"));
        assert!(removed.is_some());
        assert!(removed_monitor.is_some());
        assert!(persisted.site_policies.contains_key("google"));
        assert!(!persisted.providers.contains_key("manual"));
        assert!(!persisted.monitors.contains_key("google-login-freshness"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
