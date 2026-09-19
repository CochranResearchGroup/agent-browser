use agent_browser_service_model::{
    RouteKeeperAdoptionReceipt, RouteKeeperAuthority, RouteKeeperConnectionBinding,
    RouteKeeperConnectionCatalog, RouteKeeperFence, RouteKeeperPhase, RouteKeeperPolicy,
    RouteKeeperProtocolReadyReceipt, RouteKeeperProviderState, RouteKeeperReconcileAction,
    RouteKeeperStartPriority, RouteKeeperStopDisposition, RouteKeeperStopReceipt,
};

fn connection_catalog(maximum_slots: u32) -> RouteKeeperConnectionCatalog {
    RouteKeeperConnectionCatalog::new((1..=maximum_slots).map(|sequence| {
        RouteKeeperConnectionBinding {
            slot_id: format!("route-slot-{sequence:02}"),
            connection_key: format!("route-{sequence:02}"),
            connection_name: format!("Agent Browser Route {sequence:02}"),
            guacamole_connection_id: u64::from(sequence),
        }
    }))
    .unwrap()
}

fn authority(host_generation: u64) -> RouteKeeperAuthority {
    let mut authority = RouteKeeperAuthority::new(host_generation).unwrap();
    authority
        .replace_connection_catalog(connection_catalog(authority.policy.maximum_slots))
        .unwrap();
    authority
}

