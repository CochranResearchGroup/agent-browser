# RDP Reattachment Stress Hardening Plan

Date: 2026-07-05
State: VALIDATED
Lane: P67
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/plans/0060-2026-06-27-s7-route-pool-exhaustion-plan.md`
- `docs/dev/plans/0066-2026-06-28-rdp-browser-reattachment-plan.md`

## Purpose

Find the remaining gaps after P66 by stressing the exact operating model the
operator expects: more live RDP-backed browsers than Guacamole routes, repeated
browser switching, authenticated profile identity, dashboard clients and
external Guacamole viewers, service reconcile, daemon restart, and failure
rollback.

P66 proved the core invariant for the current two-route host. P67 should try to
break it by combining previously independent risks. A test only counts when it
proves the browser remains reattachable until explicit close, or records a typed
non-reattachable terminal reason backed by process and service-state evidence.

## Current Proven Baseline

The following live gates passed on 2026-07-05 and are the baseline, not the end
state:

- `pnpm test:rdp-two-route-switching-live`
- `pnpm test:rdp-browser-reattach-until-close-live`
- `pnpm test:remote-view-reconcile-reattach-live`
- `pnpm test:dashboard-rdp-reattachable-rail-live`
- `pnpm test:rdp-guac-cold-restart-readback-live`
- `pnpm test:remote-view-open-live`
- `pnpm test:rdp-guac-browser-switch-live`
- `pnpm test:rdp-guac-viewer-transfer-live`

Notable P66 repairs that P67 must regression-test:

- route switch must not release a route currently owned by another browser;
- a route released by one browser must become reusable by another browser;
- explicit `runtimeProfile` or `profileId` must prevent service access-plan
  defaults from injecting another profile path;
- dashboard cross-session `view_focus` must preserve service identity and job
  timeout fields;
- repeated browser alternation acceptance must be based on viewport/browser
  identity and final retained state, with focus-job observation as supporting
  evidence rather than the sole acceptance criterion.

## Non-Negotiable Rules

- Use two ready Guacamole routes from the current route-pool readiness output.
- Use at least two independent dashboard clients when validating viewer
  behavior. Prefer different executables, for example Google Chrome and Brave.
- Do not edit service state JSON by hand to manufacture failure cases.
- Prefer service actions, live route-pool repair, forced proof-failure hooks,
  supported close commands, and no-launch unit tests for synthetic edge states.
- Every live failure writes an artifact directory with service status, jobs,
  incidents, route-pool readiness, display-content inspection, screenshots, and
  a short failure classification.
- Two consecutive failures in the same scenario lock that scenario and require
  a plan update before another live retry.
- A route URL is not proof. Acceptance requires browser id, session id, profile
  id, route id, display allocation, route-pool entry, iframe source, and
  operator-visible or dashboard DOM evidence to agree.

## Harness Direction

Add a shared P67 live harness:

```bash
scripts/test-p67-rdp-stress-hardening-live.js
```

Supported modes:

- `route-churn-soak`
- `restart-reconcile`
- `profile-identity`
- `viewer-contention`
- `rollback-and-close`
- `dashboard-rail-persistence`

Package scripts:

```json
"test:p67-rdp-route-churn-soak-live": "node scripts/test-p67-rdp-stress-hardening-live.js --mode=route-churn-soak",
"test:p67-rdp-restart-reconcile-live": "node scripts/test-p67-rdp-stress-hardening-live.js --mode=restart-reconcile",
"test:p67-rdp-profile-identity-live": "node scripts/test-p67-rdp-stress-hardening-live.js --mode=profile-identity",
"test:p67-rdp-viewer-contention-live": "node scripts/test-p67-rdp-stress-hardening-live.js --mode=viewer-contention",
"test:p67-rdp-rollback-and-close-live": "node scripts/test-p67-rdp-stress-hardening-live.js --mode=rollback-and-close",
"test:p67-rdp-dashboard-rail-persistence-live": "node scripts/test-p67-rdp-stress-hardening-live.js --mode=dashboard-rail-persistence"
```

The harness should reuse helpers from:

- `scripts/test-p66-rdp-reattachment-live.js`
- `scripts/test-rdp-guac-browser-switch-live.js`
- `scripts/test-rdp-guac-viewer-transfer-live.js`
- `scripts/smoke-rdp-guac-route-pool-readiness.js`
- `scripts/inspect-rdp-route-displays.js`

## Scenario A | Route Churn Soak

Goal: prove route assignment is stable under more live browser identities than
route slots.

Steps:

1. Start four live remote-headed browser sessions with distinct runtime
   profiles and distinct visible page titles.
2. Confirm only two Guacamole route slots are checked out at any time.
3. Cycle the dashboard viewport through this sequence at least 30 times:
   `A -> B -> C -> A -> D -> B -> A`.
4. After each switch, assert:
   - requested browser id, session id, profile id, title, and tab id are
     selected;
   - previous parked browsers remain in the owned rail as reattachable;
   - route-pool entries do not retain stale owners;
   - display allocations do not report owner mismatch;
   - no browser disappears before explicit close.
5. Close Browser C midway, then continue cycling among A, B, and D.
6. Assert C becomes `not_reattachable_closed` and the remaining browsers are
   unaffected.

Failure classes this should expose:

- stale route owner after repeated release and checkout;
- least-recently-viewed route selection stealing an active controller lease;
- owned rail compaction hiding a parked live browser;
- target/title drift when a route is reassigned to a different browser.

## Scenario B | Restart And Reconcile During Parked State

Goal: prove persisted state can reconstruct attachability after restart while
some browsers are parked.

Steps:

1. Launch three live browsers and park one browser by switching routes away
   from it.
2. Capture service status, route-pool readiness, route display content, jobs,
   and incidents.
3. Restart the dashboard/client-facing stream process or daemon session through
   supported commands. Do not kill Chrome unless the scenario explicitly tests
   process exit.
4. Run `service reconcile`.
5. Confirm all live browsers remain visible in the owned rail, with exactly one
   of:
   - `attached_ready`;
   - `reattachable_no_route`;
   - `reattachable_stale_route` plus a repair action.
6. Reattach the parked browser and verify the dashboard iframe displays the
   correct route and title.

Failure classes this should expose:

- restart loses the route-pool entry needed for the parked browser;
- reconcile compacts a live browser because its route is stale;
- stale pending acquisition lease blocks a valid route after restart;
- dashboard reload points at an old Guacamole route id.

## Scenario C | Profile Identity And Access-Plan Collision

Goal: stress the P66 fix that explicit runtime profile identity wins over
service-name access-plan reuse.

Steps:

1. Launch Browser A and Browser B with the same `serviceName` but distinct
   top-level `runtimeProfile`, `browserId`, and `sessionName`.
2. Repeat with `profileId` instead of `runtimeProfile`.
3. Repeat with a service access-plan default profile present in service state.
4. For each variant, launch and switch both browsers through the dashboard.
5. Assert no request tries to use the other browser's user-data-dir and no
   profile-lock diagnostic appears.
6. Assert service state retains both profile records and browser rows with the
   correct profile id.

Failure classes this should expose:

- planner-injected `profile` path overriding explicit runtime identity;
- top-level identity preserved by HTTP but lost by dashboard relay;
- retained profile allocation reporting a false conflict;
- service job trace filters associating the wrong profile with a browser.

## Scenario D | Viewer Contention And External Route Survival

Goal: combine route switching with two dashboard clients, direct Guacamole
popout, mobile viewport resize, refresh, and controller takeover.

Steps:

1. Open Browser A in Client 1.
2. Open Browser B in Client 2.
3. Open Browser A's external Guacamole route from Client 1 and record takeover
   behavior.
4. Switch Client 1 to Browser B while Client 2 remains on Browser B.
5. Refresh both clients, resize one client to a mobile viewport, and switch
   Client 1 back to Browser A.
6. Assert either simultaneous view is preserved or single-viewer takeover is
   explicitly surfaced with a working Take over path.
7. Assert external popout does not cause the owned rail to lose either browser.

Failure classes this should expose:

- viewer lease and controller lease point at stale route after switch;
- iframe route and external route diverge;
- mobile resize drops interaction controls;
- refresh recovers dashboard URL but not selected browser identity.

## Scenario E | Rollback, Proof Failure, And Close Boundaries

Goal: prove failed acquisition cleanup never turns a live browser into an
unreachable or hidden browser.

Steps:

1. Launch Browser A and Browser B on two routes.
2. Force proof failure on a route switch or remote-view open using the existing
   forced proof-failure hook.
3. Assert cleanup closes only the newly opened tab or newly launched browser
   described by the failure, and rolls back route-pool and display allocation
   state.
4. Assert the pre-existing live browser remains reattachable.
5. Close Browser A explicitly and verify:
   - Browser A route, viewer lease, controller lease, and display allocation
     are released;
   - Browser A is no longer reattachable;
   - Browser B remains ready and reattachable.
6. Run `service reconcile` and repeat the final assertions.

Failure classes this should expose:

- rollback restores stale route proof instead of current browser proof;
- forced proof failure leaves pending route-pool entries;
- close of one browser releases another browser's route;
- reconcile resurrects a closed browser as reattachable.

## Scenario F | Dashboard Rail Persistence

Goal: ensure the owned-browser rail is a durable browser inventory, not a view
of currently attached route slots.

Steps:

1. Launch four browsers with two routes.
2. Verify all four appear in the owned rail after dashboard resource refresh.
3. Apply filters and selected workspace navigation across:
   - attached browser;
   - parked browser;
   - stale-route browser;
   - closed browser.
4. Assert parked browsers remain visible with a reattach action, while closed
   browsers are absent or explicitly terminal according to the UI contract.
5. Verify right-pane inspector shows browser identity, route slot identity,
   attachability state, and next repair action.

Failure classes this should expose:

- rail filters hide parked live browsers;
- selected workspace context resets to the wrong browser after resource refresh;
- action buttons enable against stale route ids;
- right pane mixes incident/job action state from another browser.

## No-Launch Coverage

Add focused Rust and client tests for edge states that should not require live
Guacamole:

```bash
cargo test --manifest-path cli/Cargo.toml remote_view_reattach_stress -- --nocapture
cargo test --manifest-path cli/Cargo.toml service_reconcile_reattach_stress -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_profile_identity_stress -- --nocapture
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-view-streams
pnpm test:service-request-client
pnpm test:service-observability-client
pnpm test:service-api-mcp-parity
```

Required no-launch model fixtures:

- four browsers, two routes, two display allocations, two parked browsers;
- stale route owner with live browser stream proof;
- pending acquisition lease older than the active browser proof;
- explicit runtime profile plus conflicting access-plan default;
- viewer/controller lease pointing at a route released by another browser;
- closed browser with still-retained route and display records.

## Live Validation Matrix

Minimum live command set:

```bash
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 \
AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 \
pnpm test:rdp-guac-route-pool-readiness -- --report-only

AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome-stable \
AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser \
AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:10 \
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 \
AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 \
AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> \
pnpm test:p67-rdp-route-churn-soak-live

AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome-stable \
AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser \
AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:10 \
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 \
AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 \
AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> \
pnpm test:p67-rdp-restart-reconcile-live

AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome-stable \
AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser \
AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:10 \
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 \
AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 \
AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> \
pnpm test:p67-rdp-profile-identity-live

AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome-stable \
AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser \
AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:10 \
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 \
AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 \
AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> \
pnpm test:p67-rdp-viewer-contention-live

AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome-stable \
AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser \
AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:10 \
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 \
AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 \
AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> \
pnpm test:p67-rdp-rollback-and-close-live

AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome-stable \
AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser \
AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:10 \
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 \
AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 \
AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> \
pnpm test:p67-rdp-dashboard-rail-persistence-live
```

Run existing regression gates after any P67 implementation slice:

```bash
pnpm test:rdp-two-route-switching-live
pnpm test:rdp-browser-reattach-until-close-live
pnpm test:remote-view-reconcile-reattach-live
pnpm test:dashboard-rdp-reattachable-rail-live
pnpm test:rdp-guac-browser-switch-live
pnpm test:rdp-guac-viewer-transfer-live
pnpm test:rdp-guac-cold-restart-readback-live
pnpm test:remote-view-open-live
```

## Acceptance Criteria

- Four live browsers can share two Guacamole route slots without any live
  browser disappearing from the owned rail before explicit close.
- Repeated switching never produces `display_allocation_owner_mismatch`,
  profile-lock collision, stale route iframe, wrong selected browser, or
  route-pool entry owner drift.
- Restart plus reconcile preserves or reconstructs attachability for live
  browsers from persisted service state.
- Dashboard clients and external Guacamole viewers either simultaneously view
  correctly or expose a working, typed single-viewer takeover path.
- Forced acquisition failure and explicit browser close release only the
  intended route/display/viewer/controller state.
- Every failure produces a precise scenario artifact and classification so the
  next repair can be bounded.

## Implementation Order

1. Add the shared P67 harness skeleton and package scripts with fixture-only
   validation for each mode.
2. Implement no-launch model fixtures for the six edge-state families.
3. Implement `profile-identity` live mode first because it protects the P66
   profile-default repair.
4. Implement `route-churn-soak` next because it is the highest-value gap finder
   for two-route operation.
5. Implement `restart-reconcile` and `rollback-and-close` after churn exposes
   any remaining route-state bugs.
6. Implement `viewer-contention` and `dashboard-rail-persistence` last, because
   they depend on stable route churn and reconcile behavior.
7. Update `docs/dev/notes/` with a dated execution ledger after the first full
   P67 run.

## Execution Ledger

2026-07-05 implementation slice:

- Added shared harness `scripts/test-p67-rdp-stress-hardening-live.js`.
- Added package scripts:
  - `pnpm test:p67-rdp-stress-fixtures`
  - `pnpm test:p67-rdp-route-churn-soak-live`
  - `pnpm test:p67-rdp-restart-reconcile-live`
  - `pnpm test:p67-rdp-profile-identity-live`
  - `pnpm test:p67-rdp-viewer-contention-live`
  - `pnpm test:p67-rdp-rollback-and-close-live`
  - `pnpm test:p67-rdp-dashboard-rail-persistence-live`
- Added no-launch Rust stress fixtures in
  `cli/src/native/remote_view_attachability.rs`:
  - `remote_view_reattach_stress_keeps_four_browsers_visible_with_two_routes`
  - `service_reconcile_reattach_stress_classifies_stale_and_closed_records`
  - `service_profile_identity_stress_preserves_explicit_profile_attachability`
- Passed validation:
  - `pnpm test:p67-rdp-stress-fixtures`
  - `cargo test --manifest-path cli/Cargo.toml stress -- --nocapture`
  - `cargo test --manifest-path cli/Cargo.toml explicit_profile -- --nocapture`
  - `cargo fmt --manifest-path cli/Cargo.toml -- --check`
  - `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
  - `node --check scripts/test-p67-rdp-stress-hardening-live.js`
  - `pnpm test:dashboard-workspace-nodes`
  - `pnpm test:dashboard-workspace-navigator`
  - `pnpm test:dashboard-view-streams`
  - `pnpm test:service-request-client`
  - `pnpm test:service-observability-client`
  - `pnpm test:service-api-mcp-parity`
