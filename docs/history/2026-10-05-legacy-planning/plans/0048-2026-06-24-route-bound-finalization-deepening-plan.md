# Route-Bound Finalization Deepening Plan

Date: 2026-06-24
State: COMPLETE
Lane: P48
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/plans/0047-2026-06-24-remote-view-foundational-architecture-plan.md`
- `docs/dev/plans/0047-6-2026-06-24-p46-s2-reaudit-and-unlock-plan.md`
- `docs/dev/notes/2026-06-24-p47-6-s2-reaudit-blocker.md`
- `/tmp/architecture-review-agent-browser-2026-06-24T21-05-00.html`

## Purpose

Deepen the route-bound finalization module before any further P46 stress
execution. P47 fixed the viewer-client confusion and proved the UX path can
work, but S2 still failed because service state drifted after an otherwise
functional run.

Observed blocker:

- `remote-view-route:guacamole:3` became an active incident;
- incident message: `Remote route 'guacamole:3' is orphaned: orphaned display_allocation display_allocation_unavailable`;
- related route-pool row: `guacamole-rdp-a` stayed pending
  `remote_view_open_acquisition`;
- reset-after returned to zero active incidents.

This plan must be executed from a fresh session. Do not retry the P46 stress
matrix until the earlier goals close with evidence.

## Architecture Review

Report:

`/tmp/architecture-review-agent-browser-2026-06-24T21-05-00.html`

Preview:

`https://previews.ecochran.dyndns.org/s/2ae9d7971901`

Top recommendation: deepen route-bound finalization first. The current
helpers are shallow modules because callers still need to understand the
ordering and invariants across route-pool entry, display allocation, route
record, browser stream, acquisition lease, and incidents.

## Execution Rules

- Execute goals in order.
- Each goal is `/goal` compatible and should be closed or blocked separately.
- Do not run P46 S2 or later stress scenarios until the final goal.
- Keep all remediation source-backed with focused tests.
- Preserve the installed/runtime contract across CLI, HTTP service, MCP,
  generated client, dashboard, and P46 runner artifacts.
- If a goal finds the module shape is wrong, stop and update this plan before
  continuing.

## Goal 1: Audit Finalization Drift

`/goal execute P48 goal 1: audit route-bound finalization drift so the exact write order and stale reconciliation trigger behind the P47.6 S2 active incident are source-backed before code changes`

### Work

- Trace the successful S2 retry artifact:
  `/tmp/agent-browser-p47-6-s2-retry-2026-06-24`.
- Compare `remote-view-open.json`, `service-status-after-two-operators.json`,
  `service-incidents-after-two-operators.json`, and reset-after artifacts.
- Inspect the current write order in:
  - `remote_view_open_begin_acquisition_lease`;
  - `handle_service_remote_view_route_checkout`;
  - `remote_view_open_complete_acquisition_lease`;
  - `repair_route_pool_service_state`;
  - incident reconciliation paths.

### Evidence Required

- A short note naming the exact record mismatch.
- A focused failing or characterization test that reproduces the stale pending
  acquisition or orphaned display allocation condition without launching
  browsers.

### Stop Condition

Stop if the S2 artifact and source do not agree on the failure shape.

### Status

Closed on 2026-06-25.

Goal 1 audit note:

`docs/dev/notes/2026-06-25-p48-goal1-finalization-drift-audit.md`

Characterization test:

`test_repair_route_pool_service_state_characterizes_completed_lease_pending_drift`

The S2 artifact and source agree on the failure shape: a completed acquisition
lease can coexist with pending route-pool and display-allocation records, then
health reconciliation marks the route orphaned because the display allocation
never finalized to ready.

## Goal 2: Deepen The Finalization Module

`/goal execute P48 goal 2: deepen route-bound finalization so one module finalizes route-pool entry, display allocation, route record, browser stream, acquisition lease, and incident facts together`

### Work

- Move finalization ownership out of scattered `actions.rs` helpers into one
  route-bound finalization module.
- Keep command dispatch and repository plumbing in `actions.rs`.
- Ensure finalization either commits one coherent ownership record or returns
  a typed blocker.
- Ensure pending lease state cannot survive after a successful checkout and
  ready operator-visible proof.

### Evidence Required

- Unit tests for the successful finalization path.
- Unit tests for partial-write failure rollback.
- Unit tests proving finalized records do not produce stale pending
  acquisition repair candidates.

### Stop Condition

Stop if finalization still requires callers to update route-pool, display,
route, browser, and lease records independently.

### Status

Closed on 2026-06-25 for no-launch remediation.

