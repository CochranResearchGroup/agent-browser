//! Durable controller ownership for retained Browser Session Manager handoffs.

use agent_browser_service_model::{
    BrowserSessionState, ControlInputProvider, RemoteViewHandoff, RouteKeeperHandoffBinding,
    ViewStreamProvider,
};
use rusqlite::{Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

use super::*;

const DOCUMENT: &str = "desktop_control";
const SCHEMA: &str = "agent-browser.desktop-control.v1";
const LIVE_VIEWER_TTL_MS: u64 = 15_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopControlTransferRequest {
    pub(crate) operation_id: String,
    pub(crate) handoff_id: String,
    pub(crate) client_connection_id: String,
    pub(crate) expected_binding: RouteKeeperHandoffBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopControlLease {
    pub(crate) operation_id: String,
    pub(crate) epoch: u64,
    pub(crate) handoff_id: String,
    pub(crate) browser_id: String,
    pub(crate) session_id: String,
    pub(crate) tab_id: String,
    pub(crate) target_id: String,
    pub(crate) controller_client_connection_id: String,
    pub(crate) host_generation: u64,
    pub(crate) route_binding: RouteKeeperHandoffBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiveViewerControlAuthority {
    pub(crate) lease_id: String,
    pub(crate) authenticated_principal: String,
    pub(crate) provider_route_id: String,
    pub(crate) updated_at_ms: u64,
    pub(crate) expires_at_ms: u64,
    pub(crate) lease: DesktopControlLease,
}

/// One exact logical browser selected from current authenticated viewer
/// authority. The scheduler may observe this projection, but replacement still
/// requires a fresh browser liveness proof and the shared recovery fence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveViewerBrowserRecoveryCandidate {
    pub(crate) viewer_lease_id: String,
    pub(crate) viewer_updated_at_ms: u64,
    pub(crate) browser_id: String,
    pub(crate) profile_id: String,
    pub(crate) session_id: String,
    pub(crate) session_name: String,
    pub(crate) handoff_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiveViewerActivationRequest {
    pub(crate) operation_id: String,
    pub(crate) handoff_id: String,
    pub(crate) client_connection_id: String,
    pub(crate) authenticated_principal: String,
    pub(crate) provider_route_id: String,
    pub(crate) guacamole_connection_id: u64,
    pub(crate) guacamole_primary_active_connection_id: String,
    pub(crate) observed_shared_connection_count: u64,
    pub(crate) observed_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiveViewerHeartbeatRequest {
    pub(crate) lease_id: String,
    pub(crate) authenticated_principal: String,
    pub(crate) provider_route_id: String,
    pub(crate) guacamole_connection_id: u64,
    pub(crate) guacamole_primary_active_connection_id: String,
    pub(crate) observed_shared_connection_count: u64,
    pub(crate) observed_at_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DesktopControlState {
    next_epoch: u64,
    current_operation_ids: BTreeMap<String, String>,
    operations: BTreeMap<String, DesktopControlRecord>,
    #[serde(default)]
    live_viewers: BTreeMap<String, LiveViewerRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LiveViewerRecord {
    lease_id: String,
    operation_id: String,
    handoff_id: String,
    client_connection_id: String,
    authenticated_principal: String,
    provider_route_id: String,
    guacamole_connection_id: u64,
    guacamole_primary_active_connection_id: String,
    slot_id: String,
    host_generation: u64,
    boot_epoch: String,
    controller_epoch: u64,
    observed_shared_connection_count: u64,
    state: String,
    created_at_ms: u64,
    updated_at_ms: u64,
    expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DesktopControlRecord {
    operation_id: String,
    epoch: u64,
    handoff_id: String,
    browser_id: String,
    session_id: String,
    tab_id: String,
    target_id: String,
    controller_client_connection_id: String,
    host_generation: u64,
    route_binding: DesktopControlRouteBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DesktopControlRouteBinding {
    slot_id: String,
    keeper_id: String,
    fence: agent_browser_service_model::RouteKeeperFence,
    route_user: String,
    display_name: String,
    guacamole_connection_id: u64,
    guacamole_connection_uuid: String,
}

struct ResolvedDesktopControl {
    browser_id: String,
    session_id: String,
    tab_id: String,
    target_id: String,
    host_generation: u64,
    route_binding: RouteKeeperHandoffBinding,
}

impl DesktopControlRouteBinding {
    fn from_binding(binding: &RouteKeeperHandoffBinding) -> Self {
        Self {
            slot_id: binding.slot_id.clone(),
            keeper_id: binding.keeper_id.clone(),
            fence: binding.fence.clone(),
            route_user: binding.route_user.clone(),
            display_name: binding.display_name.clone(),
            guacamole_connection_id: binding.guacamole_connection_id,
            guacamole_connection_uuid: binding.guacamole_connection_uuid.clone(),
        }
    }
}

impl DesktopControlRecord {
    fn lease(&self, route_binding: RouteKeeperHandoffBinding) -> DesktopControlLease {
        DesktopControlLease {
            operation_id: self.operation_id.clone(),
            epoch: self.epoch,
            handoff_id: self.handoff_id.clone(),
            browser_id: self.browser_id.clone(),
            session_id: self.session_id.clone(),
            tab_id: self.tab_id.clone(),
            target_id: self.target_id.clone(),
            controller_client_connection_id: self.controller_client_connection_id.clone(),
            host_generation: self.host_generation,
            route_binding,
        }
    }
}

impl BrowserRuntimeSqliteStore {
    pub(crate) fn active_viewer_browser_recovery_candidates(
        &self,
        now_ms: u64,
    ) -> Result<Vec<ActiveViewerBrowserRecoveryCandidate>, String> {
        let boot_epoch = crate::process_identity::current_boot_epoch()
            .ok_or_else(|| "live_viewer_boot_epoch_unavailable".to_string())?;
        self.active_viewer_browser_recovery_candidates_for_boot(now_ms, &boot_epoch)
    }

    fn active_viewer_browser_recovery_candidates_for_boot(
        &self,
        now_ms: u64,
        boot_epoch: &str,
    ) -> Result<Vec<ActiveViewerBrowserRecoveryCandidate>, String> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| format!("desktop_control_read_begin_failed:{error}"))?;
        let state: DesktopControlState = load_optional_document(&transaction, DOCUMENT, SCHEMA)?;
        let sessions: BrowserSessionState = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        let mut candidates = BTreeMap::new();
        for record in state.live_viewers.values() {
            if record.state != "controlling"
                || record.boot_epoch != boot_epoch
                || record.expires_at_ms <= now_ms
                || record.observed_shared_connection_count == 0
            {
                continue;
            }
            let Ok(lease) = load_and_validate_current(
                &transaction,
                &record.handoff_id,
                &record.client_connection_id,
                record.controller_epoch,
            ) else {
                continue;
            };
            if record.operation_id != lease.operation_id
                || record.slot_id != lease.route_binding.slot_id
                || record.host_generation != lease.host_generation
            {
                continue;
            }
            let Some(session) = sessions.sessions.get(&lease.session_id) else {
                continue;
            };
            let Some(browser) = sessions.browsers.get(&lease.browser_id) else {
                continue;
            };
            if session.browser_id != browser.id || session.profile_id != browser.profile_id {
                continue;
            }
            candidates.entry(browser.id.clone()).or_insert_with(|| {
                ActiveViewerBrowserRecoveryCandidate {
                    viewer_lease_id: record.lease_id.clone(),
                    viewer_updated_at_ms: record.updated_at_ms,
                    browser_id: browser.id.clone(),
                    profile_id: browser.profile_id.clone(),
                    session_id: session.id.clone(),
                    session_name: session.name.clone(),
                    handoff_id: lease.handoff_id.clone(),
                }
            });
        }
        transaction
            .commit()
            .map_err(|error| format!("desktop_control_read_commit_failed:{error}"))?;
        Ok(candidates.into_values().collect())
    }

    pub(crate) fn current_live_viewer_control(
        &self,
        lease_id: &str,
    ) -> Result<LiveViewerControlAuthority, String> {
        validate_identifier(lease_id, "live_viewer_lease")?;
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| format!("desktop_control_read_begin_failed:{error}"))?;
        let authority = load_live_viewer_control(&transaction, lease_id, current_time_ms()?)?;
        transaction
            .commit()
            .map_err(|error| format!("desktop_control_read_commit_failed:{error}"))?;
        Ok(authority)
    }

    pub(crate) fn with_current_live_viewer_control<T>(
        &mut self,
        lease_id: &str,
        expected_state: &BrowserSessionState,
        effect: impl FnOnce(&LiveViewerControlAuthority) -> Result<T, String>,
    ) -> Result<T, String> {
        validate_identifier(lease_id, "live_viewer_lease")?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("desktop_control_effect_begin_failed:{error}"))?;
        let persisted_state: BrowserSessionState = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        if &persisted_state != expected_state {
            return Err("desktop_control_session_state_stale".to_string());
        }
        let authority = load_live_viewer_control(&transaction, lease_id, current_time_ms()?)?;
        let result = effect(&authority)?;
        transaction
            .commit()
            .map_err(|error| format!("desktop_control_effect_commit_failed:{error}"))?;
        Ok(result)
    }

    /// Transfer controller ownership to one exact client and retained handoff.
    ///
    /// The operation is idempotent only while it remains current and its
    /// complete request payload is unchanged. A superseded operation can never
    /// reclaim ownership by replaying its old identifier.
    pub(crate) fn transfer_desktop_control(
        &mut self,
        request: &DesktopControlTransferRequest,
    ) -> Result<DesktopControlLease, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("desktop_control_begin_failed:{error}"))?;
        let mut state: DesktopControlState =
            load_optional_document(&transaction, DOCUMENT, SCHEMA)?;
        let lease = stage_desktop_control_transfer(&transaction, &mut state, request)?;
        save_document(&transaction, DOCUMENT, SCHEMA, &state)?;
        transaction
            .commit()
            .map_err(|error| format!("desktop_control_commit_failed:{error}"))?;
        Ok(lease)
    }

    /// Grant control only after the caller has independently observed at
    /// least one live restricted Guacamole sharing tunnel for this keeper.
    pub(crate) fn activate_live_viewer(
        &mut self,
        request: &LiveViewerActivationRequest,
    ) -> Result<DesktopControlLease, String> {
        validate_live_viewer_request(request)?;
        let boot_epoch = crate::process_identity::current_boot_epoch()
            .ok_or_else(|| "live_viewer_boot_epoch_unavailable".to_string())?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("live_viewer_begin_failed:{error}"))?;
        let resolved = resolve_ready_manager_handoff(&transaction, &request.handoff_id)?;
        if resolved.route_binding.guacamole_connection_id != request.guacamole_connection_id {
            return Err("live_viewer_connection_binding_mismatch".to_string());
        }
        let mut state: DesktopControlState =
            load_optional_document(&transaction, DOCUMENT, SCHEMA)?;
        expire_live_viewers(&mut state, request.observed_at_ms, &boot_epoch);
        let lease = stage_desktop_control_transfer(
            &transaction,
            &mut state,
            &DesktopControlTransferRequest {
                operation_id: request.operation_id.clone(),
                handoff_id: request.handoff_id.clone(),
                client_connection_id: request.client_connection_id.clone(),
                expected_binding: resolved.route_binding.clone(),
            },
        )?;
        let lease_id = format!("live-viewer:{}", request.operation_id);
        let expires_at_ms = request
            .observed_at_ms
            .checked_add(LIVE_VIEWER_TTL_MS)
            .ok_or_else(|| "live_viewer_expiry_overflow".to_string())?;
        if let Some(existing) = state.live_viewers.get(&lease_id) {
            if existing.operation_id != request.operation_id
                || existing.handoff_id != request.handoff_id
                || existing.client_connection_id != request.client_connection_id
                || existing.authenticated_principal != request.authenticated_principal
                || existing.provider_route_id != request.provider_route_id
                || existing.guacamole_connection_id != request.guacamole_connection_id
                || existing.guacamole_primary_active_connection_id
                    != request.guacamole_primary_active_connection_id
            {
                return Err("live_viewer_operation_conflict".to_string());
            }
        }
        let created_at_ms = state
            .live_viewers
            .get(&lease_id)
            .map(|record| record.created_at_ms)
            .unwrap_or(request.observed_at_ms);
        state.live_viewers.insert(
            lease_id,
            LiveViewerRecord {
                lease_id: format!("live-viewer:{}", request.operation_id),
                operation_id: request.operation_id.clone(),
                handoff_id: request.handoff_id.clone(),
                client_connection_id: request.client_connection_id.clone(),
                authenticated_principal: request.authenticated_principal.clone(),
                provider_route_id: request.provider_route_id.clone(),
                guacamole_connection_id: request.guacamole_connection_id,
                guacamole_primary_active_connection_id: request
                    .guacamole_primary_active_connection_id
                    .clone(),
                slot_id: lease.route_binding.slot_id.clone(),
                host_generation: lease.host_generation,
                boot_epoch,
                controller_epoch: lease.epoch,
                observed_shared_connection_count: request.observed_shared_connection_count,
                state: "controlling".to_string(),
                created_at_ms,
                updated_at_ms: request.observed_at_ms,
                expires_at_ms,
            },
        );
        save_document(&transaction, DOCUMENT, SCHEMA, &state)?;
        transaction
            .commit()
            .map_err(|error| format!("live_viewer_commit_failed:{error}"))?;
        Ok(lease)
    }

    pub(crate) fn heartbeat_live_viewer(
        &mut self,
        request: &LiveViewerHeartbeatRequest,
    ) -> Result<DesktopControlLease, String> {
        validate_identifier(&request.lease_id, "live_viewer_lease")?;
        validate_identifier(&request.authenticated_principal, "authenticated_principal")?;
        validate_identifier(&request.provider_route_id, "provider_route")?;
        validate_identifier(
            &request.guacamole_primary_active_connection_id,
            "guacamole_primary_active_connection",
        )?;
        if request.guacamole_connection_id == 0 {
            return Err("live_viewer_connection_binding_invalid".to_string());
        }
        if request.observed_shared_connection_count == 0 {
            return Err("live_viewer_tunnel_absent".to_string());
        }
        let boot_epoch = crate::process_identity::current_boot_epoch()
            .ok_or_else(|| "live_viewer_boot_epoch_unavailable".to_string())?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("live_viewer_heartbeat_begin_failed:{error}"))?;
        let mut state: DesktopControlState =
            load_optional_document(&transaction, DOCUMENT, SCHEMA)?;
        expire_live_viewers(&mut state, request.observed_at_ms, &boot_epoch);
        let record = state
            .live_viewers
            .get_mut(&request.lease_id)
            .ok_or_else(|| "live_viewer_lease_inactive".to_string())?;
        if record.state != "controlling" {
            return Err("live_viewer_lease_inactive".to_string());
        }
        if record.authenticated_principal != request.authenticated_principal
            || record.provider_route_id != request.provider_route_id
            || record.guacamole_connection_id != request.guacamole_connection_id
            || record.guacamole_primary_active_connection_id
                != request.guacamole_primary_active_connection_id
            || record.boot_epoch != boot_epoch
        {
            return Err("live_viewer_authority_mismatch".to_string());
        }
        record.observed_shared_connection_count = request.observed_shared_connection_count;
        record.updated_at_ms = request.observed_at_ms;
        record.expires_at_ms = request
            .observed_at_ms
            .checked_add(LIVE_VIEWER_TTL_MS)
            .ok_or_else(|| "live_viewer_expiry_overflow".to_string())?;
        let handoff_id = record.handoff_id.clone();
        let client_connection_id = record.client_connection_id.clone();
        let controller_epoch = record.controller_epoch;
        let lease = load_and_validate_current(
            &transaction,
            &handoff_id,
            &client_connection_id,
            controller_epoch,
        )?;
        save_document(&transaction, DOCUMENT, SCHEMA, &state)?;
        transaction
            .commit()
            .map_err(|error| format!("live_viewer_heartbeat_commit_failed:{error}"))?;
        Ok(lease)
    }

    pub(crate) fn disconnect_live_viewer(
        &mut self,
        lease_id: &str,
        authenticated_principal: &str,
        disconnected_at_ms: u64,
    ) -> Result<(), String> {
        validate_identifier(lease_id, "live_viewer_lease")?;
        validate_identifier(authenticated_principal, "authenticated_principal")?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("live_viewer_disconnect_begin_failed:{error}"))?;
        let mut state: DesktopControlState =
            load_optional_document(&transaction, DOCUMENT, SCHEMA)?;
        let record = state
            .live_viewers
            .get_mut(lease_id)
            .ok_or_else(|| "live_viewer_lease_inactive".to_string())?;
        if record.authenticated_principal != authenticated_principal {
            return Err("live_viewer_authority_mismatch".to_string());
        }
        record.state = "disconnected".to_string();
        record.updated_at_ms = disconnected_at_ms;
        record.expires_at_ms = disconnected_at_ms;
        if state
            .current_operation_ids
            .get(&record.slot_id)
            .map(String::as_str)
            == Some(record.operation_id.as_str())
        {
            state.current_operation_ids.remove(&record.slot_id);
        }
        save_document(&transaction, DOCUMENT, SCHEMA, &state)?;
        transaction
            .commit()
            .map_err(|error| format!("live_viewer_disconnect_commit_failed:{error}"))?;
        Ok(())
    }

    /// Publish one controller transfer together with its guarded effect and
    /// resulting manager state under a single SQLite commit.
    pub(crate) fn activate_desktop_control<T>(
        &mut self,
        request: &DesktopControlTransferRequest,
        expected_state: &BrowserSessionState,
        effect: impl FnOnce(&DesktopControlLease) -> Result<(T, BrowserSessionState), String>,
    ) -> Result<(T, DesktopControlLease), String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("desktop_control_activation_begin_failed:{error}"))?;
        let persisted_state: BrowserSessionState = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        if &persisted_state != expected_state {
            return Err("desktop_control_session_state_stale".to_string());
        }
        let mut control_state: DesktopControlState =
            load_optional_document(&transaction, DOCUMENT, SCHEMA)?;
        let lease = stage_desktop_control_transfer(&transaction, &mut control_state, request)?;
        let (result, resulting_state) = effect(&lease)?;
        if resulting_state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err(format!(
                "browser_session_state_schema_unsupported:{}",
                resulting_state.schema_version
            ));
        }
        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            &resulting_state,
        )?;
        save_document(&transaction, DOCUMENT, SCHEMA, &control_state)?;
        transaction
            .commit()
            .map_err(|error| format!("desktop_control_activation_commit_failed:{error}"))?;
        Ok((result, lease))
    }

    /// Validate current controller ownership against one consistent SQLite snapshot.
    #[cfg(test)]
    pub(crate) fn current_desktop_control(
        &self,
        handoff_id: &str,
        client_connection_id: &str,
        epoch: u64,
    ) -> Result<DesktopControlLease, String> {
        validate_identifier(handoff_id, "handoff")?;
        validate_identifier(client_connection_id, "client_connection")?;
        if epoch == 0 {
            return Err("desktop_control_epoch_invalid".to_string());
        }
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| format!("desktop_control_read_begin_failed:{error}"))?;
        let lease =
            load_and_validate_current(&transaction, handoff_id, client_connection_id, epoch)?;
        transaction
            .commit()
            .map_err(|error| format!("desktop_control_read_commit_failed:{error}"))?;
        Ok(lease)
    }

    /// Run one short synchronous effect while the validated controller lease
    /// and every competing transfer remain serialized by the SQLite transaction.
    pub(crate) fn with_current_desktop_control<T>(
        &mut self,
        handoff_id: &str,
        client_connection_id: &str,
        epoch: u64,
        expected_state: &BrowserSessionState,
        effect: impl FnOnce(&DesktopControlLease) -> Result<T, String>,
    ) -> Result<T, String> {
        validate_identifier(handoff_id, "handoff")?;
        validate_identifier(client_connection_id, "client_connection")?;
        if epoch == 0 {
            return Err("desktop_control_epoch_invalid".to_string());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("desktop_control_effect_begin_failed:{error}"))?;
        let persisted_state: BrowserSessionState = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        if &persisted_state != expected_state {
            return Err("desktop_control_session_state_stale".to_string());
        }
        let lease =
            load_and_validate_current(&transaction, handoff_id, client_connection_id, epoch)?;
        validate_live_viewer_current(&transaction, &lease, current_time_ms()?)?;
        let result = effect(&lease)?;
        transaction
            .commit()
            .map_err(|error| format!("desktop_control_effect_commit_failed:{error}"))?;
        Ok(result)
    }
}

