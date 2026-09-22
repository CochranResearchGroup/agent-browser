use agent_browser_service_model::{
    RecordedProcessIdentity, RouteKeeperAdoptionReceipt, RouteKeeperAuthority,
    RouteKeeperConnectionBinding, RouteKeeperConnectionCatalog, RouteKeeperFence,
    RouteKeeperHostProcessClaim, RouteKeeperPhase, RouteKeeperPolicy,
    RouteKeeperProtocolReadyReceipt, RouteKeeperProviderState, RouteKeeperReconcileAction,
    RouteKeeperStartPriority, RouteKeeperStopDisposition, RouteKeeperStopReceipt,
    RouteKeeperXrdpOwnershipWitness, ROUTE_KEEPER_AUTHORITY_SCHEMA_V4,
};

fn host_process_claim(host_generation: u64) -> RouteKeeperHostProcessClaim {
    RouteKeeperHostProcessClaim {
        host_generation,
        boot_epoch: format!("linux:boot:{host_generation}"),
        process_identity: RecordedProcessIdentity {
            pid: u32::try_from(4_000 + host_generation).unwrap(),
            start_token: format!("linux:start:{host_generation}"),
            executable_path: Some("/opt/agent-browser".to_string()),
            browser_family: None,
        },
    }
}

fn connection_catalog(maximum_slots: u32) -> RouteKeeperConnectionCatalog {
    RouteKeeperConnectionCatalog::new((1..=maximum_slots).map(|sequence| {
        RouteKeeperConnectionBinding {
            slot_id: format!("route-slot-{sequence:02}"),
            connection_key: format!("route-{sequence:02}"),
            connection_name: format!("Agent Browser Route {sequence:02}"),
            route_user: format!("agent-browser-rdp-{sequence}"),
            guacamole_connection_id: u64::from(sequence),
        }
    }))
    .unwrap()
}

fn authority(host_generation: u64) -> RouteKeeperAuthority {
    let mut authority = RouteKeeperAuthority::new(host_generation).unwrap();
    authority
        .register_host_process_claim(host_process_claim(host_generation))
        .unwrap();
    authority
        .replace_connection_catalog(connection_catalog(authority.policy.maximum_slots))
        .unwrap();
    authority
}

#[test]
fn host_process_claims_are_append_only_and_fence_active_records() {
    let mut authority = RouteKeeperAuthority::new(1).unwrap();
    assert_eq!(authority.schema_version, ROUTE_KEEPER_AUTHORITY_SCHEMA_V4);
    assert!(authority.host_process_claims.is_empty());

    let first = host_process_claim(1);
    authority
        .register_host_process_claim(first.clone())
        .unwrap();
    assert_eq!(authority.host_process_claims.get(&1), Some(&first));

    authority
        .replace_connection_catalog(connection_catalog(6))
        .unwrap();
    let (_, _, first_fence) = expect_start(&mut authority, RouteKeeperStartPriority::Minimum);
    assert_eq!(first_fence.host_generation, 1);

    let rebound = RouteKeeperHostProcessClaim {
        process_identity: RecordedProcessIdentity {
            start_token: "linux:other-process".to_string(),
            ..first.process_identity.clone()
        },
        ..first.clone()
    };
    assert_eq!(
        authority.register_host_process_claim(rebound),
        Err("route_keeper_host_process_claim_rebound:1".to_string())
    );

    let second = host_process_claim(2);
    authority
        .register_host_process_claim(second.clone())
        .unwrap();
    assert_eq!(authority.host_process_claims.get(&1), Some(&first));
    assert_eq!(authority.host_process_claims.get(&2), Some(&second));
    assert_eq!(
        authority.records["route-slot-01"].fence.host_generation, 1,
        "an active predecessor remains bound to its original process"
    );
    assert!(authority
        .records
        .values()
        .filter(|record| record.phase == RouteKeeperPhase::Absent)
        .all(|record| record.fence.host_generation == 2));
}

