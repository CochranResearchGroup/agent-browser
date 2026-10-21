# P259 independent handoff retention reconciliation

Product lane: PL-PLATFORM
Disposition: accepted-evidence
Owning plan or work item: docs/dev/plans/0259-2026-10-20-handoff-retention-reconciliation.md / CochranResearchGroup/agent-browser#260
Related lanes: P237, P240, #105

## Scope and resulting behavior

Issue #260 remained open after integrated P237 delivered independent link and
tab lifetimes. This slice changes only the owner regression: its finite pending
human hold now exercises ten minutes and four hours instead of only 50 ms.
Both exceed the ordinary five-minute physical-idle interval. It proves persisted
protection just before each deadline, exact-target expiry at the deadline and
a still-valid perpetual link, across two owner restarts. No new default or
policy is introduced. The configured values are fixture inputs, not product
constants. No product defect was reproduced in this slice.

## Acceptance map

| #260 criterion | Current source qualification |
| --- | --- |
| Configurable pending-human protection beyond five minutes | `finite_pending_human_deadline_expires_exact_target_after_restart` now runs 600000 and 14400000 ms, checking deadline minus one and deadline. `unused_human_handoff_protects_configured_tab_expiry_across_restart` retains the unlimited case. |
| Independent perpetual and finite links | `durable_named_handoff_has_no_default_expiry_across_restart`, `disposable_handoff_defaults_perpetual_across_restart`, `existing_link_lifetime_switches_finite_and_perpetual_without_target_effects`, and `handoff_link_retention_extension_and_expiry_do_not_change_browser_or_issue_viewer_grants`. |
| Independent deletion and persisted deadlines | `deleting_handoff_revokes_only_link_and_preserves_target_across_restart`, `independent_handoff_policy_deadlines_and_creation_context_survive_restart`, and concurrent delete/policy publication regressions. |
| Exact tab and last-browser lifecycle | `configured_tab_lifetime_expires_only_its_target_and_keeps_the_link`, `closing_last_live_tab_closes_browser_without_deleting_handoff`, and `automatic_last_tab_expiry_closes_owned_browser_with_empty_peer_session`. |
| Recovery through the original valid link | `valid_handoff_recreates_expired_task_tab_after_host_restart`, `native_idle_pool_returns_capacity_and_recovers_same_logical_handoff`, and `expired_or_deleted_link_cannot_recreate_missing_target_after_restart`. |
| Qualified first use, never agent polling | `human_use_releases_pending_hold_but_agent_preparation_does_not`, `public_human_use_request_preserves_pending_hold`, native-use exact binding/publication/replay tests, and the separate service-model native-use transport suite. |
| Configured measured pressure and protected peers/users/operations | `pressure_configuration_is_opt_in_and_invalid_edits_preserve_state`, `configured_pressure_retires_inactive_tab_without_revoking_its_link`, idle-viewer protection, and model `pressure_policy_preserves_active_peer_after_link_deletion` / `idle_browser_close_preserves_active_peers_and_external_operation_custody`. |

The owner configuration parser validates `pendingHumanRetentionMs` before
effects, falling back to `AGENT_BROWSER_PENDING_HUMAN_RETENTION_MS`; default 0
means until use. Inspect reports the retained request/deadline independently
from link expiry and tab inactivity. Existing README, CLI help, workflow skill
and Remote View docs describe this behavior. No new public behavior requires
a documentation-contract change.

## Evidence boundaries

The new two-duration test passes on current source in 10.59 seconds after the
exact test binary build. The final complete owner pool passes 94 tests with
one real-provider composition test ignored in 491.49 seconds. The service-model
suite passes 357 tests across 21 suites, including doc tests, with none ignored.
Cargo-safe formatting and strict workspace Clippy pass; handoff documentation,
links, patch hygiene and P259 planning audit pass. The audit first rejected an
abbreviated owner locator in this note; the canonical plan/work-item locator
corrected it before the passing run. Standards and specification review found
no unresolved source criterion. No product code changed, so no failing product
baseline or new runtime repair is claimed.

The [P237 installed acceptance](0237-2026-10-16-installed-acceptance.md) remains
historical evidence for its exact isolated installed generation, including
actual valid-link recreation, first use, independent expiry/deletion and
last-tab closure. Its statement that wider #260 work remains separate is
reconciled by this criterion map, not silently edited or treated as a new
installed run. No workstation-wide pressure stress, uptime SLA, restoration of
lost in-memory page state or current private-account continuity is claimed.

This slice changes no production binary, unit, provider, browser, profile or
shared skill. No native fixture, cleanup or CI dispatch occurs. #105 retains
resource-history and broader resource convergence ownership.