- Route-pool readiness initially failed because the expected `:13` and `:14`
  X11 sockets were absent. Reopened route displays, removed stale route-viewer
  process `24022`, and validated the live pool on displays `:10` and `:11`.
- Passed P67 live validation:
  - `pnpm test:p67-rdp-route-churn-soak-live`
    (`/tmp/agent-browser-p67-rdp-stress-route-churn-soak-2026-07-05T21-15-46-707Z`)
  - `pnpm test:p67-rdp-restart-reconcile-live`
    (`/tmp/agent-browser-p67-rdp-stress-restart-reconcile-2026-07-05T21-31-41-571Z`)
  - `pnpm test:p67-rdp-profile-identity-live`
    (`/tmp/agent-browser-p67-rdp-stress-profile-identity-2026-07-05T21-39-05-574Z`)
  - `pnpm test:p67-rdp-viewer-contention-live`
    (`/tmp/agent-browser-p67-rdp-stress-viewer-contention-2026-07-05T21-18-28-392Z`)
  - `pnpm test:p67-rdp-rollback-and-close-live`
    (`/tmp/agent-browser-p67-rdp-stress-rollback-and-close-2026-07-05T21-17-06-229Z`)
  - `pnpm test:p67-rdp-dashboard-rail-persistence-live`
    (`/tmp/agent-browser-p67-rdp-stress-dashboard-rail-persistence-2026-07-05T21-17-43-504Z`)
- Passed existing live regression gates after P67 coverage:
  - `pnpm test:rdp-two-route-switching-live`
  - `pnpm test:rdp-browser-reattach-until-close-live`
  - `pnpm test:remote-view-reconcile-reattach-live`
  - `pnpm test:dashboard-rdp-reattachable-rail-live`
  - `pnpm test:rdp-guac-browser-switch-live`
  - `pnpm test:rdp-guac-viewer-transfer-live`
  - `pnpm test:rdp-guac-cold-restart-readback-live`
  - `pnpm test:remote-view-open-live`
- The strengthened `profile-identity` live mode exposed a remaining durable
  gap: service access-plan defaults could inject another browser's
  `runtimeProfile` even when the request carried explicit top-level
  `profileId`. Repaired launch identity normalization in
  `cli/src/native/actions.rs` so explicit `profileId` is treated as an
  explicit runtime identity and blocks default profile injection.