fn load_live_viewer_control(
    connection: &Connection,
    lease_id: &str,
    now_ms: u64,
) -> Result<LiveViewerControlAuthority, String> {
    let state: DesktopControlState = load_optional_document(connection, DOCUMENT, SCHEMA)?;
    let record = state
        .live_viewers
        .get(lease_id)
        .ok_or_else(|| "authenticated_live_viewer_authority_unavailable".to_string())?;
    let lease = load_and_validate_current(
        connection,
        &record.handoff_id,
        &record.client_connection_id,
        record.controller_epoch,
    )?;
    validate_live_viewer_current(connection, &lease, now_ms)?;
    if record.lease_id != lease_id {
        return Err("authenticated_live_viewer_authority_unavailable".to_string());
    }
    Ok(LiveViewerControlAuthority {
        lease_id: record.lease_id.clone(),
        authenticated_principal: record.authenticated_principal.clone(),
        provider_route_id: record.provider_route_id.clone(),
        updated_at_ms: record.updated_at_ms,
        expires_at_ms: record.expires_at_ms,
        lease,
    })
}

pub(super) fn validate_active_viewer_browser_recovery_authority(
    connection: &Connection,
    lease_id: &str,
    browser_id: &str,
    handoff_id: &str,
    now_ms: u64,
) -> Result<(), String> {
    let state: DesktopControlState = load_optional_document(connection, DOCUMENT, SCHEMA)?;
    let record = state
        .live_viewers
        .get(lease_id)
        .ok_or_else(|| "authenticated_live_viewer_authority_unavailable".to_string())?;
    let current_boot = crate::process_identity::current_boot_epoch()
        .ok_or_else(|| "live_viewer_boot_epoch_unavailable".to_string())?;
    if record.state != "controlling"
        || record.boot_epoch != current_boot
        || record.expires_at_ms <= now_ms
        || record.observed_shared_connection_count == 0
        || record.handoff_id != handoff_id
    {
        return Err("authenticated_live_viewer_authority_unavailable".to_string());
    }
    let lease = load_and_validate_current(
        connection,
        handoff_id,
        &record.client_connection_id,
        record.controller_epoch,
    )?;
    if lease.browser_id != browser_id
        || record.operation_id != lease.operation_id
        || record.slot_id != lease.route_binding.slot_id
        || record.host_generation != lease.host_generation
    {
        return Err("authenticated_live_viewer_authority_unavailable".to_string());
    }
    Ok(())
}

