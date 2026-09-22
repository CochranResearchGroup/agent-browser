//! Pure lifecycle authority for provider-neutral presentation route keepers.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::RecordedProcessIdentity;

pub const ROUTE_KEEPER_AUTHORITY_SCHEMA_V1: &str = "agent-browser.route-keeper-authority.v1";
pub const ROUTE_KEEPER_AUTHORITY_SCHEMA_V2: &str = "agent-browser.route-keeper-authority.v2";
pub const ROUTE_KEEPER_AUTHORITY_SCHEMA_V3: &str = "agent-browser.route-keeper-authority.v3";
pub const ROUTE_KEEPER_AUTHORITY_SCHEMA_V4: &str = "agent-browser.route-keeper-authority.v4";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperConnectionBinding {
    pub slot_id: String,
    pub connection_key: String,
    pub connection_name: String,
    pub route_user: String,
    pub guacamole_connection_id: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperConnectionCatalog {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_base: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_operator_url: Option<String>,
    pub bindings: BTreeMap<String, RouteKeeperConnectionBinding>,
}

impl RouteKeeperConnectionCatalog {
    pub fn new(
        bindings: impl IntoIterator<Item = RouteKeeperConnectionBinding>,
    ) -> Result<Self, String> {
        let mut catalog = Self::default();
        for binding in bindings {
            if catalog
                .bindings
                .insert(binding.slot_id.clone(), binding)
                .is_some()
            {
                return Err("route_keeper_connection_catalog_slot_duplicate".to_string());
            }
        }
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn with_provider_base(
        provider_base: impl Into<String>,
        bindings: impl IntoIterator<Item = RouteKeeperConnectionBinding>,
    ) -> Result<Self, String> {
        let mut catalog = Self::new(bindings)?;
        let provider_base = provider_base.into();
        if provider_base.trim().is_empty() {
            return Err("route_keeper_connection_catalog_provider_invalid".to_string());
        }
        catalog.provider_base = Some(provider_base);
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn with_provider_urls(
        provider_base: impl Into<String>,
        public_operator_url: impl Into<String>,
        bindings: impl IntoIterator<Item = RouteKeeperConnectionBinding>,
    ) -> Result<Self, String> {
        let mut catalog = Self::with_provider_base(provider_base, bindings)?;
        let public_operator_url = public_operator_url.into();
        if public_operator_url.trim().is_empty() {
            return Err("route_keeper_connection_catalog_public_operator_invalid".to_string());
        }
        catalog.public_operator_url = Some(public_operator_url);
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn digest(&self) -> Result<String, String> {
        self.validate()?;
        let canonical = serde_json::to_vec(self)
            .map_err(|error| format!("route_keeper_connection_catalog_serialize_failed:{error}"))?;
        Ok(format!("{:x}", Sha256::digest(canonical)))
    }

    fn validate(&self) -> Result<(), String> {
        if self
            .provider_base
            .as_ref()
            .is_some_and(|provider_base| provider_base.trim().is_empty())
        {
            return Err("route_keeper_connection_catalog_provider_invalid".to_string());
        }
        if self
            .public_operator_url
            .as_ref()
            .is_some_and(|public_operator_url| public_operator_url.trim().is_empty())
        {
            return Err("route_keeper_connection_catalog_public_operator_invalid".to_string());
        }
        let mut connection_keys = BTreeMap::new();
        let mut connection_names = BTreeMap::new();
        let mut route_users = BTreeMap::new();
        let mut connection_ids = BTreeMap::new();
        for (slot_id, binding) in &self.bindings {
            if slot_id.is_empty()
                || binding.slot_id != *slot_id
                || binding.connection_key.is_empty()
                || binding.connection_name.is_empty()
                || binding.route_user.is_empty()
                || binding.guacamole_connection_id == 0
            {
                return Err("route_keeper_connection_catalog_binding_invalid".to_string());
            }
            if connection_keys
                .insert(binding.connection_key.as_str(), slot_id)
                .is_some()
                || connection_names
                    .insert(binding.connection_name.as_str(), slot_id)
                    .is_some()
                || route_users
                    .insert(binding.route_user.as_str(), slot_id)
                    .is_some()
                || connection_ids
                    .insert(binding.guacamole_connection_id, slot_id)
                    .is_some()
            {
                return Err("route_keeper_connection_catalog_identity_duplicate".to_string());
            }
        }
        Ok(())
    }
}

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

/// Exact process instance that owns one route-keeper host generation.
///
/// Claims are append-only. Retaining predecessor claims allows a successor to
/// prove one process generation exited while routes are adopted individually.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperHostProcessClaim {
    pub host_generation: u64,
    pub boot_epoch: String,
    pub process_identity: RecordedProcessIdentity,
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
    #[serde(default)]
    pub connection_catalog_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperXrdpOwnershipWitness {
    pub schema_version: String,
    pub boot_id: String,
    pub route_user: String,
    pub route_uid: u32,
    pub session_id: String,
    pub session_service: String,
    pub session_scope: String,
    pub scope_invocation_id: String,
    pub cgroup_path: String,
    pub cgroup_device: u64,
    pub cgroup_inode: u64,
    pub leader_pid: u32,
    pub leader_start_ticks: u64,
    pub x_server_pid: u32,
    pub x_server_start_ticks: u64,
    pub display_name: String,
    pub x11_socket_inode: u64,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xrdp_ownership: Option<RouteKeeperXrdpOwnershipWitness>,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteKeeperHandoffBinding {
    pub slot_id: String,
    pub keeper_id: String,
    pub fence: RouteKeeperFence,
    pub route_user: String,
    pub display_name: String,
    pub guacamole_connection_id: u64,
    pub guacamole_connection_uuid: String,
    pub public_operator_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteKeeperAdoptionReceipt {
    pub previous_host_generation: u64,
    /// The predecessor's provider-created transport occurrence.
    ///
    /// Guacamole issues a fresh tunnel UUID when a keeper reconnects. The
    /// route's durable identity is instead fenced by the catalog and exact
    /// XRDP ownership witness, while this value preserves the predecessor
    /// occurrence for audit and stale-event fencing.
    #[serde(default)]
    pub previous_guacamole_connection_uuid: String,
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
    #[serde(default)]
    pub connection_catalog: RouteKeeperConnectionCatalog,
    #[serde(default)]
    pub host_process_claims: BTreeMap<u64, RouteKeeperHostProcessClaim>,
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
        let connection_catalog = RouteKeeperConnectionCatalog::default();
        let connection_catalog_digest = connection_catalog.digest()?;
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
                            connection_catalog_digest: connection_catalog_digest.clone(),
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
            schema_version: ROUTE_KEEPER_AUTHORITY_SCHEMA_V4.to_string(),
            policy,
            connection_catalog,
            host_process_claims: BTreeMap::new(),
            records,
        })
    }

    /// Append one exact host-process claim and move only idle slots to it.
    ///
    /// Active routes remain fenced to their predecessor generation so they can
    /// be recovered independently. A generation can never be rebound to a
    /// different process instance.
    pub fn register_host_process_claim(
        &mut self,
        claim: RouteKeeperHostProcessClaim,
    ) -> Result<(), String> {
        validate_host_process_claim(&claim)?;
        if let Some(existing) = self.host_process_claims.get(&claim.host_generation) {
            if existing != &claim {
                return Err(format!(
                    "route_keeper_host_process_claim_rebound:{}",
                    claim.host_generation
                ));
            }
            let generation = claim.host_generation;
            for record in self
                .records
                .values_mut()
                .filter(|record| record.phase == RouteKeeperPhase::Absent)
            {
                record.fence.host_generation = generation;
                record.fence.operation_id.clear();
                record.fence.operation_generation = 0;
            }
            return self.validate();
        }
        let highest_claimed = self.host_process_claims.keys().next_back().copied();
        let highest_recorded = self
            .records
            .values()
            .map(|record| record.fence.host_generation)
            .max()
            .unwrap_or(0);
        if highest_claimed.is_some_and(|generation| claim.host_generation <= generation)
            || claim.host_generation < highest_recorded
        {
            return Err("route_keeper_host_process_claim_generation_stale".to_string());
        }
        let generation = claim.host_generation;
        self.host_process_claims.insert(generation, claim);
        for record in self
            .records
            .values_mut()
            .filter(|record| record.phase == RouteKeeperPhase::Absent)
        {
            record.fence.host_generation = generation;
            record.fence.operation_id.clear();
            record.fence.operation_generation = 0;
        }
        self.validate()
    }

    pub fn replace_connection_catalog(
        &mut self,
        connection_catalog: RouteKeeperConnectionCatalog,
    ) -> Result<(), String> {
        connection_catalog.validate()?;
        for slot_id in connection_catalog.bindings.keys() {
            if !self.records.contains_key(slot_id) {
                return Err("route_keeper_connection_catalog_slot_unknown".to_string());
            }
        }
        if self.connection_catalog == connection_catalog {
            return Ok(());
        }
        if self
            .records
            .values()
            .any(|record| record.phase != RouteKeeperPhase::Absent)
        {
            return Err("route_keeper_connection_catalog_active".to_string());
        }
        let digest = connection_catalog.digest()?;
        self.connection_catalog = connection_catalog;
        for record in self.records.values_mut() {
            set_record_catalog_digest(record, &digest);
        }
        self.validate()
    }

    pub fn upgrade_from_v1(mut self) -> Result<Self, String> {
        if self.schema_version != ROUTE_KEEPER_AUTHORITY_SCHEMA_V1 {
            return Err("route_keeper_schema_upgrade_source_invalid".to_string());
        }
        if self
            .records
            .values()
            .any(|record| record.phase != RouteKeeperPhase::Absent)
        {
            return Err("route_keeper_v1_active_migration_unproven".to_string());
        }
        self.schema_version = ROUTE_KEEPER_AUTHORITY_SCHEMA_V2.to_string();
        self.connection_catalog = RouteKeeperConnectionCatalog::default();
        let digest = self.connection_catalog.digest()?;
        for record in self.records.values_mut() {
            set_record_catalog_digest(record, &digest);
        }
        self.upgrade_from_v2()
    }

    pub fn upgrade_from_v2(mut self) -> Result<Self, String> {
        if self.schema_version != ROUTE_KEEPER_AUTHORITY_SCHEMA_V2 {
            return Err("route_keeper_schema_upgrade_source_invalid".to_string());
        }
        if self
            .records
            .values()
            .any(|record| record.phase != RouteKeeperPhase::Absent)
        {
            return Err("route_keeper_v2_active_host_claim_migration_unproven".to_string());
        }
        self.schema_version = ROUTE_KEEPER_AUTHORITY_SCHEMA_V3.to_string();
        self.host_process_claims.clear();
        self.validate()?;
        self.upgrade_from_v3()
    }

    /// Upgrade v3 records whose adoption receipt did not retain a separate
    /// predecessor transport occurrence. v3 required the ready receipt to
    /// carry the same Guacamole UUID as its predecessor, so the current ready
    /// value is the only deterministic migration source.
    pub fn upgrade_from_v3(mut self) -> Result<Self, String> {
        if self.schema_version != ROUTE_KEEPER_AUTHORITY_SCHEMA_V3 {
            return Err("route_keeper_schema_upgrade_source_invalid".to_string());
        }
        for record in self.records.values_mut() {
            let Some(adoption) = record.adoption.as_mut() else {
                continue;
            };
            let ready = record
                .protocol_ready
                .as_ref()
                .filter(|ready| *ready == &adoption.ready)
                .ok_or_else(|| "route_keeper_v3_adoption_migration_unproven".to_string())?;
            adoption.previous_guacamole_connection_uuid = ready.guacamole_connection_uuid.clone();
        }
        self.schema_version = ROUTE_KEEPER_AUTHORITY_SCHEMA_V4.to_string();
        self.validate()?;
        Ok(self)
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

    pub fn ready_handoff_binding(
        &self,
        slot_id: &str,
        display_name: &str,
    ) -> Result<RouteKeeperHandoffBinding, String> {
        self.validate()?;
        let record = self
            .records
            .get(slot_id)
            .ok_or_else(|| "route_keeper_handoff_slot_missing".to_string())?;
        if record.phase != RouteKeeperPhase::Ready {
            return Err("route_keeper_handoff_not_ready".to_string());
        }
        let ready = record
            .protocol_ready
            .as_ref()
            .ok_or_else(|| "route_keeper_handoff_ready_receipt_missing".to_string())?;
        if ready.display_name != display_name {
            return Err("route_keeper_handoff_display_mismatch".to_string());
        }
        let binding = self
            .connection_catalog
            .bindings
            .get(slot_id)
            .ok_or_else(|| "route_keeper_handoff_connection_unconfigured".to_string())?;
        let ownership = ready
            .xrdp_ownership
            .as_ref()
            .ok_or_else(|| "route_keeper_ready_xrdp_ownership_missing".to_string())?;
        if ownership.route_user != binding.route_user {
            return Err("route_keeper_handoff_route_user_mismatch".to_string());
        }
        let public_operator_url = self
            .connection_catalog
            .public_operator_url
            .as_ref()
            .filter(|url| !url.trim().is_empty())
            .ok_or_else(|| "route_keeper_handoff_public_operator_unconfigured".to_string())?;
        Ok(RouteKeeperHandoffBinding {
            slot_id: record.slot_id.clone(),
            keeper_id: record.keeper_id.clone(),
            fence: record.fence.clone(),
            route_user: binding.route_user.clone(),
            display_name: ready.display_name.clone(),
            guacamole_connection_id: binding.guacamole_connection_id,
            guacamole_connection_uuid: ready.guacamole_connection_uuid.clone(),
            public_operator_url: public_operator_url.clone(),
        })
    }

    pub fn begin_adoption(
        &mut self,
        slot_id: &str,
        new_host_generation: u64,
    ) -> Result<RouteKeeperReconcileAction, String> {
        if !self.host_process_claims.contains_key(&new_host_generation) {
            return Err(format!(
                "route_keeper_host_process_claim_missing:{new_host_generation}"
            ));
        }
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

    /// Transfer an interrupted logical adoption to a newer registered host.
    ///
    /// The caller must prove that both the retained ready host and the host of
    /// `expected_fence` have exited before publishing this transition. This pure
    /// model method verifies the exact pending operation; it does not observe
    /// processes or confer process-exit authority. The logical operation number
    /// and original ready evidence survive, while the host fence changes.
    pub fn refence_interrupted_adoption(
        &mut self,
        slot_id: &str,
        expected_fence: &RouteKeeperFence,
        new_host_generation: u64,
    ) -> Result<RouteKeeperReconcileAction, String> {
        self.validate()?;
        if !self.host_process_claims.contains_key(&new_host_generation) {
            return Err(format!(
                "route_keeper_host_process_claim_missing:{new_host_generation}"
            ));
        }
        let record = self
            .records
            .get(slot_id)
            .ok_or_else(|| "route_keeper_slot_missing".to_string())?;
        let ready = record
            .protocol_ready
            .as_ref()
            .ok_or_else(|| "route_keeper_adoption_source_missing".to_string())?;
        let operation_generation = ready
            .fence
            .operation_generation
            .checked_add(1)
            .ok_or_else(|| "route_keeper_operation_generation_exhausted".to_string())?;
        if record.phase != RouteKeeperPhase::Adopting
            || record.fence != *expected_fence
            || record.adoption.is_some()
            || new_host_generation <= expected_fence.host_generation
            || expected_fence.operation_generation != operation_generation
            || expected_fence.operation_id
                != format!(
                    "route-keeper:{}:{slot_id}:{operation_generation}",
                    expected_fence.host_generation
                )
        {
            return Err("route_keeper_interrupted_adoption_candidate_changed".to_string());
        }
        let previous_host_generation = ready.fence.host_generation;
        let record = self
            .records
            .get_mut(slot_id)
            .expect("validated route exists");
        record.fence.host_generation = new_host_generation;
        record.fence.operation_id =
            format!("route-keeper:{new_host_generation}:{slot_id}:{operation_generation}");
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
        if self.connection_catalog.bindings.is_empty() {
            return Err("route_keeper_connection_catalog_empty".to_string());
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
                self.connection_catalog
                    .bindings
                    .contains_key(&record.slot_id)
                    && matches!(
                        record.phase,
                        RouteKeeperPhase::Degraded | RouteKeeperPhase::RecoveryFailed
                    )
            })
            .map(|record| record.slot_id.clone())
            .or_else(|| {
                self.records
                    .values()
                    .find(|record| {
                        record.phase == RouteKeeperPhase::Absent
                            && self
                                .connection_catalog
                                .bindings
                                .contains_key(&record.slot_id)
                    })
                    .map(|record| record.slot_id.clone())
            });
        let Some(slot_id) = selected_slot else {
            return Err("route_keeper_connection_catalog_capacity_insufficient".to_string());
        };
        let record = self
            .records
            .get_mut(&slot_id)
            .ok_or_else(|| "route_keeper_slot_missing".to_string())?;
        if !self
            .host_process_claims
            .contains_key(&record.fence.host_generation)
        {
            return Err(format!(
                "route_keeper_host_process_claim_missing:{}",
                record.fence.host_generation
            ));
        }
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
        record.fence.connection_catalog_digest = self.connection_catalog.digest()?;
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

    /// Record termination of the exact successor task while adopting a route.
    ///
    /// The predecessor ready receipt remains the recovery source. A transport
    /// UUID, when the connector had already published one, must not identify
    /// that predecessor occurrence; the connector supplies the separate task
    /// occurrence fence before calling this durable transition.
    pub fn record_adoption_terminated(
        &mut self,
        slot_id: &str,
        fence: &RouteKeeperFence,
        guacamole_connection_uuid: Option<&str>,
    ) -> Result<(), String> {
        let record = self.record_for_fence_mut(slot_id, fence)?;
        if record.phase != RouteKeeperPhase::Adopting {
            return Err("route_keeper_phase_not_adopting".to_string());
        }
        let predecessor = record
            .protocol_ready
            .as_ref()
            .ok_or_else(|| "route_keeper_adoption_source_missing".to_string())?;
        if guacamole_connection_uuid.is_some_and(str::is_empty) {
            return Err("route_keeper_adoption_connection_identity_invalid".to_string());
        }
        if guacamole_connection_uuid
            .is_some_and(|uuid| uuid == predecessor.guacamole_connection_uuid)
        {
            return Err("route_keeper_connection_identity_mismatch".to_string());
        }
        record.phase = RouteKeeperPhase::RecoveryFailed;
        record.adoption = None;
        record.cleanup_obligation = None;
        Ok(())
    }

    pub fn adopt(&mut self, receipt: RouteKeeperAdoptionReceipt) -> Result<(), String> {
        validate_ready_receipt(&receipt.ready)?;
        let expected_catalog_digest = self.connection_catalog.digest()?;
        let binding = self
            .connection_catalog
            .bindings
            .get(&receipt.ready.slot_id)
            .ok_or_else(|| "route_keeper_adoption_connection_unconfigured".to_string())?;
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
            || receipt.ready.fence.connection_catalog_digest != expected_catalog_digest
        {
            return Err("route_keeper_adoption_generation_mismatch".to_string());
        }
        let previous = record
            .protocol_ready
            .as_ref()
            .ok_or_else(|| "route_keeper_adoption_source_missing".to_string())?;
        if previous.fence.host_generation != receipt.previous_host_generation
            || receipt.previous_guacamole_connection_uuid != previous.guacamole_connection_uuid
            || receipt.previous_guacamole_connection_uuid.is_empty()
            || previous.xrdp_session_id != receipt.ready.xrdp_session_id
            || previous.display_name != receipt.ready.display_name
            || previous.xrdp_ownership != receipt.ready.xrdp_ownership
            || previous
                .xrdp_ownership
                .as_ref()
                .is_none_or(|ownership| ownership.route_user != binding.route_user)
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
        if self.schema_version != ROUTE_KEEPER_AUTHORITY_SCHEMA_V3
            && self.schema_version != ROUTE_KEEPER_AUTHORITY_SCHEMA_V4
        {
            return Err("route_keeper_schema_unsupported".to_string());
        }
        validate_policy(&self.policy)?;
        for (generation, claim) in &self.host_process_claims {
            validate_host_process_claim(claim)?;
            if *generation != claim.host_generation {
                return Err("route_keeper_host_process_claim_key_mismatch".to_string());
            }
        }
        self.connection_catalog.validate()?;
        let connection_catalog_digest = self.connection_catalog.digest()?;
        for slot_id in self.connection_catalog.bindings.keys() {
            if !self.records.contains_key(slot_id) {
                return Err("route_keeper_connection_catalog_slot_unknown".to_string());
            }
        }
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
            validate_record(record, &slot_id, &keeper_id, &connection_catalog_digest)?;
            if self.schema_version == ROUTE_KEEPER_AUTHORITY_SCHEMA_V4
                && record
                    .adoption
                    .as_ref()
                    .is_some_and(|adoption| adoption.previous_guacamole_connection_uuid.is_empty())
            {
                return Err("route_keeper_adoption_receipt_invalid".to_string());
            }
            if record.phase != RouteKeeperPhase::Absent
                && !self
                    .host_process_claims
                    .contains_key(&record.fence.host_generation)
            {
                return Err(format!(
                    "route_keeper_host_process_claim_missing:{}",
                    record.fence.host_generation
                ));
            }
            if let Some(ready) = &record.protocol_ready {
                if !self
                    .host_process_claims
                    .contains_key(&ready.fence.host_generation)
                {
                    return Err(format!(
                        "route_keeper_host_process_claim_missing:{}",
                        ready.fence.host_generation
                    ));
                }
            }
        }
        Ok(())
    }
}

fn validate_host_process_claim(claim: &RouteKeeperHostProcessClaim) -> Result<(), String> {
    if claim.host_generation == 0
        || claim.boot_epoch.trim().is_empty()
        || claim.process_identity.pid == 0
        || claim.process_identity.start_token.trim().is_empty()
        || claim
            .process_identity
            .executable_path
            .as_deref()
            .is_none_or(|path| path.trim().is_empty())
        || claim.process_identity.browser_family.is_some()
    {
        return Err("route_keeper_host_process_claim_invalid".to_string());
    }
    Ok(())
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
        || !is_sha256_hex(&receipt.fence.connection_catalog_digest)
        || receipt.guacamole_connection_uuid.is_empty()
        || receipt.xrdp_session_id.is_empty()
        || receipt.display_name.is_empty()
        || receipt.observed_at.is_empty()
    {
        return Err("route_keeper_ready_receipt_invalid".to_string());
    }
    let ownership = receipt
        .xrdp_ownership
        .as_ref()
        .ok_or_else(|| "route_keeper_ready_xrdp_ownership_missing".to_string())?;
    if ownership.schema_version != "agent-browser.route-keeper-xrdp-ownership.v1"
        || ownership.boot_id.is_empty()
        || ownership.route_user.is_empty()
        || ownership.route_uid == 0
        || ownership.session_id != receipt.xrdp_session_id
        || ownership.session_service != "xrdp-sesman"
        || ownership.session_scope != format!("session-{}.scope", ownership.session_id)
        || ownership.scope_invocation_id.is_empty()
        || ownership.cgroup_path
            != format!(
                "/user.slice/user-{}.slice/{}",
                ownership.route_uid, ownership.session_scope
            )
        || ownership.cgroup_device == 0
        || ownership.cgroup_inode == 0
        || ownership.leader_pid == 0
        || ownership.leader_start_ticks == 0
        || ownership.x_server_pid == 0
        || ownership.x_server_start_ticks == 0
        || ownership.display_name != receipt.display_name
        || !valid_x11_display_name(&ownership.display_name)
        || ownership.x11_socket_inode == 0
    {
        return Err("route_keeper_ready_xrdp_ownership_invalid".to_string());
    }
    Ok(())
}

fn valid_x11_display_name(value: &str) -> bool {
    value.strip_prefix(':').is_some_and(|number| {
        !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn validate_record(
    record: &RouteKeeperRecord,
    expected_slot_id: &str,
    expected_keeper_id: &str,
    expected_connection_catalog_digest: &str,
) -> Result<(), String> {
    if record.slot_id != expected_slot_id
        || record.keeper_id != expected_keeper_id
        || record.fence.host_generation == 0
        || record.fence.connection_catalog_digest != expected_connection_catalog_digest
        || (record.fence.operation_generation == 0 && !record.fence.operation_id.is_empty())
        || (record.fence.operation_generation > 0 && record.fence.operation_id.is_empty())
    {
        return Err("route_keeper_record_identity_invalid".to_string());
    }
    if let Some(ready) = &record.protocol_ready {
        validate_ready_receipt(ready)?;
        if ready.slot_id != record.slot_id
            || ready.keeper_id != record.keeper_id
            || ready.fence.connection_catalog_digest != record.fence.connection_catalog_digest
        {
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
                if ready.fence.host_generation > record.fence.host_generation
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
            if ready.fence.host_generation > record.fence.host_generation
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

fn set_record_catalog_digest(record: &mut RouteKeeperRecord, digest: &str) {
    record.fence.connection_catalog_digest = digest.to_string();
    if let Some(ready) = record.protocol_ready.as_mut() {
        ready.fence.connection_catalog_digest = digest.to_string();
    }
    if let Some(adoption) = record.adoption.as_mut() {
        adoption.ready.fence.connection_catalog_digest = digest.to_string();
    }
    if let Some(stop) = record.last_stop.as_mut() {
        stop.fence.connection_catalog_digest = digest.to_string();
    }
    if let Some(obligation) = record.cleanup_obligation.as_mut() {
        obligation.fence.connection_catalog_digest = digest.to_string();
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host_claim(host_generation: u64) -> RouteKeeperHostProcessClaim {
        RouteKeeperHostProcessClaim {
            host_generation,
            boot_epoch: format!("boot:{host_generation}"),
            process_identity: RecordedProcessIdentity {
                pid: u32::try_from(4_000 + host_generation).unwrap(),
                start_token: format!("start:{host_generation}"),
                executable_path: Some("/opt/agent-browser".to_string()),
                browser_family: None,
            },
        }
    }

    fn ready_receipt(
        slot_id: &str,
        keeper_id: &str,
        fence: RouteKeeperFence,
        guacamole_connection_uuid: &str,
    ) -> RouteKeeperProtocolReadyReceipt {
        let session_id = "xrdp-fixture".to_string();
        let display_name = ":42".to_string();
        RouteKeeperProtocolReadyReceipt {
            slot_id: slot_id.to_string(),
            keeper_id: keeper_id.to_string(),
            fence,
            guacamole_connection_uuid: guacamole_connection_uuid.to_string(),
            xrdp_session_id: session_id.clone(),
            display_name: display_name.clone(),
            xrdp_ownership: Some(RouteKeeperXrdpOwnershipWitness {
                schema_version: "agent-browser.route-keeper-xrdp-ownership.v1".to_string(),
                boot_id: "boot-fixture".to_string(),
                route_user: "agent-browser-rdp-1".to_string(),
                route_uid: 2_001,
                session_id: session_id.clone(),
                session_service: "xrdp-sesman".to_string(),
                session_scope: format!("session-{session_id}.scope"),
                scope_invocation_id: "invocation-fixture".to_string(),
                cgroup_path: format!("/user.slice/user-2001.slice/session-{session_id}.scope"),
                cgroup_device: 28,
                cgroup_inode: 1_001,
                leader_pid: 4_101,
                leader_start_ticks: 5_101,
                x_server_pid: 4_102,
                x_server_start_ticks: 5_102,
                display_name,
                x11_socket_inode: 6_101,
            }),
            observed_at: "2026-09-21T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn adoption_fences_transport_occurrences_and_requires_exact_xrdp_witness() {
        let mut authority = RouteKeeperAuthority::new(1).unwrap();
        assert_eq!(authority.schema_version, ROUTE_KEEPER_AUTHORITY_SCHEMA_V4);
        authority
            .register_host_process_claim(host_claim(1))
            .unwrap();
        authority
            .replace_connection_catalog(
                RouteKeeperConnectionCatalog::new([RouteKeeperConnectionBinding {
                    slot_id: "route-slot-01".to_string(),
                    connection_key: "route-01".to_string(),
                    connection_name: "Route 01".to_string(),
                    route_user: "agent-browser-rdp-1".to_string(),
                    guacamole_connection_id: 1,
                }])
                .unwrap(),
            )
            .unwrap();
        let (slot_id, keeper_id, original_fence) = match authority.next_reconcile_action().unwrap()
        {
            RouteKeeperReconcileAction::Start {
                slot_id,
                keeper_id,
                fence,
                ..
            } => (slot_id, keeper_id, fence),
            other => panic!("expected start action, got {other:?}"),
        };
        let original = ready_receipt(
            &slot_id,
            &keeper_id,
            original_fence.clone(),
            "predecessor-occurrence",
        );
        authority.record_protocol_ready(original.clone()).unwrap();
        authority
            .record_disconnect(
                &slot_id,
                &original_fence,
                &original.guacamole_connection_uuid,
            )
            .unwrap();
        authority
            .register_host_process_claim(host_claim(2))
            .unwrap();
        let adoption_fence = match authority.begin_adoption(&slot_id, 2).unwrap() {
            RouteKeeperReconcileAction::Adopt { fence, .. } => fence,
            other => panic!("expected adoption action, got {other:?}"),
        };
        let successor = RouteKeeperProtocolReadyReceipt {
            fence: adoption_fence,
            guacamole_connection_uuid: "successor-occurrence".to_string(),
            observed_at: "2026-09-21T12:01:00Z".to_string(),
            ..original.clone()
        };
        let mut witness_drift = successor.clone();
        witness_drift.xrdp_ownership.as_mut().unwrap().cgroup_inode += 1;
        let mut drift_authority = authority.clone();
        assert_eq!(
            drift_authority.adopt(RouteKeeperAdoptionReceipt {
                previous_host_generation: 1,
                previous_guacamole_connection_uuid: original.guacamole_connection_uuid.clone(),
                ready: witness_drift,
                adopted_at: "2026-09-21T12:01:01Z".to_string(),
            }),
            Err("route_keeper_adoption_observation_mismatch".to_string())
        );
        let mut catalog_drift = successor.clone();
        catalog_drift.fence.connection_catalog_digest = "f".repeat(64);
        let mut catalog_drift_authority = authority.clone();
        assert_eq!(
            catalog_drift_authority.adopt(RouteKeeperAdoptionReceipt {
                previous_host_generation: 1,
                previous_guacamole_connection_uuid: original.guacamole_connection_uuid.clone(),
                ready: catalog_drift,
                adopted_at: "2026-09-21T12:01:01Z".to_string(),
            }),
            Err("route_keeper_adoption_generation_mismatch".to_string())
        );
        let mut missing_predecessor_authority = authority.clone();
        assert_eq!(
            missing_predecessor_authority.adopt(RouteKeeperAdoptionReceipt {
                previous_host_generation: 1,
                previous_guacamole_connection_uuid: String::new(),
                ready: successor.clone(),
                adopted_at: "2026-09-21T12:01:01Z".to_string(),
            }),
            Err("route_keeper_adoption_observation_mismatch".to_string())
        );

        authority
            .adopt(RouteKeeperAdoptionReceipt {
                previous_host_generation: 1,
                previous_guacamole_connection_uuid: original.guacamole_connection_uuid.clone(),
                ready: successor.clone(),
                adopted_at: "2026-09-21T12:01:01Z".to_string(),
            })
            .unwrap();
        let record = &authority.records[&slot_id];
        let adoption = record.adoption.as_ref().unwrap();
        assert_eq!(
            adoption.previous_guacamole_connection_uuid,
            "predecessor-occurrence"
        );
        assert_eq!(
            adoption.ready.guacamole_connection_uuid,
            "successor-occurrence"
        );
        assert_eq!(
            authority.record_disconnect(
                &slot_id,
                &original_fence,
                &original.guacamole_connection_uuid,
            ),
            Err("route_keeper_generation_stale:1:2".to_string())
        );
        assert_eq!(authority.records[&slot_id].phase, RouteKeeperPhase::Ready);
        assert_eq!(
            authority.record_disconnect(
                &slot_id,
                &successor.fence,
                &original.guacamole_connection_uuid,
            ),
            Err("route_keeper_connection_identity_mismatch".to_string())
        );
        assert_eq!(authority.records[&slot_id].phase, RouteKeeperPhase::Ready);

        let mut v3 = authority.clone();
        v3.schema_version = ROUTE_KEEPER_AUTHORITY_SCHEMA_V3.to_string();
        v3.records
            .get_mut(&slot_id)
            .unwrap()
            .adoption
            .as_mut()
            .unwrap()
            .previous_guacamole_connection_uuid
            .clear();
        let migrated = v3.upgrade_from_v3().unwrap();
        assert_eq!(migrated.schema_version, ROUTE_KEEPER_AUTHORITY_SCHEMA_V4);
        assert_eq!(
            migrated.records[&slot_id]
                .adoption
                .as_ref()
                .unwrap()
                .previous_guacamole_connection_uuid,
            "successor-occurrence"
        );
    }
}
