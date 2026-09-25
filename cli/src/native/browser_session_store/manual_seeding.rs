//! SQLite custody for a CDP-free manual-seeding acquisition.

use std::collections::BTreeMap;
use std::path::Path;

use agent_browser_service_model::{
    BrowserProfileKind, ControlInputProvider, DurableHandoffPresentationReceipt,
    RecordedProcessIdentity, RemoteViewHandoff, RouteKeeperFence, RouteKeeperHandoffBinding,
    ViewStreamProvider,
};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    load_optional_document, load_route_keeper_authority_document, observe_operation_in_transaction,
    reserve_operation_in_transaction, save_document, BrowserManagerHandoffRegistry,
    BrowserProfileCatalog, BrowserRuntimeOperation, BrowserRuntimeOperationState,
    BrowserRuntimeSqliteStore, BrowserSessionState, BROWSER_PROFILE_CATALOG_SCHEMA_V1,
    BROWSER_SESSION_STATE_SCHEMA_V1, MANAGER_HANDOFF_REGISTRY_DOCUMENT,
    MANAGER_HANDOFF_REGISTRY_SCHEMA_V1, PROFILE_CATALOG_DOCUMENT, SESSION_STATE_DOCUMENT,
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
    #[serde(default)]
    pub(crate) uncertain_launch_pid: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManualSeedingRegistry {
    records: BTreeMap<String, ManualSeedingRecord>,
}

pub(super) fn route_slot_reserved_by_manual_seeding(
    connection: &Connection,
    slot_id: &str,
) -> Result<bool, String> {
    let registry: ManualSeedingRegistry = load_optional_document(
        connection,
        MANUAL_SEEDING_DOCUMENT,
        MANUAL_SEEDING_SCHEMA_V1,
    )?;
    Ok(registry.records.values().any(|record| {
        record.state != ManualSeedingState::Closed
            && record.route_slot_id.as_deref() == Some(slot_id)
    }))
}

