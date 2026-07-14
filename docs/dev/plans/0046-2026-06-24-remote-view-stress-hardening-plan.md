# Remote View Stress Hardening Plan

Date: 2026-06-24
State: COMPLETE
Lane: P46
Depends On:
- `docs/dev/plans/0044-2026-06-22-rdp-browser-deterministic-refactor-plan.md`
- `docs/dev/plans/0045-2026-06-23-remote-view-architecture-consolidation-plan.md`
- `docs/dev/notes/2026-06-23-p44-fresh-session-handoff.md`
- `docs/dev/plans/0048-2026-06-24-route-bound-finalization-deepening-plan.md`
- `docs/dev/plans/0049-2026-06-26-p46-s3-remediation-plan.md`
- `docs/dev/plans/0050-2026-06-26-s3-binary-authority-visible-window-plan.md`
- `docs/dev/plans/0052-2026-06-27-s3-clearance-pathway-plan.md`
- `docs/dev/plans/0053-2026-06-27-s4-single-profile-window-topology-plan.md`
- `docs/dev/plans/0054-2026-06-27-s5-viewer-client-port-allocation-plan.md`
- `docs/dev/plans/0055-2026-06-27-s6-dashboard-swap-navigation-plan.md`
- `docs/dev/plans/0060-2026-06-27-s7-route-pool-exhaustion-plan.md`
- `docs/dev/plans/0061-2026-06-27-s8-display-access-recovery-plan.md`
- `docs/dev/plans/0062-2026-06-27-s9-stale-target-recovery-plan.md`
- `docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md`
- `docs/dev/plans/0064-2026-06-27-s11-stale-url-live-target-recovery-plan.md`

## Purpose

Harden remote viewing until the operator experience "just works" across
increasingly complex multi-operator, multi-tab, multi-window, and multi-profile
states. The test campaign must prove that route selection, permissions,
display binding, profile reuse, tab selection, browser-window control, and
dashboard UX controls behave predictably without excuses or manual cleanup.

This plan is intentionally operational. It should iterate through states that
are simple enough to diagnose at first, then complex enough to expose real
route, permission, display, lease, profile, and UI ownership bugs.

## Non-Negotiable Rules

- Reset runtime state between every scenario.
- Every scenario must include live visual confirmation through the dashboard or
  Guacamole route, not only JSON success.
- Every scenario must prove the visible controls are functional. At minimum,
  focus, navigate, open tab, switch tab, and close or release controls must be
  exercised when the scenario exposes those controls.
- Any failure immediately triggers an audit and planning phase before the next
  execution attempt.
- Two consecutive failures in the same scenario lock the plan. Stop execution
  and return to chat planning with the maintainer before more live retries.
- Do not manually edit service state to pass a scenario. Use service actions,
  close commands, prune or repair commands, and explicit route cleanup actions.
- Do not accept terminal-only, terminal-topmost, stale target, stale profile,
  stale route-pool, stale display, or permission-denied states as partial
  success.
- Do not change route mode, provider mode, profile assignment, or dashboard
  classification silently to make a case pass.

## Source Of Truth

Use current repo and live runtime state, not old notes, when executing the
campaign:

- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `agent-browser --json service status`
- `agent-browser --json service incidents --summary`
- `node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`
- `node scripts/inspect-rdp-route-displays.js --display-content`
- dashboard API and browser screenshots for UX readback
- Guacamole route URLs for direct operator readback

Graphiti memory is advisory only. CodeGraph should be used for structural
source exploration when tools are exposed. If CodeGraph is unavailable, record
the fallback and use direct source reads plus focused tests.

## Reset Protocol

Every scenario starts from a clean, explicit baseline.

1. Capture pre-reset evidence:
   - service status;
   - incident summary;
   - route-pool readiness;
   - display-content inspection;
   - active browser and session inventory.
2. Close service-owned browsers created by the previous scenario:
   - use `agent-browser --json --session <session> close`;
   - use service close or release actions where the scenario creates multiple
     sessions;
   - do not kill processes unless the service action fails and the failure is
     being audited.
3. Release or verify route-pool entries:
   - route A and route B must return to the expected available or intended
     checked-out state before the next case;
   - stale route allocation IDs must be resolved through service actions.
4. Run garbage collection or retained-state cleanup only through supported
   commands:
   - `agent-browser --json service gc --dry-run`;
   - `agent-browser service prune-retained --dry-run`;
   - apply variants require an explicit review note in the scenario artifact.
5. Verify baseline:
   - install doctor is green or any non-green issue is explained and unrelated
     to the next scenario;
   - remote-view doctor is green;
   - route displays are browser-ready and terminal-free;
   - active incidents are zero;
   - dashboard live rail has no stale retained control rows.

If reset cannot reach baseline, do not continue to the next scenario. Treat the
reset failure as the scenario failure and enter the audit protocol.

## Audit Protocol

On the first failure in a scenario:

1. Stop the scenario without retrying blindly.
2. Save artifacts under `/tmp/agent-browser-p46-<scenario>-<timestamp>/`.
3. Capture:
   - command JSON output;
   - service status;
   - incident summary and activity for related incident IDs;
   - route-pool readiness;
   - display-content inspection;
   - dashboard screenshot;
   - Guacamole screenshot or HTML route readback;
   - browser target list and selected target readback when applicable.
4. Classify the failure:
   - route selection;
   - route pool state;
   - display access or X11 permission;
   - browser launch or attach;
   - profile lock or profile mismatch;
   - tab acquisition or stale target;
   - operator-visible proof;
   - dashboard inventory or live rail;
   - UX control action;
   - incident cleanup or stale retained state;
   - test harness defect.
5. Write a short audit note in the scenario artifact directory.
6. Update this plan or a follow-up note with the repair plan before the next
   execution attempt.

On the second consecutive failure in the same scenario, mark the campaign
locked and return to chat planning with the maintainer. Do not continue down
the matrix.

## Lock Record

Locked on 2026-06-24 during S2 after two consecutive failures.

Evidence:

- first S2 attempt artifact:
  `/tmp/agent-browser-p46-s2-2026-06-24T18-51-22-187Z`;
- second S2 attempt artifact:
  `/tmp/agent-browser-p46-s2-2026-06-24T18-56-05-128Z`;
- cleanup evidence:
  `/tmp/agent-browser-p46-status-after-s2-lock-cleanup.json`.

The first S2 attempt proved that two dashboard viewers can observe and refresh
the same route-bound browser, but the harness incorrectly used service-owned
`agent-browser` sessions as dashboard operator browsers. Those operator
sessions generated route-pool and faulted-browser incidents, which was a real
test-design failure for P46. The incidents were resolved through service
actions before retry.

The second S2 attempt moved operator viewers to external Chromium contexts, but
operator A's external Chromium instance did not become ready over CDP. Because
that was the second consecutive S2 failure, no further scenario execution is
allowed until a maintainer planning discussion decides the next S2 harness and
runtime-audit approach.

## Replan Record

Replanned on 2026-06-26 after P48 completed the source-backed route-bound
finalization repair and fresh-session S2 retry.

P48 unlock evidence:

- P48 state is complete in
  `docs/dev/plans/0048-2026-06-24-route-bound-finalization-deepening-plan.md`;
- fresh-session preflight artifact:
  `/tmp/agent-browser-p48-goal6-preflight-20260625T141308Z`;
- passed S2 retry artifact:
  `/tmp/agent-browser-p46-s2-2026-06-25T14-13-40-415Z`.

