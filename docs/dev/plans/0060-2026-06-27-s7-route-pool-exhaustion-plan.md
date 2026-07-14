# S7 Route Pool Exhaustion Plan

Date: 2026-06-27
State: COMPLETE
Lane: P60
Parent: `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`

## Problem

S7 requires a third route-bound browser request to fail closed with an explicit
route-capacity blocker after two healthy route-bound browsers occupy both
route-pool entries.

The first strict S7 attempts proved the functional safety properties, but not
the diagnostic contract:

- no fake profile C browser row was retained;
- no terminal fallback appeared on either occupied route display;
- retry after releasing one route succeeded;
- the third request still surfaced
  `display_allocation_owner_mismatch` instead of route capacity exhaustion.

Artifacts:

- `/tmp/agent-browser-p46-s7-2026-06-27T20-41-16-219Z`
- `/tmp/agent-browser-p46-s7-2026-06-27T20-43-19-369Z`
- `/tmp/agent-browser-p46-s7-2026-06-27T20-50-09-968Z`
- `/tmp/agent-browser-p46-s7-2026-06-27T20-55-23-570Z`
- `/tmp/agent-browser-p46-s7-2026-06-27T20-56-50-842Z`

## Repair

- Added S7 metadata and evaluator checks to the P46 scenario harness.
- Added live S7 capture for two occupied route-bound profiles, a third demand
  probe, and retry after one route release.
- Tightened `plan_remote_view_acquisition` so unpinned route-bound demand that
  selects a checked-out route-pool display owned by another session reports
  `route_pool_exhausted` before owner-mismatch fallback.
- Added focused Rust coverage for checked-out inline route-pool exhaustion and
  browser-display fallback exhaustion.

## Validation

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml acquisition_plan_reports_route_pool_exhausted -- --nocapture
node scripts/test-p47-scenario-harness.js
cargo build --manifest-path cli/Cargo.toml
node scripts/run-p46-stress-scenario.js --scenario s7 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

The live pass used artifact:

```text
/tmp/agent-browser-p46-s7-2026-06-27T20-58-30-721Z
```

## Result

S7 passed. The third route-bound request returned `route_pool_exhausted`, did
not create a retained profile C browser row, left both occupied route displays
browser-window-visible with no terminal fallback, and succeeded after profile A
released a route. Reset-after closed profile B and profile C and reported zero
active incidents.

P46 may continue at S8.
