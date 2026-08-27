# Service Profile Lease v1

This contract defines the canonical profile lease resource and guarded
lifecycle operations introduced by Plan 0134 Slice C. It builds on
`service-principal-authority.v1.md`.

## Record identity

A profile lease record is derived from one authenticated principal-to-profile
capability and the exact current runtime owner binding. Its stable id is based
on the principal and canonical profile identity. Its `leaseRevision` is a
digest of the complete projected authority and subordinate-work evidence, so
any holder, browser, route, expiry, cleanup, or identity change invalidates a
stale mutation request.

Each record includes principal provenance, profile identity, logical browser,
sessions, tabs, mode, state, owner generation, process identity, routes,
heartbeat and expiry evidence, cleanup obligation, blocking identity axes,
authorized actions, typed continuity recourse, and observation-only status.

Legacy profiles appear only when retained session or owner evidence exists.
They remain observation-only and never become effect-capable from labels or a
principal-shaped session field.

## Read operations

The canonical read family is `list`, `inspect`, `explain`, `doctor`, and
`watch`. `doctor` evaluates every projected lease and returns typed findings
with exact safe actions. Reads do not launch a browser or mutate Service State.

## Owner-scoped operations

`rejoin`, `renew`, and `release` require a current authenticated profile
capability and an exact `leaseRevision`.

- `rejoin` returns the retained owner route without transferring ownership.
- `renew` advances subordinate work lease revisions and expiry.
- `release` refuses active subordinate tabs and never releases another
  principal's work. It releases only matching current session work leases.

Authorized actions are projected on each record. A client must not infer an
action from state labels alone.

## Reconciliation

Reconciliation is a two-step plan and apply protocol. A plan is bound to the
lease revision, owner generation, principal, profile, browser, process,
routes, boot epoch, proposed transitions, expiry, idempotency key, and a
service seal. Apply rejects any mismatch and stores an idempotent receipt.
Replay returns that receipt without repeating transitions.

Until Plan 0134 introduces a current boot epoch, reconciliation planning is
available for diagnosis but returns `effectCapable: false` with
`boot_epoch_unavailable`. This prevents the control plane from applying a
cross-boot repair using incomplete evidence.

The current source checkpoint provides the canonical model and pure guarded
operations. CLI, HTTP, MCP, dashboard, generated-client, event, schema, and
watch parity remain in progress within Slice C.