/// Rebind the current controller and viewer lease to target identities emitted
/// by a retained-browser replacement. The caller owns the surrounding SQLite
/// transaction that also publishes session state, the reused handoff, and
/// recovery success.
pub(super) fn rebind_after_browser_recovery(
    connection: &Connection,
    handoff_id: &str,
) -> Result<(), String> {
    let mut state: DesktopControlState = load_optional_document(connection, DOCUMENT, SCHEMA)?;
    let has_current_control = state.current_operation_ids.values().any(|operation_id| {
        state
            .operations
            .get(operation_id)
            .is_some_and(|record| record.handoff_id == handoff_id)
    });
    if !has_current_control {
        return Ok(());
    }
    let resolved = resolve_ready_manager_handoff(connection, handoff_id)?;
    let Some(operation_id) = state
        .current_operation_ids
        .get(&resolved.route_binding.slot_id)
        .cloned()
    else {
        return Ok(());
    };
    let record = state
        .operations
        .get_mut(&operation_id)
        .ok_or_else(|| "desktop_control_state_incomplete".to_string())?;
    if record.handoff_id != handoff_id {
        return Ok(());
    }
    record.browser_id = resolved.browser_id.clone();
    record.session_id = resolved.session_id.clone();
    record.tab_id = resolved.tab_id.clone();
    record.target_id = resolved.target_id.clone();
    record.host_generation = resolved.host_generation;
    record.route_binding = DesktopControlRouteBinding::from_binding(&resolved.route_binding);
    for viewer in state.live_viewers.values_mut().filter(|viewer| {
        viewer.state == "controlling"
            && viewer.operation_id == operation_id
            && viewer.handoff_id == handoff_id
    }) {
        viewer.slot_id = resolved.route_binding.slot_id.clone();
        viewer.host_generation = resolved.host_generation;
    }
    save_document(connection, DOCUMENT, SCHEMA, &state)
}

