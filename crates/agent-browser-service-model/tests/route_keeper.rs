use agent_browser_service_model::{
    RouteKeeperAdoptionReceipt, RouteKeeperAuthority, RouteKeeperFence, RouteKeeperPhase,
    RouteKeeperPolicy, RouteKeeperProtocolReadyReceipt, RouteKeeperProviderState,
    RouteKeeperReconcileAction, RouteKeeperStartPriority, RouteKeeperStopDisposition,
    RouteKeeperStopReceipt,
};

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
    let mut authority = RouteKeeperAuthority::new(1).unwrap();
    let (slot_id, keeper_id, fence) =
        expect_start(&mut authority, RouteKeeperStartPriority::Minimum);
    let receipt = ready_receipt(&slot_id, &keeper_id, fence);
    authority.record_protocol_ready(receipt.clone()).unwrap();
    (authority, receipt)
}

#[test]
fn reconcile_satisfies_minimum_then_warm_target_and_becomes_idempotent() {
    let mut authority = RouteKeeperAuthority::new(7).unwrap();
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
    let mut authority = RouteKeeperAuthority::new(1).unwrap();
    let record = authority.records.get_mut("route-slot-01").unwrap();
    record.phase = RouteKeeperPhase::Ready;
    assert_eq!(
        authority.projection(),
        Err("route_keeper_ready_receipt_missing".to_string())
    );
}
