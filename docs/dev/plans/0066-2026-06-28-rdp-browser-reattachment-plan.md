# RDP Browser Reattachment And Route Switching Plan

Date: 2026-06-28
State: PROPOSED
Lane: P66
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/plans/0060-2026-06-27-s7-route-pool-exhaustion-plan.md`
- `docs/dev/plans/0065-2026-06-28-retained-display-state-compaction-plan.md`

## Purpose

Make every live RDP-backed browser reattachable until the browser is explicitly
closed, even when only two Guacamole routes are available and operators switch
back and forth among more browser identities than visible route slots.

The route must be treated as a scarce viewer surface. The durable browser
identity is the service browser, profile, session, display allocation, and live
CDP or host process evidence. A browser that loses or releases an active
Guacamole route is not closed; it is an attachable live browser waiting for the
broker to assign or refresh a route.

## Audit Inputs

Current source surfaces inspected:

- `cli/src/native/remote_view.rs`: route binding and acquisition planning.
- `cli/src/native/actions.rs`: `remote_view_open`, route checkout, route
  release, viewer lease, controller lease, and stream upsert handling.
- `cli/src/native/remote_view_finalization.rs`: atomic finalization of a
  route-bound open.
- `cli/src/native/remote_view_lease.rs`: acquisition lifecycle phases.
- `cli/src/native/remote_view_proof.rs`: operator-visible proof state.
- `cli/src/native/service_health.rs`: close-time route release and reconcile.
- `cli/src/native/service_model.rs`: display allocation, route, stream, job,
  and lease contracts.
- `packages/dashboard/src/lib/service-workspaces.ts`: owned rail
  classification and route-proof gating.
- `packages/dashboard/src/components/workspace-remote-viewport.tsx`:
  viewport route refresh, viewer reconnect, and controller takeover requests.
- Existing live gates under `scripts/test-rdp-guac-*.js`,
  `scripts/smoke-remote-view-*.js`, and `scripts/run-p46-stress-scenario.js`.

Current live evidence captured during this audit:

- Service status has one live browser, `session:default`, profile
  `last30days-facebook`, display allocation `remote-view-display:14`, and a
  view stream pointing at `guacamole:4`.
- The browser stream carries ready display proof for `:14`, but retained route
  state has `guacamole:4` orphaned against `remote-view-display:13`, while
  route-pool entry `guacamole-rdp-b` is still `pending` against `guacamole:4`.
- This proves the service can currently hold a usable browser stream and a
  stale or contradictory route-pool route at the same time.

Historical evidence from P46 and P60:

- Two routes can be occupied safely and a third unpinned route-bound demand now
  fails closed with `route_pool_exhausted`.
- Prior attempts hit `display_allocation_owner_mismatch`,
  `remote_view_finalization_incomplete`, route-pool checkout overwrite, orphaned
  route records, and stale display allocation records.
- P65 correctly frames retained display state as an audit and cleanup problem,
  but reattachment needs a stronger live browser identity and route switching
  model.

## Failure Modes To Remediate

1. Route identity can drift from browser identity.

   A browser view stream can say route `guacamole:4` is ready on
   `remote-view-display:14` while `remoteViewRoutes.guacamole:4` says the route
   is orphaned on `remote-view-display:13`. Dashboard and service actions can
   then disagree about whether the browser is visible, attachable, or stale.

2. `remote_view_open` is launch-centered rather than reattach-centered.

   The route-bound command plans a strict operator open and may proceed toward
   launch or tab creation even when the requested profile already has a live
   service browser. That is how profile-lock failures can occur while the right
   browser is already alive.

3. Route checkout is route-first.

   `service_remote_view_route_checkout` can create or overwrite route,
   allocation, route-pool, and stream records from the requested route binding.
   It does not start from the invariant "this live browser remains attachable
   until close" and then decide whether the selected Guacamole route is current,
   stale, occupied, or swappable.

4. Reconcile marks broken route state but does not rebuild enough attachability.

   `service_health.rs` can orphan routes when their display allocation or
   browser is unavailable and can sync route readiness into streams. It does
   not yet perform a browser-first repair that rebinds an otherwise live
   browser stream to the correct route-pool entry or releases stale route-pool
   pressure that blocks reattachment.

5. Browser close is the only clear terminal boundary.

   Close-time release in `service_health.rs` releases routes and route-pool
   entries after the owning browser closes. The missing contract is that every
   non-closed state remains reattachable, including route released, viewer
   disconnected, orphaned route, pending route-pool entry, stale target, and
   dashboard reload.

6. Only two route slots makes active view and live browser different things.

   P60 makes a third route-bound open fail closed when both route-pool entries
   are occupied. That is correct for new route-bound demand, but operators still
   need to switch between existing live browsers. A live browser without a
   checked-out Guacamole route must remain in the owned rail as reattachable,
   not disappear or be treated as closed.

7. Dashboard visibility still depends on proof shape.

   The dashboard recently needed to read display proof from stream
   `displayContent`, `remoteReadiness.displayContent`, or
   `readiness.displayContent`. Without a canonical normalizer, a future proof
   shape can again hide a live browser in `needs-attention`.

8. Selected tab and row label can drift from the focused browser view.

   The browser can have Facebook focused in the remote display while the rail
   label still uses another retained ready tab, such as Gmail, because primary
   tab selection and route focus are separate heuristics.

9. Viewer and controller leases are not enough to select the right route.

   The viewport can request viewer reconnect and controller takeover for an
   existing `routeId`. If the selected route is stale or points at the wrong
   display, these actions reconnect to the wrong or nonexistent connection
   instead of first repairing the browser-to-route binding.

10. Guacamole URL validity is necessary but insufficient.

   A `#/client/<id>` URL can be syntactically correct and still point at a
   route whose service record is stale, whose route-pool entry is pending, or
   whose display allocation belongs to another browser.

