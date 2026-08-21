# P117 Slice H Source Acceptance

Date: 2026-08-20

Plan: `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`

## Accepted Boundary

Slice H is accepted at the source, contract, deterministic-fixture, and
no-launch runtime boundary. The P117 candidate has not been installed into the
live workstation by this slice. Controlled installed convergence, eligible
resource reclamation, authenticated-profile readback, and rollback proof remain
Slice I and require explicit live authorization.

The narrow privileged-helper installation and timer restoration completed in a
separate maintenance workflow. They preserve the authenticated browser but do
not establish P117 installed-runtime acceptance.

## Compatibility-Safe Lifecycle Storage

New lifecycle evidence is stored in
`runtime-lifecycle-registry.json` with schema
`agent-browser.runtime-lifecycle-registry.v1`. The locked Service State
repository merges that sidecar into current snapshots and commits Service
State, handoffs, owner registry, and lifecycle registry as one recoverable
four-file transaction.

The serialized `state.json` and `runtime-owner-registry.json` shapes remain
unchanged. Exact deny-unknown legacy-reader tests prove that installed older
consumers can continue reading both files while current source retains the new
lifecycle evidence.

## Shared Runtime Lifecycle Contract

CLI runtime health and Service Status now expose the same additive
`runtimeLifecycle` projection. It reports dashboard, runtime-host,
legacy-daemon, and generation multiplicity; lifecycle counts; reconciliation
freshness; pressure; cleanup obligations; retention; and the blocking incident.
Owner counts and the rest of the projection derive from one reconciled Service
State snapshot. Monitor receipt time supplies the observation timestamp, so
fixed input produces byte-stable HTTP and MCP results.

The typed MCP `service_status` tool, generated client helper, JSON schema, CLI
help, README, repository skill, docs site, and dashboard all use the shared
contract. The dashboard retains a legacy fallback for installed payloads that
do not yet publish `runtimeLifecycle`. Public lifecycle-store failures use a
stable redacted code, and monitor or lifecycle authority degradation makes the
overall projection degraded.

## Validation

- the repository Rust cadence passes 1,352 parallel-safe tests, 57 intentional
  ignores, every integration scope, and every serialized environment-sensitive
  partition;
- strict Clippy, Rust formatting, and patch whitespace checks pass;
- all thirteen lifecycle-store tests pass, including every write and rename
  rollback boundary and exact legacy-reader shape guards;
- lifecycle, status-projection, typed MCP, generated-client, fixed-input parity,
  service API and MCP parity, and cross-seam contract tests pass;
- the isolated Service Status smoke proves CLI and MCP readback through an
  admitted runtime host without launching Chrome or creating browser records;
- dashboard navigator, inspector-action, production build, docs production
  build, remote-view documentation, workstation source-free install, runtime
  host, Guacamole assets, PostgreSQL durability, and route-user fixtures pass;
- the confirmation test now owns a temporary Service State home, eliminating a
  cross-test durable-state dependency exposed by the prescribed Rust cadence.

## Accepted Commits

- `12c82f2e` isolates the lifecycle ledger while preserving legacy reader
  compatibility and transactional recovery.
- `3d2b16ff` aligns the shared CLI, HTTP, MCP, client, dashboard, schema, and
  documentation contract.
- `23db5747` isolates confirmed-close test state and preserves its original
  single-execution and confirmation-gate assertions.

## Next Boundary

Obtain explicit live authorization for Slice I. Then use the transactional
workstation path to converge to one dashboard, one runtime host, and one
selected generation while preserving authenticated browsers and durable
handoffs. Apply only identity-proven cleanup and verify rollback readiness from
the installed runtime.
