//! Versioned, staged migration for the primary Service State document.
//!
//! Legacy unversioned JSON is an input format only. It never grants principal
//! authority from service, agent, task, session, or profile labels. The
//! existing principal registry and exact runtime-owner bindings remain the
//! only migration inputs that can produce effect-capable lease authority.

use super::service_model::{BrowserProfile, ServiceState};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const SERVICE_STATE_SCHEMA_VERSION: &str = "agent-browser.service-state.v2";
pub(crate) const LEGACY_SERVICE_STATE_SCHEMA_VERSION: &str =
    "agent-browser.service-state.unversioned";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ServiceStateMigrationStatus {
    NotRequired,
    Required,
    BlockedNewerSchema,
    BlockedInvariant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceStateMigrationPlan {
    pub(crate) schema_version: &'static str,
    pub(crate) source_state_schema: String,
    pub(crate) target_state_schema: &'static str,
    pub(crate) source_profile_lease_schema: Option<String>,
    pub(crate) target_profile_lease_schema: &'static str,
    pub(crate) status: ServiceStateMigrationStatus,
    pub(crate) forward_reader_available: bool,
    pub(crate) old_reader_compatible: bool,
    pub(crate) principal_authority_policy: &'static str,
    pub(crate) stable_identity_policy: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StagedServiceStateMigration {
    pub(crate) plan: ServiceStateMigrationPlan,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) fn read_service_state(raw: &str) -> Result<ServiceState, serde_json::Error> {
    let value: Value = serde_json::from_str(raw)?;
    let source_schema = value.get("schemaVersion").and_then(Value::as_str);
    if let Some(schema) = source_schema {
        if schema != SERVICE_STATE_SCHEMA_VERSION {
            return Err(custom_json_error(format!(
                "service_state_schema_unsupported:{schema}"
            )));
        }
    }
    if let Some(schema) = value
        .get("profileLeaseSchemaVersion")
        .and_then(Value::as_str)
    {
        if schema != super::service_profile_lease::PROFILE_LEASE_SCHEMA_VERSION {
            return Err(custom_json_error(format!(
                "profile_lease_schema_unsupported:{schema}"
            )));
        }
    }
    let mut state: ServiceState = serde_json::from_value(value)?;
    stamp_current_versions(&mut state);
    Ok(state)
}

pub(crate) fn prepare_service_state_for_persistence(
    state: &mut ServiceState,
) -> Result<(), String> {
    stamp_current_versions(state);
    Ok(())
}

pub(crate) fn plan_service_state_migration(raw: &str) -> Result<ServiceStateMigrationPlan, String> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| format!("service_state_migration_json_invalid:{error}"))?;
    let source_state_schema = value
        .get("schemaVersion")
        .and_then(Value::as_str)
        .unwrap_or(LEGACY_SERVICE_STATE_SCHEMA_VERSION)
        .to_string();
    let source_profile_lease_schema = value
        .get("profileLeaseSchemaVersion")
        .and_then(Value::as_str)
        .map(str::to_string);
    let status = if source_state_schema == SERVICE_STATE_SCHEMA_VERSION {
        match source_profile_lease_schema.as_deref() {
            Some(schema)
                if schema == super::service_profile_lease::PROFILE_LEASE_SCHEMA_VERSION =>
            {
                ServiceStateMigrationStatus::NotRequired
            }
            None => ServiceStateMigrationStatus::Required,
            Some(_) => ServiceStateMigrationStatus::BlockedNewerSchema,
        }
    } else if source_state_schema == LEGACY_SERVICE_STATE_SCHEMA_VERSION {
        ServiceStateMigrationStatus::Required
    } else {
        ServiceStateMigrationStatus::BlockedNewerSchema
    };
    Ok(ServiceStateMigrationPlan {
        schema_version: "agent-browser.service-state-migration-plan.v1",
        source_state_schema,
        target_state_schema: SERVICE_STATE_SCHEMA_VERSION,
        source_profile_lease_schema,
        target_profile_lease_schema: super::service_profile_lease::PROFILE_LEASE_SCHEMA_VERSION,
        forward_reader_available: status != ServiceStateMigrationStatus::BlockedNewerSchema,
        old_reader_compatible: true,
        status,
        principal_authority_policy: "authenticated_capability_or_exact_current_owner_only",
        stable_identity_policy: "preserve_stable_ids_refresh_ephemeral_evidence",
    })
}

