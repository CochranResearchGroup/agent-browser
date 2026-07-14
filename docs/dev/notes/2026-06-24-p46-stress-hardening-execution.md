# P46 Stress Hardening Execution Notes

Date: 2026-06-24
Plan: `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
State: complete

## Discovery

Graphiti runtime was healthy. A focused `agent_browser_main` discovery query
returned general service-control-plane and repo-policy context, but no
P46-specific sourced episode. Current repo files, the P46 plan, and live
runtime artifacts are authoritative.

CodeGraph tools were not exposed in this session, so direct source reads and
focused command evidence were used for the first runner slice.

## Harness Added

Added `scripts/run-p46-stress-scenario.js` and package script
`test:p46-stress-scenario`.

Implemented scenarios:

- `s0`: baseline doctor, route-pool, display, service, incident, dashboard HTTP,
  Guacamole HTTP, dry-run cleanup, and route-display screenshot capture.
- `s1`: one operator route-bound open on one profile, URL readback, current-tab
  navigation, new-tab control, tab-list readback, display-content proof,
  route-display screenshot, incident readback, and reset-after cleanup.
- `s2`: two dashboard operator contexts observing one route-bound browser,
  viewport refresh control, controlled-browser navigation, display-content
  proof, screenshot capture, incident readback, and reset cleanup.

The runner writes an artifact directory under `/tmp/agent-browser-p46-<scenario>-<timestamp>/`,
including `summary.json` and `FAILURE_AUDIT.md` on failure. Reset is explicit
through `--reset-before` and `--reset-after`.

## S0 Result

Command:

```bash
pnpm test:p46-stress-scenario -- --scenario s0 --reset-before
```

Artifact directory:

```text
/tmp/agent-browser-p46-s0-2026-06-24T18-42-30-422Z
```

Result: passed.

Evidence:

- reset closed `default`;
- zero active incidents before and after reset;
- install doctor and remote-view doctor were green;
- route A `guacamole:3` on display `:13` ready;
- route B `guacamole:4` on display `:14` ready;
- route displays were `non_browser_windows` with only Openbox and no terminal;
- dashboard root returned HTTP 200;
- public Guacamole route returned HTTP 200 through the login entry path;
- two route-display screenshots were captured.

Known S0 warning:

- authenticated dashboard live-rail visual proof is not implemented in the S0
  runner yet.

## S1 First Attempt

Command:

```bash
pnpm test:p46-stress-scenario -- --scenario s1 --reset-before --reset-after
```

Artifact directory:

```text
/tmp/agent-browser-p46-s1-2026-06-24T18-44-18-291Z
```

Result: failed due to runner bug, not runtime behavior.

The `get url` CLI returns object-shaped data such as
`{"url":"https://example.com/"}`. The first S1 evaluator compared the object
directly, producing false failures:

- `URL after open was [object Object]`
- `URL after navigate was [object Object]`

The artifact-backed evidence showed the runtime had opened Example Domain,
navigated to IANA, created tabs, displayed Chromium on route A, and reset with
zero active incidents. The evaluator was fixed to normalize string,
`data.url`, and `data.value` URL shapes before retry.

## S1 Retry

Command:

```bash
pnpm test:p46-stress-scenario -- --scenario s1 --reset-before --reset-after
```

Artifact directory:

```text
/tmp/agent-browser-p46-s1-2026-06-24T18-46-11-031Z
```

Result: passed.

Evidence:

- route A selected as `guacamole:3`;
- display `:13`;
- route display state `browser_window_visible`;
- visible windows included `Example Domain - Chromium`;
- current-tab navigation reached
  `https://www.iana.org/domains/reserved`;
- new-tab control succeeded;
- tab list contained three tabs;
- route-display screenshot captured;
- reset-after closed `default`;
- zero active incidents after reset.

Known S1 warning:

- S1 exercises browser controls through the route-bound session. Authenticated
  dashboard button-click UX remains for a later runner slice.

## Next Recommended Slice

P46 is locked before further live scenario execution.

## S2 First Attempt

Command:

```bash
pnpm test:p46-stress-scenario -- --scenario s2 --reset-before --reset-after
```

Artifact directory:

```text
/tmp/agent-browser-p46-s2-2026-06-24T18-51-22-187Z
```

Result: failed.

Useful proof from the failed attempt:

- route-bound open worked on route A, `guacamole:3`, display `:13`;
- operator A and operator B both reached dashboard workspace state with a
  Guacamole iframe;
- the dashboard refresh control was clicked successfully;
- controlled-browser navigation reached
  `https://www.iana.org/domains/reserved`;
- display-content proof showed `IANA-managed Reserved Domains - Chromium`.

Failure:

- the harness used `agent-browser` sessions as dashboard operator browsers;
- those service-owned operator sessions tried to acquire remote-view route
  resources themselves;
- service status reported active incidents for
  `session:p46-s2-operator-a` and `session:p46-s2-operator-b`.

Incident cleanup used service actions, not manual state edits:

```text
/tmp/agent-browser-p46-resolve-s2-operator-a.json
/tmp/agent-browser-p46-resolve-s2-operator-b.json
/tmp/agent-browser-p46-incidents-after-s2-resolve.json
```

The incident summary after cleanup showed no active incidents.

Classification: test harness defect with product-relevant coverage. The
operator UX proof must not consume or fault service-managed remote-view browser
slots.

## S2 Retry

Command:

```bash
pnpm test:p46-stress-scenario -- --scenario s2 --reset-before --reset-after
```

Artifact directory:

```text
/tmp/agent-browser-p46-s2-2026-06-24T18-56-05-128Z
```

Result: failed.

Failure:

```text
Error: operator-a external Chromium was not ready: fetch failed
```

The retry moved the dashboard operator contexts out of service-owned
`agent-browser` sessions and into external Chromium CDP contexts. Operator A's
external Chromium did not become ready, so S2 could not complete the two-user
dashboard UX confirmation. The failure audit is recorded in:

```text
/tmp/agent-browser-p46-s2-2026-06-24T18-56-05-128Z/FAILURE_AUDIT.md
```

Post-failure audit captured:

```text
/tmp/agent-browser-p46-s2-2026-06-24T18-56-05-128Z/post-failure-service-status.json
/tmp/agent-browser-p46-s2-2026-06-24T18-56-05-128Z/post-failure-service-incidents-summary.json
/tmp/agent-browser-p46-current-status-after-s2-lock.json
```

Cleanup closed the leftover `default` service session:

```text
/tmp/agent-browser-p46-close-default-after-s2-lock.json
/tmp/agent-browser-p46-status-after-s2-lock-cleanup.json
```

Cleanup result:

- close command succeeded;
- final service status had no sessions;
- final service status had no browsers;
- final service status had no active incidents;
- no P46 external Chromium operator processes were left running.

Because this was the second consecutive S2 failure, the campaign is locked by
the P46 rule and must return to maintainer chat planning before any further S2
or later scenario execution.

## Required Planning Before Retry