fn current_time_ms() -> Result<u64, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "live_viewer_clock_invalid".to_string())?
        .as_millis();
    u64::try_from(millis).map_err(|_| "live_viewer_clock_invalid".to_string())
}

fn validate_live_viewer_current(
    connection: &Connection,
    lease: &DesktopControlLease,
    now_ms: u64,
) -> Result<(), String> {
    let state: DesktopControlState = load_optional_document(connection, DOCUMENT, SCHEMA)?;
    let lease_id = format!("live-viewer:{}", lease.operation_id);
    let record = state
        .live_viewers
        .get(&lease_id)
        .ok_or_else(|| "authenticated_live_viewer_authority_unavailable".to_string())?;
    let current_boot = crate::process_identity::current_boot_epoch()
        .ok_or_else(|| "live_viewer_boot_epoch_unavailable".to_string())?;
    if record.state != "controlling"
        || record.operation_id != lease.operation_id
        || record.handoff_id != lease.handoff_id
        || record.client_connection_id != lease.controller_client_connection_id
        || record.slot_id != lease.route_binding.slot_id
        || record.host_generation != lease.host_generation
        || record.controller_epoch != lease.epoch
        || record.boot_epoch != current_boot
        || record.observed_shared_connection_count == 0
        || record.expires_at_ms <= now_ms
    {
        return Err("authenticated_live_viewer_authority_unavailable".to_string());
    }
    Ok(())
}

fn validate_live_viewer_request(request: &LiveViewerActivationRequest) -> Result<(), String> {
    for (value, field) in [
        (&request.operation_id, "operation"),
        (&request.handoff_id, "handoff"),
        (&request.client_connection_id, "client_connection"),
        (&request.authenticated_principal, "authenticated_principal"),
        (&request.provider_route_id, "provider_route"),
        (
            &request.guacamole_primary_active_connection_id,
            "guacamole_primary_active_connection",
        ),
    ] {
        validate_identifier(value, field)?;
    }
    if request.guacamole_connection_id == 0 || request.observed_shared_connection_count == 0 {
        return Err("live_viewer_tunnel_absent".to_string());
    }
    Ok(())
}

fn expire_live_viewers(state: &mut DesktopControlState, now_ms: u64, boot_epoch: &str) {
    let mut expired = Vec::new();
    for record in state.live_viewers.values_mut() {
        if record.state == "controlling"
            && (record.boot_epoch != boot_epoch || record.expires_at_ms <= now_ms)
        {
            record.state = "expired".to_string();
            record.updated_at_ms = now_ms;
            expired.push((record.slot_id.clone(), record.operation_id.clone()));
        }
    }
    for (slot_id, operation_id) in expired {
        if state
            .current_operation_ids
            .get(&slot_id)
            .map(String::as_str)
            == Some(operation_id.as_str())
        {
            state.current_operation_ids.remove(&slot_id);
        }
    }
}

fn stage_desktop_control_transfer(
    connection: &Connection,
    state: &mut DesktopControlState,
    request: &DesktopControlTransferRequest,
) -> Result<DesktopControlLease, String> {
    validate_identifier(&request.operation_id, "operation")?;
    validate_identifier(&request.handoff_id, "handoff")?;
    validate_identifier(&request.client_connection_id, "client_connection")?;
    let resolved = resolve_ready_manager_handoff(connection, &request.handoff_id)?;
    if request.expected_binding != resolved.route_binding {
        return Err("desktop_control_expected_binding_stale".to_string());
    }

    if let Some(record) = state.operations.get(&request.operation_id) {
        if record.handoff_id != request.handoff_id
            || record.controller_client_connection_id != request.client_connection_id
            || record.route_binding
                != DesktopControlRouteBinding::from_binding(&request.expected_binding)
        {
            return Err("desktop_control_operation_conflict".to_string());
        }
        if state
            .current_operation_ids
            .get(&resolved.route_binding.slot_id)
            .map(String::as_str)
            != Some(request.operation_id.as_str())
        {
            return Err("desktop_control_operation_superseded".to_string());
        }
        return validate_record(connection, record);
    }

    let epoch = state
        .next_epoch
        .checked_add(1)
        .ok_or_else(|| "desktop_control_epoch_exhausted".to_string())?;
    let slot_id = resolved.route_binding.slot_id.clone();
    let record = DesktopControlRecord {
        operation_id: request.operation_id.clone(),
        epoch,
        handoff_id: request.handoff_id.clone(),
        browser_id: resolved.browser_id,
        session_id: resolved.session_id,
        tab_id: resolved.tab_id,
        target_id: resolved.target_id,
        controller_client_connection_id: request.client_connection_id.clone(),
        host_generation: resolved.host_generation,
        route_binding: DesktopControlRouteBinding::from_binding(&resolved.route_binding),
    };
    let lease = record.lease(resolved.route_binding);
    state.next_epoch = epoch;
    state
        .current_operation_ids
        .insert(slot_id, request.operation_id.clone());
    state
        .operations
        .insert(request.operation_id.clone(), record);
    Ok(lease)
}

fn load_and_validate_current(
    connection: &Connection,
    handoff_id: &str,
    client_connection_id: &str,
    epoch: u64,
) -> Result<DesktopControlLease, String> {
    let state: DesktopControlState = load_optional_document(connection, DOCUMENT, SCHEMA)?;
    let resolved = resolve_ready_manager_handoff(connection, handoff_id)?;
    let operation_id = state
        .current_operation_ids
        .get(&resolved.route_binding.slot_id)
        .map(String::as_str)
        .ok_or_else(|| "desktop_control_unclaimed".to_string())?;
    let record = state
        .operations
        .get(operation_id)
        .ok_or_else(|| "desktop_control_state_incomplete".to_string())?;
    if record.handoff_id != handoff_id {
        return Err("desktop_control_handoff_stale".to_string());
    }
    if record.controller_client_connection_id != client_connection_id {
        return Err("desktop_control_client_stale".to_string());
    }
    if record.epoch != epoch {
        return Err("desktop_control_epoch_stale".to_string());
    }
    validate_record(connection, record)
}

fn validate_record(
    connection: &Connection,
    record: &DesktopControlRecord,
) -> Result<DesktopControlLease, String> {
    let resolved = resolve_ready_manager_handoff(connection, &record.handoff_id)?;
    if record.browser_id != resolved.browser_id
        || record.session_id != resolved.session_id
        || record.tab_id != resolved.tab_id
        || record.target_id != resolved.target_id
    {
        return Err("desktop_control_target_stale".to_string());
    }
    if record.host_generation != resolved.host_generation {
        return Err("desktop_control_host_generation_stale".to_string());
    }
    if record.route_binding != DesktopControlRouteBinding::from_binding(&resolved.route_binding) {
        return Err("desktop_control_route_binding_stale".to_string());
    }
    Ok(record.lease(resolved.route_binding))
}