Execution resumed at S3. S0 through S2 are treated as baseline evidence, not
full campaign completion. The original lock rule still applies: any failure
gets an artifact-backed audit before retry; if the same scenario fails twice
and cannot be repaired agentically, stop execution and return to maintainer
planning.

## Current Execution Ledger

- S0 passed on 2026-06-24:
  `/tmp/agent-browser-p46-s0-2026-06-24T18-42-30-422Z`.
- S1 failed once on 2026-06-24 due to harness URL parsing, then passed:
  `/tmp/agent-browser-p46-s1-2026-06-24T18-46-11-031Z`.
- S2 failed twice on 2026-06-24, which locked the campaign.
- S2 passed after P48 repair on 2026-06-25:
  `/tmp/agent-browser-p46-s2-2026-06-25T14-13-40-415Z`.
- S3 attempt 1 failed on 2026-06-26:
  `/tmp/agent-browser-p46-s3-2026-06-26T12-59-10-995Z`.
- S3 attempt 2 failed on 2026-06-26:
  `/tmp/agent-browser-p46-s3-2026-06-26T13-03-59-219Z`.
- S3 was cleared by P52 on 2026-06-27:
  - narrow `s3-open` pass:
    `/tmp/agent-browser-p46-s3-open-2026-06-27T16-36-31-141Z`;
  - full S3 pass:
    `/tmp/agent-browser-p46-s3-2026-06-27T16-37-24-659Z`.
- S4 attempt 1 failed on 2026-06-27:
  `/tmp/agent-browser-p46-s4-2026-06-27T18-32-23-755Z`.
- S4 attempt 2 failed on 2026-06-27:
  `/tmp/agent-browser-p46-s4-2026-06-27T18-34-00-072Z`.
- S4 was locked by the two-consecutive-failure rule until P53 selected and
  implemented the supported one-process, one-route, same-profile window
  topology.
- S4 passed on 2026-06-27:
  `/tmp/agent-browser-p46-s4-2026-06-27T19-12-55-449Z`.
- S5 attempt 1 failed on 2026-06-27:
  `/tmp/agent-browser-p46-s5-2026-06-27T19-22-25-831Z`.
- S5 attempt 1 was classified as a route-pool persistence merge defect and
  repaired before retry.
- S5 attempt 2 failed on 2026-06-27:
  `/tmp/agent-browser-p46-s5-2026-06-27T19-32-56-059Z`.
- P46 was locked at S5 by the two-consecutive-failure rule.
- P54 repaired the viewer-client DevTools port allocation path and passed
  no-live validation. One S5 retry is authorized from the explicit
  rebuilt-binary lane after live baseline preflight.
- S5 passed after the P54 repair on 2026-06-27:
  `/tmp/agent-browser-p46-s5-2026-06-27T19-41-29-598Z`.
- S6 attempt 1 failed on 2026-06-27:
  `/tmp/agent-browser-p46-s6-2026-06-27T19-49-19-793Z`.
- S6 attempt 1 was classified as a viewer-client adapter hang before swapped
  dashboard selection evidence. `scripts/lib/p47-viewer-client.js` now bounds
  CDP commands with a 30000ms timeout before retry.
- S6 attempt 2 failed on 2026-06-27:
  `/tmp/agent-browser-p46-s6-2026-06-27T19-56-33-105Z`.
- P46 was locked at S6 by the two-consecutive-failure rule.
- P55 repaired the dashboard swap navigation path and preserved the
  retained-browser reset repair. One S6 retry is authorized from the explicit
  rebuilt-binary lane after live baseline preflight.
- P59 cleared S6 on 2026-06-27:
  `/tmp/agent-browser-p46-s6-2026-06-27T20-32-54-709Z`.
- P60 cleared S7 on 2026-06-27:
  `/tmp/agent-browser-p46-s7-2026-06-27T20-58-30-721Z`.
- P61 cleared S8 on 2026-06-27:
  `/tmp/agent-browser-p46-s8-2026-06-27T21-07-22-844Z`.
- P62 cleared S9 on 2026-06-27:
  `/tmp/agent-browser-p46-s9-2026-06-27T22-03-14-950Z`.
- P63 cleared S10 on 2026-06-27:
  `/tmp/agent-browser-p46-s10-2026-06-27T22-52-43-936Z`.
- P64 cleared S11 on 2026-06-27:
  `/tmp/agent-browser-p46-s11-2026-06-27T23-09-57-372Z`.
- S12 passed after route-cleanup and selector repairs on 2026-06-27:
  `/tmp/agent-browser-p46-s12-2026-06-28T01-05-24-861Z`.

S3 attempt 1 failure shape:

- harness selected default-profile tabs from position-only tab-list rows, so
  an existing retained default-profile tab was mixed into the test target set;
- default-profile route stream evidence omitted redundant `routeId` and
  readiness fields even though the route and display records were ready;
- reset-after returned to zero active incidents.

Agentic repair after attempt 1:

- `scripts/run-p46-stress-scenario.js` now selects S3 tab A and tab B from the
  command-returned service tab handles and scenario URLs instead of blind tab
  positions;
- `scripts/lib/p46-scenario-harness.js` now accepts route/display binding as
  the route identity source when a view stream omits `routeId`, while still
  requiring a completed lease, checked-out route-pool entry, ready display,
  ready route, ready browser, and route/display agreement;
- `node scripts/test-p47-scenario-harness.js` passed after the repair.

S3 attempt 2 failure shape:

- `remote-view open` failed with
  `browser_window_not_visible: route 'guacamole:3' display ':13' state is
  'non_browser_windows'`;
- the scenario then ran against an existing default session with many retained
  tabs and `stealthcdp-default` profile state, which is not the intended clean
  default-profile S3 target;
- service status reported two active incidents during the scenario:
  `remote_view_route_pool_exhausted` and `browser_health_changed`;
- reset-after removed retained sessions and browsers, but left one stale
  `session:default` incident because the remedy could not find the browser.

Cleanup after attempt 2:

- `agent-browser --json service remedies apply --escalation
  os_degraded_possible` returned `Service browser not found: session:default`;
- `agent-browser --json service resolve session:default --by codex --note
  ...` resolved the stale retained incident;
- final status check reported zero retained sessions, zero retained browsers,
  and zero active incidents.

These S3 retry questions were answered by P52:

- S3 can run from the explicit rebuilt-binary lane with the ambient `default`
  profile when reset-before proves zero active incidents and the default
  daemon socket has one matching listener.
- Route-bound open now proved the selected route display reaches
  `browser_window_visible` before full S3 proceeds.
- Reset-before and reset-after are required and were sufficient for the passed
  S3 run.
- `remote_view_route_pool_exhausted` on
  `display:private_virtual_display:session-default` was traced to stale
  private display allocation metadata overriding a fresh route-pool display
  target. P52 fixed route binding so a route-pool entry with
  `target.displayName` defaults to `shared_display` unless the entry explicitly
  requests another isolation mode.

S4 implementation and failure shape:

- `scripts/lib/p46-scenario-harness.js` now defines `s4` as two target-browser
  windows, two viewer clients, and one route lease.
- `scripts/run-p46-stress-scenario.js` supports `--scenario s4` with explicit
  command authority, reset-before and reset-after capture, one route-bound
  remote-view open plus one same-profile window open, per-window dashboard
  proof, per-window navigation and new-tab controls, route-display screenshots,
  closing window A, and verifying window B remains addressable.
- `scripts/test-p47-scenario-harness.js` covers the S4 metadata, capture and
  evaluation wiring, one daemon session against one runtime profile, one
  route-bound open plus one same-profile window open, and the close-A
  verify-B proof.