The next S2 plan should keep the useful first-attempt UX coverage but repair
operator context launch and artifact capture:

- use an external operator browser that is not tracked as a service browser;
- prefer the known-good installed Chromium path from the runtime launch config
  or install-doctor output over `/snap/bin/chromium`;
- capture external Chromium stdout and stderr into the S2 artifact directory;
- fail fast with the exact executable path, port, and readiness URL when CDP
  readiness fails;
- continue to require authenticated dashboard screenshots, live rail checks,
  viewport refresh, route display proof, and zero active incidents.

Do not advance past S2 if authenticated dashboard UX proof cannot be captured.
That remains a P46 runner capability gap, not a product pass.

## Prepared S2 Harness Repair

After lock, the runner was changed without re-running S2:

- `launchExternalDashboardOperator` now receives the S2 baseline install-doctor
  JSON;
- external operator browsers prefer the verified
  `stealthcdp_chromium` executable from
  `launchConfig.browserBuildManifests.stealthcdp_chromium.executablePath`
  before falling back to system Chrome or Chromium commands;
- operator launch artifacts now include:
  - `<operator>-chromium-launch.json`;
  - `<operator>-chromium-stdout.log`;
  - `<operator>-chromium-stderr.log`;
- readiness failures include the exact CDP readiness URL;
- failed operator launches terminate the spawned process before propagating the
  failure.

No S2 retry was run after these changes because the plan is still locked by the
two-failure rule. The verified Chromium executable from the last S2
install-doctor artifact exists and is executable:

```text
/home/ecochran76/workspace.local/chromium/artifacts/chromium-stealthcdp/150.0.7835.0+stealthcdp.3676a7503929/chrome-linux/chrome
```

Validation after the no-live harness repair:

```bash
node --check scripts/run-p46-stress-scenario.js
git diff --check -- scripts/run-p46-stress-scenario.js docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md package.json
```

Both checks passed.

## S4 Runner Support And Lock

Date: 2026-06-27

S4 runner support was added:

- `scripts/lib/p46-scenario-harness.js` declares `s4` with two
  target-browser windows, two viewer clients, and one route lease.
- `scripts/run-p46-stress-scenario.js` supports `--scenario s4` from the
  explicit rebuilt-binary lane and captures command authority, reset evidence,
  one route-bound open plus one same-profile window open, dashboard proof,
  per-window navigation and new-tab controls, route-display screenshots,
  close-window-A cleanup, and window-B-after-close readback.
- `scripts/test-p47-scenario-harness.js` covers S4 metadata, capture and
  evaluation wiring, one daemon session against one runtime profile, one
  route-bound open plus one same-profile window open, and close-A verify-B
  evidence.

Validation:

```bash
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
```

Both checks passed.

Before live S4, the runtime baseline was repaired and verified:

- `./cli/target/debug/agent-browser --json install doctor` passed with one
  authoritative default socket listener.
- `./cli/target/debug/agent-browser --json service status` reported zero
  sessions, zero browsers, zero tabs, and zero active incidents.
- `./cli/target/debug/agent-browser --json doctor remote-view` reported ready.
- route-pool readiness was ready for routes A and B.
- route displays `:13` and `:14` were Openbox-only.

S4 attempt 1:

```bash
node scripts/run-p46-stress-scenario.js --scenario s4 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s4-2026-06-27T18-32-23-755Z
```

Result: failed.

Failure:

- window B reused the baseline route-pool JSON and selected route A instead of
  route B;
- `remote-view open` failed with `display_allocation_owner_mismatch` for
  `remote-view-display:13`;
- window A had reached `operatorVisible.state=ready` on route A before the
  second open failed.

Repair after attempt 1:

- S4 now pins window A to `guacamole-rdp-a`;
- S4 now pins window B to `guacamole-rdp-b`;
- no-live harness coverage asserts both route-pool entry IDs are present in
  the S4 runner.

S4 attempt 2:

```bash
node scripts/run-p46-stress-scenario.js --scenario s4 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s4-2026-06-27T18-34-00-072Z
```

Result: failed.

Failure:

- window B `remote-view open` timed out;
- window A initially reported `operatorVisible.state=ready`, then its browser
  process exited;
- route `guacamole:3` became `remote_view_finalization_incomplete`;
- finalization evidence showed `remote-view-display:13` orphaned and window A
  browser stream incomplete.

Because this was the second consecutive S4 failure, P46 is locked at S4. Do
not run another live S4 attempt until a follow-up plan addresses the
route-bound finalization and browser process-exit failure shape.

Post-lock cleanup:

- resolved `session:p46-s4-window-a`;
- resolved `remote-view-route:guacamole:3`;
- released retained route `guacamole:3` through authenticated local
  `/api/service/request` with action `service_remote_view_route_release`;
- final service status reported zero sessions, zero browsers, zero tabs, and
  zero active incidents;
- route-pool readiness was ready;
- route displays `:13` and `:14` were Openbox-only;
- install doctor passed with one authoritative default socket.

## S4 Post-Lock Diagnosis

Date: 2026-06-27

The second S4 failure is not safe to retry as-is. The successful window A
payload proves Chrome became operator-visible on `p46-s4-profile`, route
`guacamole:3`, and display `:13`. The window B command then asked for the same
runtime profile on a different route-pool entry and timed out; the audit found
window A had exited and left route-bound finalization incomplete.

Source and docs readback show the intended profile-sharing policy is one
retained browser lane for shared authenticated profiles. A second independent
Chrome process on the same profile requires explicit reviewed
`allowDuplicateProfileLane` intent, and the CLI S4 runner did not express that
intent or receive a typed early blocker. P53 now owns the repair before another
live S4 attempt:

```text
docs/dev/plans/0053-2026-06-27-s4-single-profile-window-topology-plan.md
```

P53 Goal 1 no-live guard was implemented in the S4 runner. The next S4 command
will now write `s4-topology-preflight.json` and stop with
`same_profile_multi_process_unsupported` before launching window B unless
reviewed duplicate-lane intent is explicit. This is still failure evidence, not
an S4 pass.

P53 Goal 2 selected the supported same-profile topology: one retained
remote-headed browser process, one route lease, one runtime profile, and two
top-level browser windows. `agent-browser window new [url] --same-profile` now
creates the second window in the current profile, and the S4 runner uses that
for window B instead of a second `remote-view open`. The runner also uses a
unique `p46-s4-window-<timestamp>` daemon session for each S4 run so stale named
session daemons cannot mask the rebuilt binary.

P53/S4 closeout:

- rebuilt and converged the local runtime with
  `pnpm converge:local-runtime -- --apply --json`;
- resolved the temporary `session:default` incident created while proving the
  old daemon-authority gate behavior;
- adjusted the S4 runner so explicit-command remediation runs allow no
  pre-existing daemon listener, while still rejecting duplicated or mismatched
  listeners;
- adjusted S4 evaluation to require one retained same-profile browser row, not
  two independent browser rows.

