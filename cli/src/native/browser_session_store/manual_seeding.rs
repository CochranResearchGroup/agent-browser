//! SQLite custody for a CDP-free manual-seeding acquisition.

use std::collections::BTreeMap;
use std::path::Path;

use agent_browser_service_model::{
    BrowserProfileKind, RecordedProcessIdentity, RouteKeeperFence, RouteKeeperHandoffBinding,
};
use rusqlite::TransactionBehavior;
use serde::{Deserialize, Serialize};

use super::{
    load_optional_document, load_route_keeper_authority_document, observe_operation_in_transaction,
    reserve_operation_in_transaction, save_document, BrowserProfileCatalog,
    BrowserRuntimeOperation, BrowserRuntimeSqliteStore, BROWSER_PROFILE_CATALOG_SCHEMA_V1,
    PROFILE_CATALOG_DOCUMENT,
};

const MANUAL_SEEDING_DOCUMENT: &str = "manual_seeding_registry";
const MANUAL_SEEDING_SCHEMA_V1: &str = "agent-browser.manual-seeding-registry.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ManualSeedingState {
    Reserved,
    LaunchObserved,
    Ready,
    RecoveryRequired,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ManualSeedingReservation {
    pub(crate) operation_id: String,
    pub(crate) profile_id: String,
    pub(crate) target_service_id: String,
    pub(crate) handoff_id: String,
    pub(crate) requested_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ManualSeedingRecord {
    pub(crate) operation_id: String,
    pub(crate) generation: u64,
    pub(crate) profile_id: String,
    pub(crate) target_service_id: String,
    pub(crate) handoff_id: String,
    pub(crate) user_data_dir: String,
    pub(crate) requested_url: Option<String>,
    pub(crate) state: ManualSeedingState,
    pub(crate) route_slot_id: Option<String>,
    pub(crate) display_name: Option<String>,
    pub(crate) route_fence: Option<RouteKeeperFence>,
    pub(crate) process_identity: Option<RecordedProcessIdentity>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManualSeedingRegistry {
    records: BTreeMap<String, ManualSeedingRecord>,
}

impl BrowserRuntimeSqliteStore {
    /// Reserve one named profile for one seeding target in the same transaction
    /// that creates its operation generation. This performs no browser or
    /// provider effect and publishes no ready handoff.
    #[allow(dead_code)]
    pub(crate) fn reserve_manual_seeding(
        &mut self,
        request: &ManualSeedingReservation,
    ) -> Result<(ManualSeedingRecord, BrowserRuntimeOperation), String> {
        if [
            request.operation_id.as_str(),
            request.profile_id.as_str(),
            request.target_service_id.as_str(),
            request.handoff_id.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err("manual_seeding_identity_invalid".to_string());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("manual_seeding_reservation_begin_failed:{error}"))?;
        let catalog: BrowserProfileCatalog = load_optional_document(
            &transaction,
            PROFILE_CATALOG_DOCUMENT,
            BROWSER_PROFILE_CATALOG_SCHEMA_V1,
        )?;
        let profile = catalog
            .profiles
            .get(&request.profile_id)
            .filter(|profile| profile.id == request.profile_id)
            .ok_or_else(|| {
                format!(
                    "manual_seeding_profile_not_registered:{}",
                    request.profile_id
                )
            })?;
        if profile.kind != BrowserProfileKind::Named
            || !Path::new(&profile.user_data_dir).is_absolute()
        {
            return Err(format!(
                "manual_seeding_named_profile_required:{}",
                request.profile_id
            ));
        }
        let mut registry: ManualSeedingRegistry = load_optional_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
        )?;
        if let Some(existing) = registry.records.get(&request.profile_id) {
            if existing.state != ManualSeedingState::Closed {
                if existing.operation_id == request.operation_id
                    && existing.target_service_id == request.target_service_id
                    && existing.handoff_id == request.handoff_id
                    && existing.requested_url == request.requested_url
                    && existing.user_data_dir == profile.user_data_dir
                {
                    let operation =
                        super::load_optional_operation(&transaction, &request.operation_id)?
                            .ok_or_else(|| {
                                "manual_seeding_reservation_operation_missing".to_string()
                            })?;
                    if operation.generation != existing.generation
                        || operation.request
                            != serde_json::to_value(request).map_err(|error| {
                                format!("manual_seeding_request_serialize_failed:{error}")
                            })?
                    {
                        return Err("manual_seeding_reservation_operation_mismatch".to_string());
                    }
                    return Ok((existing.clone(), operation));
                }
                return Err(format!(
                    "manual_seeding_profile_busy:{}",
                    request.profile_id
                ));
            }
            if existing.handoff_id == request.handoff_id {
                return Err("manual_seeding_closed_handoff_reuse_forbidden".to_string());
            }
        }
        let owner_key = format!("manual-seeding-profile:{}", request.profile_id);
        let operation = reserve_operation_in_transaction(
            &transaction,
            &request.operation_id,
            &owner_key,
            serde_json::to_value(request)
                .map_err(|error| format!("manual_seeding_request_serialize_failed:{error}"))?,
        )?;
        let record = ManualSeedingRecord {
            operation_id: request.operation_id.clone(),
            generation: operation.generation,
            profile_id: request.profile_id.clone(),
            target_service_id: request.target_service_id.clone(),
            handoff_id: request.handoff_id.clone(),
            user_data_dir: profile.user_data_dir.clone(),
            requested_url: request.requested_url.clone(),
            state: ManualSeedingState::Reserved,
            route_slot_id: None,
            display_name: None,
            route_fence: None,
            process_identity: None,
        };
        registry
            .records
            .insert(request.profile_id.clone(), record.clone());
        save_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
            &registry,
        )?;
        transaction
            .commit()
            .map_err(|error| format!("manual_seeding_reservation_commit_failed:{error}"))?;
        Ok((record, operation))
    }

    /// Journal one exact detached process and current provider-owned route in
    /// the same transaction. A later ready handoff must recheck both witnesses.
    #[allow(dead_code)]
    pub(crate) fn observe_manual_seeding_launch(
        &mut self,
        profile_id: &str,
        operation_id: &str,
        generation: u64,
        binding: &RouteKeeperHandoffBinding,
        process_identity: &RecordedProcessIdentity,
    ) -> Result<(ManualSeedingRecord, BrowserRuntimeOperation), String> {
        if process_identity.pid == 0 || process_identity.start_token.trim().is_empty() {
            return Err("manual_seeding_process_identity_invalid".to_string());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("manual_seeding_observation_begin_failed:{error}"))?;
        let mut registry: ManualSeedingRegistry = load_optional_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
        )?;
        let record = registry
            .records
            .get_mut(profile_id)
            .ok_or_else(|| format!("manual_seeding_reservation_missing:{profile_id}"))?;
        if record.operation_id != operation_id || record.generation != generation {
            return Err("manual_seeding_operation_identity_mismatch".to_string());
        }
        if record.state != ManualSeedingState::Reserved
            && record.state != ManualSeedingState::LaunchObserved
        {
            return Err("manual_seeding_observation_state_invalid".to_string());
        }
        let authority = load_route_keeper_authority_document(&transaction)?;
        let current = authority.ready_handoff_binding(&binding.slot_id, &binding.display_name)?;
        if current != *binding {
            return Err("manual_seeding_route_binding_changed".to_string());
        }
        if record.state == ManualSeedingState::LaunchObserved
            && (record.route_slot_id.as_deref() != Some(binding.slot_id.as_str())
                || record.display_name.as_deref() != Some(binding.display_name.as_str())
                || record.route_fence.as_ref() != Some(&binding.fence)
                || record.process_identity.as_ref() != Some(process_identity))
        {
            return Err("manual_seeding_launch_observation_changed".to_string());
        }
        let observation = serde_json::json!({
            "phase": "manual_seeding_launch_observed",
            "profileId": profile_id,
            "handoffId": record.handoff_id,
            "routeSlotId": binding.slot_id,
            "displayName": binding.display_name,
            "routeFence": binding.fence,
            "processIdentity": process_identity,
        });
        let operation =
            observe_operation_in_transaction(&transaction, operation_id, generation, observation)?;
        if operation.owner_key != format!("manual-seeding-profile:{profile_id}") {
            return Err("manual_seeding_operation_owner_mismatch".to_string());
        }
        record.state = ManualSeedingState::LaunchObserved;
        record.route_slot_id = Some(binding.slot_id.clone());
        record.display_name = Some(binding.display_name.clone());
        record.route_fence = Some(binding.fence.clone());
        record.process_identity = Some(process_identity.clone());
        let updated = record.clone();
        save_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
            &registry,
        )?;
        transaction
            .commit()
            .map_err(|error| format!("manual_seeding_observation_commit_failed:{error}"))?;
        Ok((updated, operation))
    }

    #[allow(dead_code)]
    pub(crate) fn load_manual_seeding_record(
        &self,
        profile_id: &str,
    ) -> Result<Option<ManualSeedingRecord>, String> {
        let registry: ManualSeedingRegistry = load_optional_document(
            &self.connection,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
        )?;
        Ok(registry.records.get(profile_id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use agent_browser_service_model::BrowserProfileCatalogEntry;

    use super::*;
    use crate::native::browser_session_store::LegacyBrowserRuntimeSources;

    #[test]
    fn manual_seeding_reservation_is_sqlite_owned_and_profile_exclusive() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-manual-seeding-sqlite-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let database_path = root.join("runtime.sqlite3");
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &root.join("browser-session-state.json"),
                profile_catalog_path: &root.join("browser-profile-catalog.json"),
                service_state_path: &root.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let mut catalog = store.load_profile_catalog().unwrap();
        catalog.profiles.insert(
            "work".to_string(),
            BrowserProfileCatalogEntry {
                id: "work".to_string(),
                name: "Work".to_string(),
                user_data_dir: root.join("profile").to_string_lossy().into_owned(),
                kind: BrowserProfileKind::Named,
            },
        );
        store.save_profile_catalog(&catalog).unwrap();
        let request = ManualSeedingReservation {
            operation_id: "seed-operation-a".to_string(),
            profile_id: "work".to_string(),
            target_service_id: "service-a".to_string(),
            handoff_id: "seed-handoff-a".to_string(),
            requested_url: Some("https://example.test/login".to_string()),
        };
        let first = store.reserve_manual_seeding(&request).unwrap();
        assert_eq!(first.0.state, ManualSeedingState::Reserved);
        assert_eq!(first.0.generation, 1);
        assert_eq!(
            first.1.state,
            super::super::BrowserRuntimeOperationState::Prepared
        );
        assert_eq!(store.reserve_manual_seeding(&request).unwrap(), first);
        assert!(store.load_handoff_registry().unwrap().handoffs.is_empty());

        let conflicting = ManualSeedingReservation {
            operation_id: "seed-operation-b".to_string(),
            target_service_id: "service-b".to_string(),
            handoff_id: "seed-handoff-b".to_string(),
            ..request.clone()
        };
        assert_eq!(
            store.reserve_manual_seeding(&conflicting),
            Err("manual_seeding_profile_busy:work".to_string())
        );
        assert!(store.find_operation("seed-operation-b").unwrap().is_none());
        assert_eq!(
            store.reserve_manual_seeding(&ManualSeedingReservation {
                profile_id: "unknown".to_string(),
                ..conflicting
            }),
            Err("manual_seeding_profile_not_registered:unknown".to_string())
        );
        drop(store);
        let reopened = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(
            reopened.load_manual_seeding_record("work").unwrap(),
            Some(first.0)
        );
        assert_eq!(reopened.list_pending_operations().unwrap(), vec![first.1]);
        fs::remove_dir_all(root).unwrap();
    }
}
