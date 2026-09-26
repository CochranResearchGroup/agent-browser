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
    LaunchIssued,
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
    #[serde(default)]
    pub(crate) executable_path: String,
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
    #[serde(default)]
    pub(crate) executable_path: String,
    pub(crate) state: ManualSeedingState,
    pub(crate) route_slot_id: Option<String>,
    pub(crate) display_name: Option<String>,
    #[serde(default)]
    pub(crate) route_user: Option<String>,
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

/// Return only the public proof summary for a process-owned keeper route.
/// Provider URLs and scene details remain transient presentation evidence.
pub(crate) fn public_manual_seeding_visibility(
    binding: &RouteKeeperHandoffBinding,
    process_identity: &RecordedProcessIdentity,
    operator_visible: &serde_json::Value,
) -> Result<serde_json::Value, String> {
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
    let proof_digest = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(operator_visible)
                .map_err(|error| format!("manual_seeding_visibility_serialize_failed:{error}"))?
        )
    );
    Ok(serde_json::json!({
        "state": "ready",
        "routeId": binding.slot_id,
        "displayName": binding.display_name,
        "manualSeedingProcess": {
            "state": "ready",
            "pid": process_identity.pid,
        },
        "proofDigest": proof_digest,
    }))
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

pub(super) fn profile_reserved_by_manual_seeding(
    connection: &Connection,
    profile_id: &str,
) -> Result<bool, String> {
    let registry: ManualSeedingRegistry = load_optional_document(
        connection,
        MANUAL_SEEDING_DOCUMENT,
        MANUAL_SEEDING_SCHEMA_V1,
    )?;
    Ok(registry
        .records
        .get(profile_id)
        .is_some_and(|record| record.state != ManualSeedingState::Closed))
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
            && record.route_user.as_deref() == Some(binding.route_user.as_str())
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
        record.route_user = Some(binding.route_user.clone());
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
            request.executable_path.as_str(),
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
        if !Path::new(&request.executable_path).is_absolute() {
            return Err("manual_seeding_executable_path_invalid".to_string());
        }
        let session_state: BrowserSessionState = load_optional_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        if session_state
            .browsers
            .values()
            .any(|browser| browser.profile_id == request.profile_id)
        {
            return Err(format!(
                "manual_seeding_profile_busy:{}",
                request.profile_id
            ));
        }
        let mut pending = transaction
            .prepare(
                "SELECT request_json FROM operation_records
                 WHERE owner_key = 'browser-runtime-open' AND state IN ('prepared', 'observed')",
            )
            .map_err(|error| format!("manual_seeding_profile_operations_read_failed:{error}"))?;
        let requests = pending
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| format!("manual_seeding_profile_operations_read_failed:{error}"))?;
        for request_json in requests {
            let request_json = request_json.map_err(|error| {
                format!("manual_seeding_profile_operations_read_failed:{error}")
            })?;
            let pending_request: serde_json::Value = serde_json::from_str(&request_json)
                .map_err(|error| format!("manual_seeding_profile_operation_invalid:{error}"))?;
            if pending_request
                .pointer("/intent/browser/profileId")
                .and_then(serde_json::Value::as_str)
                == Some(request.profile_id.as_str())
            {
                return Err(format!(
                    "manual_seeding_profile_busy:{}",
                    request.profile_id
                ));
            }
        }
        drop(pending);
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
                    && existing.executable_path == request.executable_path
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
            executable_path: request.executable_path.clone(),
            state: ManualSeedingState::Reserved,
            route_slot_id: None,
            display_name: None,
            route_user: None,
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

    /// Release only an acquisition that has not launched a process. The
    /// caller may use this after validation or a launch error whose child has
    /// been reaped; an observed or uncertain process cannot take this path.
    #[allow(dead_code)]
    pub(crate) fn abort_manual_seeding_before_launch(
        &mut self,
        profile_id: &str,
        operation_id: &str,
        generation: u64,
        reason: &str,
    ) -> Result<(ManualSeedingRecord, BrowserRuntimeOperation), String> {
        if reason.trim().is_empty() {
            return Err("manual_seeding_abort_reason_missing".to_string());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("manual_seeding_abort_begin_failed:{error}"))?;
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
        let mut operation = super::load_optional_operation(&transaction, operation_id)?
            .ok_or_else(|| "manual_seeding_reservation_operation_missing".to_string())?;
        if operation.owner_key != format!("manual-seeding-profile:{profile_id}")
            || operation.generation != generation
            || super::load_owner_generation(&transaction, &operation.owner_key)? != generation
        {
            return Err("manual_seeding_abort_operation_stale".to_string());
        }
        let result = serde_json::json!({
            "status": "failed_before_launch",
            "profileId": profile_id,
            "handoffId": record.handoff_id,
            "reason": reason,
        });
        if record.state == ManualSeedingState::Closed
            && operation.state == BrowserRuntimeOperationState::Committed
            && operation.result.as_ref() == Some(&result)
        {
            return Ok((record.clone(), operation));
        }
        if record.state != ManualSeedingState::Reserved
            || record.process_identity.is_some()
            || record.uncertain_launch_pid.is_some()
            || operation.state != BrowserRuntimeOperationState::Prepared
        {
            return Err("manual_seeding_abort_after_launch_forbidden".to_string());
        }
        record.state = ManualSeedingState::Closed;
        let updated = record.clone();
        save_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
            &registry,
        )?;
        transaction
            .execute(
                "UPDATE operation_records SET state = ?2, result_json = ?3 WHERE operation_id = ?1",
                params![
                    operation_id,
                    BrowserRuntimeOperationState::Committed.as_str(),
                    serde_json::to_string(&result)
                        .map_err(|error| format!("manual_seeding_abort_result_invalid:{error}"))?
                ],
            )
            .map_err(|error| format!("manual_seeding_abort_operation_save_failed:{error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("manual_seeding_abort_commit_failed:{error}"))?;
        operation.state = BrowserRuntimeOperationState::Committed;
        operation.result = Some(result);
        Ok((updated, operation))
    }

    /// Fence a launch attempt before spawning Chrome. If the daemon stops
    /// after this commit, replay must reconcile the exact profile and route;
    /// it cannot launch a second browser on the assumption that no effect ran.
    #[allow(dead_code)]
    pub(crate) fn mark_manual_seeding_launch_issued(
        &mut self,
        profile_id: &str,
        operation_id: &str,
        generation: u64,
        binding: &RouteKeeperHandoffBinding,
    ) -> Result<(ManualSeedingRecord, BrowserRuntimeOperation), String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("manual_seeding_launch_issue_begin_failed:{error}"))?;
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
            || record.route_user.as_deref() != Some(binding.route_user.as_str())
            || record.route_fence.as_ref() != Some(&binding.fence)
        {
            return Err("manual_seeding_route_not_reserved".to_string());
        }
        if record.state != ManualSeedingState::Reserved {
            return Err("manual_seeding_launch_issue_state_invalid".to_string());
        }
        let authority = load_route_keeper_authority_document(&transaction)?;
        if authority.ready_handoff_binding(&binding.slot_id, &binding.display_name)? != *binding {
            return Err("manual_seeding_route_binding_changed".to_string());
        }
        let observation = serde_json::json!({
            "phase": "manual_seeding_launch_issued",
            "profileId": profile_id,
            "handoffId": record.handoff_id,
            "routeSlotId": binding.slot_id,
            "displayName": binding.display_name,
            "routeFence": binding.fence,
        });
        let operation =
            observe_operation_in_transaction(&transaction, operation_id, generation, observation)?;
        if operation.owner_key != format!("manual-seeding-profile:{profile_id}") {
            return Err("manual_seeding_operation_owner_mismatch".to_string());
        }
        record.state = ManualSeedingState::LaunchIssued;
        let updated = record.clone();
        save_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
            &registry,
        )?;
        transaction
            .commit()
            .map_err(|error| format!("manual_seeding_launch_issue_commit_failed:{error}"))?;
        Ok((updated, operation))
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
            || record.route_user.as_deref() != Some(binding.route_user.as_str())
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
        if record.state != ManualSeedingState::LaunchIssued {
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

    /// Release an uncertain launch only after the recorded PID is absent.
    /// A live PID cannot be adopted or killed from this evidence. The
    /// original operation becomes terminal; a later acquire needs a new ID.
    pub(crate) fn reconcile_absent_uncertain_manual_seeding_launch(
        &mut self,
        profile_id: &str,
        operation_id: &str,
        generation: u64,
        pid: u32,
    ) -> Result<(ManualSeedingRecord, BrowserRuntimeOperation), String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("manual_seeding_absent_reconcile_begin_failed:{error}"))?;
        let mut registry: ManualSeedingRegistry = load_optional_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
        )?;
        let record = registry
            .records
            .get_mut(profile_id)
            .ok_or_else(|| format!("manual_seeding_reservation_missing:{profile_id}"))?;
        if record.operation_id != operation_id
            || record.generation != generation
            || record.uncertain_launch_pid != Some(pid)
            || record.process_identity.is_some()
        {
            return Err("manual_seeding_absent_reconcile_identity_mismatch".to_string());
        }
        let mut operation = super::load_optional_operation(&transaction, operation_id)?
            .ok_or_else(|| "manual_seeding_reservation_operation_missing".to_string())?;
        if operation.owner_key != format!("manual-seeding-profile:{profile_id}")
            || operation.generation != generation
            || super::load_owner_generation(&transaction, &operation.owner_key)? != generation
        {
            return Err("manual_seeding_absent_reconcile_operation_stale".to_string());
        }
        let result = serde_json::json!({
            "status": "uncertain_launch_process_absent",
            "profileId": profile_id,
            "handoffId": record.handoff_id,
            "pid": pid,
            "retryRequiresNewOperation": true,
        });
        if record.state == ManualSeedingState::Closed
            && operation.state == BrowserRuntimeOperationState::Committed
            && operation.result.as_ref() == Some(&result)
        {
            return Ok((record.clone(), operation));
        }
        let expected_observation = serde_json::json!({
            "phase": "manual_seeding_launch_identity_uncertain",
            "profileId": profile_id,
            "handoffId": record.handoff_id,
            "routeSlotId": record.route_slot_id.as_deref()
                .ok_or_else(|| "manual_seeding_absent_reconcile_route_missing".to_string())?,
            "displayName": record.display_name.as_deref()
                .ok_or_else(|| "manual_seeding_absent_reconcile_display_missing".to_string())?,
            "routeFence": record.route_fence.as_ref()
                .ok_or_else(|| "manual_seeding_absent_reconcile_fence_missing".to_string())?,
            "pid": pid,
        });
        if record.state != ManualSeedingState::RecoveryRequired
            || operation.state != BrowserRuntimeOperationState::Observed
            || operation.result.as_ref() != Some(&expected_observation)
        {
            return Err("manual_seeding_absent_reconcile_state_invalid".to_string());
        }
        if !matches!(
            crate::process_identity::observe_process(pid),
            crate::process_identity::ProcessObservation::Missing
        ) {
            return Err("manual_seeding_uncertain_pid_not_absent".to_string());
        }
        let handoffs: BrowserManagerHandoffRegistry = load_optional_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
        )?;
        if handoffs.handoffs.contains_key(&record.handoff_id) {
            return Err("manual_seeding_absent_reconcile_handoff_conflict".to_string());
        }
        record.state = ManualSeedingState::Closed;
        let updated = record.clone();
        save_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
            &registry,
        )?;
        transaction
            .execute(
                "UPDATE operation_records SET state = ?2, result_json = ?3 WHERE operation_id = ?1",
                params![
                    operation_id,
                    BrowserRuntimeOperationState::Committed.as_str(),
                    serde_json::to_string(&result).map_err(|error| format!(
                        "manual_seeding_absent_reconcile_result_invalid:{error}"
                    ))?
                ],
            )
            .map_err(|error| {
                format!("manual_seeding_absent_reconcile_operation_save_failed:{error}")
            })?;
        transaction
            .commit()
            .map_err(|error| format!("manual_seeding_absent_reconcile_commit_failed:{error}"))?;
        operation.state = BrowserRuntimeOperationState::Committed;
        operation.result = Some(result);
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
        if record.state != ManualSeedingState::LaunchIssued
            && record.state != ManualSeedingState::LaunchObserved
        {
            return Err("manual_seeding_observation_state_invalid".to_string());
        }
        if record.route_slot_id.as_deref() != Some(binding.slot_id.as_str())
            || record.display_name.as_deref() != Some(binding.display_name.as_str())
            || record.route_user.as_deref() != Some(binding.route_user.as_str())
            || record.route_fence.as_ref() != Some(&binding.fence)
        {
            return Err("manual_seeding_route_not_reserved".to_string());
        }
        let authority = load_route_keeper_authority_document(&transaction)?;
        let route_is_current = authority
            .ready_handoff_binding(&binding.slot_id, &binding.display_name)
            .is_ok_and(|current| current == *binding);
        if record.state == ManualSeedingState::LaunchObserved
            && (record.route_slot_id.as_deref() != Some(binding.slot_id.as_str())
                || record.display_name.as_deref() != Some(binding.display_name.as_str())
                || record.route_user.as_deref() != Some(binding.route_user.as_str())
                || record.route_fence.as_ref() != Some(&binding.fence)
                || record.process_identity.as_ref() != Some(process_identity)
                || !route_is_current)
        {
            return Err("manual_seeding_route_binding_changed".to_string());
        }
        let observation = serde_json::json!({
            "phase": if route_is_current {
                "manual_seeding_launch_observed"
            } else {
                "manual_seeding_launch_route_lost"
            },
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
        record.state = if route_is_current {
            ManualSeedingState::LaunchObserved
        } else {
            ManualSeedingState::RecoveryRequired
        };
        record.route_slot_id = Some(binding.slot_id.clone());
        record.display_name = Some(binding.display_name.clone());
        record.route_user = Some(binding.route_user.clone());
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
        let public_visibility =
            public_manual_seeding_visibility(binding, process_identity, operator_visible)?;
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
            || record.route_user.as_deref() != Some(binding.route_user.as_str())
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

    /// Rebind a ready opaque handoff after keeper adoption on the same slot,
    /// display, and route user. Fresh process-owned presentation proof is
    /// required; the public handoff URL and process identity stay unchanged.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn rebind_ready_manual_seeding_handoff(
        &mut self,
        profile_id: &str,
        operation_id: &str,
        generation: u64,
        previous_fence: &RouteKeeperFence,
        binding: &RouteKeeperHandoffBinding,
        process_identity: &RecordedProcessIdentity,
        operator_visible: &serde_json::Value,
        dashboard_deployment_generation: &str,
        observed_at: &str,
    ) -> Result<(RemoteViewHandoff, BrowserRuntimeOperation), String> {
        if dashboard_deployment_generation.trim().is_empty() || observed_at.trim().is_empty() {
            return Err("manual_seeding_rebind_presentation_identity_missing".to_string());
        }
        let public_visibility =
            public_manual_seeding_visibility(binding, process_identity, operator_visible)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("manual_seeding_rebind_begin_failed:{error}"))?;
        let mut registry: ManualSeedingRegistry = load_optional_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
        )?;
        let record = registry
            .records
            .get_mut(profile_id)
            .ok_or_else(|| format!("manual_seeding_reservation_missing:{profile_id}"))?;
        if record.state != ManualSeedingState::Ready
            || record.operation_id != operation_id
            || record.generation != generation
            || record.route_slot_id.as_deref() != Some(binding.slot_id.as_str())
            || record.display_name.as_deref() != Some(binding.display_name.as_str())
            || record.route_user.as_deref() != Some(binding.route_user.as_str())
            || record.route_fence.as_ref() != Some(previous_fence)
            || previous_fence == &binding.fence
            || record.process_identity.as_ref() != Some(process_identity)
        {
            return Err("manual_seeding_rebind_identity_mismatch".to_string());
        }
        if !crate::process_identity::recorded_process_is_running(process_identity)? {
            return Err("manual_seeding_rebind_process_exited".to_string());
        }
        let authority = load_route_keeper_authority_document(&transaction)?;
        if authority.ready_handoff_binding(&binding.slot_id, &binding.display_name)? != *binding {
            return Err("manual_seeding_rebind_route_changed".to_string());
        }
        let mut operation = super::load_optional_operation(&transaction, operation_id)?
            .ok_or_else(|| "manual_seeding_rebind_operation_missing".to_string())?;
        if operation.owner_key != format!("manual-seeding-profile:{profile_id}")
            || operation.generation != generation
            || operation.state != BrowserRuntimeOperationState::Committed
            || super::load_owner_generation(&transaction, &operation.owner_key)? != generation
        {
            return Err("manual_seeding_rebind_operation_stale".to_string());
        }
        let mut handoffs: BrowserManagerHandoffRegistry = load_optional_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
        )?;
        let handoff = handoffs
            .handoffs
            .get_mut(&record.handoff_id)
            .ok_or_else(|| "manual_seeding_rebind_handoff_missing".to_string())?;
        if handoff.state != "ready"
            || handoff.profile_id.as_deref() != Some(profile_id)
            || handoff
                .intent
                .get("manualSeeding")
                .and_then(serde_json::Value::as_bool)
                != Some(true)
            || handoff
                .intent
                .get("operationId")
                .and_then(serde_json::Value::as_str)
                != Some(operation_id)
            || handoff.last_route_id.as_deref() != Some(binding.slot_id.as_str())
            || handoff.last_display_allocation_id.as_deref() != Some(binding.display_name.as_str())
            || handoff.last_resolution.as_ref() != operation.result.as_ref()
        {
            return Err("manual_seeding_rebind_handoff_mismatch".to_string());
        }
        let durable_url =
            crate::native::remote_view_handoff::durable_remote_view_handoff_url_from_public_operator_url(
                &binding.public_operator_url,
                &record.handoff_id,
            )
            .ok_or_else(|| "manual_seeding_rebind_public_url_invalid".to_string())?;
        if handoff.handoff_url.as_deref() != Some(durable_url.as_str()) {
            return Err("manual_seeding_rebind_durable_url_changed".to_string());
        }
        let process_digest = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(process_identity)
                    .map_err(|error| format!("manual_seeding_process_serialize_failed:{error}"))?
            )
        );
        let receipt = handoff
            .presentation_receipt
            .as_mut()
            .ok_or_else(|| "manual_seeding_rebind_receipt_missing".to_string())?;
        if receipt.process_instance_digest.as_deref() != Some(process_digest.as_str())
            || receipt.logical_browser_id != format!("manual-seeding:{profile_id}:{generation}")
            || handoff.browser_id.as_deref() != Some(receipt.logical_browser_id.as_str())
            || handoff.target_id.as_deref() != Some(receipt.target_id.as_str())
        {
            return Err("manual_seeding_rebind_receipt_mismatch".to_string());
        }
        receipt.generation = binding.fence.operation_generation;
        receipt.dashboard_deployment_generation = dashboard_deployment_generation.to_string();
        receipt.route_id = binding.slot_id.clone();
        receipt.display_allocation_id = binding.display_name.clone();
        receipt.observed_at = observed_at.to_string();
        let mut result = handoff
            .last_resolution
            .clone()
            .ok_or_else(|| "manual_seeding_rebind_result_missing".to_string())?;
        if result.get("status").and_then(serde_json::Value::as_str) != Some("ready")
            || result.get("handoffId").and_then(serde_json::Value::as_str)
                != Some(record.handoff_id.as_str())
        {
            return Err("manual_seeding_rebind_result_mismatch".to_string());
        }
        result["operatorVisible"] = public_visibility;
        result["presentationGeneration"] = serde_json::json!(receipt.generation);
        result["presentationReceipt"] = serde_json::to_value(&*receipt)
            .map_err(|error| format!("manual_seeding_rebind_receipt_invalid:{error}"))?;
        handoff.updated_at = Some(observed_at.to_string());
        handoff.last_resolved_at = Some(observed_at.to_string());
        handoff.last_resolution = Some(result.clone());
        let updated_handoff = handoff.clone();
        record.route_fence = Some(binding.fence.clone());
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
        transaction
            .execute(
                "UPDATE operation_records SET result_json = ?2 WHERE operation_id = ?1",
                params![
                    operation_id,
                    serde_json::to_string(&result)
                        .map_err(|error| format!("manual_seeding_rebind_result_invalid:{error}"))?
                ],
            )
            .map_err(|error| format!("manual_seeding_rebind_operation_save_failed:{error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("manual_seeding_rebind_commit_failed:{error}"))?;
        operation.result = Some(result);
        Ok((updated_handoff, operation))
    }

    /// Close only the exact detached process after its recorded identity has
    /// exited. The seeding record, durable handoff, and pending operation are
    /// terminalized together; the provider-owned keeper remains available.
    #[allow(dead_code)]
    pub(crate) fn close_manual_seeding_after_process_exit(
        &mut self,
        profile_id: &str,
        target_service_id: &str,
        handoff_id: &str,
        process_identity: &RecordedProcessIdentity,
        closed_at: &str,
    ) -> Result<(ManualSeedingRecord, Option<RemoteViewHandoff>), String> {
        if closed_at.trim().is_empty() {
            return Err("manual_seeding_close_timestamp_missing".to_string());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("manual_seeding_close_begin_failed:{error}"))?;
        let mut registry: ManualSeedingRegistry = load_optional_document(
            &transaction,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
        )?;
        let record = registry
            .records
            .get_mut(profile_id)
            .ok_or_else(|| format!("manual_seeding_reservation_missing:{profile_id}"))?;
        if record.target_service_id != target_service_id
            || record.handoff_id != handoff_id
            || record.process_identity.as_ref() != Some(process_identity)
        {
            return Err("manual_seeding_close_identity_mismatch".to_string());
        }
        let mut handoffs: BrowserManagerHandoffRegistry = load_optional_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
        )?;
        if record.state == ManualSeedingState::Closed {
            return Ok((record.clone(), handoffs.handoffs.get(handoff_id).cloned()));
        }
        if !matches!(
            record.state,
            ManualSeedingState::LaunchObserved
                | ManualSeedingState::Ready
                | ManualSeedingState::RecoveryRequired
        ) {
            return Err("manual_seeding_close_state_invalid".to_string());
        }
        let process = crate::process_identity::assess_process_ownership(
            Some(process_identity),
            crate::process_identity::observe_process(process_identity.pid),
            crate::process_identity::LegacyProfileProof::Unproven,
        );
        if !process.authorizes_cleanup() {
            return Err(format!(
                "manual_seeding_close_process_not_absent:{}",
                process.reason
            ));
        }
        let previous_state = record.state;
        record.state = ManualSeedingState::Closed;
        let updated = record.clone();
        let handoff = if let Some(handoff) = handoffs.handoffs.get_mut(handoff_id) {
            if handoff.profile_id.as_deref() != Some(profile_id)
                || handoff
                    .intent
                    .get("manualSeeding")
                    .and_then(serde_json::Value::as_bool)
                    != Some(true)
            {
                return Err("manual_seeding_close_handoff_mismatch".to_string());
            }
            handoff.state = "closed".to_string();
            handoff.updated_at = Some(closed_at.to_string());
            handoff.last_resolution = Some(serde_json::json!({
                "status": "explicitly_closed",
                "resolved": false,
                "handoffId": handoff_id,
                "profileId": profile_id,
            }));
            if let Some(receipt) = handoff.presentation_receipt.as_mut() {
                receipt.state = "closed".to_string();
            }
            Some(handoff.clone())
        } else {
            if previous_state == ManualSeedingState::Ready {
                return Err("manual_seeding_close_ready_handoff_missing".to_string());
            }
            None
        };
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
        let operation = super::load_optional_operation(&transaction, &updated.operation_id)?
            .ok_or_else(|| "manual_seeding_close_operation_missing".to_string())?;
        if operation.owner_key != format!("manual-seeding-profile:{profile_id}")
            || operation.generation != updated.generation
        {
            return Err("manual_seeding_close_operation_mismatch".to_string());
        }
        if operation.state != BrowserRuntimeOperationState::Committed {
            let result = serde_json::json!({
                "status": "closed_without_ready_handoff",
                "profileId": profile_id,
                "handoffId": handoff_id,
            });
            transaction
                .execute(
                    "UPDATE operation_records SET state = ?2, result_json = ?3 WHERE operation_id = ?1",
                    params![
                        updated.operation_id,
                        BrowserRuntimeOperationState::Committed.as_str(),
                        serde_json::to_string(&result).map_err(|error| format!(
                            "manual_seeding_close_result_invalid:{error}"
                        ))?
                    ],
                )
                .map_err(|error| format!("manual_seeding_close_operation_save_failed:{error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("manual_seeding_close_commit_failed:{error}"))?;
        Ok((updated, handoff))
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
            executable_path: "/opt/chrome".to_string(),
        };
        let pending_open = store
            .reserve_operation(
                "open-before-seeding-profile",
                "browser-runtime-open",
                serde_json::json!({"intent":{"browser":{"profileId":"work"}}}),
            )
            .unwrap();
        assert_eq!(
            store.reserve_manual_seeding(&request),
            Err("manual_seeding_profile_busy:work".to_string())
        );
        store
            .commit_operation(
                &pending_open.operation_id,
                pending_open.generation,
                serde_json::json!({"cancelledBeforeEffect":true}),
            )
            .unwrap();
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
        let mut reopened = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(
            reopened.load_manual_seeding_record("work").unwrap(),
            Some(first.0.clone())
        );
        assert_eq!(reopened.list_pending_operations().unwrap(), vec![first.1]);
        let aborted = reopened
            .abort_manual_seeding_before_launch(
                "work",
                "seed-operation-a",
                first.0.generation,
                "launch_validation_failed",
            )
            .unwrap();
        assert_eq!(aborted.0.state, ManualSeedingState::Closed);
        assert_eq!(
            aborted.1.state,
            super::super::BrowserRuntimeOperationState::Committed
        );
        assert_eq!(
            reopened
                .abort_manual_seeding_before_launch(
                    "work",
                    "seed-operation-a",
                    first.0.generation,
                    "launch_validation_failed",
                )
                .unwrap(),
            aborted
        );
        assert!(reopened.list_pending_operations().unwrap().is_empty());
        assert!(reopened
            .load_handoff_registry()
            .unwrap()
            .handoffs
            .is_empty());
        let next = reopened
            .reserve_manual_seeding(&ManualSeedingReservation {
                operation_id: "seed-operation-c".to_string(),
                profile_id: "work".to_string(),
                target_service_id: "service-b".to_string(),
                handoff_id: "seed-handoff-c".to_string(),
                requested_url: None,
                executable_path: "/opt/chrome".to_string(),
            })
            .unwrap();
        assert_eq!(next.0.generation, first.0.generation + 1);
        let mut registry: ManualSeedingRegistry = load_optional_document(
            &reopened.connection,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
        )
        .unwrap();
        let current_process =
            crate::process_identity::capture_process_identity(std::process::id(), None, None)
                .unwrap();
        let record = registry.records.get_mut("work").unwrap();
        record.state = ManualSeedingState::LaunchObserved;
        record.process_identity = Some(current_process.clone());
        save_document(
            &reopened.connection,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
            &registry,
        )
        .unwrap();
        assert!(matches!(
            reopened.close_manual_seeding_after_process_exit(
                "work",
                "service-b",
                "seed-handoff-c",
                &current_process,
                "2026-09-25T00:00:00Z",
            ),
            Err(error) if error.starts_with("manual_seeding_close_process_not_absent:")
        ));
        assert_eq!(
            reopened
                .load_manual_seeding_record("work")
                .unwrap()
                .unwrap()
                .state,
            ManualSeedingState::LaunchObserved
        );
        let mut absent_process = current_process;
        absent_process.pid = u32::MAX;
        registry.records.get_mut("work").unwrap().process_identity = Some(absent_process.clone());
        save_document(
            &reopened.connection,
            MANUAL_SEEDING_DOCUMENT,
            MANUAL_SEEDING_SCHEMA_V1,
            &registry,
        )
        .unwrap();
        let closed = reopened
            .close_manual_seeding_after_process_exit(
                "work",
                "service-b",
                "seed-handoff-c",
                &absent_process,
                "2026-09-25T00:00:00Z",
            )
            .unwrap();
        assert_eq!(closed.0.state, ManualSeedingState::Closed);
        assert!(closed.1.is_none());
        assert_eq!(
            reopened
                .find_operation("seed-operation-c")
                .unwrap()
                .unwrap()
                .state,
            super::super::BrowserRuntimeOperationState::Committed
        );
        assert_eq!(
            reopened
                .close_manual_seeding_after_process_exit(
                    "work",
                    "service-b",
                    "seed-handoff-c",
                    &absent_process,
                    "2026-09-25T00:00:00Z",
                )
                .unwrap(),
            closed
        );
        fs::remove_dir_all(root).unwrap();
    }
}