Validation:

```bash
node scripts/test-p47-scenario-harness.js
node --check scripts/run-p46-stress-scenario.js
cargo test --manifest-path cli/Cargo.toml test_window_new_same_profile_with_url
```

Live S4 retry:

```bash
node scripts/run-p46-stress-scenario.js --scenario s4 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s4-2026-06-27T19-12-55-449Z
```

Result: passed. The run proved one retained browser process
`session:p46-s4-window-2026-06-27T19-12-53-709Z`, runtime profile
`p46-s4-profile`, route `guacamole:3`, display `:13`, two same-profile
top-level windows, working dashboard refresh controls for both operators, and
window B remaining ready after closing window A. Reset-before and reset-after
both ended with zero active incidents.

## S5 Runner Support And Attempt 1

Date: 2026-06-27

S5 runner support was added:

- `scripts/lib/p46-scenario-harness.js` declares `s5` with two independent
  target browsers, two viewer clients, and two route leases.
- `scripts/run-p46-stress-scenario.js` supports `--scenario s5` from the
  explicit rebuilt-binary lane and captures command authority, reset evidence,
  two route-bound profile opens, dashboard proof, per-profile navigation and
  new-tab controls, route-display screenshots, close-profile-A cleanup, and
  profile-B-after-close-A readback.
- `scripts/test-p47-scenario-harness.js` covers S5 metadata, runner wiring,
  distinct `guacamole-rdp-a` and `guacamole-rdp-b` route-pool entries, and the
  profile B survival proof after closing profile A.

Validation:

```bash
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
cargo test --manifest-path cli/Cargo.toml test_remote_view_open_persist_request_route_pool_preserves_active_checkout
```

All checks passed after the route-pool persistence repair.

S5 attempt 1:

```bash
node scripts/run-p46-stress-scenario.js --scenario s5 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s5-2026-06-27T19-22-25-831Z
```

Result: failed.

Useful proof from the failed attempt:

- profile A and profile B both reached ready service browser state
  concurrently;
- profile A used route `guacamole:3`, display `:13`, and runtime profile
  `p46-s5-profile-a`;
- profile B used route `guacamole:4`, display `:14`, and runtime profile
  `p46-s5-profile-b`;
- both dashboard operators reached the expected workspace iframe and refresh
  controls;
- both route displays showed browser windows, not terminals;
- both profiles navigated and opened a new tab;
- profile B remained ready after closing profile A;
- reset-after returned to zero active incidents.

Failure:

- route A's route record remained ready and route-bound, but route-pool entry
  `guacamole-rdp-a` was overwritten to `available` with no
  `currentRouteAllocationId` before profile A closed;
- route B stayed checked out to `guacamole:4`;
- the evaluator correctly rejected the run because profile A route-bound
  finalization was incomplete.

Classification: route-pool persistence merge defect. A later `remote-view open`
request included baseline route-pool data and overwrote the first active route
checkout.

Repair:

- `remote_view_open_persist_request_route_pool` now preserves an existing active
  checkout when an incoming request supplies the same route-pool entry as
  inactive baseline data.
- A focused Rust guard covers the repair:
  `test_remote_view_open_persist_request_route_pool_preserves_active_checkout`.

The next S5 run is allowed as the first retry after an audited source repair.

S5 attempt 2:

```bash
node scripts/run-p46-stress-scenario.js --scenario s5 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s5-2026-06-27T19-32-56-059Z
```

Result: failed.

Useful proof from the failed retry:

- profile A opened ready as
  `session:p46-s5-profile-a-2026-06-27T19-32-56-645Z` on route
  `guacamole:3` and display `:13`;
- profile B opened ready as
  `session:p46-s5-profile-b-2026-06-27T19-32-56-645Z` on route
  `guacamole:4` and display `:14`;
- the route-pool persistence defect from attempt 1 did not repeat before the
  viewer-client failure;
- reset-after closed both S5 profile sessions and returned to zero active
  incidents.

Failure:

- operator A's external Chromium viewer did not become ready at
  `http://127.0.0.1:50102/json/version`;
- `operator-a-chromium-stderr.log` reported `bind() failed: Address already in
  use (98)` and `Cannot start http server for devtools`;
- the run stopped before authenticated dashboard UX control proof.

Classification: viewer-client adapter defect. DevTools port allocation for the
external dashboard viewer can collide with an already-bound port.

Because this was the second consecutive S5 failure, P46 is locked at S5. Do not
retry S5 until a follow-up plan repairs collision-resistant viewer-client port
allocation and adds no-live coverage for the launch artifact/readiness path.

## P54 S5 Viewer-Client Port Allocation Repair

Date: 2026-06-27

P54 repaired the S5 attempt 2 viewer-client adapter failure:

- `scripts/lib/p47-viewer-client.js` now launches external dashboard viewer
  clients with `--remote-debugging-port=0` by default.
- The module reads the resolved Chromium DevTools port from
  `DevToolsActivePort` inside the isolated viewer profile before calling
  `/json/version` or `/json`.
- The launch artifact records requested port, resolved port, readiness URL, and
  active-port evidence.
- explicit port overrides remain available through
  `P47_VIEWER_CLIENT_REMOTE_DEBUGGING_PORT` or
  `P46_VIEWER_CLIENT_REMOTE_DEBUGGING_PORT` for diagnostics only.
- `scripts/test-p47-viewer-client-separation.js` now covers dynamic-port
  launch metadata, `DevToolsActivePort` parsing, override validation, and the
  absence of the old random fixed-port selector.

