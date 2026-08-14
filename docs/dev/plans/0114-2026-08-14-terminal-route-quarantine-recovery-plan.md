# Plan 0114: Terminal Route Quarantine Recovery

Date: 2026-08-14

State: OPEN

Lane: P114

Source baseline: `15743c05`

## Goal

Let a service client safely recover one exact route-bound acquisition lease
that remains in `rollback_incomplete` after all related retained ownership has
become terminal. Preserve quarantine when a browser, process identity, session,
route, display, or route-pool checkout could still represent a live effect.

## Incident Evidence

- The hidden Google Messages supervisor is active, its fixed loopback endpoint
  is reachable, executable provenance matches, and it has zero restarts.
- One August 13 acquisition timed out during focus and retained an active
  `rollback_incomplete` quarantine.
- The matching browser, process identity, and session are absent. The route and
  display are released, and the route-pool entry is available with no current
  allocation.
- New acquisitions are rejected by the old lease even though the guarded
  resource identities have since reached terminal retained state.
- The lease lifecycle supports promotion from `rollback_incomplete` to
  `rollback_complete`, but the production repair surface only handled pending
  acquisitions and stale checked-out routes.

## Frozen Repair

Extend `service_route_pool_repair` rather than adding another overlapping
repair action. An optional `acquisitionLeaseId` scopes both pending and terminal
acquisition repair to one exact lease. Terminal quarantine is a candidate only
when all of these are true:

- the lease is `failed/rollback_incomplete`;
- no matching browser, process-identity, or session record exists;
- the matching route and display are absent or released;
- the matching route-pool entry is absent or is available or unavailable with
  no current allocation.

Dry run remains the default. Apply records
`confirmed_inactive_retained_state`, preserves the original failure evidence,
removes the active quarantine marker, and advances only that lease to
`failed/rollback_complete`. Any live or ambiguous retained state is returned as
a typed skipped reason and remains quarantined.

## Acceptance Criteria

- Focused Rust tests prove safe promotion, exact-lease scoping, and live-browser
  rejection.
- Generated service-client types and helpers carry
  `acquisitionLeaseId`, `stalePendingAcquisitions`, and typed skipped reasons.
- README, CLI help, Agent Browser skill guidance, service-mode docs, roadmap,
  and runbook describe the recovery boundary.
- Rust format, focused tests, strict Clippy, service-client tests, selected
  validation, and diff checks pass.
- The validated checkpoint binary is installed once, and a dry run against the
  exact Google Messages lease reports one safe candidate before a single apply.
- Installed readback shows the lease at `rollback_complete`, the route pool
  still ready, and no browser, profile, route, authentication, or keyring
  replacement.

## Bounds

- Implementation attempts: `1/2`.
- Review and rework cycles: `0/1`.
- Checkpoint installs: `0/1`.
- Terminal-quarantine applies: `0/1`.
- Live read-only observations: `6/18`.
- Authentication, keyring, profile replacement, route replacement, browser
  launch, browser termination, message send, formal release, and deployment:
  zero.

## Current Checkpoint

The red Rust fixture first failed because no exact-lease terminal repair helper
existed. The focused implementation now passes safe-promotion, live-browser
rejection, exact handler scoping, and generated service-client tests. Broader
validation, documentation gates, checkpoint install, dry run, apply, and
installed readback remain open.