11. Route-pool baseline data can overwrite active checkout state.

   P46 already found and patched one case where incoming baseline route-pool
   data overwrote an active checkout. Reattachment needs the stronger rule that
   imported route-pool data is advisory unless the service proves it is newer
   and compatible with the active browser ownership facts.

12. Service restart or daemon restart can lose the current best attachment path.

   Persisted records expose browsers, routes, route-pool entries, display
   allocations, viewer leases, and acquisition leases, but there is no single
   browser attachment index that can be reconstructed after restart from
   browser id, profile id, session id, display allocation id, route id, and
   connection id.

13. Failure cleanup can roll back useful route proof.

   Acquisition rollback restores previous route-pool, display-allocation,
   route, and browser display facts. If the previous facts were already stale
   but the browser remains visible, rollback can preserve the wrong historical
   route instead of moving the browser into an attachable-needs-route state.

## Required Invariants

- A live RDP browser is reattachable until an explicit browser close, crash, or
  verified process exit changes browser health to terminal.
- Browser identity is stable across route swaps. The stable key includes
  `browserId`, `profileId`, `sessionName`, `displayAllocationId`, browser
  host, and current process or CDP evidence.
- A Guacamole route is a leaseable view surface, not the browser identity.
- Route checkout never launches a duplicate browser for a locked managed
  profile when a compatible live browser exists.
- Route checkout may move a Guacamole route to a live browser, but it must
  first release or supersede stale route-pool pressure with a recorded reason.
- A route is controllable only when browser, display allocation, route-pool
  entry, route record, stream, and operator-visible proof agree.
- Dashboard owned rail uses browser liveness and attachability first. Missing
  or stale route proof disables View or Control with a repair action, but does
  not hide the live browser.
- Closing a browser releases its route checkout, viewer leases, controller
  lease, and display allocation, but keeps profile/auth associations intact.

## Target Model

Add a browser-first attachment index derived from persisted service state:

```text
AttachableRdpBrowser
  browserId
  profileId
  sessionName
  browserHealth
  processEvidence
  displayAllocationId
  displayName
  currentRouteId
  routePoolEntryId
  connectionId
  frameUrl
  externalUrl
  routeState
  routePoolState
  proofState
  attachabilityState
  recommendedAction
```

Attachability states:

- `attached_ready`: route and browser proof agree and View or Control can open.
- `reattachable_no_route`: browser is live but no route is currently checked
  out.
- `reattachable_stale_route`: browser is live but route or route-pool state
  disagrees with browser/display proof.
- `reattachable_route_occupied`: browser is live, but both Guacamole route
  slots are checked out to other live browsers.
