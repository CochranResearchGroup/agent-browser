# Plan 0203 | Runtime Host Admission Routing

Date: 2026-09-16

Plan version: 4

State: CLOSED

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

Plan 0203 is closed. [PR #172](https://github.com/CochranResearchGroup/agent-browser/pull/172)
merged source head `283809661d789ccea932fdbfa3d3f83b45937de4` into `main`
as `136a1928af5368ea88342ae33e9a9b671dc12802`; issue #169 closed with
the merge. Exact-head CI run
[`35160652859`](https://github.com/CochranResearchGroup/agent-browser/actions/runs/35160652859)
passed every selected gate, including the complete Rust suite, no-launch
service smokes, strict Clippy, formatting, workstation fixtures, service client,
dashboard, and version sync.

The repair lets the supervised same-generation singleton host replace a
selected identity whose boot epoch is proven prior only after socket and stream
readiness and all existing transaction, binary, generation, and topology
fences pass. Current-boot ownership and missing or unavailable boot-epoch
evidence remain fail-closed. Thirteen focused ingress tests and a disposable
no-launch supervisor smoke pass. No browser, provider, credential,
installed-runtime, Service State, retained-profile, production, or release
effect occurred.

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
| Required validation and integration | 13 focused ingress tests, repository format, strict workspace Clippy, diff hygiene, planning audit, and disposable supervisor no-launch smoke pass; exact-head CI run `35160652859` passed and source head `28380966` entered `main` as `136a1928` through PR #172 | proven and integrated |

## Stop Condition

Stop before any installed-runtime, browser, provider, credential, retained
profile, Service State, production, or release effect. Stop if the repair would
permit a new legacy per-session daemon, accept a cross-generation or
transaction-active replacement, bypass transactionally selected ingress state,
or touch source owned by P197 or P202 without reconciling writer custody.