Validation:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/test-p47-viewer-client-separation.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
pnpm test:p47-viewer-client-separation
```

All checks passed. P46 is unlocked for one S5 retry from the explicit
rebuilt-binary lane after the live baseline preflight passes.

S5 retry after P54:

```bash
node scripts/run-p46-stress-scenario.js --scenario s5 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s5-2026-06-27T19-41-29-598Z
```

Result: passed.

Evidence:

- operator A and operator B viewer-client launch artifacts used Chromium
  dynamic DevTools port allocation and resolved distinct ports;
- profile A opened ready on route `guacamole:3`, display `:13`, runtime
  profile `p46-s5-profile-a`;
- profile B opened ready on route `guacamole:4`, display `:14`, runtime
  profile `p46-s5-profile-b`;
- route-bound finalization passed for both profiles with no blockers;
- dashboard refresh controls worked for both external viewer clients;
- both route displays reached `browser_window_visible`;
- profile B stayed ready after profile A closed;
- reset-after returned to zero active incidents, and the final service status
  check reported zero sessions, zero browsers, zero tabs, and zero active
  incidents.

P46 is now in progress at S6.

## S6 Runner Support And Attempt 1

Date: 2026-06-27

S6 runner support was added:

- `scripts/lib/p46-scenario-harness.js` declares `s6` with two independent
  target browsers, two viewer clients, and two route leases.
- `scripts/run-p46-stress-scenario.js` reuses the two-profile S5 capture path
  in S6 mode and adds swapped dashboard selection proof before profile A
  cleanup.
- S6 captures operator A switching from profile A to profile B, operator B
  switching from profile B to profile A, swapped refresh controls, swapped
  dashboard screenshots, and selected browser/session readback for both
  operators.
- `scripts/test-p47-scenario-harness.js` covers S6 metadata, runner wiring,
  swapped dashboard-state artifacts, and swapped selected-browser evidence.

Validation before live attempt 1:

```bash
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
node scripts/test-p47-viewer-client-separation.js
```

S6 attempt 1:

```bash
node scripts/run-p46-stress-scenario.js --scenario s6 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s6-2026-06-27T19-49-19-793Z
```

Result: failed and was manually interrupted after exceeding the expected
bounded runtime.

Useful proof from the failed attempt:

- both profile browsers opened and remained ready;
- both initial dashboard viewer clients became ready;
- both initial refresh controls worked;
- dashboard screenshots were captured for both operators;
- both profiles navigated and opened tabs.

Failure:

- no swapped dashboard selection artifacts were written;
- the harness stalled before
  `operator-a-swapped-to-profile-b-dashboard-state.json`;
- the viewer-client CDP adapter had no per-command timeout, so a stuck
  `Page.navigate` or equivalent CDP command could leave the harness waiting
  indefinitely.

Cleanup:

- closed `p46-s6-profile-a-2026-06-27T19-49-20-377Z`;
- closed `p46-s6-profile-b-2026-06-27T19-49-20-377Z`;
- final status showed zero sessions, zero browsers, zero tabs, zero active
  incidents, and both route-pool entries available.

Repair:

- `scripts/lib/p47-viewer-client.js` now bounds every CDP command with a
  30000ms timeout.
- `scripts/test-p47-viewer-client-separation.js` asserts the timeout guard is
  present.

This was the first S6 failure. One S6 retry is allowed after the focused
validation and live baseline preflight pass.

S6 attempt 2:

```bash
node scripts/run-p46-stress-scenario.js --scenario s6 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s6-2026-06-27T19-56-33-105Z
```

Result: failed.

Failure:

- `Page.navigate` timed out after 30000ms during the swapped dashboard
  selection step;
- the run stopped before swapped selection artifacts were written;
- reset-after reported zero active incidents but did not close the two retained
  browser rows because their session rows were already missing.

Manual cleanup:

- closed `p46-s6-profile-a-2026-06-27T19-56-29-450Z`;
- closed `p46-s6-profile-b-2026-06-27T19-56-29-450Z`;
- final service status reported zero sessions, zero browsers, zero tabs, zero
  active incidents, and both route-pool entries available;
- route displays returned to `non_browser_windows`.

Additional repair after the failed retry:

- `scripts/run-p46-stress-scenario.js` reset now derives close targets from
  both service session rows and retained browser rows, including
  `activeSessionIds` and `session:<name>` browser IDs.
- `scripts/test-p47-scenario-harness.js` asserts this retained-browser reset
  contract.

Because this was the second consecutive S6 failure, P46 is locked at S6. Do not
retry S6 until a follow-up plan repairs the dashboard swap navigation path and
proves reset-after closes retained browser rows whose session rows are missing.

## P55 S6 Dashboard Swap Navigation Repair

Date: 2026-06-27

P55 repaired the S6 dashboard swap path:

- `scripts/lib/p47-viewer-client.js` now exports
  `navigateDashboardViewerClient`, which changes `window.location` from inside
  the already-loaded dashboard page instead of sending raw `Page.navigate`.
- `scripts/run-p46-stress-scenario.js` now captures
  `operator-a-swapped-to-profile-b-navigate.json` and
  `operator-b-swapped-to-profile-a-navigate.json` before waiting for swapped
  dashboard state.
- The retained-browser-aware reset repair remains in place.
- `scripts/test-p47-viewer-client-separation.js` covers the in-page dashboard
  navigation helper.
- `scripts/test-p47-scenario-harness.js` covers the swapped navigation
  artifacts and asserts the S6 swap path does not use raw `Page.navigate`.

Validation:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
```

All checks passed. P46 is unlocked for one S6 retry from the explicit
rebuilt-binary lane after live baseline preflight passes.

## P55 Retry Failure And P56 Repair

Date: 2026-06-27

P55-authorized S6 retry:

```bash
node scripts/run-p46-stress-scenario.js --scenario s6 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s6-2026-06-27T20-06-51-508Z
```

Result: failed.

Evidence:

- `operator-a-swapped-to-profile-b-navigate.json` exists and records `ok:
  true`;
- `operator-a-swapped-to-profile-b-dashboard-state.json` was not written;
- failure was `CDP command Runtime.evaluate timed out after 30000ms`;
- reset-after did not close the two retained browser rows because the large
  service-status payload hit Node `spawnSync` output buffering and parsed as
  `null`.

Manual cleanup:

- closed `p46-s6-profile-a-2026-06-27T20-06-52-041Z`;
- closed `p46-s6-profile-b-2026-06-27T20-06-52-041Z`;
- final status showed zero sessions, zero browsers, zero tabs, zero active
  incidents, and both route-pool entries available.

P56 follow-up:

- added `docs/dev/plans/0056-2026-06-27-s6-dashboard-reconnect-and-reset-buffer-plan.md`;
- `scripts/lib/p47-viewer-client.js` now exports
  `reconnectDashboardViewerClient`, which rediscovers the active dashboard page
  through the viewer-client DevTools `/json` endpoint and replaces the CDP
  websocket after swap navigation;
- `scripts/run-p46-stress-scenario.js` now writes swapped reconnect artifacts
  before dashboard-state polling;
- the runner command wrapper now uses a 32 MiB `spawnSync` buffer to keep large
  service-status reset evidence parseable.

P46 remains locked at S6 until P56 validation passes and a P56-authorized retry
runs.

## P56 Retry Failure And P57 Diagnostic Plan

Date: 2026-06-27