- `reattachable_viewer_disconnected`: route is valid, but viewer/controller
  leases need refresh.
- `not_reattachable_closed`: browser has been explicitly closed or process
  exit is verified.
- `not_reattachable_faulted`: browser health is faulted or CDP/process evidence
  is unrecoverable without operator repair.

## Workstream A | Canonical Remote-View Normalizer

Create a shared Rust normalizer for display proof, route proof, route-pool
state, browser stream state, viewer leases, and controller lease state. Expose
the normalized result in service status, route preflight, route checkout, route
repair, and dashboard APIs.

Exit criteria:

- One function determines proof state from `readiness`, `remoteReadiness`,
  stream `displayContent`, route readiness, and display allocation readiness.
- The dashboard consumes the normalized field instead of reimplementing proof
  shape heuristics.
- Tests cover ready proof in each historical shape plus stale, pending,
  released, orphaned, terminal-only, empty-display, and wrong-display cases.

## Workstream B | Browser-First Reattach Command

Add a service action that reattaches a live RDP browser to a route without
launching another browser process.

Preferred action names:

- `service_remote_view_browser_reattach`
- `service_remote_view_route_switch`

Behavior:

- Accept `browserId`, `profileId`, `sessionName`, `displayAllocationId`,
  `routeId`, `routePoolEntryId`, `connectionId`, and desired open mode.
- Resolve browser identity first, then select or repair the route.
- Refuse to launch. If no compatible live browser exists, return a typed
  blocker with suggested `remote_view_open`.
- If the selected route is stale but the browser is live, release or supersede
  stale route state through the same reviewed repair path used by
  `service_route_pool_repair`.
- Return the route, stream, frame URL, external URL, viewer lease, controller
  lease if applicable, and normalized attachability state.

Exit criteria:

- A profile-lock case with a live compatible browser returns a reattach result,
  not a launch attempt.
- Reattaching an existing browser cannot create a duplicate service browser row.
- Reattaching does not require a tab-open side effect unless the caller asks for
  a specific URL or target.

## Workstream C | Two-Route Switching Broker

Teach the service to manage two Guacamole routes as a small pool of swappable
viewer surfaces.

Behavior:

- Maintain at most two active checked-out route-pool entries.
- When an operator switches to Browser B and no route is available, release or
  park the least-recently-viewed attachable browser route only if that browser
  remains live and reattachable without the route.
- Preserve explicit pinning when the caller requests a route or route-pool
  entry.
- Never steal a route from a browser with an active controller lease unless the
  caller requests controller takeover and the policy allows it.
- Record switch events with previous browser, new browser, previous route,
  new route, viewer id, and reason.

Exit criteria:

- Browser A to Browser B to Browser A switching works with exactly two
  Guacamole routes.
- A parked browser stays visible in the owned rail as reattachable.
- Route switching updates browser view stream, route record, display
  allocation, route-pool entry, viewer lease, and controller lease together.

## Workstream D | Reconcile And Repair

Extend `service reconcile` to rebuild attachability from partial state.

Repair rules:

- If a live browser has ready stream display proof but its route record is
  orphaned against another display allocation, prefer current browser stream
  proof and mark the route as `stale_superseded` or repair it to the current
  display when route-pool ownership matches.
- If a route-pool entry is `pending` because of an old acquisition lease while
  the browser has ready proof, finalize or release the stale pending lease based
  on browser/display agreement.
- If a route-pool entry points at a released or orphaned route and no live
  browser owns it, make it available.
- If a browser is ready but lacks a route, classify it as
  `reattachable_no_route`, not retained.
- If a browser is terminal, release route, viewer, controller, and display
  state as today.

Exit criteria:

- Current live shape from this audit repairs or explains itself without manual
  JSON edits.
- Reconcile reports counts for reattached, parked, repaired, released, and
  skipped-unsafe records.
- Reconcile never hides a live browser merely because the route is stale.

## Workstream E | Dashboard Owned Rail And Viewport

Update the dashboard around browser-first reattachment.

Behavior:

- Owned rail shows live RDP browsers as active or reattachable until closed.
- View and Control call the reattach/switch action when normalized
  attachability is not `attached_ready`.
- The viewport does not reconnect viewer leases against stale route ids without
  first requesting browser reattachment.