- Attempt 1 showed a harness defect: window B reused the baseline route-pool
  JSON and selected route A, producing
  `display_allocation_owner_mismatch` on `remote-view-display:13`. The runner
  was repaired to pin window A to `guacamole-rdp-a` and window B to
  `guacamole-rdp-b`.
- Attempt 2 got past the route selection repair but failed during window B
  `remote-view open` with a timeout. The diagnostics showed window A had
  initially reported `operatorVisible.state=ready`, then the window A browser
  process exited and route `guacamole:3` became
  `remote_view_finalization_incomplete` with orphaned display allocation
  `remote-view-display:13`.
- Post-failure cleanup used service incident resolution plus an authenticated
  local service request to release retained route `guacamole:3`. Final cleanup
  evidence after attempt 2 showed zero service sessions, zero browsers, zero
  tabs, zero active incidents, route pool ready, route displays Openbox-only,
  and install doctor green with one authoritative default socket.
- Post-lock diagnosis on 2026-06-27 classified the remaining S4 defect as a
  single-profile topology and typed-blocker gap. Attempt 2 proved window A
  visible on `p46-s4-profile`, route A, and display `:13`, then window B tried
  to create another independent route-bound browser process with the same
  runtime profile on route B. Repo docs and service code already require shared
  authenticated profiles to use the retained browser lane unless a request
  explicitly allows duplicate profile lanes for reviewed isolation. The CLI
  S4 path did not request that reviewed override and did not fail early with a
  typed blocker; it timed out and left route-bound finalization cleanup work.
  P53 must repair that before any live retry.
- P53 Goal 2 selected the supported topology: one retained remote-headed
  browser process, one route lease, one runtime profile, and two top-level
  same-profile windows. The runner now uses `window new --same-profile` for
  window B instead of a second route-bound browser process.

S5 implementation and attempt 1 failure shape:

- `scripts/lib/p46-scenario-harness.js` now defines `s5` as two independent
  target browsers on two runtime profiles, two viewer clients, and two route
  leases.
- `scripts/run-p46-stress-scenario.js` supports `--scenario s5` with explicit
  command authority, reset-before and reset-after capture, one route-bound open
  per profile, dashboard proof for both operators, per-profile navigation and
  new-tab controls, route-display screenshots, closing profile A, and verifying
  profile B remains ready after profile A closes.
- `scripts/test-p47-scenario-harness.js` covers S5 metadata, capture and
  evaluation wiring, distinct route-pool entries for profiles A and B, and the
  profile-B-after-close-A proof.
- Attempt 1 proved both profile browsers became ready concurrently: profile A
  on `guacamole:3` and display `:13`, profile B on `guacamole:4` and display
  `:14`. Both dashboard operators refreshed successfully, both displays showed
  browser windows, both profiles navigated and opened tabs, and profile B
  remained ready after profile A closed.
- Attempt 1 failed because the second `remote-view open` persisted baseline
  request route-pool entries over the first active checkout. Route A's route
  record remained ready, but route-pool entry `guacamole-rdp-a` was overwritten
  to `available` with no current allocation before profile A closed, so
  route-bound finalization was incomplete for profile A.
- `remote_view_open_persist_request_route_pool` now preserves an existing active
  route-pool checkout when an incoming request supplies the same entry as
  inactive baseline data. The focused guard is
  `test_remote_view_open_persist_request_route_pool_preserves_active_checkout`.
- Attempt 2 proved both profile browsers opened ready concurrently on distinct
  routes after the route-pool persistence repair. It failed before dashboard UX
  control proof because operator A's external Chromium viewer could not expose
  DevTools at `127.0.0.1:50102`; stderr reported `bind() failed: Address
  already in use (98)` and `Cannot start http server for devtools`.
- Reset-after closed both S5 profile sessions and returned to zero active
  incidents.
- This is the second consecutive S5 failure, so P46 is locked at S5. A follow-up
  plan must make viewer-client DevTools port allocation collision-resistant and
  add a no-live guard before another live S5 attempt.
- P54 implemented that repair by switching external dashboard viewer clients to
  Chromium dynamic DevTools ports (`--remote-debugging-port=0`) and reading the
  resolved port from `DevToolsActivePort`. P54 no-live validation passed, so one
  S5 retry is allowed.
- S5 passed after the P54 repair. The pass proved profile A on route
  `guacamole:3` and display `:13`, profile B on route `guacamole:4` and display
  `:14`, finalized route-bound checkouts for both profiles, working dashboard
  refresh controls for both external viewer clients, browser-visible route
  displays for both routes, profile B remaining ready after profile A closed,
  and reset-after with zero active incidents.

## UX Confirmation Standard

Each scenario needs visual proof, not just service state proof.

Required visual checks:

- dashboard left rail shows the expected live control rows only;
- the selected workspace opens the expected remote-view route;
- Guacamole or embedded viewport shows the expected browser, page, tab, and
  profile;
- no XTerm or unrelated foreground window obscures the browser;
- visible URL or page title matches the selected target readback;
- stale target IDs in dashboard URLs are rejected or recovered;
- controls perform their action and the UI updates without manual refresh.

Required control checks, when available:

- focus the selected browser;
- navigate current tab to a simple local or public page;
- open a new tab and verify it appears in service state and UI;
- switch between tabs and verify selected target and visible page agree;
- close a tab or release a route and verify state cleanup;
- reconnect or refresh the viewport and verify it returns to the same browser;
- verify read-only or non-owned inventory rows do not expose mutating controls.

Screenshots must be captured for dashboard and direct Guacamole views in every
scenario that claims operator success.

## Resumption Checklist

Resume at S6 after the P55 dashboard swap-navigation repair. Do not rerun S3,
S4, or S5 unless later planning discovers that their evidence is invalid or
stale.

Before S6 implementation or live execution:

1. Re-read this plan, P52, and current repo policy.
2. Confirm live baseline:
   - `./cli/target/debug/agent-browser --json service status`;
   - `./cli/target/debug/agent-browser --json service incidents --summary`;
   - `./cli/target/debug/agent-browser --json install doctor`;
   - `./cli/target/debug/agent-browser --json doctor remote-view`;
   - `node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`;
   - `node scripts/inspect-rdp-route-displays.js --display-content`.
3. S5 runner support exists and S5 passed after P54 repaired viewer-client
   DevTools port allocation. The next implementation slice is S6.
4. Keep the explicit rebuilt-binary lane unless a newer plan intentionally
   changes authority:

```bash
node scripts/run-p46-stress-scenario.js \
  --scenario s6 \
  --reset-before \
  --reset-after \
  --agent-browser-command ./cli/target/debug/agent-browser \
  --require-explicit-agent-browser-command \
  --require-agent-browser-daemon-command-match
```

That command is the intended S6 interface. It must not be run until S6 runner
metadata, capture, evaluation, artifact handling, and no-live harness coverage
exist.

S6 implementation must add or update:

- `scripts/lib/p46-scenario-harness.js` scenario metadata for `s6`;
- `scripts/run-p46-stress-scenario.js` capture and evaluation for `s6`;
- no-live harness coverage in `scripts/test-p47-scenario-harness.js` or the
  nearest scenario-harness test surface;
- artifact capture for command authority, reset-before, reset-after,
  route-pool readiness, display content, dashboard screenshots, Guacamole
  proof, service status, and incident summary;
- failure audit generation and the same two-consecutive-failure lock rule used
  by S2 and S3.

