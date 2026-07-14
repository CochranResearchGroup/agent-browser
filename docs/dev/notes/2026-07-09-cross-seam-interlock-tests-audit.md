# Cross-Seam Interlock Tests Audit

Date: 2026-07-09

## Context

The recent failures crossed several seams at once: service status, process resources, profile routing, left-rail inventory, view stream readiness, and viewport selection. Existing focused tests catch individual helpers, but they do not prove the interlocks agree across seams.

This audit covers Candidate 6 from the architecture review: Cross-Seam Interlock Contract Tests.

## Current Shape

The repo has many focused tests and smoke scripts:

- Rust unit tests for daemon logic
- service-client contract and generated type checks
- dashboard workspace node tests
- dashboard navigator structure tests
- dashboard view-stream helper tests
- live RDP and Guacamole smoke scripts
- schema validation utilities for service collections

These are valuable but mostly local to one module. A failure can still pass through because each layer tests its own shape:

- service status can emit a field that dashboard types do not consume
- dashboard inventory can render rows without daemon authority verdicts
- view stream helpers can consider a URL embeddable while runtime readiness is broken
- viewport selection can read a stale node even if the live rail suppressed it
- generated client types can omit additive fields unless the generator template is updated

Candidate 1 exposed this gap directly: the first daemon snapshot passed Rust tests, but subagent audit found missing dashboard consumption and missing generated typing.

## Failure Mechanism

The system has too many independent moving parts for module-local tests to be sufficient. The necessary safety property is cross-seam:

```text
daemon status truth -> generated client type -> dashboard input -> inventory placement -> viewport context
```

If any one seam ignores the authority verdict, the UI can still present stale browsers, hide viable detected browsers, or open streams that are not ready.

## Deletion Test

Deleting `scripts/test-dashboard-workspace-nodes.js` would remove many useful dashboard invariants, but not the service-status or generated-client side.

Deleting `pnpm test:service-client` would remove schema/client parity, but not prove dashboard consumption.

Deleting Rust unit tests would remove daemon confidence, but not reveal UI projection drift.

No single current test owns the whole interlock.

## Recommended Deep Test Surface

Create a cross-seam no-launch contract fixture that starts from one service-status JSON fixture and asserts all downstream projections:

- schema accepts the status
- generated client type includes the field
- dashboard workspace input consumes the field
- live rail excludes non-viable authority rows
- attention rows remain visible at the bottom
- selected-workspace context sees the same authority placement
- stream helper blocks known-bad readiness states

This should be a fast no-launch test in `scripts/`, not a live browser smoke.

## Required Interlocks

- Any new service-status authority field must be present in schema, generated client types, and dashboard input types.
- Any daemon non-viable verdict must prevent initial live-rail inclusion.
- Any daemon attention verdict with live evidence must remain visible as attention.
- Any known-bad stream readiness state must block dashboard open/control helpers.
- Selected-workspace context must not revive a row hidden by live inventory authority.

## Risks

- A broad live smoke would be too slow and flaky for this purpose.
- A snapshot-only test would become stale if it does not call real projection helpers.
- A dashboard-only test would miss generated client drift.

## Acceptance For Candidate 6

- A plan exists for a no-launch cross-seam interlock test.
- The first implementation slice should add one fixture that exercises service-status authority through dashboard projection and selected context.
- The test should run in the fast local validation set.
- Future slices should add stream readiness and gateway error fixture coverage.