#[test]
fn empty_catalog_blocks_start_and_catalog_digest_fences_every_action() {
    let mut empty = RouteKeeperAuthority::new(1).unwrap();
    empty
        .register_host_process_claim(host_process_claim(1))
        .unwrap();
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
            route_user: "agent-browser-rdp-1".to_string(),
            guacamole_connection_id: 1,
        },
        RouteKeeperConnectionBinding {
            slot_id: "route-slot-02".to_string(),
            connection_key: "route".to_string(),
            connection_name: "Route 2".to_string(),
            route_user: "agent-browser-rdp-2".to_string(),
            guacamole_connection_id: 2,
        },
    ]);
    assert_eq!(
        duplicate,
        Err("route_keeper_connection_catalog_identity_duplicate".to_string())
    );
    let duplicate_route_user = RouteKeeperConnectionCatalog::new([
        RouteKeeperConnectionBinding {
            slot_id: "route-slot-01".to_string(),
            connection_key: "route-01".to_string(),
            connection_name: "Route 1".to_string(),
            route_user: "agent-browser-rdp-1".to_string(),
            guacamole_connection_id: 1,
        },
        RouteKeeperConnectionBinding {
            slot_id: "route-slot-02".to_string(),
            connection_key: "route-02".to_string(),
            connection_name: "Route 2".to_string(),
            route_user: "agent-browser-rdp-1".to_string(),
            guacamole_connection_id: 2,
        },
    ]);
    assert_eq!(
        duplicate_route_user,
        Err("route_keeper_connection_catalog_identity_duplicate".to_string())
    );

    let mut authority = RouteKeeperAuthority::new(1).unwrap();
    let unknown = RouteKeeperConnectionCatalog::new([RouteKeeperConnectionBinding {
        slot_id: "route-slot-99".to_string(),
        connection_key: "route-99".to_string(),
        connection_name: "Route 99".to_string(),
        route_user: "agent-browser-rdp-99".to_string(),
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
    let xrdp_session_id = format!("xrdp-{slot_id}");
    let display_name = format!(":{}", slot_id.trim_start_matches("route-slot-"));
    RouteKeeperProtocolReadyReceipt {
        slot_id: slot_id.to_string(),
        keeper_id: keeper_id.to_string(),
        fence,
        guacamole_connection_uuid: format!("connection-{slot_id}"),
        xrdp_session_id: xrdp_session_id.clone(),
        display_name: display_name.clone(),
        xrdp_ownership: Some(RouteKeeperXrdpOwnershipWitness {
            schema_version: "agent-browser.route-keeper-xrdp-ownership.v1".to_string(),
            boot_id: "boot-fixture".to_string(),
            route_user: "agent-browser-rdp-1".to_string(),
            route_uid: 2001,
            session_id: xrdp_session_id.clone(),
            session_service: "xrdp-sesman".to_string(),
            session_scope: format!("session-{xrdp_session_id}.scope"),
            scope_invocation_id: "invocation-fixture".to_string(),
            cgroup_path: format!("/user.slice/user-2001.slice/session-{xrdp_session_id}.scope"),
            cgroup_device: 28,
            cgroup_inode: 1001,
            leader_pid: 4101,
            leader_start_ticks: 5101,
            x_server_pid: 4102,
            x_server_start_ticks: 5102,
            display_name,
            x11_socket_inode: 6101,
        }),
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
fn ready_handoff_binding_joins_exact_catalog_keeper_and_xrdp_evidence() {
    let mut authority = RouteKeeperAuthority::new(1).unwrap();
    authority
        .register_host_process_claim(host_process_claim(1))
        .unwrap();
    authority
        .replace_connection_catalog(
            RouteKeeperConnectionCatalog::with_provider_urls(
                "http://127.0.0.1:8193/guacamole/",
                "https://dashboard.example/operator",
                [RouteKeeperConnectionBinding {
                    slot_id: "route-slot-01".to_string(),
                    connection_key: "route-01".to_string(),
                    connection_name: "Agent Browser Route 01".to_string(),
                    route_user: "agent-browser-rdp-1".to_string(),
                    guacamole_connection_id: 1,
                }],
            )
            .unwrap(),
        )
        .unwrap();
    let (slot_id, keeper_id, fence) =
        expect_start(&mut authority, RouteKeeperStartPriority::Minimum);
    let receipt = ready_receipt(&slot_id, &keeper_id, fence.clone());
    authority.record_protocol_ready(receipt.clone()).unwrap();

    let binding = authority
        .ready_handoff_binding("route-slot-01", ":01")
        .unwrap();
    assert_eq!(binding.slot_id, "route-slot-01");
    assert_eq!(binding.keeper_id, keeper_id);
    assert_eq!(binding.fence, fence);
    assert_eq!(binding.route_user, "agent-browser-rdp-1");
    assert_eq!(binding.display_name, ":01");
    assert_eq!(binding.guacamole_connection_id, 1);
    assert_eq!(
        binding.guacamole_connection_uuid,
        receipt.guacamole_connection_uuid
    );
    assert_eq!(
        binding.public_operator_url,
        "https://dashboard.example/operator"
    );
}

#[test]
fn handoff_binding_rejects_non_ready_display_and_missing_public_origin() {
    let mut unconfigured = authority(1);
    let (slot_id, keeper_id, fence) =
        expect_start(&mut unconfigured, RouteKeeperStartPriority::Minimum);
    assert_eq!(
        unconfigured.ready_handoff_binding(&slot_id, ":01"),
        Err("route_keeper_handoff_not_ready".to_string())
    );
    unconfigured
        .record_protocol_ready(ready_receipt(&slot_id, &keeper_id, fence))
        .unwrap();
    assert_eq!(
        unconfigured.ready_handoff_binding(&slot_id, ":99"),
        Err("route_keeper_handoff_display_mismatch".to_string())
    );
    assert_eq!(
        unconfigured.ready_handoff_binding(&slot_id, ":01"),
        Err("route_keeper_handoff_public_operator_unconfigured".to_string())
    );
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
    authority
        .register_host_process_claim(host_process_claim(2))
        .unwrap();

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
    let mut bypass_ready = adopted_ready.clone();
    bypass_ready.guacamole_connection_uuid = "different-connection".to_string();
    bypass_ready.xrdp_session_id = "different-xrdp".to_string();
    bypass_ready.display_name = ":99".to_string();
    let bypass_ownership = bypass_ready.xrdp_ownership.as_mut().unwrap();
    bypass_ownership.session_id = "different-xrdp".to_string();
    bypass_ownership.session_scope = "session-different-xrdp.scope".to_string();
    bypass_ownership.cgroup_path =
        "/user.slice/user-2001.slice/session-different-xrdp.scope".to_string();
    bypass_ownership.display_name = ":99".to_string();
    assert_eq!(
        authority.record_protocol_ready(bypass_ready),
        Err("route_keeper_phase_not_observing".to_string())
    );
    authority
        .adopt(RouteKeeperAdoptionReceipt {
            previous_host_generation: 1,
            previous_guacamole_connection_uuid: original_ready.guacamole_connection_uuid.clone(),
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
fn adoption_terminal_preserves_predecessor_and_rejects_stale_occurrences() {
    let (mut authority, predecessor) = make_one_ready();
    authority
        .record_disconnect(
            &predecessor.slot_id,
            &predecessor.fence,
            &predecessor.guacamole_connection_uuid,
        )
        .unwrap();
    authority
        .register_host_process_claim(host_process_claim(2))
        .unwrap();
    let adoption_fence = match authority.begin_adoption(&predecessor.slot_id, 2).unwrap() {
        RouteKeeperReconcileAction::Adopt { fence, .. } => fence,
        other => panic!("expected adoption action, got {other:?}"),
    };

    authority
        .record_adoption_terminated(
            &predecessor.slot_id,
            &adoption_fence,
            Some("successor-occurrence"),
        )
        .unwrap();
    let failed = &authority.records[&predecessor.slot_id];
    assert_eq!(failed.phase, RouteKeeperPhase::RecoveryFailed);
    assert_eq!(failed.protocol_ready, Some(predecessor.clone()));
    assert!(failed.adoption.is_none());
    assert_eq!(authority.projection().unwrap().ready_count, 0);
    assert_eq!(
        authority.projection().unwrap().state,
        RouteKeeperProviderState::Degraded
    );

    let before_stale = authority.clone();
    assert_eq!(
        authority.record_adoption_terminated(
            &predecessor.slot_id,
            &predecessor.fence,
            Some(&predecessor.guacamole_connection_uuid),
        ),
        Err("route_keeper_generation_stale:1:2".to_string())
    );
    assert_eq!(authority, before_stale);

    let mut adopting = before_stale;
    let next_adoption_fence = match adopting.next_reconcile_action().unwrap() {
        RouteKeeperReconcileAction::Start { fence, .. } => fence,
        other => panic!("expected recovery start action, got {other:?}"),
    };
    assert_eq!(
        adopting.records[&predecessor.slot_id].phase,
        RouteKeeperPhase::Starting
    );
    assert_eq!(
        adopting.records[&predecessor.slot_id].protocol_ready,
        Some(predecessor)
    );
    assert!(next_adoption_fence.operation_generation > adoption_fence.operation_generation);
}

#[test]
fn adoption_terminal_rejects_predecessor_uuid_without_mutating_adopting_state() {
    let (mut authority, predecessor) = make_one_ready();
    authority
        .record_disconnect(
            &predecessor.slot_id,
            &predecessor.fence,
            &predecessor.guacamole_connection_uuid,
        )
        .unwrap();
    authority
        .register_host_process_claim(host_process_claim(2))
        .unwrap();
    let adoption_fence = match authority.begin_adoption(&predecessor.slot_id, 2).unwrap() {
        RouteKeeperReconcileAction::Adopt { fence, .. } => fence,
        other => panic!("expected adoption action, got {other:?}"),
    };
    let before = authority.clone();
    assert_eq!(
        authority.record_adoption_terminated(
            &predecessor.slot_id,
            &adoption_fence,
            Some(&predecessor.guacamole_connection_uuid),
        ),
        Err("route_keeper_connection_identity_mismatch".to_string())
    );
    assert_eq!(authority, before);
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

#[test]
fn legacy_ready_receipt_without_exact_xrdp_ownership_fails_closed() {
    let (mut authority, _) = make_one_ready();
    authority
        .records
        .get_mut("route-slot-01")
        .unwrap()
        .protocol_ready
        .as_mut()
        .unwrap()
        .xrdp_ownership = None;

    assert_eq!(
        authority.projection(),
        Err("route_keeper_ready_xrdp_ownership_missing".to_string())
    );
}

#[test]
fn interrupted_adoption_refences_only_exact_operation_and_retains_source() {
    let (mut authority, original) = make_one_ready();
    authority
        .record_disconnect(
            &original.slot_id,
            &original.fence,
            &original.guacamole_connection_uuid,
        )
        .unwrap();
    authority
        .register_host_process_claim(host_process_claim(2))
        .unwrap();
    authority.begin_adoption(&original.slot_id, 2).unwrap();
    let interrupted = authority.records[&original.slot_id].fence.clone();
    authority
        .register_host_process_claim(host_process_claim(3))
        .unwrap();
    let before = authority.clone();
    let mut changed = interrupted.clone();
    changed.operation_id = "unrelated-operation".to_string();
    assert!(authority
        .refence_interrupted_adoption(&original.slot_id, &changed, 3)
        .is_err());
    assert!(authority
        .refence_interrupted_adoption(&original.slot_id, &interrupted, 2)
        .is_err());
    assert_eq!(authority, before);
    let action = authority
        .refence_interrupted_adoption(&original.slot_id, &interrupted, 3)
        .unwrap();
    assert_eq!(authority.next_reconcile_action().unwrap(), action);
    let current = authority.records[&original.slot_id].fence.clone();
    assert_eq!(current.host_generation, 3);
    assert_eq!(
        current.operation_generation,
        interrupted.operation_generation
    );
    assert_eq!(
        authority.records[&original.slot_id].protocol_ready.as_ref(),
        Some(&original)
    );
    let refenced = authority.clone();
    assert!(authority
        .record_adoption_terminated(&original.slot_id, &interrupted, None)
        .is_err());
    assert_eq!(authority, refenced);
    assert!(authority
        .adopt(RouteKeeperAdoptionReceipt {
            previous_host_generation: original.fence.host_generation,
            previous_guacamole_connection_uuid: original.guacamole_connection_uuid.clone(),
            ready: RouteKeeperProtocolReadyReceipt {
                fence: interrupted.clone(),
                guacamole_connection_uuid: "stale-second-host-transport".to_string(),
                ..original.clone()
            },
            adopted_at: "2026-09-21T21:59:00Z".to_string(),
        })
        .is_err());
    assert_eq!(authority, refenced);
    assert!(authority
        .refence_interrupted_adoption(&original.slot_id, &interrupted, 3)
        .is_err());
    authority
        .adopt(RouteKeeperAdoptionReceipt {
            previous_host_generation: original.fence.host_generation,
            previous_guacamole_connection_uuid: original.guacamole_connection_uuid.clone(),
            ready: RouteKeeperProtocolReadyReceipt {
                fence: current,
                guacamole_connection_uuid: "third-host-transport".to_string(),
                ..original.clone()
            },
            adopted_at: "2026-09-21T22:00:00Z".to_string(),
        })
        .unwrap();
    assert_eq!(
        authority.records[&original.slot_id].phase,
        RouteKeeperPhase::Ready
    );
    assert_eq!(authority.host_process_claims.len(), 3);
}
