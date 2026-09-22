use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{RouteKeeperAuthority, RouteKeeperFence, RouteKeeperPhase};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationScaleInIdleEvidence {
    pub fence: RouteKeeperFence,
    pub idle_since_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationScaleInState {
    #[serde(default)]
    pub idle_slots: BTreeMap<String, PresentationScaleInIdleEvidence>,
    #[serde(default)]
    pub last_observed_ms: Option<u64>,
}

impl PresentationScaleInState {
    /// Observes provider-free route state and returns at most one idle slot eligible for scale-in.
    ///
    /// The caller owns references from browsers and handoffs and must conservatively include any
    /// slot whose use is uncertain. Selecting a slot has no effect on the route authority or
    /// provider; the adapter must reserve and execute that effect in its own transaction.
    pub fn observe_and_select(
        &mut self,
        authority: &RouteKeeperAuthority,
        referenced_slots: &BTreeSet<String>,
        admission_pending: bool,
        now_ms: u64,
        cooldown_ms: u64,
    ) -> Result<Option<String>, String> {
        authority.projection()?;

        if self
            .last_observed_ms
            .is_some_and(|last_observed_ms| now_ms < last_observed_ms)
        {
            self.idle_slots.clear();
        }
        self.last_observed_ms = Some(now_ms);

        if admission_pending
            || authority.records.values().any(|record| {
                !matches!(
                    record.phase,
                    RouteKeeperPhase::Absent | RouteKeeperPhase::Ready
                )
            })
        {
            self.idle_slots.clear();
            return Ok(None);
        }

        let Some(current_host_generation) =
            authority.host_process_claims.keys().next_back().copied()
        else {
            self.idle_slots.clear();
            return Ok(None);
        };
        let ready = authority
            .records
            .values()
            .filter(|record| {
                record.phase == RouteKeeperPhase::Ready
                    && record.fence.host_generation == current_host_generation
                    && record.protocol_ready.is_some()
            })
            .collect::<Vec<_>>();
        let eligible = ready
            .iter()
            .filter(|record| !referenced_slots.contains(&record.slot_id))
            .map(|record| (record.slot_id.as_str(), &record.fence))
            .collect::<BTreeMap<_, _>>();

        self.idle_slots
            .retain(|slot_id, _| eligible.contains_key(slot_id.as_str()));
        for (slot_id, fence) in eligible {
            let evidence = self
                .idle_slots
                .entry(slot_id.to_string())
                .or_insert_with(|| PresentationScaleInIdleEvidence {
                    fence: fence.clone(),
                    idle_since_ms: now_ms,
                });
            if evidence.fence != *fence || now_ms < evidence.idle_since_ms {
                evidence.fence = fence.clone();
                evidence.idle_since_ms = now_ms;
            }
        }

        let target = usize::try_from(
            authority
                .policy
                .minimum_ready
                .max(authority.policy.warm_target),
        )
        .unwrap_or(usize::MAX)
        .max(referenced_slots.len());
        if ready.len() <= target {
            return Ok(None);
        }

        Ok(self
            .idle_slots
            .iter()
            .rev()
            .find(|(_, evidence)| now_ms.saturating_sub(evidence.idle_since_ms) >= cooldown_ms)
            .map(|(slot_id, _)| slot_id.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RecordedProcessIdentity, RouteKeeperHostProcessClaim, RouteKeeperPolicy,
        RouteKeeperProtocolReadyReceipt, RouteKeeperXrdpOwnershipWitness,
    };

    fn authority(ready: u32, host_generation: u64) -> RouteKeeperAuthority {
        let mut authority = RouteKeeperAuthority::with_policy(
            1,
            RouteKeeperPolicy {
                minimum_ready: 1,
                warm_target: 2,
                maximum_slots: 4,
            },
        )
        .unwrap();
        authority
            .register_host_process_claim(RouteKeeperHostProcessClaim {
                host_generation,
                boot_epoch: format!("boot:{host_generation}"),
                process_identity: RecordedProcessIdentity {
                    pid: 4000 + u32::try_from(host_generation).unwrap(),
                    start_token: format!("start:{host_generation}"),
                    executable_path: Some("/opt/agent-browser".to_string()),
                    browser_family: None,
                },
            })
            .unwrap();
        for sequence in 1..=ready {
            let slot_id = format!("route-slot-{sequence:02}");
            let record = authority.records.get_mut(&slot_id).unwrap();
            record.fence.operation_generation = 1;
            record.fence.operation_id = format!("operation:{sequence}");
            record.phase = RouteKeeperPhase::Ready;
            record.protocol_ready = Some(ready_receipt(record, sequence));
        }
        authority
    }

    fn ready_receipt(
        record: &crate::RouteKeeperRecord,
        sequence: u32,
    ) -> RouteKeeperProtocolReadyReceipt {
        let session_id = format!("xrdp-{sequence}");
        let display_name = format!(":{sequence}");
        RouteKeeperProtocolReadyReceipt {
            slot_id: record.slot_id.clone(),
            keeper_id: record.keeper_id.clone(),
            fence: record.fence.clone(),
            guacamole_connection_uuid: format!("connection-{sequence}"),
            xrdp_session_id: session_id.clone(),
            display_name: display_name.clone(),
            xrdp_ownership: Some(RouteKeeperXrdpOwnershipWitness {
                schema_version: "agent-browser.route-keeper-xrdp-ownership.v1".to_string(),
                boot_id: "boot-fixture".to_string(),
                route_user: format!("route-user-{sequence}"),
                route_uid: 2000 + sequence,
                session_id: session_id.clone(),
                session_service: "xrdp-sesman".to_string(),
                session_scope: format!("session-{session_id}.scope"),
                scope_invocation_id: format!("invocation-{sequence}"),
                cgroup_path: format!(
                    "/user.slice/user-{}.slice/session-{session_id}.scope",
                    2000 + sequence
                ),
                cgroup_device: 1,
                cgroup_inode: u64::from(sequence),
                leader_pid: 4100 + sequence,
                leader_start_ticks: 5100 + u64::from(sequence),
                x_server_pid: 4200 + sequence,
                x_server_start_ticks: 5200 + u64::from(sequence),
                display_name,
                x11_socket_inode: 6100 + u64::from(sequence),
            }),
            observed_at: "2026-09-22T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn waits_for_cooldown_and_selects_only_highest_slot() {
        let authority = authority(4, 1);
        let mut state = PresentationScaleInState::default();
        assert_eq!(
            state.observe_and_select(&authority, &BTreeSet::new(), false, 100, 50),
            Ok(None)
        );
        assert_eq!(
            state.observe_and_select(&authority, &BTreeSet::new(), false, 150, 50),
            Ok(Some("route-slot-04".to_string()))
        );
    }

    #[test]
    fn references_and_pending_admission_reset_idle_evidence() {
        let authority = authority(4, 1);
        let mut state = PresentationScaleInState::default();
        state
            .observe_and_select(&authority, &BTreeSet::new(), false, 100, 50)
            .unwrap();
        assert_eq!(
            state.observe_and_select(
                &authority,
                &BTreeSet::from(["route-slot-04".to_string()]),
                false,
                200,
                50,
            ),
            Ok(Some("route-slot-03".to_string()))
        );
        assert!(!state.idle_slots.contains_key("route-slot-04"));
        assert_eq!(
            state.observe_and_select(&authority, &BTreeSet::new(), true, 300, 50),
            Ok(None)
        );
        assert!(state.idle_slots.is_empty());
    }

    #[test]
    fn fence_generation_and_time_regression_restart_cooldown() {
        let mut authority = authority(4, 1);
        let mut state = PresentationScaleInState::default();
        state
            .observe_and_select(&authority, &BTreeSet::new(), false, 100, 50)
            .unwrap();
        let record = authority.records.get_mut("route-slot-04").unwrap();
        record.fence.operation_generation = 2;
        record.fence.operation_id = "operation:replacement".to_string();
        record.protocol_ready.as_mut().unwrap().fence = record.fence.clone();
        assert_eq!(
            state.observe_and_select(&authority, &BTreeSet::new(), false, 200, 50),
            Ok(Some("route-slot-03".to_string()))
        );
        assert_eq!(state.idle_slots["route-slot-04"].idle_since_ms, 200);
        assert_eq!(
            state.observe_and_select(&authority, &BTreeSet::new(), false, 150, 50),
            Ok(None)
        );
        assert_eq!(state.idle_slots["route-slot-03"].idle_since_ms, 150);

        authority
            .register_host_process_claim(RouteKeeperHostProcessClaim {
                host_generation: 2,
                boot_epoch: "boot:2".to_string(),
                process_identity: RecordedProcessIdentity {
                    pid: 4002,
                    start_token: "start:2".to_string(),
                    executable_path: Some("/opt/agent-browser".to_string()),
                    browser_family: None,
                },
            })
            .unwrap();
        assert_eq!(
            state.observe_and_select(&authority, &BTreeSet::new(), false, 300, 50),
            Ok(None)
        );
        assert!(state.idle_slots.is_empty());
    }

    #[test]
    fn target_floor_and_transitional_phase_block_scale_in() {
        let mut state = PresentationScaleInState::default();
        let at_target = authority(2, 1);
        state
            .observe_and_select(&at_target, &BTreeSet::new(), false, 100, 0)
            .unwrap();
        assert_eq!(
            state.observe_and_select(&at_target, &BTreeSet::new(), false, 101, 0),
            Ok(None)
        );

        let above_warm = authority(4, 1);
        let external_references = (1..=4).map(|index| format!("unknown-{index}"));
        assert_eq!(
            state.observe_and_select(&above_warm, &external_references.collect(), false, 150, 0,),
            Ok(None)
        );

        let mut transitional = authority(4, 1);
        transitional.records.get_mut("route-slot-04").unwrap().phase = RouteKeeperPhase::Stopping;
        assert_eq!(
            state.observe_and_select(&transitional, &BTreeSet::new(), false, 200, 0),
            Ok(None)
        );
        assert!(state.idle_slots.is_empty());
    }
}