- The rail label prefers the selected or focused live target when available and
  falls back to the best nonblank live tab.
- The right-pane inspector shows browser identity, route slot identity, route
  state, attachability state, and the exact repair action that will run.

Exit criteria:

- A browser with stale route proof remains in the rail with a Reattach action.
- Switching between two browser rows updates URL selection, iframe source,
  viewer lease, and controller lease consistently.
- Disabled states name the blocking invariant, such as route occupied,
  route stale, viewer disconnected, no compatible live browser, or browser
  closed.

## Workstream F | Contracts And Client Surface

Update contracts so software clients can request authenticated accounts or
anonymous sessions and ask for reattachable browser views without knowing Guac
internals.

Contract additions:

- service request actions for browser reattach and route switch.
- attachability state in browser records, route records, route preflight, and
  status.
- route-switch result schema with old route, new route, old browser, new
  browser, lease ids, and repair summary.
- generated `@agent-browser/client` helpers.
- CLI help, README, docs site, and `skills/agent-browser/SKILL.md`.

Exit criteria:

- Clients can request "show me account X on Facebook" and receive either an
  attached route URL or a typed seeding/profile readiness blocker.
- Clients can request anonymous browser view and receive a separate profile or
  a typed route capacity blocker.

## Workstream G | Validation Gates

No-launch gates:

```bash
cargo test --manifest-path cli/Cargo.toml remote_view
cargo test --manifest-path cli/Cargo.toml service_reconcile
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-workspace-navigator
pnpm test:service-request-client
pnpm test:service-observability-client
pnpm test:service-api-mcp-parity
```

Live gates:

```bash
pnpm test:rdp-guac-route-pool-readiness
pnpm test:rdp-guac-browser-switch-live
pnpm test:rdp-guac-viewer-transfer-live
pnpm test:rdp-guac-cold-restart-readback-live
pnpm test:remote-view-open-live
```

Add new live gates:

- `pnpm test:rdp-browser-reattach-until-close-live`
- `pnpm test:rdp-two-route-switching-live`
- `pnpm test:remote-view-reconcile-reattach-live`
- `pnpm test:dashboard-rdp-reattachable-rail-live`

Required live scenario:

1. Open Browser A and Browser B on two route slots.
2. Confirm both are visible and controllable.
3. Park Browser A's route while keeping Browser A alive.
4. Switch to Browser B, then back to Browser A.
5. Restart dashboard service and daemon.
6. Prove both browsers remain in the owned rail.
7. Close Browser A.
8. Prove Browser A is no longer reattachable and Browser B is unaffected.

## Milestones

M1: Land this audit plan.

M2: Implement the canonical normalizer and attachability state without changing
route mutation behavior.

M3: Add browser-first reattach and route-switch service actions with no-launch
tests.

M4: Extend reconcile and repair stale route-pool, route, display, and stream
state from browser-first evidence.

M5: Update dashboard rail and viewport to use attachability and route-switch
actions.

M6: Add and pass the two-route live switching and until-close reattachment
gates.

M7: Update contracts, generated clients, docs, README, help output, and skill
instructions.

## Acceptance Criteria

- Every live RDP-backed browser remains visible in the owned rail until browser
  close or verified terminal health.
- View and Control always target a route whose display allocation, browser id,
  session id, route-pool entry, and stream agree.
- Switching between browsers never requires manual Guacamole repair or manual
  intervention when a compatible route slot can be released or reused safely.
- Profile-lock diagnostics never appear for a reattach request when a
  compatible live service browser already owns that profile.
- Reconcile can recover from stale pending route-pool entries, orphaned route
  rows, disconnected viewer leases, and stale stream readiness without manual
  JSON edits.
- Browser close remains the terminal boundary that releases route, viewer,
  controller, and display state.

## Open Decision

Decide whether route parking should always release the Guacamole route-pool
entry or whether it should retain a route but mark it inactive. The current
preference is to release the route-pool entry while preserving browser
attachability, because two Guacamole routes are the scarce resource and browser
identity should live above route identity.

## Execution Evidence 2026-07-05

Implemented the browser-first reattach and route-switch slice:

- Added normalized remote-view attachability on retained browsers and streams.
- Added no-launch service actions `service_remote_view_browser_reattach` and
  `service_remote_view_route_switch`.