fn resolve_ready_manager_handoff(
    connection: &Connection,
    handoff_id: &str,
) -> Result<ResolvedDesktopControl, String> {
    let registry: BrowserManagerHandoffRegistry = load_optional_document(
        connection,
        MANAGER_HANDOFF_REGISTRY_DOCUMENT,
        MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
    )?;
    let handoff = registry
        .handoffs
        .get(handoff_id)
        .ok_or_else(|| "desktop_control_handoff_missing".to_string())?;
    validate_manager_handoff(handoff)?;
    let sessions: BrowserSessionState = load_document(
        connection,
        SESSION_STATE_DOCUMENT,
        BROWSER_SESSION_STATE_SCHEMA_V1,
    )?;
    let session_id = handoff_intent_string(handoff, "sessionId")?;
    let browser_id = handoff_field(&handoff.browser_id, "browser")?;
    let profile_id = handoff_field(&handoff.profile_id, "profile")?;
    let session_name = handoff_field(&handoff.session_name, "session_name")?;
    let tab_id = handoff_field(&handoff.tab_id, "tab")?;
    let target_id = handoff_field(&handoff.target_id, "target")?;
    let session = sessions
        .sessions
        .get(session_id)
        .ok_or_else(|| "desktop_control_session_missing".to_string())?;
    if session.browser_id != browser_id
        || session.name != session_name
        || session.profile_id != profile_id
    {
        return Err("desktop_control_session_attribution_stale".to_string());
    }
    let browser = sessions
        .browsers
        .get(browser_id)
        .ok_or_else(|| "desktop_control_browser_missing".to_string())?;
    if browser.profile_id != profile_id {
        return Err("desktop_control_browser_attribution_stale".to_string());
    }
    let tab = sessions
        .tabs
        .get(tab_id)
        .ok_or_else(|| "desktop_control_tab_missing".to_string())?;
    if tab.session_id != session_id || tab.browser_id != browser_id || tab.target_id != target_id {
        return Err("desktop_control_target_attribution_stale".to_string());
    }
    let desktop = browser
        .desktop
        .as_ref()
        .ok_or_else(|| "desktop_control_desktop_missing".to_string())?;
    if handoff.last_route_id.as_deref() != Some(desktop.route_id.as_str())
        || handoff.last_route_pool_entry_id.as_deref() != Some(desktop.route_id.as_str())
        || handoff_intent_string(handoff, "presentationSlotId")? != desktop.route_id
        || handoff_intent_string(handoff, "displayName")? != desktop.display_name
    {
        return Err("desktop_control_route_attribution_stale".to_string());
    }
    let authority = load_route_keeper_authority_document(connection)?;
    let binding = authority.ready_handoff_binding(&desktop.route_id, &desktop.display_name)?;
    let host_generation = authority
        .host_process_claims
        .keys()
        .next_back()
        .copied()
        .ok_or_else(|| "desktop_control_host_generation_missing".to_string())?;
    if binding.fence.host_generation != host_generation {
        return Err("desktop_control_host_generation_stale".to_string());
    }
    Ok(ResolvedDesktopControl {
        browser_id: browser_id.to_string(),
        session_id: session_id.to_string(),
        tab_id: tab_id.to_string(),
        target_id: target_id.to_string(),
        host_generation,
        route_binding: binding,
    })
}

fn validate_manager_handoff(handoff: &RemoteViewHandoff) -> Result<(), String> {
    if handoff.state != "ready"
        || handoff
            .intent
            .get("browserSessionManager")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        || handoff.view_stream_provider != Some(ViewStreamProvider::RdpGateway)
        || handoff.control_input != Some(ControlInputProvider::ManualAttachedDesktop)
        || handoff
            .last_resolution
            .as_ref()
            .and_then(|resolution| resolution.get("operatorVisible"))
            .and_then(|operator_visible| operator_visible.get("state"))
            .and_then(serde_json::Value::as_str)
            != Some("ready")
    {
        return Err("desktop_control_handoff_not_ready".to_string());
    }
    Ok(())
}

fn handoff_intent_string<'a>(
    handoff: &'a RemoteViewHandoff,
    field: &str,
) -> Result<&'a str, String> {
    handoff
        .intent
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("desktop_control_handoff_intent_missing:{field}"))
}

fn handoff_field<'a>(value: &'a Option<String>, field: &str) -> Result<&'a str, String> {
    value
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("desktop_control_handoff_field_missing:{field}"))
}