pub(crate) fn stage_service_state_migration(
    raw: &str,
) -> Result<StagedServiceStateMigration, String> {
    let plan = plan_service_state_migration(raw)?;
    if !plan.forward_reader_available {
        return Err(format!(
            "service_state_migration_blocked_newer_schema:{}",
            plan.source_state_schema
        ));
    }
    let mut state = read_service_state(raw).map_err(|error| error.to_string())?;
    prepare_service_state_for_persistence(&mut state)?;
    materialize_inert_legacy_profile_placeholders(&mut state);
    // Full cross-projection integrity is an installation migration commit
    // gate. Ordinary runtime writes can update related projections in
    // multiple repository mutations and must remain able to converge them.
    validate_service_state_invariants(&state)?;
    let mut bytes = serde_json::to_vec_pretty(&state)
        .map_err(|error| format!("service_state_migration_serialize_failed:{error}"))?;
    bytes.push(b'\n');
    Ok(StagedServiceStateMigration { plan, bytes })
}

/// Preserve an inert legacy lease whose profile row was never persisted.
///
/// A placeholder restores only referential identity. It cannot manufacture
/// principal authority because the source session has no principal, work
/// capability, browser, or tab binding. Any dangling profile reference with
/// effect-bearing evidence remains untouched and fails invariant validation.
fn materialize_inert_legacy_profile_placeholders(state: &mut ServiceState) {
    let missing_profiles = state
        .sessions
        .values()
        .filter_map(|session| {
            let profile_id = session.profile_id.as_ref()?;
            (!profile_id.trim().is_empty()
                && !state.profiles.contains_key(profile_id)
                && session.principal_id.is_none()
                && session.work_lease_id.is_none()
                && session.browser_ids.is_empty()
                && session.tab_ids.is_empty())
            .then(|| profile_id.clone())
        })
        .collect::<std::collections::BTreeSet<_>>();

    for profile_id in missing_profiles {
        state.profiles.insert(
            profile_id.clone(),
            BrowserProfile {
                id: profile_id.clone(),
                name: profile_id,
                description: Some(
                    "Migrated placeholder for an inert legacy session; principal authority remains unproven."
                        .to_string(),
                ),
                persistent: true,
                ..BrowserProfile::default()
            },
        );
    }
}

fn stamp_current_versions(state: &mut ServiceState) {
    state.schema_version = SERVICE_STATE_SCHEMA_VERSION.to_string();
    state.profile_lease_schema_version =
        super::service_profile_lease::PROFILE_LEASE_SCHEMA_VERSION.to_string();
}