S6 must not claim pass from service JSON alone. It needs visual proof that two
operators can swap dashboard selection between the two live profiles without
controlling the wrong profile or route, and that refresh plus selected-target
readback remains correct after the swap.

## Scenario Matrix

Run scenarios in order. Later scenarios should not start until all earlier
scenarios have passed from a clean reset.

### S0: Baseline Doctor And Reset Readiness

Goal: prove the harness can reset and observe state before stressing it.

Setup:

- no intentional browser open;
- route A and route B available or known clean;
- install doctor and remote-view doctor green.

Actions:

- run reset protocol;
- run route-pool readiness;
- inspect route displays with content;
- open dashboard and verify live rail has no stale control rows;
- fetch public Guacamole route login entry path.

Pass criteria:

- no active incidents;
- route displays are terminal-free;
- dashboard runtime and installed binary converge;
- reset artifacts are complete.

### S1: One UX User, One Profile, One Browser, One Tab

Goal: prove the simple route-bound happy path.

Setup:

- one operator UX session;
- one runtime profile, preferably `last30days-facebook`;
- route A selected by the route pool.

Actions:

- open one URL through `remote-view open`;
- visually confirm in dashboard and direct Guacamole;
- focus browser;
- navigate current tab;
- close or release cleanly.

Pass criteria:

- `operatorVisible.state=ready`;
- route, display, browser, profile, and selected target agree;
- controls work;
- reset returns to clean baseline.

### S2: Two UX Users Viewing The Same Route-Bound Browser

Goal: prove simultaneous viewing does not create route or permission confusion.

Setup:

- two operator UX sessions or browser contexts;
- one route-bound browser;
- same profile and same selected tab.

Actions:

- operator A opens the browser;
- operator B attaches to the same dashboard or Guacamole route;
- operator A navigates;
- operator B visually observes the same page;
- operator B focuses or refreshes the viewport;
- operator A switches tabs if tabs are present.

Pass criteria:

- both operators see the same browser and page;
- no second unintended browser process appears;
- route-pool entry remains single-owner and checked out once;
- controls remain functional and do not desynchronize selected target state;
- reset releases the shared view cleanly.

### S3: Default Profile, Multiple Operators, Different Tabs

Goal: prove one browser can serve multiple operators using different tabs
without stale target or tab ownership confusion.

Runner support:

- implemented by `scripts/run-p46-stress-scenario.js --scenario s3`;
- use `pnpm test:p46-stress-scenario -- --scenario s3 --reset-before
  --reset-after`;
- first live execution after the P48 unlock starts here.

Setup:

- default session or default profile;
- two operator UX sessions;
- one browser window;
- at least two tabs with distinct URLs.

Actions:

- operator A opens tab A;
- operator B opens tab B in the same browser;
- each operator selects a different tab from the UI;
- operator A navigates tab A;
- operator B verifies tab B did not change;
- switch both operators between tabs and verify target readback follows.

Pass criteria:

- service state records both tabs with distinct target IDs;
- dashboard selection is per operator context where intended, or explicitly
  serialized where shared control is the product behavior;
- no stale `about:blank` target is treated as ready;
- controls update the intended tab only;
- reset closes tabs and browser state cleanly.

### S4: One Profile, Multiple Operators, Different Browser Windows

Goal: prove a single profile can serve multiple operators in separate browser
windows without profile lock, route, or display mistakes.

Runner support:

- not implemented as of the P52 S3 clearance;
- next resumption slice should add `--scenario s4` support before live S4
  execution;
- use the explicit rebuilt-binary and daemon-match command shape from the
  resumption checklist after support exists.

Setup:

- one runtime profile;
- two browser windows;
- one or two route-pool entries depending on intended isolation;
- two operator UX sessions.

Actions:

- open window A for operator A;
- open window B for operator B using the same profile;
- navigate each window to a distinct URL;
- verify dashboard inventory shows two controllable windows with clear labels;
- run focus and tab actions in each window;
- close one window and verify the other remains healthy.

Pass criteria:

- no duplicate-process rejection unless it is an explicit typed blocker;
- profile lease behavior is clear and deterministic;
- route and display binding for each window is correct;
- closing one window does not release or corrupt the other window;
- reset cleans both windows and routes.

### S5: Two Profiles Concurrently In Use

Goal: prove profile isolation under concurrent operator-visible browsing.

Setup:

- profile A and profile B;
- two route-bound browsers;
- two operator UX sessions;
- route A and route B preferred when available.

Actions:

- open profile A on route A;
- open profile B on route B;
- navigate both to distinct URLs;
- verify dashboard labels, profiles, route IDs, and display names;
- operate tabs and focus controls on both;
- close profile A browser and verify profile B remains healthy.

Pass criteria:

- no profile crossover;
- no route-display crossover;
- no tab target crossover;
- both Guacamole routes show the intended browser;
- route cleanup for one profile does not disturb the other.

### S6: Two UX Users, Two Profiles, Cross-Observation

Goal: prove operators cannot accidentally control the wrong profile or route
when both profiles are live.

Setup:

- profile A on route A;
- profile B on route B;
- operator A initially controls profile A;
- operator B initially controls profile B.

Actions:

- each operator navigates their assigned browser;
- swap dashboard selection between profile A and profile B;
- verify controls and read-only affordances match current selection;
- refresh both UX sessions;
- verify route URLs and selected targets remain correct.

Pass criteria:

- selection changes do not mutate the wrong browser;
- visual proof and service target proof agree after refresh;
- dashboard row labels are unambiguous;
- no stale row remains actionable.

### S7: Route Pool Exhaustion And Queued Demand

Goal: prove the system fails closed and explainably when demand exceeds route
capacity.

Setup:

- route A and route B occupied by healthy browsers;
- request a third operator-visible route-bound browser.

Actions:

- attempt third open;
- inspect typed blocker;
- verify dashboard does not create a fake live row;
- release one route and retry once.

Pass criteria:

- third request returns a clear route-capacity blocker;
- no terminal-only fallback;
- no direct remote-headed fallback;
- retry after release succeeds or returns a new explicit blocker;
- incidents and retained rows remain clean.

### S8: Permission And Display Recovery

Goal: prove display access failures are diagnosed and repaired through the
supported path.

Setup:

- clean route displays;
- use a controlled test variant that withholds or invalidates display access
  only if a safe fixture or script exists.

Actions:

- trigger or simulate display access denial;
- run the open path;
- verify typed blocker or automatic grant path;
- run doctor and route display inspection.

Pass criteria:

- no silent permission failure;
- no terminal fallback;
- no stale route row;
- remediation command is explicit;
- after repair, the same scenario passes from reset.

### S9: Stale Target And Duplicate Tab Stress

Goal: prove selected-target recovery under duplicate same-origin and blank-tab
conditions.

Setup:

- one browser;
- multiple tabs, including `about:blank` and duplicate same-origin target URLs.

Actions:

- request a URL already represented by stale or blank metadata;
- force tab switching through the UX;
- verify selected target readback and visible window proof;
- repeat with duplicate same-origin tabs.

Pass criteria:

- selected target is live and navigated;
- stale metadata cannot satisfy readiness;
- duplicate cleanup behavior is deterministic;
- dashboard selected tab and CDP selected target agree.

S9 lock status on 2026-06-27:

- First live attempt artifact:
  `/tmp/agent-browser-p46-s9-2026-06-27T21-32-27-404Z`.
- Corrected live attempt artifact:
  `/tmp/agent-browser-p46-s9-2026-06-27T21-42-54-990Z`.
