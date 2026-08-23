//! Arbitrary-N presentation slot inventory, admission, and queue authority.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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
}

impl PresentationSlot {
    pub(crate) fn warm_idle(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            state: PresentationSlotState::WarmIdle,
            lease_request_id: None,
            lease_priority: None,
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PresentationRequest {
    id: String,
    priority: PresentationPriority,
    queued_at: Option<u64>,
}

impl PresentationRequest {
    pub(crate) fn observation(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            priority: PresentationPriority::Observation,
            queued_at: None,
        }
    }

    pub(crate) fn recovery(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            priority: PresentationPriority::Recovery,
            queued_at: None,
        }
    }

    pub(crate) fn human(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            priority: PresentationPriority::HumanControl,
            queued_at: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapacityLimitingResource {
    ReservedCapacity,
    PressureAdmission,
    WarmSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapacityNextSafeAction {
    WaitForCapacity,
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
}

impl CapacityDecision {
    pub(crate) fn is_granted(&self) -> bool {
        matches!(self, Self::Granted { .. })
    }

    pub(crate) fn queue_position(&self) -> Option<usize> {
        match self {
            Self::Queued { queue_position, .. } => Some(*queue_position),
            Self::Granted { .. } => None,
        }
    }

    pub(crate) fn limiting_resource(&self) -> Option<CapacityLimitingResource> {
        match self {
            Self::Queued {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PressureAdmission {
    admitted_maximum: usize,
}

impl PressureAdmission {
    pub(crate) fn admit(admitted_maximum: usize) -> Self {
        Self { admitted_maximum }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresentationCapacityProjection {
    pub(crate) total_slots: usize,
    pub(crate) slot_ids: Vec<String>,
    pub(crate) configured_hard_maximum: usize,
    pub(crate) pressure_admitted_maximum: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct PresentationCapacityAuthority {
    config: PresentationCapacityConfig,
    slots: Vec<PresentationSlot>,
    queued_requests: Vec<PresentationRequest>,
    queue_clock: u64,
}

impl PresentationCapacityAuthority {
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
        Ok(Self {
            config,
            slots,
            queued_requests: Vec::new(),
            queue_clock: 0,
        })
    }

    pub(crate) fn projection(&self, pressure: PressureAdmission) -> PresentationCapacityProjection {
        PresentationCapacityProjection {
            total_slots: self.slots.len(),
            slot_ids: self.slots.iter().map(|slot| slot.id.clone()).collect(),
            configured_hard_maximum: self.config.hard_maximum,
            pressure_admitted_maximum: pressure.admitted_maximum.min(self.config.hard_maximum),
        }
    }

    pub(crate) fn request(
        &mut self,
        mut request: PresentationRequest,
        pressure: PressureAdmission,
    ) -> CapacityDecision {
        self.queue_clock = self.queue_clock.saturating_add(1);
        let admitted_maximum = pressure.admitted_maximum.min(self.config.hard_maximum);
        let eligible_slots = self.slots.len().min(admitted_maximum);
        let free_slots = self
            .slots
            .iter()
            .take(eligible_slots)
            .filter(|slot| slot.state == PresentationSlotState::WarmIdle)
            .count();
        let protected_reserve = self.protected_reserve(request.priority);
        if free_slots > protected_reserve {
            let slot = self
                .slots
                .iter_mut()
                .take(eligible_slots)
                .find(|slot| slot.state == PresentationSlotState::WarmIdle)
                .expect("free slot count and inventory must agree");
            slot.state = PresentationSlotState::Reserved;
            slot.lease_request_id = Some(request.id.clone());
            slot.lease_priority = Some(request.priority);
            return CapacityDecision::Granted {
                request_id: request.id,
                slot_id: slot.id.clone(),
            };
        }

        let limiting_resource = if eligible_slots < self.slots.len() {
            CapacityLimitingResource::PressureAdmission
        } else if free_slots > 0 && protected_reserve > 0 {
            CapacityLimitingResource::ReservedCapacity
        } else {
            CapacityLimitingResource::WarmSlot
        };
        request.queued_at = Some(self.queue_clock);
        self.queued_requests.push(request.clone());
        CapacityDecision::Queued {
            request_id: request.id,
            queue_position: self.queued_requests.len(),
            limiting_resource,
            next_safe_action: CapacityNextSafeAction::WaitForCapacity,
        }
    }

    pub(crate) fn release_and_dispatch(
        &mut self,
        slot_id: &str,
        pressure: PressureAdmission,
    ) -> Option<CapacityDecision> {
        self.queue_clock = self.queue_clock.saturating_add(1);
        let slot = self.slots.iter_mut().find(|slot| slot.id == slot_id)?;
        slot.state = PresentationSlotState::WarmIdle;
        slot.lease_request_id = None;
        slot.lease_priority = None;

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
            .filter(|(_, request)| free_slots > self.protected_reserve(request.priority))
            .min_by_key(|(index, request)| {
                (
                    effective_priority_rank(request, self.queue_clock),
                    request.queued_at.unwrap_or(u64::MAX),
                    *index,
                )
            })
            .map(|(index, _)| index)?;
        let request = self.queued_requests.remove(queue_index);
        let slot = self
            .slots
            .iter_mut()
            .take(eligible_slots)
            .find(|slot| slot.state == PresentationSlotState::WarmIdle)?;
        slot.state = PresentationSlotState::Reserved;
        slot.lease_request_id = Some(request.id.clone());
        slot.lease_priority = Some(request.priority);
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
