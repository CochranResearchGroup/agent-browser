//! Arbitrary-N presentation slot inventory, admission, and queue authority.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::service_model::ServiceState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PresentationSlotState {
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
pub(crate) struct PresentationSlot {
    pub(crate) id: String,
    pub(crate) state: PresentationSlotState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) lease_request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) lease_priority: Option<PresentationPriority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) route_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) display_allocation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) browser_id: Option<String>,
    #[serde(default)]
    pub(crate) scene_generation: u64,
    #[serde(default)]
    pub(crate) restoration_pending: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) cleanup_obligation_ids: Vec<String>,
}

impl PresentationSlot {
    pub(crate) fn warm_idle(id: impl Into<String>) -> Self {
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

    pub(crate) fn with_binding(
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
pub(crate) enum PresentationPriority {
    HumanControl,
    Recovery,
    ExistingEffect,
    Observation,
    Convenience,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresentationRequest {
    pub(crate) id: String,
    pub(crate) priority: PresentationPriority,
    pub(crate) browser_id: Option<String>,
    pub(crate) requires_staging: bool,
    pub(crate) queued_at: Option<u64>,
}

impl PresentationRequest {
    pub(crate) fn observation(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            priority: PresentationPriority::Observation,
            browser_id: None,
            requires_staging: false,
            queued_at: None,
        }
    }

    pub(crate) fn recovery(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            priority: PresentationPriority::Recovery,
            browser_id: None,
            requires_staging: false,
            queued_at: None,
        }
    }

    pub(crate) fn human(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            priority: PresentationPriority::HumanControl,
            browser_id: None,
            requires_staging: false,
            queued_at: None,
        }
    }

    pub(crate) fn for_browser(mut self, browser_id: impl Into<String>) -> Self {
        self.browser_id = Some(browser_id.into());
        self
    }

    pub(crate) fn requiring_staging(mut self) -> Self {
        self.requires_staging = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapacityLimitingResource {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapacityNextSafeAction {
    WaitForCapacity,
    RetryAfterQueueChange,
    RequestHumanTakeover,
    PreserveCurrentPresentation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum CapacityDecision {
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
pub(crate) struct SlotTransitionReceipt {
    pub(crate) slot_id: String,
    pub(crate) request_id: String,
    pub(crate) previous_state: PresentationSlotState,
    pub(crate) state: PresentationSlotState,
    pub(crate) scene_generation: u64,
}

impl CapacityDecision {
    pub(crate) fn is_granted(&self) -> bool {
        matches!(self, Self::Granted { .. })
    }

    pub(crate) fn queue_position(&self) -> Option<usize> {
        match self {
            Self::Queued { queue_position, .. } => Some(*queue_position),
            Self::Granted { .. } | Self::Rejected { .. } => None,
        }
    }

    pub(crate) fn limiting_resource(&self) -> Option<CapacityLimitingResource> {
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
pub(crate) struct PresentationCapacityConfig {
    pub(crate) warm_minimum: usize,
    pub(crate) hard_maximum: usize,
    pub(crate) human_priority_reserve: usize,
    pub(crate) recovery_reserve: usize,
    #[serde(default = "default_max_queue_depth")]
    pub(crate) max_queue_depth: usize,
}

const fn default_max_queue_depth() -> usize {
    64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PressureAdmission {
    admitted_maximum: usize,
}

impl PressureAdmission {
    pub(crate) fn admit(admitted_maximum: usize) -> Self {
        Self { admitted_maximum }
    }

    pub(crate) fn admitted_maximum(self) -> usize {
        self.admitted_maximum
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresentationCapacityProjection {
    pub(crate) total_slots: usize,
    pub(crate) slot_ids: Vec<String>,
    pub(crate) configured_hard_maximum: usize,
    pub(crate) pressure_admitted_maximum: usize,
    pub(crate) slot_counts: BTreeMap<String, usize>,
    pub(crate) human_protected_capacity: usize,
    pub(crate) recovery_reserved_capacity: usize,
    pub(crate) queued_by_priority: BTreeMap<String, usize>,
    pub(crate) oldest_wait_ticks: Option<u64>,
    pub(crate) binding_warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct PresentationCapacityAuthority {
    pub(crate) config: PresentationCapacityConfig,
    pub(crate) slots: Vec<PresentationSlot>,
    pub(crate) queued_requests: Vec<PresentationRequest>,
    pub(crate) queue_clock: u64,
}

impl Default for PresentationCapacityAuthority {
    fn default() -> Self {
        Self {
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

    pub(crate) fn new(
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
            config,
            slots,
            queued_requests: Vec::new(),
            queue_clock: 0,
        })
    }

    /// Builds durable slot candidates only from service-authoritative ready
    /// route, display, and pool records. Missing capacity remains missing.
    pub(crate) fn from_service_state(
        config: PresentationCapacityConfig,
        state: &ServiceState,
    ) -> Result<Self, String> {
        let mut slots = state
            .route_pool
            .values()
            .filter_map(|entry| {
                let route_id = entry.current_route_allocation_id.as_deref()?;
                let route = state.remote_view_routes.get(route_id)?;
                let display_id = route.display_allocation_id.as_deref()?;
                let display = state.display_allocations.get(display_id)?;
                let pool_ready = matches!(entry.state.as_str(), "available" | "checked_out")
                    && entry
                        .readiness
                        .as_ref()
                        .and_then(|value| value.get("state"))
                        .and_then(serde_json::Value::as_str)
                        == Some("ready");
                let route_ready = route.state == "ready";
                let display_ready = matches!(display.state.as_str(), "ready" | "active");
                (pool_ready && route_ready && display_ready).then(|| {
                    let mut slot = PresentationSlot::warm_idle(format!("slot:{}", entry.id))
                        .with_binding(route.id.clone(), display.id.clone());
                    slot.browser_id = route.browser_id.clone();
                    if slot.browser_id.is_some() {
                        slot.state = PresentationSlotState::Active;
                    }
                    slot
                })
            })
            .collect::<Vec<_>>();
        slots.sort_by(|left, right| left.id.cmp(&right.id));
        Self::new(config, slots)
    }

    pub(crate) fn projection(&self, pressure: PressureAdmission) -> PresentationCapacityProjection {
        self.projection_with_service_state(pressure, None)
    }

    pub(crate) fn projection_with_service_state(
        &self,
        pressure: PressureAdmission,
        service_state: Option<&ServiceState>,
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
            pressure_admitted_maximum: pressure.admitted_maximum.min(self.config.hard_maximum),
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
            binding_warnings: service_state
                .map(|state| self.binding_warnings(state))
                .unwrap_or_default(),
        }
    }

    pub(crate) fn request(
        &mut self,
        request: PresentationRequest,
        pressure: PressureAdmission,
    ) -> CapacityDecision {
        self.request_with_service_state(request, pressure, None)
    }

    pub(crate) fn request_with_service_state(
        &mut self,
        request: PresentationRequest,
        pressure: PressureAdmission,
        service_state: Option<&ServiceState>,
    ) -> CapacityDecision {
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
                    service_state.and_then(|state| slot_admission_conflict(state, slot, &request)),
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
    pub(crate) fn request_bound_observation(
        &mut self,
        request: PresentationRequest,
        pressure: PressureAdmission,
        service_state: &ServiceState,
        route_id: &str,
        display_allocation_id: &str,
    ) -> CapacityDecision {
        self.request_bound_presentation(
            request,
            PresentationPriority::Observation,
            pressure,
            service_state,
            route_id,
            display_allocation_id,
        )
    }

    /// Reserve the exact presentation being restored by a recovery operation.
    /// Recovery outranks observations and may reuse a durable handoff binding,
    /// but it remains subordinate to an active human controller.
    pub(crate) fn request_bound_recovery(
        &mut self,
        request: PresentationRequest,
        pressure: PressureAdmission,
        service_state: &ServiceState,
        route_id: &str,
        display_allocation_id: &str,
    ) -> CapacityDecision {
        self.request_bound_presentation(
            request,
            PresentationPriority::Recovery,
            pressure,
            service_state,
            route_id,
            display_allocation_id,
        )
    }

    fn request_bound_presentation(
        &mut self,
        request: PresentationRequest,
        expected_priority: PresentationPriority,
        pressure: PressureAdmission,
        service_state: &ServiceState,
        route_id: &str,
        display_allocation_id: &str,
    ) -> CapacityDecision {
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
        if self.queued_requests.iter().any(|queued| {
            queued.browser_id.as_deref() == Some(browser_id) && queued.id != request.id
        }) || self.slots.iter().enumerate().any(|(index, slot)| {
            index != slot_index
                && slot.browser_id.as_deref() == Some(browser_id)
                && slot.state != PresentationSlotState::WarmIdle
        }) {
            return self.enqueue_or_reject(request, CapacityLimitingResource::BrowserExclusion);
        }
        let slot = &self.slots[slot_index];
        if slot.lease_request_id.is_some()
            || !matches!(
                slot.state,
                PresentationSlotState::WarmIdle | PresentationSlotState::Active
            )
            || (slot.state == PresentationSlotState::Active
                && slot.browser_id.as_deref() != Some(browser_id))
        {
            return self.enqueue_or_reject(request, CapacityLimitingResource::WarmSlot);
        }
        if let Some(conflict) = slot_admission_conflict(service_state, slot, &request) {
            return self.enqueue_or_reject(request, conflict);
        }
        if slot.state == PresentationSlotState::WarmIdle {
            let admitted_maximum = pressure.admitted_maximum.min(self.config.hard_maximum);
            let free_slots = self
                .slots
                .iter()
                .take(admitted_maximum)
                .filter(|candidate| {
                    candidate.state == PresentationSlotState::WarmIdle
                        && slot_admission_conflict(service_state, candidate, &request).is_none()
                })
                .count();
            if free_slots <= self.protected_reserve(request.priority) {
                return self.enqueue_or_reject(request, CapacityLimitingResource::ReservedCapacity);
            }
        }
        let slot = &mut self.slots[slot_index];
        if slot.state == PresentationSlotState::WarmIdle {
            slot.state = PresentationSlotState::Reserved;
            slot.browser_id = Some(browser_id.to_string());
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
    pub(crate) fn release_bound_presentation(
        &mut self,
        slot_id: &str,
        request_id: &str,
        pressure: PressureAdmission,
        service_state: &ServiceState,
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
        Ok(self.release_and_dispatch_with_service_state(slot_id, pressure, Some(service_state)))
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

    pub(crate) fn binding_warnings(&self, state: &ServiceState) -> Vec<String> {
        let mut warnings = BTreeSet::new();
        for slot in &self.slots {
            if let Some(route_id) = slot.route_id.as_deref() {
                if !state.remote_view_routes.contains_key(route_id)
                    && !state
                        .route_pool
                        .values()
                        .any(|entry| entry.route_id == route_id)
                {
                    warnings.insert(format!("slot_route_missing:{}:{}", slot.id, route_id));
                }
            }
            if let Some(display_id) = slot.display_allocation_id.as_deref() {
                if !state.display_allocations.contains_key(display_id) {
                    warnings.insert(format!("slot_display_missing:{}:{}", slot.id, display_id));
                }
            }
            if let Some(browser_id) = slot.browser_id.as_deref() {
                if !state.browsers.contains_key(browser_id) {
                    warnings.insert(format!("slot_browser_missing:{}:{}", slot.id, browser_id));
                }
            }
        }
        warnings.into_iter().collect()
    }

    pub(crate) fn transition_slot(
        &mut self,
        slot_id: &str,
        request_id: &str,
        next: PresentationSlotState,
        service_state: Option<&ServiceState>,
    ) -> Result<SlotTransitionReceipt, String> {
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
                service_state.and_then(|state| slot_admission_conflict(state, slot, &request))
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

    pub(crate) fn quarantine_slot(
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

    pub(crate) fn release_and_dispatch(
        &mut self,
        slot_id: &str,
        pressure: PressureAdmission,
    ) -> Option<CapacityDecision> {
        self.release_and_dispatch_with_service_state(slot_id, pressure, None)
    }

    pub(crate) fn release_and_dispatch_with_service_state(
        &mut self,
        slot_id: &str,
        pressure: PressureAdmission,
        service_state: Option<&ServiceState>,
    ) -> Option<CapacityDecision> {
        self.queue_clock = self.queue_clock.saturating_add(1);
        let slot = self.slots.iter_mut().find(|slot| slot.id == slot_id)?;
        slot.state = PresentationSlotState::WarmIdle;
        slot.lease_request_id = None;
        slot.lease_priority = None;
        slot.browser_id = None;

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
                            && service_state
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
                    && service_state
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
    state: &ServiceState,
    slot: &PresentationSlot,
    request: &PresentationRequest,
) -> Option<CapacityLimitingResource> {
    let route_id = slot.route_id.as_deref();
    let display_id = slot.display_allocation_id.as_deref();
    if state.remote_view_acquisition_leases.values().any(|lease| {
        matches!(lease.state.as_str(), "pending" | "active" | "rolling_back")
            && (route_id == Some(lease.route_id.as_str())
                || display_id == Some(lease.display_allocation_id.as_str()))
    }) {
        return Some(CapacityLimitingResource::AcquisitionLease);
    }
    if let Some(route) = route_id.and_then(|id| state.remote_view_routes.get(id)) {
        if route.controller_lease_id.is_some()
            && !matches!(
                request.priority,
                PresentationPriority::HumanControl | PresentationPriority::ExistingEffect
            )
        {
            return Some(CapacityLimitingResource::HumanController);
        }
        if request.requires_staging
            && route.viewer_lease_ids.iter().any(|lease_id| {
                state.viewer_leases.get(lease_id).is_some_and(|lease| {
                    lease.viewer_role != "controller"
                        && matches!(lease.state.as_str(), "requested" | "active" | "ready")
                })
            })
        {
            return Some(CapacityLimitingResource::ViewerStagingConflict);
        }
    }
    if state.remote_view_handoffs.values().any(|handoff| {
        route_id == handoff.last_route_id.as_deref()
            && matches!(handoff.state.as_str(), "ready" | "resolving" | "active")
            && handoff.browser_id.as_deref() != request.browser_id.as_deref()
            && handoff.browser_id.as_deref().is_some_and(|browser_id| {
                state.browsers.get(browser_id).is_some_and(|browser| {
                    browser
                        .view_streams
                        .iter()
                        .any(|stream| stream.route_id.as_deref() == route_id)
                })
            })
            && request.priority != PresentationPriority::Recovery
    }) {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{BrowserProcess, RemoteViewHandoff, ViewStream};

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
    fn stale_durable_handoff_history_does_not_occupy_a_route() {
        let slot = PresentationSlot::warm_idle("slot-1").with_binding("route-1", "display-1");
        let request = PresentationRequest::observation("request-1").for_browser("browser-current");
        let mut state = ServiceState {
            remote_view_handoffs: BTreeMap::from([(
                "handoff-old".to_string(),
                RemoteViewHandoff {
                    id: "handoff-old".to_string(),
                    state: "ready".to_string(),
                    browser_id: Some("browser-old".to_string()),
                    last_route_id: Some("route-1".to_string()),
                    ..RemoteViewHandoff::default()
                },
            )]),
            ..ServiceState::default()
        };

        assert_eq!(slot_admission_conflict(&state, &slot, &request), None);

        state.browsers.insert(
            "browser-old".to_string(),
            BrowserProcess {
                id: "browser-old".to_string(),
                view_streams: vec![ViewStream {
                    route_id: Some("route-1".to_string()),
                    ..ViewStream::default()
                }],
                ..BrowserProcess::default()
            },
        );
        assert_eq!(
            slot_admission_conflict(&state, &slot, &request),
            Some(CapacityLimitingResource::DurableHandoff)
        );
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
    fn active_human_controller_blocks_automation_but_not_human_continuation() {
        let mut state = ServiceState::default();
        state.remote_view_routes.insert(
            "route-1".to_string(),
            super::super::service_model::RemoteViewRoute {
                id: "route-1".to_string(),
                controller_lease_id: Some("controller-1".to_string()),
                ..Default::default()
            },
        );
        let mut automated = one_slot_authority(8);
        let blocked = automated.request_with_service_state(
            PresentationRequest::observation("observe").for_browser("browser-1"),
            PressureAdmission::admit(1),
            Some(&state),
        );
        assert_eq!(
            blocked.limiting_resource(),
            Some(CapacityLimitingResource::HumanController)
        );

        let mut human = one_slot_authority(8);
        assert!(human
            .request_with_service_state(
                PresentationRequest::human("continue").for_browser("browser-1"),
                PressureAdmission::admit(1),
                Some(&state),
            )
            .is_granted());
    }

    #[test]
    fn passive_viewer_allows_ready_capture_but_blocks_visible_staging() {
        let mut state = ServiceState::default();
        state.viewer_leases.insert(
            "viewer-1".to_string(),
            super::super::service_model::ViewerLease {
                id: "viewer-1".to_string(),
                route_id: Some("route-1".to_string()),
                state: "active".to_string(),
                ..Default::default()
            },
        );
        state.remote_view_routes.insert(
            "route-1".to_string(),
            super::super::service_model::RemoteViewRoute {
                id: "route-1".to_string(),
                viewer_lease_ids: vec!["viewer-1".to_string()],
                ..Default::default()
            },
        );
        let mut staging = one_slot_authority(8);
        assert_eq!(
            staging
                .request_with_service_state(
                    PresentationRequest::observation("stage")
                        .for_browser("browser-1")
                        .requiring_staging(),
                    PressureAdmission::admit(1),
                    Some(&state),
                )
                .limiting_resource(),
            Some(CapacityLimitingResource::ViewerStagingConflict)
        );

        let mut capture = one_slot_authority(8);
        assert!(capture
            .request_with_service_state(
                PresentationRequest::observation("capture").for_browser("browser-1"),
                PressureAdmission::admit(1),
                Some(&state),
            )
            .is_granted());
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
    fn durable_service_state_and_projection_preserve_capacity_authority() {
        let mut state = ServiceState {
            presentation_capacity: Some(one_slot_authority(8)),
            ..Default::default()
        };
        state.presentation_capacity.as_mut().unwrap().request(
            PresentationRequest::observation("queued"),
            PressureAdmission::admit(0),
        );
        let encoded = serde_json::to_value(&state).unwrap();
        assert_eq!(
            encoded.pointer("/presentationCapacity/slots/0/id"),
            Some(&serde_json::json!("slot-1"))
        );
        let decoded: ServiceState = serde_json::from_value(encoded).unwrap();
        let authority = decoded.presentation_capacity.as_ref().unwrap();
        let projection =
            authority.projection_with_service_state(PressureAdmission::admit(1), Some(&decoded));
        assert_eq!(projection.total_slots, 1);
        assert_eq!(projection.queued_by_priority.get("observation"), Some(&1));
        assert_eq!(projection.binding_warnings.len(), 2);
    }

    #[test]
    fn service_inventory_derivation_never_manufactures_unready_capacity() {
        use super::super::service_model::{DisplayAllocation, RemoteViewRoute, RoutePoolEntry};

        let config = PresentationCapacityConfig {
            warm_minimum: 4,
            hard_maximum: 6,
            human_priority_reserve: 1,
            recovery_reserve: 1,
            max_queue_depth: 8,
        };
        let mut state = ServiceState::default();
        for (id, ready) in [("one", true), ("two", false)] {
            let route_id = format!("route-{id}");
            let display_id = format!("display-{id}");
            state.display_allocations.insert(
                display_id.clone(),
                DisplayAllocation {
                    id: display_id.clone(),
                    state: if ready { "ready" } else { "allocating" }.to_string(),
                    ..Default::default()
                },
            );
            state.remote_view_routes.insert(
                route_id.clone(),
                RemoteViewRoute {
                    id: route_id.clone(),
                    display_allocation_id: Some(display_id),
                    state: if ready { "ready" } else { "allocating" }.to_string(),
                    ..Default::default()
                },
            );
            state.route_pool.insert(
                id.to_string(),
                RoutePoolEntry {
                    id: id.to_string(),
                    route_id: format!("provider-{id}"),
                    state: "available".to_string(),
                    current_route_allocation_id: Some(route_id),
                    readiness: Some(serde_json::json!({
                        "state": if ready { "ready" } else { "blocked" }
                    })),
                    ..Default::default()
                },
            );
        }

        let authority = PresentationCapacityAuthority::from_service_state(config, &state).unwrap();
        assert_eq!(authority.slots.len(), 1);
        assert_eq!(authority.slots[0].id, "slot:one");
        assert_eq!(authority.config.warm_minimum, 4);
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

    #[test]
    fn active_browser_staging_restores_the_exact_leased_slot_without_parking() {
        let mut authority = one_slot_authority(8);
        let slot = &mut authority.slots[0];
        slot.state = PresentationSlotState::Active;
        slot.browser_id = Some("browser-1".to_string());
        let decision = authority.request_bound_observation(
            PresentationRequest::observation("episode-active").for_browser("browser-1"),
            PressureAdmission::admit(1),
            &ServiceState::default(),
            "route-1",
            "display-1",
        );
        assert!(decision.is_granted());

        authority
            .transition_slot(
                "slot-1",
                "episode-active",
                PresentationSlotState::Staging,
                None,
            )
            .unwrap();
        assert!(authority.slots[0].restoration_pending);
        authority
            .transition_slot(
                "slot-1",
                "episode-active",
                PresentationSlotState::CaptureReady,
                None,
            )
            .unwrap();
        authority
            .transition_slot(
                "slot-1",
                "episode-active",
                PresentationSlotState::Restoring,
                None,
            )
            .unwrap();
        authority
            .transition_slot(
                "slot-1",
                "episode-active",
                PresentationSlotState::Active,
                None,
            )
            .unwrap();

        assert_eq!(authority.slots[0].state, PresentationSlotState::Active);
        assert!(!authority.slots[0].restoration_pending);
        assert_eq!(
            authority.slots[0].lease_request_id.as_deref(),
            Some("episode-active")
        );
        assert_eq!(authority.slots[0].browser_id.as_deref(), Some("browser-1"));
    }

    #[test]
    fn controller_arriving_after_reservation_fences_staging() {
        let mut authority = one_slot_authority(8);
        authority.request(
            PresentationRequest::observation("episode-1").for_browser("browser-1"),
            PressureAdmission::admit(1),
        );
        let mut state = ServiceState::default();
        state.remote_view_routes.insert(
            "route-1".to_string(),
            super::super::service_model::RemoteViewRoute {
                id: "route-1".to_string(),
                controller_lease_id: Some("human-controller".to_string()),
                ..Default::default()
            },
        );

        assert_eq!(
            authority
                .transition_slot(
                    "slot-1",
                    "episode-1",
                    PresentationSlotState::Staging,
                    Some(&state),
                )
                .unwrap_err(),
            "presentation_slot_staging_blocked:human_controller"
        );
        assert_eq!(authority.slots[0].state, PresentationSlotState::Reserved);
        assert_eq!(authority.slots[0].scene_generation, 0);
    }
}