- The corrected run proved the initial stale blank-tab recovery notice and
  proved CLI navigation of the blank target to
  `https://www.iana.org/domains/reserved?p46=s9-blank-recovered`.
- The run failed before S9 pass because the dashboard rewrote operator C from
  the requested blank target
  `target:92B1ABA4B645E77E3C72BE117CD14832` to recovered duplicate target
  `target:1650B99DC1120C753CE97BFE43050090` when the harness re-requested the
  blank target after navigation.
- Reset-after reported zero sessions, browsers, tabs, and active incidents.

P46 is locked at S9. Do not run another S9 retry until P62 repairs or
redefines the dashboard selected-target recovery contract and records a
validation-backed retry authorization.

P62 clearance on 2026-06-27:

- Follow-up plan:
  `docs/dev/plans/0062-2026-06-27-s9-stale-target-recovery-plan.md`.
- Dashboard repair: explicitly selected live blank tabs are now preserved as
  exact selected targets instead of being immediately URL-rewritten to a
  different non-blank tab. Missing or dead stale targets still recover to a
  live tab.
- Runtime publish: `pnpm publish:local-dashboard -- --skip-smoke --json`
  rebuilt `packages/dashboard/out`, rebuilt the CLI, atomically installed
  `/home/ecochran76/.local/bin/agent-browser`, synced reference binaries, and
  restarted `agent-browser-dashboard.service`.
- Installed runtime proof: `agent-browser --json install doctor` reported
  `success=true`, zero issues, and live dashboard runtime ready with executable
  SHA `81c89d6ce16c10a20d019d673d72a755f0145d63433e6a0c96e40a2da5373b4b`.
- S9 pass artifact:
  `/tmp/agent-browser-p46-s9-2026-06-27T22-03-14-950Z`.
- The pass proved three distinct tab IDs, exact initial blank-target selection,
  blank navigation to
  `https://www.iana.org/domains/reserved?p46=s9-blank-recovered`, duplicate tab
  A/B independent navigation, browser-window-visible route display, route-bound
  finalization, one default-profile browser row, and zero active incidents
  after reset-after.

P46 may continue at S10.

S10 lock on 2026-06-27:

- Follow-up plan:
  `docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md`.
- First live attempt artifact:
  `/tmp/agent-browser-p46-s10-2026-06-27T22-17-57-154Z`.
- Second live attempt artifact:
  `/tmp/agent-browser-p46-s10-2026-06-27T22-20-21-552Z`.
- Both attempts opened the service-owned route-bound browser and reset
  cleanly with zero active incidents after reset-after.
- Attempt one failed before S10 evaluation because the runner queried
  `/sessions` and received dashboard HTML instead of JSON.
- Attempt two failed before S10 evaluation because `/api/sessions` requires
  dashboard authentication.
- The harness has been repaired to read `/api/sessions` and
  `/api/session-tabs?port=...` through the authenticated dashboard
  viewer-client session, and the no-live harness checks pass.

P63 clearance on 2026-06-27:

- Follow-up plan:
  `docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md`.
- Product repair: dashboard `/api/session-tabs?port=<foreign-cdp-port>` now
  falls back from agent-browser `/api/tabs` to raw Chrome CDP `/json/list` and
  reads local proxy responses to declared `Content-Length`.
- Harness repair: S10 reads dashboard inventory through the authenticated
  viewer-client session, treats selected viewport-route context as valid when
  the optional detail panel is not mounted, and scopes route-borrow detection
  to selected-workspace evidence instead of global workspace-list text.
- Installed runtime proof: `agent-browser --json install doctor` reported
  `success=true`, zero issues, and executable SHA
  `502f05830dfb756cda44eae7d6bb8c71999dd4ce39ee109eb51ff36136de155a`.
- S10 pass artifact:
  `/tmp/agent-browser-p46-s10-2026-06-27T22-52-43-936Z`.
- The pass proved authenticated foreign CDP inventory, normalized foreign tab
  inventory, no service route/display borrowing, stable foreign and
  service-owned selected workspace context, complete service-owned route-bound
  finalization, and zero active incidents after reset-after.

P46 may continue at S11.

### S10: Foreign CDP Inventory Beside Service-Owned RDP Browsers

Goal: prove non-owned browsers are addressable but cannot be mistaken for live
service-owned route-bound control rows.

Setup:

- one service-owned route-bound browser;
- one reachable foreign CDP browser if available in the environment.

Actions:

- run dashboard inventory;
- select the foreign CDP row;
- verify available actions are read-only or explicitly adopt-only;
- select the service-owned row and verify full controls;
- run viewport refresh for both rows.

Pass criteria:

- foreign CDP row is clearly non-owned;
- service-owned row remains fully controllable;
- no route or display state is borrowed from the foreign row;
- live rail does not promote diagnostics to control targets.

### S11: Dashboard Refresh, Stale URL, And Reconnect Stress

Goal: prove the UX survives reloads and stale workspace URLs.

Setup:

- one or two active route-bound browsers;
- saved dashboard URLs with and without tab parameters.

Actions:

- reload dashboard;
- open stale tab-param URLs;
- reconnect embedded viewport;
- open direct Guacamole URLs;
- compare visual state before and after refresh.

Pass criteria:

- stale target IDs are rejected or recovered;
- dashboard selection returns to a valid route-bound browser;
- direct Guacamole still shows the same browser;
- controls remain functional after refresh.

S11 first attempt on 2026-06-27:

- First live attempt artifact:
  `/tmp/agent-browser-p46-s11-2026-06-27T23-02-14-303Z`.
- The run opened the service-owned route-bound browser, launched the
  authenticated dashboard viewer, proved dashboard reload, and pushed the stale
  dashboard URL.
- The dashboard immediately recovered the URL from stale target
  `target:p46-s11-stale-target` back to the live target
  `target:9E6E67F299F964040CC151188712AF9C`.
- The harness failed because it waited for the exact stale URL to persist in
  DevTools page discovery. That contradicts the S11 pass criterion, which
  allows stale target IDs to be rejected or recovered.
- Reset-after closed `default` and reported zero active incidents.

Harness repair:

- S11 now records stale URL readback after recovery and waits for the dashboard
  stale-target recovery state instead of requiring the stale URL to persist.

One S11 retry is authorized after the no-live checks pass and incidents remain
zero immediately before the run.

S11 lock on 2026-06-27:

- Follow-up plan:
  `docs/dev/plans/0064-2026-06-27-s11-stale-url-live-target-recovery-plan.md`.
- Second live attempt artifact:
  `/tmp/agent-browser-p46-s11-2026-06-27T23-05-10-207Z`.
- The dashboard again rejected the stale target and recovered to a live target
  for the same browser/session with a healthy iframe, but without the explicit
  stale-recovery notice text that the harness expected.
- Reset-after closed `default` and reported zero active incidents.
- The harness has been repaired to accept live-target URL rewrite as an S11
  stale URL recovery form through a scoped `allowRecoveredLiveTab` option.

P64 clearance on 2026-06-27:

- Follow-up plan:
  `docs/dev/plans/0064-2026-06-27-s11-stale-url-live-target-recovery-plan.md`.
- Harness repair: S11 accepts stale URL recovery either through explicit
  stale-recovery notice text or through immediate live-target URL rewrite for
  the same browser/session with a healthy iframe.
- S11 pass artifact:
  `/tmp/agent-browser-p46-s11-2026-06-27T23-09-57-372Z`.