fn validate_identifier(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
        return Err(format!("desktop_control_{field}_invalid"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_service_model::{
        BrowserDesktopAssignment, ManagedBrowserInstance, ManagedBrowserSession, ManagedBrowserTab,
        RecordedProcessIdentity, RouteKeeperAdoptionReceipt, RouteKeeperConnectionBinding,
        RouteKeeperConnectionCatalog, RouteKeeperHostProcessClaim, RouteKeeperProtocolReadyReceipt,
        RouteKeeperReconcileAction, RouteKeeperXrdpOwnershipWitness,
    };

    struct Fixture {
        directory: PathBuf,
        store: BrowserRuntimeSqliteStore,
        binding: RouteKeeperHandoffBinding,
        second_binding: RouteKeeperHandoffBinding,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.directory);
        }
    }

    fn host_claim(generation: u64) -> RouteKeeperHostProcessClaim {
        RouteKeeperHostProcessClaim {
            host_generation: generation,
            boot_epoch: format!("boot-{generation}"),
            process_identity: RecordedProcessIdentity {
                pid: 4_000 + u32::try_from(generation).unwrap(),
                start_token: format!("start-{generation}"),
                executable_path: Some("/opt/agent-browser".to_string()),
                browser_family: None,
            },
        }
    }

    fn ready_receipt(
        slot_id: String,
        keeper_id: String,
        fence: agent_browser_service_model::RouteKeeperFence,
        connection_uuid: &str,
    ) -> RouteKeeperProtocolReadyReceipt {
        let sequence = slot_id.rsplit('-').next().unwrap().parse::<u32>().unwrap();
        let session_id = format!("xrdp-{sequence}");
        let display_name = format!(":{sequence}");
        RouteKeeperProtocolReadyReceipt {
            slot_id,
            keeper_id,
            fence,
            guacamole_connection_uuid: connection_uuid.to_string(),
            xrdp_session_id: session_id.clone(),
            display_name: display_name.clone(),
            xrdp_ownership: Some(RouteKeeperXrdpOwnershipWitness {
                schema_version: "agent-browser.route-keeper-xrdp-ownership.v1".to_string(),
                boot_id: "boot-fixture".to_string(),
                route_user: format!("agent-browser-rdp-{sequence}"),
                route_uid: 2_000 + sequence,
                session_id: session_id.clone(),
                session_service: "xrdp-sesman".to_string(),
                session_scope: format!("session-{session_id}.scope"),
                scope_invocation_id: format!("invocation-{sequence}"),
                cgroup_path: format!(
                    "/user.slice/user-{}.slice/session-{session_id}.scope",
                    2_000 + sequence
                ),
                cgroup_device: 28,
                cgroup_inode: 1_000 + u64::from(sequence),
                leader_pid: 4_100 + sequence,
                leader_start_ticks: 5_100 + u64::from(sequence),
                x_server_pid: 4_200 + sequence,
                x_server_start_ticks: 5_200 + u64::from(sequence),
                display_name,
                x11_socket_inode: 6_100 + u64::from(sequence),
            }),
            observed_at: "2026-09-22T12:00:00Z".to_string(),
        }
    }

    fn fixture(label: &str) -> Fixture {
        let directory = std::env::temp_dir().join(format!(
            "agent-browser-desktop-control-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&directory).unwrap();
        let database_path = directory.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let expected = store.load_route_keeper_authority().unwrap();
        let mut authority = expected.clone();
        authority
            .register_host_process_claim(host_claim(1))
            .unwrap();
        authority
            .replace_connection_catalog(
                RouteKeeperConnectionCatalog::with_provider_urls(
                    "http://guacamole.internal",
                    "https://browser.example.test",
                    (1_u32..=6).map(|sequence| RouteKeeperConnectionBinding {
                        slot_id: format!("route-slot-{sequence:02}"),
                        connection_key: format!("route-{sequence:02}"),
                        connection_name: format!("Agent Browser Route {sequence:02}"),
                        route_user: format!("agent-browser-rdp-{sequence}"),
                        guacamole_connection_id: u64::from(sequence),
                    }),
                )
                .unwrap(),
            )
            .unwrap();
        authority.request_ready_slots(2).unwrap();
        for sequence in 1..=2 {
            let (slot_id, keeper_id, fence) = match authority.next_reconcile_action().unwrap() {
                RouteKeeperReconcileAction::Start {
                    slot_id,
                    keeper_id,
                    fence,
                    ..
                } => (slot_id, keeper_id, fence),
                other => panic!("expected start, got {other:?}"),
            };
            authority.record_observing(&slot_id, &fence).unwrap();
            authority
                .record_protocol_ready(ready_receipt(
                    slot_id,
                    keeper_id,
                    fence,
                    &format!("guacamole-{sequence}"),
                ))
                .unwrap();
        }
        store
            .compare_and_swap_route_keeper_authority(&expected, &authority)
            .unwrap();
        let binding = authority
            .ready_handoff_binding("route-slot-01", ":1")
            .unwrap();
        let second_binding = authority
            .ready_handoff_binding("route-slot-02", ":2")
            .unwrap();

        let mut sessions = BrowserSessionState::default();
        sessions.browsers.insert(
            "browser-1".to_string(),
            ManagedBrowserInstance {
                id: "browser-1".to_string(),
                profile_id: "profile-1".to_string(),
                pid: 8_001,
                cdp_endpoint: "http://127.0.0.1:9222".to_string(),
                process_identity: None,
                desktop: Some(BrowserDesktopAssignment {
                    route_id: "route-slot-01".to_string(),
                    display_name: ":1".to_string(),
                    live_browser_count: 0,
                }),
                active_session_ids: vec!["session-1".to_string()],
            },
        );
        sessions.browsers.insert(
            "browser-2".to_string(),
            ManagedBrowserInstance {
                id: "browser-2".to_string(),
                profile_id: "profile-2".to_string(),
                pid: 8_002,
                cdp_endpoint: "http://127.0.0.1:9223".to_string(),
                process_identity: None,
                desktop: Some(BrowserDesktopAssignment {
                    route_id: "route-slot-02".to_string(),
                    display_name: ":2".to_string(),
                    live_browser_count: 0,
                }),
                active_session_ids: vec!["session-2".to_string()],
            },
        );
        sessions.sessions.insert(
            "session-1".to_string(),
            ManagedBrowserSession {
                id: "session-1".to_string(),
                name: "manager-session".to_string(),
                profile_id: "profile-1".to_string(),
                browser_id: "browser-1".to_string(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
                expires_at_ms: 10_000,
                current_tab_id: Some("tab-1".to_string()),
                handoff_ids: Vec::new(),
            },
        );
        sessions.sessions.insert(
            "session-2".to_string(),
            ManagedBrowserSession {
                id: "session-2".to_string(),
                name: "manager-session-2".to_string(),
                profile_id: "profile-2".to_string(),
                browser_id: "browser-2".to_string(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
                expires_at_ms: 10_000,
                current_tab_id: Some("tab-2".to_string()),
                handoff_ids: Vec::new(),
            },
        );
        sessions.tabs.insert(
            "tab-1".to_string(),
            ManagedBrowserTab {
                id: "tab-1".to_string(),
                target_id: "target-1".to_string(),
                browser_id: "browser-1".to_string(),
                session_id: "session-1".to_string(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
            },
        );
        sessions.tabs.insert(
            "tab-2".to_string(),
            ManagedBrowserTab {
                id: "tab-2".to_string(),
                target_id: "target-2".to_string(),
                browser_id: "browser-2".to_string(),
                session_id: "session-2".to_string(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
            },
        );
        store.save_session_state(&sessions).unwrap();
        store
            .save_manager_handoff(&RemoteViewHandoff {
                id: "handoff-1".to_string(),
                state: "ready".to_string(),
                intent: serde_json::json!({
                    "browserSessionManager": true,
                    "sessionId": "session-1",
                    "presentationSlotId": "route-slot-01",
                    "displayName": ":1"
                }),
                profile_id: Some("profile-1".to_string()),
                browser_id: Some("browser-1".to_string()),
                session_name: Some("manager-session".to_string()),
                tab_id: Some("tab-1".to_string()),
                target_id: Some("target-1".to_string()),
                view_stream_provider: Some(ViewStreamProvider::RdpGateway),
                control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                last_route_id: Some("route-slot-01".to_string()),
                last_route_pool_entry_id: Some("route-slot-01".to_string()),
                last_resolution: Some(serde_json::json!({
                    "operatorVisible": { "state": "ready" }
                })),
                ..RemoteViewHandoff::default()
            })
            .unwrap();
        store
            .save_manager_handoff(&RemoteViewHandoff {
                id: "handoff-2".to_string(),
                state: "ready".to_string(),
                intent: serde_json::json!({
                    "browserSessionManager": true,
                    "sessionId": "session-2",
                    "presentationSlotId": "route-slot-02",
                    "displayName": ":2"
                }),
                profile_id: Some("profile-2".to_string()),
                browser_id: Some("browser-2".to_string()),
                session_name: Some("manager-session-2".to_string()),
                tab_id: Some("tab-2".to_string()),
                target_id: Some("target-2".to_string()),
                view_stream_provider: Some(ViewStreamProvider::RdpGateway),
                control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                last_route_id: Some("route-slot-02".to_string()),
                last_route_pool_entry_id: Some("route-slot-02".to_string()),
                last_resolution: Some(serde_json::json!({
                    "operatorVisible": { "state": "ready" }
                })),
                ..RemoteViewHandoff::default()
            })
            .unwrap();
        Fixture {
            directory,
            store,
            binding,
            second_binding,
        }
    }

    fn request(
        operation_id: &str,
        client_connection_id: &str,
        binding: &RouteKeeperHandoffBinding,
    ) -> DesktopControlTransferRequest {
        request_for(operation_id, client_connection_id, "handoff-1", binding)
    }

    fn request_for(
        operation_id: &str,
        client_connection_id: &str,
        handoff_id: &str,
        binding: &RouteKeeperHandoffBinding,
    ) -> DesktopControlTransferRequest {
        DesktopControlTransferRequest {
            operation_id: operation_id.to_string(),
            handoff_id: handoff_id.to_string(),
            client_connection_id: client_connection_id.to_string(),
            expected_binding: binding.clone(),
        }
    }

    fn live_request(
        operation_id: &str,
        client_connection_id: &str,
        connection_id: u64,
    ) -> LiveViewerActivationRequest {
        LiveViewerActivationRequest {
            operation_id: operation_id.to_string(),
            handoff_id: "handoff-1".to_string(),
            client_connection_id: client_connection_id.to_string(),
            authenticated_principal: "operator@example.test".to_string(),
            provider_route_id: "development-route-1".to_string(),
            guacamole_connection_id: connection_id,
            guacamole_primary_active_connection_id: "primary-fixture".to_string(),
            observed_shared_connection_count: 1,
            observed_at_ms: current_time_ms().unwrap(),
        }
    }

    #[test]
    fn failed_atomic_activation_preserves_the_prior_controller_and_session_state() {
        let mut fixture = fixture("activation-failure");
        let prior = fixture
            .store
            .transfer_desktop_control(&request("operation-a", "client-a", &fixture.binding))
            .unwrap();
        let expected_state = fixture.store.load_session_state().unwrap();
        let result: Result<((), DesktopControlLease), String> =
            fixture.store.activate_desktop_control(
                &request("operation-b", "client-b", &fixture.binding),
                &expected_state,
                |_| Err("focus_failed".to_string()),
            );

        assert_eq!(result.unwrap_err(), "focus_failed");
        assert_eq!(fixture.store.load_session_state().unwrap(), expected_state);
        fixture
            .store
            .current_desktop_control("handoff-1", "client-a", prior.epoch)
            .unwrap();
    }

    #[test]
    fn successful_atomic_activation_fences_prior_and_commits_session_state() {
        let mut fixture = fixture("activation-success");
        let prior = fixture
            .store
            .transfer_desktop_control(&request("operation-a", "client-a", &fixture.binding))
            .unwrap();
        let expected_state = fixture.store.load_session_state().unwrap();
        let mut resulting_state = expected_state.clone();
        resulting_state
            .sessions
            .get_mut("session-1")
            .unwrap()
            .last_activity_at_ms = 42;
        let (result, successor) = fixture
            .store
            .activate_desktop_control(
                &request("operation-b", "client-b", &fixture.binding),
                &expected_state,
                |_| Ok(("focused", resulting_state.clone())),
            )
            .unwrap();

        assert_eq!(result, "focused");
        assert_eq!(fixture.store.load_session_state().unwrap(), resulting_state);
        assert_eq!(
            fixture
                .store
                .current_desktop_control("handoff-1", "client-a", prior.epoch)
                .unwrap_err(),
            "desktop_control_client_stale"
        );
        fixture
            .store
            .current_desktop_control("handoff-1", "client-b", successor.epoch)
            .unwrap();
    }

    #[test]
    fn atomic_activation_replays_the_current_operation_without_advancing_epoch() {
        let mut fixture = fixture("activation-replay");
        let request = request("operation-a", "client-a", &fixture.binding);
        let expected_state = fixture.store.load_session_state().unwrap();
        let (_, first) = fixture
            .store
            .activate_desktop_control(&request, &expected_state, |_| {
                Ok(((), expected_state.clone()))
            })
            .unwrap();
        let replay_state = fixture.store.load_session_state().unwrap();
        let (_, replay) = fixture
            .store
            .activate_desktop_control(&request, &replay_state, |_| Ok(((), replay_state.clone())))
            .unwrap();

        assert_eq!(replay, first);
    }

    #[test]
    fn atomic_activation_serializes_an_intervening_transfer() {
        let mut fixture = fixture("activation-intervening-transfer");
        let prior = fixture
            .store
            .transfer_desktop_control(&request("operation-a", "client-a", &fixture.binding))
            .unwrap();
        let expected_state = fixture.store.load_session_state().unwrap();
        let mut competitor = BrowserRuntimeSqliteStore::open(
            &fixture.directory.join(BROWSER_RUNTIME_DATABASE_FILENAME),
        )
        .unwrap();
        competitor
            .connection
            .busy_timeout(std::time::Duration::ZERO)
            .unwrap();
        let competing_request = request("operation-c", "client-c", &fixture.binding);
        let result: Result<((), DesktopControlLease), String> =
            fixture.store.activate_desktop_control(
                &request("operation-b", "client-b", &fixture.binding),
                &expected_state,
                |_| {
                    assert!(competitor
                        .transfer_desktop_control(&competing_request)
                        .unwrap_err()
                        .starts_with("desktop_control_begin_failed:"));
                    Err("focus_failed".to_string())
                },
            );

        assert_eq!(result.unwrap_err(), "focus_failed");
        fixture
            .store
            .current_desktop_control("handoff-1", "client-a", prior.epoch)
            .unwrap();
        let intervening = competitor
            .transfer_desktop_control(&competing_request)
            .unwrap();
        competitor
            .current_desktop_control("handoff-1", "client-c", intervening.epoch)
            .unwrap();
    }

    #[test]
    fn transfer_from_a_to_b_invalidates_a() {
        let mut fixture = fixture("transfer");
        let first = fixture
            .store
            .transfer_desktop_control(&request("operation-a", "client-a", &fixture.binding))
            .unwrap();
        let second = fixture
            .store
            .activate_live_viewer(&live_request(
                "operation-b",
                "client-b",
                fixture.binding.guacamole_connection_id,
            ))
            .unwrap();
        assert!(second.epoch > first.epoch);
        assert_eq!(
            fixture
                .store
                .current_desktop_control("handoff-1", "client-a", first.epoch)
                .unwrap_err(),
            "desktop_control_client_stale"
        );
        let expected_state = fixture.store.load_session_state().unwrap();
        assert_eq!(
            fixture
                .store
                .with_current_desktop_control(
                    "handoff-1",
                    "client-b",
                    second.epoch,
                    &expected_state,
                    |lease| Ok(lease.target_id.clone()),
                )
                .unwrap(),
            "target-1"
        );
    }

    #[test]
    fn current_live_viewer_control_returns_only_the_current_authenticated_lease() {
        let mut fixture = fixture("current-live-viewer-control");
        let first = live_request(
            "operation-a",
            "client-a",
            fixture.binding.guacamole_connection_id,
        );
        fixture.store.activate_live_viewer(&first).unwrap();

        let authority = fixture
            .store
            .current_live_viewer_control("live-viewer:operation-a")
            .unwrap();
        assert_eq!(
            authority.authenticated_principal,
            first.authenticated_principal
        );
        assert_eq!(authority.provider_route_id, first.provider_route_id);
        assert_eq!(authority.lease.browser_id, "browser-1");
        assert_eq!(authority.lease.session_id, "session-1");

        fixture
            .store
            .activate_live_viewer(&live_request(
                "operation-b",
                "client-b",
                fixture.binding.guacamole_connection_id,
            ))
            .unwrap();
        assert!(fixture
            .store
            .current_live_viewer_control("live-viewer:operation-a")
            .is_err());
        assert_eq!(
            fixture
                .store
                .current_live_viewer_control("live-viewer:operation-b")
                .unwrap()
                .lease
                .controller_client_connection_id,
            "client-b"
        );
    }

    #[test]
    fn active_viewer_recovery_projection_is_current_exact_and_deduplicated() {
        let mut fixture = fixture("active-viewer-recovery-projection");
        let now_ms = current_time_ms().unwrap();
        let mut first = live_request(
            "operation-a",
            "client-a",
            fixture.binding.guacamole_connection_id,
        );
        first.observed_at_ms = now_ms;
        fixture.store.activate_live_viewer(&first).unwrap();
        let candidates = fixture
            .store
            .active_viewer_browser_recovery_candidates(now_ms)
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].browser_id, "browser-1");
        assert_eq!(candidates[0].session_id, "session-1");
        assert_eq!(candidates[0].handoff_id, "handoff-1");

        let mut successor = live_request(
            "operation-b",
            "client-b",
            fixture.binding.guacamole_connection_id,
        );
        successor.observed_at_ms = now_ms + 1;
        fixture.store.activate_live_viewer(&successor).unwrap();
        let candidates = fixture
            .store
            .active_viewer_browser_recovery_candidates(now_ms + 1)
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].viewer_lease_id, "live-viewer:operation-b");
    }

    #[test]
    fn inactive_or_foreign_boot_viewers_create_no_browser_recovery_demand() {
        let mut fixture = fixture("inactive-viewer-recovery-projection");
        let now_ms = current_time_ms().unwrap();
        let mut activation = live_request(
            "operation-a",
            "client-a",
            fixture.binding.guacamole_connection_id,
        );
        activation.observed_at_ms = now_ms;
        fixture.store.activate_live_viewer(&activation).unwrap();
        assert!(fixture
            .store
            .active_viewer_browser_recovery_candidates(now_ms + LIVE_VIEWER_TTL_MS)
            .unwrap()
            .is_empty());

        let current_boot = crate::process_identity::current_boot_epoch().unwrap();
        assert!(fixture
            .store
            .active_viewer_browser_recovery_candidates_for_boot(now_ms, "foreign-boot")
            .unwrap()
            .is_empty());
        assert_eq!(
            fixture
                .store
                .active_viewer_browser_recovery_candidates_for_boot(now_ms, &current_boot)
                .unwrap()
                .len(),
            1
        );

        fixture
            .store
            .disconnect_live_viewer(
                "live-viewer:operation-a",
                &activation.authenticated_principal,
                now_ms + 1,
            )
            .unwrap();
        assert!(fixture
            .store
            .active_viewer_browser_recovery_candidates(now_ms + 1)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn historical_or_stale_handoffs_create_no_browser_recovery_demand() {
        let mut fixture = fixture("stale-handoff-recovery-projection");
        let now_ms = current_time_ms().unwrap();
        assert!(fixture
            .store
            .active_viewer_browser_recovery_candidates(now_ms)
            .unwrap()
            .is_empty());
        let mut activation = live_request(
            "operation-a",
            "client-a",
            fixture.binding.guacamole_connection_id,
        );
        activation.observed_at_ms = now_ms;
        fixture.store.activate_live_viewer(&activation).unwrap();
        let ready_authority = fixture.store.load_route_keeper_authority().unwrap();
        let transaction = fixture.store.connection.unchecked_transaction().unwrap();
        save_document(
            &transaction,
            ROUTE_KEEPER_AUTHORITY_DOCUMENT,
            ROUTE_KEEPER_AUTHORITY_SCHEMA_V4,
            &RouteKeeperAuthority::default(),
        )
        .unwrap();
        transaction.commit().unwrap();
        assert!(fixture
            .store
            .active_viewer_browser_recovery_candidates(now_ms)
            .unwrap()
            .is_empty());
        let transaction = fixture.store.connection.unchecked_transaction().unwrap();
        save_document(
            &transaction,
            ROUTE_KEEPER_AUTHORITY_DOCUMENT,
            ROUTE_KEEPER_AUTHORITY_SCHEMA_V4,
            &ready_authority,
        )
        .unwrap();
        transaction.commit().unwrap();

        let mut handoff = fixture
            .store
            .load_handoff_registry()
            .unwrap()
            .handoffs
            .remove("handoff-1")
            .unwrap();
        handoff.state = "closed".to_string();
        fixture.store.save_manager_handoff(&handoff).unwrap();
        assert!(fixture
            .store
            .active_viewer_browser_recovery_candidates(now_ms)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn current_live_viewer_effect_holds_the_sqlite_writer_fence() {
        let mut fixture = fixture("current-live-viewer-effect-fence");
        fixture
            .store
            .activate_live_viewer(&live_request(
                "operation-a",
                "client-a",
                fixture.binding.guacamole_connection_id,
            ))
            .unwrap();
        let expected_state = fixture.store.load_session_state().unwrap();
        let mut competitor = BrowserRuntimeSqliteStore::open(
            &fixture.directory.join(BROWSER_RUNTIME_DATABASE_FILENAME),
        )
        .unwrap();
        competitor
            .connection
            .busy_timeout(std::time::Duration::ZERO)
            .unwrap();
        let guacamole_connection_id = fixture.binding.guacamole_connection_id;

        let observed = fixture
            .store
            .with_current_live_viewer_control(
                "live-viewer:operation-a",
                &expected_state,
                |authority| {
                    assert!(competitor
                        .activate_live_viewer(&live_request(
                            "operation-b",
                            "client-b",
                            guacamole_connection_id,
                        ))
                        .unwrap_err()
                        .starts_with("live_viewer_begin_failed:"));
                    Ok(authority.lease.epoch)
                },
            )
            .unwrap();
        assert!(observed > 0);
    }

    #[test]
    fn live_viewer_heartbeat_at_the_expiry_boundary_is_rejected() {
        let mut fixture = fixture("live-viewer-expiry-boundary");
        let observed_at_ms = current_time_ms().unwrap();
        let mut activation = live_request(
            "operation-a",
            "client-a",
            fixture.binding.guacamole_connection_id,
        );
        activation.observed_at_ms = observed_at_ms;
        fixture.store.activate_live_viewer(&activation).unwrap();

        let result = fixture
            .store
            .heartbeat_live_viewer(&LiveViewerHeartbeatRequest {
                lease_id: "live-viewer:operation-a".to_string(),
                authenticated_principal: activation.authenticated_principal,
                provider_route_id: activation.provider_route_id,
                guacamole_connection_id: activation.guacamole_connection_id,
                guacamole_primary_active_connection_id: activation
                    .guacamole_primary_active_connection_id,
                observed_shared_connection_count: 1,
                observed_at_ms: observed_at_ms + LIVE_VIEWER_TTL_MS,
            });

        assert_eq!(result, Err("live_viewer_lease_inactive".to_string()));
    }

    #[test]
    fn live_viewer_heartbeat_and_disconnect_do_not_refresh_the_browser_session() {
        let mut fixture = fixture("live-viewer-session-separation");
        let before = fixture.store.load_session_state().unwrap();
        let observed_at_ms = current_time_ms().unwrap();
        let mut activation = live_request(
            "operation-a",
            "client-a",
            fixture.binding.guacamole_connection_id,
        );
        activation.observed_at_ms = observed_at_ms;
        let control = fixture.store.activate_live_viewer(&activation).unwrap();
        fixture
            .store
            .heartbeat_live_viewer(&LiveViewerHeartbeatRequest {
                lease_id: "live-viewer:operation-a".to_string(),
                authenticated_principal: activation.authenticated_principal.clone(),
                provider_route_id: activation.provider_route_id,
                guacamole_connection_id: activation.guacamole_connection_id,
                guacamole_primary_active_connection_id: activation
                    .guacamole_primary_active_connection_id,
                observed_shared_connection_count: 1,
                observed_at_ms: observed_at_ms + 1,
            })
            .unwrap();
        assert_eq!(fixture.store.load_session_state().unwrap(), before);

        fixture
            .store
            .disconnect_live_viewer(
                "live-viewer:operation-a",
                &activation.authenticated_principal,
                observed_at_ms + 2,
            )
            .unwrap();
        assert_eq!(fixture.store.load_session_state().unwrap(), before);
        assert_eq!(
            fixture
                .store
                .current_desktop_control("handoff-1", "client-a", control.epoch),
            Err("desktop_control_unclaimed".to_string())
        );
    }

    #[test]
    fn controllers_on_different_slots_remain_independent() {
        let mut fixture = fixture("per-slot");
        let first = fixture
            .store
            .transfer_desktop_control(&request("operation-a", "client-a", &fixture.binding))
            .unwrap();
        let second = fixture
            .store
            .transfer_desktop_control(&request_for(
                "operation-b",
                "client-b",
                "handoff-2",
                &fixture.second_binding,
            ))
            .unwrap();
        fixture
            .store
            .current_desktop_control("handoff-1", "client-a", first.epoch)
            .unwrap();
        fixture
            .store
            .current_desktop_control("handoff-2", "client-b", second.epoch)
            .unwrap();
    }

    #[test]
    fn same_operation_replays_the_current_lease() {
        let mut fixture = fixture("replay");
        let request = request("operation-a", "client-a", &fixture.binding);
        let first = fixture.store.transfer_desktop_control(&request).unwrap();
        let replay = fixture.store.transfer_desktop_control(&request).unwrap();
        assert_eq!(replay, first);
    }

    #[test]
    fn changed_operation_payload_is_rejected() {
        let mut fixture = fixture("conflict");
        fixture
            .store
            .transfer_desktop_control(&request("operation-a", "client-a", &fixture.binding))
            .unwrap();
        assert_eq!(
            fixture
                .store
                .transfer_desktop_control(&request("operation-a", "client-b", &fixture.binding,))
                .unwrap_err(),
            "desktop_control_operation_conflict"
        );
    }

    #[test]
    fn host_and_route_generation_change_invalidates_the_lease() {
        let mut fixture = fixture("generation");
        let lease = fixture
            .store
            .transfer_desktop_control(&request("operation-a", "client-a", &fixture.binding))
            .unwrap();
        let expected = fixture.store.load_route_keeper_authority().unwrap();
        let mut next = expected.clone();
        let previous_ready = next
            .records
            .get("route-slot-01")
            .unwrap()
            .protocol_ready
            .clone()
            .unwrap();
        next.record_disconnect(
            "route-slot-01",
            &previous_ready.fence,
            &previous_ready.guacamole_connection_uuid,
        )
        .unwrap();
        next.register_host_process_claim(host_claim(2)).unwrap();
        let adoption_fence = match next.begin_adoption("route-slot-01", 2).unwrap() {
            RouteKeeperReconcileAction::Adopt { fence, .. } => fence,
            other => panic!("expected adoption, got {other:?}"),
        };
        next.adopt(RouteKeeperAdoptionReceipt {
            previous_host_generation: 1,
            previous_guacamole_connection_uuid: previous_ready.guacamole_connection_uuid,
            ready: ready_receipt(
                "route-slot-01".to_string(),
                "route-keeper-01".to_string(),
                adoption_fence,
                "guacamole-2",
            ),
            adopted_at: "2026-09-22T12:01:00Z".to_string(),
        })
        .unwrap();
        fixture
            .store
            .compare_and_swap_route_keeper_authority(&expected, &next)
            .unwrap();
        assert_eq!(
            fixture
                .store
                .current_desktop_control("handoff-1", "client-a", lease.epoch)
                .unwrap_err(),
            "desktop_control_host_generation_stale"
        );
    }
}