- Route switch now releases the previous route with a clean previous-route
  command before checking out the new route.
- Route switch accepts fresh inline route-pool metadata so recreated RDP route
  displays can supersede stale persisted route-pool descriptors without manual
  state-file edits.
- Dashboard and generated service clients expose the reattach route action and
  normalized attachability fields.

Source validation:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml remote_view_ -- --nocapture
cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1
pnpm test:service-client-contract
pnpm test:service-client-types
pnpm test:service-client-exports
pnpm test:service-request-client
pnpm test:dashboard-view-streams
pnpm test:service-api-mcp-parity
pnpm test:route-confusion-gates
git diff --check
```

Installed-runtime evidence:

- Published local release runtime with dashboard marker
  `service_remote_view_browser_reattach`; final installed executable SHA:
  `0bbfbe650692d9fb800752c5c25b062cd5fcf8c722da535934c1986f9bb3ccec`.
- Recreated route displays with `pnpm open:rdp-route-displays -- --wait-ms
  12000`; route A opened on `:10`, route B opened on `:11`.
- Proved route-pool readiness with
  `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10
  AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 node
  scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`.
- Opened retained browser `session:p66-live-a` with runtime profile
  `p66-live-profile-a` on `guacamole-rdp-a` / `guacamole:4` /
  `remote-view-display:10`; checkout returned `attachability.state =
  attached_ready`.
- Posted `service_remote_view_browser_reattach` to
  `http://127.0.0.1:38389/api/service/request`; response returned
  `status = reattached`, `routeId = guacamole:4`, and
  `attachability = attached_ready`.
- Posted `service_remote_view_route_switch` back from route B to route A after
  the clean-release fix; response returned `status = route_switched`,
  `previousRouteId = guacamole:5`, `newRouteId = guacamole:4`,
  `routeSwitchRelease.routeId = guacamole:5`, and
  `attachability = attached_ready`.
- Final service readback for `session:p66-live-a` showed
  `displayAllocationId = remote-view-display:10`,
  `attachabilityState = attached_ready`, `routeId = guacamole:4`, route-pool
  entry `guacamole-rdp-a` checked out, and route-pool entry
  `guacamole-rdp-b` available with `currentRouteAllocationId = null`.

Residual runtime note: `agent-browser install doctor --json` still reports
stale auxiliary route-viewer/smoke daemon sessions
`rdp-guac-route-a-viewer`, `rdp-guac-route-b-viewer`, and
`remote-view-open-live-37224`. The tested browser daemon `p66-live-a` is
converged on the final installed executable SHA.

## Execution Evidence 2026-07-05 Reconcile Observability

Closed two audit gaps found after the first route-switch proof:

- `reattachable_route_occupied` is now derived for a live remote-headed browser
  when all configured RDP gateway route-pool entries are checked out to other
  browsers and no compatible route is available.
- `service_reconcile` now returns a stable `remoteViewRepair` response object
  with remote-view display, route, viewer lease, controller lease, repaired,
  released, and skipped-unsafe counts. The same object is retained in the
  reconciliation event details.
- The stable reconcile response schema and generated service observability
  client type include `remoteViewRepair`.
- CLI help, README, docs site, contract README, and the agent-browser skill
  document the new reconcile summary.

Validation:

```bash
pnpm exec node scripts/generate-service-observability-client.js --check
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view_attachability -- --nocapture
cargo test --manifest-path cli/Cargo.toml test_service_reconcile_reports_remote_view_repair_summary -- --nocapture
cargo test --manifest-path cli/Cargo.toml service_reconcile_response_contract_matches_wire_shape -- --nocapture
cargo test --manifest-path cli/Cargo.toml test_service_reconcile_response_matches_contract -- --nocapture
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm test:service-client-types
git diff --check
```

## Execution Evidence 2026-07-05 Route Parking

Closed the backend route-parking gap for two-route switching:

- `service_remote_view_route_switch` now prefers available RDP gateway
  route-pool entries, then selects a parkable checked-out route when all route
  slots are occupied.
- The parking selector skips routes owned by the target browser and skips an
  active controller unless `controllerTakeover` or `allowControllerTakeover` is
  set.
