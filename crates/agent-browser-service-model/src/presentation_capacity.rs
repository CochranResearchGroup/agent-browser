//! Provider-free presentation inventory, admission, queue, and transition authority.
//!
//! CLI adapters supply observations from their authoritative service records.
//! This module owns deterministic capacity policy and durable wire compatibility.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Immutable facts prepared by the CLI adapter for one authority snapshot.
///
/// Entries cover every durable slot in inventory order and bind its exact slot,
/// route, and display identities. Rebuild after inventory or binding changes.
/// Missing service records are represented by false/empty facts, not omitted
/// entries. This envelope is transient and is never serialized into authority.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PresentationCapacityObservations {
    pub slots: Vec<PresentationSlotObservation>,
    pub binding_warnings: Vec<String>,
}

/// Provider-free observations; request-dependent exemptions remain model policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PresentationSlotObservation {
    pub slot_id: String,
    pub route_id: Option<String>,
    pub display_allocation_id: Option<String>,
    pub acquisition_lease_active: bool,
    pub human_controller_active: bool,
    pub non_controller_staging_viewer_active: bool,
    pub live_handoff_browser_ids: BTreeSet<String>,
    pub authoritative_browser_id: Option<String>,
}

/// A ready provider binding retained by a current acquisition reservation.
/// The adapter qualifies provider readiness and acquisition custody; the model
/// matches the old slot's exact identities and non-null browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationAcquisitionRetention {
    pub slot_id: String,
    pub route_id: String,
    pub display_allocation_id: String,
    pub browser_id: String,
}

/// Service joins for one exact old slot binding during inventory revalidation.
/// These transient facts do not select a capacity transition. A pending binding
/// includes equality of the route and qualified browser, including two absent
/// browsers. An inert owner requires matching released/orphaned route and
/// display states, an available unchecked-out pool entry, and matching owners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationInventoryCustodyObservation {
    pub slot_id: String,
    pub route_id: Option<String>,
    pub display_allocation_id: Option<String>,
    pub exact_pending_binding: bool,
    pub completed_inert_owner_browser_id: Option<String>,
}

/// Retirement cannot reset browser presentation while an episode owns a lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationRetirementConflict {
    ActivePresentationLease,
}

