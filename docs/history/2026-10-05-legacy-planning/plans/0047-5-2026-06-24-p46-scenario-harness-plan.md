# P47.5 P46 Scenario Harness Plan

Date: 2026-06-24
State: DONE
Lane: P47.5
Parent Plan: `docs/dev/plans/0047-2026-06-24-remote-view-foundational-architecture-plan.md`

## Goal

`/goal execute P47 goal 5: turn the P46 stress runner into a scenario harness module with declarative scenarios, explicit roles, reset protocol, evidence recorder, and failure audit classification`

## Audit Findings

- `scripts/run-p46-stress-scenario.js` implements S0, S1, and S2 directly in
  one file.
- The runner already has useful capture, reset, evaluation, and failure audit
  boundaries.
- The missing foundation is a declarative scenario spec that records roles and
  invariants before live execution, especially the S2 rule that viewer clients
  consume zero route leases.

## Implementation Plan

1. Add a `scripts/lib/p46-scenario-harness.js` module with S0, S1, and S2
   specs.
2. Validate scenario specs before live execution.
3. Add no-live tests for role validation, S2 route-lease invariants, and
   failure classification.
4. Keep live runner behavior scoped to the already implemented scenarios.

## Validation

- PASS: `pnpm test:p47-scenario-harness`
- PASS: `node --check scripts/lib/p46-scenario-harness.js`
- PASS: `node --check scripts/test-p47-scenario-harness.js`
- PASS: `node --check scripts/run-p46-stress-scenario.js`
- PASS: `git diff --check -- scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js package.json docs/dev/plans/0047-5-2026-06-24-p46-scenario-harness-plan.md`

## Closeout

Added `scripts/lib/p46-scenario-harness.js` with declarative S0, S1, and S2
scenario specs, role validation, and failure classification. The P46 runner
now validates the scenario spec before live execution and writes classified
failure audits.