- The pass proved dashboard reload restoration, stale URL recovery to live
  target `target:2AE5C8E4CED37D0C1A77771CD3CA9F58`, viewer-client reconnect,
  viewport refresh, direct Guacamole HTTP 200, route display
  `browser_window_visible`, route-bound finalization, and zero active incidents
  after reset-after.
- Command metadata caveat: the pass used an explicit installed binary command
  and daemon realpath matching passed, but the explicit-command guard flag was
  misspelled, so the artifact reports `requireExplicit: false` while still
  reporting `explicit: true`.

P46 may continue at S12.

### S12: Long-Running Soak With Periodic Resets

Goal: prove state does not drift after repeated normal use.

Setup:

- choose the highest-complexity scenario that has passed, usually S5 or S6.

Actions:

- repeat open, navigate, tab switch, viewport refresh, close, and reset for at
  least 10 cycles;
- capture doctor and incident summary at cycle boundaries;
- include at least one dashboard reload and one Guacamole reconnect per cycle.

Pass criteria:

- zero active incidents at the end of every cycle;
- no retained stale control rows;
- route-pool state returns to baseline after each reset;
- memory, process, and profile lease pressure do not trend upward without an
  explicit retained owner.

S12 implementation and lock on 2026-06-27:

- Harness support was added for ten normal-use cycles. Each cycle opens a
  route-bound browser, reloads the dashboard, reconnects the viewer-client,
  clicks viewport refresh, navigates, creates and switches to a tab, verifies
  direct Guacamole HTTP, captures route-bound finalization, closes, resets, and
  records before and after doctor, incident, service-status, and route-pool
  boundary evidence.
- No-live validation passed:
  `node --check scripts/run-p46-stress-scenario.js`,
  `node --check scripts/lib/p46-scenario-harness.js`,
  `node --check scripts/test-p47-scenario-harness.js`,
  `node scripts/test-p47-scenario-harness.js`, and
  `node scripts/test-p47-viewer-client-separation.js`.
- First S12 attempt artifact:
  `/tmp/agent-browser-p46-s12-2026-06-27T23-20-14-868Z`.
- The first attempt completed ten cycles with zero active incidents, zero
  sessions, zero browsers, zero tabs, route-pool baseline true, and direct
  Guacamole HTTP 200 in every cycle. It failed because the evaluator counted
  completed acquisition-lease history as active pressure. The evaluator was
  repaired to exclude completed or checked-out historical leases from active
  pressure.
- Second S12 attempt artifact:
  `/tmp/agent-browser-p46-s12-2026-06-27T23-39-14-415Z`.
- The second attempt exposed real reset drift. Cycle 3 left route
  `guacamole:3` orphaned on display allocation `remote-view-display:13`, and
  route-pool entry `guacamole-rdp-a` remained checked out. Cycle 4 then failed
  route-bound open with `route_pool_entry_unavailable`.
- Cleanup was performed through the authenticated service request action
  `service_route_pool_repair`, not by manual state editing. Dry-run identified
  one stale checkout (`guacamole-rdp-a`), one stale route (`guacamole:3`), and
  one stale display allocation (`remote-view-display:13`). Apply repaired all
  three. Reconcile then showed both route-pool entries available, route
  `guacamole:3` released, and `remote-view-display:13` released.
- The remaining transient browser recovery incident for
  `session:s12-cycle-03-2026-06-27T23-43-15-712Z` was resolved with an
  explicit note after the route-pool repair. `service incidents --summary`
  reports no active incidents, only the recovered incident record.

Historical lock, superseded by the 2026-06-27 P46 S12 clearance below: P46 was
locked at S12 until the reset and route-pool release path for orphaned
route-bound displays after normal close was repaired and the selector-repaired
S12 retry passed.

## Harness Deliverables

The campaign should produce or extend scripts instead of relying on manual
one-offs:

- a scenario runner that accepts `--scenario <id>`, `--reset-before`, and
  `--artifact-dir`;
- a reset helper that captures pre-reset and post-reset state;
- a dashboard UX check using browser automation screenshots and control clicks;
- a Guacamole visual check for the direct route;
- a scenario result JSON schema with pass, fail, locked, and skipped states;
- a failure audit template written into each artifact directory;
- a summary report under `docs/dev/notes/` after the campaign.

All CLI flags in new scripts must use kebab-case.

## 2026-06-26 P49 S3 Lock Note

P49 attempted to remediate and retry S3, "Default Profile, Multiple Operators,
Different Tabs." The plan remains locked at S3.

Artifacts:

- `/tmp/agent-browser-p46-s3-2026-06-26T21-08-54-578Z`
- `/tmp/agent-browser-p46-s3-2026-06-26T21-11-13-480Z`
- `docs/dev/plans/0049-2026-06-26-p46-s3-remediation-plan.md`

Outcome:

- The S3 harness now fails closed after a failed `remote-view open` and no
  longer launches dashboard viewers or tab controls after route-bound open
  failure.
- The final S3 retry failed at `remote-view open` with
  `browser_window_not_visible` on route `guacamole:3`, display `:13`, state
  `non_browser_windows`.
- Failed-open display evidence showed only Openbox on route displays, while
  route-pool readiness remained green.
- Runtime cleanup after lock was verified: zero sessions, zero browsers, zero
  tabs, zero active incidents, and both route-pool entries available.

Caveat:

- The live retry used the installed `agent-browser` command, so the newly
  compiled repo change for bounded visible-window proof was not exercised by
  the live binary. The next planning pass must decide whether to validate with
  the freshly built binary first, repair browser-window realization on the RDP
  display first, or combine those steps.

Superseded by P52 on 2026-06-27. The historical P49 lock no longer blocks S4
planning.

## 2026-06-26 P50 Lock Note

P50 added explicit command metadata and a narrow `s3-open` proof before full
S3. The narrow proof used `./cli/target/debug/agent-browser` explicitly, but
then exposed a deeper authority problem: the default agent-browser socket had
multiple daemon listeners, including an installed binary and older repo debug
binary processes.

Artifact:

- `/tmp/agent-browser-p46-s3-open-2026-06-26T23-40-14-493Z`
- `docs/dev/plans/0050-2026-06-26-s3-binary-authority-visible-window-plan.md`

Outcome:

- Superseded by P52 on 2026-06-27. Full S3 is no longer locked.
- P52 made `s3-open` pass after proving the daemon listener for the default
  socket was singular and matched the intended repo binary.
- Runtime cleanup after P50 was verified: zero sessions, zero browsers, zero
  tabs, zero active incidents, and both route-pool entries available.

## 2026-06-27 P52 S3 Clearance Note

P52 repaired the daemon-authority and route-binding defects that kept S3
locked.

Artifacts:

- Plan: `docs/dev/plans/0052-2026-06-27-s3-clearance-pathway-plan.md`
- Passing `s3-open`:
  `/tmp/agent-browser-p46-s3-open-2026-06-27T16-36-31-141Z`
- Passing full S3:
  `/tmp/agent-browser-p46-s3-2026-06-27T16-37-24-659Z`
- Final service status:
  `/tmp/agent-browser-p52-final-service-status.json`
- Final install doctor:
  `/tmp/agent-browser-p52-final-install-doctor.json`

Outcome:

- S3 is clear.
- The narrow route-bound open proof passed with:
  - one matching `default.sock` listener for
    `./cli/target/debug/agent-browser`;
  - route `guacamole:3`;
  - display `:13`;
  - `browser_window_visible`;
  - finalized route checkout with no blockers.