fn validate_service_state_invariants(state: &ServiceState) -> Result<(), String> {
    for (key, profile) in &state.profiles {
        if profile.id.trim().is_empty() || profile.id != *key {
            return Err(format!("service_state_profile_key_mismatch:{key}"));
        }
    }
    for (key, browser) in &state.browsers {
        if browser.id.trim().is_empty() || browser.id != *key {
            return Err(format!("service_state_browser_key_mismatch:{key}"));
        }
        if let Some(profile_id) = &browser.profile_id {
            if !state.profiles.contains_key(profile_id) {
                return Err(format!(
                    "service_state_browser_profile_missing:{key}:{profile_id}"
                ));
            }
        }
        if let Some(allocation_id) = &browser.display_allocation_id {
            if !state.display_allocations.contains_key(allocation_id) {
                return Err(format!(
                    "service_state_browser_display_missing:{key}:{allocation_id}"
                ));
            }
        }
        for session_id in &browser.active_session_ids {
            if !state.sessions.contains_key(session_id) {
                return Err(format!(
                    "service_state_browser_session_missing:{key}:{session_id}"
                ));
            }
        }
    }
    for (key, session) in &state.sessions {
        if session.id.trim().is_empty() || session.id != *key {
            return Err(format!("service_state_session_key_mismatch:{key}"));
        }
        if let Some(profile_id) = &session.profile_id {
            if !state.profiles.contains_key(profile_id) {
                return Err(format!(
                    "service_state_session_profile_missing:{key}:{profile_id}"
                ));
            }
        }
        for browser_id in &session.browser_ids {
            if !state.browsers.contains_key(browser_id) {
                return Err(format!(
                    "service_state_session_browser_missing:{key}:{browser_id}"
                ));
            }
        }
        for tab_id in &session.tab_ids {
            if !state.tabs.contains_key(tab_id) {
                return Err(format!("service_state_session_tab_missing:{key}:{tab_id}"));
            }
        }
    }
    for (key, tab) in &state.tabs {
        if tab.id.trim().is_empty() || tab.id != *key {
            return Err(format!("service_state_tab_key_mismatch:{key}"));
        }
        if !tab.browser_id.trim().is_empty() && !state.browsers.contains_key(&tab.browser_id) {
            return Err(format!(
                "service_state_tab_browser_missing:{key}:{}",
                tab.browser_id
            ));
        }
        if let Some(session_id) = &tab.owner_session_id {
            if !state.sessions.contains_key(session_id) {
                return Err(format!(
                    "service_state_tab_session_missing:{key}:{session_id}"
                ));
            }
        }
        if let Some(session_id) = &tab.session_id {
            if !state.sessions.contains_key(session_id) {
                return Err(format!(
                    "service_state_tab_routed_session_missing:{key}:{session_id}"
                ));
            }
        }
    }
    for key in state.browser_process_identities.keys() {
        if !state.browsers.contains_key(key) {
            return Err(format!("service_state_process_browser_missing:{key}"));
        }
    }
    for (key, allocation) in &state.display_allocations {
        if allocation.id.trim().is_empty() || allocation.id != *key {
            return Err(format!("service_state_display_key_mismatch:{key}"));
        }
        if let Some(browser_id) = &allocation.owner_browser_id {
            if !state.browsers.contains_key(browser_id) {
                return Err(format!(
                    "service_state_display_browser_missing:{key}:{browser_id}"
                ));
            }
        }
        if let Some(session_id) = &allocation.owner_session_id {
            if !state.sessions.contains_key(session_id) {
                return Err(format!(
                    "service_state_display_session_missing:{key}:{session_id}"
                ));
            }
        }
        if let Some(profile_id) = &allocation.profile_id {
            if !state.profiles.contains_key(profile_id) {
                return Err(format!(
                    "service_state_display_profile_missing:{key}:{profile_id}"
                ));
            }
        }
        for route_id in &allocation.route_ids {
            if !state.remote_view_routes.contains_key(route_id) {
                return Err(format!(
                    "service_state_display_route_missing:{key}:{route_id}"
                ));
            }
        }
    }
    for (key, route) in &state.remote_view_routes {
        if route.id.trim().is_empty() || route.id != *key {
            return Err(format!("service_state_route_key_mismatch:{key}"));
        }
        if let Some(allocation_id) = &route.display_allocation_id {
            if !state.display_allocations.contains_key(allocation_id) {
                return Err(format!(
                    "service_state_route_display_missing:{key}:{allocation_id}"
                ));
            }
        }
        if let Some(browser_id) = &route.browser_id {
            if !state.browsers.contains_key(browser_id) {
                return Err(format!(
                    "service_state_route_browser_missing:{key}:{browser_id}"
                ));
            }
        }
        if let Some(session_id) = &route.session_id {
            if !state.sessions.contains_key(session_id) {
                return Err(format!(
                    "service_state_route_session_missing:{key}:{session_id}"
                ));
            }
        }
        for lease_id in &route.viewer_lease_ids {
            if !state.viewer_leases.contains_key(lease_id) {
                return Err(format!(
                    "service_state_route_viewer_lease_missing:{key}:{lease_id}"
                ));
            }
        }
        if let Some(lease_id) = &route.controller_lease_id {
            if !state.viewer_leases.contains_key(lease_id) {
                return Err(format!(
                    "service_state_route_controller_lease_missing:{key}:{lease_id}"
                ));
            }
        }
    }
    for (key, entry) in &state.route_pool {
        if entry.id.trim().is_empty() || entry.id != *key {
            return Err(format!("service_state_route_pool_key_mismatch:{key}"));
        }
    }
    for (key, lease) in &state.remote_view_acquisition_leases {
        if lease.id.trim().is_empty() || lease.id != *key {
            return Err(format!(
                "service_state_acquisition_lease_key_mismatch:{key}"
            ));
        }
    }
    for (key, lease) in &state.viewer_leases {
        if lease.id.trim().is_empty() || lease.id != *key {
            return Err(format!("service_state_viewer_lease_key_mismatch:{key}"));
        }
    }
    for (key, handoff) in &state.remote_view_handoffs {
        if handoff.id.trim().is_empty() || handoff.id != *key {
            return Err(format!("service_state_handoff_key_mismatch:{key}"));
        }
    }
    for (key, handoff) in &state.profile_seeding_handoffs {
        if handoff.id.trim().is_empty() || handoff.id != *key {
            return Err(format!("service_state_seeding_handoff_key_mismatch:{key}"));
        }
        if !state.profiles.contains_key(&handoff.profile_id) {
            return Err(format!(
                "service_state_seeding_handoff_profile_missing:{key}:{}",
                handoff.profile_id
            ));
        }
    }
    for (profile_digest, binding) in &state.runtime_owner_registry.principal_bindings {
        if profile_digest != &binding.profile_identity_digest
            || !state
                .runtime_owner_registry
                .owners
                .contains_key(profile_digest)
        {
            return Err(format!(
                "service_state_principal_owner_binding_mismatch:{profile_digest}"
            ));
        }
        if !state.profiles.contains_key(&binding.profile_id) {
            return Err(format!(
                "service_state_principal_profile_missing:{}",
                binding.profile_id
            ));
        }
        if !state
            .service_principals
            .profile_capabilities
            .contains_key(&binding.capability_id)
        {
            return Err(format!(
                "service_state_principal_capability_missing:{}",
                binding.capability_id
            ));
        }
    }
    Ok(())
}

