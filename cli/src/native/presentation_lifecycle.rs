//! Elastic presentation-slot lifecycle and exact cleanup authority.

use serde::Serialize;
use std::collections::BTreeMap;

use super::presentation_capacity::{
    PresentationCapacityAuthority, PresentationSlot, PresentationSlotState, PressureAdmission,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OwnedResourceKind {
    Display,
    Route,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OwnedResource {
    kind: OwnedResourceKind,
    resource_id: String,
    ownership_identity: String,
}

impl OwnedResource {
    pub(crate) fn display(
        resource_id: impl Into<String>,
        process_identity: impl Into<String>,
    ) -> Self {
        Self {
            kind: OwnedResourceKind::Display,
            resource_id: resource_id.into(),
            ownership_identity: process_identity.into(),
        }
    }

    pub(crate) fn route(
        resource_id: impl Into<String>,
        provider_identity: impl Into<String>,
    ) -> Self {
        Self {
            kind: OwnedResourceKind::Route,
            resource_id: resource_id.into(),
            ownership_identity: provider_identity.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReclaimCandidate {
    slot_id: String,
    lifecycle_generation: String,
    owned_resources: Vec<OwnedResource>,
    ambiguities: Vec<String>,
    blockers: Vec<ReclaimBlocker>,
}

impl ReclaimCandidate {
    pub(crate) fn idle(
        slot_id: impl Into<String>,
        lifecycle_generation: impl Into<String>,
    ) -> Self {
        Self {
            slot_id: slot_id.into(),
            lifecycle_generation: lifecycle_generation.into(),
            owned_resources: Vec::new(),
            ambiguities: Vec::new(),
            blockers: Vec::new(),
        }
    }

    pub(crate) fn with_owned_resource(mut self, resource: OwnedResource) -> Self {
        self.owned_resources.push(resource);
        self
    }

    pub(crate) fn with_ambiguous_resource(mut self, ambiguity: impl Into<String>) -> Self {
        self.ambiguities.push(ambiguity.into());
        self
    }

    pub(crate) fn with_blocker(mut self, blocker: ReclaimBlocker) -> Self {
        self.blockers.push(blocker);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReclaimBlocker {
    BrowserPresentation,
    AcquisitionLease,
    EpisodeLease,
    ViewerLease,
    ControllerLease,
    DurableHandoff,
    RollbackReference,
    RecoveryReference,
    PendingRestoration,
    CleanupObligation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReclaimTerminalState {
    Absent,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReclaimReceipt {
    pub(crate) slot_id: String,
    pub(crate) lifecycle_generation: String,
    pub(crate) terminal_state: ReclaimTerminalState,
    pub(crate) deleted_resource_ids: Vec<String>,
    pub(crate) ambiguities: Vec<String>,
    pub(crate) blockers: Vec<ReclaimBlocker>,
}

#[derive(Debug, Clone)]
pub(crate) struct PresentationLifecycleAuthority {
    warm_minimum: usize,
    cooldown_ticks: u64,
    next_generation: u64,
    elastic_resources: BTreeMap<String, Vec<OwnedResource>>,
    lifecycle_generations: BTreeMap<String, String>,
    cooling_since: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum ScaleOutDecision {
    Provisioned {
        slot_id: String,
        lifecycle_generation: String,
    },
    Quarantined {
        slot_id: String,
        cleanup_obligation_id: String,
        reason: String,
    },
    Deferred {
        reason: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProvisionedPresentationSlot {
    pub(crate) slot_id: String,
    pub(crate) lifecycle_generation: String,
    pub(crate) route_id: String,
    pub(crate) display_allocation_id: String,
    pub(crate) owned_resources: Vec<OwnedResource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProvisioningRollback {
    pub(crate) owned_resources: Vec<OwnedResource>,
    pub(crate) reason: String,
}

pub(crate) trait PresentationProvisioningAdapter {
    fn provision_one(
        &mut self,
        slot_id: &str,
        lifecycle_generation: &str,
    ) -> Result<ProvisionedPresentationSlot, ProvisioningRollback>;
}

pub(crate) trait PresentationReferenceAdapter {
    fn blockers(&mut self, slot_id: &str) -> Vec<ReclaimBlocker>;
    fn ambiguities(&mut self, slot_id: &str) -> Vec<String>;
}

pub(crate) trait PresentationGarbageCollectorAdapter {
    fn reclaim_owned_resource(&mut self, resource: &OwnedResource) -> Result<String, String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum ScaleInDecision {
    Reclaimed {
        receipt: ReclaimReceipt,
    },
    Deferred {
        slot_id: Option<String>,
        blockers: Vec<ReclaimBlocker>,
        reason: &'static str,
    },
    Quarantined {
        receipt: ReclaimReceipt,
    },
}

impl PresentationLifecycleAuthority {
    pub(crate) fn new(warm_minimum: usize) -> Self {
        Self {
            warm_minimum,
            cooldown_ticks: 0,
            next_generation: 0,
            elastic_resources: BTreeMap::new(),
            lifecycle_generations: BTreeMap::new(),
            cooling_since: BTreeMap::new(),
        }
    }

    pub(crate) fn elastic(warm_minimum: usize, cooldown_ticks: u64) -> Self {
        Self {
            warm_minimum,
            cooldown_ticks,
            ..Self::new(warm_minimum)
        }
    }

    pub(crate) fn scale_out_one(
        &mut self,
        capacity: &mut PresentationCapacityAuthority,
        pressure: PressureAdmission,
        provisioner: &mut dyn PresentationProvisioningAdapter,
    ) -> ScaleOutDecision {
        if capacity
            .slots
            .iter()
            .any(|slot| slot.state == PresentationSlotState::Provisioning)
        {
            return ScaleOutDecision::Deferred {
                reason: "provisioning_in_flight",
            };
        }
        if capacity.slots.len() >= capacity.config.hard_maximum {
            return ScaleOutDecision::Deferred {
                reason: "configured_hard_maximum",
            };
        }
        if capacity.slots.len() >= pressure.admitted_maximum() {
            return ScaleOutDecision::Deferred {
                reason: "pressure_admission",
            };
        }

        self.next_generation = self.next_generation.saturating_add(1);
        let slot_id = format!("slot-elastic-{}", self.next_generation);
        let lifecycle_generation = format!("lifecycle-{}", self.next_generation);
        let mut slot = PresentationSlot::warm_idle(&slot_id);
        slot.state = PresentationSlotState::Provisioning;
        capacity.slots.push(slot);

        match provisioner.provision_one(&slot_id, &lifecycle_generation) {
            Ok(provisioned)
                if provisioned.slot_id == slot_id
                    && provisioned.lifecycle_generation == lifecycle_generation
                    && !provisioned.route_id.trim().is_empty()
                    && !provisioned.display_allocation_id.trim().is_empty()
                    && !provisioned.owned_resources.is_empty()
                    && provisioned.owned_resources.iter().all(|resource| {
                        !resource.resource_id.trim().is_empty()
                            && !resource.ownership_identity.trim().is_empty()
                    }) =>
            {
                let slot = capacity
                    .slots
                    .iter_mut()
                    .find(|slot| slot.id == slot_id)
                    .expect("provisioning slot must remain present");
                slot.route_id = Some(provisioned.route_id);
                slot.display_allocation_id = Some(provisioned.display_allocation_id);
                slot.state = PresentationSlotState::WarmIdle;
                self.elastic_resources
                    .insert(slot_id.clone(), provisioned.owned_resources);
                self.lifecycle_generations
                    .insert(slot_id.clone(), lifecycle_generation.clone());
                ScaleOutDecision::Provisioned {
                    slot_id,
                    lifecycle_generation,
                }
            }
            Ok(provisioned) => self.quarantine_failed_provision(
                capacity,
                slot_id,
                lifecycle_generation,
                provisioned.owned_resources,
                "provisioned_identity_mismatch".to_string(),
            ),
            Err(rollback) => self.quarantine_failed_provision(
                capacity,
                slot_id,
                lifecycle_generation,
                rollback.owned_resources,
                rollback.reason,
            ),
        }
    }

    fn quarantine_failed_provision(
        &mut self,
        capacity: &mut PresentationCapacityAuthority,
        slot_id: String,
        lifecycle_generation: String,
        owned_resources: Vec<OwnedResource>,
        reason: String,
    ) -> ScaleOutDecision {
        let cleanup_obligation_id = format!("cleanup:{slot_id}:{lifecycle_generation}");
        let slot = capacity
            .slots
            .iter_mut()
            .find(|slot| slot.id == slot_id)
            .expect("failed provisioning slot must remain present");
        slot.state = PresentationSlotState::Quarantined;
        slot.cleanup_obligation_ids
            .push(cleanup_obligation_id.clone());
        self.elastic_resources
            .insert(slot_id.clone(), owned_resources);
        self.lifecycle_generations
            .insert(slot_id.clone(), lifecycle_generation.clone());
        ScaleOutDecision::Quarantined {
            slot_id,
            cleanup_obligation_id,
            reason,
        }
    }

    pub(crate) fn begin_cooldown(
        &mut self,
        capacity: &mut PresentationCapacityAuthority,
        slot_id: &str,
        now: u64,
    ) -> Result<(), String> {
        if !self.elastic_resources.contains_key(slot_id) {
            return Err("presentation_slot_not_elastic".to_string());
        }
        let slot = capacity
            .slots
            .iter_mut()
            .find(|slot| slot.id == slot_id)
            .ok_or_else(|| "presentation_slot_not_found".to_string())?;
        if slot.state != PresentationSlotState::WarmIdle {
            return Err("presentation_slot_not_idle".to_string());
        }
        slot.state = PresentationSlotState::Cooling;
        self.cooling_since.insert(slot_id.to_string(), now);
        Ok(())
    }

    pub(crate) fn reclaim_one_due(
        &mut self,
        capacity: &mut PresentationCapacityAuthority,
        now: u64,
        references: &mut dyn PresentationReferenceAdapter,
        garbage_collector: &mut dyn PresentationGarbageCollectorAdapter,
    ) -> ScaleInDecision {
        if capacity.slots.len() <= self.warm_minimum {
            return ScaleInDecision::Deferred {
                slot_id: None,
                blockers: Vec::new(),
                reason: "warm_minimum",
            };
        }
        let selected = self
            .cooling_since
            .iter()
            .filter(|(slot_id, since)| {
                now.saturating_sub(**since) >= self.cooldown_ticks
                    && capacity.slots.iter().any(|slot| {
                        slot.id == slot_id.as_str() && slot.state == PresentationSlotState::Cooling
                    })
            })
            .min_by_key(|(slot_id, since)| (**since, *slot_id))
            .map(|(slot_id, _)| slot_id.clone());
        let Some(slot_id) = selected else {
            return ScaleInDecision::Deferred {
                slot_id: None,
                blockers: Vec::new(),
                reason: "cooldown_not_elapsed",
            };
        };
        let blockers = references.blockers(&slot_id);
        if !blockers.is_empty() {
            return ScaleInDecision::Deferred {
                slot_id: Some(slot_id),
                blockers,
                reason: "referenced",
            };
        }
        let mut ambiguities = references.ambiguities(&slot_id);
        let resources = self
            .elastic_resources
            .get(&slot_id)
            .cloned()
            .unwrap_or_default();
        let lifecycle_generation = self
            .lifecycle_generations
            .get(&slot_id)
            .cloned()
            .unwrap_or_default();
        if resources.is_empty() {
            ambiguities.push("owned_resource_inventory_missing".to_string());
        }
        if lifecycle_generation.is_empty() {
            ambiguities.push("lifecycle_generation_missing".to_string());
        }
        if !ambiguities.is_empty() {
            return self.quarantine_scale_in(capacity, slot_id, ambiguities, Vec::new());
        }
        let mut deleted_resource_ids = Vec::new();
        let mut cleanup_errors = Vec::new();
        for resource in &resources {
            match garbage_collector.reclaim_owned_resource(resource) {
                Ok(_) => deleted_resource_ids.push(resource.resource_id.clone()),
                Err(error) => cleanup_errors.push(error),
            }
        }
        if !cleanup_errors.is_empty() {
            return self.quarantine_scale_in(
                capacity,
                slot_id,
                cleanup_errors,
                deleted_resource_ids,
            );
        }
        capacity.slots.retain(|slot| slot.id != slot_id);
        self.elastic_resources.remove(&slot_id);
        self.lifecycle_generations.remove(&slot_id);
        self.cooling_since.remove(&slot_id);
        ScaleInDecision::Reclaimed {
            receipt: ReclaimReceipt {
                slot_id,
                lifecycle_generation,
                terminal_state: ReclaimTerminalState::Absent,
                deleted_resource_ids,
                ambiguities: Vec::new(),
                blockers: Vec::new(),
            },
        }
    }

    fn quarantine_scale_in(
        &mut self,
        capacity: &mut PresentationCapacityAuthority,
        slot_id: String,
        ambiguities: Vec<String>,
        deleted_resource_ids: Vec<String>,
    ) -> ScaleInDecision {
        let lifecycle_generation = self
            .lifecycle_generations
            .get(&slot_id)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        if let Some(slot) = capacity.slots.iter_mut().find(|slot| slot.id == slot_id) {
            slot.state = PresentationSlotState::Quarantined;
            let obligation = format!("cleanup:{slot_id}:scale-in");
            if !slot.cleanup_obligation_ids.contains(&obligation) {
                slot.cleanup_obligation_ids.push(obligation);
            }
        }
        ScaleInDecision::Quarantined {
            receipt: ReclaimReceipt {
                slot_id,
                lifecycle_generation,
                terminal_state: ReclaimTerminalState::Quarantined,
                deleted_resource_ids,
                ambiguities,
                blockers: Vec::new(),
            },
        }
    }

    pub(crate) fn reclaim(candidate: ReclaimCandidate) -> ReclaimReceipt {
        if !candidate.ambiguities.is_empty()
            || !candidate.blockers.is_empty()
            || candidate.owned_resources.iter().any(|resource| {
                resource.resource_id.is_empty() || resource.ownership_identity.is_empty()
            })
        {
            return ReclaimReceipt {
                slot_id: candidate.slot_id,
                lifecycle_generation: candidate.lifecycle_generation,
                terminal_state: ReclaimTerminalState::Quarantined,
                deleted_resource_ids: Vec::new(),
                ambiguities: candidate.ambiguities,
                blockers: candidate.blockers,
            };
        }
        ReclaimReceipt {
            slot_id: candidate.slot_id,
            lifecycle_generation: candidate.lifecycle_generation,
            terminal_state: ReclaimTerminalState::Absent,
            deleted_resource_ids: candidate
                .owned_resources
                .into_iter()
                .map(|resource| resource.resource_id)
                .collect(),
            ambiguities: Vec::new(),
            blockers: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::presentation_capacity::PresentationCapacityConfig;

    struct FakeProvisioner {
        fail: bool,
        calls: usize,
    }

    impl PresentationProvisioningAdapter for FakeProvisioner {
        fn provision_one(
            &mut self,
            slot_id: &str,
            lifecycle_generation: &str,
        ) -> Result<ProvisionedPresentationSlot, ProvisioningRollback> {
            self.calls += 1;
            let resources = vec![
                OwnedResource::display(format!("display-{slot_id}"), format!("pid-{slot_id}")),
                OwnedResource::route(format!("route-{slot_id}"), format!("provider-{slot_id}")),
            ];
            if self.fail {
                return Err(ProvisioningRollback {
                    owned_resources: resources,
                    reason: "provider_failed_after_partial_effect".to_string(),
                });
            }
            Ok(ProvisionedPresentationSlot {
                slot_id: slot_id.to_string(),
                lifecycle_generation: lifecycle_generation.to_string(),
                route_id: format!("route-{slot_id}"),
                display_allocation_id: format!("display-{slot_id}"),
                owned_resources: resources,
            })
        }
    }

    #[derive(Default)]
    struct FakeReferences {
        blockers: Vec<ReclaimBlocker>,
        ambiguities: Vec<String>,
    }

    impl PresentationReferenceAdapter for FakeReferences {
        fn blockers(&mut self, _slot_id: &str) -> Vec<ReclaimBlocker> {
            self.blockers.clone()
        }
        fn ambiguities(&mut self, _slot_id: &str) -> Vec<String> {
            self.ambiguities.clone()
        }
    }

    #[derive(Default)]
    struct FakeGarbageCollector {
        reclaimed: Vec<String>,
    }

    impl PresentationGarbageCollectorAdapter for FakeGarbageCollector {
        fn reclaim_owned_resource(&mut self, resource: &OwnedResource) -> Result<String, String> {
            self.reclaimed.push(resource.resource_id.clone());
            Ok(format!("gc:{}", resource.resource_id))
        }
    }

    fn capacity() -> PresentationCapacityAuthority {
        PresentationCapacityAuthority::new(
            PresentationCapacityConfig {
                warm_minimum: 2,
                hard_maximum: 6,
                human_priority_reserve: 1,
                recovery_reserve: 1,
                max_queue_depth: 8,
            },
            vec![
                PresentationSlot::warm_idle("slot-0"),
                PresentationSlot::warm_idle("slot-1"),
            ],
        )
        .unwrap()
    }

    #[test]
    fn scale_out_provisions_exactly_one_slot_per_admitted_request() {
        let mut capacity = capacity();
        let mut lifecycle = PresentationLifecycleAuthority::elastic(2, 5);
        let mut provisioner = FakeProvisioner {
            fail: false,
            calls: 0,
        };

        let decision =
            lifecycle.scale_out_one(&mut capacity, PressureAdmission::admit(6), &mut provisioner);

        assert!(matches!(decision, ScaleOutDecision::Provisioned { .. }));
        assert_eq!(provisioner.calls, 1);
        assert_eq!(capacity.slots.len(), 3);
        assert_eq!(
            capacity
                .slots
                .iter()
                .filter(|slot| slot.id.starts_with("slot-elastic-"))
                .count(),
            1
        );
    }

    #[test]
    fn pressure_blocks_scale_out_without_calling_the_provider() {
        let mut capacity = capacity();
        let mut lifecycle = PresentationLifecycleAuthority::elastic(2, 5);
        let mut provisioner = FakeProvisioner {
            fail: false,
            calls: 0,
        };

        let decision =
            lifecycle.scale_out_one(&mut capacity, PressureAdmission::admit(2), &mut provisioner);

        assert_eq!(
            decision,
            ScaleOutDecision::Deferred {
                reason: "pressure_admission"
            }
        );
        assert_eq!(provisioner.calls, 0);
        assert_eq!(capacity.slots.len(), 2);
    }

    #[test]
    fn partial_provisioning_failure_is_quarantined_with_cleanup_obligation() {
        let mut capacity = capacity();
        let mut lifecycle = PresentationLifecycleAuthority::elastic(2, 5);
        let mut provisioner = FakeProvisioner {
            fail: true,
            calls: 0,
        };

        let decision =
            lifecycle.scale_out_one(&mut capacity, PressureAdmission::admit(6), &mut provisioner);

        assert!(matches!(decision, ScaleOutDecision::Quarantined { .. }));
        let slot = capacity.slots.last().unwrap();
        assert_eq!(slot.state, PresentationSlotState::Quarantined);
        assert_eq!(slot.cleanup_obligation_ids.len(), 1);
    }

    #[test]
    fn cooldown_and_exact_references_gate_scale_in_before_gc() {
        let mut capacity = capacity();
        let mut lifecycle = PresentationLifecycleAuthority::elastic(2, 5);
        let mut provisioner = FakeProvisioner {
            fail: false,
            calls: 0,
        };
        let ScaleOutDecision::Provisioned { slot_id, .. } =
            lifecycle.scale_out_one(&mut capacity, PressureAdmission::admit(6), &mut provisioner)
        else {
            panic!("scale out should succeed");
        };
        lifecycle
            .begin_cooldown(&mut capacity, &slot_id, 10)
            .unwrap();
        let mut references = FakeReferences {
            blockers: vec![ReclaimBlocker::DurableHandoff],
            ambiguities: Vec::new(),
        };
        let mut gc = FakeGarbageCollector::default();

        assert!(matches!(
            lifecycle.reclaim_one_due(&mut capacity, 14, &mut references, &mut gc),
            ScaleInDecision::Deferred {
                reason: "cooldown_not_elapsed",
                ..
            }
        ));
        assert!(matches!(
            lifecycle.reclaim_one_due(&mut capacity, 15, &mut references, &mut gc),
            ScaleInDecision::Deferred {
                reason: "referenced",
                ..
            }
        ));
        assert!(gc.reclaimed.is_empty());

        references.blockers.clear();
        let decision = lifecycle.reclaim_one_due(&mut capacity, 15, &mut references, &mut gc);
        assert!(matches!(decision, ScaleInDecision::Reclaimed { .. }));
        assert_eq!(gc.reclaimed.len(), 2);
        assert_eq!(capacity.slots.len(), 2);
    }

    #[test]
    fn repeated_elastic_cycles_converge_to_warm_minimum_without_owned_resource_leaks() {
        let mut capacity = capacity();
        let mut lifecycle = PresentationLifecycleAuthority::elastic(2, 1);
        let mut provisioner = FakeProvisioner {
            fail: false,
            calls: 0,
        };
        let mut references = FakeReferences::default();
        let mut gc = FakeGarbageCollector::default();

        for cycle in 0..3 {
            let ScaleOutDecision::Provisioned { slot_id, .. } = lifecycle.scale_out_one(
                &mut capacity,
                PressureAdmission::admit(6),
                &mut provisioner,
            ) else {
                panic!("cycle scale out should succeed");
            };
            lifecycle
                .begin_cooldown(&mut capacity, &slot_id, cycle * 10)
                .unwrap();
            assert!(matches!(
                lifecycle.reclaim_one_due(&mut capacity, cycle * 10 + 1, &mut references, &mut gc,),
                ScaleInDecision::Reclaimed { .. }
            ));
        }

        assert_eq!(capacity.slots.len(), 2);
        assert!(lifecycle.elastic_resources.is_empty());
        assert!(lifecycle.lifecycle_generations.is_empty());
        assert!(lifecycle.cooling_since.is_empty());
        assert_eq!(gc.reclaimed.len(), 6);
    }

    #[test]
    fn ambiguous_cleanup_is_quarantined_without_deleting_any_resource() {
        let candidate = ReclaimCandidate::idle("slot-4", "lifecycle-7")
            .with_owned_resource(OwnedResource::display("display-4", "pid-400"))
            .with_ambiguous_resource("guacamole-connection");

        let receipt = PresentationLifecycleAuthority::reclaim(candidate);

        assert_eq!(receipt.terminal_state, ReclaimTerminalState::Quarantined);
        assert!(receipt.deleted_resource_ids.is_empty());
        assert_eq!(receipt.ambiguities, vec!["guacamole-connection"]);
    }

    #[test]
    fn retained_browser_and_handoff_references_block_scale_in() {
        let blockers = vec![
            ReclaimBlocker::BrowserPresentation,
            ReclaimBlocker::AcquisitionLease,
            ReclaimBlocker::EpisodeLease,
            ReclaimBlocker::ViewerLease,
            ReclaimBlocker::ControllerLease,
            ReclaimBlocker::DurableHandoff,
            ReclaimBlocker::RollbackReference,
            ReclaimBlocker::RecoveryReference,
            ReclaimBlocker::PendingRestoration,
            ReclaimBlocker::CleanupObligation,
        ];
        let candidate = blockers.iter().copied().fold(
            ReclaimCandidate::idle("slot-5", "lifecycle-8")
                .with_owned_resource(OwnedResource::display("display-5", "pid-500")),
            ReclaimCandidate::with_blocker,
        );

        let receipt = PresentationLifecycleAuthority::reclaim(candidate);

        assert_eq!(receipt.terminal_state, ReclaimTerminalState::Quarantined);
        assert!(receipt.deleted_resource_ids.is_empty());
        assert_eq!(receipt.blockers, blockers);
    }
}