- Full S3 passed with:
  - default profile browser `session:default`;
  - distinct tab IDs;
  - operator A and operator B dashboard viewport evidence;
  - dashboard refresh controls clicked for both operators;
  - independent tab navigation evidence;
  - reset-after with zero active incidents.
- Final runtime state after P52 showed zero browsers, zero sessions, zero tabs,
  and zero active incidents.

Residual risk:

- Final install doctor still reports non-S3 install/runtime drift:
  `current_executable_path_command_mismatch`,
  `dashboard_runtime_stale_or_unreadable`, and
  `active_runtime_stale_executable`.
- The default daemon authority needed for S3 is clean:
  `defaultSocketListenerCount: 1`,
  `defaultSocketCurrentExecutableMatchCount: 1`, and
  `defaultSocketDeletedExecutableCount: 0`.

Next step:

- Continue to S4 from the explicit rebuilt-binary lane. Do not treat the
  remaining install-doctor drift as cleared by S3; handle it in its own
  runtime convergence slice if it blocks later scenarios.

## 2026-06-27 P55 Retry And P56 Lock Note

The P55-authorized S6 retry failed and did not clear S6.

Artifacts:

- Failed retry:
  `/tmp/agent-browser-p46-s6-2026-06-27T20-06-51-508Z`
- Follow-up plan:
  `docs/dev/plans/0056-2026-06-27-s6-dashboard-reconnect-and-reset-buffer-plan.md`

Outcome:

- P55's in-page dashboard navigation helper wrote
  `operator-a-swapped-to-profile-b-navigate.json` with `ok: true`.
- The run then timed out during the first post-swap dashboard-state poll:
  `CDP command Runtime.evaluate timed out after 30000ms`.
- The failed run showed a second reset weakness: `service status` can exceed
  Node's default `spawnSync` output buffer, which made reset-after parse status
  as `null` and close no retained browser rows.
- Manual cleanup closed both retained S6 profile sessions and final readback
  showed zero sessions, zero browsers, zero tabs, zero active incidents, and
  both route-pool entries available.
- P56 added a viewer-client CDP reconnect helper after swapped dashboard
  navigation and a 32 MiB command output buffer for reset evidence.

Next step:

- Do not run S6 again until P56 validation passes. The next allowed S6 retry
  must use the P56 reconnect and reset-buffer repair path.

## 2026-06-27 P56 Retry And P57 Lock Note

The P56-authorized S6 retry failed and S6 remains locked.

Artifacts:

- Failed retry:
  `/tmp/agent-browser-p46-s6-2026-06-27T20-15-52-909Z`
- Follow-up plan:
  `docs/dev/plans/0057-2026-06-27-s6-viewer-client-cdp-swap-diagnostics-plan.md`

Outcome:

- The in-page dashboard URL change still succeeded and wrote
  `operator-a-swapped-to-profile-b-navigate.json`.
- The reconnect path timed out before writing
  `operator-a-swapped-to-profile-b-reconnect.json`:
  `CDP command Page.enable timed out after 30000ms`.
- The P56 reset-buffer repair worked. Reset-after closed both retained S6
  profile sessions and final readback showed zero sessions, zero browsers, zero
  tabs, zero active incidents, and both route-pool entries available.

Next step:

- Do not run S6 again until P57 captures reconnect target-discovery evidence
  and records a concrete repair.

## 2026-06-27 P57 Retry And P58 Lock Note

The P57-authorized S6 retry failed and S6 remains locked.

Artifacts:

- Failed retry:
  `/tmp/agent-browser-p46-s6-2026-06-27T20-23-53-164Z`
- Follow-up plan:
  `docs/dev/plans/0058-2026-06-27-s6-dashboard-page-url-stabilization-plan.md`

Outcome:

- The P57 reconnect discovery artifact was written.
- Discovery showed the selected DevTools page still had the pre-swap profile A
  dashboard URL after the in-page navigation helper requested profile B.
- The P57 reconnect command-sequence repair executed and skipped domain
  enablement, but the subsequent dashboard-state `Runtime.evaluate` timed out.
- Reset-after closed both retained sessions and final readback showed zero
  sessions, zero browsers, zero tabs, zero active incidents, and both
  route-pool entries available.

Next step:

- P58 waits for the DevTools page URL to match the requested swapped dashboard
  URL before reconnecting and reading dashboard state.

## 2026-06-27 P58 Retry And P59 Lock Note

The P58-authorized S6 retry failed and S6 remains locked.

Artifacts:

- Failed retry:
  `/tmp/agent-browser-p46-s6-2026-06-27T20-28-52-103Z`
- Follow-up plan:
  `docs/dev/plans/0059-2026-06-27-s6-dashboard-history-swap-plan.md`

Outcome:

- The page-URL wait artifact was written and showed that `location.assign()`
  did not change the DevTools page URL for the same-origin dashboard workspace
  swap.
- The run failed before reconnect because the page stayed on profile A while
  the harness waited for profile B.
- Reset-after closed both retained sessions and final readback showed zero
  sessions, zero browsers, zero tabs, zero active incidents, and both
  route-pool entries available.

Next step:

- P59 changes same-origin dashboard swaps to `history.pushState` plus
  `popstate`, retaining `location.assign` only as a cross-origin fallback.

## 2026-06-27 P59 S6 Clearance Note

P59 cleared S6.

Artifact:

- `/tmp/agent-browser-p46-s6-2026-06-27T20-32-54-709Z`

Outcome:

- S6 passed from the explicit rebuilt-binary lane.
- Operator A initially targeted profile A, swapped to profile B, refreshed the
  swapped dashboard viewport, and captured a swapped dashboard screenshot.
- Operator B initially targeted profile B, swapped to profile A, refreshed the
  swapped dashboard viewport, and captured a swapped dashboard screenshot.
- Swapped selected-browser readback matched:
  - operator A: `session:p46-s6-profile-b-2026-06-27T20-32-55-285Z`;
  - operator B: `session:p46-s6-profile-a-2026-06-27T20-32-55-285Z`.
- Both route-bound profile browsers finalized on distinct routes:
  `guacamole:3` on display `:13` and `guacamole:4` on display `:14`.
- Closing profile A released route A while profile B stayed ready.
- Reset-after closed the remaining retained profile B session, reported zero
  active incidents, and final runtime readback showed zero sessions, zero
  browsers, zero tabs, zero active incidents, and both route-pool entries
  available.

Next step:

- Continue P46 at S7.

## 2026-06-27 P60 S7 Clearance Note

P60 cleared S7.

Artifact:

- `/tmp/agent-browser-p46-s7-2026-06-27T20-58-30-721Z`

Outcome:

- S7 passed from the explicit rebuilt-binary lane after rebuilding
  `./cli/target/debug/agent-browser` and restarting the stale default daemon.
- Profile A and profile B occupied both route-pool entries, with
  `guacamole-rdp-a` checked out to `guacamole:3` and `guacamole-rdp-b`
  checked out to `guacamole:4`.
- The third route-bound request failed closed with
  `route_pool_exhausted: no available route-pool entries remain`.
- The failed third demand did not create a retained profile C browser row.
- Route A and route B remained browser-window-visible after the failed third
  demand, and no terminal fallback was visible.
- Closing profile A released capacity, and profile C retry succeeded on the
  released route.
- Reset-after closed the remaining retained profile B and profile C sessions,
  reported zero active incidents, and left the runtime clean for S8.

Next step:

- Continue P46 at S8.

## 2026-06-27 P61 S8 Clearance Note

P61 cleared S8.

Artifact:

- `/tmp/agent-browser-p46-s8-2026-06-27T21-07-22-844Z`

Outcome:

- S8 passed from the explicit rebuilt-binary lane.
- A temporary `timeout` shim safely simulated display-access denial without
  mutating host X11 permissions.
- The denied open failed before browser launch with
  `display_access_grant_failed`.
- Cleanup evidence showed `skipped_before_browser_launch`,
  `restoredDisplayAllocation: true`, `restoredRemoteViewRoute: true`, and
  `restoredRoutePoolEntry: true`.
- The denied demand created no retained denied-profile browser row.
- Route A and route B remained terminal-free after denial.
- The same route-bound open succeeded after restoring normal `PATH`, with
  `displayAccessGrant.state: already_ready` and route A browser-window-visible.
- Reset-after closed the repair session, reported zero active incidents, and
  left route A and route B available.

Next step:

- Continue P46 at S9.

## 2026-06-27 P46 S12 Repair Continuation

The route-pool cleanup lock has been addressed.

Repair:

- `merge_reconciled_service_state` preserves newer remote-view release
  mutations for display allocations, remote-view routes, and route-pool
  entries.
- Close cleanup now falls back to session-owned display allocations and routes
  when process-exit reconcile removed the browser row before close persistence.
- Regression coverage includes the process-exit race with
  `remote-view-display:13`, `guacamole:3`, and `guacamole-rdp-a`.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml close_releases_session_owned_route_after_process_exit_removed_browser -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml native::service_health::tests:: -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --release --manifest-path cli/Cargo.toml`
- `pnpm converge:local-runtime -- --apply --json`

Installed command SHA:

- `43d85bebf6c2e68fb7b86a5e9a1628f6e20698d7140533bb033bf932dd26c113`

Third S12 attempt:

- artifact:
  `/tmp/agent-browser-p46-s12-2026-06-28T00-43-57-985Z`
- all ten cycles completed;
- route-pool baseline was true after every reset;
- active incidents stayed zero at every boundary;
- pressure did not increase after resets;
- direct Guacamole returned HTTP 200 in every cycle.

The remaining failure was a harness selector defect: S12 switched by stale
tab-list position when tab-list rows lacked target IDs, so some repeated cycles
selected an older IANA tab instead of the current cycle's new `example.org`
tab. The harness now prefers the `tab new` command result by service tab ID or
by exact returned index and URL before falling back to a positional selector.

No-live validation passed:

- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`

S12 is unlocked for one selector-repaired retry. If that retry fails with a new
runtime or harness class, classify it from the artifact before another run.

## 2026-06-27 P46 S12 Clearance

Selector-repaired S12 retry:

- artifact:
  `/tmp/agent-browser-p46-s12-2026-06-28T01-05-24-861Z`
- result: passed;
- command guard: `requireExplicit: true`, `explicit: true`, and daemon realpath
  matching passed for `/home/ecochran76/.local/bin/agent-browser`;
- all ten cycles completed;
- route-pool baseline was true after every reset;
- active incidents stayed zero before close, after reset, and after boundary
  checks in every cycle;
- post-reset pressure did not increase;
- checked-out route-pool, active remote-view routes, sessions, browsers, and
  tabs were zero after every reset;
- direct Guacamole returned HTTP 200 in every cycle;
- reset-before and reset-after both ended with zero active incidents.

Final closeout evidence:

- final install doctor:
  `/tmp/agent-browser-p46-s12-final-install-doctor.json`
- final remote-view doctor:
  `/tmp/agent-browser-p46-s12-final-remote-view-doctor.json`
- final incident summary:
  `/tmp/agent-browser-p46-s12-final-incidents-summary.json`
- installed command SHA:
  `43d85bebf6c2e68fb7b86a5e9a1628f6e20698d7140533bb033bf932dd26c113`
- install doctor success with no issues and runtime status `converged`;
- remote-view doctor status `ready`, next action `run_many_to_many_live_gate`;
- incident summary count 0.

P46 S12 is cleared. P46 scenario execution is complete through S12; remaining
closeout should summarize residual risk and decide the next hardening target.

## Campaign Summary

P46 is complete through S12. The campaign exercised the full scenario matrix in
order after each lock was repaired and authorized: baseline readiness, single
route-bound use, two viewers on one browser, multi-tab default-profile control,
same-profile window topology, concurrent profile isolation, cross-profile
dashboard selection, route-pool exhaustion, display-access recovery, stale
target recovery, foreign CDP inventory beside service-owned rows, dashboard
reload and stale URL recovery, and the ten-cycle S12 reset soak.

Final closeout artifacts:

- S12 pass:
  `/tmp/agent-browser-p46-s12-2026-06-28T01-05-24-861Z`
- final service status:
  `/tmp/agent-browser-p46-final-service-status.json`
- final install doctor:
  `/tmp/agent-browser-p46-final-install-doctor.json`
- final remote-view doctor:
  `/tmp/agent-browser-p46-final-remote-view-doctor.json`
- final incident summary:
  `/tmp/agent-browser-p46-final-incidents-summary.json`
- final route-pool readiness:
  `/tmp/agent-browser-p46-final-route-pool-readiness.json`

Final state:

- final install doctor succeeded with no issues and runtime status
  `converged`;
- final remote-view doctor reported `ready`;
- final incident summary count was 0;
- final route-pool readiness succeeded;
- final service status reported zero service browsers, zero service sessions,
  zero tabs, and zero active incidents;
- route-pool entries `guacamole-rdp-a` and `guacamole-rdp-b` were available
  with no current route allocation;
- active remote-view routes were released.

Residual risks:

- Historical orphaned display-allocation records remain in service status even
  though they are not live control rows and do not hold route-pool capacity.
  They should be handled by a separate retained-state compaction or pruning
  plan instead of being mixed into P46 pass criteria.
- S12 intentionally repeats a one route-bound browser normal-use reset
  contract. Multi-browser cross-observation remains covered by S6 rather than
  repeated for every S12 cycle.
- Graphiti memory writes for this closeout timed out during extraction and node
  resolution. The repo plan, runbook, execution note, and `/tmp` artifacts are
  the authoritative durable record.

Next hardening target:

- Start a dedicated retained-state compaction and doctor-surface plan for
  historical orphaned display allocations and stale metadata visibility. The
  plan should keep P46's route-pool and live-control guarantees intact while
  making final service-status output easier to audit.

## Validation Gates

Before running live stress scenarios:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- relevant focused Rust tests for touched remote-view, service, and install
  surfaces;
- dashboard tests for workspace navigation, viewport, inspector actions, and
  selected context;
- generated client tests if service contracts or generated helpers change;
- `pnpm validation:select -- --base <ref>` for the touched slice.

During live scenario execution:

- install doctor and remote-view doctor before S0 and after any repair;
- route-pool readiness before every scenario;
- display-content inspection before and after every scenario;
- service status and incident summary before reset and after reset;
- dashboard screenshot and Guacamole screenshot for every operator-visible
  success claim.

## Closeout Criteria

P46 is complete only when:

- all scenarios S0 through S12 pass in order;
- runtime state is reset between every scenario;
- no scenario required manual state edits;
- no scenario produced two consecutive failures without locking for planning;
- every failure had an artifact-backed audit before retry;
- dashboard and Guacamole visual checks prove the expected browser for every
  operator-visible scenario;
- controls were exercised and verified, not merely rendered;
- final install doctor and remote-view doctor are green;
- final service status has zero active incidents and no stale retained live
  control rows;
- the campaign summary names residual risks and the next hardening target.
