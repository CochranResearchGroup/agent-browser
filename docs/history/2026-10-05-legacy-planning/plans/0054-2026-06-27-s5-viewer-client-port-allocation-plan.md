# P54 S5 Viewer-Client Port Allocation Plan

Date: 2026-06-27
State: COMPLETE
Lane: P54
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`
- `docs/dev/plans/0047-1-2026-06-24-viewer-client-target-browser-separation-plan.md`

## Purpose

Unlock P46 S5 without a blind third live retry. S5 attempt 2 proved both
profile browsers can open ready on distinct routes after the route-pool
persistence repair, but the external dashboard viewer-client failed before
dashboard UX proof because Chromium could not bind its DevTools HTTP server on
the randomly selected port `50102`.

## Current Evidence

- S5 attempt 2 artifact:
  `/tmp/agent-browser-p46-s5-2026-06-27T19-32-56-059Z`.
- Profile A opened ready as
  `session:p46-s5-profile-a-2026-06-27T19-32-56-645Z` on route
  `guacamole:3` and display `:13`.
- Profile B opened ready as
  `session:p46-s5-profile-b-2026-06-27T19-32-56-645Z` on route
  `guacamole:4` and display `:14`.
- `operator-a-chromium-stderr.log` reported `bind() failed: Address already in
  use (98)` and `Cannot start http server for devtools`.
- Reset-after closed both S5 profile sessions and returned to zero active
  incidents.

## Non-Negotiable Rules

- Do not run S5 again until no-live validation proves the viewer-client launch
  path is collision-resistant.
- Do not route dashboard viewers through service-owned `agent-browser`
  sessions.
- Preserve P47 viewer-client separation: viewer clients consume zero route
  leases and cannot call target-browser service commands.
- Preserve P46's visual proof standard for the retry.

## Goal 1: Make Viewer-Client DevTools Port Allocation Collision-Resistant

Work:

- Stop selecting a random fixed DevTools port by default.
- Launch external Chromium viewer clients with `--remote-debugging-port=0` so
  Chromium allocates an available port.
- Read the resolved port from the isolated viewer profile's
  `DevToolsActivePort` file before calling `/json/version` or `/json`.
- Keep an explicit environment override only for diagnostic use.
- Record both requested and resolved ports in the launch artifact.

Evidence:

- `node --check scripts/lib/p47-viewer-client.js`
- `pnpm test:p47-viewer-client-separation`

## Goal 2: Add No-Live Guard Coverage

Work:

- Extend `scripts/test-p47-viewer-client-separation.js` so the default launch
  descriptor uses dynamic DevTools allocation.
- Prove `DevToolsActivePort` parsing from a profile directory.
- Assert the viewer-client source no longer uses the random fixed-port
  selection that produced the S5 collision.

Evidence:

- `node scripts/test-p47-viewer-client-separation.js`
- `node scripts/test-p47-scenario-harness.js`

## Goal 3: Retry S5 Once After Baseline Verification

Preflight:

```bash
./cli/target/debug/agent-browser service status --json
./cli/target/debug/agent-browser service incidents --summary --json
./cli/target/debug/agent-browser install doctor --json
./cli/target/debug/agent-browser doctor remote-view --json
node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only
node scripts/inspect-rdp-route-displays.js --display-content
```

Retry command:

```bash
node scripts/run-p46-stress-scenario.js \
  --scenario s5 \
  --reset-before \
  --reset-after \
  --agent-browser-command ./cli/target/debug/agent-browser \
  --require-explicit-agent-browser-command \
  --require-agent-browser-daemon-command-match
```

Pass criteria:

- Two distinct runtime profiles are concurrently route-bound.
- Profile A and profile B consume distinct route leases.
- Two external dashboard viewer clients target different profile browsers
  without consuming route leases.
- Both dashboard screenshots and route-display screenshots prove the expected
  browsers are visible.
- Closing profile A does not corrupt profile B.
- Reset-after leaves zero sessions, browsers, tabs, routes, and active
  incidents.

## Execution Log

### 2026-06-27

Initial diagnosis completed from the P46 plan, the S5 attempt 2 artifact, and
the current viewer-client source. P46 remains locked at S5 until Goals 1 and 2
pass.

Implemented Goals 1 and 2:

- `scripts/lib/p47-viewer-client.js` now launches external dashboard viewer
  clients with `--remote-debugging-port=0` by default and reads the resolved
  port from the isolated viewer profile's `DevToolsActivePort` file before
  using `/json/version` or `/json`.
- The launch artifact now records `remoteDebuggingPortMode`, requested `port`,
  resolved port, readiness URL, and active-port evidence.
- `P47_VIEWER_CLIENT_REMOTE_DEBUGGING_PORT` and
  `P46_VIEWER_CLIENT_REMOTE_DEBUGGING_PORT` remain available only as explicit
  diagnostic overrides.
- `scripts/test-p47-viewer-client-separation.js` now proves dynamic-port launch
  metadata, `DevToolsActivePort` parsing, valid override handling, and the
  absence of the old random fixed-port selector.

Validation:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/test-p47-viewer-client-separation.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
pnpm test:p47-viewer-client-separation
git diff --check -- scripts/lib/p47-viewer-client.js scripts/test-p47-viewer-client-separation.js docs/dev/plans/0054-2026-06-27-s5-viewer-client-port-allocation-plan.md
```

All checks passed. P46 may run one S5 retry from the explicit rebuilt-binary
lane after the live baseline preflight passes.

Live S5 retry:

```bash
node scripts/run-p46-stress-scenario.js --scenario s5 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s5-2026-06-27T19-41-29-598Z
```

Result: passed.

Evidence:

- operator A and operator B viewer-client launch artifacts used
  `remoteDebuggingPortMode: chromium_dynamic`, requested port `0`, and resolved
  distinct DevTools ports;
- profile A opened ready on route `guacamole:3` and display `:13`;
- profile B opened ready on route `guacamole:4` and display `:14`;
- both route-bound finalization checks passed with no blockers;
- both dashboard viewer clients clicked refresh successfully;
- both route displays reached `browser_window_visible`;
- profile B stayed ready after profile A closed;
- reset-after ended with zero active incidents.

P54 is complete. P46 is unlocked at S6.