P56 validation passed:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
git diff --check -- scripts/lib/p47-viewer-client.js scripts/run-p46-stress-scenario.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0055-2026-06-27-s6-dashboard-swap-navigation-plan.md docs/dev/plans/0056-2026-06-27-s6-dashboard-reconnect-and-reset-buffer-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md
```

P56 preflight passed with install doctor success, remote-view doctor ready,
zero active incidents, ready route-pool entries on `:13` and `:14`, and idle
route displays.

P56-authorized S6 retry:

```bash
node scripts/run-p46-stress-scenario.js --scenario s6 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s6-2026-06-27T20-15-52-909Z
```

Result: failed.

Evidence:

- `operator-a-swapped-to-profile-b-navigate.json` exists and records `ok:
  true`;
- `operator-a-swapped-to-profile-b-reconnect.json` was not written;
- failure was `CDP command Page.enable timed out after 30000ms`;
- reset-after closed both retained sessions:
  `p46-s6-profile-a-2026-06-27T20-15-53-475Z` and
  `p46-s6-profile-b-2026-06-27T20-15-53-475Z`;
- final readback showed zero sessions, zero browsers, zero tabs, zero active
  incidents, and both route-pool entries available.

P57 follow-up:

- added
  `docs/dev/plans/0057-2026-06-27-s6-viewer-client-cdp-swap-diagnostics-plan.md`;
- next repair must capture DevTools `/json` page-list evidence before sending
  `Page.enable` or any other command to the rediscovered page websocket.

P46 remains locked at S6. Do not run another S6 retry until P57 records a
concrete diagnostic result and repair.

## P57 Reconnect Discovery Repair

Date: 2026-06-27

P57 implemented target-discovery evidence before reconnect commands:

- `scripts/lib/p47-viewer-client.js` writes reconnect discovery data before
  sending commands to the rediscovered page websocket;
- discovery includes the DevTools `/json` page list, chosen page target,
  previous page target, selected URL, websocket URL, `samePageId`, and
  `sameWebSocketDebuggerUrl`;
- `scripts/run-p46-stress-scenario.js` requests
  `operator-a-swapped-to-profile-b-reconnect-discovery.json` and
  `operator-b-swapped-to-profile-a-reconnect-discovery.json`.

Diagnostic decision:

- P56 proved the reconnect command sequence itself can hang on `Page.enable`;
- P57 skips `Page.enable` and `Runtime.enable` during reconnect because the
  subsequent dashboard-state and screenshot reads use direct CDP commands and
  do not require event-domain enablement first.

Validation:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
```

All checks passed. One P57-authorized S6 retry may run after live preflight.

P57-authorized S6 retry:

```bash
node scripts/run-p46-stress-scenario.js --scenario s6 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s6-2026-06-27T20-23-53-164Z
```

Result: failed.

Evidence:

- `operator-a-swapped-to-profile-b-navigate.json` requested profile B;
- `operator-a-swapped-to-profile-b-reconnect-discovery.json` showed the chosen
  DevTools page URL still pointed at profile A;
- `operator-a-swapped-to-profile-b-reconnect.json` was written with
  `domainEnableSkipped: true`;
- failure was `CDP command Runtime.evaluate timed out after 30000ms`;
- reset-after closed both retained sessions and final readback showed zero
  sessions, zero browsers, zero tabs, zero active incidents, and both
  route-pool entries available.

P58 follow-up:

- added
  `docs/dev/plans/0058-2026-06-27-s6-dashboard-page-url-stabilization-plan.md`;
- `scripts/lib/p47-viewer-client.js` now exports
  `waitForDashboardViewerClientPageUrl`, which polls DevTools `/json` until the
  chosen page URL matches the requested dashboard URL;
- `scripts/run-p46-stress-scenario.js` now writes swapped page-URL artifacts
  before reconnecting CDP.

P58 authorizes one S6 retry after live preflight.

P58-authorized S6 retry:

```bash
node scripts/run-p46-stress-scenario.js --scenario s6 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s6-2026-06-27T20-28-52-103Z
```

Result: failed.

Evidence:

- `operator-a-swapped-to-profile-b-navigate.json` requested profile B;
- `operator-a-swapped-to-profile-b-page-url.json` showed the selected DevTools
  page URL remained on profile A;
- failure occurred before reconnect while waiting for the profile B dashboard
  URL;
- reset-after closed both retained sessions and final readback showed zero
  sessions, zero browsers, zero tabs, zero active incidents, and both
  route-pool entries available.

P59 follow-up:

- added `docs/dev/plans/0059-2026-06-27-s6-dashboard-history-swap-plan.md`;
- same-origin dashboard swaps now use `history.pushState` and dispatch
  `popstate`;
- cross-origin swaps still fall back to `window.location.assign`.

P59 authorizes one S6 retry after validation and live preflight.

P59-authorized S6 retry:

```bash
node scripts/run-p46-stress-scenario.js --scenario s6 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s6-2026-06-27T20-32-54-709Z
```

Result: passed.

Evidence:

- operator A swapped from profile A to profile B and read back
  `session:p46-s6-profile-b-2026-06-27T20-32-55-285Z`;
- operator B swapped from profile B to profile A and read back
  `session:p46-s6-profile-a-2026-06-27T20-32-55-285Z`;
- both swapped refresh controls clicked successfully;
- both swapped screenshots were captured;
- closing profile A released route A while profile B stayed ready;
- reset-after closed the remaining retained profile B session and final runtime
  readback showed zero sessions, zero browsers, zero tabs, zero active
  incidents, and both route-pool entries available.

P60 follow-up:

- added `docs/dev/plans/0060-2026-06-27-s7-route-pool-exhaustion-plan.md`;
- added S7 metadata, live capture, evaluator checks, and no-live harness
  assertions for route-pool exhaustion and retry-after-release behavior;
- tightened `plan_remote_view_acquisition` so unpinned route-bound demand that
  selects a checked-out pool display owned by another session reports
  `route_pool_exhausted` before owner-mismatch fallback;
- rebuilt `./cli/target/debug/agent-browser` and restarted the stale default
  daemon before the final live retry.

Validation:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml acquisition_plan_reports_route_pool_exhausted -- --nocapture
node scripts/test-p47-scenario-harness.js
cargo build --manifest-path cli/Cargo.toml
node scripts/run-p46-stress-scenario.js --scenario s7 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

S7 pass artifact:

```text
/tmp/agent-browser-p46-s7-2026-06-27T20-58-30-721Z
```

Evidence:

- profile A and profile B occupied both route-pool entries;
- the third route-bound request failed closed with `route_pool_exhausted`;
- the failed third demand did not create a retained profile C browser row;
- both occupied route displays stayed browser-window-visible with no terminal
  fallback;
- retry after profile A close succeeded for profile C;
- reset-after closed remaining retained sessions and reported zero active
  incidents.

P61 follow-up:

- added `docs/dev/plans/0061-2026-06-27-s8-display-access-recovery-plan.md`;
- added S8 metadata, live capture, evaluator checks, and no-live harness
  assertions for display-access denial and recovery;
- used a temporary `timeout` shim in `PATH` to simulate display-access denial
  safely without mutating host X11 permissions;
- reran the same route-bound open under normal display access as the repair
  proof.

Validation:

```bash
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
git diff --check -- scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js
node scripts/run-p46-stress-scenario.js --scenario s8 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

S8 pass artifact:

```text
/tmp/agent-browser-p46-s8-2026-06-27T21-07-22-844Z
```

Evidence:

- denied open failed before browser launch with
  `display_access_grant_failed`;
- cleanup evidence reported `skipped_before_browser_launch`,
  `restoredDisplayAllocation: true`, `restoredRemoteViewRoute: true`, and
  `restoredRoutePoolEntry: true`;
- denied demand created no retained denied-profile browser row;
- route A and route B remained terminal-free after denial;
- repair open succeeded with `displayAccessGrant.state: already_ready`;
- route A became browser-window-visible and route-bound finalization passed;
- reset-after closed the repair session and reported zero active incidents.

P46 is now in progress at S9.

## 2026-06-27 S9 Lock And P62 Follow-up

S9 implementation work added stale target and duplicate tab stress metadata,
live capture, evaluator checks, and no-live source assertions. The viewer-client
helper now has a narrow `allowRecoveredStaleTab` option so S9 can capture the
dashboard's stale selected-tab recovery notice without accepting that state as
the final selected target proof.

Validation before live retry:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
git diff --check -- scripts/lib/p47-viewer-client.js scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js
```