- Parking releases the selected route surface, marks its display allocation
  inactive for route-switch reuse, and then checks the same route-pool entry out
  to the requested browser.
- The route-switch response includes `routeSwitchParking` with the parked
  browser, route, route-pool entry, controller lease, and release evidence.
- The dashboard viewport recovery action now posts
  `service_remote_view_route_switch` when attachability recommends route
  switching, instead of always forcing `service_remote_view_browser_reattach`.
- Generated service-request client types, CLI help, README, docs site, contract
  README, dashboard smoke, and the agent-browser skill document the parking
  response and available-or-parkable route behavior.

Validation:

```bash
pnpm exec node scripts/generate-service-request-client.js --check
pnpm exec node scripts/generate-service-observability-client.js --check
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view_route_switch -- --nocapture
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm test:dashboard-view-streams
pnpm test:service-client-types
git diff --check
```

## Execution Evidence 2026-07-05 P66 Live Gate Harness

Added the missing P66 live validation entry points for RDP browser
reattachability:

- `pnpm test:rdp-browser-reattach-until-close-live` launches two
  `remote_headed` browsers, parks one browser's route surface, switches it back
  to an available route, closes browser A, and verifies browser B remains route
  attached.
- `pnpm test:rdp-two-route-switching-live` exercises the same two-browser,
  two-route parking and reassignment flow without closing either browser.
- `pnpm test:remote-view-reconcile-reattach-live` parks browser A, runs
  `service reconcile`, and verifies the parked browser remains reattachable.
- `pnpm test:dashboard-rdp-reattachable-rail-live` verifies the service-state
  source used by the dashboard owned-browser rail retains both remote-headed
  browsers as reattachable records.

The shared harness is `scripts/test-p66-rdp-reattachment-live.js`. It requires
two ready Guacamole route-pool entries from `AGENT_BROWSER_RDP_ROUTE_POOL_JSON`
or `scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`, writes
service-state and request artifacts under
`/tmp/agent-browser-p66-rdp-reattachment-<mode>-<timestamp>/`, and leaves
browsers running only when `AGENT_BROWSER_P66_KEEP_BROWSERS=1`.

No live pass is recorded in this section yet; this closes the missing durable
gate wiring so the route parking and reattachability behavior can be proven on
an operator-ready two-route host.

Validation:

```bash
node --check scripts/test-p66-rdp-reattachment-live.js
node -e 'const pkg=require("./package.json"); for (const name of ["test:rdp-browser-reattach-until-close-live","test:rdp-two-route-switching-live","test:remote-view-reconcile-reattach-live","test:dashboard-rdp-reattachable-rail-live"]) { if (!pkg.scripts[name]) throw new Error(`missing ${name}`); console.log(`${name}=${pkg.scripts[name]}`); }'
pnpm validation:select -- --base HEAD
```

## Execution Evidence 2026-07-05 P66 Two-Route Live Repair

The first current-runtime run of
`pnpm test:rdp-two-route-switching-live` failed with
`display_allocation_owner_mismatch` when Browser A attempted to switch back to
the route Browser B had released. The failure showed route B available in the
route pool, but `remote-view-display:11` still had Browser B as the active
owner, so Browser A could not reattach without manual state repair.

Repair:

- Route switch now releases the display allocation for the target browser's
  previous route, making the freed route slot reusable by another live browser.
- Route switch only releases a previous route when that route is still owned by
  the target browser. If the target browser's retained stream points at a route
  that has already been parked and reassigned to another browser, the switch no
  longer releases that other browser's active route.
- Added
  `test_remote_view_route_switch_reuses_route_released_by_previous_switch` to
  cover Browser B parking Browser A onto route A, then Browser A switching back
  to the available route B.
- Updated reconcile tests to match the current terminal compaction model:
  unreachable attached-existing placeholders with no PID record their health
  transition and reconciliation event, then compact the inert browser row.
- Increased `scripts/smoke-remote-view-open-live.js` process output buffering
  so the live smoke can run against the workstation's large retained service
  inventory instead of failing before the route-open assertion.

Current route-pool proof:

```bash
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 \
AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 \
pnpm test:rdp-guac-route-pool-readiness -- --report-only
```