impl BrowserRuntimeSqliteStore {
    /// Bind a currently ready keeper slot before launching a detached browser.
    /// The same SQLite transaction rejects live browser membership, waiting
    /// browser-open operations, and any other manual-seeding reservation for
    /// that slot. No provider or browser effect occurs here.
    #[allow(dead_code)]
    pub(crate) fn bind_manual_seeding_route(
        &mut self,
        profile_id: &str,
        operation_id: &str,
        generation: u64,
        binding: &RouteKeeperHandoffBinding,
    ) -> Result<ManualSeedingRecord, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("manual_seeding_route_begin_failed:{error}"))?;
        let mut registry: ManualSeedingRegistry = load_optional_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
        )?;
        let record = registry
            .records
            .get(profile_id)
            .ok_or_else(|| format!("manual_seeding_reservation_missing:{profile_id}"))?;
        if record.operation_id != operation_id || record.generation != generation {
            return Err("manual_seeding_operation_identity_mismatch".to_string());
        }
        if record.state != ManualSeedingState::Reserved {
            return Err("manual_seeding_route_state_invalid".to_string());
        }
        let operation = super::load_optional_operation(&transaction, operation_id)?
            .ok_or_else(|| "manual_seeding_reservation_operation_missing".to_string())?;
        if operation.owner_key != format!("manual-seeding-profile:{profile_id}")
            || operation.generation != generation
            || operation.state != BrowserRuntimeOperationState::Prepared
            || super::load_owner_generation(&transaction, &operation.owner_key)? != generation
        {
            return Err("manual_seeding_route_operation_stale".to_string());
        }
        let authority = load_route_keeper_authority_document(&transaction)?;
        if authority.ready_handoff_binding(&binding.slot_id, &binding.display_name)? != *binding {
            return Err("manual_seeding_route_binding_changed".to_string());
        }
        let exact_replay = record.route_slot_id.as_deref() == Some(binding.slot_id.as_str())
            && record.display_name.as_deref() == Some(binding.display_name.as_str())
            && record.route_fence.as_ref() == Some(&binding.fence);
        if record.route_slot_id.is_some() && !exact_replay {
            return Err("manual_seeding_route_rebinding_forbidden".to_string());
        }
        if registry.records.iter().any(|(other_profile, other)| {
            other_profile != profile_id
                && other.state != ManualSeedingState::Closed
                && other.route_slot_id.as_deref() == Some(binding.slot_id.as_str())
        }) {
            return Err("manual_seeding_route_busy".to_string());
        }
        let session_state: BrowserSessionState = load_optional_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        if session_state.browsers.values().any(|browser| {
            browser
                .desktop
                .as_ref()
                .is_some_and(|desktop| desktop.route_id == binding.slot_id)
        }) {
            return Err("manual_seeding_route_busy".to_string());
        }
        let mut statement = transaction
            .prepare(
                "SELECT request_json FROM operation_records
                 WHERE owner_key = 'browser-runtime-open' AND state IN ('prepared', 'observed')",
            )
            .map_err(|error| format!("manual_seeding_route_operations_read_failed:{error}"))?;
        let requests = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| format!("manual_seeding_route_operations_read_failed:{error}"))?;
        for request in requests {
            let request = request
                .map_err(|error| format!("manual_seeding_route_operations_read_failed:{error}"))?;
            let request: serde_json::Value = serde_json::from_str(&request)
                .map_err(|error| format!("manual_seeding_route_operation_invalid:{error}"))?;
            if request
                .pointer("/intent/slot/routeId")
                .and_then(serde_json::Value::as_str)
                == Some(binding.slot_id.as_str())
            {
                return Err("manual_seeding_route_busy".to_string());
            }
        }
        drop(statement);
        if exact_replay {
            return Ok(record.clone());
        }
        let record = registry
            .records
            .get_mut(profile_id)
            .ok_or_else(|| format!("manual_seeding_reservation_missing:{profile_id}"))?;
        record.route_slot_id = Some(binding.slot_id.clone());
        record.display_name = Some(binding.display_name.clone());
        record.route_fence = Some(binding.fence.clone());
        let updated = record.clone();
        save_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
            &registry,
        )?;
        transaction
            .commit()
            .map_err(|error| format!("manual_seeding_route_commit_failed:{error}"))?;
        Ok(updated)
    }
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
            uncertain_launch_pid: None,
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

    /// Retain an uncertain detached launch before any retry. A PID alone is
    /// insufficient authority to terminate or adopt a process, so this state
    /// blocks ready publication and profile reuse pending exact reconciliation.
    #[allow(dead_code)]
    pub(crate) fn observe_uncertain_manual_seeding_launch(
        &mut self,
        profile_id: &str,
        operation_id: &str,
        generation: u64,
        binding: &RouteKeeperHandoffBinding,
        pid: u32,
    ) -> Result<(ManualSeedingRecord, BrowserRuntimeOperation), String> {
        if pid == 0 {
            return Err("manual_seeding_uncertain_pid_invalid".to_string());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("manual_seeding_uncertain_begin_failed:{error}"))?;
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
        if record.route_slot_id.as_deref() != Some(binding.slot_id.as_str())
            || record.display_name.as_deref() != Some(binding.display_name.as_str())
            || record.route_fence.as_ref() != Some(&binding.fence)
        {
            return Err("manual_seeding_route_not_reserved".to_string());
        }
        if record.state == ManualSeedingState::RecoveryRequired {
            if record.uncertain_launch_pid != Some(pid) {
                return Err("manual_seeding_uncertain_launch_changed".to_string());
            }
            let operation = super::load_optional_operation(&transaction, operation_id)?
                .ok_or_else(|| "manual_seeding_reservation_operation_missing".to_string())?;
            return Ok((record.clone(), operation));
        }
        if record.state != ManualSeedingState::Reserved {
            return Err("manual_seeding_uncertain_state_invalid".to_string());
        }
        let observation = serde_json::json!({
            "phase": "manual_seeding_launch_identity_uncertain",
            "profileId": profile_id,
            "handoffId": record.handoff_id,
            "routeSlotId": binding.slot_id,
            "displayName": binding.display_name,
            "routeFence": binding.fence,
            "pid": pid,
        });
        let operation =
            observe_operation_in_transaction(&transaction, operation_id, generation, observation)?;
        if operation.owner_key != format!("manual-seeding-profile:{profile_id}") {
            return Err("manual_seeding_operation_owner_mismatch".to_string());
        }
        record.state = ManualSeedingState::RecoveryRequired;
        record.uncertain_launch_pid = Some(pid);
        let updated = record.clone();
        save_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
            &registry,
        )?;
        transaction
            .commit()
            .map_err(|error| format!("manual_seeding_uncertain_commit_failed:{error}"))?;
        Ok((updated, operation))
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
        if record.route_slot_id.as_deref() != Some(binding.slot_id.as_str())
            || record.display_name.as_deref() != Some(binding.display_name.as_str())
            || record.route_fence.as_ref() != Some(&binding.fence)
        {
            return Err("manual_seeding_route_not_reserved".to_string());
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

    /// Publish the opaque ready handoff, seeding lifecycle, and operation
    /// result together after the caller has proved a current process-owned
    /// window and authenticated public operator route. No provider URL enters
    /// the operator response.
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn publish_manual_seeding_ready(
        &mut self,
        profile_id: &str,
        operation_id: &str,
        generation: u64,
        binding: &RouteKeeperHandoffBinding,
        process_identity: &RecordedProcessIdentity,
        operator_visible: &serde_json::Value,
        dashboard_deployment_generation: &str,
        observed_at: &str,
    ) -> Result<(RemoteViewHandoff, BrowserRuntimeOperation), String> {
        if dashboard_deployment_generation.trim().is_empty() || observed_at.trim().is_empty() {
            return Err("manual_seeding_presentation_identity_missing".to_string());
        }
        if operator_visible
            .get("state")
            .and_then(serde_json::Value::as_str)
            != Some("ready")
            || operator_visible
                .get("routeId")
                .and_then(serde_json::Value::as_str)
                != Some(binding.slot_id.as_str())
            || operator_visible
                .get("displayName")
                .and_then(serde_json::Value::as_str)
                != Some(binding.display_name.as_str())
            || operator_visible
                .pointer("/manualSeedingProcess/state")
                .and_then(serde_json::Value::as_str)
                != Some("ready")
            || operator_visible
                .pointer("/manualSeedingProcess/pid")
                .and_then(serde_json::Value::as_u64)
                != Some(u64::from(process_identity.pid))
        {
            return Err("manual_seeding_operator_visibility_unproven".to_string());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("manual_seeding_publication_begin_failed:{error}"))?;
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
        if record.route_slot_id.as_deref() != Some(binding.slot_id.as_str())
            || record.display_name.as_deref() != Some(binding.display_name.as_str())
            || record.route_fence.as_ref() != Some(&binding.fence)
            || record.process_identity.as_ref() != Some(process_identity)
        {
            return Err("manual_seeding_publication_observation_changed".to_string());
        }
        let authority = load_route_keeper_authority_document(&transaction)?;
        if authority.ready_handoff_binding(&binding.slot_id, &binding.display_name)? != *binding {
            return Err("manual_seeding_route_binding_changed".to_string());
        }
        let mut operation = super::load_optional_operation(&transaction, operation_id)?
            .ok_or_else(|| "manual_seeding_reservation_operation_missing".to_string())?;
        if operation.owner_key != format!("manual-seeding-profile:{profile_id}")
            || operation.generation != generation
            || super::load_owner_generation(&transaction, &operation.owner_key)? != generation
        {
            return Err("manual_seeding_publication_operation_stale".to_string());
        }
        let mut handoffs: BrowserManagerHandoffRegistry = load_optional_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
        )?;
        if record.state == ManualSeedingState::Ready
            && operation.state == BrowserRuntimeOperationState::Committed
        {
            let handoff = handoffs
                .handoffs
                .get(&record.handoff_id)
                .filter(|handoff| handoff.state == "ready")
                .cloned()
                .ok_or_else(|| "manual_seeding_ready_handoff_missing".to_string())?;
            return Ok((handoff, operation));
        }
        if record.state != ManualSeedingState::LaunchObserved
            || operation.state != BrowserRuntimeOperationState::Observed
        {
            return Err("manual_seeding_publication_state_invalid".to_string());
        }
        let expected_observation = serde_json::json!({
            "phase": "manual_seeding_launch_observed",
            "profileId": profile_id,
            "handoffId": record.handoff_id,
            "routeSlotId": binding.slot_id,
            "displayName": binding.display_name,
            "routeFence": binding.fence,
            "processIdentity": process_identity,
        });
        if operation.result.as_ref() != Some(&expected_observation) {
            return Err("manual_seeding_publication_observation_mismatch".to_string());
        }
        if handoffs.handoffs.contains_key(&record.handoff_id) {
            return Err("manual_seeding_handoff_identity_conflict".to_string());
        }
        let handoff_url = crate::native::remote_view_handoff::durable_remote_view_handoff_url_from_public_operator_url(
            &binding.public_operator_url,
            &record.handoff_id,
        )
        .ok_or_else(|| "manual_seeding_public_operator_url_invalid".to_string())?;
        let browser_id = format!("manual-seeding:{profile_id}:{generation}");
        let target_id = format!("manual-seeding-window:{}", process_identity.pid);
        let process_digest = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(process_identity)
                    .map_err(|error| format!("manual_seeding_process_serialize_failed:{error}"))?
            )
        );
        let visibility_digest = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(operator_visible).map_err(|error| format!(
                    "manual_seeding_visibility_serialize_failed:{error}"
                ))?
            )
        );
        let public_visibility = serde_json::json!({
            "state": "ready",
            "routeId": binding.slot_id,
            "displayName": binding.display_name,
            "manualSeedingProcess": {
                "state": "ready",
                "pid": process_identity.pid,
            },
            "proofDigest": visibility_digest,
        });
        let receipt = DurableHandoffPresentationReceipt {
            schema_version: "agent-browser.durable-handoff-presentation.v1".to_string(),
            generation: binding.fence.operation_generation,
            dashboard_deployment_generation: dashboard_deployment_generation.to_string(),
            logical_browser_id: browser_id.clone(),
            daemon_owner_generation: None,
            process_instance_digest: Some(process_digest),
            target_id: target_id.clone(),
            required_stream_provider: ViewStreamProvider::RdpGateway,
            observed_stream_provider: ViewStreamProvider::RdpGateway,
            route_id: binding.slot_id.clone(),
            display_allocation_id: binding.display_name.clone(),
            observed_at: observed_at.to_string(),
            state: "ready".to_string(),
        };
        let result = serde_json::json!({
            "status": "ready",
            "resolved": true,
            "manualSeeding": true,
            "profileId": profile_id,
            "targetServiceId": record.target_service_id,
            "handoffId": record.handoff_id,
            "handoffUrl": handoff_url,
            "browserId": browser_id,
            "targetId": target_id,
            "operatorVisible": public_visibility,
            "viewStreamProvider": ViewStreamProvider::RdpGateway,
            "controlInput": ControlInputProvider::ManualAttachedDesktop,
            "presentationGeneration": receipt.generation,
            "presentationReceipt": receipt,
            "authentication": {
                "state": "not_probed",
                "reason": "visibility_is_not_authentication_evidence",
            },
        });
        let handoff = RemoteViewHandoff {
            id: record.handoff_id.clone(),
            state: "ready".to_string(),
            intent: serde_json::json!({
                "manualSeeding": true,
                "profileId": profile_id,
                "targetServiceId": record.target_service_id,
                "operationId": operation_id,
                "presentationSlotId": binding.slot_id,
                "displayName": binding.display_name,
            }),
            handoff_url: Some(handoff_url),
            desired_url: record.requested_url.clone(),
            profile_id: Some(profile_id.to_string()),
            browser_id: Some(browser_id),
            session_name: None,
            tab_id: None,
            target_id: Some(target_id),
            view_stream_provider: Some(ViewStreamProvider::RdpGateway),
            control_input: Some(ControlInputProvider::ManualAttachedDesktop),
            last_route_id: Some(binding.slot_id.clone()),
            last_route_pool_entry_id: None,
            last_display_allocation_id: Some(binding.display_name.clone()),
            created_at: Some(observed_at.to_string()),
            updated_at: Some(observed_at.to_string()),
            last_resolved_at: Some(observed_at.to_string()),
            last_resolution: Some(result.clone()),
            presentation_receipt: Some(receipt),
        };
        handoffs
            .handoffs
            .insert(handoff.id.clone(), handoff.clone());
        record.state = ManualSeedingState::Ready;
        save_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
            &registry,
        )?;
        save_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
            &handoffs,
        )?;
        let result_json = serde_json::to_string(&result)
            .map_err(|error| format!("manual_seeding_result_serialize_failed:{error}"))?;
        transaction
            .execute(
                "UPDATE operation_records SET state = ?2, result_json = ?3 WHERE operation_id = ?1",
                params![
                    operation_id,
                    BrowserRuntimeOperationState::Committed.as_str(),
                    result_json
                ],
            )
            .map_err(|error| format!("manual_seeding_operation_save_failed:{error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("manual_seeding_publication_commit_failed:{error}"))?;
        operation.state = BrowserRuntimeOperationState::Committed;
        operation.result = Some(result);
        Ok((handoff, operation))
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
    fn sqlite_manual_seeding_plan_uses_exact_catalog_profile_without_json_selection() {
        let profile = BrowserProfileCatalogEntry {
            id: "work".to_string(),
            name: "Work".to_string(),
            user_data_dir: std::env::temp_dir()
                .join("sqlite-manual-seeding-profile")
                .to_string_lossy()
                .into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let command = serde_json::json!({
            "action": "cdp_free_launch",
            "runtimeProfile": "work",
            "profileId": "work",
            "executablePath": "/opt/chrome",
            "url": "https://example.test/login",
        });
        let build =
            crate::native::action_runtime::runtime::build_cdp_free_launch_plan_for_sqlite_profile;
        let plan = build(&command, &profile).unwrap();
        assert_eq!(
            plan.launch_options.profile.as_deref(),
            Some(profile.user_data_dir.as_str())
        );
        assert_eq!(plan.launch_options.runtime_profile.as_deref(), Some("work"));
        assert!(!plan.launch_options.attachable);
        assert_eq!(plan.url.as_deref(), Some("https://example.test/login"));
        assert_eq!(
            plan.metadata
                .browser_capability_launch
                .as_ref()
                .and_then(|value| value.get("reason")),
            Some(&serde_json::json!("sqlite_reserved_executable_path"))
        );

        let mut mismatched = command.clone();
        mismatched["runtimeProfile"] = serde_json::json!("other");
        assert!(matches!(
            build(&mismatched, &profile),
            Err(error) if error == "cdp_free_sqlite_profile_selector_mismatch:runtimeProfile"
        ));
        let mut path_override = command.clone();
        path_override["profile"] = serde_json::json!("/tmp/other");
        assert!(matches!(
            build(&path_override, &profile),
            Err(error) if error == "cdp_free_sqlite_profile_path_override_forbidden"
        ));
        let mut no_executable = command;
        no_executable
            .as_object_mut()
            .unwrap()
            .remove("executablePath");
        assert!(matches!(
            build(&no_executable, &profile),
            Err(error) if error == "cdp_free_sqlite_executable_path_required"
        ));
    }

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