First live attempt:

```text
/tmp/agent-browser-p46-s9-2026-06-27T21-32-27-404Z
```

Result: failed during operator C initial blank-tab dashboard state wait. The
dashboard recovered the stale blank target to another live tab and the first
harness version treated the recovery notice as a launch failure.

Harness repair:

- accepted the initial `Recovered stale selected tab identity` state only for
  S9 operator C;
- required the initial state to have a mismatched tab param and recovery notice;
- kept the final post-navigation operator C state as an exact blank-tab readback
  requirement;
- added no-live checks for the helper option and the imported page URL helper.

Corrected live attempt:

```text
/tmp/agent-browser-p46-s9-2026-06-27T21-42-54-990Z
```

Result: failed before S9 pass. Evidence:

- blank target:
  `target:92B1ABA4B645E77E3C72BE117CD14832`;
- duplicate target A:
  `target:1650B99DC1120C753CE97BFE43050090`;
- duplicate target B:
  `target:AD26C70D100CF68CAD54C74B5D225325`;
- operator C initial state reported `recoveredStaleTab: true` and the stale
  selected-tab recovery notice;
- CLI selection and navigation of the blank tab succeeded with
  `https://www.iana.org/domains/reserved?p46=s9-blank-recovered`;
- when the harness returned operator C to the requested blank-tab dashboard URL,
  the dashboard URL stayed on duplicate target A instead of the blank target;
- reset-after reported zero sessions, zero browsers, zero tabs, and zero active
  incidents.

P62 follow-up:

- added `docs/dev/plans/0062-2026-06-27-s9-stale-target-recovery-plan.md`;
- P46 is locked at S9 pending product or contract repair for dashboard
  selected-target recovery after a blank tab becomes live.

Do not run another S9 retry until P62 records validation-backed retry
authorization.

## 2026-06-27 P62 Clearance And S9 Pass

P62 repaired the S9 selected-target recovery contract. The dashboard now
preserves an explicitly selected live blank tab as the selected target instead
of immediately rewriting the URL to another live non-blank tab. Missing or dead
stale selections still recover to a current live tab.

Implementation:

- updated `selectedTabForBrowser` in
  `packages/dashboard/src/components/workspace-remote-viewport.tsx`;
- updated the S9 viewer-client recovery helper and evaluator to accept exact
  blank-target preservation or typed stale-target recovery;
- updated no-live source guards in dashboard, viewer-client, and scenario
  harness tests.

Validation:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-dashboard-view-streams.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
git diff --check -- packages/dashboard/src/components/workspace-remote-viewport.tsx scripts/lib/p47-viewer-client.js scripts/run-p46-stress-scenario.js scripts/test-dashboard-view-streams.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js
pnpm publish:local-dashboard -- --skip-smoke --json
agent-browser --json install doctor
node scripts/smoke-local-dashboard-runtime.js --dashboard-url http://127.0.0.1:4848/ --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --skip-browser --json
node scripts/run-p46-stress-scenario.js --scenario s9 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Installed runtime proof:

- `agent-browser --json install doctor` reported `success=true` and zero
  issues;
- live dashboard runtime was ready at
  `http://127.0.0.1:4848/api/runtime/manifest`;
- executable SHA:
  `81c89d6ce16c10a20d019d673d72a755f0145d63433e6a0c96e40a2da5373b4b`;
- dashboard SHA:
  `ae0c9c440786bed461b0e66a93843f5e96151cd29a2bd325dd33e4433ac81968`.

S9 pass artifact:

```text
/tmp/agent-browser-p46-s9-2026-06-27T22-03-14-950Z
```

Evidence:

- distinct tab IDs:
  `target:BC61994FBD06EADBD71FCF1EFE607F1E`,
  `target:7F12E4CC20D48BC2883B1A775C207E6E`, and
  `target:A75D1D87CFC15493A9AB5E65A848DE7C`;
- initial blank target selection was exact:
  `blankInitialExactSelection: true`;
- blank target navigated to
  `https://www.iana.org/domains/reserved?p46=s9-blank-recovered`;
- duplicate A stayed on the IANA URL after duplicate B navigated to
  `https://example.org/?p46=s9-duplicate-b`;
- route display `:13` was `browser_window_visible`;
- route-bound finalization was complete with no blockers;
- reset-after closed `default` and reported zero active incidents.

P62 is complete. P46 is now in progress at S10.

## S10 First Attempts

Commands:

```bash
node scripts/run-p46-stress-scenario.js --scenario s10 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact directories:

```text
/tmp/agent-browser-p46-s10-2026-06-27T22-17-57-154Z
/tmp/agent-browser-p46-s10-2026-06-27T22-20-21-552Z
```

Result: locked by the two-consecutive-failure rule before S10 evaluation.

The runner opened the service-owned route-bound browser, then failed while
trying to read dashboard inventory:

- attempt one queried `/sessions` and received dashboard HTML instead of JSON;
- attempt two queried `/api/sessions` without dashboard authentication and
  received HTTP 401.

Both attempts reset cleanly. Reset-after reported zero active incidents.

Follow-up plan:

```text
docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md
```

Harness repair completed before stopping:

- added S10 scenario metadata for one service-owned target browser, one
  zero-lease foreign CDP browser, and one zero-lease dashboard operator;
- added a live foreign CDP launcher that uses a Chromium profile outside
  `~/.agent-browser` with dynamic DevTools port allocation;
- added authenticated dashboard JSON reads through the logged-in viewer-client
  for `/api/sessions` and `/api/session-tabs?port=<foreign-cdp-port>`;
- added S10 evaluator checks for foreign non-owned classification, mutation
  gating, route/display non-borrowing, service-owned control readiness, and
  selected workspace stability.

Validation after the authenticated inventory fix:

```bash
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
```

P46 is locked at S10. Do not run another S10 retry until P63 records the full
green preflight and authorizes exactly one live retry.

## 2026-06-27 P63 Clearance And S10 Pass

P63 repaired the S10 authenticated foreign CDP inventory path and cleared the
S10 lock.

Implementation:

- dashboard `/api/session-tabs?port=<foreign-cdp-port>` now falls back from
  agent-browser `/api/tabs` to raw Chrome CDP `/json/list`;
- the local dashboard proxy now reads responses until declared
  `Content-Length` rather than waiting for backend connection close;
- S10 now reads `/api/sessions` and `/api/session-tabs` through the
  authenticated dashboard viewer-client session;
- foreign CDP browser cleanup is best-effort and no longer masks scenario
  failures;
- selected workspace probing accepts viewport-route context when the optional
  detail panel is not mounted;
- foreign route-borrow detection is scoped to selected-workspace facts and
  selected viewport text, not global workspace-list text.

Validation:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml dashboard -- --nocapture
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
node --check scripts/run-p46-stress-scenario.js
node --check scripts/lib/p46-scenario-harness.js
node scripts/test-p47-scenario-harness.js
node scripts/test-dashboard-workspace-nodes.js
git diff --check -- cli/src/native/stream/dashboard.rs scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md
pnpm publish:local-dashboard -- --skip-smoke --json
/home/ecochran76/.local/bin/agent-browser --json install doctor
node scripts/smoke-local-dashboard-runtime.js --dashboard-url http://127.0.0.1:4848/ --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --skip-browser --json
/home/ecochran76/.local/bin/agent-browser --json service incidents --summary
node scripts/run-p46-stress-scenario.js --scenario s10 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Installed runtime proof:

- `agent-browser --json install doctor` reported `success=true` and zero
  issues;
- live dashboard runtime smoke passed against `http://127.0.0.1:4848/`;
- executable SHA:
  `502f05830dfb756cda44eae7d6bb8c71999dd4ce39ee109eb51ff36136de155a`;