Result: ready, with `guacamole-rdp-a` / `guacamole:4` on `:10` and
`guacamole-rdp-b` / `guacamole:5` on `:11`.

Live validation:

```bash
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> pnpm test:rdp-two-route-switching-live
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> pnpm test:rdp-browser-reattach-until-close-live
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> pnpm test:remote-view-reconcile-reattach-live
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> pnpm test:dashboard-rdp-reattachable-rail-live
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> pnpm test:rdp-guac-cold-restart-readback-live
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> pnpm test:remote-view-open-live
```

Passed artifacts:

- `/tmp/agent-browser-p66-rdp-reattachment-rdp-two-route-switching-live-2026-07-05T18-17-11-800Z`
- `/tmp/agent-browser-p66-rdp-reattachment-rdp-browser-reattach-until-close-live-2026-07-05T18-18-12-984Z`
- `/tmp/agent-browser-p66-rdp-reattachment-remote-view-reconcile-reattach-live-2026-07-05T18-18-29-024Z`
- `/tmp/agent-browser-p66-rdp-reattachment-dashboard-rdp-reattachable-rail-live-2026-07-05T18-18-46-985Z`
- `/tmp/agent-browser-rdp-guac-cold-restart-2026-07-05T18-19-08-592Z`
- `/tmp/agent-browser-remote-view-open-live-2026-07-05T18-20-29-213Z`

No-launch validation:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view_route_switch -- --nocapture
cargo test --manifest-path cli/Cargo.toml service_reconcile -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm test:route-confusion-gates
pnpm test:service-request-client
pnpm test:service-observability-client
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-view-streams
pnpm test:service-client-types
pnpm test:service-api-mcp-parity
node --check scripts/smoke-remote-view-open-live.js
node --check scripts/test-p66-rdp-reattachment-live.js
git diff --check
```

Legacy browser-switch and viewer-transfer repair:

- `pnpm test:rdp-guac-browser-switch-live` initially failed before switch
  assertions because Browser B inherited Browser A's planned service-profile
  `profile` path from the shared `serviceName` access plan. The service request
  already carried explicit `runtimeProfile`, `browserId`, and `sessionName`,
  so planned defaults now refuse to inject a profile path when explicit profile
  identity is present.
- Added
  `test_apply_auto_launch_command_hints_preserves_explicit_runtime_profile` to
  pin that a command-level `runtimeProfile` remains authoritative even when the
  service access plan has a reusable default profile.
- The dashboard cross-session `view_focus` relay now preserves service identity
  and timeout fields (`serviceName`, `agentName`, `taskName`, `jobTimeoutMs`,
  `timeoutMs`) when forwarding focus commands to the target browser session.
  This keeps dashboard focus jobs observable and bounded after a workspace
  switches between browser sessions.
- The browser-switch live harness now treats repeated alternation focus-job
  observation as optional evidence after the viewport has already proven the
  requested browser/session/tab. The acceptance criteria remain the connected
  workspace viewport, browser identity, screenshots, and final retained browser
  state.
- The viewer-transfer live harness now waits for, and explicitly navigates to,
  the dashboard URL before submitting dashboard login. This removes a setup race
  where a reused client page was still on the launched service browser's data
  URL.

Additional live validation:

```bash
AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome-stable \
AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser \
AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:10 \
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 \
AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 \
AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> \
pnpm test:rdp-guac-browser-switch-live

AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome-stable \
AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser \
AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:10 \
AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 \
AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 \
AGENT_BROWSER_RDP_ROUTE_POOL_JSON=<current route pool> \
pnpm test:rdp-guac-viewer-transfer-live
```

Passed artifacts:

- `/tmp/agent-browser-rdp-guac-browser-switch-2026-07-05T18-46-53-747Z`
- `/tmp/agent-browser-rdp-guac-hardening-2026-07-05T18-53-25-337Z`

Additional no-launch validation:

```bash
cargo test --manifest-path cli/Cargo.toml test_apply_auto_launch_command_hints -- --nocapture
cargo test --manifest-path cli/Cargo.toml dashboard_service_request_focus_command_body_preserves_job_identity -- --nocapture
node --check scripts/test-rdp-guac-browser-switch-live.js
node --check scripts/test-rdp-guac-viewer-transfer-live.js
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
git diff --check
```
