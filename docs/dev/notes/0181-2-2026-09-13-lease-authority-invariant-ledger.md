# Plan 0181 P0 | Lease-authority invariant ledger

Date: 2026-09-13  
Product lane: PL-PLATFORM  
Disposition: active-input  
Owning plan: Plan 0181  
Issue: #99  
Related lane: P144

## Freeze

The current authority population is 106 Rust tests. Freeze 99 tests for
movement into `agent-browser-lease-authority` and retain seven in the CLI as
adapter-boundary tests. This is a test-placement ledger, not evidence that the
extraction or any provider/runtime gate is complete.

## Final source-placement reconciliation

The extracted source preserves all 106 frozen authority and protocol test
identities. The final security boundary places 103 of those tests in
`agent-browser-lease-authority` and retains three in the CLI adapter. Five new
principal and profile-identity tests bring the crate total to 108; they do not
replace any frozen invariant.

The three retained CLI tests are:

- `service_state_round_trips_active_claims_and_history_separately`;
- `effect_boundary_rejects_diverged_owner_principal_binding`; and
- `repository_boundary_atomically_admits_exactly_one_contender`.

Four initially retained tests moved to the crate because keeping their existing
private-key fixtures in the CLI would expose signing keys, proof fields,
authority maps, or raw plan issuance across the crate boundary:

- `public_signing_oracle_requires_the_exact_private_profile_capability`;
- `exact_holder_release_fences_authority_and_replays_terminal_receipt`;
- `strict_controller_recovery_advances_the_fence_and_replays_after_controller_revocation`; and
- `administrative_revocation_is_exact_holder_independent_and_replayable`.

Those four now exercise typed authenticated kernel operations with private
signing fixtures inside the crate. Their placement changed; their test names and
security invariants did not. Final baseline accounting is `103 + 3 = 106`.

Frozen seams:

- Move pure principal registry, authentication, rotation, and digest mechanics.
  Retain session/tab continuity and migration in the CLI adapter.
- Retain `*_in_repository`, `*_for_state`, and runtime-owner/profile-resolution
  adapters. Move the authority decisions they exercise when they are kernel
  pure.
- Move profile-name validation and canonical profile-identity digest mechanics;
  retain `resolve_profile`.
- Protocol implementation imports switch to the crate; no CLI/native import
  may point upward from the crate.

## Initial retained CLI adapter classification (7)

These remain in the CLI because they prove product integration boundaries that
cannot be owned by the kernel crate:

- `public_signing_oracle_requires_the_exact_private_profile_capability` — the
  CLI adapter supplies the private profile capability boundary and must prove
  the public-signing oracle cannot bypass it.
- `service_state_round_trips_active_claims_and_history_separately` — Service
  State serialization and projection compatibility belong to the CLI adapter,
  not the authority kernel.
- `effect_boundary_rejects_diverged_owner_principal_binding` — the adapter
  joins effect requests to the CLI owner/principal evidence before admission.
- `repository_boundary_atomically_admits_exactly_one_contender` — repository
  transaction and file-lock admission are CLI persistence responsibilities.
- `exact_holder_release_fences_authority_and_replays_terminal_receipt` — the
  CLI release adapter binds the request to the repository/runtime holder and
  its terminal receipt.
- `strict_controller_recovery_advances_the_fence_and_replays_after_controller_revocation` —
  controller recovery crosses CLI persistence and runtime-owner evidence.
- `administrative_revocation_is_exact_holder_independent_and_replayable` —
  administrative revocation is routed through the CLI repository/service
  adapter and its replay contract.

## Initial moved classification (99)

### `cli/src/native/service_lease_authority.rs` (15)

- `signing_key_file_is_private_stable_and_never_serialized_into_a_bearer`
- `selected_trust_generation_is_atomic_rotatable_and_stale_safe`
- `verifier_keyring_preserves_bounded_old_proofs_and_rejects_epoch_rollback`
- `terminal_events_never_block_atomic_acquisition`
- `acquisition_claim_revision_compare_and_swap_has_one_winner`
- `unrelated_authority_activity_cannot_create_a_profile_acquisition_conflict`
- `strict_claim_requires_first_class_recovery_metadata`
- `caller_cannot_create_an_unbounded_claim`
- `exhausted_fencing_counter_fails_before_authority_mutation`
- `unsupported_authority_schema_fails_before_acquisition_mutation`
- `unsupported_receipt_schema_cannot_be_replayed`
- `acquisition_receipt_replay_after_expiry_grants_no_new_authority`
- `effect_authorization_is_redacted_and_expires_independently`
- `same_principal_new_operation_rejoins_current_claim`
- `strict_claim_cannot_implicitly_rejoin`

### `cli/src/native/service_lease_authority/protocol.rs` (56)