- dashboard SHA:
  `6f850d76121720b42ed3f386cbcf415e9f884fba04168f77688a2a02cbdb78a6`.

S10 pass artifact:

```text
/tmp/agent-browser-p46-s10-2026-06-27T22-52-43-936Z
```

Evidence:

- foreign CDP row detected as `ownership: foreign_cdp`, `provider:
  detected-cdp`, and `addressability: cdp_reachable`;
- foreign tab inventory returned through authenticated dashboard
  `/api/session-tabs`;
- selected foreign workspace stayed on
  `daemon-session:detected-s10-foreign-cdp-profile-nnyhwn-38405`;
- `foreignRouteBorrowed: false`;
- `foreignContextStable: true`;
- `serviceControlReady: true`;
- `serviceContextStable: true`;
- `serviceBrowserHasRoute: true`;
- route `guacamole:3`, route-pool entry `guacamole-rdp-a`, and display `:13`
  remained bound to the service-owned browser;
- reset-before and reset-after ended with zero active incidents.

The accepted warnings are limited to the optional selected-workspace detail
panel not being mounted on the route used by the harness. S10 used
viewport-route context and dashboard session capabilities as evidence.

P63 is complete. P46 is now in progress at S11.

## S11 First Attempt

Command:

```bash
node scripts/run-p46-stress-scenario.js --scenario s11 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s11-2026-06-27T23-02-14-303Z
```

Result: failed in the S11 harness before evaluator.

The run opened the service-owned route-bound browser, launched the
authenticated dashboard viewer, proved dashboard reload, and pushed the stale
dashboard URL. The dashboard then recovered the URL from stale target
`target:p46-s11-stale-target` back to live target
`target:9E6E67F299F964040CC151188712AF9C`.

The harness failure was waiting for the exact stale URL to persist in DevTools
page discovery. S11 should allow stale target IDs to be rejected or recovered,
so the harness now records stale URL readback after recovery and waits for the
dashboard stale-target recovery state instead.

Reset-after closed `default` and reported zero active incidents.

One S11 retry is authorized after focused no-live checks pass and active
incidents remain zero immediately before the run.

## S11 Lock And P64 Repair

Second S11 attempt artifact:

```text
/tmp/agent-browser-p46-s11-2026-06-27T23-05-10-207Z
```

Result: locked by the two-consecutive-failure rule before S11 evaluation.

The dashboard again rejected the stale target, but this time the harness had
already moved past the exact stale URL wait. The last dashboard state showed a
healthy route-bound viewport for browser/session `session:default`, with iframe
`http://127.0.0.1:8092/guacamole/#/client/MwBjAHBvc3RncmVzcWw=`, refresh
control, no password prompt, and live target
`target:734BA856384987537291FC56C628FFBE` instead of the requested stale
target `target:p46-s11-stale-target`.

The harness was still too narrow because it required explicit
stale-recovery notice text. S11 pass criteria allow stale target IDs to be
rejected or recovered, so P64 now scopes a harness repair that accepts immediate
live-target rewrite for the same browser/session with a healthy iframe.

Follow-up plan:

```text
docs/dev/plans/0064-2026-06-27-s11-stale-url-live-target-recovery-plan.md
```

No-live repair completed:

- added `allowRecoveredLiveTab` to the viewer-client dashboard-state wait
  helper without changing default strict matching;
- wired S11 stale URL and post-refresh waits to accept either explicit stale
  recovery notice or immediate live-target rewrite;
- added no-live source guards for the scoped helper and S11 evaluator evidence.

P46 is locked at S11. Do not run another S11 retry until P64's validation gate
passes and P64 authorizes exactly one retry.

## P64 Clearance And S11 Pass

P64 repaired the S11 stale URL live-target recovery acceptance boundary and
cleared the S11 lock.

Validation:

```bash
node --check scripts/run-p46-stress-scenario.js
node --check scripts/lib/p46-scenario-harness.js
node --check scripts/lib/p47-viewer-client.js
node scripts/test-p47-scenario-harness.js
node scripts/test-p47-viewer-client-separation.js
git diff --check -- scripts/lib/p47-viewer-client.js scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js scripts/test-p47-viewer-client-separation.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0064-2026-06-27-s11-stale-url-live-target-recovery-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md
/home/ecochran76/.local/bin/agent-browser --json service incidents --summary
node scripts/run-p46-stress-scenario.js --scenario s11 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command-match --require-agent-browser-daemon-command-match
```

S11 pass artifact:

```text
/tmp/agent-browser-p46-s11-2026-06-27T23-09-57-372Z
```

Evidence:

- dashboard reload restored the route-bound browser viewport;
- stale URL request for `target:p46-s11-stale-target` recovered to live target
  `target:2AE5C8E4CED37D0C1A77771CD3CA9F58`;
- `staleRecovered: true`;
- `staleRecoveredLiveTab: true`;
- viewer-client reconnect after stale URL recovery succeeded;
- viewport refresh after stale URL recovery clicked successfully;
- direct Guacamole frame URL returned
  `200 http://127.0.0.1:8092/guacamole/#/client/MwBjAHBvc3RncmVzcWw=`;
- route display `:13` remained `browser_window_visible`;
- route-bound finalization was complete with no blockers;
- reset-before and reset-after both ended with zero active incidents.

Command metadata caveat: the pass used an explicit installed binary command and
daemon realpath matching was enforced and passed, but the explicit-command
guard flag was misspelled in the retry. The artifact reports
`requireExplicit: false`, while also reporting `explicit: true` and the
installed binary realpath.

P64 is complete. P46 is now in progress at S12.

