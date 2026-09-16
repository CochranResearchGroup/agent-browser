# Plan 0203 | Runtime Host Admission Routing

Date: 2026-09-16

Plan version: 1

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
Source inspection shows endpoint selection is split between the selected
ingress registry and the explicit host-admission environment gate. Provider-free
red evidence and the smallest safe routing repair remain pending.

## Consolidated Batch

1. Add a provider-free fixture for a fresh client with a current selected
   singleton ingress, reachable authenticated endpoint, and no legacy session
   daemon.
2. Prove the current client path rejects or misses that selected endpoint.
3. Route ordinary fresh clients through the selected singleton endpoint while
   preserving explicit socket overrides used for candidate observation and
   preserving fail-closed behavior when selection is absent or invalid.
4. Cover the MCP/service-request command path at the connection boundary and
   retain the no-legacy-launch interlock.
5. Run focused regression, formatting, strict workspace Clippy, planning audit,
   and every additional gate selected from the changed surface.

## Scope And Effect Boundary

Expected source writes are limited to `cli/src/connection.rs`, its inline
tests, and, only if the endpoint contract cannot be expressed there,
`cli/src/runtime_host.rs`. This plan and its branch-local roadmap, runbook, and
active-lane projections are also in scope.

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
sole writer for `cli/src/connection.rs` in this packet. P202 owns abandoned
browser retirement and P197 owns challenge consumer integration; their source
surfaces are disjoint. Shared planning projections will be reconciled before
integration.

## Evidence And Exit

Exit requires current evidence that:

- a fresh client with a current selected singleton ingress resolves the
  runtime-host endpoint and attaches the requested logical lane;
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
| Fresh client adopts selected ingress | Current source gates selected ingress behind explicit admission state | pending red fixture |
| MCP and CLI share connection routing | Both paths reach `ensure_daemon` and `send_command`; exact regression pending | pending |
| Candidate override remains isolated | Explicit `AGENT_BROWSER_SOCKET_DIR` has first priority | existing control; revalidation pending |
| Missing selection fails closed | Legacy launch admission rejection exists | existing control; revalidation pending |
| Required validation and integration | Not run | pending |

## Stop Condition

Stop before any installed-runtime, browser, provider, credential, retained
profile, Service State, production, or release effect. Stop if the repair would
permit a new legacy per-session daemon, bypass transactionally selected ingress
state, or touch source owned by P197 or P202 without reconciling writer custody.