fn custom_json_error(message: String) -> serde_json::Error {
    <serde_json::Error as serde::de::Error>::custom(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn legacy_labels_remain_unproven_and_stage_without_effects() {
        let raw = json!({
            "profiles": {
                "fedex": { "id": "fedex", "name": "FedEx" }
            },
            "sessions": {
                "odollo": {
                    "id": "odollo",
                    "serviceName": "Odollo fulfillment",
                    "profileId": "fedex",
                    "lease": "exclusive"
                }
            }
        })
        .to_string();
        let staged = stage_service_state_migration(&raw).unwrap();
        assert_eq!(staged.plan.status, ServiceStateMigrationStatus::Required);
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(migrated["schemaVersion"], SERVICE_STATE_SCHEMA_VERSION);
        assert_eq!(
            migrated["profileLeaseSchemaVersion"],
            super::super::service_profile_lease::PROFILE_LEASE_SCHEMA_VERSION
        );
        assert!(migrated
            .get("servicePrincipals")
            .and_then(Value::as_object)
            .is_none_or(|registry| registry.is_empty()));
    }

    #[test]
    fn unknown_newer_schema_is_read_only_and_bytes_are_unchanged() {
        let raw = r#"{"schemaVersion":"agent-browser.service-state.v99","profiles":{}}"#;
        let before = raw.as_bytes().to_vec();
        let plan = plan_service_state_migration(raw).unwrap();
        assert_eq!(plan.status, ServiceStateMigrationStatus::BlockedNewerSchema);
        assert!(!plan.forward_reader_available);
        assert!(stage_service_state_migration(raw)
            .unwrap_err()
            .contains("blocked_newer_schema"));
        assert_eq!(raw.as_bytes(), before);
    }

    #[test]
    fn unknown_lease_schema_blocks_without_rewriting_current_state() {
        let raw = format!(
            r#"{{"schemaVersion":"{SERVICE_STATE_SCHEMA_VERSION}","profileLeaseSchemaVersion":"agent-browser.profile-leases.v99"}}"#
        );
        let before = raw.as_bytes().to_vec();
        let plan = plan_service_state_migration(&raw).unwrap();
        assert_eq!(plan.status, ServiceStateMigrationStatus::BlockedNewerSchema);
        assert!(!plan.forward_reader_available);
        assert!(read_service_state(&raw)
            .unwrap_err()
            .to_string()
            .contains("profile_lease_schema_unsupported"));
        assert!(stage_service_state_migration(&raw)
            .unwrap_err()
            .contains("blocked_newer_schema"));
        assert_eq!(raw.as_bytes(), before);
    }

    #[test]
    fn staged_additive_versions_remain_readable_by_a_legacy_projection() {
        #[derive(Debug, Default, Deserialize)]
        #[serde(default, rename_all = "camelCase")]
        struct LegacyProjection {
            profiles:
                std::collections::BTreeMap<String, super::super::service_model::BrowserProfile>,
            sessions:
                std::collections::BTreeMap<String, super::super::service_model::BrowserSession>,
        }

        let raw = json!({
            "profiles": {"fedex": {"id": "fedex", "name": "FedEx"}},
            "sessions": {}
        })
        .to_string();
        let staged = stage_service_state_migration(&raw).unwrap();
        assert!(staged.plan.old_reader_compatible);
        let legacy: LegacyProjection = serde_json::from_slice(&staged.bytes).unwrap();
        assert!(legacy.profiles.contains_key("fedex"));
        assert!(legacy.sessions.is_empty());
    }

    #[test]
    fn inert_legacy_session_materializes_missing_profile_without_authority() {
        let raw = json!({
            "sessions": {
                "holder": {
                    "id": "holder",
                    "owner": "system",
                    "lease": "exclusive",
                    "profileId": "work",
                    "browserIds": [],
                    "tabIds": []
                }
            }
        })
        .to_string();

        let staged = stage_service_state_migration(&raw).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(migrated["profiles"]["work"]["id"], "work");
        assert_eq!(migrated["profiles"]["work"]["name"], "work");
        assert_eq!(migrated["profiles"]["work"]["persistent"], true);
        assert_eq!(
            migrated["profiles"]["work"]["description"],
            "Migrated placeholder for an inert legacy session; principal authority remains unproven."
        );
        assert!(migrated
            .get("servicePrincipals")
            .and_then(Value::as_object)
            .is_none_or(|registry| registry.is_empty()));
    }

    #[test]
    fn principal_bound_session_with_missing_profile_stays_blocked() {
        let raw = json!({
            "sessions": {
                "holder": {
                    "id": "holder",
                    "principalId": "service-a",
                    "lease": "exclusive",
                    "profileId": "work",
                    "browserIds": [],
                    "tabIds": []
                }
            }
        })
        .to_string();

        assert_eq!(
            stage_service_state_migration(&raw).unwrap_err(),
            "service_state_session_profile_missing:holder:work"
        );
    }

    #[test]
    fn broken_references_fail_before_staged_bytes_exist() {
        let raw = json!({
            "sessions": {
                "odollo": {
                    "id": "odollo",
                    "browserIds": ["missing-browser"],
                    "lease": "exclusive"
                }
            }
        })
        .to_string();
        assert!(stage_service_state_migration(&raw)
            .unwrap_err()
            .contains("service_state_session_browser_missing"));
    }
}