Implemented `cli/src/native/remote_view_finalization.rs` and wired
`remote_view_open_complete_acquisition_lease` through it. Lease completion now
finalizes route-pool entry, display allocation, remote-view route, browser
ownership, and lease state in one repository mutation.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_finalization`

## Goal 3: Align Reconciliation With Finalization

`/goal execute P48 goal 3: align remote-view reconciliation with finalized ownership so incident generation reads the same finalized route-bound fact instead of reinterpreting partial records`

### Work

- Make route-pool repair and incident reconciliation consume finalized
  ownership state.
- Distinguish:
  - active finalized route;
  - pending acquisition;
  - rolled-back acquisition;
  - stale orphan;
  - released history.
- Prevent a finalized route from being marked orphaned only because an older
  pending acquisition snapshot exists.

### Evidence Required

- No-launch incident tests for:
  - finalized ready route;
  - pending acquisition without ready browser;
  - rolled-back acquisition;
  - missing display allocation;
  - released route.
- Existing repair tests still pass.

### Stop Condition

Stop if incident reconciliation still derives route health from route-pool or
display records without checking the acquisition lease state.

### Status

Closed on 2026-06-25 for no-launch reconciliation.

Incident generation now classifies completed-lease plus pending route-bound
records as `remote_view_finalization_incomplete` instead of collapsing the
failure into a generic display or route incident.

Validation:

- `cargo test --manifest-path cli/Cargo.toml refresh_derived_views_distinguishes_route_bound_finalization_states`

## Goal 4: Publish Finalized Ownership To Inventory

`/goal execute P48 goal 4: publish finalized route-bound ownership to workspace inventory so dashboard actionability depends on finalization state, not stream URL shape`

### Work

- Extend the canonical inventory path to carry finalized ownership state.
- Keep TypeScript as an adapter for rendering and compatibility fallback.
- Ensure diagnostic, retained, pending, rolled-back, and viewer-client records
  cannot become control rows.

### Evidence Required

- Dashboard workspace-node tests covering finalized, pending, rolled-back,
  diagnostic, retained, and viewer-client rows.
- Generated client or schema updates if a contract shape changes.

### Stop Condition

Stop if dashboard code still has to infer final ownership from URL params,
stream URLs, or row shape.

### Status

Closed on 2026-06-25 for dashboard inventory.

Workspace nodes now expose `routeBoundOwnership`. RDP view and control actions
require finalized route-bound ownership. Pending, rolled-back, diagnostic,
retained, and viewer-client records stay visible but non-controllable.

Validation:

- `pnpm test:dashboard-workspace-nodes`

## Goal 5: Teach The Harness The New Finalization Evidence

`/goal execute P48 goal 5: teach the P46 scenario harness to record finalized route-bound ownership evidence and fail with a typed finalization blocker when ownership is not coherent`

### Work

- Add finalization evidence to S2 artifacts.
- Make the evaluator distinguish:
  - viewer-client adapter failure;
  - route/display finalization failure;
  - proof failure;
  - reset failure.
- Keep the two-failure lock rule intact.

### Evidence Required

- No-live scenario harness tests.
- `node --check scripts/run-p46-stress-scenario.js`.
- `pnpm test:p47-scenario-harness`.

### Stop Condition

Stop if harness classification can collapse finalization drift back into a
generic active incident.

### Status

Closed on 2026-06-25 for no-live harness validation.

The S2 runner now writes `route-bound-finalization-evidence.json` and fails
with a typed route-bound finalization blocker when lease, route-pool, display,
route, and browser ownership are incoherent.

Validation:

- `node --check scripts/run-p46-stress-scenario.js`
- `pnpm test:p47-scenario-harness`

## Goal 6: Fresh-Session Stress Retry

`/goal execute P48 goal 6: from a fresh session, rerun the blocked P46 stress-testing plan after finalization, reconciliation, inventory, and harness validation pass`

### Work

- Start from a fresh agent session.
- Run no-mutation preflight:
  - install doctor;
  - remote-view doctor;
  - service status;
  - route-pool readiness;
  - display-content inspection;
  - dashboard HTTP readback;
  - harness spec validation.
- If preflight is clean, run the blocked stress retry:

```bash
pnpm test:p46-stress-scenario -- --scenario s2 --reset-before --reset-after
```

### Evidence Required

- One route-bound target browser.
- Two external viewer clients.
- One route lease for the target browser.
- Zero route leases for viewer clients.
- Dashboard screenshots for both operators.
- Functional refresh/control proof.
- Route display screenshot.
- Controlled-browser URL/title after navigation.
- Zero active incidents before reset-after or a typed finalization blocker.
- Zero active incidents after reset.

### Stop Condition

Stop and keep P46 locked if S2 creates active incidents, extra route leases,
extra target browsers, viewer-client route consumption, or untyped display
allocation drift.

### Status

Closed on 2026-06-25 from a fresh continuation session.

Preflight artifact directory:

`/tmp/agent-browser-p48-goal6-preflight-20260625T141308Z`

S2 artifact directory:

`/tmp/agent-browser-p46-s2-2026-06-25T14-13-40-415Z`

Result:

- `pnpm test:p46-stress-scenario -- --scenario s2 --reset-before --reset-after`
  passed.
- Reset-before reported zero active incidents before and after reset.
- Exactly one target browser was present after the two-operator run:
  `session:default` with profile `p46-s2-profile`.
- Viewer leases were zero.
- Finalization evidence was coherent:
  - lease `remote-view-open:default:guacamole-3:2026-06-25T14-14-05-566650612Z`
    was `completed` / `checked_out`;
  - route-pool entry `guacamole-rdp-a` was `checked_out`;
  - display allocation `remote-view-display:13` was `ready`;
  - route `guacamole:3` was `ready`;
  - browser `session:default` was `ready` and had a ready route stream.
- Operator A and B dashboard screenshots were captured.
- Operator B refresh click returned `clicked: true`.
- Route display screenshot was captured for `:13`.
- Controlled browser URL after operator A navigation was
  `https://www.iana.org/domains/reserved`.
- Active incidents were zero before reset-after and zero after reset.

## Completion Criteria

P48 is complete only when goals 1 through 5 close with focused validation and
goal 6 either passes S2 or records a new source-backed blocker that is not the
P47.6 stale finalization drift.

## Completion Status

Complete on 2026-06-25. Goals 1 through 5 closed with focused validation, and
Goal 6 passed S2 with reset-before and reset-after evidence.
