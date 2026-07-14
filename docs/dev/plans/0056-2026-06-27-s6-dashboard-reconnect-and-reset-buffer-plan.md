# P56 S6 Dashboard Reconnect And Reset Buffer Plan

Date: 2026-06-27
State: SUPERSEDED BY P57
Lane: P56
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/plans/0055-2026-06-27-s6-dashboard-swap-navigation-plan.md`
- `docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`

## Purpose

Repair the S6 failure exposed by the P55-authorized retry without a blind
additional live run. P55 moved the failure from a raw `Page.navigate` timeout
to a post-swap dashboard-state `Runtime.evaluate` timeout after the in-page
URL change artifact had already been written.

## Current Evidence

- P55-authorized S6 retry artifact:
  `/tmp/agent-browser-p46-s6-2026-06-27T20-06-51-508Z`.
- Failure:
  `CDP command Runtime.evaluate timed out after 30000ms`.
- `operator-a-swapped-to-profile-b-navigate.json` exists and records
  `ok: true`, proving the page accepted the dashboard URL change request.
- `operator-a-swapped-to-profile-b-dashboard-state.json` was not written,
  proving the failure happened during the first post-swap state poll.
- Reset-after again closed no sessions because the `service status` artifact hit
  Node `spawnSync` output buffering before JSON parsing.
- Manual cleanup closed:
  - `p46-s6-profile-a-2026-06-27T20-06-52-041Z`;
  - `p46-s6-profile-b-2026-06-27T20-06-52-041Z`.
- Final cleanup readback showed zero sessions, zero browsers, zero tabs, zero
  active incidents, and both route-pool entries available.

## Non-Negotiable Rules

- Do not run S6 again until the reconnect path and reset buffer guard have
  no-live coverage.
- Do not use service-owned `agent-browser` sessions for viewer clients.
- Preserve the swapped selection proof: operator A must read profile B after
  swap, and operator B must read profile A after swap.
- Reset cleanup must parse large `service status` payloads and close retained
  browser rows even when session rows are absent.

## Goal 1: Reconnect Viewer-Client CDP After Swap Navigation

Work:

- Add a viewer-client helper that rediscovers the current dashboard page from
  the viewer-client DevTools `/json` endpoint.
- Reconnect the viewer-client CDP websocket after each S6 swapped dashboard URL
  change, before polling dashboard state.
- Write explicit reconnect artifacts for both swapped operators.

Evidence:

```bash
node --check scripts/lib/p47-viewer-client.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
```

## Goal 2: Prevent Reset Evidence Truncation

Work:

- Give the P46 runner command wrapper an explicit `spawnSync` `maxBuffer` large
  enough for current `service status` payloads.
- Keep retained-browser close-target discovery from P55.

Evidence:

```bash
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
```

## Goal 3: Authorize One More S6 Retry

Preflight:

```bash
./cli/target/debug/agent-browser --json service status
./cli/target/debug/agent-browser --json service incidents --summary
./cli/target/debug/agent-browser --json install doctor
./cli/target/debug/agent-browser --json doctor remote-view
node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only
node scripts/inspect-rdp-route-displays.js --display-content
```

Retry command:

```bash
node scripts/run-p46-stress-scenario.js \
  --scenario s6 \
  --reset-before \
  --reset-after \
  --agent-browser-command ./cli/target/debug/agent-browser \
  --require-explicit-agent-browser-command \
  --require-agent-browser-daemon-command-match
```

Pass criteria:

- Both swapped reconnect artifacts exist and report `reconnected: true`.
- Swapped dashboard selected browser, session, and tab readback match the
  opposite profile for both operators.
- Refresh controls and screenshots work after the swap.
- Reset-after leaves zero sessions, browsers, tabs, route checkouts, and active
  incidents.

## Execution Log

### 2026-06-27

Implemented Goals 1 and 2:

- `scripts/lib/p47-viewer-client.js` now exports
  `reconnectDashboardViewerClient`, which rediscovers the active page from the
  resolved DevTools port and replaces the viewer-client CDP websocket.
- `scripts/run-p46-stress-scenario.js` now writes
  `operator-a-swapped-to-profile-b-reconnect.json` and
  `operator-b-swapped-to-profile-a-reconnect.json` before swapped state polling.
- The P46 runner command wrapper now uses a 32 MiB `spawnSync` buffer so large
  service-status payloads do not truncate reset evidence.
- No-live assertions cover reconnect artifacts and the reset buffer guard.

P46 remains locked at S6 until the listed validation passes and one P56
authorized live retry is run.

Validation:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
git diff --check -- scripts/lib/p47-viewer-client.js scripts/run-p46-stress-scenario.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0055-2026-06-27-s6-dashboard-swap-navigation-plan.md docs/dev/plans/0056-2026-06-27-s6-dashboard-reconnect-and-reset-buffer-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md
```

All checks passed.

P56-authorized S6 retry:

```bash
node scripts/run-p46-stress-scenario.js \
  --scenario s6 \
  --reset-before \
  --reset-after \
  --agent-browser-command ./cli/target/debug/agent-browser \
  --require-explicit-agent-browser-command \
  --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s6-2026-06-27T20-15-52-909Z
```

Result: failed.

Failure:

- `operator-a-swapped-to-profile-b-navigate.json` was written with `ok: true`.
- `operator-a-swapped-to-profile-b-reconnect.json` was not written.
- Reconnect timed out on `CDP command Page.enable timed out after 30000ms`.

Successful repair proof:

- Reset-after closed both retained profile sessions:
  `p46-s6-profile-a-2026-06-27T20-15-53-475Z` and
  `p46-s6-profile-b-2026-06-27T20-15-53-475Z`.
- Final readback showed zero sessions, zero browsers, zero tabs, zero active
  incidents, and both route-pool entries available.

P56 did not unlock S6. Follow-up diagnosis moved to
`docs/dev/plans/0057-2026-06-27-s6-viewer-client-cdp-swap-diagnostics-plan.md`.