impl PresentationSlotObservation {
    fn matches(&self, slot: &PresentationSlot) -> bool {
        self.slot_id == slot.id
            && self.route_id == slot.route_id
            && self.display_allocation_id == slot.display_allocation_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationSlotState {
    Absent,
    Provisioning,
    WarmIdle,
    Reserved,
    Staging,
    CaptureReady,
    Active,
    Restoring,
    Cooling,
    Reclaiming,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationSlot {
    pub id: String,
    pub state: PresentationSlotState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_priority: Option<PresentationPriority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_allocation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser_id: Option<String>,
    #[serde(default)]
    pub scene_generation: u64,
    #[serde(default)]
    pub restoration_pending: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cleanup_obligation_ids: Vec<String>,
}

impl PresentationSlot {
    pub fn warm_idle(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            state: PresentationSlotState::WarmIdle,
            lease_request_id: None,
            lease_priority: None,
            route_id: None,
            display_allocation_id: None,
            browser_id: None,
            scene_generation: 0,
            restoration_pending: false,
            cleanup_obligation_ids: Vec::new(),
        }
    }

    pub fn with_binding(
        mut self,
        route_id: impl Into<String>,
        display_allocation_id: impl Into<String>,
    ) -> Self {
        self.route_id = Some(route_id.into());
        self.display_allocation_id = Some(display_allocation_id.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationPriority {
    HumanControl,
    Recovery,
    ExistingEffect,
    Observation,
    Convenience,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationRequest {
    pub id: String,
    pub priority: PresentationPriority,
    pub browser_id: Option<String>,
    pub requires_staging: bool,
    pub queued_at: Option<u64>,
}

impl PresentationRequest {
    pub fn observation(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            priority: PresentationPriority::Observation,
            browser_id: None,
            requires_staging: false,
            queued_at: None,
        }
    }

    pub fn recovery(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            priority: PresentationPriority::Recovery,
            browser_id: None,
            requires_staging: false,
            queued_at: None,
        }
    }

    pub fn human(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            priority: PresentationPriority::HumanControl,
            browser_id: None,
            requires_staging: false,
            queued_at: None,
        }
    }

    pub fn for_browser(mut self, browser_id: impl Into<String>) -> Self {
        self.browser_id = Some(browser_id.into());
        self
    }

    pub fn requiring_staging(mut self) -> Self {
        self.requires_staging = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityLimitingResource {
    ReservedCapacity,
    PressureAdmission,
    WarmSlot,
    BrowserExclusion,
    HumanController,
    ViewerStagingConflict,
    AcquisitionLease,
    DurableHandoff,
    QueueBound,
    InvalidRequest,
    InventoryAdmission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityNextSafeAction {
    WaitForCapacity,
    RetryAfterQueueChange,
    RequestHumanTakeover,
    PreserveCurrentPresentation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CapacityDecision {
    Granted {
        request_id: String,
        slot_id: String,
    },
    Queued {
        request_id: String,
        queue_position: usize,
        limiting_resource: CapacityLimitingResource,
        next_safe_action: CapacityNextSafeAction,
    },
    Rejected {
        request_id: String,
        limiting_resource: CapacityLimitingResource,
        next_safe_action: CapacityNextSafeAction,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotTransitionReceipt {
    pub slot_id: String,
    pub request_id: String,
    pub previous_state: PresentationSlotState,
    pub state: PresentationSlotState,
    pub scene_generation: u64,
}

impl CapacityDecision {
    pub fn is_granted(&self) -> bool {
        matches!(self, Self::Granted { .. })
    }

    pub fn queue_position(&self) -> Option<usize> {
        match self {
            Self::Queued { queue_position, .. } => Some(*queue_position),
            Self::Granted { .. } | Self::Rejected { .. } => None,
        }
    }

    pub fn limiting_resource(&self) -> Option<CapacityLimitingResource> {
        match self {
            Self::Queued {
                limiting_resource, ..
            }
            | Self::Rejected {
                limiting_resource, ..
            } => Some(*limiting_resource),
            Self::Granted { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationCapacityConfig {
    pub warm_minimum: usize,
    pub hard_maximum: usize,
    pub human_priority_reserve: usize,
    pub recovery_reserve: usize,
    #[serde(default = "default_max_queue_depth")]
    pub max_queue_depth: usize,
}

const fn default_max_queue_depth() -> usize {
    64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressureAdmission {
    admitted_maximum: usize,
}

impl PressureAdmission {
    pub fn admit(admitted_maximum: usize) -> Self {
        Self { admitted_maximum }
    }

    pub fn admitted_maximum(self) -> usize {
        self.admitted_maximum
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationCapacityProjection {
    pub total_slots: usize,
    pub slot_ids: Vec<String>,
    pub configured_hard_maximum: usize,
    pub pressure_admitted_maximum: usize,
    pub slot_counts: BTreeMap<String, usize>,
    pub human_protected_capacity: usize,
    pub recovery_reserved_capacity: usize,
    pub queued_by_priority: BTreeMap<String, usize>,
    pub oldest_wait_ticks: Option<u64>,
    pub binding_warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PresentationCapacityAuthority {
    /// Failed inventory admission disables new presentation effects, not state reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    admission_error: Option<String>,
    config: PresentationCapacityConfig,
    slots: Vec<PresentationSlot>,
    queued_requests: Vec<PresentationRequest>,
    queue_clock: u64,
}

impl Default for PresentationCapacityAuthority {
    fn default() -> Self {
        Self {
            admission_error: None,
            config: PresentationCapacityConfig {
                warm_minimum: 0,
                hard_maximum: 0,
                human_priority_reserve: 0,
                recovery_reserve: 0,
                max_queue_depth: default_max_queue_depth(),
            },
            slots: Vec::new(),
            queued_requests: Vec::new(),
            queue_clock: 0,
        }
    }
}

impl PresentationCapacityAuthority {
    pub fn config(&self) -> &PresentationCapacityConfig {
        &self.config
    }

    pub fn slots(&self) -> &[PresentationSlot] {
        &self.slots
    }

    pub fn queued_requests(&self) -> &[PresentationRequest] {
        &self.queued_requests
    }

    pub fn queue_clock(&self) -> u64 {
        self.queue_clock
    }

    pub fn admission_error(&self) -> Option<&str> {
        self.admission_error.as_deref()
    }

    /// Refresh qualified inventory while retaining exact slot and acquisition
    /// custody. Fresh inventory is validated before restoration, then the merged
    /// slots are validated again. Queue history survives unchanged.
    pub fn from_refreshed_inventory(
        config: PresentationCapacityConfig,
        mut qualified_slots: Vec<PresentationSlot>,
        previous: Option<&Self>,
        retained_acquisitions: &[PresentationAcquisitionRetention],
    ) -> Result<Self, String> {
        qualified_slots.sort_by(|left, right| left.id.cmp(&right.id));
        let mut capacity = Self::new(config, qualified_slots)?;
        if let Some(previous) = previous {
            capacity.queued_requests = previous.queued_requests.clone();
            capacity.queue_clock = previous.queue_clock;
            for slot in &mut capacity.slots {
                if let Some(old) = previous.slots.iter().find(|old| {
                    old.id == slot.id
                        && old.route_id == slot.route_id
                        && old.display_allocation_id == slot.display_allocation_id
                }) {
                    *slot = old.clone();
                }
            }
            for old in &previous.slots {
                if capacity.slots.iter().any(|slot| slot.id == old.id) {
                    continue;
                }
                if retained_acquisitions.iter().any(|fact| {
                    old.id == fact.slot_id
                        && old.route_id.as_deref() == Some(fact.route_id.as_str())
                        && old.display_allocation_id.as_deref()
                            == Some(fact.display_allocation_id.as_str())
                        && old.browser_id.as_deref() == Some(fact.browser_id.as_str())
                }) {
                    capacity.slots.push(old.clone());
                }
            }
            capacity.slots.sort_by(|left, right| left.id.cmp(&right.id));
            capacity.slots = Self::new(config, std::mem::take(&mut capacity.slots))?.slots;
        }
        Ok(capacity)
    }

    /// Revalidate inventory without silently changing prior slot custody.
    /// Custody errors precede constructor validation; only exact pending
    /// acquisition and completed inert-owner facts permit browser changes.
    pub fn from_revalidated_inventory(
        config: PresentationCapacityConfig,
        mut qualified_slots: Vec<PresentationSlot>,
        previous: Option<&Self>,
        observations: &[PresentationInventoryCustodyObservation],
    ) -> Result<Self, String> {
        qualified_slots.sort_by(|left, right| left.id.cmp(&right.id));
        if let Some(previous) = previous {
            for old in &previous.slots {
                let Some(slot) = qualified_slots.iter_mut().find(|slot| {
                    slot.id == old.id
                        && slot.route_id == old.route_id
                        && slot.display_allocation_id == old.display_allocation_id
                }) else {
                    return Err(format!(
                        "production_presentation_inventory_capacity_custody_changed:{}",
                        serde_json::json!({"reason":"binding_changed","previousSlot":old})
                    ));
                };
                let facts = observations.iter().find(|fact| {
                    fact.slot_id == old.id
                        && fact.route_id == old.route_id
                        && fact.display_allocation_id == old.display_allocation_id
                });
                let exact_pending_binding = facts.is_some_and(|fact| fact.exact_pending_binding);
                let exact_pending_browser_acquisition = old.state
                    == PresentationSlotState::WarmIdle
                    && old.browser_id.is_none()
                    && old.lease_request_id.is_none()
                    && old.cleanup_obligation_ids.is_empty()
                    && slot.browser_id.is_some()
                    && exact_pending_binding;
                let exact_completed_inert_owner = old.state == PresentationSlotState::Active
                    && old.browser_id.is_some()
                    && slot.browser_id.is_none()
                    && facts.is_some_and(|fact| {
                        fact.completed_inert_owner_browser_id == old.browser_id
                    });
                if exact_pending_browser_acquisition || exact_completed_inert_owner {
                    continue;
                }
                if slot.browser_id != old.browser_id {
                    return Err(format!(
                        "production_presentation_inventory_capacity_custody_changed:{}",
                        serde_json::json!({
                            "reason":"browser_changed",
                            "exactPendingBinding":exact_pending_binding,
                            "previousSlot":old,
                            "qualifiedSlot":slot,
                        })
                    ));
                }
                *slot = old.clone();
            }
        }
        let mut capacity = Self::new(config, qualified_slots)?;
        if let Some(previous) = previous {
            capacity.queued_requests = previous.queued_requests.clone();
            capacity.queue_clock = previous.queue_clock;
        }
        Ok(capacity)
    }

    /// Fence new admission after inventory failure while preserving all custody.
    pub fn record_inventory_failure(&mut self, error: String) {
        self.admission_error = Some(error);
    }

    /// Admit and append one provisioning slot, preserving gate precedence.
    /// Lifecycle generation and provider effects remain adapter responsibilities.
    pub fn begin_provisioning(
        &mut self,
        slot_id: &str,
        pressure: PressureAdmission,
    ) -> Result<(), &'static str> {
        if self
            .slots
            .iter()
            .any(|slot| slot.state == PresentationSlotState::Provisioning)
        {
            return Err("provisioning_in_flight");
        }
        if self.slots.len() >= self.config.hard_maximum {
            return Err("configured_hard_maximum");
        }
        if self.slots.len() >= pressure.admitted_maximum() {
            return Err("pressure_admission");
        }
        let mut slot = PresentationSlot::warm_idle(slot_id);
        slot.state = PresentationSlotState::Provisioning;
        self.slots.push(slot);
        Ok(())
    }

    /// Reflect an adapter-validated provisioning result in its first exact slot.
    /// Panics if the adapter lost the admitted slot across its provider call.
    pub fn complete_provisioning(
        &mut self,
        slot_id: &str,
        route_id: String,
        display_allocation_id: String,
    ) {
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.id == slot_id)
            .expect("provisioning slot must remain present");
        slot.route_id = Some(route_id);
        slot.display_allocation_id = Some(display_allocation_id);
        slot.state = PresentationSlotState::WarmIdle;
    }

    /// Retain failed provisioning custody and append its exact cleanup obligation.
    /// Unlike episode quarantine, this preserves insertion order and duplicates.
    pub fn quarantine_failed_provisioning(
        &mut self,
        slot_id: &str,
        lifecycle_generation: &str,
    ) -> String {
        let obligation = format!("cleanup:{slot_id}:{lifecycle_generation}");
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.id == slot_id)
            .expect("failed provisioning slot must remain present");
        slot.state = PresentationSlotState::Quarantined;
        slot.cleanup_obligation_ids.push(obligation.clone());
        obligation
    }

    /// Begin cooldown after the adapter verifies elastic-resource ownership.
    pub fn begin_cooldown(&mut self, slot_id: &str) -> Result<(), String> {
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.id == slot_id)
            .ok_or_else(|| "presentation_slot_not_found".to_string())?;
        if slot.state != PresentationSlotState::WarmIdle {
            return Err("presentation_slot_not_idle".to_string());
        }
        slot.state = PresentationSlotState::Cooling;
        Ok(())
    }

    /// Remove every matching slot after the adapter proves resource reclamation.
    pub fn complete_reclamation(&mut self, slot_id: &str) {
        self.slots.retain(|slot| slot.id != slot_id);
    }

    /// Retain failed reclamation custody, deduplicating without reordering.
    /// A missing slot is deliberately a no-op, matching the lifecycle adapter.
    pub fn quarantine_failed_reclamation(&mut self, slot_id: &str) {
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.id == slot_id) {
            slot.state = PresentationSlotState::Quarantined;
            let obligation = format!("cleanup:{slot_id}:scale-in");
            if !slot.cleanup_obligation_ids.contains(&obligation) {
                slot.cleanup_obligation_ids.push(obligation);
            }
        }
    }

    /// Atomically park every slot of a retired browser, regardless of slot state.
    /// Any matching active lease rejects the whole operation before mutation.
    /// Route identities, scene history and cleanup obligations remain intact.
    pub fn retire_browser(
        &mut self,
        browser_id: &str,
    ) -> Result<(), PresentationRetirementConflict> {
        if self.slots.iter().any(|slot| {
            slot.browser_id.as_deref() == Some(browser_id) && slot.lease_request_id.is_some()
        }) {
            return Err(PresentationRetirementConflict::ActivePresentationLease);
        }
        for slot in &mut self.slots {
            if slot.browser_id.as_deref() == Some(browser_id) {
                slot.browser_id = None;
                slot.state = PresentationSlotState::WarmIdle;
                slot.lease_priority = None;
                slot.restoration_pending = false;
            }
        }
        Ok(())
    }

    fn browser_is_excluded(&self, request: &PresentationRequest) -> bool {
        request.browser_id.as_ref().is_some_and(|browser_id| {
            self.slots.iter().any(|slot| {
                slot.browser_id.as_deref() == Some(browser_id.as_str())
                    && slot.state != PresentationSlotState::WarmIdle
                    && slot.lease_request_id.as_deref() != Some(request.id.as_str())
            }) || self.queued_requests.iter().any(|queued| {
                queued.id != request.id && queued.browser_id.as_deref() == Some(browser_id.as_str())
            })
        })
    }

    fn protected_reserve(&self, priority: PresentationPriority) -> usize {
        match priority {
            PresentationPriority::HumanControl | PresentationPriority::ExistingEffect => 0,
            PresentationPriority::Recovery => self.config.human_priority_reserve,
            PresentationPriority::Observation | PresentationPriority::Convenience => {
                self.config.human_priority_reserve + self.config.recovery_reserve
            }
        }
    }

    pub fn new(
        config: PresentationCapacityConfig,
        slots: Vec<PresentationSlot>,
    ) -> Result<Self, String> {
        if config.warm_minimum > config.hard_maximum {
            return Err("presentation_warm_minimum_exceeds_hard_maximum".to_string());
        }
        if config.human_priority_reserve + config.recovery_reserve > config.hard_maximum {
            return Err("presentation_reserves_exceed_hard_maximum".to_string());
        }
        if slots.len() > config.hard_maximum {
            return Err("presentation_inventory_exceeds_hard_maximum".to_string());
        }
        let ids = slots
            .iter()
            .map(|slot| slot.id.as_str())
            .collect::<BTreeSet<_>>();
        if ids.len() != slots.len() || ids.contains("") {
            return Err("presentation_slot_identity_invalid".to_string());
        }
        for (field, values) in [
            (
                "route",
                slots
                    .iter()
                    .filter_map(|slot| slot.route_id.as_deref())
                    .collect::<Vec<_>>(),
            ),
            (
                "display",
                slots
                    .iter()
                    .filter_map(|slot| slot.display_allocation_id.as_deref())
                    .collect::<Vec<_>>(),
            ),
        ] {
            if values.iter().collect::<BTreeSet<_>>().len() != values.len() {
                return Err(format!("presentation_slot_{field}_identity_duplicate"));
            }
        }
        Ok(Self {
            admission_error: None,
            config,
            slots,
            queued_requests: Vec::new(),
            queue_clock: 0,
        })
    }

    pub fn projection(&self, pressure: PressureAdmission) -> PresentationCapacityProjection {
        self.projection_with_observations(pressure, None)
    }

    pub fn projection_with_observations(
        &self,
        pressure: PressureAdmission,
        observations: Option<&PresentationCapacityObservations>,
    ) -> PresentationCapacityProjection {
        let mut slot_counts = BTreeMap::new();
        for slot in &self.slots {
            *slot_counts
                .entry(slot_state_name(slot.state).to_string())
                .or_insert(0) += 1;
        }
        let mut queued_by_priority = BTreeMap::new();
        for request in &self.queued_requests {
            *queued_by_priority
                .entry(priority_name(request.priority).to_string())
                .or_insert(0) += 1;
        }
        PresentationCapacityProjection {
            total_slots: self.slots.len(),
            slot_ids: self.slots.iter().map(|slot| slot.id.clone()).collect(),
            configured_hard_maximum: self.config.hard_maximum,
            pressure_admitted_maximum: if self.admission_error.is_some() {
                0
            } else {
                pressure.admitted_maximum.min(self.config.hard_maximum)
            },
            slot_counts,
            human_protected_capacity: self.config.human_priority_reserve,
            recovery_reserved_capacity: self.config.recovery_reserve,
            queued_by_priority,
            oldest_wait_ticks: self
                .queued_requests
                .iter()
                .filter_map(|request| request.queued_at)
                .min()
                .map(|queued_at| self.queue_clock.saturating_sub(queued_at)),
            binding_warnings: observations
                .map(|state| self.binding_warnings(state))
                .unwrap_or_else(|| self.admission_error.iter().cloned().collect()),
        }
    }

    pub fn request(
        &mut self,
        request: PresentationRequest,
        pressure: PressureAdmission,
    ) -> CapacityDecision {
        self.request_with_observations(request, pressure, None)
    }

    pub fn request_with_observations(
        &mut self,
        request: PresentationRequest,
        pressure: PressureAdmission,
        observations: Option<&PresentationCapacityObservations>,
    ) -> CapacityDecision {
        if self.admission_error.is_some() {
            return CapacityDecision::Rejected {
                request_id: request.id,
                limiting_resource: CapacityLimitingResource::InventoryAdmission,
                next_safe_action: CapacityNextSafeAction::PreserveCurrentPresentation,
            };
        }
        self.queue_clock = self.queue_clock.saturating_add(1);
        if request.id.trim().is_empty()
            || self
                .queued_requests
                .iter()
                .any(|queued| queued.id == request.id)
            || self
                .slots
                .iter()
                .any(|slot| slot.lease_request_id.as_deref() == Some(request.id.as_str()))
        {
            return CapacityDecision::Rejected {
                request_id: request.id,
                limiting_resource: CapacityLimitingResource::InvalidRequest,
                next_safe_action: CapacityNextSafeAction::PreserveCurrentPresentation,
            };
        }
        if self.browser_is_excluded(&request) {
            return self.enqueue_or_reject(request, CapacityLimitingResource::BrowserExclusion);
        }
        let admitted_maximum = pressure.admitted_maximum.min(self.config.hard_maximum);
        let eligible_slots = self.slots.len().min(admitted_maximum);
        let candidates = self
            .slots
            .iter()
            .take(eligible_slots)
            .filter(|slot| slot.state == PresentationSlotState::WarmIdle)
            .map(|slot| {
                (
                    slot.id.clone(),
                    observations.and_then(|state| slot_admission_conflict(state, slot, &request)),
                )
            })
            .collect::<Vec<_>>();
        let free_slots = candidates
            .iter()
            .filter(|(_, conflict)| conflict.is_none())
            .count();
        let protected_reserve = self.protected_reserve(request.priority);
        if free_slots > protected_reserve {
            let selected_id = candidates
                .iter()
                .find(|(_, conflict)| conflict.is_none())
                .map(|(id, _)| id.clone())
                .expect("free slot count and candidate inventory must agree");
            let slot = self
                .slots
                .iter_mut()
                .find(|slot| slot.id == selected_id)
                .expect("free slot count and inventory must agree");
            slot.state = PresentationSlotState::Reserved;
            slot.lease_request_id = Some(request.id.clone());
            slot.lease_priority = Some(request.priority);
            slot.browser_id = request.browser_id.clone();
            return CapacityDecision::Granted {
                request_id: request.id,
                slot_id: slot.id.clone(),
            };
        }

        let limiting_resource = candidates
            .iter()
            .filter_map(|(_, conflict)| *conflict)
            .min()
            .unwrap_or({
                if eligible_slots < self.slots.len() {
                    CapacityLimitingResource::PressureAdmission
                } else if free_slots > 0 && protected_reserve > 0 {
                    CapacityLimitingResource::ReservedCapacity
                } else {
                    CapacityLimitingResource::WarmSlot
                }
            });
        self.enqueue_or_reject(request, limiting_resource)
    }

    /// Reserve the exact presentation already bound to an observation target.
    /// An active retained browser keeps its slot state and browser binding; the
    /// episode owns only the lease. A matching warm slot transitions to the
    /// normal reserved state and continues to honor protected capacity.
    pub fn request_bound_observation(
        &mut self,
        request: PresentationRequest,
        pressure: PressureAdmission,
        observations: &PresentationCapacityObservations,
        route_id: &str,
        display_allocation_id: &str,
    ) -> CapacityDecision {
        self.request_bound_presentation(
            request,
            PresentationPriority::Observation,
            pressure,
            observations,
            (route_id, display_allocation_id),
            false,
        )
    }

    /// Reserve the exact presentation being restored by a recovery operation.
    /// Recovery outranks observations and may reuse a durable handoff binding,
    /// but it remains subordinate to an active human controller.
    pub fn request_bound_recovery(
        &mut self,
        request: PresentationRequest,
        pressure: PressureAdmission,
        observations: &PresentationCapacityObservations,
        route_id: &str,
        display_allocation_id: &str,
    ) -> CapacityDecision {
        self.request_bound_presentation(
            request,
            PresentationPriority::Recovery,
            pressure,
            observations,
            (route_id, display_allocation_id),
            false,
        )
    }

    /// Reserve a route-switch destination before releasing either the moving
    /// browser's source slot or a browser currently occupying the destination.
    /// The destination keeps its current binding until the route-switch
    /// transaction parks it, then checkout transfers the leased slot.
    pub fn request_bound_route_switch_recovery(
        &mut self,
        request: PresentationRequest,
        pressure: PressureAdmission,
        observations: &PresentationCapacityObservations,
        route_id: &str,
        display_allocation_id: &str,
    ) -> CapacityDecision {
        self.request_bound_presentation(
            request,
            PresentationPriority::Recovery,
            pressure,
            observations,
            (route_id, display_allocation_id),
            true,
        )
    }

    fn request_bound_presentation(
        &mut self,
        request: PresentationRequest,
        expected_priority: PresentationPriority,
        pressure: PressureAdmission,
        observations: &PresentationCapacityObservations,
        binding: (&str, &str),
        route_switch: bool,
    ) -> CapacityDecision {
        let (route_id, display_allocation_id) = binding;
        if self.admission_error.is_some() {
            return CapacityDecision::Rejected {
                request_id: request.id,
                limiting_resource: CapacityLimitingResource::InventoryAdmission,
                next_safe_action: CapacityNextSafeAction::PreserveCurrentPresentation,
            };
        }
        self.queue_clock = self.queue_clock.saturating_add(1);
        let browser_id = request.browser_id.as_deref();
        if request.id.trim().is_empty()
            || request.priority != expected_priority
            || request.requires_staging
            || browser_id.is_none()
            || self
                .queued_requests
                .iter()
                .any(|queued| queued.id == request.id)
            || self
                .slots
                .iter()
                .any(|slot| slot.lease_request_id.as_deref() == Some(request.id.as_str()))
        {
            return CapacityDecision::Rejected {
                request_id: request.id,
                limiting_resource: CapacityLimitingResource::InvalidRequest,
                next_safe_action: CapacityNextSafeAction::PreserveCurrentPresentation,
            };
        }
        let browser_id = browser_id.expect("validated above");
        let Some(slot_index) = self.slots.iter().position(|slot| {
            slot.route_id.as_deref() == Some(route_id)
                && slot.display_allocation_id.as_deref() == Some(display_allocation_id)
        }) else {
            return self.enqueue_or_reject(request, CapacityLimitingResource::WarmSlot);
        };
        if slot_index >= pressure.admitted_maximum.min(self.config.hard_maximum) {
            return self.enqueue_or_reject(request, CapacityLimitingResource::PressureAdmission);
        }
        let source_slots = self
            .slots
            .iter()
            .enumerate()
            .filter(|(index, slot)| {
                *index != slot_index && slot.browser_id.as_deref() == Some(browser_id)
            })
            .collect::<Vec<_>>();
        let source_is_migratable = route_switch
            && source_slots.len() <= 1
            && source_slots.iter().all(|(_index, slot)| {
                slot.state == PresentationSlotState::Active && slot.lease_request_id.is_none()
            });
        if self.queued_requests.iter().any(|queued| {
            queued.browser_id.as_deref() == Some(browser_id) && queued.id != request.id
        }) || (!source_slots.is_empty() && !source_is_migratable)
        {
            return self.enqueue_or_reject(request, CapacityLimitingResource::BrowserExclusion);
        }
        let slot = &self.slots[slot_index];
        if slot.lease_request_id.is_some()
            || !matches!(
                slot.state,
                PresentationSlotState::WarmIdle | PresentationSlotState::Active
            )
            || (slot.state == PresentationSlotState::Active
                && slot.browser_id.as_deref() != Some(browser_id)
                && !route_switch)
        {
            return self.enqueue_or_reject(request, CapacityLimitingResource::WarmSlot);
        }
        if let Some(conflict) = slot_admission_conflict(observations, slot, &request) {
            return self.enqueue_or_reject(request, conflict);
        }
        if slot.state == PresentationSlotState::WarmIdle && !source_is_migratable {
            let admitted_maximum = pressure.admitted_maximum.min(self.config.hard_maximum);
            let free_slots = self
                .slots
                .iter()
                .take(admitted_maximum)
                .filter(|candidate| {
                    candidate.state == PresentationSlotState::WarmIdle
                        && slot_admission_conflict(observations, candidate, &request).is_none()
                })
                .count();
            if free_slots <= self.protected_reserve(request.priority) {
                return self.enqueue_or_reject(request, CapacityLimitingResource::ReservedCapacity);
            }
        }
        let slot = &mut self.slots[slot_index];
        if slot.state == PresentationSlotState::WarmIdle {
            slot.state = PresentationSlotState::Reserved;
            slot.browser_id = (!route_switch).then(|| browser_id.to_string());
        }
        slot.lease_request_id = Some(request.id.clone());
        slot.lease_priority = Some(request.priority);
        CapacityDecision::Granted {
            request_id: request.id,
            slot_id: slot.id.clone(),
        }
    }

    /// Release a bound presentation lease without parking an already-active
    /// retained browser. Warm-slot reservations continue through normal dispatch.
    pub fn release_bound_presentation(
        &mut self,
        slot_id: &str,
        request_id: &str,
        pressure: PressureAdmission,
        observations: &PresentationCapacityObservations,
    ) -> Result<Option<CapacityDecision>, String> {
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.id == slot_id)
            .ok_or_else(|| "presentation_reserved_slot_missing".to_string())?;
        if slot.lease_request_id.as_deref() != Some(request_id) {
            return Err("presentation_release_lease_mismatch".to_string());
        }
        if slot.state == PresentationSlotState::Active {
            slot.lease_request_id = None;
            slot.lease_priority = None;
            self.queue_clock = self.queue_clock.saturating_add(1);
            return Ok(None);
        }
        Ok(self.release_and_dispatch_with_observations(slot_id, pressure, Some(observations)))
    }

    fn enqueue_or_reject(
        &mut self,
        mut request: PresentationRequest,
        limiting_resource: CapacityLimitingResource,
    ) -> CapacityDecision {
        if self.queued_requests.len() >= self.config.max_queue_depth {
            return CapacityDecision::Rejected {
                request_id: request.id,
                limiting_resource: CapacityLimitingResource::QueueBound,
                next_safe_action: CapacityNextSafeAction::RetryAfterQueueChange,
            };
        }
        request.queued_at = Some(self.queue_clock);
        self.queued_requests.push(request.clone());
        let queue_position = queue_order(&self.queued_requests, self.queue_clock)
            .iter()
            .position(|index| self.queued_requests[*index].id == request.id)
            .map(|index| index + 1)
            .expect("queued request must have a queue position");
        CapacityDecision::Queued {
            request_id: request.id,
            queue_position,
            limiting_resource,
            next_safe_action: match limiting_resource {
                CapacityLimitingResource::HumanController => {
                    CapacityNextSafeAction::RequestHumanTakeover
                }
                _ => CapacityNextSafeAction::WaitForCapacity,
            },
        }
    }

    /// Return deterministic adapter warnings, including failed inventory admission.
    pub fn binding_warnings(&self, observations: &PresentationCapacityObservations) -> Vec<String> {
        let mut warnings = observations
            .binding_warnings
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        warnings.extend(self.admission_error.iter().cloned());
        warnings.into_iter().collect()
    }

    /// Reflect a service-authoritative route checkout in the matching slot.
    /// An in-flight bound recovery reservation is preserved while the slot
    /// becomes active for the retained browser.
    pub fn activate_bound_browser(
        &mut self,
        route_id: &str,
        display_allocation_id: &str,
        browser_id: &str,
    ) -> Result<(), String> {
        if let Some(error) = &self.admission_error {
            return Err(format!("presentation_inventory_not_admitted: {error}"));
        }
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| {
                slot.route_id.as_deref() == Some(route_id)
                    && slot.display_allocation_id.as_deref() == Some(display_allocation_id)
            })
            .ok_or_else(|| "presentation_bound_slot_missing".to_string())?;
        if !matches!(
            slot.state,
            PresentationSlotState::WarmIdle
                | PresentationSlotState::Reserved
                | PresentationSlotState::Active
        ) || slot
            .browser_id
            .as_deref()
            .is_some_and(|current| current != browser_id)
        {
            return Err("presentation_bound_slot_not_activatable".to_string());
        }
        slot.state = PresentationSlotState::Active;
        slot.browser_id = Some(browser_id.to_string());
        Ok(())
    }

    /// Reflect a service-authoritative route release without reclaiming the
    /// warm provider slot. Active episode and recovery leases fail closed.
    pub fn release_bound_browser(
        &mut self,
        route_id: &str,
        display_allocation_id: &str,
        browser_id: &str,
    ) -> Result<(), String> {
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| {
                slot.route_id.as_deref() == Some(route_id)
                    && slot.display_allocation_id.as_deref() == Some(display_allocation_id)
            })
            .ok_or_else(|| "presentation_bound_slot_missing".to_string())?;
        if slot.browser_id.as_deref() != Some(browser_id) {
            return Err("presentation_bound_slot_browser_mismatch".to_string());
        }
        if slot.lease_request_id.is_some() {
            return Err("presentation_bound_slot_lease_active".to_string());
        }
        if slot.state != PresentationSlotState::Active {
            return Err("presentation_bound_slot_not_releasable".to_string());
        }
        slot.state = PresentationSlotState::WarmIdle;
        slot.browser_id = None;
        slot.lease_priority = None;
        slot.restoration_pending = false;
        Ok(())
    }

    /// Park a route-switch occupant while retaining the destination recovery
    /// lease. Ordinary source-slot releases still follow the normal release
    /// path because they carry no recovery lease.
    pub fn release_bound_browser_for_route_switch(
        &mut self,
        route_id: &str,
        display_allocation_id: &str,
        browser_id: &str,
    ) -> Result<(), String> {
        let Some(slot) = self.slots.iter_mut().find(|slot| {
            slot.route_id.as_deref() == Some(route_id)
                && slot.display_allocation_id.as_deref() == Some(display_allocation_id)
        }) else {
            return Err("presentation_bound_slot_missing".to_string());
        };
        if slot.browser_id.as_deref() != Some(browser_id) {
            return Err("presentation_bound_slot_browser_mismatch".to_string());
        }
        if slot.lease_request_id.is_some() {
            if slot.lease_priority != Some(PresentationPriority::Recovery)
                || slot.state != PresentationSlotState::Active
            {
                return Err("presentation_bound_slot_lease_active".to_string());
            }
            slot.state = PresentationSlotState::Reserved;
            slot.browser_id = None;
            slot.restoration_pending = false;
            return Ok(());
        }
        self.release_bound_browser(route_id, display_allocation_id, browser_id)
    }

    /// Re-derive unleased warm and active slots from durable route and browser
    /// ownership. In-flight leases remain untouched so reconciliation cannot
    /// steal presentation authority from an active episode or recovery.
    pub fn reconcile_authoritative_bindings(
        &mut self,
        state: &PresentationCapacityObservations,
    ) -> usize {
        if self.admission_error.is_some() {
            return 0;
        }
        let mut repaired = 0;
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if slot.lease_request_id.is_some()
                || !matches!(
                    slot.state,
                    PresentationSlotState::WarmIdle | PresentationSlotState::Active
                )
            {
                continue;
            }

            let authoritative_browser_id = state
                .slots
                .get(index)
                .filter(|observed| observed.matches(slot))
                .expect("capacity reconciliation observations must match inventory order")
                .authoritative_browser_id
                .as_deref();

            match authoritative_browser_id {
                Some(browser_id)
                    if slot.state != PresentationSlotState::Active
                        || slot.browser_id.as_deref() != Some(browser_id) =>
                {
                    slot.state = PresentationSlotState::Active;
                    slot.browser_id = Some(browser_id.to_string());
                    slot.lease_priority = None;
                    slot.restoration_pending = false;
                    repaired += 1;
                }
                None if slot.state != PresentationSlotState::WarmIdle
                    || slot.browser_id.is_some()
                    || slot.lease_priority.is_some()
                    || slot.restoration_pending =>
                {
                    slot.state = PresentationSlotState::WarmIdle;
                    slot.browser_id = None;
                    slot.lease_priority = None;
                    slot.restoration_pending = false;
                    repaired += 1;
                }
                _ => {}
            }
        }
        repaired
    }

    pub fn transition_slot(
        &mut self,
        slot_id: &str,
        request_id: &str,
        next: PresentationSlotState,
        observations: Option<&PresentationCapacityObservations>,
    ) -> Result<SlotTransitionReceipt, String> {
        if let Some(error) = &self.admission_error {
            return Err(format!("presentation_inventory_not_admitted: {error}"));
        }
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.id == slot_id)
            .ok_or_else(|| "presentation_slot_not_found".to_string())?;
        if slot.lease_request_id.as_deref() != Some(request_id) {
            return Err("presentation_slot_lease_mismatch".to_string());
        }
        let allowed = matches!(
            (slot.state, next),
            (
                PresentationSlotState::Reserved,
                PresentationSlotState::Staging
            ) | (
                PresentationSlotState::Active,
                PresentationSlotState::Staging
            ) | (
                PresentationSlotState::Staging,
                PresentationSlotState::CaptureReady
            ) | (
                PresentationSlotState::Staging,
                PresentationSlotState::Restoring
            ) | (
                PresentationSlotState::CaptureReady,
                PresentationSlotState::Restoring
            ) | (
                PresentationSlotState::CaptureReady,
                PresentationSlotState::Active
            ) | (
                PresentationSlotState::Active,
                PresentationSlotState::Restoring
            ) | (
                PresentationSlotState::Restoring,
                PresentationSlotState::WarmIdle
            ) | (
                PresentationSlotState::Restoring,
                PresentationSlotState::Reserved
            ) | (
                PresentationSlotState::Restoring,
                PresentationSlotState::Active
            )
        );
        if !allowed {
            return Err("presentation_slot_transition_invalid".to_string());
        }
        if next == PresentationSlotState::Staging {
            let request = PresentationRequest {
                id: request_id.to_string(),
                priority: slot
                    .lease_priority
                    .unwrap_or(PresentationPriority::Observation),
                browser_id: slot.browser_id.clone(),
                requires_staging: true,
                queued_at: None,
            };
            if let Some(conflict) =
                observations.and_then(|state| slot_admission_conflict(state, slot, &request))
            {
                return Err(format!(
                    "presentation_slot_staging_blocked:{}",
                    limiting_resource_name(conflict)
                ));
            }
            slot.scene_generation = slot.scene_generation.saturating_add(1);
        }
        let previous_state = slot.state;
        slot.state = next;
        if matches!(
            next,
            PresentationSlotState::Staging | PresentationSlotState::Restoring
        ) {
            slot.restoration_pending = true;
        } else if matches!(
            next,
            PresentationSlotState::WarmIdle
                | PresentationSlotState::Reserved
                | PresentationSlotState::Active
        ) {
            slot.restoration_pending = false;
        }
        if next == PresentationSlotState::WarmIdle {
            slot.lease_request_id = None;
            slot.lease_priority = None;
            slot.browser_id = None;
        }
        Ok(SlotTransitionReceipt {
            slot_id: slot.id.clone(),
            request_id: request_id.to_string(),
            previous_state,
            state: next,
            scene_generation: slot.scene_generation,
        })
    }

    pub fn quarantine_slot(
        &mut self,
        slot_id: &str,
        cleanup_obligation_id: impl Into<String>,
    ) -> Result<(), String> {
        let obligation = cleanup_obligation_id.into();
        if obligation.trim().is_empty() {
            return Err("presentation_cleanup_obligation_invalid".to_string());
        }
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.id == slot_id)
            .ok_or_else(|| "presentation_slot_not_found".to_string())?;
        slot.state = PresentationSlotState::Quarantined;
        if !slot.cleanup_obligation_ids.contains(&obligation) {
            slot.cleanup_obligation_ids.push(obligation);
            slot.cleanup_obligation_ids.sort();
        }
        Ok(())
    }

    pub fn release_and_dispatch(
        &mut self,
        slot_id: &str,
        pressure: PressureAdmission,
    ) -> Option<CapacityDecision> {
        self.release_and_dispatch_with_observations(slot_id, pressure, None)
    }

    pub fn release_and_dispatch_with_observations(
        &mut self,
        slot_id: &str,
        pressure: PressureAdmission,
        observations: Option<&PresentationCapacityObservations>,
    ) -> Option<CapacityDecision> {
        self.queue_clock = self.queue_clock.saturating_add(1);
        let slot = self.slots.iter_mut().find(|slot| slot.id == slot_id)?;
        slot.state = PresentationSlotState::WarmIdle;
        slot.lease_request_id = None;
        slot.lease_priority = None;
        slot.browser_id = None;

        if self.admission_error.is_some() {
            return None;
        }
        let admitted_maximum = pressure.admitted_maximum.min(self.config.hard_maximum);
        let eligible_slots = self.slots.len().min(admitted_maximum);
        let free_slots = self
            .slots
            .iter()
            .take(eligible_slots)
            .filter(|slot| slot.state == PresentationSlotState::WarmIdle)
            .count();
        let queue_index = self
            .queued_requests
            .iter()
            .enumerate()
            .filter(|(_, request)| {
                !self.browser_is_excluded(request)
                    && free_slots > self.protected_reserve(request.priority)
                    && self.slots.iter().take(eligible_slots).any(|slot| {
                        slot.state == PresentationSlotState::WarmIdle
                            && observations
                                .and_then(|state| slot_admission_conflict(state, slot, request))
                                .is_none()
                    })
            })
            .min_by_key(|(index, request)| {
                (
                    effective_priority_rank(request, self.queue_clock),
                    request.queued_at.unwrap_or(u64::MAX),
                    *index,
                )
            })
            .map(|(index, _)| index)?;
        let request = self.queued_requests.remove(queue_index);
        let selected_id = self
            .slots
            .iter()
            .take(eligible_slots)
            .find(|slot| {
                slot.state == PresentationSlotState::WarmIdle
                    && observations
                        .and_then(|state| slot_admission_conflict(state, slot, &request))
                        .is_none()
            })?
            .id
            .clone();
        let slot = self.slots.iter_mut().find(|slot| slot.id == selected_id)?;
        slot.state = PresentationSlotState::Reserved;
        slot.lease_request_id = Some(request.id.clone());
        slot.lease_priority = Some(request.priority);
        slot.browser_id = request.browser_id.clone();
        Some(CapacityDecision::Granted {
            request_id: request.id,
            slot_id: slot.id.clone(),
        })
    }

    #[cfg(test)]
    fn advance_queue_clock(&mut self, ticks: u64) {
        self.queue_clock = self.queue_clock.saturating_add(ticks);
    }
}

fn effective_priority_rank(request: &PresentationRequest, now: u64) -> u8 {
    let base = match request.priority {
        PresentationPriority::HumanControl => 0,
        PresentationPriority::Recovery => 1,
        PresentationPriority::ExistingEffect => 2,
        PresentationPriority::Observation => 3,
        PresentationPriority::Convenience => 4,
    };
    if base == 0 {
        return 0;
    }
    let age = now.saturating_sub(request.queued_at.unwrap_or(now));
    let bounded_boost = (age / 4).min(u64::from(base - 1)) as u8;
    base - bounded_boost
}

fn queue_order(requests: &[PresentationRequest], now: u64) -> Vec<usize> {
    let mut indices = (0..requests.len()).collect::<Vec<_>>();
    indices.sort_by_key(|index| {
        let request = &requests[*index];
        (
            effective_priority_rank(request, now),
            request.queued_at.unwrap_or(u64::MAX),
            *index,
        )
    });
    indices
}

fn slot_admission_conflict(
    observations: &PresentationCapacityObservations,
    slot: &PresentationSlot,
    request: &PresentationRequest,
) -> Option<CapacityLimitingResource> {
    let observed = observations
        .slots
        .iter()
        .find(|observed| observed.matches(slot))
        .expect("capacity admission observations must cover the selected inventory");
    if observed.acquisition_lease_active {
        return Some(CapacityLimitingResource::AcquisitionLease);
    }
    if observed.human_controller_active
        && !matches!(
            request.priority,
            PresentationPriority::HumanControl | PresentationPriority::ExistingEffect
        )
    {
        return Some(CapacityLimitingResource::HumanController);
    }
    if request.requires_staging && observed.non_controller_staging_viewer_active {
        return Some(CapacityLimitingResource::ViewerStagingConflict);
    }
    if request.priority != PresentationPriority::Recovery
        && observed
            .live_handoff_browser_ids
            .iter()
            .any(|browser_id| Some(browser_id.as_str()) != request.browser_id.as_deref())
    {
        return Some(CapacityLimitingResource::DurableHandoff);
    }
    None
}

fn slot_state_name(state: PresentationSlotState) -> &'static str {
    match state {
        PresentationSlotState::Absent => "absent",
        PresentationSlotState::Provisioning => "provisioning",
        PresentationSlotState::WarmIdle => "warm_idle",
        PresentationSlotState::Reserved => "reserved",
        PresentationSlotState::Staging => "staging",
        PresentationSlotState::CaptureReady => "capture_ready",
        PresentationSlotState::Active => "active",
        PresentationSlotState::Restoring => "restoring",
        PresentationSlotState::Cooling => "cooling",
        PresentationSlotState::Reclaiming => "reclaiming",
        PresentationSlotState::Quarantined => "quarantined",
    }
}

fn priority_name(priority: PresentationPriority) -> &'static str {
    match priority {
        PresentationPriority::HumanControl => "human_control",
        PresentationPriority::Recovery => "recovery",
        PresentationPriority::ExistingEffect => "existing_effect",
        PresentationPriority::Observation => "observation",
        PresentationPriority::Convenience => "convenience",
    }
}

fn limiting_resource_name(resource: CapacityLimitingResource) -> &'static str {
    match resource {
        CapacityLimitingResource::ReservedCapacity => "reserved_capacity",
        CapacityLimitingResource::PressureAdmission => "pressure_admission",
        CapacityLimitingResource::WarmSlot => "warm_slot",
        CapacityLimitingResource::BrowserExclusion => "browser_exclusion",
        CapacityLimitingResource::HumanController => "human_controller",
        CapacityLimitingResource::ViewerStagingConflict => "viewer_staging_conflict",
        CapacityLimitingResource::AcquisitionLease => "acquisition_lease",
        CapacityLimitingResource::DurableHandoff => "durable_handoff",
        CapacityLimitingResource::QueueBound => "queue_bound",
        CapacityLimitingResource::InvalidRequest => "invalid_request",
        CapacityLimitingResource::InventoryAdmission => "inventory_admission",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refreshed_inventory_retains_exact_custody_and_queue_without_retaining_failure() {
        let mut previous = one_slot_authority(8);
        previous.config.hard_maximum = 3;
        previous.slots[0].state = PresentationSlotState::Quarantined;
        previous.slots[0].scene_generation = 17;
        previous.slots[0].cleanup_obligation_ids = vec!["z".into(), "a".into()];
        let mut pending =
            PresentationSlot::warm_idle("slot-0").with_binding("route-0", "display-0");
        pending.browser_id = Some("browser".into());
        pending.state = PresentationSlotState::Active;
        previous.slots.push(pending.clone());
        previous.slots.push(PresentationSlot::warm_idle("dropped"));
        previous.queue_clock = 29;
        previous.queued_requests = vec![PresentationRequest::observation("queued")];
        previous.record_inventory_failure("outage".into());
        let facts = vec![PresentationAcquisitionRetention {
            slot_id: pending.id.clone(),
            route_id: "route-0".into(),
            display_allocation_id: "display-0".into(),
            browser_id: "browser".into(),
        }];
        let config = PresentationCapacityConfig {
            max_queue_depth: 0,
            ..previous.config
        };
        let fresh =
            vec![PresentationSlot::warm_idle("slot-1").with_binding("route-1", "display-1")];
        let refreshed = PresentationCapacityAuthority::from_refreshed_inventory(
            config,
            fresh.clone(),
            Some(&previous),
            &facts,
        )
        .unwrap();
        assert_eq!(refreshed.slots(), &[pending, previous.slots[0].clone()]);
        assert_eq!(refreshed.config(), &config);
        assert_eq!(refreshed.queued_requests(), previous.queued_requests());
        assert_eq!(refreshed.queue_clock(), 29);
        assert_eq!(refreshed.admission_error(), None);
        for field in 0..4 {
            let mut wrong = facts.clone();
            match field {
                0 => wrong[0].slot_id.push('x'),
                1 => wrong[0].route_id.push('x'),
                2 => wrong[0].display_allocation_id.push('x'),
                _ => wrong[0].browser_id.push('x'),
            }
            let refreshed = PresentationCapacityAuthority::from_refreshed_inventory(
                config,
                fresh.clone(),
                Some(&previous),
                &wrong,
            )
            .unwrap();
            assert_eq!(refreshed.slots(), &previous.slots[..1]);
        }
        let replaced_binding =
            PresentationSlot::warm_idle("slot-0").with_binding("new-route", "new-display");
        let refreshed = PresentationCapacityAuthority::from_refreshed_inventory(
            config,
            vec![replaced_binding.clone()],
            Some(&previous),
            &facts,
        )
        .unwrap();
        assert_eq!(refreshed.slots(), &[replaced_binding]);
    }

    #[test]
    fn inventory_constructors_preserve_distinct_validation_order_and_second_refresh_validation() {
        let mut previous = one_slot_authority(8);
        previous.slots[0].browser_id = Some("browser".into());
        let invalid = PresentationCapacityConfig {
            warm_minimum: 2,
            ..previous.config
        };
        assert_eq!(
            PresentationCapacityAuthority::from_refreshed_inventory(
                invalid,
                Vec::new(),
                Some(&previous),
                &[],
            )
            .unwrap_err(),
            "presentation_warm_minimum_exceeds_hard_maximum"
        );
        assert_eq!(
            PresentationCapacityAuthority::from_revalidated_inventory(
                invalid,
                Vec::new(),
                Some(&previous),
                &[],
            )
            .unwrap_err(),
            format!(
                "production_presentation_inventory_capacity_custody_changed:{}",
                serde_json::json!({"reason":"binding_changed","previousSlot":previous.slots[0]})
            )
        );
        let facts = [PresentationAcquisitionRetention {
            slot_id: "slot-1".into(),
            route_id: "route-1".into(),
            display_allocation_id: "display-1".into(),
            browser_id: "browser".into(),
        }];
        assert_eq!(
            PresentationCapacityAuthority::from_refreshed_inventory(
                previous.config,
                vec![PresentationSlot::warm_idle("other")],
                Some(&previous),
                &facts,
            )
            .unwrap_err(),
            "presentation_inventory_exceeds_hard_maximum"
        );
        let config = PresentationCapacityConfig {
            hard_maximum: 2,
            ..previous.config
        };
        assert_eq!(
            PresentationCapacityAuthority::from_refreshed_inventory(
                config,
                vec![PresentationSlot::warm_idle("other").with_binding("route-1", "other-display")],
                Some(&previous),
                &facts,
            )
            .unwrap_err(),
            "presentation_slot_route_identity_duplicate"
        );
        previous.slots[0].browser_id = None;
        assert!(PresentationCapacityAuthority::from_refreshed_inventory(
            config,
            Vec::new(),
            Some(&previous),
            &facts,
        )
        .unwrap()
        .slots()
        .is_empty());
    }

    #[test]
    fn revalidated_inventory_preserves_exact_custody_error_and_pending_exception_fences() {
        let previous = one_slot_authority(8);
        let mut qualified = previous.slots[0].clone();
        qualified.browser_id = Some("acquiring".into());
        qualified.state = PresentationSlotState::Active;
        let fact = PresentationInventoryCustodyObservation {
            slot_id: "slot-1".into(),
            route_id: Some("route-1".into()),
            display_allocation_id: Some("display-1".into()),
            exact_pending_binding: true,
            completed_inert_owner_browser_id: None,
        };
        assert_eq!(
            PresentationCapacityAuthority::from_revalidated_inventory(
                previous.config,
                vec![qualified.clone()],
                Some(&previous),
                &[],
            )
            .unwrap_err(),
            format!(
                "production_presentation_inventory_capacity_custody_changed:{}",
                serde_json::json!({"reason":"browser_changed","exactPendingBinding":false,
                "previousSlot":previous.slots[0],"qualifiedSlot":qualified})
            )
        );
        let result = PresentationCapacityAuthority::from_revalidated_inventory(
            previous.config,
            vec![qualified.clone()],
            Some(&previous),
            std::slice::from_ref(&fact),
        )
        .unwrap();
        assert_eq!(result.slots(), &[qualified.clone()]);
        for fence in 0..6 {
            let mut old = previous.clone();
            let mut observed = fact.clone();
            match fence {
                0 => old.slots[0].state = PresentationSlotState::Active,
                1 => old.slots[0].lease_request_id = Some("lease".into()),
                2 => old.slots[0].cleanup_obligation_ids.push("cleanup".into()),
                3 => observed.slot_id.push('x'),
                4 => observed.route_id = None,
                _ => observed.display_allocation_id = None,
            }
            assert!(PresentationCapacityAuthority::from_revalidated_inventory(
                old.config,
                vec![qualified.clone()],
                Some(&old),
                &[observed],
            )
            .unwrap_err()
            .contains("browser_changed"));
        }
        // Metadata not fenced by the existing pending-acquisition exception is
        // deliberately discarded together with the old warm slot.
        let mut old = previous.clone();
        old.slots[0].lease_priority = Some(PresentationPriority::Recovery);
        old.slots[0].restoration_pending = true;
        old.slots[0].scene_generation = 9;
        assert_eq!(
            PresentationCapacityAuthority::from_revalidated_inventory(
                old.config,
                vec![qualified.clone()],
                Some(&old),
                &[fact],
            )
            .unwrap()
            .slots(),
            &[qualified]
        );
    }

    #[test]
    fn revalidated_inventory_inert_exception_and_unchanged_custody_preserve_queue() {
        let mut previous = one_slot_authority(8);
        previous.slots[0].state = PresentationSlotState::Active;
        previous.slots[0].browser_id = Some("retired".into());
        previous.slots[0].lease_request_id = Some("lease".into());
        previous.slots[0].cleanup_obligation_ids = vec!["cleanup".into()];
        previous.queue_clock = 41;
        previous.queued_requests = vec![PresentationRequest::human("queued")];
        previous.record_inventory_failure("outage".into());
        let fresh = PresentationSlot::warm_idle("slot-1").with_binding("route-1", "display-1");
        let mut fact = PresentationInventoryCustodyObservation {
            slot_id: "slot-1".into(),
            route_id: Some("route-1".into()),
            display_allocation_id: Some("display-1".into()),
            exact_pending_binding: false,
            completed_inert_owner_browser_id: Some("retired".into()),
        };
        let recovered = PresentationCapacityAuthority::from_revalidated_inventory(
            previous.config,
            vec![fresh.clone()],
            Some(&previous),
            std::slice::from_ref(&fact),
        )
        .unwrap();
        assert_eq!(recovered.slots(), &[fresh.clone()]);
        assert_eq!(recovered.queue_clock(), 41);
        assert_eq!(recovered.queued_requests(), previous.queued_requests());
        assert_eq!(recovered.admission_error(), None);
        fact.completed_inert_owner_browser_id = Some("foreign".into());
        assert!(PresentationCapacityAuthority::from_revalidated_inventory(
            previous.config,
            vec![fresh.clone()],
            Some(&previous),
            &[fact],
        )
        .unwrap_err()
        .contains("browser_changed"));
        let mut same_browser = fresh;
        same_browser.browser_id = Some("retired".into());
        let restored = PresentationCapacityAuthority::from_revalidated_inventory(
            previous.config,
            vec![same_browser],
            Some(&previous),
            &[],
        )
        .unwrap();
        assert_eq!(restored.slots(), previous.slots());
    }

    #[test]
    fn lifecycle_mutations_preserve_admission_order_and_exact_cleanup_semantics() {
        let mut capacity = one_slot_authority(8);
        let before = capacity.clone();
        assert_eq!(
            capacity.begin_provisioning("new", PressureAdmission::admit(0)),
            Err("configured_hard_maximum")
        );
        assert_eq!(capacity, before);
        capacity.config.hard_maximum = 3;
        let before = capacity.clone();
        assert_eq!(
            capacity.begin_provisioning("new", PressureAdmission::admit(1)),
            Err("pressure_admission")
        );
        assert_eq!(capacity, before);
        capacity.record_inventory_failure("outage".into());
        // Inventory fencing was not a scale-out gate and must not become one.
        capacity
            .begin_provisioning("new", PressureAdmission::admit(3))
            .unwrap();
        assert_eq!(
            capacity.begin_provisioning("another", PressureAdmission::admit(0)),
            Err("provisioning_in_flight")
        );
        capacity.complete_provisioning("new", "route-new".into(), "display-new".into());
        assert_eq!(
            capacity.slots()[1],
            PresentationSlot::warm_idle("new").with_binding("route-new", "display-new")
        );
        assert_eq!(
            capacity.begin_cooldown("missing").unwrap_err(),
            "presentation_slot_not_found"
        );
        capacity.begin_cooldown("new").unwrap();
        assert_eq!(
            capacity.begin_cooldown("new").unwrap_err(),
            "presentation_slot_not_idle"
        );
        capacity.slots[1].cleanup_obligation_ids.push("z".into());
        for _ in 0..2 {
            assert_eq!(
                capacity.quarantine_failed_provisioning("new", "g"),
                "cleanup:new:g"
            );
            capacity.quarantine_failed_reclamation("new");
        }
        assert_eq!(
            capacity.slots()[1].cleanup_obligation_ids,
            [
                "z",
                "cleanup:new:g",
                "cleanup:new:scale-in",
                "cleanup:new:g"
            ]
        );
        let before = capacity.clone();
        capacity.quarantine_failed_reclamation("missing");
        assert_eq!(capacity, before);
        capacity.slots.push(capacity.slots[1].clone());
        capacity.complete_reclamation("new");
        assert_eq!(capacity.slots(), &before.slots[..1]);
        // Permissive decoded duplicate identities keep the original first-match
        // completion and all-match reclamation semantics.
        capacity
            .begin_provisioning("slot-1", PressureAdmission::admit(3))
            .unwrap();
        capacity.complete_provisioning("slot-1", "first".into(), "first".into());
        assert_eq!(capacity.slots()[0].route_id.as_deref(), Some("first"));
        assert_eq!(
            capacity.slots()[1].state,
            PresentationSlotState::Provisioning
        );
    }

    #[test]
    #[should_panic(expected = "provisioning slot must remain present")]
    fn provisioning_completion_preserves_missing_slot_panic() {
        one_slot_authority(8).complete_provisioning("missing", "route".into(), "display".into());
    }

    #[test]
    #[should_panic(expected = "failed provisioning slot must remain present")]
    fn provisioning_quarantine_preserves_missing_slot_panic() {
        one_slot_authority(8).quarantine_failed_provisioning("missing", "generation");
    }

    #[test]
    fn retirement_is_atomic_across_all_states_and_preserves_unrelated_custody() {
        let mut capacity = one_slot_authority(8);
        let states = [
            PresentationSlotState::Absent,
            PresentationSlotState::Provisioning,
            PresentationSlotState::WarmIdle,
            PresentationSlotState::Reserved,
            PresentationSlotState::Staging,
            PresentationSlotState::CaptureReady,
            PresentationSlotState::Active,
            PresentationSlotState::Restoring,
            PresentationSlotState::Cooling,
            PresentationSlotState::Reclaiming,
            PresentationSlotState::Quarantined,
        ];
        for (index, state) in states.into_iter().enumerate() {
            let mut slot = PresentationSlot::warm_idle(format!("retired-{index}"))
                .with_binding(format!("route-{index}"), format!("display-{index}"));
            slot.state = state;
            slot.browser_id = Some("browser".into());
            slot.scene_generation = 12;
            slot.restoration_pending = true;
            slot.cleanup_obligation_ids = vec!["z".into(), "a".into()];
            slot.lease_priority = Some(PresentationPriority::Recovery);
            capacity.slots.push(slot);
        }
        capacity.slots.last_mut().unwrap().lease_request_id = Some("lease".into());
        capacity.queue_clock = 19;
        capacity
            .queued_requests
            .push(PresentationRequest::observation("queued"));
        let before_failure = capacity.clone();
        capacity.record_inventory_failure("outage".into());
        let mut expected_failure = before_failure;
        expected_failure.admission_error = Some("outage".into());
        assert_eq!(capacity, expected_failure);
        assert_eq!(
            capacity.retire_browser("browser"),
            Err(PresentationRetirementConflict::ActivePresentationLease)
        );
        assert_eq!(capacity, expected_failure);
        capacity.slots.last_mut().unwrap().lease_request_id = None;
        let mut expected = capacity.clone();
        for slot in &mut expected.slots[1..] {
            slot.browser_id = None;
            slot.state = PresentationSlotState::WarmIdle;
            slot.lease_priority = None;
            slot.restoration_pending = false;
        }
        capacity.retire_browser("browser").unwrap();
        assert_eq!(capacity, expected);
        capacity.retire_browser("missing").unwrap();
        assert_eq!(capacity, expected);
    }

    #[test]
    fn projection_preserves_zero_one_two_four_six_and_eight_slots() {
        for count in [0, 1, 2, 4, 6, 8] {
            let slots = (0..count)
                .map(|index| PresentationSlot::warm_idle(format!("slot-{index}")))
                .collect::<Vec<_>>();
            let authority = PresentationCapacityAuthority::new(
                PresentationCapacityConfig {
                    warm_minimum: count.min(4),
                    hard_maximum: count,
                    human_priority_reserve: usize::from(count > 0),
                    recovery_reserve: usize::from(count > 1),
                    max_queue_depth: 8,
                },
                slots,
            )
            .expect("fixture capacity should be valid");

            let projection = authority.projection(PressureAdmission::admit(count));
            assert_eq!(projection.total_slots, count);
            assert_eq!(projection.slot_ids.len(), count);
            assert_eq!(projection.configured_hard_maximum, count);
            assert_eq!(projection.pressure_admitted_maximum, count);
        }
    }

    #[test]
    fn four_slot_profile_preserves_human_and_recovery_reserves() {
        let mut authority = PresentationCapacityAuthority::new(
            PresentationCapacityConfig {
                warm_minimum: 4,
                hard_maximum: 6,
                human_priority_reserve: 1,
                recovery_reserve: 1,
                max_queue_depth: 8,
            },
            (0..4)
                .map(|index| PresentationSlot::warm_idle(format!("slot-{index}")))
                .collect(),
        )
        .unwrap();
        let pressure = PressureAdmission::admit(6);

        assert!(authority
            .request(PresentationRequest::observation("observe-1"), pressure)
            .is_granted());
        assert!(authority
            .request(PresentationRequest::observation("observe-2"), pressure)
            .is_granted());
        let queued = authority.request(PresentationRequest::observation("observe-3"), pressure);
        assert_eq!(queued.queue_position(), Some(1));
        assert_eq!(
            queued.limiting_resource(),
            Some(CapacityLimitingResource::ReservedCapacity)
        );
        assert!(authority
            .request(PresentationRequest::recovery("recover-1"), pressure)
            .is_granted());
        assert!(authority
            .request(PresentationRequest::human("human-1"), pressure)
            .is_granted());
    }

    #[test]
    fn queued_agents_are_dispatched_fifo_within_one_priority_class() {
        let mut authority = PresentationCapacityAuthority::new(
            PresentationCapacityConfig {
                warm_minimum: 1,
                hard_maximum: 1,
                human_priority_reserve: 0,
                recovery_reserve: 0,
                max_queue_depth: 8,
            },
            vec![PresentationSlot::warm_idle("slot-0")],
        )
        .unwrap();
        let pressure = PressureAdmission::admit(1);

        assert!(authority
            .request(PresentationRequest::observation("observe-1"), pressure)
            .is_granted());
        assert_eq!(
            authority
                .request(PresentationRequest::observation("observe-2"), pressure)
                .queue_position(),
            Some(1)
        );
        assert_eq!(
            authority
                .request(PresentationRequest::observation("observe-3"), pressure)
                .queue_position(),
            Some(2)
        );

        assert_eq!(
            authority.release_and_dispatch("slot-0", pressure),
            Some(CapacityDecision::Granted {
                request_id: "observe-2".to_string(),
                slot_id: "slot-0".to_string(),
            })
        );
    }

    #[test]
    fn host_pressure_rejection_is_typed_capacity_not_browser_failure() {
        let mut authority = PresentationCapacityAuthority::new(
            PresentationCapacityConfig {
                warm_minimum: 2,
                hard_maximum: 4,
                human_priority_reserve: 0,
                recovery_reserve: 0,
                max_queue_depth: 8,
            },
            vec![
                PresentationSlot::warm_idle("slot-0"),
                PresentationSlot::warm_idle("slot-1"),
            ],
        )
        .unwrap();

        let decision = authority.request(
            PresentationRequest::observation("observe-pressure"),
            PressureAdmission::admit(0),
        );

        assert_eq!(
            decision.limiting_resource(),
            Some(CapacityLimitingResource::PressureAdmission)
        );
    }

    #[test]
    fn bounded_aging_prevents_observation_starvation_without_outranking_humans() {
        let mut authority = PresentationCapacityAuthority::new(
            PresentationCapacityConfig {
                warm_minimum: 1,
                hard_maximum: 1,
                human_priority_reserve: 0,
                recovery_reserve: 0,
                max_queue_depth: 8,
            },
            vec![PresentationSlot::warm_idle("slot-0")],
        )
        .unwrap();
        let pressure = PressureAdmission::admit(1);

        authority.request(PresentationRequest::observation("holder"), pressure);
        authority.request(
            PresentationRequest::observation("aged-observation"),
            pressure,
        );
        authority.advance_queue_clock(8);
        authority.request(PresentationRequest::recovery("new-recovery"), pressure);
        authority.request(PresentationRequest::human("new-human"), pressure);

        assert_eq!(
            authority.release_and_dispatch("slot-0", pressure),
            Some(CapacityDecision::Granted {
                request_id: "new-human".to_string(),
                slot_id: "slot-0".to_string(),
            })
        );
        assert_eq!(
            authority.release_and_dispatch("slot-0", pressure),
            Some(CapacityDecision::Granted {
                request_id: "aged-observation".to_string(),
                slot_id: "slot-0".to_string(),
            })
        );
    }

    fn one_slot_authority(max_queue_depth: usize) -> PresentationCapacityAuthority {
        PresentationCapacityAuthority::new(
            PresentationCapacityConfig {
                warm_minimum: 1,
                hard_maximum: 1,
                human_priority_reserve: 0,
                recovery_reserve: 0,
                max_queue_depth,
            },
            vec![PresentationSlot::warm_idle("slot-1").with_binding("route-1", "display-1")],
        )
        .unwrap()
    }

    #[test]
    fn queue_is_bounded_and_one_browser_cannot_hold_two_presentations() {
        let mut authority = PresentationCapacityAuthority::new(
            PresentationCapacityConfig {
                warm_minimum: 2,
                hard_maximum: 2,
                human_priority_reserve: 0,
                recovery_reserve: 0,
                max_queue_depth: 1,
            },
            vec![
                PresentationSlot::warm_idle("slot-1"),
                PresentationSlot::warm_idle("slot-2"),
            ],
        )
        .unwrap();
        let pressure = PressureAdmission::admit(2);
        assert!(authority
            .request(
                PresentationRequest::observation("first").for_browser("browser-1"),
                pressure,
            )
            .is_granted());
        assert_eq!(
            authority
                .request(
                    PresentationRequest::observation("second").for_browser("browser-1"),
                    pressure,
                )
                .limiting_resource(),
            Some(CapacityLimitingResource::BrowserExclusion)
        );
        assert_eq!(
            authority
                .request(
                    PresentationRequest::observation("third").for_browser("browser-1"),
                    pressure,
                )
                .limiting_resource(),
            Some(CapacityLimitingResource::QueueBound)
        );
        assert_eq!(
            authority.release_and_dispatch("slot-2", pressure),
            None,
            "releasing another slot must not bypass per-browser exclusion"
        );
        assert_eq!(
            authority.release_and_dispatch("slot-1", pressure),
            Some(CapacityDecision::Granted {
                request_id: "second".to_string(),
                slot_id: "slot-1".to_string(),
            })
        );
    }

    #[test]
    fn slot_authority_fences_scene_transitions_and_quarantines_uncertain_cleanup() {
        let mut authority = one_slot_authority(8);
        assert!(authority
            .request(
                PresentationRequest::observation("episode-1").for_browser("browser-1"),
                PressureAdmission::admit(1),
            )
            .is_granted());
        assert_eq!(
            authority
                .transition_slot(
                    "slot-1",
                    "wrong-episode",
                    PresentationSlotState::Staging,
                    None,
                )
                .unwrap_err(),
            "presentation_slot_lease_mismatch"
        );
        let staging = authority
            .transition_slot("slot-1", "episode-1", PresentationSlotState::Staging, None)
            .unwrap();
        assert_eq!(staging.scene_generation, 1);
        authority
            .transition_slot(
                "slot-1",
                "episode-1",
                PresentationSlotState::CaptureReady,
                None,
            )
            .unwrap();
        authority
            .transition_slot("slot-1", "episode-1", PresentationSlotState::Active, None)
            .unwrap();
        authority
            .quarantine_slot("slot-1", "cleanup:episode-1")
            .unwrap();
        assert_eq!(authority.slots[0].state, PresentationSlotState::Quarantined);
        assert_eq!(
            authority.slots[0].cleanup_obligation_ids,
            vec!["cleanup:episode-1"]
        );
    }

    fn observations(authority: &PresentationCapacityAuthority) -> PresentationCapacityObservations {
        PresentationCapacityObservations {
            slots: authority
                .slots
                .iter()
                .map(|slot| PresentationSlotObservation {
                    slot_id: slot.id.clone(),
                    route_id: slot.route_id.clone(),
                    display_allocation_id: slot.display_allocation_id.clone(),
                    ..Default::default()
                })
                .collect(),
            binding_warnings: Vec::new(),
        }
    }

    #[test]
    fn constructor_error_order_and_identity_validation_are_preserved() {
        let good = one_slot_authority(8);
        let mut config = good.config;
        config.warm_minimum = 2;
        config.human_priority_reserve = 2;
        assert_eq!(
            PresentationCapacityAuthority::new(config, vec![]).unwrap_err(),
            "presentation_warm_minimum_exceeds_hard_maximum"
        );
        config.warm_minimum = 0;
        assert_eq!(
            PresentationCapacityAuthority::new(config, vec![]).unwrap_err(),
            "presentation_reserves_exceed_hard_maximum"
        );
        config.human_priority_reserve = 0;
        let duplicate = vec![good.slots[0].clone(); 2];
        assert_eq!(
            PresentationCapacityAuthority::new(config, duplicate.clone()).unwrap_err(),
            "presentation_inventory_exceeds_hard_maximum"
        );
        config.hard_maximum = 2;
        assert_eq!(
            PresentationCapacityAuthority::new(config, duplicate).unwrap_err(),
            "presentation_slot_identity_invalid"
        );
        let mut slots = vec![good.slots[0].clone(); 2];
        slots[1].id = "slot-2".into();
        assert_eq!(
            PresentationCapacityAuthority::new(config, slots.clone()).unwrap_err(),
            "presentation_slot_route_identity_duplicate"
        );
        slots[1].route_id = Some("route-2".into());
        assert_eq!(
            PresentationCapacityAuthority::new(config, slots).unwrap_err(),
            "presentation_slot_display_identity_duplicate"
        );
        assert_eq!(
            PresentationCapacityAuthority::new(config, vec![PresentationSlot::warm_idle("")])
                .unwrap_err(),
            "presentation_slot_identity_invalid"
        );
    }

    #[test]
    fn serde_defaults_omissions_and_permissive_decode_remain_compatible() {
        let decoded: PresentationCapacityAuthority = serde_json::from_str("{}").unwrap();
        assert_eq!(decoded, PresentationCapacityAuthority::default());
        assert_eq!(decoded.config.max_queue_depth, 64);
        let value = serde_json::to_value(one_slot_authority(8)).unwrap();
        assert!(value.get("admissionError").is_none());
        assert_eq!(
            value["slots"][0],
            serde_json::json!({
                "id": "slot-1", "state": "warm_idle",
                "routeId": "route-1", "displayAllocationId": "display-1",
                "sceneGeneration": 0, "restorationPending": false
            })
        );
        let mut value = value;
        value["config"]
            .as_object_mut()
            .unwrap()
            .remove("maxQueueDepth");
        value["config"]["hardMaximum"] = serde_json::json!(0);
        let decoded: PresentationCapacityAuthority = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.config.max_queue_depth, 64);
        assert_eq!(
            decoded.slots.len(),
            1,
            "decode does not run constructor validation"
        );
        let request = serde_json::to_value(PresentationRequest::observation("request")).unwrap();
        assert!(request["browserId"].is_null());
        assert!(request["queuedAt"].is_null());
        assert_eq!(
            serde_json::to_value(CapacityDecision::Granted {
                request_id: "r".into(),
                slot_id: "s".into()
            })
            .unwrap(),
            serde_json::json!({"state":"granted","request_id":"r","slot_id":"s"})
        );
    }

    #[test]
    fn request_and_release_keep_clock_and_inventory_error_order() {
        let mut authority = one_slot_authority(8);
        let pressure = PressureAdmission::admit(1);
        assert_eq!(
            authority
                .request(PresentationRequest::observation(" "), pressure)
                .limiting_resource(),
            Some(CapacityLimitingResource::InvalidRequest)
        );
        assert_eq!(authority.queue_clock, 1);
        authority.admission_error = Some("inventory-error".into());
        assert_eq!(
            authority
                .request(PresentationRequest::observation(" "), pressure)
                .limiting_resource(),
            Some(CapacityLimitingResource::InventoryAdmission)
        );
        assert_eq!(authority.queue_clock, 1);
        assert_eq!(authority.release_and_dispatch("missing", pressure), None);
        assert_eq!(authority.queue_clock, 2);
        authority.slots[0].state = PresentationSlotState::Active;
        authority.slots[0].browser_id = Some("browser".into());
        authority.slots[0].restoration_pending = true;
        authority.slots[0]
            .cleanup_obligation_ids
            .push("cleanup".into());
        assert_eq!(authority.release_and_dispatch("slot-1", pressure), None);
        assert_eq!(authority.queue_clock, 3);
        assert_eq!(authority.slots[0].state, PresentationSlotState::WarmIdle);
        assert_eq!(authority.slots[0].browser_id, None);
        assert!(authority.slots[0].restoration_pending);
        assert_eq!(authority.slots[0].cleanup_obligation_ids, ["cleanup"]);
        assert_eq!(authority.projection(pressure).pressure_admitted_maximum, 0);
        authority.quarantine_slot("slot-1", "cleanup-2").unwrap();
        assert_eq!(authority.slots[0].state, PresentationSlotState::Quarantined);
    }

    #[test]
    fn observation_conflict_precedence_and_priority_exemptions_are_model_policy() {
        let authority = one_slot_authority(8);
        let mut facts = observations(&authority);
        let observed = &mut facts.slots[0];
        observed.acquisition_lease_active = true;
        observed.human_controller_active = true;
        observed.non_controller_staging_viewer_active = true;
        observed.live_handoff_browser_ids.insert("incumbent".into());
        let request = PresentationRequest::observation("r")
            .for_browser("new")
            .requiring_staging();
        let conflict = |facts: &PresentationCapacityObservations, request: &PresentationRequest| {
            let mut candidate = authority.clone();
            candidate
                .request_with_observations(
                    request.clone(),
                    PressureAdmission::admit(1),
                    Some(facts),
                )
                .limiting_resource()
        };
        assert_eq!(
            conflict(&facts, &request),
            Some(CapacityLimitingResource::AcquisitionLease)
        );
        facts.slots[0].acquisition_lease_active = false;
        assert_eq!(
            conflict(&facts, &request),
            Some(CapacityLimitingResource::HumanController)
        );
        let mut human = request.clone();
        human.priority = PresentationPriority::HumanControl;
        assert_eq!(
            conflict(&facts, &human),
            Some(CapacityLimitingResource::ViewerStagingConflict)
        );
        human.priority = PresentationPriority::ExistingEffect;
        assert_eq!(
            conflict(&facts, &human),
            Some(CapacityLimitingResource::ViewerStagingConflict)
        );
        facts.slots[0].human_controller_active = false;
        assert_eq!(
            conflict(&facts, &request),
            Some(CapacityLimitingResource::ViewerStagingConflict)
        );
        facts.slots[0].non_controller_staging_viewer_active = false;
        assert_eq!(
            conflict(&facts, &request),
            Some(CapacityLimitingResource::DurableHandoff)
        );
        assert_eq!(conflict(&facts, &PresentationRequest::recovery("r")), None);
        assert_eq!(
            conflict(
                &facts,
                &PresentationRequest::observation("r").for_browser("incumbent")
            ),
            None
        );
    }

    #[test]
    fn admission_counts_unblocked_slots_but_dispatch_counts_all_warm_slots() {
        let mut authority = one_slot_authority(8);
        authority.config.hard_maximum = 2;
        authority.config.human_priority_reserve = 1;
        authority
            .slots
            .push(PresentationSlot::warm_idle("slot-2").with_binding("route-2", "display-2"));
        let mut facts = observations(&authority);
        facts.slots[0].human_controller_active = true;
        let pressure = PressureAdmission::admit(2);
        let queued = authority.request_with_observations(
            PresentationRequest::observation("r"),
            pressure,
            Some(&facts),
        );
        assert_eq!(
            queued.limiting_resource(),
            Some(CapacityLimitingResource::HumanController)
        );
        assert_eq!(
            authority.release_and_dispatch_with_observations("slot-2", pressure, Some(&facts)),
            Some(CapacityDecision::Granted {
                request_id: "r".into(),
                slot_id: "slot-2".into()
            })
        );
    }

    #[test]
    fn cross_slot_conflicts_use_enum_order_and_pressure_uses_inventory_prefix() {
        let mut authority = one_slot_authority(8);
        authority.config.hard_maximum = 2;
        authority
            .slots
            .push(PresentationSlot::warm_idle("slot-0").with_binding("route-0", "display-0"));
        let mut facts = observations(&authority);
        facts.slots[0].acquisition_lease_active = true;
        facts.slots[1].human_controller_active = true;
        assert_eq!(
            authority
                .request_with_observations(
                    PresentationRequest::observation("r"),
                    PressureAdmission::admit(2),
                    Some(&facts)
                )
                .limiting_resource(),
            Some(CapacityLimitingResource::HumanController)
        );
        facts.slots[1].human_controller_active = false;
        assert_eq!(
            authority
                .request_with_observations(
                    PresentationRequest::observation("r2"),
                    PressureAdmission::admit(1),
                    Some(&facts)
                )
                .limiting_resource(),
            Some(CapacityLimitingResource::AcquisitionLease)
        );
        facts.slots[0].acquisition_lease_active = false;
        assert_eq!(
            authority.request_with_observations(
                PresentationRequest::human("r3"),
                PressureAdmission::admit(2),
                Some(&facts)
            ),
            CapacityDecision::Granted {
                request_id: "r3".into(),
                slot_id: "slot-1".into()
            }
        );
    }

    #[test]
    fn late_controller_blocks_staging_without_advancing_scene_generation() {
        let mut authority = one_slot_authority(8);
        authority.request(
            PresentationRequest::observation("r"),
            PressureAdmission::admit(1),
        );
        let mut facts = observations(&authority);
        facts.slots[0].human_controller_active = true;
        assert_eq!(
            authority
                .transition_slot("slot-1", "r", PresentationSlotState::Staging, Some(&facts))
                .unwrap_err(),
            "presentation_slot_staging_blocked:human_controller"
        );
        assert_eq!(authority.slots[0].state, PresentationSlotState::Reserved);
        assert_eq!(authority.slots[0].scene_generation, 0);
    }

    #[test]
    fn transition_graph_and_restoration_metadata_are_exact() {
        use PresentationSlotState::*;
        let states = [
            Absent,
            Provisioning,
            WarmIdle,
            Reserved,
            Staging,
            CaptureReady,
            Active,
            Restoring,
            Cooling,
            Reclaiming,
            Quarantined,
        ];
        let edges = [
            (Reserved, Staging),
            (Active, Staging),
            (Staging, CaptureReady),
            (Staging, Restoring),
            (CaptureReady, Restoring),
            (CaptureReady, Active),
            (Active, Restoring),
            (Restoring, WarmIdle),
            (Restoring, Reserved),
            (Restoring, Active),
        ];
        for from in states {
            for to in states {
                let mut authority = one_slot_authority(8);
                let slot = &mut authority.slots[0];
                slot.state = from;
                slot.lease_request_id = Some("r".into());
                slot.lease_priority = Some(PresentationPriority::Observation);
                slot.browser_id = Some("b".into());
                let before = authority.clone();
                let result = authority.transition_slot("slot-1", "r", to, None);
                if edges.contains(&(from, to)) {
                    let receipt = result.unwrap();
                    assert_eq!((receipt.previous_state, receipt.state), (from, to));
                    assert_eq!(receipt.scene_generation, u64::from(to == Staging));
                    assert_eq!(
                        authority.slots[0].restoration_pending,
                        matches!(to, Staging | Restoring)
                    );
                    if to == WarmIdle {
                        assert_eq!(authority.slots[0].lease_request_id, None);
                        assert_eq!(authority.slots[0].browser_id, None);
                    }
                } else {
                    assert_eq!(result.unwrap_err(), "presentation_slot_transition_invalid");
                    assert_eq!(authority, before);
                }
            }
        }
    }

    #[test]
    fn bound_active_release_preserves_browser_and_does_not_dispatch() {
        let mut authority = one_slot_authority(8);
        authority
            .activate_bound_browser("route-1", "display-1", "browser")
            .unwrap();
        let facts = observations(&authority);
        let pressure = PressureAdmission::admit(1);
        assert!(authority
            .request_bound_observation(
                PresentationRequest::observation("lease").for_browser("browser"),
                pressure,
                &facts,
                "route-1",
                "display-1"
            )
            .is_granted());
        authority.request(PresentationRequest::observation("queued"), pressure);
        let clock = authority.queue_clock;
        assert_eq!(
            authority
                .release_bound_presentation("slot-1", "wrong", pressure, &facts)
                .unwrap_err(),
            "presentation_release_lease_mismatch"
        );
        assert_eq!(
            authority
                .release_bound_presentation("missing", "lease", pressure, &facts)
                .unwrap_err(),
            "presentation_reserved_slot_missing"
        );
        assert_eq!(
            authority
                .release_bound_presentation("slot-1", "lease", pressure, &facts)
                .unwrap(),
            None
        );
        assert_eq!(authority.queue_clock, clock + 1);
        assert_eq!(authority.slots[0].state, PresentationSlotState::Active);
        assert_eq!(authority.slots[0].browser_id.as_deref(), Some("browser"));
        assert_eq!(authority.queued_requests.len(), 1);
    }

    #[test]
    fn route_switch_retains_destination_lease_until_checkout() {
        let mut authority = one_slot_authority(8);
        authority.config.hard_maximum = 2;
        authority
            .slots
            .push(PresentationSlot::warm_idle("destination").with_binding("route-2", "display-2"));
        authority
            .activate_bound_browser("route-1", "display-1", "moving")
            .unwrap();
        authority
            .activate_bound_browser("route-2", "display-2", "incumbent")
            .unwrap();
        let facts = observations(&authority);
        let decision = authority.request_bound_route_switch_recovery(
            PresentationRequest::recovery("recovery").for_browser("moving"),
            PressureAdmission::admit(2),
            &facts,
            "route-2",
            "display-2",
        );
        assert!(decision.is_granted());
        assert_eq!(authority.slots[1].browser_id.as_deref(), Some("incumbent"));
        assert_eq!(
            authority
                .release_bound_browser("route-2", "display-2", "incumbent")
                .unwrap_err(),
            "presentation_bound_slot_lease_active"
        );
        authority
            .release_bound_browser_for_route_switch("route-2", "display-2", "incumbent")
            .unwrap();
        assert_eq!(authority.slots[1].state, PresentationSlotState::Reserved);
        assert_eq!(
            authority.slots[1].lease_request_id.as_deref(),
            Some("recovery")
        );
        authority
            .release_bound_browser("route-1", "display-1", "moving")
            .unwrap();
        authority
            .activate_bound_browser("route-2", "display-2", "moving")
            .unwrap();
        assert_eq!(authority.slots[1].state, PresentationSlotState::Active);
        assert_eq!(
            authority.slots[1].lease_request_id.as_deref(),
            Some("recovery")
        );
    }

    #[test]
    fn reconciliation_only_repairs_unleased_warm_or_active_slots() {
        let mut authority = one_slot_authority(8);
        let mut facts = observations(&authority);
        facts.slots[0].authoritative_browser_id = Some("browser".into());
        assert_eq!(authority.reconcile_authoritative_bindings(&facts), 1);
        assert_eq!(authority.reconcile_authoritative_bindings(&facts), 0);
        assert_eq!(authority.slots[0].browser_id.as_deref(), Some("browser"));
        authority.slots[0].lease_request_id = Some("lease".into());
        facts.slots[0].authoritative_browser_id = None;
        assert_eq!(authority.reconcile_authoritative_bindings(&facts), 0);
        authority.slots[0].lease_request_id = None;
        authority.quarantine_slot("slot-1", "cleanup").unwrap();
        assert_eq!(authority.reconcile_authoritative_bindings(&facts), 0);
        authority.slots[0].state = PresentationSlotState::Active;
        assert_eq!(authority.reconcile_authoritative_bindings(&facts), 1);
        assert_eq!(authority.slots[0].state, PresentationSlotState::WarmIdle);
        authority.admission_error = Some("invalid".into());
        facts.slots[0].authoritative_browser_id = Some("browser".into());
        assert_eq!(authority.reconcile_authoritative_bindings(&facts), 0);
    }

    #[test]
    fn warning_and_quarantine_ordering_are_deduplicated() {
        let mut authority = one_slot_authority(8);
        let mut facts = observations(&authority);
        facts.binding_warnings = vec!["z".into(), "a".into(), "a".into()];
        authority.admission_error = Some("m".into());
        assert_eq!(
            authority
                .projection_with_observations(PressureAdmission::admit(1), Some(&facts))
                .binding_warnings,
            ["a", "m", "z"]
        );
        assert_eq!(
            authority.quarantine_slot("missing", " ").unwrap_err(),
            "presentation_cleanup_obligation_invalid"
        );
        assert_eq!(
            authority.quarantine_slot("missing", "c").unwrap_err(),
            "presentation_slot_not_found"
        );
        for obligation in ["z", "a", "z"] {
            authority.quarantine_slot("slot-1", obligation).unwrap();
        }
        assert_eq!(authority.slots[0].cleanup_obligation_ids, ["a", "z"]);
    }
}