## S12 Harness And Lock

S12 harness support now runs ten normal-use cycles. Each cycle captures
before/after boundary doctor, incident, service-status, and route-pool
evidence; opens a route-bound browser; reloads the dashboard; reconnects the
viewer-client; clicks viewport refresh; navigates; creates and switches tabs;
checks direct Guacamole; captures route-bound finalization; closes; and resets.

No-live validation:

```bash
node --check scripts/run-p46-stress-scenario.js
node --check scripts/lib/p46-scenario-harness.js
node --check scripts/test-p47-scenario-harness.js
node scripts/test-p47-scenario-harness.js
node scripts/test-p47-viewer-client-separation.js
```

First S12 attempt artifact:

```text
/tmp/agent-browser-p46-s12-2026-06-27T23-20-14-868Z
```

The first attempt completed all ten cycles. Evidence showed zero active
incidents, zero sessions, zero browsers, zero tabs, route-pool baseline true,
and direct Guacamole HTTP 200 for every cycle. It failed only because the new
S12 evaluator counted completed acquisition-lease history as active pressure.
The evaluator was corrected to exclude completed or checked-out historical
leases from active pressure.

Second S12 attempt artifact:

```text
/tmp/agent-browser-p46-s12-2026-06-27T23-39-14-415Z
```

The second attempt exposed real route-pool reset drift. Cycle 3 left route
`guacamole:3` orphaned on display allocation `remote-view-display:13`, and
route-pool entry `guacamole-rdp-a` remained checked out. Cycle 4 then failed
route-bound open with `route_pool_entry_unavailable`.

Cleanup was performed through service-owned actions:

- authenticated `service_route_pool_repair` dry-run found stale checkout
  `guacamole-rdp-a`, stale route `guacamole:3`, and stale display allocation
  `remote-view-display:13`;
- authenticated `service_route_pool_repair` apply repaired all three;
- `agent-browser --json service reconcile` showed both route-pool entries
  available, `guacamole:3` released, and `remote-view-display:13` released;
- the transient browser recovery incident for
  `session:s12-cycle-03-2026-06-27T23-43-15-712Z` was resolved with an explicit
  note after repair;
- final `service incidents --summary` reported no active incidents, only the
  recovered incident record.

Historical state, superseded by the repair and S12 clearance below: P46 was
locked at S12 until a follow-up repair addressed orphaned route-bound display
cleanup after normal close.

## S12 Route Cleanup Repair And Selector Audit

Follow-up repair:

- `merge_reconciled_service_state` now preserves newer remote-view release
  mutations instead of allowing a stale reconciler snapshot to resurrect a
  released route, display allocation, or available route-pool entry.
- Normal close cleanup now releases display allocations and routes by
  `owner_browser_id` or `owner_session_id` even when a concurrent process-exit
  reconcile removed the browser row before close persistence ran.
- Regression coverage now includes a close path where the browser row is
  already absent but `remote-view-display:13`, `guacamole:3`, and
  `guacamole-rdp-a` remain session-owned.

Validation:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml close_releases_session_owned_route_after_process_exit_removed_browser -- --nocapture
cargo test --manifest-path cli/Cargo.toml native::service_health::tests:: -- --nocapture
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo build --release --manifest-path cli/Cargo.toml
pnpm converge:local-runtime -- --apply --json
```

Installed-runtime evidence:

- local-runtime convergence passed after stale daemon listeners were closed;
- final install doctor and remote-view doctor passed;
- installed command SHA:
  `43d85bebf6c2e68fb7b86a5e9a1628f6e20698d7140533bb033bf932dd26c113`;
- install doctor artifact:
  `/tmp/agent-browser-p46-s12-install-doctor-after-second-repair.json`.

Third S12 attempt artifact:

```text
/tmp/agent-browser-p46-s12-2026-06-28T00-43-57-985Z
```

Route-pool result:

- all ten cycles completed;
- each cycle reported zero active incidents before close, after reset, and at
  the after boundary;
- each cycle returned to route-pool baseline after reset;
- post-reset pressure did not increase;
- checked-out route-pool, active remote-view routes, sessions, browsers, and
  tabs were all zero after every reset;
- direct Guacamole HTTP returned 200 for every cycle.

The remaining failure was classified as a harness selector defect, not
route-pool drift. S12 created a new `example.org` tab, but the tab list can
retain older same-session targets across repeated cycles and does not always
include target IDs. The harness fell back to `tabs[1]`, sometimes switching to
an older IANA tab while the evaluator expected the current cycle's
`example.org` tab.

Harness repair:

- S12 now prefers the current `tab new` command result when selecting the tab to
  switch to.
- It matches tab-list entries by service tab ID when available, otherwise by the
  exact returned index and URL.
- The no-live harness test now guards that S12 uses `tabMatchesCommandData`
  before `tabSelector`.

No-live validation after the selector repair:

```bash
node --check scripts/run-p46-stress-scenario.js
node --check scripts/test-p47-scenario-harness.js
node scripts/test-p47-scenario-harness.js
```

S12 is unlocked for one retry of the selector-repaired harness. The route-pool
cleanup defect has artifact-backed live evidence of repair; the retry must
confirm the final S12 pass with the same installed command SHA or a newer
validated runtime.

## S12 Clearance

Selector-repaired S12 retry artifact:

```text
/tmp/agent-browser-p46-s12-2026-06-28T01-05-24-861Z
```

Outcome:

- S12 passed.
- The artifact reports `requireExplicit: true`, `explicit: true`, and daemon
  realpath matching passed for `/home/ecochran76/.local/bin/agent-browser`.
- All ten cycles completed.
- Each cycle restored route-pool baseline after reset.
- Active incidents were zero before close, after reset, and after the boundary
  checks in every cycle.
- Post-reset pressure did not increase.
- Checked-out route-pool, active remote-view routes, sessions, browsers, and
  tabs were zero after every reset.
- Direct Guacamole frame readback returned HTTP 200 in every cycle.
- Reset-before and reset-after both ended with zero active incidents.

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

P46 S12 is cleared.

## Campaign Closeout

P46 is complete through S12. Final evidence:

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

- install doctor succeeded with no issues and runtime status `converged`;
- remote-view doctor status was `ready`;
- incident summary count was 0;
- route-pool readiness succeeded;
- service status reported zero service browsers, zero service sessions, zero
  tabs, and zero active incidents;
- route-pool entries `guacamole-rdp-a` and `guacamole-rdp-b` were available
  with no current route allocation.

Residual risks:

- Historical orphaned display-allocation records remain visible in service
  status, but they are not live control rows and do not hold route-pool
  capacity.
- S12 repeats one route-bound normal-use reset contract; S6 remains the
  cross-observation proof for two live profiles.
- Graphiti closeout memory writes timed out, so the repo docs and `/tmp`
  artifacts remain the authoritative durable record.

Next hardening target:

- Create a retained-state compaction and doctor-surface plan for historical
  orphaned display allocations and stale metadata visibility.
