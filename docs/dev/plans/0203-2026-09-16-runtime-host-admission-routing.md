# Plan 0203 | Runtime Host Admission Routing

Date: 2026-09-16

Plan version: 3

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P203

Work item: `CochranResearchGroup/agent-browser#169`

Branch: `platform/p203-runtime-host-admission`

Target: `main`

Integration: merge through the protected `main` workflow after provider-free regression and changed-surface validation

Source baseline: `dd34977efb256c256ffd96bd2401d225b10c2f62`

## Objective

Make a fresh CLI or MCP client adopt the transactionally selected singleton
runtime host for control-plane and service commands even when it has not been
given explicit host-admission environment state. Preserve the hard rejection
of all new legacy per-session daemon creation.

## Current State

Issue #169 is accepted and P203 is active. Current production evidence is only
a defect locator: a valid access plan can select a durable profile while the
next no-launch CLI or MCP command reports `runtime_host_admission_required`.
P203 is source-complete at `08bebd29`. Fresh runtime readback identified the exact split-brain: the selected ingress
registry retains a prior-boot epoch and dead PID while the supervised
same-generation singleton host is reachable at the selected socket under a new
PID. The host's existing CAS-fenced self-adoption path rejects the prior epoch
before it can replace that necessarily stale identity, so ordinary clients
correctly refuse the registry and fall through to retired legacy admission. The
repair now treats a PID from a proven prior boot as non-authoritative only in
the already-scoped supervised self-adoption path. Current-boot owner, missing-
epoch, binary, generation, topology, and transaction fences remain fail-closed.
Protected exact-head CI, review, integration, and closeout remain.

## Consolidated Batch

1. Add a provider-free fixture for a supervised same-generation replacement
   encountering a selected singleton registry from a prior boot.
2. Prove the current adoption path rejects the stale epoch before refreshing
   the selected PID, host, socket identity, and boot epoch.
3. Permit that exact self-authenticating prior-boot replacement while retaining
   binary, generation, topology, transaction, and atomic-write fences.
4. Revalidate fresh-client connection routing, the MCP/service-request boundary,
   and the no-legacy-launch interlock against the refreshed registry.
5. Run focused regression, formatting, strict workspace Clippy, planning audit,
   and every additional gate selected from the changed surface.

## Scope And Effect Boundary

Expected source writes are limited to `cli/src/runtime_host_ingress.rs` and its
inline tests. `cli/src/connection.rs` tests may be updated only if a direct
client-routing regression is required after the ingress repair. This plan and
its branch-local roadmap, runbook, and active-lane projections are also in
scope.

This plan does not authorize browser launch, provider access, credential use,
installed-runtime installation or restart, workstation repair, Service State
mutation, retained-profile mutation, production mutation, or release. Tests
may create only disposable local sockets, tokens, and ingress registries under
their own temporary directories.

## Delivery Sequence And Budget

- Attempt 1: red fresh-client fixture plus missing-selection fail-closed control.
- Attempt 2: one connection-bound endpoint-selection repair and focused tests.
- Attempt 3: one bounded correction if changed-surface validation exposes a
  defect introduced by this packet.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 180 active minutes through protected integration.

No subagent, runtime operator, browser worker, provider worker, production
install, or shared-runtime mutation is assigned.

## Worker Assignments

The P203 lane owner holds the critical path, plan, branch, provider-free
fixtures, source repair, validation, integration, and closeout. P203 is the
sole writer for `cli/src/runtime_host_ingress.rs` in this packet. P202 owns
abandoned browser retirement and P197 owns challenge consumer integration;
their source surfaces are disjoint. Shared planning projections will be
reconciled before integration.

## Evidence And Exit

Exit requires current evidence that:

- a supervised same-generation singleton host can atomically refresh a
  prior-boot selected identity after its own socket and stream are ready;
- a fresh client then resolves the refreshed runtime-host endpoint and attaches
  the requested logical lane;
- the same routing boundary is used by MCP `service_request` and ordinary
  no-launch service commands;
- an explicit candidate socket override remains isolated from the selected
  ingress;
- absent, stale, or invalid ingress selection does not authorize a legacy
  daemon launch;
- focused regression, formatting, strict Clippy, and all selector-required
  changed-surface gates pass; and
- the exact source checkpoint enters `main` through the linked pull request
  with applicable exact-head CI green.

| Requirement | Current evidence | State |
| --- | --- | --- |
| Prior-boot supervised replacement refreshes selection | Red fixture failed with `runtime_host_boot_epoch_prior`; it now passes while exercising numeric PID reuse | focused pass |
| Fresh client adopts selected ingress | Repaired fixture refreshes epoch/PID/socket identity atomically and `selected_socket_dir()` returns the replacement route | focused pass |
| MCP and CLI share connection routing | No client bypass was added; both continue through the existing selected-ingress connection boundary | source verified |
| Candidate override remains isolated | Explicit socket override behavior is unchanged; existing ingress transaction suite passes | focused pass |
| Missing selection fails closed | Missing-epoch fixture returns `runtime_host_boot_epoch_missing`; legacy launch rejection remains unchanged | focused pass |
| Required validation and integration | 13 focused ingress tests, repository format, strict workspace Clippy, diff hygiene, planning audit, and disposable supervisor no-launch smoke pass at `08bebd29` | local pass; protected integration pending |

## Stop Condition

Stop before any installed-runtime, browser, provider, credential, retained
profile, Service State, production, or release effect. Stop if the repair would
permit a new legacy per-session daemon, accept a cross-generation or
transaction-active replacement, bypass transactionally selected ingress state,
or touch source owned by P197 or P202 without reconciling writer custody.
