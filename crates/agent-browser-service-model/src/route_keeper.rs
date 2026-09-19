//! Pure lifecycle authority for provider-neutral presentation route keepers.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const ROUTE_KEEPER_AUTHORITY_SCHEMA_V1: &str = "agent-browser.route-keeper-authority.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteKeeperPhase {
    Absent,
    Starting,
    Observing,
    Adopting,
    Ready,
    Degraded,
    RecoveryFailed,
    Stopping,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperPolicy {
    pub minimum_ready: u32,
    pub warm_target: u32,
    pub maximum_slots: u32,
}

impl Default for RouteKeeperPolicy {
    fn default() -> Self {
        Self {
            minimum_ready: 1,
            warm_target: 4,
            maximum_slots: 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperFence {
    pub host_generation: u64,
    pub operation_id: String,
    pub operation_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperProtocolReadyReceipt {
    pub slot_id: String,
    pub keeper_id: String,
    pub fence: RouteKeeperFence,
    pub guacamole_connection_uuid: String,
    pub xrdp_session_id: String,
    pub display_name: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperAdoptionReceipt {
    pub previous_host_generation: u64,
    pub ready: RouteKeeperProtocolReadyReceipt,
    pub adopted_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperStopReceipt {
    pub slot_id: String,
    pub keeper_id: String,
    pub fence: RouteKeeperFence,
    pub guacamole_connection_uuid: Option<String>,
    pub xrdp_session_id: Option<String>,
    pub stopped_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "disposition")]
pub enum RouteKeeperStopDisposition {
    Stopped,
    Quarantined {
        obligation: RouteKeeperCleanupObligation,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperCleanupObligation {
    pub slot_id: String,
    pub keeper_id: String,
    pub fence: RouteKeeperFence,
    pub preserved_observed_keeper_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperRecord {
    pub slot_id: String,
    pub keeper_id: String,
    pub phase: RouteKeeperPhase,
    pub fence: RouteKeeperFence,
    pub protocol_ready: Option<RouteKeeperProtocolReadyReceipt>,
    pub adoption: Option<RouteKeeperAdoptionReceipt>,
    pub last_stop: Option<RouteKeeperStopReceipt>,
    pub cleanup_obligation: Option<RouteKeeperCleanupObligation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteKeeperStartPriority {
    Minimum,
    Warm,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum RouteKeeperReconcileAction {
    Start {
        slot_id: String,
        keeper_id: String,
        fence: RouteKeeperFence,
        priority: RouteKeeperStartPriority,
    },
    Observe {
        slot_id: String,
        keeper_id: String,
        fence: RouteKeeperFence,
    },
    Adopt {
        slot_id: String,
        keeper_id: String,
        fence: RouteKeeperFence,
        previous_host_generation: u64,
    },
    Stop {
        slot_id: String,
        keeper_id: String,
        fence: RouteKeeperFence,
    },
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperAuthority {
    pub schema_version: String,
    pub policy: RouteKeeperPolicy,
    pub records: BTreeMap<String, RouteKeeperRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteKeeperProviderState {
    Starting,
    Ready,
    Degraded,
    Quarantined,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperProjection {
    pub state: RouteKeeperProviderState,
    pub keeper_count: u32,
    pub ready_count: u32,
    pub desired_minimum: u32,
    pub desired_warm: u32,
    pub minimum_satisfied: bool,
    pub warm_target_satisfied: bool,
}

impl Default for RouteKeeperAuthority {
    fn default() -> Self {
        Self::new(1).expect("the built-in route keeper policy and generation are valid")
    }
}

impl RouteKeeperAuthority {
    pub fn new(host_generation: u64) -> Result<Self, String> {
        Self::with_policy(host_generation, RouteKeeperPolicy::default())
    }

    pub fn with_policy(host_generation: u64, policy: RouteKeeperPolicy) -> Result<Self, String> {
        if host_generation == 0 {
            return Err("route_keeper_host_generation_invalid".to_string());
        }
        validate_policy(&policy)?;
        let records = (1..=policy.maximum_slots)
            .map(|sequence| {
                let slot_id = format!("route-slot-{sequence:02}");
                let keeper_id = format!("route-keeper-{sequence:02}");
                (
                    slot_id.clone(),
                    RouteKeeperRecord {
                        slot_id,
                        keeper_id,
                        phase: RouteKeeperPhase::Absent,
                        fence: RouteKeeperFence {
                            host_generation,
                            operation_id: String::new(),
                            operation_generation: 0,
                        },
                        protocol_ready: None,
                        adoption: None,
                        last_stop: None,
                        cleanup_obligation: None,
                    },
                )
            })
            .collect();
        Ok(Self {
            schema_version: ROUTE_KEEPER_AUTHORITY_SCHEMA_V1.to_string(),
            policy,
            records,
        })
    }

    pub fn projection(&self) -> Result<RouteKeeperProjection, String> {
        self.validate()?;
        let ready_count = self
            .records
            .values()
            .filter(|record| record.phase == RouteKeeperPhase::Ready)
            .count() as u32;
        let keeper_count = self
            .records
            .values()
            .filter(|record| record.phase != RouteKeeperPhase::Absent)
            .count() as u32;
        let state = if self
            .records
            .values()
            .any(|record| record.phase == RouteKeeperPhase::Quarantined)
        {
            RouteKeeperProviderState::Quarantined
        } else if ready_count >= self.policy.warm_target {
            RouteKeeperProviderState::Ready
        } else if ready_count >= self.policy.minimum_ready {
            RouteKeeperProviderState::Starting
        } else if self.records.values().any(|record| {
            matches!(
                record.phase,
                RouteKeeperPhase::Degraded | RouteKeeperPhase::RecoveryFailed
            )
        }) {
            RouteKeeperProviderState::Degraded
        } else {
            RouteKeeperProviderState::Starting
        };
        Ok(RouteKeeperProjection {
            state,
            keeper_count,
            ready_count,
            desired_minimum: self.policy.minimum_ready,
            desired_warm: self.policy.warm_target,
            minimum_satisfied: ready_count >= self.policy.minimum_ready,
            warm_target_satisfied: ready_count >= self.policy.warm_target,
        })
    }

    pub fn begin_adoption(
        &mut self,
        slot_id: &str,
        new_host_generation: u64,
    ) -> Result<RouteKeeperReconcileAction, String> {
        let record = self
            .records
            .get_mut(slot_id)
            .ok_or_else(|| "route_keeper_slot_missing".to_string())?;
        if record.phase != RouteKeeperPhase::Degraded
            || new_host_generation <= record.fence.host_generation
        {
            return Err("route_keeper_adoption_generation_mismatch".to_string());
        }
        let previous_host_generation = record.fence.host_generation;
        record.fence.host_generation = new_host_generation;
        record.fence.operation_generation = record
            .fence
            .operation_generation
            .checked_add(1)
            .ok_or_else(|| "route_keeper_operation_generation_exhausted".to_string())?;
        record.fence.operation_id = format!(
            "route-keeper:{new_host_generation}:{}:{}",
            record.slot_id, record.fence.operation_generation
        );
        record.phase = RouteKeeperPhase::Adopting;
        record.adoption = None;
        Ok(RouteKeeperReconcileAction::Adopt {
            slot_id: record.slot_id.clone(),
            keeper_id: record.keeper_id.clone(),
            fence: record.fence.clone(),
            previous_host_generation,
        })
    }

    pub fn next_reconcile_action(&mut self) -> Result<RouteKeeperReconcileAction, String> {
        self.validate()?;
        if let Some(record) = self
            .records
            .values()
            .find(|record| record.phase == RouteKeeperPhase::Stopping)
        {
            return Ok(RouteKeeperReconcileAction::Stop {
                slot_id: record.slot_id.clone(),
                keeper_id: record.keeper_id.clone(),
                fence: record.fence.clone(),
            });
        }
        if let Some(record) = self
            .records
            .values()
            .find(|record| record.phase == RouteKeeperPhase::Adopting)
        {
            let previous_host_generation = record
                .protocol_ready
                .as_ref()
                .ok_or_else(|| "route_keeper_adoption_source_missing".to_string())?
                .fence
                .host_generation;
            return Ok(RouteKeeperReconcileAction::Adopt {
                slot_id: record.slot_id.clone(),
                keeper_id: record.keeper_id.clone(),
                fence: record.fence.clone(),
                previous_host_generation,
            });
        }
        if let Some(record) = self.records.values().find(|record| {
            matches!(
                record.phase,
                RouteKeeperPhase::Starting | RouteKeeperPhase::Observing
            )
        }) {
            return Ok(RouteKeeperReconcileAction::Observe {
                slot_id: record.slot_id.clone(),
                keeper_id: record.keeper_id.clone(),
                fence: record.fence.clone(),
            });
        }
        let ready_count = self
            .records
            .values()
            .filter(|record| record.phase == RouteKeeperPhase::Ready)
            .count() as u32;
        if ready_count >= self.policy.warm_target {
            return Ok(RouteKeeperReconcileAction::Noop);
        }
        let priority = if ready_count < self.policy.minimum_ready {
            RouteKeeperStartPriority::Minimum
        } else {
            RouteKeeperStartPriority::Warm
        };
        let selected_slot = self
            .records
            .values()
            .find(|record| {
                matches!(
                    record.phase,
                    RouteKeeperPhase::Degraded | RouteKeeperPhase::RecoveryFailed
                )
            })
            .map(|record| record.slot_id.clone())
            .or_else(|| {
                self.records
                    .values()
                    .find(|record| record.phase == RouteKeeperPhase::Absent)
                    .map(|record| record.slot_id.clone())
            });
        let Some(slot_id) = selected_slot else {
            return Ok(RouteKeeperReconcileAction::Noop);
        };
        let record = self
            .records
            .get_mut(&slot_id)
            .ok_or_else(|| "route_keeper_slot_missing".to_string())?;
        let was_degraded = matches!(
            record.phase,
            RouteKeeperPhase::Degraded | RouteKeeperPhase::RecoveryFailed
        );
        record.fence.operation_generation = record
            .fence
            .operation_generation
            .checked_add(1)
            .ok_or_else(|| "route_keeper_operation_generation_exhausted".to_string())?;
        record.fence.operation_id = format!(
            "route-keeper:{}:{}:{}",
            record.fence.host_generation, record.slot_id, record.fence.operation_generation
        );
        record.phase = RouteKeeperPhase::Starting;
        record.adoption = None;
        record.cleanup_obligation = None;
        Ok(RouteKeeperReconcileAction::Start {
            slot_id: record.slot_id.clone(),
            keeper_id: record.keeper_id.clone(),
            fence: record.fence.clone(),
            priority: if was_degraded {
                RouteKeeperStartPriority::Recovery
            } else {
                priority
            },
        })
    }

    pub fn record_observing(
        &mut self,
        slot_id: &str,
        fence: &RouteKeeperFence,
    ) -> Result<(), String> {
        let record = self.record_for_fence_mut(slot_id, fence)?;
        if record.phase != RouteKeeperPhase::Starting {
            return Err("route_keeper_phase_not_starting".to_string());
        }
        record.phase = RouteKeeperPhase::Observing;
        Ok(())
    }

    pub fn record_protocol_ready(
        &mut self,
        receipt: RouteKeeperProtocolReadyReceipt,
    ) -> Result<(), String> {
        validate_ready_receipt(&receipt)?;
        let record = self.record_for_fence_mut(&receipt.slot_id, &receipt.fence)?;
        if record.keeper_id != receipt.keeper_id {
            return Err("route_keeper_keeper_identity_mismatch".to_string());
        }
        if !matches!(
            record.phase,
            RouteKeeperPhase::Starting | RouteKeeperPhase::Observing
        ) {
            return Err("route_keeper_phase_not_observing".to_string());
        }
        record.phase = RouteKeeperPhase::Ready;
        record.protocol_ready = Some(receipt);
        record.cleanup_obligation = None;
        Ok(())
    }

    pub fn record_disconnect(
        &mut self,
        slot_id: &str,
        fence: &RouteKeeperFence,
        guacamole_connection_uuid: &str,
    ) -> Result<(), String> {
        let record = self.record_for_fence_mut(slot_id, fence)?;
        if record.phase != RouteKeeperPhase::Ready {
            return Err("route_keeper_phase_not_ready".to_string());
        }
        let ready = record
            .protocol_ready
            .as_ref()
            .ok_or_else(|| "route_keeper_ready_receipt_missing".to_string())?;
        if ready.guacamole_connection_uuid != guacamole_connection_uuid {
            return Err("route_keeper_connection_identity_mismatch".to_string());
        }
        record.phase = RouteKeeperPhase::Degraded;
        Ok(())
    }

    /// A protocol task that terminates before publishing readiness owns no
    /// usable keeper. Return the exact slot to `Absent` so reconciliation can
    /// reserve a newly fenced attempt instead of polling a dead task forever.
    pub fn record_start_terminated(
        &mut self,
        slot_id: &str,
        fence: &RouteKeeperFence,
    ) -> Result<(), String> {
        let record = self.record_for_fence_mut(slot_id, fence)?;
        if !matches!(
            record.phase,
            RouteKeeperPhase::Starting | RouteKeeperPhase::Observing
        ) {
            return Err("route_keeper_phase_not_starting".to_string());
        }
        if record.protocol_ready.is_some() {
            record.phase = RouteKeeperPhase::RecoveryFailed;
        } else {
            record.phase = RouteKeeperPhase::Absent;
        }
        record.adoption = None;
        record.cleanup_obligation = None;
        Ok(())
    }

    pub fn adopt(&mut self, receipt: RouteKeeperAdoptionReceipt) -> Result<(), String> {
        validate_ready_receipt(&receipt.ready)?;
        let record = self
            .records
            .get_mut(&receipt.ready.slot_id)
            .ok_or_else(|| "route_keeper_slot_missing".to_string())?;
        if record.keeper_id != receipt.ready.keeper_id {
            return Err("route_keeper_keeper_identity_mismatch".to_string());
        }
        if record.phase != RouteKeeperPhase::Adopting
            || record.fence != receipt.ready.fence
            || receipt.ready.fence.host_generation <= receipt.previous_host_generation
            || receipt.adopted_at.is_empty()
        {
            return Err("route_keeper_adoption_generation_mismatch".to_string());
        }
        let previous = record
            .protocol_ready
            .as_ref()
            .ok_or_else(|| "route_keeper_adoption_source_missing".to_string())?;
        if previous.fence.host_generation != receipt.previous_host_generation
            || previous.guacamole_connection_uuid != receipt.ready.guacamole_connection_uuid
            || previous.xrdp_session_id != receipt.ready.xrdp_session_id
            || previous.display_name != receipt.ready.display_name
        {
            return Err("route_keeper_adoption_observation_mismatch".to_string());
        }
        record.fence = receipt.ready.fence.clone();
        record.phase = RouteKeeperPhase::Ready;
        record.protocol_ready = Some(receipt.ready.clone());
        record.adoption = Some(receipt);
        record.cleanup_obligation = None;
        Ok(())
    }

    pub fn begin_stop(
        &mut self,
        slot_id: &str,
        fence: &RouteKeeperFence,
    ) -> Result<RouteKeeperReconcileAction, String> {
        let record = self.record_for_fence_mut(slot_id, fence)?;
        if !matches!(
            record.phase,
            RouteKeeperPhase::Ready | RouteKeeperPhase::Degraded
        ) {
            return Err("route_keeper_phase_not_stoppable".to_string());
        }
        record.phase = RouteKeeperPhase::Stopping;
        Ok(RouteKeeperReconcileAction::Stop {
            slot_id: record.slot_id.clone(),
            keeper_id: record.keeper_id.clone(),
            fence: record.fence.clone(),
        })
    }

    pub fn record_stopped(
        &mut self,
        receipt: RouteKeeperStopReceipt,
    ) -> Result<RouteKeeperStopDisposition, String> {
        let record = self.record_for_fence_mut(&receipt.slot_id, &receipt.fence)?;
        if record.phase != RouteKeeperPhase::Stopping {
            return Err("route_keeper_phase_not_stopping".to_string());
        }
        let expected_guacamole = record
            .protocol_ready
            .as_ref()
            .map(|ready| ready.guacamole_connection_uuid.clone());
        let expected_xrdp = record
            .protocol_ready
            .as_ref()
            .map(|ready| ready.xrdp_session_id.clone());
        if record.keeper_id != receipt.keeper_id
            || receipt.guacamole_connection_uuid != expected_guacamole
            || receipt.xrdp_session_id != expected_xrdp
            || receipt.stopped_at.is_empty()
        {
            let observed_identity = format!(
                "keeper={};guacamole={};xrdp={}",
                receipt.keeper_id,
                receipt
                    .guacamole_connection_uuid
                    .as_deref()
                    .unwrap_or("absent"),
                receipt.xrdp_session_id.as_deref().unwrap_or("absent")
            );
            record.phase = RouteKeeperPhase::Quarantined;
            let obligation = RouteKeeperCleanupObligation {
                slot_id: record.slot_id.clone(),
                keeper_id: record.keeper_id.clone(),
                fence: record.fence.clone(),
                preserved_observed_keeper_id: observed_identity,
                reason: "route_keeper_stop_observation_unproven".to_string(),
            };
            record.cleanup_obligation = Some(obligation.clone());
            return Ok(RouteKeeperStopDisposition::Quarantined { obligation });
        }
        record.phase = RouteKeeperPhase::Absent;
        record.protocol_ready = None;
        record.adoption = None;
        record.last_stop = Some(receipt);
        record.cleanup_obligation = None;
        Ok(RouteKeeperStopDisposition::Stopped)
    }

    pub fn quarantine_unproven_stop(
        &mut self,
        slot_id: &str,
        fence: &RouteKeeperFence,
        preserved_observed_keeper_id: String,
    ) -> Result<RouteKeeperCleanupObligation, String> {
        if preserved_observed_keeper_id.is_empty() {
            return Err("route_keeper_observed_identity_invalid".to_string());
        }
        let record = self.record_for_fence_mut(slot_id, fence)?;
        if record.phase != RouteKeeperPhase::Stopping {
            return Err("route_keeper_phase_not_stopping".to_string());
        }
        let obligation = RouteKeeperCleanupObligation {
            slot_id: record.slot_id.clone(),
            keeper_id: record.keeper_id.clone(),
            fence: record.fence.clone(),
            preserved_observed_keeper_id,
            reason: "route_keeper_stop_ownership_unproven".to_string(),
        };
        record.phase = RouteKeeperPhase::Quarantined;
        record.cleanup_obligation = Some(obligation.clone());
        Ok(obligation)
    }

    fn record_for_fence_mut(
        &mut self,
        slot_id: &str,
        fence: &RouteKeeperFence,
    ) -> Result<&mut RouteKeeperRecord, String> {
        let record = self
            .records
            .get_mut(slot_id)
            .ok_or_else(|| "route_keeper_slot_missing".to_string())?;
        if record.fence != *fence {
            if record.fence.host_generation != fence.host_generation {
                return Err(format!(
                    "route_keeper_generation_stale:{}:{}",
                    fence.host_generation, record.fence.host_generation
                ));
            }
            return Err("route_keeper_stale_observation".to_string());
        }
        Ok(record)
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version != ROUTE_KEEPER_AUTHORITY_SCHEMA_V1 {
            return Err("route_keeper_schema_unsupported".to_string());
        }
        validate_policy(&self.policy)?;
        if self.records.len() != self.policy.maximum_slots as usize {
            return Err("route_keeper_slot_count_invalid".to_string());
        }
        for sequence in 1..=self.policy.maximum_slots {
            let slot_id = format!("route-slot-{sequence:02}");
            let keeper_id = format!("route-keeper-{sequence:02}");
            let record = self
                .records
                .get(&slot_id)
                .ok_or_else(|| "route_keeper_slot_identity_invalid".to_string())?;
            validate_record(record, &slot_id, &keeper_id)?;
        }
        Ok(())
    }
}

fn validate_policy(policy: &RouteKeeperPolicy) -> Result<(), String> {
    if policy.minimum_ready == 0
        || policy.minimum_ready > policy.warm_target
        || policy.warm_target > policy.maximum_slots
    {
        return Err("route_keeper_policy_invalid".to_string());
    }
    Ok(())
}

fn validate_ready_receipt(receipt: &RouteKeeperProtocolReadyReceipt) -> Result<(), String> {
    if receipt.slot_id.is_empty()
        || receipt.keeper_id.is_empty()
        || receipt.fence.host_generation == 0
        || receipt.fence.operation_id.is_empty()
        || receipt.fence.operation_generation == 0
        || receipt.guacamole_connection_uuid.is_empty()
        || receipt.xrdp_session_id.is_empty()
        || receipt.display_name.is_empty()
        || receipt.observed_at.is_empty()
    {
        return Err("route_keeper_ready_receipt_invalid".to_string());
    }
    Ok(())
}

fn validate_record(
    record: &RouteKeeperRecord,
    expected_slot_id: &str,
    expected_keeper_id: &str,
) -> Result<(), String> {
    if record.slot_id != expected_slot_id
        || record.keeper_id != expected_keeper_id
        || record.fence.host_generation == 0
        || (record.fence.operation_generation == 0 && !record.fence.operation_id.is_empty())
        || (record.fence.operation_generation > 0 && record.fence.operation_id.is_empty())
    {
        return Err("route_keeper_record_identity_invalid".to_string());
    }
    if let Some(ready) = &record.protocol_ready {
        validate_ready_receipt(ready)?;
        if ready.slot_id != record.slot_id || ready.keeper_id != record.keeper_id {
            return Err("route_keeper_record_receipt_identity_mismatch".to_string());
        }
    }
    if let Some(adoption) = &record.adoption {
        if adoption.adopted_at.is_empty()
            || record.protocol_ready.as_ref() != Some(&adoption.ready)
            || adoption.previous_host_generation >= adoption.ready.fence.host_generation
        {
            return Err("route_keeper_adoption_receipt_invalid".to_string());
        }
    }
    match record.phase {
        RouteKeeperPhase::Absent => {
            if record.protocol_ready.is_some()
                || record.adoption.is_some()
                || record.cleanup_obligation.is_some()
            {
                return Err("route_keeper_absent_record_invalid".to_string());
            }
        }
        RouteKeeperPhase::Starting | RouteKeeperPhase::Observing => {
            if record.cleanup_obligation.is_some() {
                return Err("route_keeper_active_cleanup_invalid".to_string());
            }
            if let Some(ready) = &record.protocol_ready {
                if ready.fence.host_generation != record.fence.host_generation
                    || ready.fence.operation_generation >= record.fence.operation_generation
                {
                    return Err("route_keeper_recovery_source_invalid".to_string());
                }
            }
        }
        RouteKeeperPhase::RecoveryFailed => {
            let ready = record
                .protocol_ready
                .as_ref()
                .ok_or_else(|| "route_keeper_recovery_source_missing".to_string())?;
            if ready.fence.host_generation != record.fence.host_generation
                || ready.fence.operation_generation >= record.fence.operation_generation
                || record.cleanup_obligation.is_some()
            {
                return Err("route_keeper_recovery_source_invalid".to_string());
            }
        }
        RouteKeeperPhase::Adopting => {
            let ready = record
                .protocol_ready
                .as_ref()
                .ok_or_else(|| "route_keeper_adoption_source_missing".to_string())?;
            if ready.fence.host_generation >= record.fence.host_generation
                || record.cleanup_obligation.is_some()
            {
                return Err("route_keeper_adoption_source_invalid".to_string());
            }
        }
        RouteKeeperPhase::Ready
        | RouteKeeperPhase::Degraded
        | RouteKeeperPhase::Stopping
        | RouteKeeperPhase::Quarantined => {
            let ready = record
                .protocol_ready
                .as_ref()
                .ok_or_else(|| "route_keeper_ready_receipt_missing".to_string())?;
            if ready.fence != record.fence {
                return Err("route_keeper_record_receipt_fence_mismatch".to_string());
            }
            if record.phase == RouteKeeperPhase::Quarantined {
                let obligation = record
                    .cleanup_obligation
                    .as_ref()
                    .ok_or_else(|| "route_keeper_cleanup_obligation_missing".to_string())?;
                if obligation.slot_id != record.slot_id
                    || obligation.keeper_id != record.keeper_id
                    || obligation.fence != record.fence
                    || obligation.preserved_observed_keeper_id.is_empty()
                    || obligation.reason.is_empty()
                {
                    return Err("route_keeper_cleanup_obligation_invalid".to_string());
                }
            } else if record.cleanup_obligation.is_some() {
                return Err("route_keeper_active_cleanup_invalid".to_string());
            }
        }
    }
    Ok(())
}