- `browser_adoption_refuses_while_original_executor_is_current`
- `browser_adoption_prepares_one_candidate_after_executor_disappears`
- `browser_adoption_refuses_a_surviving_effect_channel`
- `browser_adoption_commit_atomically_advances_owner_generation_and_executor`
- `browser_adoption_uncertainty_terminalizes_without_transferring_owner`
- `uncertain_browser_adoption_blocks_a_second_candidate_until_reconciliation`
- `expired_prepared_adoption_without_effect_custody_cannot_block_a_fresh_candidate`
- `expired_uncertain_adoption_without_effect_custody_cannot_block_a_fresh_candidate`
- `browser_adoption_prepare_replay_returns_receipt_without_reissuing_physical_authority`
- `browser_adoption_requests_reject_caller_owned_runtime_observations`
- `protocol_rejects_generic_signing_and_state_mutation_oracles`
- `framed_transport_rejects_an_oversized_request_before_reading_its_payload`
- `typed_dispatcher_returns_only_a_nonce_bound_service_challenge`
- `framed_service_returns_a_typed_error_for_a_generic_signing_oracle`
- `administrative_dispatch_requires_a_kernel_authenticated_root_peer`
- `administrative_revoke_plan_and_apply_are_authority_timed_durable_and_replayable`
- `authority_time_floor_survives_restart_and_cannot_move_backward`
- `caller_time_and_stale_claim_revision_cannot_drive_administrative_revoke`
- `protected_service_rejects_non_root_before_consulting_installed_paths`
- `protected_service_accepts_only_a_banked_root_generation_path`
- `protected_service_accepts_only_its_exact_systemd_socket_activation`
- `protected_service_store_open_never_bootstraps_missing_state`
- `acquire_request_is_typed_and_redacts_the_profile_capability`
- `profile_enrollment_request_has_no_caller_owned_principal_or_physical_identity`
- `release_request_has_no_caller_owned_time_identity_or_authorization`
- `effect_request_has_no_caller_owned_time_identity_or_proof`
- `effect_receipt_keys_are_domain_principal_resource_and_action_scoped`
- `effect_completion_request_has_no_caller_owned_time_executor_or_authorization`
- `protected_profile_enrollment_is_uid_bound_durable_and_acquisition_ready`
- `effect_executor_kernel_identity_is_peer_start_and_cgroup_bound`
- `browser_process_kernel_identity_is_uid_pid_start_and_cgroup_bound`
- `browser_launch_completion_derives_an_exact_direct_child_process_identity`
- `browser_owner_executor_observation_rejects_replaced_process_identity`
- `browser_adoption_observes_exact_profile_listener_lock_and_candidate_socket`
- `profile_enrollment_path_identity_is_peer_owned_and_alias_canonical`
- `acquire_request_rejects_caller_owned_time_and_owner_evidence`
- `strict_recovery_plan_is_controller_authenticated_durable_and_exact`
- `recovery_protocol_rejects_caller_owned_time`
- `authenticated_acquire_derives_holder_identity_inside_the_kernel`
- `protected_state_round_trip_preserves_replay_without_persisting_the_bearer`
- `protected_state_persists_exact_administrative_intent_but_projection_does_not`
- `authenticated_acquire_cannot_invent_an_unregistered_profile_resource`
- `protected_state_rejects_two_profile_ids_for_one_physical_identity`
- `protected_state_rejects_noncanonical_physical_identity_digest`
- `protected_state_cannot_prove_its_own_epoch_after_rollback`
- `protected_state_rejects_owner_for_unregistered_physical_resource`
- `protected_state_rejects_owner_binding_without_registered_capability`
- `protected_owner_registry_cannot_serialize_runtime_lifecycle_history`
- `publication_crash_before_selector_keeps_prior_generation_selected`
- `stale_publisher_cannot_reselect_an_older_valid_generation`
- `only_mutation_load_can_publish_without_rewriting_prior_history`
- `segmented_history_keeps_legacy_full_snapshot_generation_readable`
- `corrupt_history_degrades_history_without_blocking_current_authority`
- `corrupt_selected_authority_never_falls_back_to_an_older_generation`
- `corrupt_history_manifest_degrades_history_only`
- `service_identity_challenge_binds_nonce_domain_epoch_and_custody`

### `cli/src/native/service_lease_authority/protocol/client.rs` (7)

- `cohesive_client_owns_exchange_and_typed_response_validation`
- `ordinary_profile_enrollment_and_acquisition_require_no_lease_choreography`
- `protected_launch_request_is_closed_and_replay_is_not_effect_capable`
- `owner_reconciliation_request_contains_no_caller_process_assertion`
- `browser_adoption_client_exposes_only_claims_and_redacted_authority_projections`
- `profile_authority_inspection_keeps_reservation_holder_and_occupancy_independent`
- `first_launch_delivery_and_exact_completion_are_typed`

### `cli/src/native/service_lease_authority/protocol/custody.rs` (13)

- `same_user_service_cannot_claim_protected_authority_custody`
- `user_writable_state_root_cannot_hold_operational_authority`
- `protected_endpoint_identity_is_bound_to_the_exact_socket_instance`
- `candidate_owned_socket_cannot_impersonate_the_authority_endpoint`
- `candidate_writable_executable_cannot_be_the_stable_authority`
- `socket_peer_must_be_the_exact_root_authority_process`
- `systemd_socket_activator_is_valid_endpoint_custody_but_not_service_custody`
- `endpoint_custody_accepts_both_socket_activator_and_active_service_states`
- `active_service_cgroup_requires_the_exact_protected_system_unit`
- `host_root_projection_accepts_direct_and_unmapped_kernel_identities_only`
- `socket_activated_endpoint_rejects_a_root_process_other_than_pid_one`
- `linux_request_peer_identity_comes_from_the_connected_socket`
- `linux_endpoint_inspection_rejects_a_user_owned_impostor`

### `cli/src/native/service_lease_authority/protocol/service.rs` (8)

- `browser_adoption_requests_always_enter_the_durable_mutation_transaction`
- `bootstrap_is_atomic_single_use_and_loadable`
- `ordinary_challenge_is_independent_of_administrator_credential_availability`
- `profile_enrollment_and_acquisition_are_published_before_reply`
- `non_root_administrative_request_is_rejected_before_authority_state_load`
- `root_revoke_plan_and_apply_are_published_before_the_service_replies`
- `bootstrap_rejects_zero_group_without_creating_state`
- `bootstrap_requires_root_and_a_banked_executable`

## Count invariant

The five source files contain 22 + 56 + 7 + 13 + 8 = 106 tests. The ledger
contains 15 + 56 + 7 + 13 + 8 = 99 moved tests and 7 retained tests, proving
`99 + 7 = 106`.
