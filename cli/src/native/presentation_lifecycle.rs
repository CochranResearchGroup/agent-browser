//! Elastic presentation-slot lifecycle and exact cleanup authority.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

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
struct LifecycleSlot {
    lifecycle_generation: String,
    owned_resources: Vec<OwnedResource>,
}

#[derive(Debug, Clone)]
pub(crate) struct PresentationLifecycleAuthority {
    warm_minimum: usize,
    slots: BTreeMap<String, LifecycleSlot>,
}

impl PresentationLifecycleAuthority {
    pub(crate) fn new(warm_minimum: usize) -> Self {
        Self {
            warm_minimum,
            slots: BTreeMap::new(),
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

    pub(crate) fn add_warm_slot(&mut self, slot_id: impl Into<String>) {
        let slot_id = slot_id.into();
        self.slots.insert(
            slot_id,
            LifecycleSlot {
                lifecycle_generation: "static".to_string(),
                owned_resources: Vec::new(),
            },
        );
    }

    pub(crate) fn provision_exact(
        &mut self,
        slot_id: impl Into<String>,
        owned_resources: Vec<OwnedResource>,
    ) {
        let slot_id = slot_id.into();
        self.slots.insert(
            slot_id.clone(),
            LifecycleSlot {
                lifecycle_generation: format!("generation-{slot_id}"),
                owned_resources,
            },
        );
    }

    pub(crate) fn reclaim_exact(&mut self, slot_id: &str) -> Option<ReclaimReceipt> {
        if self.slots.len() <= self.warm_minimum {
            return None;
        }
        let slot = self.slots.remove(slot_id)?;
        let candidate = slot.owned_resources.into_iter().fold(
            ReclaimCandidate::idle(slot_id, slot.lifecycle_generation),
            ReclaimCandidate::with_owned_resource,
        );
        Some(Self::reclaim(candidate))
    }

    pub(crate) fn warm_slot_count(&self) -> usize {
        self.slots.len()
    }

    pub(crate) fn live_owned_resource_ids(&self) -> BTreeSet<&str> {
        self.slots
            .values()
            .flat_map(|slot| slot.owned_resources.iter())
            .map(|resource| resource.resource_id.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn exact_reclaim_cycles_converge_to_warm_minimum_without_leaks() {
        let mut lifecycle = PresentationLifecycleAuthority::new(2);
        lifecycle.add_warm_slot("slot-0");
        lifecycle.add_warm_slot("slot-1");

        for cycle in 0..3 {
            let slot_id = format!("slot-elastic-{cycle}");
            lifecycle.provision_exact(
                &slot_id,
                vec![
                    OwnedResource::display(format!("display-{cycle}"), format!("pid-{cycle}")),
                    OwnedResource::route(format!("route-{cycle}"), format!("provider-{cycle}")),
                ],
            );
            let receipt = lifecycle
                .reclaim_exact(&slot_id)
                .expect("elastic slot should reclaim exactly");
            assert_eq!(receipt.terminal_state, ReclaimTerminalState::Absent);
            assert_eq!(receipt.deleted_resource_ids.len(), 2);
        }

        assert_eq!(lifecycle.warm_slot_count(), 2);
        assert!(lifecycle.live_owned_resource_ids().is_empty());
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