#[test]
fn empty_catalog_blocks_start_and_catalog_digest_fences_every_action() {
    let mut empty = RouteKeeperAuthority::new(1).unwrap();
    assert_eq!(
        empty.next_reconcile_action(),
        Err("route_keeper_connection_catalog_empty".to_string())
    );

    let catalog = connection_catalog(6);
    let digest = catalog.digest().unwrap();
    empty.replace_connection_catalog(catalog.clone()).unwrap();
    let (_, _, fence) = expect_start(&mut empty, RouteKeeperStartPriority::Minimum);
    assert_eq!(fence.connection_catalog_digest, digest);
    assert_eq!(
        empty.replace_connection_catalog(RouteKeeperConnectionCatalog::default()),
        Err("route_keeper_connection_catalog_active".to_string())
    );

    let reversed = RouteKeeperConnectionCatalog::new(
        catalog.bindings.values().rev().cloned().collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(catalog.digest().unwrap(), reversed.digest().unwrap());
}

#[test]
fn catalog_rejects_duplicate_provider_identities_and_unknown_slots() {
    let duplicate = RouteKeeperConnectionCatalog::new([
        RouteKeeperConnectionBinding {
            slot_id: "route-slot-01".to_string(),
            connection_key: "route".to_string(),
            connection_name: "Route 1".to_string(),
            guacamole_connection_id: 1,
        },
        RouteKeeperConnectionBinding {
            slot_id: "route-slot-02".to_string(),
            connection_key: "route".to_string(),
            connection_name: "Route 2".to_string(),
            guacamole_connection_id: 2,
        },
    ]);
    assert_eq!(
        duplicate,
        Err("route_keeper_connection_catalog_identity_duplicate".to_string())
    );

    let mut authority = RouteKeeperAuthority::new(1).unwrap();
    let unknown = RouteKeeperConnectionCatalog::new([RouteKeeperConnectionBinding {
        slot_id: "route-slot-99".to_string(),
        connection_key: "route-99".to_string(),
        connection_name: "Route 99".to_string(),
        guacamole_connection_id: 99,
    }])
    .unwrap();
    assert_eq!(
        authority.replace_connection_catalog(unknown),
        Err("route_keeper_connection_catalog_slot_unknown".to_string())
    );
}

#[test]
fn v1_upgrade_refuses_to_manufacture_catalog_provenance_for_active_keeper() {
    let (mut active, _) = make_one_ready();
    active.schema_version =
        agent_browser_service_model::ROUTE_KEEPER_AUTHORITY_SCHEMA_V1.to_string();
    assert_eq!(
        active.upgrade_from_v1(),
        Err("route_keeper_v1_active_migration_unproven".to_string())
    );
}

fn expect_start(
    authority: &mut RouteKeeperAuthority,
    expected_priority: RouteKeeperStartPriority,
) -> (String, String, RouteKeeperFence) {
    match authority.next_reconcile_action().unwrap() {
        RouteKeeperReconcileAction::Start {
            slot_id,
            keeper_id,
            fence,
            priority,
        } => {
            assert_eq!(priority, expected_priority);
            (slot_id, keeper_id, fence)
        }
        other => panic!("expected start action, got {other:?}"),
    }
}

fn ready_receipt(
    slot_id: &str,
    keeper_id: &str,
    fence: RouteKeeperFence,
) -> RouteKeeperProtocolReadyReceipt {
    RouteKeeperProtocolReadyReceipt {
        slot_id: slot_id.to_string(),
        keeper_id: keeper_id.to_string(),
        fence,
        guacamole_connection_uuid: format!("connection-{slot_id}"),
        xrdp_session_id: format!("xrdp-{slot_id}"),
        display_name: format!(":{}", slot_id.trim_start_matches("route-slot-")),
        observed_at: "2026-09-19T12:00:00Z".to_string(),
    }
}

fn make_one_ready() -> (RouteKeeperAuthority, RouteKeeperProtocolReadyReceipt) {
    let mut authority = authority(1);
    let (slot_id, keeper_id, fence) =
        expect_start(&mut authority, RouteKeeperStartPriority::Minimum);
    let receipt = ready_receipt(&slot_id, &keeper_id, fence);
    authority.record_protocol_ready(receipt.clone()).unwrap();
    (authority, receipt)
}

#[test]
fn reconcile_satisfies_minimum_then_warm_target_and_becomes_idempotent() {
    let mut authority = authority(7);
    assert_eq!(authority.policy.minimum_ready, 1);
    assert_eq!(authority.policy.warm_target, 4);
    assert_eq!(authority.policy.maximum_slots, 6);
    assert_eq!(authority.records.len(), 6);
    assert_eq!(
        authority.projection().unwrap().state,
        RouteKeeperProviderState::Starting
    );

    let mut started_slots = Vec::new();
    for index in 0..4 {
        let priority = if index == 0 {
            RouteKeeperStartPriority::Minimum
        } else {
            RouteKeeperStartPriority::Warm
        };
        let (slot_id, keeper_id, fence) = expect_start(&mut authority, priority);
        assert_eq!(fence.host_generation, 7);
        assert_eq!(
            authority.next_reconcile_action().unwrap(),
            RouteKeeperReconcileAction::Observe {
                slot_id: slot_id.clone(),
                keeper_id: keeper_id.clone(),
                fence: fence.clone(),
            }
        );
        authority.record_observing(&slot_id, &fence).unwrap();
        authority
            .record_protocol_ready(ready_receipt(&slot_id, &keeper_id, fence))
            .unwrap();
        started_slots.push(slot_id);

        let projection = authority.projection().unwrap();
        assert_eq!(projection.ready_count, index + 1);
        assert!(projection.minimum_satisfied);
        assert_eq!(projection.warm_target_satisfied, index == 3);
    }

    assert_eq!(
        started_slots,
        [
            "route-slot-01",
            "route-slot-02",
            "route-slot-03",
            "route-slot-04"
        ]
    );
    assert_eq!(
        authority.next_reconcile_action().unwrap(),
        RouteKeeperReconcileAction::Noop
    );
    assert_eq!(
        authority.next_reconcile_action().unwrap(),
        RouteKeeperReconcileAction::Noop
    );
    let projection = authority.projection().unwrap();
    assert_eq!(projection.state, RouteKeeperProviderState::Ready);
    assert_eq!(projection.keeper_count, 4);
    assert_eq!(projection.ready_count, 4);

    let custom = RouteKeeperAuthority::with_policy(
        9,
        RouteKeeperPolicy {
            minimum_ready: 2,
            warm_target: 3,
            maximum_slots: 5,
        },
    )
    .unwrap();
    assert_eq!(custom.records.len(), 5);
    assert_eq!(custom.projection().unwrap().desired_minimum, 2);
    assert_eq!(
        RouteKeeperAuthority::with_policy(
            9,
            RouteKeeperPolicy {
                minimum_ready: 0,
                warm_target: 3,
                maximum_slots: 5,
            },
        ),
        Err("route_keeper_policy_invalid".to_string())
    );
}

#[test]
fn disconnect_restarts_and_exact_adoption_fences_stale_generation() {
    let (mut authority, original_ready) = make_one_ready();
    authority
        .record_disconnect(
            &original_ready.slot_id,
            &original_ready.fence,
            &original_ready.guacamole_connection_uuid,
        )
        .unwrap();
    assert_eq!(
        authority.records[&original_ready.slot_id].phase,
        RouteKeeperPhase::Degraded
    );

    let adoption_action = authority
        .begin_adoption(&original_ready.slot_id, 2)
        .unwrap();
    let (slot_id, keeper_id, adoption_fence) = match adoption_action.clone() {
        RouteKeeperReconcileAction::Adopt {
            slot_id,
            keeper_id,
            fence,
            previous_host_generation,
        } => {
            assert_eq!(previous_host_generation, 1);
            (slot_id, keeper_id, fence)
        }
        other => panic!("expected adopt action, got {other:?}"),
    };
    assert_eq!(slot_id, original_ready.slot_id);
    assert_eq!(keeper_id, original_ready.keeper_id);
    assert_eq!(adoption_fence.host_generation, 2);
    assert!(adoption_fence.operation_generation > original_ready.fence.operation_generation);
    assert_eq!(authority.next_reconcile_action().unwrap(), adoption_action);

    let adopted_ready = RouteKeeperProtocolReadyReceipt {
        fence: adoption_fence.clone(),
        observed_at: "2026-09-19T12:01:00Z".to_string(),
        ..original_ready.clone()
    };
    let bypass_ready = RouteKeeperProtocolReadyReceipt {
        guacamole_connection_uuid: "different-connection".to_string(),
        xrdp_session_id: "different-xrdp".to_string(),
        display_name: ":99".to_string(),
        ..adopted_ready.clone()
    };
    assert_eq!(
        authority.record_protocol_ready(bypass_ready),
        Err("route_keeper_phase_not_observing".to_string())
    );
    authority
        .adopt(RouteKeeperAdoptionReceipt {
            previous_host_generation: 1,
            ready: adopted_ready.clone(),
            adopted_at: "2026-09-19T12:01:01Z".to_string(),
        })
        .unwrap();
    assert_eq!(authority.records[&slot_id].phase, RouteKeeperPhase::Ready);
    assert_eq!(authority.records[&slot_id].fence, adopted_ready.fence);

    assert_eq!(
        authority.record_protocol_ready(original_ready),
        Err("route_keeper_generation_stale:1:2".to_string())
    );
    assert_eq!(
        authority.records[&slot_id].protocol_ready,
        Some(adopted_ready)
    );
}

#[test]
fn pre_ready_terminal_returns_exact_attempt_to_absent_for_new_fence() {
    let mut authority = authority(3);
    let (slot_id, keeper_id, first_fence) =
        expect_start(&mut authority, RouteKeeperStartPriority::Minimum);
    authority.record_observing(&slot_id, &first_fence).unwrap();
    authority
        .record_start_terminated(&slot_id, &first_fence)
        .unwrap();
    assert_eq!(authority.records[&slot_id].phase, RouteKeeperPhase::Absent);
    assert!(authority.records[&slot_id].protocol_ready.is_none());

    let (next_slot, next_keeper, next_fence) =
        expect_start(&mut authority, RouteKeeperStartPriority::Minimum);
    assert_eq!(next_slot, slot_id);
    assert_eq!(next_keeper, keeper_id);
    assert_eq!(next_fence.host_generation, first_fence.host_generation);
    assert!(next_fence.operation_generation > first_fence.operation_generation);
    assert_ne!(next_fence.operation_id, first_fence.operation_id);
    assert_eq!(
        authority.record_start_terminated(&slot_id, &first_fence),
        Err("route_keeper_stale_observation".to_string())
    );
}

#[test]
fn failed_recovery_retains_prior_protocol_evidence_for_next_fenced_attempt() {
    let (mut authority, original_ready) = make_one_ready();
    authority
        .record_disconnect(
            &original_ready.slot_id,
            &original_ready.fence,
            &original_ready.guacamole_connection_uuid,
        )
        .unwrap();
    let (_, _, recovery_fence) = expect_start(&mut authority, RouteKeeperStartPriority::Recovery);
    authority
        .record_start_terminated(&original_ready.slot_id, &recovery_fence)
        .unwrap();
    let failed = &authority.records[&original_ready.slot_id];
    assert_eq!(failed.phase, RouteKeeperPhase::RecoveryFailed);
    assert_eq!(failed.protocol_ready, Some(original_ready.clone()));
    assert_eq!(failed.fence, recovery_fence);
    assert_eq!(
        authority.projection().unwrap().state,
        RouteKeeperProviderState::Degraded
    );

    let (slot_id, keeper_id, next_fence) =
        expect_start(&mut authority, RouteKeeperStartPriority::Recovery);
    assert_eq!(slot_id, original_ready.slot_id);
    assert_eq!(keeper_id, original_ready.keeper_id);
    assert!(next_fence.operation_generation > recovery_fence.operation_generation);
    assert_eq!(
        authority.records[&slot_id].protocol_ready,
        Some(original_ready)
    );
}

#[test]
fn stop_requires_exact_receipt_and_unproven_ownership_is_quarantined() {
    let (mut exact_authority, exact_ready) = make_one_ready();
    let stop_action = exact_authority
        .begin_stop(&exact_ready.slot_id, &exact_ready.fence)
        .unwrap();
    assert_eq!(
        stop_action,
        RouteKeeperReconcileAction::Stop {
            slot_id: exact_ready.slot_id.clone(),
            keeper_id: exact_ready.keeper_id.clone(),
            fence: exact_ready.fence.clone(),
        }
    );
    let exact_stop = RouteKeeperStopReceipt {
        slot_id: exact_ready.slot_id.clone(),
        keeper_id: exact_ready.keeper_id.clone(),
        fence: exact_ready.fence.clone(),
        guacamole_connection_uuid: Some(exact_ready.guacamole_connection_uuid.clone()),
        xrdp_session_id: Some(exact_ready.xrdp_session_id.clone()),
        stopped_at: "2026-09-19T12:02:00Z".to_string(),
    };
    assert_eq!(
        exact_authority.record_stopped(exact_stop.clone()).unwrap(),
        RouteKeeperStopDisposition::Stopped
    );
    let exact_record = &exact_authority.records[&exact_ready.slot_id];
    assert_eq!(exact_record.phase, RouteKeeperPhase::Absent);
    assert_eq!(exact_record.last_stop, Some(exact_stop));
    assert!(exact_record.protocol_ready.is_none());

    let (mut foreign_authority, foreign_ready) = make_one_ready();
    foreign_authority
        .begin_stop(&foreign_ready.slot_id, &foreign_ready.fence)
        .unwrap();
    let mismatched_stop = RouteKeeperStopReceipt {
        keeper_id: "foreign-keeper".to_string(),
        slot_id: foreign_ready.slot_id.clone(),
        fence: foreign_ready.fence.clone(),
        guacamole_connection_uuid: Some(foreign_ready.guacamole_connection_uuid.clone()),
        xrdp_session_id: Some(foreign_ready.xrdp_session_id.clone()),
        stopped_at: "2026-09-19T12:03:00Z".to_string(),
    };
    let disposition = foreign_authority.record_stopped(mismatched_stop).unwrap();
    let foreign_record = &foreign_authority.records[&foreign_ready.slot_id];
    assert_eq!(foreign_record.phase, RouteKeeperPhase::Quarantined);
    assert_eq!(foreign_record.protocol_ready, Some(foreign_ready));
    let obligation = foreign_record.cleanup_obligation.as_ref().unwrap();
    assert_eq!(
        disposition,
        RouteKeeperStopDisposition::Quarantined {
            obligation: obligation.clone()
        }
    );
    assert!(obligation
        .preserved_observed_keeper_id
        .starts_with("keeper=foreign-keeper;"));
    assert_eq!(obligation.reason, "route_keeper_stop_observation_unproven");
    assert!(matches!(
        foreign_authority.next_reconcile_action().unwrap(),
        RouteKeeperReconcileAction::Start {
            slot_id,
            priority: RouteKeeperStartPriority::Minimum,
            ..
        } if slot_id == "route-slot-02"
    ));
}

#[test]
fn invalid_ready_record_cannot_project_or_persist_false_capacity() {
    let mut authority = authority(1);
    let record = authority.records.get_mut("route-slot-01").unwrap();
    record.phase = RouteKeeperPhase::Ready;
    assert_eq!(
        authority.projection(),
        Err("route_keeper_ready_receipt_missing".to_string())
    );
}
